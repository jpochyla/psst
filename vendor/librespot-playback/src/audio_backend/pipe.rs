use super::{Open, Sink, SinkAsBytes, SinkError, SinkResult};
use crate::config::AudioFormat;
use crate::convert::Converter;
use crate::decoder::AudioPacket;

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process::exit;
use thiserror::Error;

#[derive(Debug, Error)]
enum StdoutError {
    #[error("<StdoutSink> {0}")]
    OnWrite(std::io::Error),

    #[error("<StdoutSink> File Path {file} Can Not be Opened and/or Created, {e}")]
    OpenFailure { file: String, e: std::io::Error },

    #[error("<StdoutSink> Failed to Flush the Output Stream, {0}")]
    FlushFailure(std::io::Error),

    #[error("<StdoutSink> The Output Stream is None")]
    NoOutput,
}

impl From<StdoutError> for SinkError {
    fn from(e: StdoutError) -> SinkError {
        use StdoutError::*;
        let es = e.to_string();
        match e {
            FlushFailure(_) | OnWrite(_) => SinkError::OnWrite(es),
            OpenFailure { .. } => SinkError::ConnectionRefused(es),
            NoOutput => SinkError::NotConnected(es),
        }
    }
}

pub struct StdoutSink {
    output: Option<Box<dyn Write>>,
    file: Option<String>,
    format: AudioFormat,
}

impl Open for StdoutSink {
    fn open(file: Option<String>, format: AudioFormat) -> Self {
        if let Some("?") = file.as_deref() {
            println!(
                "\nUsage:\n\nOutput to stdout:\n\n\t--backend pipe\n\nOutput to file:\n\n\t--backend pipe --device {{filename}}\n"
            );
            exit(0);
        }

        info!("Using StdoutSink (pipe) with format: {format:?}");

        Self {
            output: None,
            file,
            format,
        }
    }
}

impl Sink for StdoutSink {
    fn start(&mut self) -> SinkResult<()> {
        self.output.get_or_insert({
            match self.file.as_deref() {
                Some(file) => Box::new(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(file)
                        .map_err(|e| StdoutError::OpenFailure {
                            file: file.to_string(),
                            e,
                        })?,
                ),
                None => Box::new(io::stdout()),
            }
        });

        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        self.output
            .take()
            .ok_or(StdoutError::NoOutput)?
            .flush()
            .map_err(StdoutError::FlushFailure)?;

        Ok(())
    }

    sink_as_bytes!();
}

impl SinkAsBytes for StdoutSink {
    #[inline]
    fn write_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        self.output
            .as_deref_mut()
            .ok_or(StdoutError::NoOutput)?
            .write_all(data)
            .map_err(StdoutError::OnWrite)?;

        Ok(())
    }
}

impl StdoutSink {
    pub const NAME: &'static str = "pipe";
}
