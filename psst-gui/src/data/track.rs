use crate::data::{Album, Artist, AudioDuration, Navigation};
use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use psst_core::item_id::{ItemId, ItemIdType};
use std::{ops::Deref, str::FromStr, sync::Arc};

#[derive(Clone, Debug, Data)]
pub enum TrackOrigin {
    Library,
    Album(Arc<str>),
    Artist(Arc<str>),
    Playlist(Arc<str>),
    Search(String),
}

impl TrackOrigin {
    pub fn as_nav(&self) -> Navigation {
        match &self {
            TrackOrigin::Library => Navigation::Library,
            TrackOrigin::Album(id) => Navigation::AlbumDetail(id.to_owned()),
            TrackOrigin::Artist(id) => Navigation::ArtistDetail(id.to_owned()),
            TrackOrigin::Playlist(_) => todo!(),
            TrackOrigin::Search(_) => todo!(),
        }
    }
}

#[derive(Clone, Debug, Data)]
pub struct TrackList {
    pub origin: TrackOrigin,
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Debug, Data)]
pub struct TrackCtx {
    pub playback_item: Option<Arc<Track>>,
    pub saved_tracks: HashSet<TrackId>,
}

impl TrackCtx {
    pub fn is_track_playing(&self, track: &Track) -> bool {
        self.playback_item
            .as_ref()
            .map(|t| t.id.same(&track.id))
            .unwrap_or(false)
    }

    pub fn is_track_saved(&self, track: &Track) -> bool {
        self.saved_tracks.contains(&track.id)
    }

    pub fn set_saved_tracks(&mut self, tracks: &Vector<Arc<Track>>) {
        self.saved_tracks = tracks.iter().map(|track| track.id).collect();
    }
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Track {
    pub id: TrackId,
    pub album: Option<Album>,
    pub artists: Vector<Artist>,
    pub disc_number: usize,
    pub duration: AudioDuration,
    pub explicit: bool,
    pub is_local: bool,
    pub is_playable: Option<bool>,
    pub name: Arc<str>,
    pub popularity: Option<u32>,
    pub track_number: usize,
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

    pub fn link(&self) -> String {
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
