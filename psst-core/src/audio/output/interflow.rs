use std::{
    fmt::{Debug, Display, Formatter},
    sync::{Arc, PoisonError, RwLock},
};

use crossbeam_channel::Sender;
use interflow::{
    prelude::default_output_device, AudioCallbackContext, AudioDevice, AudioOutputDevice,
};
use log::{debug, error, info, trace};
use ndarray::Array;

use crate::{
    actor::{Act, Actor},
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

pub struct InterflowOutput {
    sink: Box<InterflowSink>,
}

impl InterflowOutput {
    pub fn open() -> Result<Box<dyn AudioOutput>, Error> {
        info!("opening audio output: interflow");
        let mut stream = InterflowStream::new();
        let channel_count = stream.channel_count.clone();
        let sample_rate = stream.sample_rate.clone();
        let stream_handle = InterflowStream::spawn_with_default_cap("audio_output", move |this| {
            stream.open().unwrap()
        });
        let sink = Box::new(InterflowSink {
            stream_send: stream_handle.sender(),
            channel_count: channel_count.clone(),
            sample_rate: sample_rate.clone(),
        });
        Ok(Box::new(Self { sink: sink }))
    }
}

impl AudioOutput for InterflowOutput {
    fn sink(&self) -> Box<dyn AudioSink> {
        return self.sink.clone();
    }
}

enum StreamMsg {
    Open,
    Close,
    Pause,
    Resume,
    Stop,
    Play(Box<dyn AudioSource>),
    SetVolume(f32),
}

#[derive(Clone)]
struct InterflowStream {
    channel_count: Arc<RwLock<usize>>,
    sample_rate: Arc<RwLock<u32>>,
    playing: Arc<RwLock<bool>>,
    volume: Arc<RwLock<f32>>,
    source: Arc<RwLock<Box<dyn AudioSource>>>,
}

impl InterflowStream {
    pub fn new() -> Self {
        info!("creating stream");
        Self {
            channel_count: Arc::new(RwLock::new(2)),
            sample_rate: Arc::new(RwLock::new(44_100)),
            playing: Arc::new(RwLock::new(false)),
            volume: Arc::new(RwLock::new(0.0)),
            source: Arc::new(RwLock::new(Box::new(Empty))),
        }
    }

    pub fn open(&mut self) -> Result<Self, Error> {
        info!("opening stream");
        let callback = InterflowCallback {
            channel_count: self.channel_count.clone(),
            sample_rate: self.sample_rate.clone(),
            playing: self.playing.clone(),
            volume: self.volume.clone(),
            source: self.source.clone(),
        };
        let device = default_output_device();
        let mut config = device
            .default_output_config()
            .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        config.samplerate = 44_100 as f64;
        if !device.is_config_supported(&config) {
            error!("device does not support config: {:?}", config);
            config = device
                .default_output_config()
                .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        }
        let stream = device
            .create_output_stream(config, callback)
            .map(Box::new)
            .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        self.set_channel_count(config.channels as usize)
            .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        self.set_sample_rate(config.samplerate as u32)
            .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        return Ok(self.clone());
    }

    pub fn set_playing(&mut self, playing: bool) -> Result<(), Error> {
        self.playing
            .write()
            .map(|mut v| *v = playing)
            .map_err(|err| Error::AudioOutputError(Box::new(InterflowError::from(err))))
    }

    pub fn set_channel_count(&mut self, count: usize) -> Result<(), Error> {
        debug!("setting channel count to {}", count);
        self.channel_count
            .write()
            .map(|mut v| *v = count)
            .map_err(|err| Error::AudioOutputError(Box::new(InterflowError::from(err))))
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<(), Error> {
        debug!("setting sample rate to {}", sample_rate);
        self.sample_rate
            .write()
            .map(|mut v| *v = sample_rate)
            .map_err(|err| Error::AudioOutputError(Box::new(InterflowError::from(err))))
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<(), Error> {
        self.volume
            .write()
            .map(|mut v| *v = volume)
            .map_err(|err| Error::AudioOutputError(Box::new(InterflowError::from(err))))
    }

    pub fn set_source(&mut self, source: Box<dyn AudioSource>) -> Result<(), Error> {
        self.source
            .write()
            .map(|mut v| *v = source)
            .map_err(|err| Error::AudioOutputError(Box::new(InterflowError::from(err))))
    }
}

impl Actor for InterflowStream {
    type Message = StreamMsg;
    type Error = Error;

    fn handle(&mut self, msg: StreamMsg) -> Result<Act<Self>, Self::Error> {
        match msg {
            StreamMsg::Open => {
                debug!("opening stream");
                Ok(Act::Continue)
            }
            StreamMsg::Close => {
                debug!("closing stream");
                self.set_playing(false)?;
                self.set_source(Box::new(Empty))?;
                Ok(Act::Shutdown)
            }
            StreamMsg::Pause => {
                debug!("pausing stream");
                self.set_playing(false)?;
                Ok(Act::Continue)
            }
            StreamMsg::Resume => {
                debug!("resuming stream");
                self.set_playing(true)?;
                Ok(Act::Continue)
            }
            StreamMsg::Stop => {
                debug!("stopping stream");
                self.set_playing(false)?;
                self.set_source(Box::new(Empty))?;
                Ok(Act::Continue)
            }
            StreamMsg::Play(source) => {
                debug!("playing stream");
                self.set_source(source)?;
                self.set_playing(true)?;
                Ok(Act::Continue)
            }
            StreamMsg::SetVolume(volume) => {
                debug!("setting volume on stream to {}", volume);
                self.set_volume(volume)?;
                Ok(Act::Continue)
            }
        }
    }
}

struct InterflowCallback {
    channel_count: Arc<RwLock<usize>>,
    sample_rate: Arc<RwLock<u32>>,
    playing: Arc<RwLock<bool>>,
    volume: Arc<RwLock<f32>>,
    source: Arc<RwLock<Box<dyn AudioSource>>>,
}

impl interflow::AudioOutputCallback for InterflowCallback {
    fn on_output_data(
        &mut self,
        context: AudioCallbackContext,
        mut output: interflow::AudioOutput<f32>,
    ) {
        if (context.stream_config.channels as usize) != *self.channel_count.read().unwrap() {
            info!(
                "channel count mismatch: context channels: {}, source channels: {}",
                context.stream_config.channels,
                self.source.read().unwrap().channel_count()
            );
            self.channel_count
                .write()
                .map(|mut v| *v = context.stream_config.channels as usize)
                .unwrap();
        }
        if (context.stream_config.samplerate as u32) != *self.sample_rate.read().unwrap() {
            info!(
                "sample rate mismatch: context sample rate: {}, source sample rate: {}",
                context.stream_config.samplerate,
                self.source.read().unwrap().sample_rate()
            );
            self.sample_rate
                .write()
                .map(|mut v| *v = context.stream_config.samplerate as u32)
                .unwrap();
        }
        if *self
            .playing
            .read()
            .inspect_err(|err| {
                error!("failed to read playing state: {}", err);
            })
            .unwrap()
        {
            trace!(
                "samples: {} channels: {} context_sample_rate: {} context_channels: {}",
                output.buffer.num_samples(),
                output.buffer.num_channels(),
                context.stream_config.samplerate,
                context.stream_config.channels
            );
            let mut buf = vec![0.0; output.buffer.num_samples() * output.buffer.num_channels()];
            let _written = self.source.write().unwrap().write(&mut buf);
            // output.buffer.set_frame(samples, &Array::from_vec(buf));
            let _res = output.buffer.copy_from_interleaved(&buf);
            output.buffer.change_amplitude(
                *self
                    .volume
                    .read()
                    .inspect_err(|err| error!("failed to read volume: {}", err))
                    .unwrap(),
            );
        }
    }
}

#[derive(Clone)]
struct InterflowSink {
    stream_send: Sender<StreamMsg>,
    channel_count: Arc<RwLock<usize>>,
    sample_rate: Arc<RwLock<u32>>,
}

impl AudioSink for InterflowSink {
    fn channel_count(&self) -> usize {
        *self
            .channel_count
            .read()
            .inspect_err(|err| error!("failed to read channel count: {}", err))
            .unwrap()
    }

    fn sample_rate(&self) -> u32 {
        *self
            .sample_rate
            .read()
            .inspect_err(|err| error!("failed to read sample rate: {}", err))
            .unwrap()
    }

    fn set_volume(&self, volume: f32) {
        self.stream_send
            .send(StreamMsg::SetVolume(volume))
            .inspect_err(|err| {
                error!("failed to send set volume message to audio stream: {}", err);
            })
            .unwrap();
    }

    fn play(&self, source: Box<dyn AudioSource>) {
        self.stream_send
            .send(StreamMsg::Play(source))
            .inspect_err(|err| {
                error!("failed to send play message to audio stream: {}", err);
            })
            .unwrap();
    }

    fn pause(&self) {
        self.stream_send
            .send(StreamMsg::Pause)
            .inspect_err(|err| {
                error!("failed to send pause message to audio stream: {}", err);
            })
            .unwrap();
    }

    fn resume(&self) {
        self.stream_send
            .send(StreamMsg::Resume)
            .inspect_err(|err| {
                error!("failed to send pause message to audio stream: {}", err);
            })
            .unwrap();
    }

    fn stop(&self) {
        self.stream_send
            .send(StreamMsg::Stop)
            .inspect_err(|err| {
                error!("failed to send stop message to audio stream: {}", err);
            })
            .unwrap();
    }

    fn close(&self) {
        self.stream_send
            .send(StreamMsg::Close)
            .inspect_err(|err| {
                error!("failed to send close message to audio stream: {}", err);
            })
            .unwrap();
    }
}

#[derive(Debug)]
struct InterflowError(String);

impl std::error::Error for InterflowError {}

impl Display for InterflowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Interflow output error: {:?}", self.0)
    }
}

impl From<String> for InterflowError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for InterflowError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<InterflowError> for Error {
    fn from(err: InterflowError) -> Self {
        Error::AudioOutputError(Box::new(err))
    }
}

impl<T> From<PoisonError<T>> for InterflowError {
    fn from(err: PoisonError<T>) -> Self {
        InterflowError(format!("Poison error: {}", err))
    }
}
