use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};
use rb::{RbConsumer, RbProducer, RB};
use symphonia::core::{
    audio::{Channels, SampleBuffer, SignalSpec},
    conv::ConvertibleSample,
};

use crate::error::Error;

const SAMPLE_RATE: u32 = 41000;
const RING_BUF_SIZE: usize = 1024 * 4;

pub struct AudioOutput {
    device: cpal::Device,
    sink: Arc<AudioSink<f32>>,
    event_sender: Sender<InternalEvent>,
    event_receiver: Receiver<InternalEvent>,
}

impl AudioOutput {
    pub fn open() -> Result<Self, Error> {
        // Open the default output device.
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(cpal::DefaultStreamConfigError::DeviceNotAvailable)?;

        // Channel used for controlling the audio output state.
        let (event_sender, event_receiver) = bounded(0);

        let sink = Arc::new(AudioSink::new(rb::SpscRb::new(RING_BUF_SIZE)));

        Ok(Self {
            device,
            sink,
            event_sender,
            event_receiver,
        })
    }

    pub fn remote(&self) -> AudioOutputRemote {
        AudioOutputRemote {
            event_sender: self.event_sender.clone(),
        }
    }

    pub fn sink(&self) -> Arc<AudioSink<f32>> {
        Arc::clone(&self.sink)
    }

    pub fn play(&self) -> Result<(), Error> {
        // TODO: Add additional sample formats.
        let stream = OutputStream::try_open::<f32>(
            &self.device,
            SignalSpec {
                rate: SAMPLE_RATE,
                channels: Channels::FRONT_LEFT | Channels::FRONT_RIGHT,
            },
            self.sink.ring_buf.consumer(),
        )?;
        for event in &self.event_receiver {
            match event {
                InternalEvent::Close => {
                    log::debug!("closing audio output");
                    let _ = stream.pause();
                    break;
                }
                InternalEvent::Pause => {
                    log::debug!("pausing audio output");
                    if let Err(err) = stream.pause() {
                        log::error!("failed to stop stream: {}", err);
                    }
                }
                InternalEvent::Resume => {
                    log::debug!("resuming audio output");
                    if let Err(err) = stream.play() {
                        log::error!("failed to start stream: {}", err);
                    }
                }
                InternalEvent::SetVolume(_vol) => {
                    log::debug!("volume has changed");
                }
            }
        }

        Ok(())
    }
}

pub trait OutputSample: cpal::Sample + ConvertibleSample + Send + 'static {}

impl OutputSample for i16 {}
impl OutputSample for u16 {}
impl OutputSample for f32 {}

struct OutputStream {
    stream: cpal::Stream,
}

impl OutputStream {
    fn try_open<T: OutputSample>(
        device: &cpal::Device,
        spec: SignalSpec,
        ring_buf_cons: rb::Consumer<T>,
    ) -> Result<Self, Error> {
        let config = Self::config_for_spec(&spec);
        let stream = device.build_output_stream(
            &config,
            move |output: &mut [T], _| {
                // Write out as many samples as possible from the ring buffer to the audio
                // output.
                let written = ring_buf_cons.read(output).unwrap_or(0);
                // Mute any remaining samples.
                output[written..].iter_mut().for_each(|s| *s = T::MID);
            },
            |err| {
                log::error!("audio output error: {}", err);
            },
        )?;
        Ok(Self { stream })
    }

    fn config_for_spec(spec: &SignalSpec) -> cpal::StreamConfig {
        cpal::StreamConfig {
            channels: spec.channels.count() as cpal::ChannelCount,
            sample_rate: cpal::SampleRate(spec.rate),
            buffer_size: cpal::BufferSize::Default,
        }
    }

    fn play(&self) -> Result<(), Error> {
        Ok(self.stream.play()?)
    }

    fn pause(&self) -> Result<(), Error> {
        Ok(self.stream.pause()?)
    }
}

pub struct AudioSink<T: OutputSample> {
    ring_buf: rb::SpscRb<T>,
    ring_buf_prod: rb::Producer<T>,
}

impl<T: OutputSample> AudioSink<T> {
    fn new(ring_buf: rb::SpscRb<T>) -> AudioSink<T> {
        Self {
            ring_buf_prod: ring_buf.producer(),
            ring_buf,
        }
    }

    pub fn clear(&self) {
        self.ring_buf.clear();
    }

    pub fn write_blocking(&self, sample_buf: &SampleBuffer<T>) -> Result<(), Error> {
        // Write out all samples from the sample buffer to the ring buffer.
        let mut i = 0;
        while i < sample_buf.len() {
            let writeable_samples = &sample_buf.samples()[i..];

            // Write as many samples as possible to the ring buffer. This blocks until some
            // samples are written or the consumer has been destroyed (None is returned).
            if let Some(written) = self.ring_buf_prod.write_blocking(writeable_samples) {
                i += written;
            } else {
                // Consumer destroyed, return an error.
                return Err(cpal::PlayStreamError::DeviceNotAvailable.into());
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
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

    pub fn set_volume(&self, volume: f64) {
        self.send(InternalEvent::SetVolume(volume));
    }

    fn send(&self, event: InternalEvent) {
        self.event_sender.send(event).expect("Audio output died");
    }
}

enum InternalEvent {
    Close,
    Pause,
    Resume,
    SetVolume(f64),
}

impl From<cpal::DefaultStreamConfigError> for Error {
    fn from(err: cpal::DefaultStreamConfigError) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}

impl From<cpal::BuildStreamError> for Error {
    fn from(err: cpal::BuildStreamError) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}

impl From<cpal::PlayStreamError> for Error {
    fn from(err: cpal::PlayStreamError) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}

impl From<cpal::PauseStreamError> for Error {
    fn from(err: cpal::PauseStreamError) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}
