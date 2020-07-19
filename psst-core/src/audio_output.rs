use crate::error::Error;
use std::{
    convert::TryInto,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

pub type AudioSample = i16;

pub trait AudioSource: Iterator<Item = AudioSample> {
    fn channels(&self) -> u8;
    fn sample_rate(&self) -> u32;
}

pub struct AudioOutputCtrl {
    ctx: Arc<soundio::Context>,
    event_sender: Sender<InternalEvent>,
}

impl AudioOutputCtrl {
    pub fn close(&self) {
        self.send(InternalEvent::Close);
    }

    pub fn pause(&self) {
        self.send(InternalEvent::Pause);
    }

    pub fn resume(&self) {
        self.send(InternalEvent::Resume);
    }

    fn send(&self, event: InternalEvent) {
        self.event_sender.send(event).expect("Audio output died");
        self.ctx.wakeup();
    }
}

pub struct AudioOutput {
    ctx: Arc<soundio::Context>,
    event_sender: Sender<InternalEvent>,
    event_receiver: Receiver<InternalEvent>,
}

const WRITE_CALLBACK_LATENCY_SEC: f64 = 0.025;

impl AudioOutput {
    pub fn open() -> Result<Self, Error> {
        let (event_sender, event_receiver) = mpsc::channel();

        let error_callback = {
            let event_sender = event_sender.clone();
            move |err| {
                event_sender
                    .send(InternalEvent::Error(err))
                    .expect("Failed to send AudioOutputEvent::Error")
            }
        };
        let dev_callback = {
            let event_sender = event_sender.clone();
            move || {
                event_sender
                    .send(InternalEvent::DevicesChanged)
                    .expect("Failed to send AudioOutputEvent::DevicesChanged")
            }
        };

        let mut ctx = soundio::Context::new_with_callbacks(
            Some(error_callback),
            Some(dev_callback),
            None::<fn()>,
        );
        ctx.set_app_name("Psst");
        ctx.connect()?;

        Ok(Self {
            ctx: Arc::new(ctx),
            event_sender,
            event_receiver,
        })
    }

    pub fn controller(&self) -> AudioOutputCtrl {
        AudioOutputCtrl {
            ctx: self.ctx.clone(),
            event_sender: self.event_sender.clone(),
        }
    }

    pub fn start_playback<T>(&self, source: Arc<Mutex<T>>) -> Result<(), Error>
    where
        T: AudioSource,
    {
        loop {
            match self.play_on_default_device(source.clone())? {
                Continuation::Restart => {
                    continue;
                }
                Continuation::Close => {
                    return Ok(());
                }
            }
        }
    }

    fn play_on_default_device<T>(&self, source: Arc<Mutex<T>>) -> Result<Continuation, Error>
    where
        T: AudioSource,
    {
        let write_callback = {
            let source = source.clone();
            move |out: &mut soundio::OutStreamWriter| {
                let mut source = source.lock().expect("Failed to acquire audio source lock");
                out.begin_write(out.frame_count_max())
                    .expect("Failed to begin writing the audio output stream");
                for f in 0..out.frame_count() {
                    for c in 0..out.channel_count() {
                        let silence = 0;
                        out.set_sample(c, f, source.next().unwrap_or(silence));
                    }
                }
                out.end_write();
            }
        };

        let underflow_callback = {
            let event_sender = self.event_sender.clone();
            move || {
                event_sender
                    .send(InternalEvent::Underflow)
                    .expect("Failed to send AudioOutputEvent::Underflow")
            }
        };

        let error_callback = {
            let event_sender = self.event_sender.clone();
            move |err| {
                event_sender
                    .send(InternalEvent::Error(err))
                    .expect("Failed to send AudioOutputEvent::Error")
            }
        };

        let device = self.ctx.default_output_device()?;

        let mut stream = {
            let source = source.lock().expect("Failed to acquire audio source lock");
            let sample_rate = source
                .sample_rate()
                .try_into()
                .expect("Invalid sample rate");
            let channels = source.channels().into();
            device.open_outstream(
                sample_rate,
                soundio::Format::S16LE,
                soundio::ChannelLayout::get_default(channels),
                WRITE_CALLBACK_LATENCY_SEC,
                write_callback,
                Some(underflow_callback),
                Some(error_callback),
            )?
        };

        stream.start()?;

        loop {
            for event in self.event_receiver.try_iter() {
                match event {
                    InternalEvent::Error(err) => {
                        log::error!("audio output error: {}", err);
                        return Err(err.into());
                    }
                    InternalEvent::Close => {
                        log::debug!("closing audio output");
                        return Ok(Continuation::Close);
                    }
                    InternalEvent::DevicesChanged => {
                        log::info!("audio devices changed");
                        // TODO:
                        //  List devices and decide if we want to transfer.
                    }
                    InternalEvent::Underflow => {
                        log::warn!("audio output underflow");
                    }
                    InternalEvent::Pause => {
                        log::debug!("pausing audio output");
                        stream
                            .pause(true)
                            .expect("Failed to pause the audio output");
                    }
                    InternalEvent::Resume => {
                        log::debug!("resuming audio output");
                        stream
                            .pause(false)
                            .expect("Failed to resume the audio output");
                    }
                }
            }
            self.ctx.wait_events();
        }
    }
}

enum InternalEvent {
    Error(soundio::Error),
    DevicesChanged,
    Underflow,
    Close,
    Pause,
    Resume,
}

enum Continuation {
    Restart,
    Close,
}

impl From<soundio::Error> for Error {
    fn from(err: soundio::Error) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}
