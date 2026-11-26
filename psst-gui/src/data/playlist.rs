use std::sync::Arc;

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Deserializer, Serialize};

use crate::data::utils::sanitize_html_string;
use crate::data::{user::PublicUser, Image, Promise, Track, TrackId};

#[derive(Clone, Debug, Data, Lens)]
pub struct PlaylistDetail {
    pub query: String,
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
    pub track_uri: Arc<str>,
}

#[derive(Clone, Debug, Data)]
pub struct PlaylistReorder {
    pub link: PlaylistLink,
    pub track_id: TrackId,
    pub position: usize,
    pub down: bool,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Playlist {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Option<Vector<Image>>,
    pub description: Arc<str>,
    pub track_count: Option<usize>,
    pub owner: PublicUser,
    pub collaborative: bool,
    pub public: Option<bool>,
    pub snapshot_id: Option<Arc<str>>,
}

impl<'de> Deserialize<'de> for Playlist {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Count {
            total: Option<usize>,
        }
        #[derive(Deserialize)]
        struct Payload {
            id: Arc<str>,
            name: Arc<str>,
            images: Option<Vector<Image>>,
            #[serde(deserialize_with = "deserialize_description")]
            description: Arc<str>,
            items: Option<Count>,
            tracks: Option<Count>,
            owner: PublicUser,
            collaborative: bool,
            public: Option<bool>,
            snapshot_id: Option<Arc<str>>,
        }
        let p = Payload::deserialize(deserializer)?;
        Ok(Self {
            id: p.id,
            name: p.name,
            images: p.images,
            description: p.description,
            track_count: p
                .items
                .and_then(|c| c.total)
                .or_else(|| p.tracks.and_then(|c| c.total)),
            owner: p.owner,
            collaborative: p.collaborative,
            public: p.public,
            snapshot_id: p.snapshot_id,
        })
    }
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
    pub query: String,
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

fn deserialize_description<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let description: String = String::deserialize(deserializer)?;
    Ok(sanitize_html_string(&description))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playlist_counts_support_current_legacy_and_combined_payloads() {
        let base = serde_json::json!({"id":"p", "name":"Playlist", "description":"", "owner":{"id":"u", "display_name":"User"}, "collaborative":false, "public":false});
        for (items, tracks, expected) in [
            (Some(42), None, Some(42)),
            (None, Some(12), Some(12)),
            (Some(42), Some(12), Some(42)),
            (None, None, None),
        ] {
            let mut value = base.clone();
            if let Some(total) = items {
                value["items"] = serde_json::json!({"total":total});
            }
            if let Some(total) = tracks {
                value["tracks"] = serde_json::json!({"total":total});
            }
            let playlist: Playlist = serde_json::from_value(value).unwrap();
            assert_eq!(playlist.track_count, expected);
        }
    }
}
