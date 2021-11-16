use crate::error::Error;

#[derive(Copy, Clone)]
pub enum ResamplingQuality {
    SincBestQuality = libsamplerate::SRC_SINC_BEST_QUALITY as isize,
    SincMediumQuality = libsamplerate::SRC_SINC_MEDIUM_QUALITY as isize,
    SincFastest = libsamplerate::SRC_SINC_FASTEST as isize,
    ZeroOrderHold = libsamplerate::SRC_ZERO_ORDER_HOLD as isize,
    Linear = libsamplerate::SRC_LINEAR as isize,
}

#[derive(Copy, Clone)]
pub struct ResamplingSpec {
    pub from_rate: usize,
    pub to_rate: usize,
    pub channels: usize,
}

impl ResamplingSpec {
    pub fn max_output_size(&self, max_input_size: usize) -> usize {
        (self.ratio() * max_input_size as f64 * 1.2) as usize
    }

    pub fn ratio(&self) -> f64 {
        self.to_rate as f64 / self.from_rate as f64
    }
}

pub struct AudioResampler {
    pub spec: ResamplingSpec,
    state: *mut libsamplerate::SRC_STATE,
}

impl AudioResampler {
    pub fn new(quality: ResamplingQuality, spec: ResamplingSpec) -> Result<Self, Error> {
        let mut error_int = 0i32;
        let state = unsafe {
            libsamplerate::src_new(
                quality as i32,
                spec.channels as i32,
                &mut error_int as *mut i32,
            )
        };
        if error_int != 0 {
            Err(Error::ResamplingError(error_int))
        } else {
            Ok(Self { state, spec })
        }
    }

    pub fn resample(&mut self, input: &[f32], output: &mut [f32]) -> Result<usize, Error> {
        if self.spec.from_rate == self.spec.to_rate {
            // Bypass conversion completely in case the sample rates are equal.
            let output = &mut output[..input.len()];
            output.copy_from_slice(input);
            return Ok(output.len());
        }
        let mut src = libsamplerate::SRC_DATA {
            data_in: input.as_ptr(),
            data_out: output.as_mut_ptr(),
            input_frames: (input.len() / self.spec.channels) as _,
            output_frames: (output.len() / self.spec.channels) as _,
            src_ratio: self.spec.ratio(),
            end_of_input: 0, // TODO: Use this.
            input_frames_used: 0,
            output_frames_gen: 0,
        };
        let error_int = unsafe { libsamplerate::src_process(self.state, &mut src as *mut _) };
        if error_int != 0 {
            Err(Error::ResamplingError(error_int))
        } else {
            let output_len = src.output_frames_gen as usize * self.spec.channels;
            let processed_len = src.input_frames_used as usize * self.spec.channels;
            if processed_len != input.len() {
                log::warn!("skipping frames while resampling");
            }
            Ok(output_len)
        }
    }
}

impl Drop for AudioResampler {
    fn drop(&mut self) {
        unsafe { libsamplerate::src_delete(self.state) };
    }
}
