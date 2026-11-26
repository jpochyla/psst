use super::{Open, Sink, SinkAsBytes, SinkError, SinkResult};
use crate::config::AudioFormat;
use crate::convert::Converter;
use crate::decoder::AudioPacket;
use shell_words::split;

use std::io::{ErrorKind, Write};
use std::process::{Child, Command, Stdio, exit};
use thiserror::Error;

#[derive(Debug, Error)]
enum SubprocessError {
    #[error("<SubprocessSink> {0}")]
    OnWrite(std::io::Error),

    #[error("<SubprocessSink> Command {command} Can Not be Executed, {e}")]
    SpawnFailure { command: String, e: std::io::Error },

    #[error("<SubprocessSink> Failed to Parse Command args for {command}, {e}")]
    InvalidArgs {
        command: String,
        e: shell_words::ParseError,
    },

    #[error("<SubprocessSink> Failed to Flush the Subprocess, {0}")]
    FlushFailure(std::io::Error),

    #[error("<SubprocessSink> Failed to Kill the Subprocess, {0}")]
    KillFailure(std::io::Error),

    #[error("<SubprocessSink> Failed to Wait for the Subprocess to Exit, {0}")]
    WaitFailure(std::io::Error),

    #[error("<SubprocessSink> The Subprocess is no longer able to accept Bytes")]
    WriteZero,

    #[error("<SubprocessSink> Missing Required Shell Command")]
    MissingCommand,

    #[error("<SubprocessSink> The Subprocess is None")]
    NoChild,

    #[error("<SubprocessSink> The Subprocess's stdin is None")]
    NoStdin,
}

impl From<SubprocessError> for SinkError {
    fn from(e: SubprocessError) -> SinkError {
        use SubprocessError::*;
        let es = e.to_string();
        match e {
            FlushFailure(_) | KillFailure(_) | WaitFailure(_) | OnWrite(_) | WriteZero => {
                SinkError::OnWrite(es)
            }
            SpawnFailure { .. } => SinkError::ConnectionRefused(es),
            MissingCommand | InvalidArgs { .. } => SinkError::InvalidParams(es),
            NoChild | NoStdin => SinkError::NotConnected(es),
        }
    }
}

pub struct SubprocessSink {
    shell_command: Option<String>,
    child: Option<Child>,
    format: AudioFormat,
}

impl Open for SubprocessSink {
    fn open(shell_command: Option<String>, format: AudioFormat) -> Self {
        if let Some("?") = shell_command.as_deref() {
            println!(
                "\nUsage:\n\nOutput to a Subprocess:\n\n\t--backend subprocess --device {{shell_command}}\n"
            );
            exit(0);
        }

        info!("Using SubprocessSink with format: {format:?}");

        Self {
            shell_command,
            child: None,
            format,
        }
    }
}

impl Sink for SubprocessSink {
    fn start(&mut self) -> SinkResult<()> {
        self.child.get_or_insert({
            match self.shell_command.as_deref() {
                Some(command) => {
                    let args = split(command).map_err(|e| SubprocessError::InvalidArgs {
                        command: command.to_string(),
                        e,
                    })?;

                    Command::new(&args[0])
                        .args(&args[1..])
                        .stdin(Stdio::piped())
                        .spawn()
                        .map_err(|e| SubprocessError::SpawnFailure {
                            command: command.to_string(),
                            e,
                        })?
                }
                None => return Err(SubprocessError::MissingCommand.into()),
            }
        });

        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        let child = &mut self.child.take().ok_or(SubprocessError::NoChild)?;

        match child.try_wait() {
            // The process has already exited
            // nothing to do.
            Ok(Some(_)) => Ok(()),
            Ok(_) => {
                // The process Must DIE!!!
                child
                    .stdin
                    .take()
                    .ok_or(SubprocessError::NoStdin)?
                    .flush()
                    .map_err(SubprocessError::FlushFailure)?;

                child.kill().map_err(SubprocessError::KillFailure)?;
                child.wait().map_err(SubprocessError::WaitFailure)?;

                Ok(())
            }
            Err(e) => Err(SubprocessError::WaitFailure(e).into()),
        }
    }

    sink_as_bytes!();
}

impl SinkAsBytes for SubprocessSink {
    #[inline]
    fn write_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        // We get one attempted restart per write.
        // We don't want to get stuck in a restart loop.
        let mut restarted = false;
        let mut start_index = 0;
        let data_len = data.len();
        let mut end_index = data_len;

        loop {
            match self
                .child
                .as_ref()
                .ok_or(SubprocessError::NoChild)?
                .stdin
                .as_ref()
                .ok_or(SubprocessError::NoStdin)?
                .write(&data[start_index..end_index])
            {
                Ok(0) => {
                    // Potentially fatal.
                    // As per the docs a return value of 0
                    // means we shouldn't try to write to the
                    // process anymore so let's try a restart
                    // if we haven't already.
                    self.try_restart(SubprocessError::WriteZero, &mut restarted)?;

                    continue;
                }
                Ok(bytes_written) => {
                    // What we want, a successful write.
                    start_index = data_len.min(start_index + bytes_written);
                    end_index = data_len.min(start_index + bytes_written);

                    if end_index == data_len {
                        break Ok(());
                    }
                }
                // Non-fatal, retry the write.
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => {
                    // Very possibly fatal,
                    // but let's try a restart anyway if we haven't already.
                    self.try_restart(SubprocessError::OnWrite(e), &mut restarted)?;

                    continue;
                }
            }
        }
    }
}

impl SubprocessSink {
    pub const NAME: &'static str = "subprocess";

    fn try_restart(&mut self, e: SubprocessError, restarted: &mut bool) -> SinkResult<()> {
        // If the restart fails throw the original error back.
        if !*restarted && self.stop().is_ok() && self.start().is_ok() {
            *restarted = true;

            Ok(())
        } else {
            Err(e.into())
        }
    }
}
