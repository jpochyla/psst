use super::VolumeCtrl;
use crate::player::db_to_ratio;

pub trait MappedCtrl {
    fn to_mapped(&self, volume: u16) -> f64;
    fn as_unmapped(&self, mapped_volume: f64) -> u16;

    fn db_range(&self) -> f64;
    fn set_db_range(&mut self, new_db_range: f64);
    fn range_ok(&self) -> bool;
}

impl MappedCtrl for VolumeCtrl {
    fn to_mapped(&self, volume: u16) -> f64 {
        // More than just an optimization, this ensures that zero volume is
        // really mute (both the log and cubic equations would otherwise not
        // reach zero).
        if volume == 0 {
            return 0.0;
        } else if volume == Self::MAX_VOLUME {
            // And limit in case of rounding errors (as is the case for log).
            return 1.0;
        }

        let normalized_volume = volume as f64 / Self::MAX_VOLUME as f64;
        let mapped_volume = if self.range_ok() {
            match *self {
                Self::Cubic(db_range) => {
                    CubicMapping::linear_to_mapped(normalized_volume, db_range)
                }
                Self::Log(db_range) => LogMapping::linear_to_mapped(normalized_volume, db_range),
                _ => normalized_volume,
            }
        } else {
            // Ensure not to return -inf or NaN due to division by zero.
            error!("{self:?} does not work with 0 dB range, using linear mapping instead");
            normalized_volume
        };

        debug!(
            "Input volume {} mapped to: {:.2}%",
            volume,
            mapped_volume * 100.0
        );

        mapped_volume
    }

    fn as_unmapped(&self, mapped_volume: f64) -> u16 {
        // More than just an optimization, this ensures that zero mapped volume
        // is unmapped to non-negative real numbers (otherwise the log and cubic
        // equations would respectively return -inf and -1/9.)
        if f64::abs(mapped_volume - 0.0) <= f64::EPSILON {
            return 0;
        } else if f64::abs(mapped_volume - 1.0) <= f64::EPSILON {
            return Self::MAX_VOLUME;
        }

        let unmapped_volume = if self.range_ok() {
            match *self {
                Self::Cubic(db_range) => CubicMapping::mapped_to_linear(mapped_volume, db_range),
                Self::Log(db_range) => LogMapping::mapped_to_linear(mapped_volume, db_range),
                _ => mapped_volume,
            }
        } else {
            // Ensure not to return -inf or NaN due to division by zero.
            error!("{self:?} does not work with 0 dB range, using linear mapping instead");
            mapped_volume
        };

        (unmapped_volume * Self::MAX_VOLUME as f64) as u16
    }

    fn db_range(&self) -> f64 {
        match *self {
            Self::Fixed => 0.0,
            Self::Linear => Self::DEFAULT_DB_RANGE, // arbitrary, could be anything > 0
            Self::Log(db_range) | Self::Cubic(db_range) => db_range,
        }
    }

    fn set_db_range(&mut self, new_db_range: f64) {
        match self {
            Self::Cubic(db_range) | Self::Log(db_range) => *db_range = new_db_range,
            _ => error!("Invalid to set dB range for volume control type {self:?}"),
        }

        debug!("Volume control is now {self:?}")
    }

    fn range_ok(&self) -> bool {
        self.db_range() > 0.0 || matches!(self, Self::Fixed | Self::Linear)
    }
}

pub trait VolumeMapping {
    fn linear_to_mapped(unmapped_volume: f64, db_range: f64) -> f64;
    fn mapped_to_linear(mapped_volume: f64, db_range: f64) -> f64;
}

// Volume conversion taken from: https://www.dr-lex.be/info-stuff/volumecontrols.html#ideal2
//
// As the human auditory system has a logarithmic sensitivity curve, this
// mapping results in a near linear loudness experience with the listener.
pub struct LogMapping {}
impl VolumeMapping for LogMapping {
    fn linear_to_mapped(normalized_volume: f64, db_range: f64) -> f64 {
        let (db_ratio, ideal_factor) = Self::coefficients(db_range);
        f64::exp(ideal_factor * normalized_volume) / db_ratio
    }

    fn mapped_to_linear(mapped_volume: f64, db_range: f64) -> f64 {
        let (db_ratio, ideal_factor) = Self::coefficients(db_range);
        f64::ln(db_ratio * mapped_volume) / ideal_factor
    }
}

impl LogMapping {
    fn coefficients(db_range: f64) -> (f64, f64) {
        let db_ratio = db_to_ratio(db_range);
        let ideal_factor = f64::ln(db_ratio);
        (db_ratio, ideal_factor)
    }
}

// Ported from: https://github.com/alsa-project/alsa-utils/blob/master/alsamixer/volume_mapping.c
// which in turn was inspired by: https://www.robotplanet.dk/audio/audio_gui_design/
//
// Though this mapping is computationally less expensive than the logarithmic
// mapping, it really does not matter as librespot memoizes the mapped value.
// Use this mapping if you have some reason to mimic Alsa's native mixer or
// prefer a more granular control in the upper volume range.
//
// Note: https://www.dr-lex.be/info-stuff/volumecontrols.html#ideal3 shows
// better approximations to the logarithmic curve but because we only intend
// to mimic Alsa here, we do not implement them. If your desire is to use a
// logarithmic mapping, then use that volume control.
pub struct CubicMapping {}
impl VolumeMapping for CubicMapping {
    fn linear_to_mapped(normalized_volume: f64, db_range: f64) -> f64 {
        let min_norm = Self::min_norm(db_range);
        f64::powi(normalized_volume * (1.0 - min_norm) + min_norm, 3)
    }

    fn mapped_to_linear(mapped_volume: f64, db_range: f64) -> f64 {
        let min_norm = Self::min_norm(db_range);
        (mapped_volume.powf(1.0 / 3.0) - min_norm) / (1.0 - min_norm)
    }
}

impl CubicMapping {
    fn min_norm(db_range: f64) -> f64 {
        // Note that this 60.0 is unrelated to DEFAULT_DB_RANGE.
        // Instead, it's the cubic voltage to dB ratio.
        f64::powf(10.0, -db_range / 60.0)
    }
}
