use crate::dither::{Ditherer, DithererBuilder};
use zerocopy::{Immutable, IntoBytes};

#[derive(Immutable, IntoBytes, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
#[repr(transparent)]
pub struct i24([u8; 3]);
impl i24 {
    fn from_s24(sample: i32) -> Self {
        // trim the padding in the most significant byte
        #[allow(unused_variables)]
        let [a, b, c, d] = sample.to_ne_bytes();
        #[cfg(target_endian = "little")]
        return Self([a, b, c]);
        #[cfg(target_endian = "big")]
        return Self([b, c, d]);
    }
}

pub struct Converter {
    ditherer: Option<Box<dyn Ditherer>>,
}

impl Converter {
    pub fn new(dither_config: Option<DithererBuilder>) -> Self {
        match dither_config {
            Some(ditherer_builder) => {
                let ditherer = (ditherer_builder)();
                info!("Converting with ditherer: {}", ditherer.name());
                Self {
                    ditherer: Some(ditherer),
                }
            }
            None => Self { ditherer: None },
        }
    }

    /// Base bit positions for PCM format scaling. These represent the position
    /// of the most significant bit in each format's full-scale representation.
    /// For signed integers in two's complement, full scale is 2^(bits-1).
    const SHIFT_S16: u8 = 15; // 16-bit: 2^15 = 32768
    const SHIFT_S24: u8 = 23; // 24-bit: 2^23 = 8388608  
    const SHIFT_S32: u8 = 31; // 32-bit: 2^31 = 2147483648

    /// Additional bit shifts needed to scale from 16-bit to higher bit depths.
    /// These are the differences between the base shift amounts above.
    const SHIFT_16_TO_24: u8 = Self::SHIFT_S24 - Self::SHIFT_S16; // 23 - 15 = 8
    const SHIFT_16_TO_32: u8 = Self::SHIFT_S32 - Self::SHIFT_S16; // 31 - 15 = 16

    /// Pre-calculated scale factor for 24-bit clamping bounds
    const SCALE_S24: f64 = (1_u64 << Self::SHIFT_S24) as f64;

    /// Scale audio samples with optimal dithering strategy for Spotify's 16-bit source material.
    ///
    /// Since Spotify audio is always 16-bit depth, this function:
    /// 1. When dithering: applies noise at 16-bit level, preserves fractional precision,
    ///    then scales to target format and rounds once at the end
    /// 2. When not dithering: scales directly from normalized float to target format
    ///
    /// The `shift` parameter specifies how many extra bits to shift beyond
    /// the base 16-bit scaling (0 for 16-bit, 8 for 24-bit, 16 for 32-bit).
    #[inline]
    pub fn scale(&mut self, sample: f64, shift: u8) -> f64 {
        match self.ditherer.as_mut() {
            Some(d) => {
                // With dithering: Apply noise at 16-bit level to address original quantization,
                // then scale up to target format while preserving sub-LSB information
                let dithered_16bit = sample * (1_u64 << Self::SHIFT_S16) as f64 + d.noise();
                let scaled = dithered_16bit * (1_u64 << shift) as f64;
                scaled.round()
            }
            None => {
                // No dithering: Scale directly from normalized float to target format
                // using a single bit shift operation (base 16-bit shift + additional shift)
                let total_shift = Self::SHIFT_S16 + shift;
                (sample * (1_u64 << total_shift) as f64).round()
            }
        }
    }

    /// Clamping scale specifically for 24-bit output to prevent MSB overflow.
    /// Only used for S24 formats where samples are packed in 32-bit words.
    /// Ensures the most significant byte is zero to prevent overflow during dithering.
    #[inline]
    pub fn clamping_scale_s24(&mut self, sample: f64) -> f64 {
        let int_value = self.scale(sample, Self::SHIFT_16_TO_24);

        // In two's complement, there are more negative than positive values.
        let min = -Self::SCALE_S24;
        let max = Self::SCALE_S24 - 1.0;

        int_value.clamp(min, max)
    }

    #[inline]
    pub fn f64_to_f32(&mut self, samples: &[f64]) -> Vec<f32> {
        samples.iter().map(|sample| *sample as f32).collect()
    }

    #[inline]
    pub fn f64_to_s32(&mut self, samples: &[f64]) -> Vec<i32> {
        samples
            .iter()
            .map(|sample| self.scale(*sample, Self::SHIFT_16_TO_32) as i32)
            .collect()
    }

    /// S24 is 24-bit PCM packed in an upper 32-bit word
    #[inline]
    pub fn f64_to_s24(&mut self, samples: &[f64]) -> Vec<i32> {
        samples
            .iter()
            .map(|sample| self.clamping_scale_s24(*sample) as i32)
            .collect()
    }

    /// S24_3 is 24-bit PCM in a 3-byte array
    #[inline]
    pub fn f64_to_s24_3(&mut self, samples: &[f64]) -> Vec<i24> {
        samples
            .iter()
            .map(|sample| i24::from_s24(self.clamping_scale_s24(*sample) as i32))
            .collect()
    }

    #[inline]
    pub fn f64_to_s16(&mut self, samples: &[f64]) -> Vec<i16> {
        samples
            .iter()
            .map(|sample| self.scale(*sample, 0) as i16)
            .collect()
    }
}
