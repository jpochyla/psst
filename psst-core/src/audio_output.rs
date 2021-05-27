use crate::error::Error;
use crossbeam_channel::{unbounded, Receiver, Sender};
use miniaudio::{Context, Device, DeviceConfig, DeviceType, Format};
use std::sync::{Arc, Mutex};

pub type AudioSample = f32;

pub trait AudioSource: Iterator<Item = AudioSample> {
    fn channels(&self) -> u8;
    fn sample_rate(&self) -> u32;
    fn normalization_factor(&self) -> Option<f32>;
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

    pub fn set_volume(&self, volume: f32) {
        self.send(InternalEvent::SetVolume(volume));
    }

    fn send(&self, event: InternalEvent) {
        self.event_sender.send(event).expect("Audio output died");
    }
}

pub struct AudioOutput {
    context: Context,
    event_sender: Sender<InternalEvent>,
    event_receiver: Receiver<InternalEvent>,
}

impl AudioOutput {
    pub fn open() -> Result<Self, Error> {
        let backends = &[]; // Use default backend order.
        let config = None; // Use default context config.
        let context = Context::new(backends, config)?;

        // Channel used for controlling the audio output.
        let (event_sender, event_receiver) = unbounded();

        Ok(Self {
            context,
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
        // Create a device config that describes the kind of device we want to use.
        let mut config = DeviceConfig::new(DeviceType::Playback);

        {
            // Setup the device config for playback with the channel count and sample rate
            // from the audio source.
            let source = source.lock().expect("Failed to acquire audio source lock");
            config.playback_mut().set_format(Format::F32);
            config.playback_mut().set_channels(source.channels().into());
            config.set_sample_rate(source.sample_rate());
        };

        // Move the source into the config's data callback.  Callback will get cloned
        // for each device we create.
        config.set_data_callback(move |_device, output, _frames| {
            let mut source = source.lock().expect("Failed to acquire audio source lock");
            // Get the audio normalization factor.
            let norm_factor = source.normalization_factor().unwrap_or(1.0);
            // Fill the buffer with audio samples from the source.
            for sample in output.as_samples_mut() {
                let s = source.next().unwrap_or(0.0); // Use silence in case the
                                                      // source has finished.
                *sample = s * norm_factor;
            }
        });

        let device = {
            let context = self.context.clone();
            Device::new(Some(context), &config)?
        };

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
                InternalEvent::SetVolume(volume) => {
                    log::debug!("volume has changed");
                    if device.is_started() {
                        device.set_master_volume(volume);
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
    SetVolume(f32),
}

impl From<miniaudio::Error> for Error {
    fn from(err: miniaudio::Error) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}
