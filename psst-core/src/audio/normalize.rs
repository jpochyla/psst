use std::{
    io,
    io::{Read, Seek, SeekFrom},
};

use byteorder::{ReadBytesExt, LE};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NormalizationLevel {
    None,
    Track,
    Album,
}

#[derive(Clone, Copy)]
pub struct NormalizationData {
    track_gain_db: f32,
    track_peak: f32,
    album_gain_db: f32,
    album_peak: f32,
}

impl NormalizationData {
    pub fn parse(mut file: impl Read + Seek) -> io::Result<Self> {
        const NORMALIZATION_OFFSET: u64 = 144;

        file.seek(SeekFrom::Start(NORMALIZATION_OFFSET))?;

        let track_gain_db = file.read_f32::<LE>()?;
        let track_peak = file.read_f32::<LE>()?;
        let album_gain_db = file.read_f32::<LE>()?;
        let album_peak = file.read_f32::<LE>()?;

        Ok(Self {
            track_gain_db,
            track_peak,
            album_gain_db,
            album_peak,
        })
    }

    pub fn factor_for_level(&self, level: NormalizationLevel, pregain: f32) -> f32 {
        match level {
            NormalizationLevel::None => 1.0,
            NormalizationLevel::Track => Self::factor(pregain, self.track_gain_db, self.track_peak),
            NormalizationLevel::Album => Self::factor(pregain, self.album_gain_db, self.album_peak),
        }
    }

    fn factor(pregain: f32, gain: f32, peak: f32) -> f32 {
        let mut nf = f32::powf(10.0, (pregain + gain) / 20.0);
        if nf * peak > 1.0 {
            nf = 1.0 / peak;
        }
        nf
    }
}
