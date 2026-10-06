mod album;
mod artist;
pub mod config;
pub mod connect;
mod ctx;
mod find;
mod id;
mod nav;
pub mod news;
mod playback;
mod playlist;
mod promise;
mod recommend;
pub mod resume;
mod search;
mod show;
mod slider_scroll_scale;
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
    time::{Duration, Instant},
};

use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use psst_core::{item_id::ItemId, session::SessionService};

pub use crate::data::{
    album::{Album, AlbumDetail, AlbumLink, AlbumType, DatePrecision},
    artist::{
        Artist, ArtistAlbums, ArtistDetail, ArtistInfo, ArtistLink, ArtistOverview, ArtistStats,
    },
    config::{AudioQuality, Authentication, Config, Preferences, PreferencesTab, Theme},
    ctx::Ctx,
    find::{FindQuery, Finder, MatchFindQuery},
    nav::{Nav, Route, SpotifyUrl},
    playback::{
        NowPlaying, Playable, PlayableMatcher, Playback, PlaybackOrigin, PlaybackPayload,
        PlaybackState, QueueBehavior, QueueEntry,
    },
    playlist::{
        Playlist, PlaylistAddTrack, PlaylistDetail, PlaylistLink, PlaylistRemoveTrack,
        PlaylistReorder, PlaylistTracks,
    },
    promise::{Promise, PromiseState},
    recommend::{
        Range, Recommend, Recommendations, RecommendationsKnobs, RecommendationsParams,
        RecommendationsRequest, Toggled,
    },
    search::{Search, SearchResults, SearchTopic},
    show::{Episode, EpisodeId, EpisodeLink, Show, ShowDetail, ShowEpisodes, ShowLink},
    slider_scroll_scale::SliderScrollScale,
    track::{AudioAnalysis, Lyrics, Track, TrackId, TrackLines},
    user::{PublicUser, UserProfile},
    utils::{Cached, Float64, Image, Page},
};
use crate::ui::credits::TrackCredits;

pub const ALERT_DURATION: Duration = Duration::from_secs(5);

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub splitify: crate::splitify::SplitState,
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
    pub show_detail: ShowDetail,
    pub library: Arc<Library>,
    pub common_ctx: Arc<CommonCtx>,
    pub home_detail: HomeDetail,
    pub alerts: Vector<Alert>,
    pub cache_notice: String,
    pub selected_folder: Option<String>,
    pub queue_panel_open: bool,
    pub queue_page: usize,
    pub folder_name: String,
    pub editing_folder: Option<String>,
    pub playlist_picker_track: Option<TrackId>,
    pub playlist_picker_filter: String,
    pub playlist_picker_url: String,
    pub playlist_picker_status: String,
    pub finder: Finder,
    pub added_queue: Vector<QueueEntry>,
    #[data(ignore)]
    pub engine_queue: Option<psst_core::player::queue::QueueSnapshot>,
    pub lyrics: Promise<Lyrics, String>,
    pub connect: connect::ConnectState,
    pub news: news::NewsState,
    pub credits: Option<TrackCredits>,
}

