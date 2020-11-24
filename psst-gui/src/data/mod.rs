mod album;
mod artist;
mod config;
mod ctx;
mod promise;
mod route;
mod track;
mod utils;

pub use crate::data::{
    album::{Album, AlbumType},
    artist::Artist,
    config::{AudioQuality, Config},
    ctx::Ctx,
    promise::{Promise, PromiseState},
    route::{Navigation, Route},
    track::{Track, TrackCtx, TrackId},
    utils::{AudioDuration, Image},
};

use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens)]
pub struct State {
    pub route: Route,
    pub history: Vector<Navigation>,
    pub config: Config,
    pub playback: Playback,
    pub search: Search,
    pub album: AlbumDetail,
    pub artist: ArtistDetail,
    pub playlist: PlaylistDetail,
    pub library: Library,
    pub track_ctx: TrackCtx,
}

impl Default for State {
    fn default() -> Self {
        Self {
            route: Route::Home,
            history: Vector::new(),
            config: Config::default(),
            playback: Playback {
                state: PlaybackState::Stopped,
                progress: None,
                item: None,
            },
            search: Search {
                input: "".into(),
                results: Promise::Empty,
            },
            album: AlbumDetail {
                id: "".into(),
                album: Promise::Empty,
            },
            artist: ArtistDetail {
                id: "".into(),
                artist: Promise::Empty,
                albums: Promise::Empty,
                top_tracks: Promise::Empty,
                related: Promise::Empty,
            },
            playlist: PlaylistDetail {
                playlist: Promise::Empty,
                tracks: Promise::Empty,
            },
            library: Library {
                saved_albums: Promise::Empty,
                saved_tracks: Promise::Empty,
                playlists: Promise::Empty,
            },
            track_ctx: TrackCtx {
                playback_item: None,
                saved_tracks: HashSet::new(),
            },
        }
    }
}

impl State {
    pub fn set_playback_loading(&mut self, item: Arc<Track>) {
        self.playback.state = PlaybackState::Loading;
        self.playback.item.replace(item);
        self.playback.progress.take();
        self.track_ctx.playback_item.take();
    }

    pub fn set_playback_playing(&mut self, item: Arc<Track>) {
        self.playback.state = PlaybackState::Playing;
        self.playback.item.replace(item.clone());
        self.playback.progress.take();
        self.track_ctx.playback_item.replace(item);
    }

    pub fn set_playback_progress(&mut self, progress: AudioDuration) {
        self.playback.state = PlaybackState::Playing;
        self.playback.progress.replace(progress);
    }

    pub fn set_playback_paused(&mut self) {
        self.playback.state = PlaybackState::Paused;
    }

    pub fn set_playback_stopped(&mut self) {
        self.playback.state = PlaybackState::Stopped;
        self.playback.item.take();
        self.playback.progress.take();
        self.track_ctx.playback_item.take();
    }
}

#[derive(Clone, Debug, Data)]
pub struct PlaybackCtx {
    pub tracks: Vector<Arc<Track>>,
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
    pub progress: Option<AudioDuration>,
    pub item: Option<Arc<Track>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Search {
    pub input: String,
    pub results: Promise<SearchResults, String>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct SearchResults {
    pub artists: Vector<Artist>,
    pub albums: Vector<Album>,
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Library {
    pub saved_albums: Promise<Vector<Album>>,
    pub saved_tracks: Promise<Vector<Arc<Track>>>,
    pub playlists: Promise<Vector<Playlist>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct AlbumDetail {
    pub id: Arc<str>,
    pub album: Promise<Album, Arc<str>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct ArtistDetail {
    pub id: Arc<str>,
    pub artist: Promise<Artist, Arc<str>>,
    pub albums: Promise<ArtistAlbums, Arc<str>>,
    pub top_tracks: Promise<Vector<Arc<Track>>, Arc<str>>,
    pub related: Promise<Vector<Artist>, Arc<str>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct ArtistAlbums {
    pub albums: Vector<Album>,
    pub singles: Vector<Album>,
    pub compilations: Vector<Album>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct PlaylistDetail {
    pub playlist: Promise<Playlist>,
    pub tracks: Promise<Vector<Arc<Track>>, Arc<str>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Playlist {
    pub id: Arc<str>,
    pub images: Vector<Image>,
    pub name: Arc<str>,
}
