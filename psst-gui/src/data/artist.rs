use crate::data::Image;
use druid::{im::Vector, Data, Lens};
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens)]
pub struct Artist {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
}

impl Artist {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        self.images
            .iter()
            .rev()
            .find(|img| !img.fits(width, height))
            .or_else(|| self.images.back())
    }
}
