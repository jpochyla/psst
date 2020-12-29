use crate::data::{AlbumLink, ArtistLink, AudioDuration, Nav, PlaylistLink, Track};
use druid::{im::Vector, Data, Lens};
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens)]
pub struct Playback {
    pub state: PlaybackState,
    pub current: Option<CurrentPlayback>,
}

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq)]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct CurrentPlayback {
    pub item: Arc<Track>,
    pub origin: PlaybackOrigin,
    pub progress: AudioDuration,
}

#[derive(Clone, Debug, Data)]
pub enum PlaybackOrigin {
    Library,
    Album(AlbumLink),
    Artist(ArtistLink),
    Playlist(PlaylistLink),
    Search(String),
}

impl PlaybackOrigin {
    pub fn as_nav(&self) -> Nav {
        match &self {
            PlaybackOrigin::Library => Nav::Library,
            PlaybackOrigin::Album(link) => Nav::AlbumDetail(link.clone()),
            PlaybackOrigin::Artist(link) => Nav::ArtistDetail(link.clone()),
            PlaybackOrigin::Playlist(link) => Nav::PlaylistDetail(link.clone()),
            PlaybackOrigin::Search(query) => Nav::SearchResults(query.clone()),
        }
    }

    pub fn as_string(&self) -> String {
        match &self {
            PlaybackOrigin::Library => "Saved Tracks".to_string(),
            PlaybackOrigin::Album(link) => link.name.to_string(),
            PlaybackOrigin::Artist(link) => link.name.to_string(),
            PlaybackOrigin::Playlist(link) => link.name.to_string(),
            PlaybackOrigin::Search(query) => query.clone(),
        }
    }
}

#[derive(Clone, Debug, Data)]
pub struct PlaybackPayload {
    pub origin: PlaybackOrigin,
    pub tracks: Vector<Arc<Track>>,
    pub position: usize,
}
