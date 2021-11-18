use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::source::Empty,
    error::Error,
};

use super::source::AudioSource;

pub struct AudioOutput {
    _handle: ActorHandle<StreamMsg>,
    sink: AudioSink,
}

impl AudioOutput {
    pub fn open() -> Result<Self, Error> {
        // Open the default output device.
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(cpal::DefaultStreamConfigError::DeviceNotAvailable)?;

        if let Ok(name) = device.name() {
            log::info!("using audio device: {:?}", name);
        }

        // Get the default device config, so we know what sample format and sample rate
        // the device supports.
        let supported = Self::preferred_output_config(&device)?;

        let (callback_send, callback_recv) = bounded(16);

        let handle = Stream::spawn_default({
            let config = supported.config();
            // TODO: Support additional sample formats.
            move |this| Stream::open(device, config, callback_recv, this).unwrap()
        });
        let sink = AudioSink {
            sample_rate: supported.sample_rate(),
            stream_send: handle.sender(),
            callback_send,
        };

        Ok(Self {
            _handle: handle,
            sink,
        })
    }

    fn preferred_output_config(
        device: &cpal::Device,
    ) -> Result<cpal::SupportedStreamConfig, Error> {
        const PREFERRED_SAMPLE_FORMAT: cpal::SampleFormat = cpal::SampleFormat::F32;
        const PREFERRED_SAMPLE_RATE: cpal::SampleRate = cpal::SampleRate(44_100);
        const PREFERRED_CHANNELS: cpal::ChannelCount = 2;

        for s in device.supported_output_configs()? {
            let rates = s.min_sample_rate()..=s.max_sample_rate();
            if s.channels() == PREFERRED_CHANNELS
                && s.sample_format() == PREFERRED_SAMPLE_FORMAT
                && rates.contains(&PREFERRED_SAMPLE_RATE)
            {
                return Ok(s.with_sample_rate(PREFERRED_SAMPLE_RATE));
            }
        }

        Ok(device.default_output_config()?)
    }

    pub fn sink(&self) -> AudioSink {
        self.sink.clone()
    }
}

#[derive(Clone)]
pub struct AudioSink {
    sample_rate: cpal::SampleRate,
    callback_send: Sender<CallbackMsg>,
    stream_send: Sender<StreamMsg>,
}

impl AudioSink {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.0
    }

    pub fn set_volume(&self, volume: f32) {
        self.send_to_callback(CallbackMsg::SetVolume(volume));
    }

    pub fn play(&self, source: impl AudioSource) {
        self.send_to_callback(CallbackMsg::PlaySource(Box::new(source)));
    }

    pub fn pause(&self) {
        self.send_to_stream(StreamMsg::Pause);
        self.send_to_callback(CallbackMsg::Pause);
    }

    pub fn resume(&self) {
        self.send_to_stream(StreamMsg::Resume);
        self.send_to_callback(CallbackMsg::Resume);
    }

    pub fn close(&self) {
        self.send_to_stream(StreamMsg::Close);
    }

    fn send_to_callback(&self, msg: CallbackMsg) {
        if self.callback_send.send(msg).is_err() {
            log::error!("output stream actor is dead");
        }
    }

    fn send_to_stream(&self, msg: StreamMsg) {
        if self.stream_send.send(msg).is_err() {
            log::error!("output stream actor is dead");
        }
    }
}

enum StreamMsg {
    Pause,
    Resume,
    Close,
}

struct Stream {
    stream: cpal::Stream,
    _device: cpal::Device,
}

impl Stream {
    fn open(
        device: cpal::Device,
        config: cpal::StreamConfig,
        callback_recv: Receiver<CallbackMsg>,
        stream_send: Sender<StreamMsg>,
    ) -> Result<Self, Error> {
        let mut callback = StreamCallback {
            callback_recv,
            _stream_send: stream_send,
            source: Box::new(Empty),
            volume: 1.0, // We start with the full volume.
            state: CallbackState::Paused,
        };

        log::info!("opening output stream: {:?}", config);
        let stream = device.build_output_stream(
            &config,
            move |output, _| {
                callback.write_samples(output);
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

impl Actor for Stream {
    type Message = StreamMsg;
    type Error = Error;

    fn handle(&mut self, msg: Self::Message) -> Result<Act<Self>, Self::Error> {
        match msg {
            StreamMsg::Pause => {
                log::debug!("pausing audio output stream");
                if let Err(err) = self.stream.pause() {
                    log::error!("failed to stop stream: {}", err);
                }
                Ok(Act::Continue)
            }
            StreamMsg::Resume => {
                log::debug!("resuming audio output stream");
                if let Err(err) = self.stream.play() {
                    log::error!("failed to start stream: {}", err);
                }
                Ok(Act::Continue)
            }
            StreamMsg::Close => {
                log::debug!("closing audio output stream");
                let _ = self.stream.pause();
                Ok(Act::Shutdown)
            }
        }
    }
}

enum CallbackMsg {
    PlaySource(Box<dyn AudioSource>),
    SetVolume(f32),
    Pause,
    Resume,
}

enum CallbackState {
    Playing,
    Paused,
}

struct StreamCallback {
    _stream_send: Sender<StreamMsg>,
    callback_recv: Receiver<CallbackMsg>,
    source: Box<dyn AudioSource>,
    state: CallbackState,
    volume: f32,
}

impl StreamCallback {
    fn write_samples(&mut self, output: &mut [f32]) {
        // Process any pending data messages.
        while let Ok(msg) = self.callback_recv.try_recv() {
            match msg {
                CallbackMsg::PlaySource(src) => {
                    self.source = src;
                }
                CallbackMsg::SetVolume(volume) => {
                    self.volume = volume;
                }
                CallbackMsg::Pause => {
                    self.state = CallbackState::Paused;
                }
                CallbackMsg::Resume => {
                    self.state = CallbackState::Playing;
                }
            }
        }

        let written = if matches!(self.state, CallbackState::Playing) {
            // Write out as many samples as possible from the audio source to the
            // output buffer.
            let written = self.source.write(output);

            // Apply the global volume level.
            output[..written].iter_mut().for_each(|s| *s *= self.volume);

            written
        } else {
            0
        };

        // Mute any remaining samples.
        output[written..].iter_mut().for_each(|s| *s = 0.0);
    }
}

impl From<cpal::DefaultStreamConfigError> for Error {
    fn from(err: cpal::DefaultStreamConfigError) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}

impl From<cpal::SupportedStreamConfigsError> for Error {
    fn from(err: cpal::SupportedStreamConfigsError) -> Error {
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
