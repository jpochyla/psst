use crate::data::{
    AlbumLink, ArtistLink, AudioAnalysis, Nav, PlaylistLink, Promise, Track, TrackId,
};
use druid::{im::Vector, Data, Lens};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Data, Lens)]
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

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq)]
pub enum QueueBehavior {
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq)]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct NowPlaying {
    pub item: Arc<Track>,
    pub origin: PlaybackOrigin,
    pub progress: Duration,
    pub analysis: Promise<AudioAnalysis, TrackId>,
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
    pub fn to_nav(&self) -> Nav {
        match &self {
            PlaybackOrigin::Library => Nav::SavedTracks,
            PlaybackOrigin::Album(link) => Nav::AlbumDetail(link.clone()),
            PlaybackOrigin::Artist(link) => Nav::ArtistDetail(link.clone()),
            PlaybackOrigin::Playlist(link) => Nav::PlaylistDetail(link.clone()),
            PlaybackOrigin::Search(query) => Nav::SearchResults(query.clone()),
        }
    }

    pub fn to_string(&self) -> String {
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
