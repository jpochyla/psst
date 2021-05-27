mod album;
mod artist;
mod config;
mod ctx;
mod nav;
mod playback;
mod playlist;
mod promise;
mod search;
mod track;
mod user;
mod utils;

pub use crate::data::{
    album::{Album, AlbumDetail, AlbumLink, AlbumType, Copyright, CopyrightType},
    artist::{Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks},
    config::{AudioQuality, Authentication, Config, Preferences, PreferencesTab, Theme},
    ctx::Ctx,
    nav::Nav,
    playback::{
        NowPlaying, Playback, PlaybackOrigin, PlaybackPayload, PlaybackState, QueueBehavior,
        QueuedTrack,
    },
    playlist::{Playlist, PlaylistDetail, PlaylistLink, PlaylistTracks},
    promise::{Promise, PromiseState},
    search::{Search, SearchResults},
    track::{AudioAnalysis, AudioSegment, TimeInterval, Track, TrackId},
    user::UserProfile,
    utils::{Cached, Image, Page},
};
use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use psst_core::session::SessionHandle;
use std::{sync::Arc, time::Duration};

#[derive(Clone, Data, Lens)]
pub struct State {
    #[data(ignore)]
    pub session: SessionHandle,

    pub route: Nav,
    pub volume: f64,
    pub history: Vector<Nav>,
    pub config: Config,
    pub preferences: Preferences,
    pub playback: Playback,
    pub search: Search,
    pub album: AlbumDetail,
    pub artist: ArtistDetail,
    pub playlist: PlaylistDetail,
    pub library: Arc<Library>,
    pub common_ctx: CommonCtx,
    pub user_profile: Promise<UserProfile>,
    pub personalized: Personalized,
}

impl Default for State {
    fn default() -> Self {
        Self {
            session: SessionHandle::new(),
            route: Nav::Home,
            volume: 50.0,
            history: Vector::new(),
            config: Config::default(),
            preferences: Preferences {
                active: PreferencesTab::General,
                auth: Authentication {
                    username: String::new(),
                    password: String::new(),
                    result: Promise::Empty,
                },
                cache_size: Promise::Empty,
            },
            playback: Playback {
                state: PlaybackState::Stopped,
                now_playing: None,
                queue_behavior: QueueBehavior::Sequential,
                queue: Vector::new(),
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
            user_profile: Promise::Empty,
            personalized: Personalized {
                made_for_you: Promise::Empty,
            },
        }
    }
}

impl State {
    pub fn navigate(&mut self, nav: &Nav) {
        if &self.route != nav {
            self.history.push_back(self.route.clone());
            self.route = nav.to_owned();
        }
    }

    pub fn navigate_back(&mut self) {
        if let Some(nav) = self.history.pop_back() {
            self.route = nav;
        }
    }
}

impl State {
    pub fn queued_track(&self, track_id: &TrackId) -> Option<QueuedTrack> {
        self.playback
            .queue
            .iter()
            .find(|queued| queued.track.id.same(track_id))
            .cloned()
    }

    pub fn loading_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin) {
        self.common_ctx.playback_item.take();
        self.playback.state = PlaybackState::Loading;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress: Duration::default(),
            analysis: Promise::default(),
        });
    }

    pub fn start_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin, progress: Duration) {
        self.common_ctx.playback_item.replace(item.clone());
        self.playback.state = PlaybackState::Playing;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress,
            analysis: Promise::default(),
        });
    }

    pub fn progress_playback(&mut self, progress: Duration) {
        self.playback.now_playing.as_mut().map(|current| {
            current.progress = progress;
        });
    }

    pub fn pause_playback(&mut self) {
        self.playback.state = PlaybackState::Paused;
    }

    pub fn resume_playback(&mut self) {
        self.playback.state = PlaybackState::Playing;
    }

    pub fn block_playback(&mut self) {
        // TODO: Figure out how to signal blocked playback properly.
    }

    pub fn stop_playback(&mut self) {
        self.playback.state = PlaybackState::Stopped;
        self.playback.now_playing.take();
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

#[derive(Clone, Data, Lens)]
pub struct Library {
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<Vector<Album>>,
    pub saved_tracks: Promise<SavedTracks>,
}

#[derive(Clone, Data, Lens)]
pub struct SavedTracks {
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Data)]
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

#[derive(Clone, Data, Lens)]
pub struct Personalized {
    pub made_for_you: Promise<Vector<Playlist>>,
}
