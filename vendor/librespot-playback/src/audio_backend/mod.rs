use crate::config::AudioFormat;
use crate::convert::Converter;
use crate::decoder::AudioPacket;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SinkError {
    #[error("Audio Sink Error Not Connected: {0}")]
    NotConnected(String),
    #[error("Audio Sink Error Connection Refused: {0}")]
    ConnectionRefused(String),
    #[error("Audio Sink Error On Write: {0}")]
    OnWrite(String),
    #[error("Audio Sink Error Invalid Parameters: {0}")]
    InvalidParams(String),
    #[error("Audio Sink Error Changing State: {0}")]
    StateChange(String),
}

pub type SinkResult<T> = Result<T, SinkError>;

pub trait Open {
    fn open(_: Option<String>, format: AudioFormat) -> Self;
}

pub trait Sink {
    fn start(&mut self) -> SinkResult<()> {
        Ok(())
    }
    fn stop(&mut self) -> SinkResult<()> {
        Ok(())
    }
    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()>;
}

pub type SinkBuilder = fn(Option<String>, AudioFormat) -> Box<dyn Sink>;

pub trait SinkAsBytes {
    fn write_bytes(&mut self, data: &[u8]) -> SinkResult<()>;
}

fn mk_sink<S: Sink + Open + 'static>(device: Option<String>, format: AudioFormat) -> Box<dyn Sink> {
    Box::new(S::open(device, format))
}

// reuse code for various backends
macro_rules! sink_as_bytes {
    () => {
        #[inline]
        fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
            use crate::convert::i24;
            use zerocopy::IntoBytes;
            match packet {
                AudioPacket::Samples(samples) => match self.format {
                    AudioFormat::F64 => self.write_bytes(samples.as_bytes()),
                    AudioFormat::F32 => {
                        let samples_f32: &[f32] = &converter.f64_to_f32(&samples);
                        self.write_bytes(samples_f32.as_bytes())
                    }
                    AudioFormat::S32 => {
                        let samples_s32: &[i32] = &converter.f64_to_s32(&samples);
                        self.write_bytes(samples_s32.as_bytes())
                    }
                    AudioFormat::S24 => {
                        let samples_s24: &[i32] = &converter.f64_to_s24(&samples);
                        self.write_bytes(samples_s24.as_bytes())
                    }
                    AudioFormat::S24_3 => {
                        let samples_s24_3: &[i24] = &converter.f64_to_s24_3(&samples);
                        self.write_bytes(samples_s24_3.as_bytes())
                    }
                    AudioFormat::S16 => {
                        let samples_s16: &[i16] = &converter.f64_to_s16(&samples);
                        self.write_bytes(samples_s16.as_bytes())
                    }
                },
                AudioPacket::Raw(samples) => self.write_bytes(&samples),
            }
        }
    };
}

#[cfg(feature = "alsa-backend")]
mod alsa;
#[cfg(feature = "alsa-backend")]
use self::alsa::AlsaSink;

#[cfg(feature = "portaudio-backend")]
mod portaudio;
#[cfg(feature = "portaudio-backend")]
use self::portaudio::PortAudioSink;

#[cfg(feature = "pulseaudio-backend")]
mod pulseaudio;
#[cfg(feature = "pulseaudio-backend")]
use self::pulseaudio::PulseAudioSink;

#[cfg(feature = "jackaudio-backend")]
mod jackaudio;
#[cfg(feature = "jackaudio-backend")]
use self::jackaudio::JackSink;

#[cfg(feature = "gstreamer-backend")]
mod gstreamer;
#[cfg(feature = "gstreamer-backend")]
use self::gstreamer::GstreamerSink;

#[cfg(any(feature = "rodio-backend", feature = "rodiojack-backend"))]
mod rodio;
#[cfg(feature = "rodio-backend")]
use self::rodio::RodioSink;

#[cfg(feature = "sdl-backend")]
mod sdl;
#[cfg(feature = "sdl-backend")]
use self::sdl::SdlSink;

mod pipe;
use self::pipe::StdoutSink;

mod subprocess;
use self::subprocess::SubprocessSink;

pub const BACKENDS: &[(&str, SinkBuilder)] = &[
    #[cfg(feature = "rodio-backend")]
    (RodioSink::NAME, rodio::mk_rodio), // default goes first
    #[cfg(feature = "alsa-backend")]
    (AlsaSink::NAME, mk_sink::<AlsaSink>),
    #[cfg(feature = "portaudio-backend")]
    (PortAudioSink::NAME, mk_sink::<PortAudioSink<'_>>),
    #[cfg(feature = "pulseaudio-backend")]
    (PulseAudioSink::NAME, mk_sink::<PulseAudioSink>),
    #[cfg(feature = "jackaudio-backend")]
    (JackSink::NAME, mk_sink::<JackSink>),
    #[cfg(feature = "gstreamer-backend")]
    (GstreamerSink::NAME, mk_sink::<GstreamerSink>),
    #[cfg(feature = "rodiojack-backend")]
    ("rodiojack", rodio::mk_rodiojack),
    #[cfg(feature = "sdl-backend")]
    (SdlSink::NAME, mk_sink::<SdlSink>),
    (StdoutSink::NAME, mk_sink::<StdoutSink>),
    (SubprocessSink::NAME, mk_sink::<SubprocessSink>),
];

pub fn find(name: Option<String>) -> Option<SinkBuilder> {
    if let Some(name) = name {
        BACKENDS
            .iter()
            .find(|backend| name == backend.0)
            .map(|backend| backend.1)
    } else {
        BACKENDS.first().map(|backend| backend.1)
    }
}
