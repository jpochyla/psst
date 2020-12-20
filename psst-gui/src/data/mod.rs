mod album;
mod artist;
mod config;
mod ctx;
mod playback;
mod promise;
mod route;
mod track;
mod utils;

pub use crate::data::{
    album::{Album, AlbumType},
    artist::Artist,
    config::{AudioQuality, Config},
    ctx::Ctx,
    playback::{Playback, PlaybackCtx, PlaybackState},
    promise::{Promise, PromiseState},
    route::{Navigation, Route},
    track::{Track, TrackCtx, TrackId, TrackList, TrackOrigin, LOCAL_TRACK_ID},
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
    pub library: Arc<Library>,
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
                origin: None,
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
            library: Arc::new(Library {
                saved_albums: Promise::Empty,
                saved_tracks: Promise::Empty,
                playlists: Promise::Empty,
            }),
            track_ctx: TrackCtx {
                playback_item: None,
                saved_tracks: HashSet::new(),
                saved_albums: HashSet::new(),
            },
        }
    }
}

impl State {
    pub fn set_playback_loading(&mut self, item: Arc<Track>, origin: TrackOrigin) {
        self.playback.state = PlaybackState::Loading;
        self.playback.origin.replace(origin);
        self.playback.item.replace(item);
        self.playback.progress.take();
        self.track_ctx.playback_item.take();
    }

    pub fn set_playback_playing(&mut self, item: Arc<Track>, origin: TrackOrigin) {
        self.playback.state = PlaybackState::Playing;
        self.playback.origin.replace(origin);
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
        self.playback.origin.take();
        self.playback.item.take();
        self.playback.progress.take();
        self.track_ctx.playback_item.take();
    }
}

impl State {
    pub fn save_track(&mut self, track: Arc<Track>) {
        if let Promise::Resolved(list) = &mut self.library_mut().saved_tracks {
            list.tracks.push_front(track);
        }
        if let Promise::Resolved(list) = &self.library.saved_tracks {
            self.track_ctx.set_saved_tracks(&list.tracks);
        }
    }

    pub fn unsave_track(&mut self, track_id: &TrackId) {
        if let Promise::Resolved(list) = &mut self.library_mut().saved_tracks {
            list.tracks.retain(|track| &track.id != track_id);
        }
        if let Promise::Resolved(list) = &self.library.saved_tracks {
            self.track_ctx.set_saved_tracks(&list.tracks);
        }
    }

    pub fn save_album(&mut self, album: Album) {
        if let Promise::Resolved(albums) = &mut self.library_mut().saved_albums {
            albums.push_front(album);
        }
    }

    pub fn unsave_album(&mut self, album_id: &Arc<str>) {
        if let Promise::Resolved(albums) = &mut self.library_mut().saved_albums {
            albums.retain(|album| &album.id != album_id)
        }
    }

    pub fn library_mut(&mut self) -> &mut Library {
        Arc::make_mut(&mut self.library)
    }
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
    pub tracks: TrackList,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Library {
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<Vector<Album>>,
    pub saved_tracks: Promise<TrackList>,
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
    pub related: Promise<Vector<Artist>, Arc<str>>,
    pub top_tracks: Promise<TrackList, Arc<str>>,
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
    pub tracks: Promise<TrackList, Arc<str>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Playlist {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
}