impl AppState {
    pub fn default_with_config(config: Config) -> Self {
        let library = Arc::new(Library {
            user_profile: Promise::Empty,
            saved_albums: Promise::Empty,
            saved_tracks: Promise::Empty,
            saved_shows: Promise::Empty,
            track_overrides: Default::default(),
            album_overrides: Default::default(),
            show_overrides: Default::default(),
            playlists: Promise::Empty,
        });
        let common_ctx = Arc::new(CommonCtx {
            now_playing: None,
            playing_origin: None,
            library: Arc::clone(&library),
            show_track_cover: config.show_track_cover,
            nav: Nav::Home,
        });
        let playback = Playback {
            state: PlaybackState::Stopped,
            now_playing: None,
            queue_behavior: config.queue_behavior,
            queue: Vector::new(),
            up_next: Vector::new(),
            volume: config.volume,
            lyrics_follow: true,
        };
        let mut state = Self {
            splitify: crate::splitify::SplitState::default(),
            session: SessionService::empty(),
            nav: Nav::Home,
            history: Vector::new(),
            config,
            preferences: Preferences {
                active: PreferencesTab::General,
                cache: None,
                cache_size: Promise::Empty,
                auth: Authentication::new(),
                lastfm_auth_result: None,
            },
            playback,
            added_queue: Vector::new(),
            queue_panel_open: false,
            queue_page: 0,
            engine_queue: None,
            search: Search {
                input: "".into(),
                topic: None,
                results: Promise::Empty,
            },
            recommend: Recommend {
                knobs: Default::default(),
                results: Promise::Empty,
            },
            home_detail: HomeDetail {
                made_for_you: Promise::Empty,
                user_top_mixes: Promise::Empty,
                best_of_artists: Promise::Empty,
                recommended_stations: Promise::Empty,
                your_shows: Promise::Empty,
                shows_that_you_might_like: Promise::Empty,
                uniquely_yours: Promise::Empty,
                jump_back_in: Promise::Empty,
                user_top_tracks: Promise::Empty,
                user_top_artists: Promise::Empty,
            },
            album_detail: AlbumDetail {
                album: Promise::Empty,
            },
            artist_detail: ArtistDetail {
                artist: Promise::Empty,
                albums: Promise::Empty,
                overview: Promise::Empty,
            },
            playlist_detail: PlaylistDetail {
                playlist: Promise::Empty,
                tracks: Promise::Empty,
            },
            show_detail: ShowDetail {
                show: Promise::Empty,
                episodes: Promise::Empty,
            },
            library,
            common_ctx,
            alerts: Vector::new(),
            cache_notice: String::new(),
            selected_folder: None,
            folder_name: String::new(),
            editing_folder: None,
            playlist_picker_track: None,
            playlist_picker_filter: String::new(),
            playlist_picker_url: String::new(),
            playlist_picker_status: String::new(),
            finder: Finder::new(),
            lyrics: Promise::Empty,
            connect: connect::ConnectState::default(),
            news: news::NewsState::default(),
            credits: None,
        };
        state.restore_resume();
        state
    }
}

impl AppState {
    pub fn navigate(&mut self, nav: &Nav) {
        if &self.nav != nav {
            let previous = mem::replace(&mut self.nav, nav.to_owned());
            self.history.push_back(previous);
            self.config.last_route.replace(nav.to_owned());
            Arc::make_mut(&mut self.common_ctx).nav = nav.to_owned();
        }
    }

    pub fn navigate_back(&mut self) {
        if let Some(mut nav) = self.history.pop_back() {
            if let Nav::SearchResults(query) = &nav {
                if SpotifyUrl::parse(query).is_some() {
                    nav = self.history.pop_back().unwrap_or(Nav::Home);
                }
            }

            if let Nav::AlbumDetail(album, _) = nav {
                nav = Nav::AlbumDetail(album, None);
            }

            self.nav = nav;
            self.config.last_route.replace(self.nav.to_owned());
            Arc::make_mut(&mut self.common_ctx).nav = self.nav.clone();
        }
    }

    pub fn refresh_current_route(&mut self) {
        match self.nav.clone() {
            Nav::SavedTracks => self.with_library_mut(|library| {
                library.saved_tracks.clear();
                library.track_overrides.clear();
            }),
            Nav::SavedAlbums => self.with_library_mut(|library| {
                library.saved_albums.clear();
                library.album_overrides.clear();
            }),
            Nav::Shows => self.with_library_mut(|library| {
                library.saved_shows.clear();
                library.show_overrides.clear();
            }),
            Nav::AlbumDetail(_, _) => self.album_detail.album.clear(),
            Nav::ArtistDetail(_) => {
                self.artist_detail.artist.clear();
                self.artist_detail.albums.clear();
                self.artist_detail.overview.clear();
            }
            Nav::PlaylistDetail(_) => {
                self.playlist_detail.playlist.clear();
                self.playlist_detail.tracks.clear();
            }
            Nav::ShowDetail(_) => {
                self.show_detail.show.clear();
                self.show_detail.episodes.clear();
            }
            Nav::SearchResults(_) => self.search.results.clear(),
            Nav::Recommendations(_) => self.recommend.results.clear(),
            Nav::Notifications => self.news.feed.clear(),
            Nav::Lyrics => self.lyrics.clear(),
            Nav::Home => {
                self.home_detail.made_for_you.clear();
                self.home_detail.user_top_mixes.clear();
                self.home_detail.best_of_artists.clear();
                self.home_detail.recommended_stations.clear();
                self.home_detail.your_shows.clear();
                self.home_detail.shows_that_you_might_like.clear();
                self.home_detail.uniquely_yours.clear();
                self.home_detail.jump_back_in.clear();
                self.home_detail.user_top_tracks.clear();
                self.home_detail.user_top_artists.clear();
            }
            Nav::Queue | Nav::Devices => {}
        }
    }
}

