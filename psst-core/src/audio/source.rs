use crate::audio::resample::ResamplingSpec;

use super::resample::{AudioResampler, ResamplingQuality};

/// Types that can produce audio samples in `f32` format. `Send`able across
/// threads.
pub trait AudioSource: Send + 'static {
    /// Write at most of `output.len()` samples into the `output`. Returns the
    /// number of written samples. Should take care to always output a full
    /// frame, and should _never_ block.
    fn write(&mut self, output: &mut [f32]) -> usize;
    fn channel_count(&self) -> usize;
    fn sample_rate(&self) -> u32;
}

/// Empty audio source. Does not produce any samples.
pub struct Empty;

impl AudioSource for Empty {
    fn write(&mut self, _output: &mut [f32]) -> usize {
        0
    }

    fn channel_count(&self) -> usize {
        0
    }

    fn sample_rate(&self) -> u32 {
        0
    }
}

pub struct StereoMappedSource<S> {
    source: S,
    input_channels: usize,
    output_channels: usize,
    buffer: Vec<f32>,
}

impl<S> StereoMappedSource<S>
where
    S: AudioSource,
{
    pub fn new(source: S, output_channels: usize) -> Self {
        const BUFFER_SIZE: usize = 16 * 1024;

        let input_channels = source.channel_count();
        Self {
            source,
            input_channels,
            output_channels,
            buffer: vec![0.0; BUFFER_SIZE],
        }
    }
}

impl<S> AudioSource for StereoMappedSource<S>
where
    S: AudioSource,
{
    fn write(&mut self, output: &mut [f32]) -> usize {
        let input_max = (output.len() / self.output_channels) * self.input_channels;
        let buffer_max = input_max.min(self.buffer.len());
        let written = self.source.write(&mut self.buffer[..buffer_max]);
        let input = &self.buffer[..written];
        let input_frames = input.chunks_exact(self.input_channels);
        let output_frames = output.chunks_exact_mut(self.output_channels);
        for (i, o) in input_frames.zip(output_frames) {
            o[0] = i[0];
            o[1] = i[1];
            // Assume the rest is is implicitly silence.
        }
        output.len()
    }

    fn channel_count(&self) -> usize {
        self.output_channels
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
}

pub struct ResampledSource<S> {
    source: S,
    resampler: AudioResampler,
    inp: Buf,
    out: Buf,
}

impl<S> ResampledSource<S> {
    pub fn new(source: S, output_sample_rate: u32, quality: ResamplingQuality) -> Self
    where
        S: AudioSource,
    {
        const BUFFER_SIZE: usize = 1024;

        let spec = ResamplingSpec {
            channels: source.channel_count(),
            input_rate: source.sample_rate(),
            output_rate: output_sample_rate,
        };
        let inp_buf = vec![0.0; BUFFER_SIZE];
        let out_buf = vec![0.0; spec.output_size(BUFFER_SIZE)];
        Self {
            resampler: AudioResampler::new(quality, spec).unwrap(),
            source,
            inp: Buf {
                buf: inp_buf,
                start: 0,
                end: 0,
            },
            out: Buf {
                buf: out_buf,
                start: 0,
                end: 0,
            },
        }
    }
}

impl<S> AudioSource for ResampledSource<S>
where
    S: AudioSource,
{
    fn write(&mut self, output: &mut [f32]) -> usize {
        let mut total = 0;

        while total < output.len() {
            if self.out.is_empty() {
                if self.inp.is_empty() {
                    let n = self.source.write(&mut self.inp.buf);
                    self.inp.buf[n..].iter_mut().for_each(|s| *s = 0.0);
                    self.inp.start = 0;
                    self.inp.end = self.inp.buf.len();
                }
                let (inp_consumed, out_written) = self
                    .resampler
                    .process(&self.inp.buf[self.inp.start..], &mut self.out.buf)
                    .unwrap();
                self.inp.start += inp_consumed;
                self.out.start = 0;
                self.out.end = out_written;
            }
            let source = self.out.get();
            let target = &mut output[total..];
            let to_write = self.out.len().min(target.len());
            target[..to_write].copy_from_slice(&source[..to_write]);
            total += to_write;
            self.out.start += to_write;
        }

        total
    }

    fn channel_count(&self) -> usize {
        self.resampler.spec.channels
    }

    fn sample_rate(&self) -> u32 {
        self.resampler.spec.output_rate
    }
}

struct Buf {
    buf: Vec<f32>,
    start: usize,
    end: usize,
}

impl Buf {
    fn get(&self) -> &[f32] {
        &self.buf[self.start..self.end]
    }

    fn len(&self) -> usize {
        self.end - self.start
    }

    fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}
