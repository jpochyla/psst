use std::{convert::TryFrom, sync::Arc, time::Duration};

use druid::{im::Vector, Data, Lens};
use psst_core::item_id::{ItemId, ItemIdType};
use serde::{Deserialize, Serialize};
use time::{macros::format_description, Date};

use crate::data::{Image, Promise};

use super::album::DatePrecision;

#[derive(Clone, Data, Lens)]
pub struct ShowDetail {
    pub show: Promise<Arc<Show>, ShowLink>,
    pub episodes: Promise<ShowEpisodes, ShowLink>,
}

#[derive(Clone, Data, Lens, Deserialize)]
pub struct Show {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
    pub publisher: Arc<str>,
    pub description: Arc<str>,
}

impl Show {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        Image::at_least_of_size(&self.images, width, height)
    }

    pub fn link(&self) -> ShowLink {
        ShowLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct ShowEpisodes {
    pub show: ShowLink,
    pub episodes: Vector<Arc<Episode>>,
}

#[derive(Clone, Debug, Data, Lens, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ShowLink {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

impl ShowLink {
    pub fn url(&self) -> String {
        format!("https://open.spotify.com/show/{id}", id = self.id)
    }
}

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct Episode {
    pub id: EpisodeId,
    pub name: Arc<str>,
    pub show: ShowLink,
    pub images: Vector<Image>,
    pub description: Arc<str>,
    pub languages: Vector<Arc<str>>,
    #[serde(rename = "duration_ms")]
    #[serde(deserialize_with = "super::utils::deserialize_millis")]
    pub duration: Duration,
    #[serde(deserialize_with = "super::utils::deserialize_date_option")]
    #[data(same_fn = "PartialEq::eq")]
    pub release_date: Option<Date>,
    #[data(same_fn = "PartialEq::eq")]
    pub release_date_precision: Option<DatePrecision>,
    pub resume_point: Option<ResumePoint>,
}

impl Episode {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        Image::at_least_of_size(&self.images, width, height)
    }

    pub fn url(&self) -> String {
        format!(
            "https://open.spotify.com/episode/{id}",
            id = self.id.0.to_base62()
        )
    }

    pub fn release(&self) -> String {
        let format = format_description!("[month repr:short] [day], [year]");
        self.release_date
            .as_ref()
            .map(|date| date.format(format).expect("Invalid format"))
            .unwrap_or_else(|| '-'.to_string())
    }
}

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct EpisodeLink {
    pub id: EpisodeId,
    pub name: Arc<str>,
}

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct ResumePoint {
    pub fully_played: bool,
    #[serde(rename = "resume_position_ms")]
    #[serde(deserialize_with = "super::utils::deserialize_millis")]
    pub resume_position: Duration,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Hash, Deserialize, Serialize)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct EpisodeId(pub ItemId);

impl Data for EpisodeId {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl TryFrom<String> for EpisodeId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ItemId::from_base62(&value, ItemIdType::Podcast)
            .ok_or("Invalid ID")
            .map(Self)
    }
}

impl From<EpisodeId> for String {
    fn from(id: EpisodeId) -> Self {
        id.0.to_base62()
    }
}
