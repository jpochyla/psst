use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

const DEF_BUFFER_SIZE: usize = 8096;
struct LockFreeRingBuffer {
    buffer: Box<[f32]>,
    read_pos: AtomicUsize,
    write_pos: AtomicUsize,
}

impl LockFreeRingBuffer {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size].into_boxed_slice(),
            read_pos: AtomicUsize::new(0),
            write_pos: AtomicUsize::new(0),
        }
    }

    fn write(&mut self, data: &[f32]) -> usize {
        let mut written = 0;
        let mut write_pos = self.write_pos.load(Ordering::Relaxed) as usize;
        for &sample in data {
            self.buffer[write_pos] = sample;
            write_pos = (write_pos + 1) % DEF_BUFFER_SIZE;
            written += 1;
            if write_pos == self.read_pos.load(Ordering::Relaxed) as usize {
                break;
            }
        }
        self.write_pos.store(write_pos, Ordering::Release);
        written
    }

    fn read(&self, data: &mut [f32]) -> usize {
        let mut read = 0;
        let mut read_pos = self.read_pos.load(Ordering::Relaxed) as usize;
        for sample in data.iter_mut() {
            if read_pos == self.write_pos.load(Ordering::Relaxed) as usize {
                break;
            }
            *sample = self.buffer[read_pos];
            read_pos = (read_pos + 1) % DEF_BUFFER_SIZE;
            read += 1;
        }
        self.read_pos.store(read_pos, Ordering::Release);
        read
    }
}

pub struct CpalOutput {
    _handle: ActorHandle<AudioMessage>,
    sink: CpalSink,
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
        let (sender, receiver) = bounded(100);

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

// Implement a state machine for managing playback states
#[derive(Clone, Copy, PartialEq)]
enum PlaybackState {
    Playing = 0,
    Paused = 1,
    Stopped = 2,
}

impl PlaybackState {
    fn from_usize(value: usize) -> Self {
        match value {
            0 => PlaybackState::Playing,
            1 => PlaybackState::Paused,
            _ => PlaybackState::Stopped,
        }
    }
}

struct Stream {
    stream: cpal::Stream,
    config: StreamConfig,
    state: Arc<AtomicUsize>,
    sender: Sender<AudioMessage>,
}

impl Stream {
    fn open(
        device: cpal::Device,
        config: cpal::StreamConfig,
        receiver: Receiver<AudioMessage>,
        stream_send: Sender<AudioMessage>,
    ) -> Result<Self, Error> {
        let state = Arc::new(AtomicUsize::new((PlaybackState::Stopped as u8) as usize));
        let mut callback: StreamCallback =
            StreamCallback::new(receiver, state.clone(), DEF_BUFFER_SIZE);

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
            state,
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
        self.state
            .store((PlaybackState::Playing as u8) as usize, Ordering::SeqCst);
        self.stream.play()?;
        Ok(Act::Continue)
    }

    fn pause_playback(&mut self) -> Result<Act<Self>, Error> {
        log::debug!("pausing audio output stream");
        self.state
            .store((PlaybackState::Paused as u8) as usize, Ordering::SeqCst);
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

struct StreamCallback {
    receiver: Receiver<AudioMessage>,
    current_source: Box<dyn AudioSource>,
    state: Arc<AtomicUsize>,
    volume: f32,
    ring_buffer: LockFreeRingBuffer,
}

impl StreamCallback {
    fn new(receiver: Receiver<AudioMessage>, state: Arc<AtomicUsize>, buffer_size: usize) -> Self {
        Self {
            receiver,
            current_source: Box::new(Empty),
            state,
            volume: 1.0,
            ring_buffer: LockFreeRingBuffer::new(buffer_size),
        }
    }

    fn write_samples(&mut self, output: &mut [f32]) {
        self.process_messages();

        match PlaybackState::from_usize(self.state.load(Ordering::Relaxed)) {
            PlaybackState::Playing => self.write_playing(output),
            PlaybackState::Paused => self.write_paused(output),
            PlaybackState::Stopped => output.iter_mut().for_each(|s| *s = 0.0),
        }
    }

    fn write_playing(&mut self, output: &mut [f32]) {
        let mut temp_buffer = vec![0.0; output.len()];
        let written = self.current_source.write(&mut temp_buffer);
        self.ring_buffer.write(&temp_buffer[..written]);
        let read = self.ring_buffer.read(output);
        self.apply_volume(output, read);
    }

    fn write_paused(&mut self, output: &mut [f32]) {
        let read = self.ring_buffer.read(output);
        self.apply_volume(output, read);
        output[read..].iter_mut().for_each(|s| *s = 0.0);
    }

    fn apply_volume(&self, output: &mut [f32], written: usize) {
        const VOLUME_CHUNK_SIZE: usize = 1024;
        output[..written]
            .chunks_mut(VOLUME_CHUNK_SIZE)
            .for_each(|chunk| {
                chunk.iter_mut().for_each(|sample| *sample *= self.volume);
            });
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                AudioMessage::SwitchTrack(new_source) => {
                    self.current_source = new_source;
                    self.state
                        .store(PlaybackState::Playing as usize, Ordering::Release);
                    self.ring_buffer = LockFreeRingBuffer::new(self.ring_buffer.buffer.len());
                }
                AudioMessage::Pause => self
                    .state
                    .store(PlaybackState::Paused as usize, Ordering::Release),
                AudioMessage::Resume => self
                    .state
                    .store(PlaybackState::Playing as usize, Ordering::Release),
                AudioMessage::SetVolume(volume) => self.volume = volume,
                AudioMessage::Close => self
                    .state
                    .store(PlaybackState::Stopped as usize, Ordering::Release),
            }
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
