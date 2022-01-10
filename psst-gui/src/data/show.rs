use std::{sync::Arc, time::Duration};

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Serialize};
use time::Date;

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

impl ShowEpisodes {
    pub fn link(&self) -> ShowLink {
        self.show.clone()
    }
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

#[derive(Clone, Data, Lens, Deserialize)]
pub struct Episode {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
    pub description: Arc<str>,
    pub languages: Vector<Arc<str>>,
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
}

#[derive(Clone, Data, Lens, Deserialize)]
pub struct ResumePoint {
    pub fully_played: bool,
    #[serde(rename = "resume_position_ms")]
    #[serde(deserialize_with = "super::utils::deserialize_millis")]
    pub resume_position: Duration,
}
