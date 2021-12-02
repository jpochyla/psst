mod album;
mod artist;
pub mod config;
mod ctx;
mod find;
mod id;
mod nav;
mod playback;
mod playlist;
mod promise;
mod recommend;
mod search;
mod track;
mod user;
pub mod utils;

use std::{
    fmt::Display,
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use psst_core::session::SessionService;

pub use crate::data::{
    album::{Album, AlbumDetail, AlbumLink, AlbumType, Copyright, CopyrightType},
    artist::{Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks},
    config::{AudioQuality, Authentication, Config, Preferences, PreferencesTab, Theme},
    ctx::Ctx,
    find::{FindQuery, Finder, MatchFindQuery},
    nav::{Nav, Route, SpotifyUrl},
    playback::{
        NowPlaying, Playback, PlaybackOrigin, PlaybackPayload, PlaybackState, QueueBehavior,
        QueuedTrack,
    },
    playlist::{Playlist, PlaylistAddTrack, PlaylistDetail, PlaylistLink, PlaylistTracks},
    promise::{Promise, PromiseState},
    recommend::{
        Range, Recommend, Recommendations, RecommendationsKnobs, RecommendationsParams,
        RecommendationsRequest, Toggled,
    },
    search::{Search, SearchResults, SearchTopic},
    track::{AudioAnalysis, AudioSegment, TimeInterval, Track, TrackId},
    user::UserProfile,
    utils::{Cached, Float64, Image, Page},
};

#[derive(Clone, Data, Lens)]
pub struct AppState {
    #[data(ignore)]
    pub session: SessionService,

    pub nav: Nav,
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
    pub personalized: Personalized,
    pub alerts: Vector<Alert>,
}

impl AppState {
    pub fn default_with_config(config: Config) -> Self {
        let library = Arc::new(Library {
            user_profile: Promise::Empty,
            saved_albums: Promise::Empty,
            saved_tracks: Promise::Empty,
            playlists: Promise::Empty,
        });
        let common_ctx = Arc::new(CommonCtx {
            playback_item: None,
            library: Arc::clone(&library),
        });
        let playback = Playback {
            state: PlaybackState::Stopped,
            now_playing: None,
            queue_behavior: config.queue_behavior,
            queue: Vector::new(),
            volume: config.volume,
        };
        Self {
            session: SessionService::empty(),
            nav: Nav::Home,
            history: Vector::new(),
            config,
            preferences: Preferences {
                active: PreferencesTab::General,
                auth: Authentication {
                    username: String::new(),
                    password: String::new(),
                    result: Promise::Empty,
                },
                cache_size: Promise::Empty,
            },
            playback,
            search: Search {
                input: "".into(),
                results: Promise::Empty,
            },
            recommend: Recommend {
                knobs: Default::default(),
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
                finder: Default::default(),
            },
            library,
            common_ctx,
            personalized: Personalized {
                made_for_you: Promise::Empty,
            },
            alerts: Vector::new(),
        }
    }
}

impl AppState {
    pub fn navigate(&mut self, nav: &Nav) {
        if &self.nav != nav {
            let previous = mem::replace(&mut self.nav, nav.to_owned());
            self.history.push_back(previous);
            self.config.last_route.replace(nav.to_owned());
            self.config.save();
        }
    }

    pub fn navigate_back(&mut self) {
        if let Some(nav) = self.history.pop_back() {
            self.config.last_route.replace(nav.clone());
            self.config.save();
            self.nav = nav;
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
            library: Arc::clone(&self.library),
        });
    }

    pub fn start_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin, progress: Duration) {
        self.common_ctx_mut().playback_item.replace(item.clone());
        self.playback.state = PlaybackState::Playing;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress,
            library: Arc::clone(&self.library),
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

    pub fn set_queue_behavior(&mut self, queue_behavior: QueueBehavior) {
        self.playback.queue_behavior = queue_behavior;
        self.config.queue_behavior = queue_behavior;
        self.config.save();
    }
}

impl AppState {
    pub fn common_ctx_mut(&mut self) -> &mut CommonCtx {
        Arc::make_mut(&mut self.common_ctx)
    }

    pub fn with_library_mut(&mut self, func: impl FnOnce(&mut Library)) {
        func(Arc::make_mut(&mut self.library));
        self.library_updated();
    }

    fn library_updated(&mut self) {
        if let Some(now_playing) = &mut self.playback.now_playing {
            now_playing.library = Arc::clone(&self.library);
        }
        self.common_ctx_mut().library = Arc::clone(&self.library);
    }
}

