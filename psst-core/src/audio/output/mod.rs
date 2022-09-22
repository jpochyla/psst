use crate::audio::source::AudioSource;

#[cfg(feature = "cpal")]
pub mod cpal;
#[cfg(feature = "cubeb")]
pub mod cubeb;

#[cfg(feature = "cubeb")]
pub type DefaultAudioOutput = cubeb::CubebOutput;
#[cfg(feature = "cpal")]
pub type DefaultAudioOutput = cpal::CpalOutput;

pub type DefaultAudioSink = <DefaultAudioOutput as AudioOutput>::Sink;

pub trait AudioOutput {
    type Sink: AudioSink;

    fn sink(&self) -> Self::Sink;
}

pub trait AudioSink {
    fn channel_count(&self) -> usize;
    fn sample_rate(&self) -> u32;
    fn set_volume(&self, volume: f32);
    fn play(&self, source: impl AudioSource);
    fn pause(&self);
    fn resume(&self);
    fn stop(&self);
    fn close(&self);
}
