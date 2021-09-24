use std::{fmt, sync::Arc, time::Duration};

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Serialize};

use super::{AlbumLink, ArtistLink, Library, Nav, PlaylistLink, RecommendationsRequest, Track};

#[derive(Clone, Data, Lens)]
pub struct Playback {
    pub state: PlaybackState,
    pub now_playing: Option<NowPlaying>,
    pub queue_behavior: QueueBehavior,
    pub queue: Vector<QueuedTrack>,
    pub volume: f64,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct QueuedTrack {
    pub track: Arc<Track>,
    pub origin: PlaybackOrigin,
}

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueueBehavior {
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

impl Default for QueueBehavior {
    fn default() -> Self {
        QueueBehavior::Sequential
    }
}

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq)]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Data, Lens)]
pub struct NowPlaying {
    pub item: Arc<Track>,
    pub origin: PlaybackOrigin,
    pub progress: Duration,

    // Although keeping a ref to the `Library` here is a bit of a hack, it dramatically
    // simplifies displaying the track context menu in the playback bar.
    pub library: Arc<Library>,
}

impl NowPlaying {
    pub fn cover_image_url(&self, width: f64, height: f64) -> Option<&str> {
        self.item_album()
            .and_then(|album| album.image(width, height))
            .map(|image| image.url.as_ref())
    }

    pub fn item_album(&self) -> Option<&AlbumLink> {
        self.item.album.as_ref().or_else(|| match &self.origin {
            PlaybackOrigin::Album(album) => Some(album),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, Data)]
pub enum PlaybackOrigin {
    Library,
    Album(AlbumLink),
    Artist(ArtistLink),
    Playlist(PlaylistLink),
    Search(Arc<str>),
    Recommendations(Arc<RecommendationsRequest>),
}

impl PlaybackOrigin {
    pub fn to_nav(&self) -> Nav {
        match &self {
            PlaybackOrigin::Library => Nav::SavedTracks,
            PlaybackOrigin::Album(link) => Nav::AlbumDetail(link.clone()),
            PlaybackOrigin::Artist(link) => Nav::ArtistDetail(link.clone()),
            PlaybackOrigin::Playlist(link) => Nav::PlaylistDetail(link.clone()),
            PlaybackOrigin::Search(query) => Nav::SearchResults(query.clone()),
            PlaybackOrigin::Recommendations(request) => Nav::Recommendations(request.clone()),
        }
    }
}

impl fmt::Display for PlaybackOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            PlaybackOrigin::Library => f.write_str("Saved Tracks"),
            PlaybackOrigin::Album(link) => link.name.fmt(f),
            PlaybackOrigin::Artist(link) => link.name.fmt(f),
            PlaybackOrigin::Playlist(link) => link.name.fmt(f),
            PlaybackOrigin::Search(query) => query.fmt(f),
            PlaybackOrigin::Recommendations(_) => f.write_str("Recommended"),
        }
    }
}

#[derive(Clone, Debug, Data)]
pub struct PlaybackPayload {
    pub origin: PlaybackOrigin,
    pub tracks: Vector<Arc<Track>>,
    pub position: usize,
}
