use std::{env, ffi::CString, ops::Deref};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

pub struct CubebOutput {
    #[allow(unused)]
    handle: ActorHandle<StreamMsg>,
    sink: CubebSink,
}

impl CubebOutput {
    pub fn open() -> Result<Self, Error> {
        let (callback_send, callback_recv) = bounded(16);

        let handle = Stream::spawn_with_default_cap("audio_output", {
            move |_| Stream::open(callback_recv).unwrap()
        });
        let sink = CubebSink {
            callback_send,
            stream_send: handle.sender(),
        };

        Ok(Self { handle, sink })
    }
}

impl AudioOutput for CubebOutput {
    type Sink = CubebSink;

    fn sink(&self) -> Self::Sink {
        self.sink.clone()
    }
}

type Frame = cubeb::StereoFrame<f32>;

const STREAM_CHANNELS: usize = 2;
const SAMPLE_RATE: u32 = 44_100;
const STREAM_LATENCY: u32 = 0x1000;

struct Stream {
    #[allow(unused)]
    ctx: cubeb::Context,
    stream: cubeb::Stream<Frame>,
}

impl Stream {
    fn open(callback_recv: Receiver<CallbackMsg>) -> Result<Self, Error> {
        let backend_name = env::var("CUBEB_BACKEND")
            .ok()
            .and_then(|s| CString::new(s).ok());
        let ctx_name = CString::new("Psst").ok();
        let ctx = cubeb::Context::init(ctx_name.as_deref(), backend_name.as_deref())?;

        let mut callback = StreamCallback {
            callback_recv,
            source: Box::new(Empty),
            state: CallbackState::Paused,
            buffer: vec![0.0; 1024 * 1024],
        };

        let params = cubeb::StreamParamsBuilder::new()
            .format(cubeb::SampleFormat::Float32NE)
            .rate(SAMPLE_RATE)
            .channels(STREAM_CHANNELS as u32)
            .layout(cubeb::ChannelLayout::STEREO)
            .take();

        let mut builder = cubeb::StreamBuilder::new();
        builder
            .name("Psst")
            .default_output(&params)
            .latency(STREAM_LATENCY)
            .data_callback(move |_, output| {
                callback.write_samples(output);
                output.len() as isize
            })
            .state_callback(|state| {
                log::debug!("stream state: {:?}", state);
            });
        let stream = builder.init(&ctx)?;

        Ok(Self { ctx, stream })
    }
}

enum StreamMsg {
    Pause,
    Resume,
    Close,
    SetVolume(f32),
}

impl Actor for Stream {
    type Message = StreamMsg;
    type Error = Error;

    fn handle(&mut self, msg: Self::Message) -> Result<Act<Self>, Self::Error> {
        match msg {
            StreamMsg::Pause => {
                log::debug!("pausing audio output stream");
                if let Err(err) = self.stream.stop() {
                    log::error!("failed to stop stream: {}", err);
                }
                Ok(Act::Continue)
            }
            StreamMsg::Resume => {
                log::debug!("resuming audio output stream");
                if let Err(err) = self.stream.start() {
                    log::error!("failed to start stream: {}", err);
                }
                Ok(Act::Continue)
            }
            StreamMsg::Close => {
                log::debug!("closing audio output stream");
                let _ = self.stream.stop();
                Ok(Act::Shutdown)
            }
            StreamMsg::SetVolume(volume) => {
                log::debug!("setting volume");
                if let Err(err) = self.stream.set_volume(volume) {
                    log::error!("failed to set volume: {}", err);
                }
                Ok(Act::Continue)
            }
        }
    }
}

#[derive(Clone)]
pub struct CubebSink {
    callback_send: Sender<CallbackMsg>,
    stream_send: Sender<StreamMsg>,
}

impl AudioSink for CubebSink {
    fn channel_count(&self) -> usize {
        STREAM_CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn set_volume(&self, volume: f32) {
        self.stream_send.send(StreamMsg::SetVolume(volume)).unwrap();
    }

    fn play(&self, source: impl AudioSource) {
        self.callback_send
            .send(CallbackMsg::PlaySource(Box::new(source)))
            .unwrap()
    }

    fn pause(&self) {
        self.callback_send.send(CallbackMsg::Pause).unwrap();
        self.stream_send.send(StreamMsg::Pause).unwrap();
    }

    fn resume(&self) {
        self.callback_send.send(CallbackMsg::Resume).unwrap();
        self.stream_send.send(StreamMsg::Resume).unwrap();
    }

    fn stop(&self) {
        self.pause();
    }

    fn close(&self) {
        self.stop();
    }
}

enum CallbackMsg {
    PlaySource(Box<dyn AudioSource>),
    Pause,
    Resume,
}

enum CallbackState {
    Playing,
    Paused,
}

struct StreamCallback {
    callback_recv: Receiver<CallbackMsg>,
    source: Box<dyn AudioSource>,
    state: CallbackState,
    buffer: Vec<f32>,
}

impl StreamCallback {
    fn write_samples(&mut self, output: &mut [Frame]) {
        // Process any pending data messages.
        while let Ok(msg) = self.callback_recv.try_recv() {
            match msg {
                CallbackMsg::PlaySource(src) => {
                    self.source = src;
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
            let n_output_frames = output.len();
            let n_output_samples = n_output_frames * STREAM_CHANNELS;
            let n_samples = self.source.write(&mut self.buffer[..n_output_samples]);
            let mut n_frames = 0;
            for (i, o) in self.buffer[..n_samples]
                .chunks(STREAM_CHANNELS)
                .zip(output.iter_mut())
            {
                o.l = i[0];
                o.r = i[1];
                n_frames += 1;
            }
            n_frames
        } else {
            0
        };

        // Mute any remaining samples.
        output[written..].iter_mut().for_each(|s| {
            s.l = 0.0;
            s.r = 0.0;
        });
    }
}

unsafe impl Sync for StreamCallback {}

impl From<cubeb::Error> for Error {
    fn from(err: cubeb::Error) -> Self {
        Error::AudioOutputError(Box::new(err))
    }
}
