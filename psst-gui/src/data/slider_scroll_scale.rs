use std::f64;

use druid::Lens;

use {
    druid::Data,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, Data, Lens, PartialEq, Serialize, Deserialize)]
pub struct SliderScrollScale {
    // Volume percentage per 'bump' of the wheel(s)
    pub scale: f64,
    // If you have an MX Master, or another mouse with a free wheel, setting this to the
    // number of scroll events that get fired per 'bump' of the wheel will make it
    // change the volume at the same rate as the thumb wheel
    pub y: f64,
    // In case anyone wants it
    pub x: f64,
}

impl Default for SliderScrollScale {
    fn default() -> Self {
        Self {
            scale: 3.0,
            y: 1.0,
            x: 1.0,
        }
    }
}
