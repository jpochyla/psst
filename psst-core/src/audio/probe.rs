use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use symphonia::core::codecs::CodecType;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::{Hint, Probe};
use symphonia::default::formats::{Mp3Reader, OggReader};

use crate::error::Error;

pub struct TrackProbe {
    pub codec: CodecType,
    pub duration: Option<Duration>,
}

macro_rules! probe_err {
    ($message:tt) => {
        // This is necessary to work around the fact that the two impls for From<&str> are:
        //   Box<dyn std::error::Error>
        //   Box<dyn std::error::Error + Send + Sync>
        // And the trait bound on our error is:
        //   Box<dyn std::error::Error + Send>
        // Normally we could just do `$message.into()`, but no impl exists for exactly
        // `Error + Send`, so we have to be explicit about which we want to use.
        Error::AudioProbeError(Box::<dyn std::error::Error + Send + Sync>::from($message))
    };
}

impl TrackProbe {
    pub fn new(path: &PathBuf) -> Result<Self, Error> {
        // Register all supported file formats for detection.
        let mut probe = Probe::default();
        probe.register_all::<Mp3Reader>();
        probe.register_all::<OggReader>();

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let file = File::open(path)?;
        let mss_opts = MediaSourceStreamOptions::default();
        let mss = MediaSourceStream::new(Box::new(file), mss_opts);

        let fmt_opts = FormatOptions::default();
        let meta_opts = MetadataOptions::default();
        let probe_result = probe
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|_| probe_err!("failed to probe file"))?;
        let track = probe_result
            .format
            .default_track()
            .ok_or(probe_err!("file contained no tracks"))?;
        let params = &track.codec_params;

        let duration =
            if let (Some(time_base), Some(n_frames)) = (params.time_base, params.n_frames) {
                let time = time_base.calc_time(n_frames);
                let secs = time.seconds;
                let ms = (time.frac * 1_000.0).round() as u64;
                Some(Duration::from_millis(secs * 1_000 + ms))
            } else {
                None
            };

        Ok(Self {
            codec: params.codec,
            duration,
        })
    }
}
