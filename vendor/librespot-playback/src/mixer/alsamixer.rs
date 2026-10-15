use crate::player::{db_to_ratio, ratio_to_db};

use super::mappings::{LogMapping, MappedCtrl, VolumeMapping};
use super::{Mixer, MixerConfig, VolumeCtrl};

use alsa::Error as AlsaError;
use alsa::ctl::{ElemId, ElemIface};
use alsa::mixer::{MilliBel, SelemChannelId, SelemId};
use alsa::{Ctl, Round};

use librespot_core::Error;
use std::ffi::{CString, NulError};
use thiserror::Error;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AlsaMixer {
    config: MixerConfig,
    min: i64,
    max: i64,
    range: i64,
    min_db: f64,
    max_db: f64,
    db_range: f64,
    has_switch: bool,
    is_softvol: bool,
    use_linear_in_db: bool,
}

// min_db cannot be depended on to be mute. Also note that contrary to
// its name copied verbatim from Alsa, this is in millibel scale.
const SND_CTL_TLV_DB_GAIN_MUTE: MilliBel = MilliBel(-9999999);
const ZERO_DB: MilliBel = MilliBel(0);

#[derive(Debug, Error)]
enum AlsaMixerError {
    #[error("Could not open Alsa mixer. {0}")]
    CouldNotOpen(AlsaError),
    #[error("Could not find Alsa mixer control")]
    CouldNotFindController,
    #[error("Could not open Alsa softvol with that device. {0}")]
    CouldNotOpenWithDevice(AlsaError),
    #[error("Could not open Alsa softvol with that name. {0}")]
    CouldNotOpenWithName(NulError),
    #[error("Could not get Alsa softvol dB range. {0}")]
    NoDbRange(AlsaError),
    #[error("Could not convert Alsa raw volume to dB volume. {0}")]
    CouldNotConvertRaw(AlsaError),
}

impl From<AlsaMixerError> for Error {
    fn from(value: AlsaMixerError) -> Self {
        Error::failed_precondition(value)
    }
}

