use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use std::mem::transmute;

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

pub struct CpalOutput {
    _handle: ActorHandle<AudioMessage>,
    sink: CpalSink,
}

struct AtomicF32(AtomicU32);

impl AtomicF32 {
    fn new(value: f32) -> Self {
        Self(AtomicU32::new(unsafe { transmute(value) }))
    }

    fn load(&self, order: Ordering) -> f32 {
        unsafe { transmute(self.0.load(order)) }
    }

    fn store(&self, value: f32, order: Ordering) {
        self.0.store(unsafe { transmute(value) }, order)
    }
}

impl CpalOutput {
    pub fn open() -> Result<Self, Error> {
        let device =
            cpal::default_host()
                .default_output_device()
                .ok_or(Error::AudioOutputError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No output device found",
                ))))?;

        if let Ok(name) = device.name() {
            log::info!("using audio device: {:?}", name);
        }

        let supported = Self::preferred_output_config(&device)?;
        let (sender, receiver) = unbounded();

        let handle = Stream::spawn_with_default_cap("audio_output", {
            let config = supported.config();
            move |this| Stream::open(device, config, receiver, this).unwrap()
        });
        let sink = CpalSink {
            channel_count: supported.channels(),
            sample_rate: supported.sample_rate(),
            sender,
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
    sender: Sender<AudioMessage>,
}

impl CpalSink {
    fn send_message(&self, msg: AudioMessage) {
        if self.sender.send(msg).is_err() {
            log::error!("Audio channel is closed");
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
        self.send_message(AudioMessage::SetVolume(volume));
    }

    fn play(&self, source: impl AudioSource) {
        self.send_message(AudioMessage::SwitchTrack(Box::new(source)));
        self.send_message(AudioMessage::Resume);
    }

    fn pause(&self) {
        self.send_message(AudioMessage::Pause);
    }

    fn resume(&self) {
        self.send_message(AudioMessage::Resume);
    }

    fn stop(&self) {
        self.play(Empty);
        self.pause();
    }

    fn close(&self) {
        self.send_message(AudioMessage::Close);
    }

    fn switch_track(&self, new_source: impl AudioSource) {
        self.send_message(AudioMessage::SwitchTrack(Box::new(new_source)));
    }
}

struct Stream {
    stream: cpal::Stream,
    config: StreamConfig,
    sender: Sender<AudioMessage>,
}

impl Stream {
    fn open(
        device: cpal::Device,
        config: cpal::StreamConfig,
        receiver: Receiver<AudioMessage>,
        stream_send: Sender<AudioMessage>,
    ) -> Result<Self, Error> {
        let mut callback = StreamCallback::new(receiver);

        log::info!("opening output stream: {:?}", config);

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
            sender: stream_send,
        })
    }

    fn handle(&mut self, msg: AudioMessage) -> Result<Act<Self>, Error> {
        match msg {
            AudioMessage::Pause => self.pause_playback(),
            AudioMessage::Resume => self.resume_playback(),
            AudioMessage::Close => {
                log::debug!("closing audio output stream");
                let _ = self.stream.pause();
                Ok(Act::Shutdown)
            }
            AudioMessage::SwitchTrack(_) | AudioMessage::SetVolume(_) => Ok(Act::Continue),
        }
    }

    fn resume_playback(&mut self) -> Result<Act<Self>, Error> {
        log::debug!("resuming audio output stream");
        self.stream.play()?;
        Ok(Act::Continue)
    }

    fn pause_playback(&mut self) -> Result<Act<Self>, Error> {
        log::debug!("pausing audio output stream");
        self.stream.pause()?;
        Ok(Act::Continue)
    }
}

impl Actor for Stream {
    type Message = AudioMessage;
    type Error = Error;

    fn handle(&mut self, msg: Self::Message) -> Result<Act<Self>, Self::Error> {
        self.handle(msg)
    }
}

enum AudioMessage {
    SwitchTrack(Box<dyn AudioSource>),
    Pause,
    Resume,
    SetVolume(f32),
    Close,
}

#[repr(usize)]
enum State {
    Playing,
    Paused,
}

struct StreamCallback {
    receiver: Receiver<AudioMessage>,
    current_source: Box<dyn AudioSource>,
    state: AtomicUsize,
    volume: AtomicF32,
}

impl StreamCallback {
    fn new(receiver: Receiver<AudioMessage>) -> Self {
        Self {
            receiver,
            current_source: Box::new(Empty),
            state: AtomicUsize::new(State::Paused as usize),
            volume: AtomicF32::new(1.0),
        }
    }

    fn write_samples(&mut self, output: &mut [f32]) {
        self.process_messages();

        match State::from_usize(self.state.load(Ordering::Relaxed)) {
            State::Playing => self.write_playing(output),
            State::Paused => output.iter_mut().for_each(|s| *s = 0.0),
        }
    }

    fn write_playing(&mut self, output: &mut [f32]) {
        let written = self.current_source.write(output);
        self.apply_volume(output, written);
    }

    fn apply_volume(&self, output: &mut [f32], written: usize) {
        let volume = self.volume.load(Ordering::Relaxed);
        output[..written].iter_mut().for_each(|s| *s *= volume);
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                AudioMessage::SwitchTrack(new_source) => {
                    self.current_source = new_source;
                    self.state.store(State::Playing as usize, Ordering::Relaxed);
                }
                AudioMessage::Pause => self.state.store(State::Paused as usize, Ordering::Relaxed),
                AudioMessage::Resume => {
                    self.state.store(State::Playing as usize, Ordering::Relaxed)
                }
                AudioMessage::SetVolume(volume) => self.volume.store(volume, Ordering::Relaxed),
                AudioMessage::Close => self.state.store(State::Paused as usize, Ordering::Relaxed),
            }
        }
    }
}

impl State {
    fn from_usize(value: usize) -> Self {
        match value {
            0 => State::Playing,
            1 => State::Paused,
            _ => panic!("Invalid state value"),
        }
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
