use crate::error::Error;

#[derive(Copy, Clone)]
pub enum ResamplingAlgo {
    SincBestQuality = libsamplerate::SRC_SINC_BEST_QUALITY as isize,
    SincMediumQuality = libsamplerate::SRC_SINC_MEDIUM_QUALITY as isize,
    SincFastest = libsamplerate::SRC_SINC_FASTEST as isize,
    ZeroOrderHold = libsamplerate::SRC_ZERO_ORDER_HOLD as isize,
    Linear = libsamplerate::SRC_LINEAR as isize,
}

pub struct ResamplingSpec {
    pub from_rate: usize,
    pub to_rate: usize,
    pub channels: usize,
}

impl ResamplingSpec {
    fn ratio(&self) -> f64 {
        self.to_rate as f64 / self.from_rate as f64
    }
}

pub struct AudioResampler {
    state: *mut libsamplerate::SRC_STATE,
    spec: ResamplingSpec,
    output: Vec<f32>,
}

impl AudioResampler {
    pub fn new(algo: ResamplingAlgo, spec: ResamplingSpec, capacity: usize) -> Result<Self, Error> {
        let mut error_int = 0i32;
        let state = unsafe {
            libsamplerate::src_new(
                algo as i32,
                spec.channels as i32,
                &mut error_int as *mut i32,
            )
        };
        if error_int != 0 {
            Err(Error::ResamplingError(error_int))
        } else {
            Ok(Self {
                state,
                spec,
                output: vec![0.0; capacity],
            })
        }
    }

    pub fn resample(&mut self, inter_input: &[f32]) -> Result<&mut [f32], Error> {
        if self.spec.from_rate == self.spec.to_rate {
            // Bypass conversion completely in case the sample rates are equal.
            let output = &mut self.output[..inter_input.len()];
            output.copy_from_slice(inter_input);
            return Ok(output);
        }
        let mut src = libsamplerate::SRC_DATA {
            data_in: inter_input.as_ptr(),
            data_out: self.output.as_mut_ptr(),
            input_frames: (inter_input.len() / self.spec.channels) as _,
            output_frames: (self.output.len() / self.spec.channels) as _,
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
            if processed_len != inter_input.len() {
                log::warn!("skipping frames while resampling");
            }
            Ok(&mut self.output[..output_len])
        }
    }
}

impl Drop for AudioResampler {
    fn drop(&mut self) {
        unsafe { libsamplerate::src_delete(self.state) };
    }
}
