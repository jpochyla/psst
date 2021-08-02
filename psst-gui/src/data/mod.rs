mod album;
mod artist;
mod config;
mod ctx;
mod nav;
mod playback;
mod playlist;
mod promise;
mod recommend;
mod search;
mod track;
mod user;
mod utils;

pub use crate::data::{
    album::{Album, AlbumDetail, AlbumLink, AlbumType, Copyright, CopyrightType},
    artist::{Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks},
    config::{AudioQuality, Authentication, Config, Preferences, PreferencesTab, Theme},
    ctx::Ctx,
    nav::{Nav, SpotifyUrl},
    playback::{
        NowPlaying, Playback, PlaybackOrigin, PlaybackPayload, PlaybackState, QueueBehavior,
        QueuedTrack,
    },
    playlist::{Playlist, PlaylistDetail, PlaylistLink, PlaylistTracks},
    promise::{Promise, PromiseState},
    recommend::{Recommend, Recommendations, RecommendationsRequest},
    search::{Search, SearchResults},
    track::{AudioAnalysis, AudioSegment, TimeInterval, Track, TrackId},
    user::UserProfile,
    utils::{Cached, Image, Page},
};
use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use psst_core::session::SessionService;
use std::{sync::Arc, time::Duration};

#[derive(Clone, Data, Lens)]
pub struct AppState {
    #[data(ignore)]
    pub session: SessionService,

    pub route: Nav,
    pub history: Vector<Nav>,
    pub config: Config,
    pub preferences: Preferences,
    pub playback: Playback,
    pub search: Search,
    pub recommend: Recommend,
    pub album_detail: AlbumDetail,
    pub artist_detail: ArtistDetail,
    pub playlist_detail: PlaylistDetail,
    pub library: Arc<Library>,
    pub common_ctx: Arc<CommonCtx>,
    pub user_profile: Promise<UserProfile>,
    pub personalized: Personalized,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: SessionService::empty(),
            route: Nav::Home,
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
                volume: 1.0, // 100% volume.
            },
            search: Search {
                input: "".into(),
                results: Promise::Empty,
            },
            recommend: Recommend {
                counter: 0,
                request: None,
                results: Promise::Empty,
            },
            album_detail: AlbumDetail {
                album: Promise::Empty,
            },
            artist_detail: ArtistDetail {
                artist: Promise::Empty,
                albums: Promise::Empty,
                top_tracks: Promise::Empty,
                related_artists: Promise::Empty,
            },
            playlist_detail: PlaylistDetail {
                playlist: Promise::Empty,
                tracks: Promise::Empty,
            },
            library: Arc::new(Library {
                saved_albums: Promise::Empty,
                saved_tracks: Promise::Empty,
                playlists: Promise::Empty,
            }),
            common_ctx: Arc::new(CommonCtx {
                playback_item: None,
                saved_tracks: HashSet::new(),
                saved_albums: HashSet::new(),
            }),
            user_profile: Promise::Empty,
            personalized: Personalized {
                made_for_you: Promise::Empty,
            },
        }
    }
}

impl AppState {
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

impl AppState {
    pub fn queued_track(&self, track_id: &TrackId) -> Option<QueuedTrack> {
        self.playback
            .queue
            .iter()
            .find(|queued| queued.track.id.same(track_id))
            .cloned()
    }

    pub fn loading_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin) {
        self.common_ctx_mut().playback_item.take();
        self.playback.state = PlaybackState::Loading;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress: Duration::default(),
            analysis: Promise::default(),
        });
    }

    pub fn start_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin, progress: Duration) {
        self.common_ctx_mut().playback_item.replace(item.clone());
        self.playback.state = PlaybackState::Playing;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress,
            analysis: Promise::default(),
        });
    }

    pub fn progress_playback(&mut self, progress: Duration) {
        if let Some(now_playing) = &mut self.playback.now_playing {
            now_playing.progress = progress;
        }
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
        self.common_ctx_mut().playback_item.take();
    }
}

impl AppState {
    pub fn save_track(&mut self, track: Arc<Track>) {
        self.library_mut().save_track(track);
        if let Promise::Resolved(saved) = &self.library.saved_tracks {
            Arc::make_mut(&mut self.common_ctx).set_saved_tracks(saved);
        }
    }

    pub fn unsave_track(&mut self, track_id: &TrackId) {
        self.library_mut().unsave_track(track_id);
        if let Promise::Resolved(saved) = &self.library.saved_tracks {
            Arc::make_mut(&mut self.common_ctx).set_saved_tracks(saved);
        }
    }

    pub fn save_album(&mut self, album: Arc<Album>) {
        self.library_mut().save_album(album);
        if let Promise::Resolved(saved) = &self.library.saved_albums {
            Arc::make_mut(&mut self.common_ctx).set_saved_albums(saved);
        }
    }

    pub fn unsave_album(&mut self, album_id: &Arc<str>) {
        self.library_mut().unsave_album(album_id);
        if let Promise::Resolved(saved) = &self.library.saved_albums {
            Arc::make_mut(&mut self.common_ctx).set_saved_albums(saved);
        }
    }

    pub fn common_ctx_mut(&mut self) -> &mut CommonCtx {
        Arc::make_mut(&mut self.common_ctx)
    }

    pub fn library_mut(&mut self) -> &mut Library {
        Arc::make_mut(&mut self.library)
    }
}

#[derive(Clone, Data, Lens)]
pub struct Library {
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<SavedAlbums>,
    pub saved_tracks: Promise<SavedTracks>,
}

impl Library {
    pub fn save_track(&mut self, track: Arc<Track>) {
        if let Promise::Resolved(saved) = &mut self.saved_tracks {
            saved.tracks.push_front(track);
        }
    }

    pub fn unsave_track(&mut self, track_id: &TrackId) {
        if let Promise::Resolved(saved) = &mut self.saved_tracks {
            saved.tracks.retain(|t| &t.id != track_id);
        }
    }

    pub fn save_album(&mut self, album: Arc<Album>) {
        if let Promise::Resolved(saved) = &mut self.saved_albums {
            saved.albums.push_front(album);
        }
    }

    pub fn unsave_album(&mut self, album_id: &Arc<str>) {
        if let Promise::Resolved(saved) = &mut self.saved_albums {
            saved.albums.retain(|a| &a.id != album_id)
        }
    }
}

#[derive(Clone, Default, Data, Lens)]
pub struct SavedTracks {
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Default, Data, Lens)]
pub struct SavedAlbums {
    pub albums: Vector<Arc<Album>>,
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

    pub fn set_saved_tracks(&mut self, saved: &SavedTracks) {
        self.saved_tracks = saved.tracks.iter().map(|t| t.id).collect();
    }

    pub fn is_album_saved(&self, album: &Album) -> bool {
        self.saved_albums.contains(&album.id)
    }

    pub fn set_saved_albums(&mut self, saved: &SavedAlbums) {
        self.saved_albums = saved.albums.iter().map(|a| a.id.clone()).collect();
    }
}

#[derive(Clone, Data, Lens)]
pub struct Personalized {
    pub made_for_you: Promise<Vector<Playlist>>,
}
