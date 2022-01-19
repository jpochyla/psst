use {
    druid::Data,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, Data, PartialEq, Serialize, Deserialize)]
pub struct SliderScrollScale {
    // Volume percentage per 'bump' of the wheel(s)
    scale: Option<f64>,
    // If you have an MX Master, or another mouse with a free wheel, setting this to the
    // number of scroll events that get fired per 'bump' of the wheel will make it
    // change the volume at the same rate as the thumb wheel
    y: Option<f64>,
    // In case anyone wants it
    x: Option<f64>,
}
impl SliderScrollScale {
    #[inline(always)]
    pub fn scale(&self) -> f64 {
        self.scale.unwrap_or(5f64)
    }

    #[inline(always)]
    pub fn x(&self) -> f64 {
        self.x.unwrap_or(1f64)
    }

    #[inline(always)]
    pub fn y(&self) -> f64 {
        self.y.unwrap_or(1f64)
    }
}
impl Default for SliderScrollScale {
    fn default() -> Self {
        Self {
            scale: Some(5.),
            y: Some(1.),
            x: Some(1.),
        }
    }
}