impl AppState {
    pub fn queued_entry(&self, item_id: ItemId) -> Option<QueueEntry> {
        if let Some(queued) = self
            .playback
            .queue
            .iter()
            .find(|queued| queued.item.id() == item_id)
            .cloned()
        {
            Some(queued)
        } else {
            self.added_queue
                .iter()
                .find(|queued| queued.item.id() == item_id)
                .cloned()
        }
    }

    pub fn add_queued_entry(&mut self, queue_entry: QueueEntry) {
        self.added_queue.push_back(queue_entry);
    }

    pub fn loading_playback(&mut self, item: Playable, origin: PlaybackOrigin) {
        self.common_ctx_mut().now_playing.take();
        self.playback.state = PlaybackState::Loading;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress: Duration::default(),
            library: Arc::clone(&self.library),
        });
    }

    pub fn start_playback(&mut self, item: Playable, origin: PlaybackOrigin, progress: Duration) {
        self.common_ctx_mut().now_playing.replace(item.clone());
        self.common_ctx_mut().playing_origin = Some(origin.clone());
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
        self.common_ctx_mut().now_playing.take();
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
    pub fn add_alert(&mut self, message: impl Display, style: AlertStyle) {
        let alert = Alert {
            message: message.to_string().into(),
            style,
            id: Alert::fresh_id(),
            created_at: Instant::now(),
        };
        self.alerts.push_back(alert);
    }

    pub fn info_alert(&mut self, message: impl Display) {
        self.add_alert(message, AlertStyle::Info);
    }

    pub fn error_alert(&mut self, message: impl Display) {
        self.add_alert(message, AlertStyle::Error);
    }

    pub fn dismiss_alert(&mut self, id: usize) {
        self.alerts.retain(|a| a.id != id);
    }

    pub fn cleanup_alerts(&mut self) {
        let now = Instant::now();
        self.alerts
            .retain(|alert| now.duration_since(alert.created_at) < ALERT_DURATION);
    }
}

#[derive(Clone, Data, Lens)]
pub struct Library {
    pub user_profile: Promise<UserProfile>,
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<SavedAlbums>,
    pub saved_tracks: Promise<SavedTracks>,
    pub saved_shows: Promise<Shows>,
    pub track_overrides: druid::im::HashMap<TrackId, bool>,
    pub album_overrides: druid::im::HashMap<Arc<str>, bool>,
    pub show_overrides: druid::im::HashMap<Arc<str>, bool>,
}

impl Library {
    pub fn add_track(&mut self, track: Arc<Track>) {
        self.track_overrides.insert(track.id, true);
        if let Some(saved) = self.saved_tracks.resolved_mut() {
            if saved.set.insert(track.id).is_none() {
                saved.tracks.push_front(track);
            }
        }
    }

    pub fn remove_track(&mut self, track_id: &TrackId) {
        self.track_overrides.insert(*track_id, false);
        if let Some(saved) = self.saved_tracks.resolved_mut() {
            saved.set.remove(track_id);
            saved.tracks.retain(|t| &t.id != track_id);
        }
    }

    pub fn contains_track(&self, track: &Track) -> bool {
        if let Some(saved) = self.track_overrides.get(&track.id) {
            return *saved;
        }
        if let Some(saved) = self.saved_tracks.resolved() {
            saved.set.contains(&track.id)
        } else {
            false
        }
    }

