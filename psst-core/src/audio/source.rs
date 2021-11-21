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

pub struct StereoMapper<S> {
    source: S,
    input_channels: usize,
    output_channels: usize,
    buffer: Vec<f32>,
}

impl<S> StereoMapper<S> {
    pub fn new(
        source: S,
        input_channels: usize,
        output_channels: usize,
        max_input_size: usize,
    ) -> Self {
        Self {
            source,
            input_channels,
            output_channels,
            buffer: vec![0.0; (max_input_size / input_channels) * output_channels],
        }
    }

    fn input_size(&mut self, output_size: usize) -> usize {
        (output_size / self.output_channels) * self.input_channels
    }
}

impl<S> AudioSource for StereoMapper<S>
where
    S: AudioSource,
{
    fn write(&mut self, output: &mut [f32]) -> usize {
        let input_max = self.input_size(output.len()).min(self.buffer.len());
        let written = self.source.write(&mut self.buffer[..input_max]);
        let input = &self.buffer[..written];
        let input_frames = input.chunks_exact(self.input_channels);
        let output_frames = output.chunks_exact_mut(self.output_channels);
        for (i, o) in input_frames.zip(output_frames) {
            o[0] = i[0];
            o[1] = i[1];
        }
        output.len()
    }
}
