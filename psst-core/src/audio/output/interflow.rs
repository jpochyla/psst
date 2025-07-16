use std::{
    fmt::{Debug, Display, Formatter},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        Arc, PoisonError, RwLock,
    },
};

use crossbeam_channel::Sender;
use interflow::{
    channel_map::Bitset, prelude::default_output_device, AudioCallbackContext, AudioDevice,
    AudioOutputDevice,
};
use log::{debug, error, info};

use crate::{
    actor::{Act, Actor},
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

const DEFAULT_CHANNEL_COUNT: usize = 2;
const DEFAULT_SAMPLE_RATE: u32 = 44_100;

pub struct InterflowOutput {
    sink: Box<InterflowSink>,
}

impl InterflowOutput {
    pub fn open() -> Result<Box<dyn AudioOutput>, Error> {
        info!("opening audio output: interflow");
        let mut stream = InterflowStream::new();
        let channel_count = stream.channel_count.clone();
        let sample_rate = stream.sample_rate.clone();
        let stream_handle = InterflowStream::spawn_with_default_cap("audio_output", move |_| {
            stream.open().unwrap()
        });
        let sink = Box::new(InterflowSink {
            stream_send: stream_handle.sender(),
            channel_count,
            sample_rate,
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
    opened: Arc<AtomicBool>,
    channel_count: Arc<AtomicUsize>,
    sample_rate: Arc<AtomicU32>,
    playing: Arc<AtomicBool>,
    volume: Arc<AtomicF32>,
    source: Arc<RwLock<Box<dyn AudioSource>>>,
}

impl InterflowStream {
    pub fn new() -> Self {
        info!("creating stream");
        Self {
            opened: Arc::new(AtomicBool::new(false)),
            channel_count: Arc::new(AtomicUsize::new(DEFAULT_CHANNEL_COUNT)),
            sample_rate: Arc::new(AtomicU32::new(DEFAULT_SAMPLE_RATE)),
            playing: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicF32::new(0.0)),
            source: Arc::new(RwLock::new(Box::new(Empty))),
        }
    }

    pub fn open(&mut self) -> Result<Self, Error> {
        info!("opening stream");
        if self.opened.load(Ordering::SeqCst) {
            info!("stream is already opened");
            return Ok(self.clone());
        }

        let callback = InterflowCallback {
            playing: self.playing.clone(),
            volume: self.volume.clone(),
            source: self.source.clone(),
        };
        // NOTE: AudioOutputDevice is not dyn compatible, hard to keep a reference to it.
        let device = default_output_device();
        let mut config = device
            .default_output_config()
            .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        // NOTE: Channels is a bitset of channels, not a count.
        config.channels = 0b11; // Stereo
        config.samplerate = DEFAULT_SAMPLE_RATE as f64;
        if !device.is_config_supported(&config) {
            error!("device does not support config: {:?}", config);
            config = device
                .default_output_config()
                .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        }
        info!(
            "config channel count: {} config sample rate: {}",
            config.channels, config.samplerate
        );
        // NOTE: Stream handle is a generic type on AudioOutputDevice, hard to keep a reference to it.
        // We want to save the stream so that we can eject it on close for graceful shutdown.
        let _stream = device
            .create_output_stream(config, callback)
            .map(Box::new)
            .map_err(|err| InterflowError::from(format!("{:?}", err)))?;
        self.set_channel_count(config.channels.count())?;
        self.set_sample_rate(config.samplerate as u32)?;
        self.opened.store(true, Ordering::SeqCst);
        return Ok(self.clone());
    }

    pub fn close(&mut self) -> Result<Self, Error> {
        info!("closing stream");
        if !self.opened.load(Ordering::SeqCst) {
            info!("stream is already closed");
            return Ok(self.clone());
        }
        self.set_playing(false)?;
        self.set_source(Box::new(Empty))?;
        self.opened.store(false, Ordering::SeqCst);
        Ok(self.clone())
    }

    pub fn set_playing(&mut self, playing: bool) -> Result<Self, Error> {
        debug!("setting playing to {}", playing);
        self.playing.store(playing, Ordering::SeqCst);
        Ok(self.clone())
    }

    pub fn set_channel_count(&mut self, channel_count: usize) -> Result<Self, Error> {
        debug!("setting channel count to {}", channel_count);
        self.channel_count.store(channel_count, Ordering::SeqCst);
        Ok(self.clone())
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<Self, Error> {
        debug!("setting sample rate to {}", sample_rate);
        self.sample_rate.store(sample_rate, Ordering::SeqCst);
        Ok(self.clone())
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<Self, Error> {
        debug!("setting volume to {}", volume);
        self.volume.store(volume, Ordering::SeqCst);
        Ok(self.clone())
    }

    pub fn set_source(&mut self, source: Box<dyn AudioSource>) -> Result<Self, Error> {
        debug!("setting source");
        self.source
            .write()
            .map(|mut v| *v = source)
            .map_err(|err| Error::AudioOutputError(Box::new(InterflowError::from(err))))?;
        Ok(self.clone())
    }
}

impl Actor for InterflowStream {
    type Message = StreamMsg;
    type Error = Error;

    fn handle(&mut self, msg: StreamMsg) -> Result<Act<Self>, Self::Error> {
        match msg {
            StreamMsg::Open => {
                debug!("opening stream");
                self.open()?;
                Ok(Act::Continue)
            }
            StreamMsg::Close => {
                debug!("closing stream");
                self.close()?;
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
    playing: Arc<AtomicBool>,
    volume: Arc<AtomicF32>,
    source: Arc<RwLock<Box<dyn AudioSource>>>,
}

impl interflow::AudioOutputCallback for InterflowCallback {
    fn on_output_data(
        &mut self,
        _context: AudioCallbackContext,
        mut output: interflow::AudioOutput<f32>,
    ) {
        if self.playing.load(Ordering::SeqCst) {
            let mut buf = vec![0.0; output.buffer.num_samples() * output.buffer.num_channels()];
            let _written = self.source.write().unwrap().write(&mut buf);
            let _res = output.buffer.copy_from_interleaved(&buf);
            output
                .buffer
                .change_amplitude(self.volume.load(Ordering::SeqCst));
        } else {
            output.buffer.change_amplitude(0 as f32);
        }
    }
}

#[derive(Clone)]
struct InterflowSink {
    channel_count: Arc<AtomicUsize>,
    sample_rate: Arc<AtomicU32>,
    stream_send: Sender<StreamMsg>,
}

impl AudioSink for InterflowSink {
    fn channel_count(&self) -> usize {
        self.channel_count.load(Ordering::SeqCst)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate.load(Ordering::SeqCst)
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

struct AtomicF32 {
    storage: AtomicU32,
}

impl AtomicF32 {
    fn new(value: f32) -> Self {
        Self {
            storage: AtomicU32::new(value.to_bits()),
        }
    }

    fn store(&self, value: f32, ordering: Ordering) {
        self.storage.store(value.to_bits(), ordering)
    }

    fn load(&self, ordering: Ordering) -> f32 {
        f32::from_bits(self.storage.load(ordering))
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
