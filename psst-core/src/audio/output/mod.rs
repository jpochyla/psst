use std::fmt::Display;

use crate::{audio::source::AudioSource, error::Error};

#[cfg(feature = "cpal")]
pub mod cpal;
#[cfg(feature = "cubeb")]
pub mod cubeb;
#[cfg(feature = "interflow")]
pub mod interflow;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AudioBackend {
    #[cfg(feature = "cpal")]
    Cpal,
    #[cfg(feature = "cubeb")]
    Cubeb,
    #[cfg(feature = "interflow")]
    Interflow,
}

pub const AUDIO_BACKENDS : &[AudioBackend] = &[
    #[cfg(feature = "interflow")]
    AudioBackend::Interflow,
    #[cfg(feature = "cpal")]
    AudioBackend::Cpal,
    #[cfg(feature = "cubeb")]
    AudioBackend::Cubeb,
];

impl AudioBackend {
    pub fn open(&self) -> Result<Box<dyn AudioOutput>, Error> {
        match self {
            #[cfg(feature = "cpal")]
            AudioBackend::Cpal => cpal::CpalOutput::open(),
            #[cfg(feature = "cubeb")]
            AudioBackend::Cubeb => cubeb::CubebOutput::open(),
            #[cfg(feature = "interflow")]
            AudioBackend::Interflow => interflow::InterflowOutput::open(),
            #[allow(unreachable_patterns)]
            _ => panic!("no audio output backend is available"),
        }
    }
}

impl Default for AudioBackend {
    fn default() -> Self {
        AUDIO_BACKENDS[0]
    }
}

impl Display for AudioBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "cpal")]
            AudioBackend::Cpal => write!(f, "CPAL"),
            #[cfg(feature = "cubeb")]
            AudioBackend::Cubeb => write!(f, "cubeb"),
            #[cfg(feature = "interflow")]
            AudioBackend::Interflow => write!(f, "Interflow"),
        }
    }
}

pub trait AudioOutput: Send + 'static {
    fn sink(&self) -> Box<dyn AudioSink>;
}

pub trait AudioSink: Send + 'static {
    fn channel_count(&self) -> usize;
    fn sample_rate(&self) -> u32;
    fn set_volume(&self, volume: f32);
    fn play(&self, source: Box<dyn AudioSource>);
    fn pause(&self);
    fn resume(&self);
    fn stop(&self);
    fn close(&self);
}
