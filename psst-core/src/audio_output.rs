use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;
use rb::{RbConsumer, RbProducer, RB};
use symphonia::core::sample::Sample;

use crate::{
    actor::{Actor, ActorHandle, ActorOp},
    error::Error,
};

const RING_BUF_SIZE: usize = 1024 * 8;

pub struct AudioOutput {
    sink: AudioSink<f32>,
    _handle: ActorHandle<OutputStreamMsg>,
    _ring_buf: rb::SpscRb<f32>,
}

impl AudioOutput {
    pub fn open() -> Result<Self, Error> {
        // Open the default output device.
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(cpal::DefaultStreamConfigError::DeviceNotAvailable)?;

        // Get the default device config, so we know what sample format and sample rate
        // the device supports.
        let supported = device.default_output_config()?;

        // Open an output stream with a ring buffer that will get consumed in the audio
        // thread and get written to in the playback threads (through an `AudioSink`).
        let ring_buf = rb::SpscRb::new(RING_BUF_SIZE);
        let handle = OutputStream::spawn_default({
            let config = supported.config();
            let consumer = ring_buf.consumer();
            // TODO: Support additional sample formats.
            move |_| OutputStream::open::<f32>(device, config, consumer).unwrap()
        });
        let sink = AudioSink {
            sample_rate: supported.sample_rate(),
            ring_buf_prod: Arc::new(ring_buf.producer()),
            msg_sender: handle.sender(),
        };

        Ok(Self {
            _ring_buf: ring_buf,
            _handle: handle,
            sink,
        })
    }

    pub fn sink(&self) -> AudioSink<f32> {
        self.sink.clone()
    }
}

pub trait OutputSample: cpal::Sample + Sample + Default + Send + 'static {}

impl OutputSample for i16 {}
impl OutputSample for u16 {}
impl OutputSample for f32 {}

#[derive(Clone)]
pub struct AudioSink<T: OutputSample> {
    sample_rate: cpal::SampleRate,
    ring_buf_prod: Arc<rb::Producer<T>>,
    msg_sender: Sender<OutputStreamMsg>,
}

impl<T: OutputSample> AudioSink<T> {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.0
    }

    pub fn write_blocking(&self, samples: &[T]) -> Result<(), Error> {
        // Write out all samples from the sample buffer to the ring buffer.
        let mut i = 0;
        while i < samples.len() {
            let writeable_samples = &samples[i..];

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

    pub fn pause(&self) {
        self.send(OutputStreamMsg::Pause);
    }

    pub fn resume(&self) {
        self.send(OutputStreamMsg::Resume);
    }

    pub fn close(&self) {
        self.send(OutputStreamMsg::Close);
    }

    fn send(&self, msg: OutputStreamMsg) {
        if self.msg_sender.send(msg).is_err() {
            log::error!("output stream actor is dead");
        }
    }
}

enum OutputStreamMsg {
    Pause,
    Resume,
    Close,
}

struct OutputStream {
    _device: cpal::Device,
    stream: cpal::Stream,
}

impl OutputStream {
    fn open<T: OutputSample>(
        device: cpal::Device,
        config: cpal::StreamConfig,
        ring_buf_cons: rb::Consumer<T>,
    ) -> Result<Self, Error> {
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
        Ok(Self {
            _device: device,
            stream,
        })
    }
}

impl Actor for OutputStream {
    type Message = OutputStreamMsg;
    type Error = Error;

    fn handle(&mut self, msg: Self::Message) -> Result<ActorOp, Self::Error> {
        match msg {
            OutputStreamMsg::Pause => {
                log::debug!("pausing audio output stream");
                if let Err(err) = self.stream.pause() {
                    log::error!("failed to stop stream: {}", err);
                }
                Ok(ActorOp::Continue)
            }
            OutputStreamMsg::Resume => {
                log::debug!("resuming audio output stream");
                if let Err(err) = self.stream.play() {
                    log::error!("failed to start stream: {}", err);
                }
                Ok(ActorOp::Continue)
            }
            OutputStreamMsg::Close => {
                log::debug!("closing audio output stream");
                let _ = self.stream.pause();
                Ok(ActorOp::Shutdown)
            }
        }
    }
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