impl Mixer for AlsaMixer {
    fn open(config: MixerConfig) -> Result<Self, Error> {
        info!(
            "Mixing with Alsa and volume control: {:?} for device: {} with mixer control: {},{}",
            config.volume_ctrl, config.device, config.control, config.index,
        );

        let mut config = config; // clone

        let mixer =
            alsa::mixer::Mixer::new(&config.device, false).map_err(AlsaMixerError::CouldNotOpen)?;
        let simple_element = mixer
            .find_selem(&SelemId::new(&config.control, config.index))
            .ok_or(AlsaMixerError::CouldNotFindController)?;

        // Query capabilities
        let has_switch = simple_element.has_playback_switch();
        let is_softvol = simple_element
            .get_playback_vol_db(SelemChannelId::mono())
            .is_err();

        // Query raw volume range
        let (min, max) = simple_element.get_playback_volume_range();
        let range = i64::abs(max - min);

        // Query dB volume range -- note that Alsa exposes a different
        // API for hardware and software mixers
        let (min_millibel, max_millibel) = if is_softvol {
            let control =
                Ctl::new(&config.device, false).map_err(AlsaMixerError::CouldNotOpenWithDevice)?;
            let mut element_id = ElemId::new(ElemIface::Mixer);
            element_id.set_name(
                &CString::new(config.control.as_str())
                    .map_err(AlsaMixerError::CouldNotOpenWithName)?,
            );
            element_id.set_index(config.index);
            let (min_millibel, mut max_millibel) = control
                .get_db_range(&element_id)
                .map_err(AlsaMixerError::NoDbRange)?;

            // Alsa can report incorrect maximum volumes due to rounding
            // errors. e.g. Alsa rounds [-60.0..0.0] in range [0..255] to
            // step size 0.23. Then multiplying 0.23 by 255 incorrectly
            // returns a dB range of 58.65 instead of 60 dB, from
            // [-60.00..-1.35]. This workaround checks the default case
            // where the maximum dB volume is expected to be 0, and cannot
            // cover all cases.
            if max_millibel != ZERO_DB {
                warn!("Alsa mixer reported maximum dB != 0, which is suspect");
                let reported_step_size = (max_millibel - min_millibel).0 / range;
                let assumed_step_size = (ZERO_DB - min_millibel).0 / range;
                if reported_step_size == assumed_step_size {
                    warn!(
                        "Alsa rounding error detected, setting maximum dB to {:.2} instead of {:.2}",
                        ZERO_DB.to_db(),
                        max_millibel.to_db()
                    );
                    max_millibel = ZERO_DB;
                } else {
                    warn!("Please manually set `--volume-range` if this is incorrect");
                }
            }
            (min_millibel, max_millibel)
        } else {
            let (mut min_millibel, max_millibel) = simple_element.get_playback_db_range();

            // Some controls report that their minimum volume is mute, instead
            // of their actual lowest dB setting before that.
            if min_millibel == SND_CTL_TLV_DB_GAIN_MUTE && min < max {
                debug!("Alsa mixer reported minimum dB as mute, trying workaround");
                min_millibel = simple_element
                    .ask_playback_vol_db(min + 1)
                    .map_err(AlsaMixerError::CouldNotConvertRaw)?;
            }
            (min_millibel, max_millibel)
        };

        let min_db = min_millibel.to_db() as f64;
        let max_db = max_millibel.to_db() as f64;
        let reported_db_range = f64::abs(max_db - min_db);

        // Synchronize the volume control dB range with the mixer control,
        // unless it was already set with a command line option.
        let db_range = if config.volume_ctrl.range_ok() {
            let db_range_override = config.volume_ctrl.db_range();
            if db_range_override.is_normal() {
                db_range_override
            } else {
                reported_db_range
            }
        } else {
            config.volume_ctrl.set_db_range(reported_db_range);
            reported_db_range
        };

        if reported_db_range == db_range {
            debug!("Alsa dB volume range was reported as {}", reported_db_range);
            if reported_db_range > 100.0 {
                debug!("Alsa mixer reported dB range > 100, which is suspect");
                debug!("Please manually set `--volume-range` if this is incorrect");
            }
        } else {
            debug!(
                "Alsa dB volume range was reported as {} but overridden to {}",
                reported_db_range, db_range
            );
        }

        // For hardware controls with a small range (24 dB or less),
        // force using the dB API with a linear mapping.
        let mut use_linear_in_db = false;
        if !is_softvol && db_range <= 24.0 {
            use_linear_in_db = true;
            config.volume_ctrl = VolumeCtrl::Linear;
        }

        debug!("Alsa mixer control is softvol: {}", is_softvol);
        debug!("Alsa support for playback (mute) switch: {}", has_switch);
        debug!("Alsa raw volume range: [{}..{}] ({})", min, max, range);
        debug!(
            "Alsa dB volume range: [{:.2}..{:.2}] ({:.2})",
            min_db, max_db, db_range
        );
        debug!("Alsa forcing linear dB mapping: {}", use_linear_in_db);

        Ok(Self {
            config,
            min,
            max,
            range,
            min_db,
            max_db,
            db_range,
            has_switch,
            is_softvol,
            use_linear_in_db,
        })
    }