    pub fn add_album(&mut self, album: Arc<Album>) {
        self.album_overrides.insert(album.id.clone(), true);
        if let Some(saved) = self.saved_albums.resolved_mut() {
            if saved.set.insert(album.id.clone()).is_none() {
                saved.albums.push_front(album);
            }
        }
    }

    pub fn remove_album(&mut self, album_id: &str) {
        self.album_overrides.insert(album_id.into(), false);
        if let Some(saved) = self.saved_albums.resolved_mut() {
            saved.set.remove(album_id);
            saved.albums.retain(|a| a.id.as_ref() != album_id);
        }
    }

    pub fn contains_album(&self, album: &Album) -> bool {
        if let Some(saved) = self.album_overrides.get(&album.id) {
            return *saved;
        }
        if let Some(saved) = self.saved_albums.resolved() {
            saved.set.contains(&album.id)
        } else {
            false
        }
    }

    pub fn add_show(&mut self, show: Arc<Show>) {
        self.show_overrides.insert(show.id.clone(), true);
        if let Some(saved) = self.saved_shows.resolved_mut() {
            if saved.set.insert(show.id.clone()).is_none() {
                saved.shows.push_front(show);
            }
        }
    }

    pub fn remove_show(&mut self, show_id: &str) {
        self.show_overrides.insert(show_id.into(), false);
        if let Some(saved) = self.saved_shows.resolved_mut() {
            saved.set.remove(show_id);
            saved.shows.retain(|a| a.id.as_ref() != show_id);
        }
    }

