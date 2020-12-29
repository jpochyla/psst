use crate::data::{Album, Artist, AudioDuration};
use druid::{im::Vector, Data, Lens};
use psst_core::item_id::{ItemId, ItemIdType};
use std::{ops::Deref, str::FromStr, sync::Arc};

#[derive(Clone, Debug, Data, Lens)]
pub struct Track {
    pub id: TrackId,
    pub name: Arc<str>,
    pub album: Option<Album>,
    pub artists: Vector<Artist>,
    pub duration: AudioDuration,
    pub disc_number: usize,
    pub track_number: usize,
    pub explicit: bool,
    pub is_local: bool,
    pub is_playable: Option<bool>,
    pub popularity: Option<u32>,
}

impl Track {
    pub fn artist_name(&self) -> String {
        self.artists
            .front()
            .map(|artist| artist.name.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn album_name(&self) -> String {
        self.album
            .as_ref()
            .map(|album| album.name.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn url(&self) -> String {
        format!(
            "https://open.spotify.com/track/{id}",
            id = self.id.to_base62()
        )
    }
}

pub const LOCAL_TRACK_ID: TrackId = TrackId(ItemId::new(0u128, ItemIdType::Unknown));

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TrackId(ItemId);

impl Data for TrackId {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Deref for TrackId {
    type Target = ItemId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ItemId> for TrackId {
    fn from(id: ItemId) -> Self {
        Self(id)
    }
}

impl FromStr for TrackId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(id) = ItemId::from_base62(s, ItemIdType::Track) {
            Ok(Self(id))
        } else {
            Err(())
        }
    }
}
