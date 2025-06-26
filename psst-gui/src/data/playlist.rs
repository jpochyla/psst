use std::sync::Arc;

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Deserializer, Serialize};

use crate::data::utils::sanitize_html_string;
use crate::data::{user::PublicUser, Image, Promise, Track, TrackId};

#[derive(Clone, Debug, Data, Lens)]
pub struct PlaylistDetail {
    pub playlist: Promise<Playlist, PlaylistLink>,
    pub tracks: Promise<PlaylistTracks, PlaylistLink>,
}

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct PlaylistAddTrack {
    pub link: PlaylistLink,
    pub track_id: TrackId,
}

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct PlaylistRemoveTrack {
    pub link: PlaylistLink,
    pub track_pos: usize,
}

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct Playlist {
    pub id: Arc<str>,
    pub name: Arc<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vector<Image>>,
    #[serde(deserialize_with = "deserialize_description")]
    pub description: Arc<str>,
    #[serde(rename = "tracks")]
    #[serde(deserialize_with = "deserialize_track_count")]
    pub track_count: Option<usize>,
    pub owner: PublicUser,
    pub collaborative: bool,
    #[serde(rename = "public")]
    pub public: Option<bool>,
}

impl Playlist {
    pub fn link(&self) -> PlaylistLink {
        PlaylistLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }

    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        self.images
            .as_ref()
            .and_then(|images| Image::at_least_of_size(images, width, height))
    }

    pub fn url(&self) -> String {
        format!("https://open.spotify.com/playlist/{id}", id = self.id)
    }
}

#[derive(Clone, Debug, Data, Lens)]
pub struct PlaylistTracks {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub tracks: Vector<Arc<Track>>,
}

impl PlaylistTracks {
    pub fn link(&self) -> PlaylistLink {
        PlaylistLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Data, Lens, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct PlaylistLink {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

fn deserialize_track_count<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct PlaylistTracksRef {
        total: Option<usize>,
    }

    Ok(PlaylistTracksRef::deserialize(deserializer)?.total)
}

fn deserialize_description<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let description: String = String::deserialize(deserializer)?;
    Ok(sanitize_html_string(&description))
}
