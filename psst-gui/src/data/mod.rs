mod album;
mod artist;
mod config;
mod ctx;
mod playback;
mod playlist;
mod promise;
mod route;
mod search;
mod track;
mod utils;

pub use crate::data::{
    album::{Album, AlbumDetail, AlbumLink, AlbumType},
    artist::{Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks},
    config::{AudioQuality, Config},
    ctx::Ctx,
    playback::{CurrentPlayback, Playback, PlaybackOrigin, PlaybackPayload, PlaybackState},
    playlist::{Playlist, PlaylistDetail, PlaylistLink, PlaylistTracks},
    promise::{Promise, PromiseState},
    route::Navigation,
    search::{Search, SearchResults},
    track::{Track, TrackId, LOCAL_TRACK_ID},
    utils::{AudioDuration, Image},
};
use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens)]
pub struct State {
    pub route: Navigation,
    pub history: Vector<Navigation>,
    pub config: Config,
    pub playback: Playback,
    pub search: Search,
    pub album: AlbumDetail,
    pub artist: ArtistDetail,
    pub playlist: PlaylistDetail,
    pub library: Arc<Library>,
    pub common_ctx: CommonCtx,
}

impl Default for State {
    fn default() -> Self {
        Self {
            route: Navigation::Home,
            history: Vector::new(),
            config: Config::default(),
            playback: Playback {
                state: PlaybackState::Stopped,
                current: None,
            },
            search: Search {
                input: "".into(),
                results: Promise::Empty,
            },
            album: AlbumDetail {
                album: Promise::Empty,
            },
            artist: ArtistDetail {
                artist: Promise::Empty,
                albums: Promise::Empty,
                top_tracks: Promise::Empty,
                related_artists: Promise::Empty,
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
            common_ctx: CommonCtx {
                playback_item: None,
                saved_tracks: HashSet::new(),
                saved_albums: HashSet::new(),
            },
        }
    }
}

impl State {
    pub fn set_playback_loading(&mut self, item: Arc<Track>, origin: PlaybackOrigin) {
        self.common_ctx.playback_item.take();
        self.playback.state = PlaybackState::Loading;
        self.playback.current.replace(CurrentPlayback {
            item,
            origin,
            progress: Default::default(),
        });
    }

    pub fn set_playback_playing(&mut self, item: Arc<Track>, origin: PlaybackOrigin) {
        self.common_ctx.playback_item.replace(item.clone());
        self.playback.state = PlaybackState::Playing;
        self.playback.current.replace(CurrentPlayback {
            item,
            origin,
            progress: Default::default(),
        });
    }

    pub fn set_playback_progress(&mut self, progress: AudioDuration) {
        self.playback.state = PlaybackState::Playing;
        self.playback.current.as_mut().map(|current| {
            current.progress = progress;
        });
    }

    pub fn set_playback_paused(&mut self) {
        self.playback.state = PlaybackState::Paused;
    }

    pub fn set_playback_stopped(&mut self) {
        self.playback.state = PlaybackState::Stopped;
        self.playback.current.take();
        self.common_ctx.playback_item.take();
    }
}

impl State {
    pub fn save_track(&mut self, track: Arc<Track>) {
        if let Promise::Resolved(saved) = &mut self.library_mut().saved_tracks {
            saved.tracks.push_front(track);
        }
        if let Promise::Resolved(saved) = &self.library.saved_tracks {
            self.common_ctx.set_saved_tracks(&saved.tracks);
        }
    }

    pub fn unsave_track(&mut self, track_id: &TrackId) {
        if let Promise::Resolved(saved) = &mut self.library_mut().saved_tracks {
            saved.tracks.retain(|track| &track.id != track_id);
        }
        if let Promise::Resolved(saved) = &self.library.saved_tracks {
            self.common_ctx.set_saved_tracks(&saved.tracks);
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
pub struct Library {
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<Vector<Album>>,
    pub saved_tracks: Promise<SavedTracks>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct SavedTracks {
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Debug, Data)]
pub struct CommonCtx {
    pub playback_item: Option<Arc<Track>>,
    pub saved_tracks: HashSet<TrackId>,
    pub saved_albums: HashSet<Arc<str>>,
}

impl CommonCtx {
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
        self.saved_tracks = tracks.iter().map(|track| track.id.clone()).collect();
    }

    pub fn is_album_saved(&self, album: &Album) -> bool {
        self.saved_albums.contains(&album.id)
    }

    pub fn set_saved_albums(&mut self, albums: &Vector<Album>) {
        self.saved_albums = albums.iter().map(|album| album.id.clone()).collect();
    }
}