impl AppState {
    pub fn info_alert(&mut self, message: impl Display) {
        self.alerts.push_back(Alert {
            message: message.to_string().into(),
            style: AlertStyle::Info,
            id: Alert::fresh_id(),
        });
    }

    pub fn error_alert(&mut self, message: impl Display) {
        self.alerts.push_back(Alert {
            message: message.to_string().into(),
            style: AlertStyle::Error,
            id: Alert::fresh_id(),
        });
    }

    pub fn dismiss_alert(&mut self, id: usize) {
        self.alerts.retain(|a| a.id != id);
    }
}

#[derive(Clone, Data, Lens)]
pub struct Library {
    pub user_profile: Promise<UserProfile>,
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<SavedAlbums>,
    pub saved_tracks: Promise<SavedTracks>,
}

impl Library {
    pub fn add_track(&mut self, track: Arc<Track>) {
        if let Some(saved) = self.saved_tracks.resolved_mut() {
            saved.set.insert(track.id);
            saved.tracks.push_front(track);
        }
    }

    pub fn remove_track(&mut self, track_id: &TrackId) {
        if let Some(saved) = self.saved_tracks.resolved_mut() {
            saved.set.remove(track_id);
            saved.tracks.retain(|t| &t.id != track_id);
        }
    }

    pub fn contains_track(&self, track: &Track) -> bool {
        if let Some(saved) = self.saved_tracks.resolved() {
            saved.set.contains(&track.id)
        } else {
            false
        }
    }

    pub fn add_album(&mut self, album: Arc<Album>) {
        if let Some(saved) = self.saved_albums.resolved_mut() {
            saved.set.insert(album.id.clone());
            saved.albums.push_front(album);
        }
    }

    pub fn remove_album(&mut self, album_id: &str) {
        if let Some(saved) = self.saved_albums.resolved_mut() {
            saved.set.remove(album_id);
            saved.albums.retain(|a| a.id.as_ref() != album_id);
        }
    }

    pub fn contains_album(&self, album: &Album) -> bool {
        if let Some(saved) = self.saved_albums.resolved() {
            saved.set.contains(&album.id)
        } else {
            false
        }
    }

    pub fn writable_playlists(&self) -> Vec<&Playlist> {
        if let Some(saved) = self.playlists.resolved() {
            saved
                .iter()
                .filter(|playlist| {
                    self.user_profile
                        .resolved()
                        .map(|user| playlist.owner.id == user.id)
                        .unwrap_or(false)
                        || playlist.collaborative
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn increment_playlist_track_count(&mut self, link: &PlaylistLink) {
        if let Some(saved) = self.playlists.resolved_mut() {
            for playlist in saved.iter_mut() {
                if playlist.id == link.id {
                    playlist.track_count += 1;
                }
            }
        }
    }
}

#[derive(Clone, Default, Data, Lens)]
pub struct SavedTracks {
    pub tracks: Vector<Arc<Track>>,
    pub set: HashSet<TrackId>,
}

impl SavedTracks {
    pub fn new(tracks: Vector<Arc<Track>>) -> Self {
        let set = tracks.iter().map(|t| t.id).collect();
        Self { tracks, set }
    }
}

#[derive(Clone, Default, Data, Lens)]
pub struct SavedAlbums {
    pub albums: Vector<Arc<Album>>,
    pub set: HashSet<Arc<str>>,
}

impl SavedAlbums {
    pub fn new(albums: Vector<Arc<Album>>) -> Self {
        let set = albums.iter().map(|a| a.id.clone()).collect();
        Self { albums, set }
    }
}

#[derive(Clone, Data)]
pub struct CommonCtx {
    pub playback_item: Option<Arc<Track>>,
    pub library: Arc<Library>,
}

impl CommonCtx {
    pub fn is_track_playing(&self, track: &Track) -> bool {
        self.playback_item
            .as_ref()
            .map(|t| t.id.same(&track.id))
            .unwrap_or(false)
    }
}

pub type WithCtx<T> = Ctx<Arc<CommonCtx>, T>;

#[derive(Clone, Data, Lens)]
pub struct Personalized {
    pub made_for_you: Promise<Vector<Playlist>>,
}

static ALERT_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Data, Lens)]
pub struct Alert {
    pub id: usize,
    pub message: Arc<str>,
    pub style: AlertStyle,
}

impl Alert {
    fn fresh_id() -> usize {
        ALERT_ID.fetch_add(1, Ordering::SeqCst)
    }
}

#[derive(Clone, Data, Eq, PartialEq)]
pub enum AlertStyle {
    Error,
    Info,
}
