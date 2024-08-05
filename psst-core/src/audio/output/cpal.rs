use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use crossbeam_channel::{bounded, Receiver, Sender};
use num_traits::Pow;
use std::time::Duration;

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

pub struct CpalOutput {
    _handle: ActorHandle<StreamMsg>,
    sink: CpalSink,
}

impl CpalOutput {
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

        let handle = Stream::spawn_with_default_cap("audio_output", {
            let config = supported.config();
            // TODO: Support additional sample formats.
            move |this| Stream::open(device, config, callback_recv, this).unwrap()
        });
        let sink = CpalSink {
            channel_count: supported.channels(),
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
}

impl AudioOutput for CpalOutput {
    type Sink = CpalSink;

    fn sink(&self) -> Self::Sink {
        self.sink.clone()
    }
}

#[derive(Clone)]
pub struct CpalSink {
    channel_count: cpal::ChannelCount,
    sample_rate: cpal::SampleRate,
    callback_send: Sender<CallbackMsg>,
    stream_send: Sender<StreamMsg>,
}

impl CpalSink {
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

impl AudioSink for CpalSink {
    fn channel_count(&self) -> usize {
        self.channel_count as usize
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate.0
    }

    fn set_volume(&self, volume: f32) {
        self.send_to_callback(CallbackMsg::SetVolume(volume));
    }

    fn play(&self, source: impl AudioSource) {
        self.send_to_callback(CallbackMsg::PlayAndResume(Box::new(source)));
        self.send_to_stream(StreamMsg::FlushAndResume);
    }

    fn pause(&self) {
        self.send_to_stream(StreamMsg::Pause);
        self.send_to_callback(CallbackMsg::Pause);
    }

    fn resume(&self) {
        self.send_to_callback(CallbackMsg::Resume);
        self.send_to_stream(StreamMsg::Resume);
    }

    fn stop(&self) {
        self.play(Empty);
        self.pause();
    }

    fn close(&self) {
        self.send_to_stream(StreamMsg::Close);
    }
}

struct Stream {
    stream: cpal::Stream,
    config: StreamConfig,
    callback_send: Sender<CallbackMsg>,
    stream_send: Sender<StreamMsg>,
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
            stream_send,
            source: Box::new(Empty),
            volume: 1.0, // We start with the full volume.
            state: CallbackState::Paused,
        };

        log::info!("opening output stream: {:?}", config);
        let (callback_send, _callback_recv) = crossbeam_channel::unbounded();
        let (stream_send, _stream_recv) = crossbeam_channel::unbounded();

        let stream = device.build_output_stream(
            &config,
            move |output, _| {
                callback.write_samples(output);
            },
            |err| {
                log::error!("audio output error: {}", err);
            },
            None,
        )?;

        Ok(Self {
            stream,
            config,
            callback_send,
            stream_send,
        })
    }
}

impl Stream {
    fn handle(&mut self, msg: StreamMsg) -> Result<Act<Self>, Error> {
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
            StreamMsg::FlushAndResume => {
                self.flush_buffer();
                self.handle(StreamMsg::Resume)
            }
        }
    }

    fn flush_buffer(&mut self) {
        // Step 1: Stop the stream
        if let Err(err) = self.stream.pause() {
            log::error!("Failed to pause stream for buffer flush: {}", err);
            return;
        }

        // Step 2: Create a temporary silent buffer
        let buffer_duration = Duration::from_millis(100); // Adjust as needed
        let num_samples = (self.config.sample_rate.0 as f32 * buffer_duration.as_secs_f32())
            as usize
            * self.config.channels as usize;
        let silent_buffer = vec![0.0f32; num_samples];

        // Step 3: Write silent samples to flush any remaining audio
        let (temp_sender, temp_receiver) = crossbeam_channel::bounded(16);

        let mut temp_callback = StreamCallback {
            callback_recv: temp_receiver,
            stream_send: self.stream_send.clone(),
            source: Box::new(Empty),
            state: CallbackState::Playing,
            volume: 1.0,
        };

        // Simulate a write operation to flush the buffer
        let mut output_buffer = vec![0.0f32; silent_buffer.len()];
        temp_callback.write_samples(&mut output_buffer);

        // Step 4: Prepare the stream for playback again
        if let Err(err) = self.stream.play() {
            log::error!("Failed to resume stream after buffer flush: {}", err);
        }
    }

    fn resume_playback(&mut self) -> Result<Act<Self>, Error> {
        log::debug!("resuming audio output stream");
        if let Err(err) = self.stream.play() {
            log::error!("failed to start stream: {}", err);
        }
        Ok(Act::Continue)
    }

    fn pause_playback(&mut self) -> Result<Act<Self>, Error> {
        log::debug!("pausing audio output stream");
        if let Err(err) = self.stream.pause() {
            log::error!("failed to pause stream: {}", err);
        }
        Ok(Act::Continue)
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
            StreamMsg::FlushAndResume => {
                log::debug!("flushing buffer and resuming audio output stream");
                self.flush_buffer();
                if let Err(err) = self.stream.play() {
                    log::error!("failed to resume stream after flush: {}", err);
                }
                Ok(Act::Continue)
            }
        }
    }
}

enum StreamMsg {
    FlushAndResume,
    Pause,
    Resume,
    Close,
}

enum CallbackMsg {
    PlayAndResume(Box<dyn AudioSource>),
    Pause,
    Resume,
    SetVolume(f32),
}

enum CallbackState {
    Playing,
    Paused,
}

struct StreamCallback {
    #[allow(unused)]
    stream_send: Sender<StreamMsg>,
    callback_recv: Receiver<CallbackMsg>,
    source: Box<dyn AudioSource>,
    state: CallbackState,
    volume: f32,
}

impl StreamCallback {
    fn write_samples(&mut self, output: &mut [f32]) {
        while let Ok(msg) = self.callback_recv.try_recv() {
            match msg {
                CallbackMsg::PlayAndResume(src) => {
                    self.source = src;
                    self.state = CallbackState::Playing;
                }
                CallbackMsg::Pause => {
                    self.state = CallbackState::Paused;
                }
                CallbackMsg::Resume => {
                    self.state = CallbackState::Playing;
                }
                CallbackMsg::SetVolume(volume) => {
                    self.volume = volume;
                }
            }
        }

        let written = if matches!(self.state, CallbackState::Playing) {
            // Write out as many samples as possible from the audio source to the
            // output buffer.
            let written = self.source.write(output);

            // Apply scaled global volume level.
            let scaled_volume = self.volume.pow(4);
            output[..written]
                .iter_mut()
                .for_each(|s| *s *= scaled_volume);

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
