use crate::data::{AudioDuration, Track, TrackList, TrackOrigin};
use druid::{Data, Lens};
use std::sync::Arc;

#[derive(Clone, Debug, Data)]
pub struct PlaybackCtx {
    pub tracks: TrackList,
    pub position: usize,
}

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq)]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Playback {
    pub state: PlaybackState,
    pub origin: Option<TrackOrigin>,
    pub progress: Option<AudioDuration>,
    pub item: Option<Arc<Track>>,
}