    pub fn contains_show(&self, show: &Show) -> bool {
        if let Some(saved) = self.show_overrides.get(&show.id) {
            return *saved;
        }
        if let Some(saved) = self.saved_shows.resolved() {
            saved.set.contains(&show.id)
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

    pub fn add_playlist(&mut self, playlist: Playlist) {
        if let Some(playlists) = self.playlists.resolved_mut() {
            playlists.push_back(playlist);
        }
    }

    pub fn remove_from_playlist(&mut self, id: &str) {
        if let Some(playlists) = self.playlists.resolved_mut() {
            playlists.retain(|p| p.id.as_ref() != id);
        }
    }

    pub fn rename_playlist(&mut self, link: PlaylistLink) {
        if let Some(saved) = self.playlists.resolved_mut() {
            for playlist in saved.iter_mut() {
                if playlist.id == link.id {
                    playlist.name = link.name;
                    break;
                }
            }
        }
    }

    pub fn is_created_by_user(&self, playlist: &Playlist) -> bool {
        if let Some(profile) = self.user_profile.resolved() {
            profile.id == playlist.owner.id
        } else {
            false
        }
    }

    pub fn contains_playlist(&self, playlist: &Playlist) -> bool {
        if let Some(playlists) = self.playlists.resolved() {
            playlists.iter().any(|p| p.id == playlist.id)
        } else {
            false
        }
    }

    pub fn increment_playlist_track_count(&mut self, link: &PlaylistLink) {
        if let Some(saved) = self.playlists.resolved_mut() {
            if let Some(playlist) = saved.iter_mut().find(|p| p.id == link.id) {
                playlist.track_count = playlist.track_count.map(|count| count + 1);
            }
        }
    }

    pub fn decrement_playlist_track_count(&mut self, link: &PlaylistLink) {
        if let Some(saved) = self.playlists.resolved_mut() {
            if let Some(playlist) = saved.iter_mut().find(|p| p.id == link.id) {
                playlist.track_count = playlist.track_count.map(|count| count.saturating_sub(1));
            }
        }
    }
}

impl Default for Library {
    fn default() -> Self {
        Library {
            user_profile: Promise::Empty,
            playlists: Promise::Empty,
            saved_albums: Promise::Empty,
            saved_tracks: Promise::Empty,
            saved_shows: Promise::Empty,
            track_overrides: Default::default(),
            album_overrides: Default::default(),
            show_overrides: Default::default(),
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

#[derive(Clone, Default, Data, Lens)]
pub struct Shows {
    pub shows: Vector<Arc<Show>>,
    pub set: HashSet<Arc<str>>,
}

impl Shows {
    pub fn new(shows: Vector<Arc<Show>>) -> Self {
        let set = shows.iter().map(|a| a.id.clone()).collect();
        Self { shows, set }
    }
}

#[derive(Clone, Data)]
pub struct CommonCtx {
    pub now_playing: Option<Playable>,
    pub playing_origin: Option<PlaybackOrigin>,
    pub library: Arc<Library>,
    pub show_track_cover: bool,
    pub nav: Nav,
}

impl CommonCtx {
    pub fn is_playing_at(&self, item: &Playable, origin: &PlaybackOrigin) -> bool {
        let source_matches = match (&self.playing_origin, origin) {
            (Some(PlaybackOrigin::Playlist(a)), PlaybackOrigin::Playlist(b)) => a.id == b.id,
            (Some(PlaybackOrigin::Album(a)), PlaybackOrigin::Album(b)) => a.id == b.id,
            (Some(a), b) => a.same(b),
            _ => false,
        };
        source_matches
            && self
                .now_playing
                .as_ref()
                .is_some_and(|current| current.id() == item.id())
    }
}

pub type WithCtx<T> = Ctx<Arc<CommonCtx>, T>;

#[derive(Clone, Data, Lens)]
pub struct HomeDetail {
    pub made_for_you: Promise<MixedView>,
    pub user_top_mixes: Promise<MixedView>,
    pub best_of_artists: Promise<MixedView>,
    pub recommended_stations: Promise<MixedView>,
    pub uniquely_yours: Promise<MixedView>,
    pub your_shows: Promise<MixedView>,
    pub shows_that_you_might_like: Promise<MixedView>,
    pub jump_back_in: Promise<MixedView>,
    pub user_top_tracks: Promise<Vector<Arc<Track>>>,
    pub user_top_artists: Promise<Vector<Artist>>,
}

#[derive(Clone, Data, Lens)]
pub struct MixedView {
    pub title: Arc<str>,
    pub playlists: Vector<Playlist>,
    pub artists: Vector<Artist>,
    pub albums: Vector<Arc<Album>>,
    pub shows: Vector<Arc<Show>>,
}

static ALERT_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Data, Lens)]
pub struct Alert {
    pub id: usize,
    pub message: Arc<str>,
    pub style: AlertStyle,
    pub created_at: Instant,
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

#[cfg(test)]
mod refresh_tests {
    use super::*;
    #[test]
    fn refreshing_saved_tracks_preserves_other_views() {
        let mut state = AppState::default_with_config(Config::default());
        state.with_library_mut(|library| {
            library
                .saved_tracks
                .resolve((), SavedTracks::new(Vector::new()));
            library
                .saved_albums
                .resolve((), SavedAlbums::new(Vector::new()));
        });
        state.nav = Nav::SavedTracks;
        state.refresh_current_route();
        assert!(!state.library.saved_tracks.is_resolved());
        assert!(state.library.saved_albums.is_resolved());
    }
}

#[cfg(test)]
mod library_status_tests {
    use super::*;
    #[test]
    fn confirmed_save_is_visible_without_prefetching_all_saved_tracks() {
        let track: Track = serde_json::from_value(serde_json::json!({
            "id":"7omij53d6AvXefx13NNyfn", "name":"Fixture", "artists":[], "duration_ms":180000,
            "disc_number":1,"track_number":1,"explicit":false,"is_local":false
        }))
        .unwrap();
        let mut library = Library::default();
        assert!(!library.contains_track(&track));
        library.add_track(Arc::new(track.clone()));
        assert!(library.contains_track(&track));
        assert!(!library.saved_tracks.is_resolved());
        library.remove_track(&track.id);
        assert!(!library.contains_track(&track));
        library
            .saved_tracks
            .resolve((), SavedTracks::new(Vector::new()));
        library.add_track(Arc::new(track.clone()));
        library.add_track(Arc::new(track));
        assert_eq!(library.saved_tracks.resolved().unwrap().tracks.len(), 1);
    }
}