    fn volume(&self) -> u16 {
        let mixer =
            alsa::mixer::Mixer::new(&self.config.device, false).expect("Could not open Alsa mixer");
        let simple_element = mixer
            .find_selem(&SelemId::new(&self.config.control, self.config.index))
            .expect("Could not find Alsa mixer control");

        if self.switched_off() {
            return 0;
        }

        let mut mapped_volume = if self.is_softvol {
            let raw_volume = simple_element
                .get_playback_volume(SelemChannelId::mono())
                .expect("Could not get raw Alsa volume");
            raw_volume as f64 / self.range as f64 - self.min as f64
        } else {
            let db_volume = simple_element
                .get_playback_vol_db(SelemChannelId::mono())
                .expect("Could not get Alsa dB volume")
                .to_db() as f64;

            if self.use_linear_in_db {
                (db_volume - self.min_db) / self.db_range
            } else if f64::abs(db_volume - SND_CTL_TLV_DB_GAIN_MUTE.to_db() as f64) <= f64::EPSILON
            {
                0.0
            } else {
                db_to_ratio(db_volume - self.max_db)
            }
        };

        // see comment in `set_volume` why we are handling an antilog volume
        if mapped_volume > 0.0 && self.is_some_linear() {
            mapped_volume = LogMapping::linear_to_mapped(mapped_volume, self.db_range);
        }

        self.config.volume_ctrl.as_unmapped(mapped_volume)
    }

    fn set_volume(&self, volume: u16) {
        let mixer =
            alsa::mixer::Mixer::new(&self.config.device, false).expect("Could not open Alsa mixer");
        let simple_element = mixer
            .find_selem(&SelemId::new(&self.config.control, self.config.index))
            .expect("Could not find Alsa mixer control");

        if self.has_switch {
            if volume == 0 {
                debug!("Disabling playback (setting mute) on Alsa");
                simple_element
                    .set_playback_switch_all(0)
                    .expect("Could not disable playback (set mute) on Alsa");
            } else if self.switched_off() {
                debug!("Enabling playback (unsetting mute) on Alsa");
                simple_element
                    .set_playback_switch_all(1)
                    .expect("Could not enable playback (unset mute) on Alsa");
            }
        }

        let mut mapped_volume = self.config.volume_ctrl.to_mapped(volume);

        // Alsa's linear algorithms map everything onto log. Alsa softvol does
        // this internally. In the case of `use_linear_in_db` this happens
        // automatically by virtue of the dB scale. This means that linear
        // controls become log, log becomes log-on-log, and so on. To make
        // the controls work as expected, perform an antilog calculation to
        // counteract what Alsa will be doing to the set volume.
        if mapped_volume > 0.0 && self.is_some_linear() {
            mapped_volume = LogMapping::mapped_to_linear(mapped_volume, self.db_range);
        }

        if self.is_softvol {
            let scaled_volume = (self.min as f64 + mapped_volume * self.range as f64) as i64;
            debug!("Setting Alsa raw volume to {}", scaled_volume);
            simple_element
                .set_playback_volume_all(scaled_volume)
                .expect("Could not set Alsa raw volume");
            return;
        }

        let db_volume = if self.use_linear_in_db {
            self.min_db + mapped_volume * self.db_range
        } else if volume == 0 {
            // prevent ratio_to_db(0.0) from returning -inf
            SND_CTL_TLV_DB_GAIN_MUTE.to_db() as f64
        } else {
            ratio_to_db(mapped_volume) + self.max_db
        };

        debug!("Setting Alsa volume to {:.2} dB", db_volume);
        simple_element
            .set_playback_db_all(MilliBel::from_db(db_volume as f32), Round::Floor)
            .expect("Could not set Alsa dB volume");
    }
}

impl AlsaMixer {
    pub const NAME: &'static str = "alsa";

    fn switched_off(&self) -> bool {
        if !self.has_switch {
            return false;
        }

        let mixer =
            alsa::mixer::Mixer::new(&self.config.device, false).expect("Could not open Alsa mixer");
        let simple_element = mixer
            .find_selem(&SelemId::new(&self.config.control, self.config.index))
            .expect("Could not find Alsa mixer control");

        simple_element
            .get_playback_switch(SelemChannelId::mono())
            .map(|playback| playback == 0)
            .unwrap_or(false)
    }

    fn is_some_linear(&self) -> bool {
        self.is_softvol || self.use_linear_in_db
    }
}
