use super::VolumeGetter;
use super::{MappedCtrl, VolumeCtrl};
use super::{Mixer, MixerConfig};
use librespot_core::Error;
use portable_atomic::AtomicU64;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(Clone)]
pub struct SoftMixer {
    // There is no AtomicF64, so we store the f64 as bits in a u64 field.
    // It's much faster than a Mutex<f64>.
    volume: Arc<AtomicU64>,
    volume_ctrl: VolumeCtrl,
}

impl Mixer for SoftMixer {
    fn open(config: MixerConfig) -> Result<Self, Error> {
        let volume_ctrl = config.volume_ctrl;
        info!("Mixing with softvol and volume control: {volume_ctrl:?}");

        Ok(Self {
            volume: Arc::new(AtomicU64::new(f64::to_bits(0.5))),
            volume_ctrl,
        })
    }

    fn volume(&self) -> u16 {
        let mapped_volume = f64::from_bits(self.volume.load(Ordering::Relaxed));
        self.volume_ctrl.as_unmapped(mapped_volume)
    }

    fn set_volume(&self, volume: u16) {
        let mapped_volume = self.volume_ctrl.to_mapped(volume);
        self.volume
            .store(mapped_volume.to_bits(), Ordering::Relaxed)
    }

    fn get_soft_volume(&self) -> Box<dyn VolumeGetter + Send> {
        Box::new(SoftVolume(self.volume.clone()))
    }
}

impl SoftMixer {
    pub const NAME: &'static str = "softvol";
}

struct SoftVolume(Arc<AtomicU64>);

impl VolumeGetter for SoftVolume {
    #[inline]
    fn attenuation_factor(&self) -> f64 {
        f64::from_bits(self.0.load(Ordering::Relaxed))
    }
}
