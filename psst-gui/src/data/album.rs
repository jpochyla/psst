use crate::data::{Artist, Image, TrackList};
use aspotify::DatePrecision;
use chrono::NaiveDate;
use druid::{im::Vector, Data, Lens};
use itertools::Itertools;
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens)]
pub struct Album {
    pub album_type: AlbumType,
    pub artists: Vector<Artist>,
    pub id: Arc<str>,
    pub images: Vector<Image>,
    pub genres: Vector<Arc<str>>,
    pub copyrights: Vector<Arc<str>>,
    pub label: Arc<str>,
    pub name: Arc<str>,
    #[data(same_fn = "PartialEq::eq")]
    pub release_date: Option<NaiveDate>,
    #[data(same_fn = "PartialEq::eq")]
    pub release_date_precision: Option<DatePrecision>,
    pub tracks: TrackList,
}

impl Album {
    pub fn artist_list(&self) -> String {
        self.artists.iter().map(|artist| &artist.name).join(", ")
    }

    pub fn release(&self) -> String {
        self.format_release_date(match self.release_date_precision {
            Some(DatePrecision::Year) | None => "%Y",
            Some(DatePrecision::Month) => "%B %Y",
            Some(DatePrecision::Day) => "%v",
        })
    }

    pub fn release_year(&self) -> String {
        self.format_release_date("%Y")
    }

    fn format_release_date(&self, format: &str) -> String {
        self.release_date
            .as_ref()
            .map(|date| date.format(format).to_string())
            .unwrap_or_else(|| '-'.to_string())
    }

    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        self.images
            .iter()
            .rev()
            .find(|img| !img.fits(width, height))
            .or_else(|| self.images.back())
    }
}

#[derive(Clone, Debug, Data, Eq, PartialEq)]
pub enum AlbumType {
    Album,
    Single,
    Compilation,
}

impl Default for AlbumType {
    fn default() -> Self {
        Self::Album
    }
}
