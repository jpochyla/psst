use crate::error::Error;
use crossbeam_channel::{unbounded, Receiver, Sender};
use miniaudio::{Device, DeviceConfig, DeviceType, Format};
use std::sync::{Arc, Mutex};

pub type AudioSample = i16;

pub trait AudioSource: Iterator<Item = AudioSample> {
    fn channels(&self) -> u8;
    fn sample_rate(&self) -> u32;
}

pub struct AudioOutputRemote {
    event_sender: Sender<InternalEvent>,
}

impl AudioOutputRemote {
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
    }
}

pub struct AudioOutput {
    event_sender: Sender<InternalEvent>,
    event_receiver: Receiver<InternalEvent>,
}

impl AudioOutput {
    pub fn open() -> Result<Self, Error> {
        let (event_sender, event_receiver) = unbounded();
        Ok(Self {
            event_sender,
            event_receiver,
        })
    }

    pub fn remote(&self) -> AudioOutputRemote {
        AudioOutputRemote {
            event_sender: self.event_sender.clone(),
        }
    }

    pub fn start_playback<T>(&self, source: Arc<Mutex<T>>) -> Result<(), Error>
    where
        T: AudioSource + Send + 'static,
    {
        let config = {
            let source = source.lock().expect("Failed to acquire audio source lock");
            let mut config = DeviceConfig::new(DeviceType::Playback);
            config.playback_mut().set_format(Format::S16);
            config.playback_mut().set_channels(source.channels().into());
            config.set_sample_rate(source.sample_rate());
            config
        };

        let mut device = Device::new(None, &config)?;

        device.set_data_callback(move |_device, output, _frames| {
            let mut source = source.lock().expect("Failed to acquire audio source lock");
            for sample in output.as_samples_mut() {
                *sample = source.next().unwrap_or(0);
            }
        });

        for event in self.event_receiver.iter() {
            match event {
                InternalEvent::Close => {
                    log::debug!("closing audio output");
                    if device.is_started() {
                        device.stop()?;
                    }
                    break;
                }
                InternalEvent::Pause => {
                    log::debug!("pausing audio output");
                    if device.is_started() {
                        device.stop()?;
                    }
                }
                InternalEvent::Resume => {
                    log::debug!("resuming audio output");
                    if !device.is_started() {
                        device.start()?;
                    }
                }
            }
        }

        Ok(())
    }
}

enum InternalEvent {
    Close,
    Pause,
    Resume,
}

impl From<miniaudio::Error> for Error {
    fn from(err: miniaudio::Error) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}
