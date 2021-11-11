use rb::{Consumer, RbConsumer};

/// Types that can produce audio samples in `f32` format. `Send`able across
/// threads.
pub trait AudioSource: Send + 'static {
    /// Write at most of `output.len()` samples into the `output`. Returns the
    /// number of written samples. Should take care to always output a full
    /// frame, and should _never_ block.
    fn write(&mut self, output: &mut [f32]) -> usize;
}

/// Ring-buffers of `f32` can act as an audio source, provided the producer
/// always pushes in whole frames and we don't block on reading.
impl AudioSource for Consumer<f32> {
    fn write(&mut self, output: &mut [f32]) -> usize {
        self.read(output).unwrap_or(0)
    }
}

/// Empty audio source. Does not produce any samples.
pub struct Empty;

impl AudioSource for Empty {
    fn write(&mut self, _output: &mut [f32]) -> usize {
        0
    }
}
