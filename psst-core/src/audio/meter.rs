//! Small, allocation-free PCM analyzer for the playback bar. No Web API reads.
use std::{
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        OnceLock,
    },
    time::Instant,
};

pub const BAND_COUNT: usize = 8;
static LEVELS: [AtomicU32; BAND_COUNT] = [const { AtomicU32::new(0) }; BAND_COUNT];
static UPDATED_MS: AtomicU64 = AtomicU64::new(0);
static CLOCK: OnceLock<Instant> = OnceLock::new();

fn clock_ms() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

/// Stale output (disconnected device, buffering or stopped stream) is silence.
pub fn levels() -> [f32; BAND_COUNT] {
    let updated = UPDATED_MS.load(Ordering::Acquire);
    if updated == 0 || clock_ms().saturating_sub(updated) > 250 {
        return [0.0; BAND_COUNT];
    }
    std::array::from_fn(|i| f32::from_bits(LEVELS[i].load(Ordering::Relaxed)))
}

pub struct AudioMeter {
    coefficients: [f32; BAND_COUNT - 1],
    low_pass: [[f32; BAND_COUNT - 1]; 2],
    power: [f32; BAND_COUNT],
    channels: usize,
    frames: u32,
    window_frames: u32,
}

impl AudioMeter {
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        // Initialize the clock outside the real-time callback.
        clock_ms();
        let rate = sample_rate.max(1) as f32;
        let cutoffs = [80.0_f32, 200.0, 500.0, 1200.0, 3000.0, 6500.0, 11000.0];
        Self {
            coefficients: cutoffs
                .map(|hz| 1.0 - (-std::f32::consts::TAU * hz.min(rate * 0.45) / rate).exp()),
            low_pass: [[0.0; BAND_COUNT - 1]; 2],
            power: [0.0; BAND_COUNT],
            channels: channels.max(1),
            frames: 0,
            window_frames: (sample_rate / 30).max(1),
        }
    }

    pub fn process(&mut self, samples: &[f32]) {
        for frame in samples.chunks_exact(self.channels) {
            // Analyze stereo channels independently to retain opposite-phase audio.
            for (channel, &sample) in frame.iter().take(2).enumerate() {
                let sample = if sample.is_finite() {
                    sample.clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                let mut previous = 0.0;
                for band in 0..BAND_COUNT - 1 {
                    let low = &mut self.low_pass[channel][band];
                    *low += self.coefficients[band] * (sample - *low);
                    let value = *low - previous;
                    self.power[band] += value * value;
                    previous = *low;
                }
                let high = sample - previous;
                self.power[BAND_COUNT - 1] += high * high;
            }
            self.frames += 1;
            if self.frames >= self.window_frames {
                let levels = self.finish_window();
                for (slot, level) in LEVELS.iter().zip(levels) {
                    slot.store(level.to_bits(), Ordering::Relaxed);
                }
                UPDATED_MS.store(clock_ms(), Ordering::Release);
            }
        }
    }

    fn finish_window(&mut self) -> [f32; BAND_COUNT] {
        let count = (self.frames.max(1) as usize * self.channels.min(2)) as f32;
        let levels = self.power.map(|power| {
            let rms = (power / count).sqrt();
            // Log scale makes quiet passages readable while genuine silence stays zero.
            ((20.0 * rms.max(1e-6).log10() + 66.0) / 66.0).clamp(0.0, 1.0)
        });
        self.power.fill(0.0);
        self.frames = 0;
        levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frequency: f32, amplitude: f32, stereo: bool) -> [f32; BAND_COUNT] {
        let mut meter = AudioMeter::new(48_000, if stereo { 2 } else { 1 });
        // Keep publication out of this deterministic test; inspect its own analyzer.
        meter.window_frames = 100_000;
        for frame in 0..4800 {
            let sample =
                amplitude * (std::f32::consts::TAU * frequency * frame as f32 / 48_000.0).sin();
            if stereo {
                meter.process(&[sample, -sample]);
            } else {
                meter.process(&[sample]);
            }
        }
        meter.finish_window()
    }

    #[test]
    fn bands_follow_audio_frequency_and_preserve_opposite_phase_stereo() {
        let bass = tone(60.0, 0.5, false);
        let treble = tone(8000.0, 0.5, false);
        assert!(bass[0] > treble[0] + 0.25);
        assert!(treble[6] > bass[6] + 0.25);
        let stereo = tone(60.0, 0.5, true);
        for (mono, stereo) in bass.into_iter().zip(stereo) {
            assert!((mono - stereo).abs() < 0.001);
        }
        assert!(tone(60.0, 0.05, false)[0] < bass[0]);
    }

    #[test]
    fn silence_invalid_samples_and_partial_frames_are_safe() {
        let mut meter = AudioMeter::new(48_000, 2);
        meter.process(&[f32::NAN, f32::INFINITY, 0.0, 0.0, 1.0]);
        assert_eq!(meter.frames, 2);
        assert_eq!(meter.finish_window(), [0.0; BAND_COUNT]);
        assert_eq!(meter.finish_window(), [0.0; BAND_COUNT]);
    }
}
