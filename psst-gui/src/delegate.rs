use crate::{
    commands::*,
    consts,
    data::{Config, Navigation, PlaybackReport, Route, State, Track},
    database::Web,
    widgets::remote_image,
};
use druid::{
    im::Vector, AppDelegate, Application, Command, DelegateCtx, Env, Event, ExtEventSink, Handled,
    HotKey, ImageBuf, SysMods, Target, WindowId,
};
use lru_cache::LruCache;
use psst_core::{
    audio_output::AudioOutput,
    audio_player::{PlaybackConfig, PlaybackItem, Player, PlayerCommand, PlayerEvent},
    cache::Cache,
    cdn::{Cdn, CdnHandle},
    connection::Credentials,
    session::SessionHandle,
    spotify_id::{SpotifyId, SpotifyIdType},
};
use std::{
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
    thread::JoinHandle,
    time::Duration,
};
use tokio::runtime::Runtime;

struct SessionDelegate {
    handle: SessionHandle,
    thread: JoinHandle<()>,
}

// Amount of time to wait before trying to connect again, in case session dies.
const RECONNECT_AFTER_DELAY: Duration = Duration::from_secs(3);

impl SessionDelegate {
    fn new(config: &Config, event_sink: ExtEventSink) -> Self {
        let creds = Credentials::from_username_and_password(
            config.username.as_ref().cloned().unwrap(),
            config.password.as_ref().cloned().unwrap(),
        );
        let handle = SessionHandle::new();
        let thread = thread::spawn({
            let handle = handle.clone();
            move || {
                let connect_and_service_single_session = || {
                    let session = handle.connect(creds.clone())?;
                    log::info!("session connected");
                    event_sink
                        .submit_command(SESSION_CONNECTED, (), Target::Auto)
                        .unwrap();
                    session.service()
                };
                loop {
                    if let Err(err) = connect_and_service_single_session() {
                        log::error!("connection error: {:?}", err);
                        event_sink
                            .submit_command(SESSION_LOST, (), Target::Auto)
                            .unwrap();
                        thread::sleep(RECONNECT_AFTER_DELAY);
                    }
                }
            }
        });

        Self { handle, thread }
    }
}

struct PlayerDelegate {
    cdn: CdnHandle,
    session: SessionHandle,
    player_sender: Sender<PlayerEvent>,
    player_queue: Vector<Arc<Track>>,
    player_thread: JoinHandle<()>,
    audio_output_thread: JoinHandle<()>,
}

fn handle_player_events(
    mut player: Player,
    player_events: Receiver<PlayerEvent>,
    sink: ExtEventSink,
) {
    for event in player_events {
        match &event {
            PlayerEvent::Started { path } => {
                let report = PlaybackReport {
                    item: path.item_id.to_base62(),
                    // TODO: Zero duration is wrong here.
                    progress: Duration::new(0, 0).into(),
                };
                sink.submit_command(PLAYBACK_PLAYING, report, Target::Auto)
                    .unwrap();
            }
            PlayerEvent::Playing { path, duration, .. } => {
                let report = PlaybackReport {
                    item: path.item_id.to_base62(),
                    progress: duration.to_owned().into(),
                };
                sink.submit_command(PLAYBACK_PROGRESS, report, Target::Auto)
                    .unwrap();
            }
            PlayerEvent::Paused { .. } => {
                sink.submit_command(PLAYBACK_PAUSED, (), Target::Auto)
                    .unwrap();
            }
            PlayerEvent::Finished => {}
            _ => {}
        }
        player.handle(event);
    }
}

impl PlayerDelegate {
    fn new(session: SessionHandle, event_sink: ExtEventSink) -> Self {
        let cdn = Cdn::connect(session.clone());
        let cache = Cache::new().expect("Failed to open cache");

        let audio_output = AudioOutput::open().expect("Failed to open audio output");

        let (player, player_receiver) = {
            let session = session.clone();
            let cdn = cdn.clone();
            let cache = cache.clone();
            let ctrl = audio_output.controller();
            let config = PlaybackConfig {
                country: "CZ".to_string(),
            };
            Player::new(session, cdn, cache, config, ctrl)
        };
        let player_sender = player.event_sender();

        let audio_output_thread = thread::spawn({
            let player_source = player.audio_source();
            move || {
                audio_output
                    .start_playback(player_source)
                    .expect("Playback failed");
            }
        });

        let player_thread = thread::spawn(move || {
            handle_player_events(player, player_receiver, event_sink);
        });

        Self {
            cdn,
            session,
            audio_output_thread,
            player_thread,
            player_sender,
            player_queue: Vector::new(),
        }
    }

    fn get_track(&self, track_id: &str) -> Option<Arc<Track>> {
        self.player_queue
            .iter()
            .find(|track| matches!(&track.id, Some(id) if id == track_id))
            .cloned()
    }

    fn set_tracks(&mut self, tracks: Vector<Arc<Track>>) {
        self.player_queue = tracks;
    }

    fn play(&mut self, position: usize) {
        let items = self
            .player_queue
            .iter()
            .map(|track| {
                let id = track.id.as_ref().unwrap();
                let id_type = SpotifyIdType::Track;
                let item_id = SpotifyId::from_base62(&id, id_type).unwrap();
                PlaybackItem { item_id }
            })
            .collect();
        self.player_sender
            .send(PlayerEvent::Command(PlayerCommand::LoadQueue {
                items,
                position,
            }))
            .unwrap();
    }

    fn pause(&mut self) {
        self.player_sender
            .send(PlayerEvent::Command(PlayerCommand::Pause))
            .unwrap();
    }

    fn resume(&mut self) {
        self.player_sender
            .send(PlayerEvent::Command(PlayerCommand::Resume))
            .unwrap();
    }

    fn previous(&mut self) {
        self.player_sender
            .send(PlayerEvent::Command(PlayerCommand::Previous))
            .unwrap();
    }

    fn next(&mut self) {
        self.player_sender
            .send(PlayerEvent::Command(PlayerCommand::Next))
            .unwrap();
    }

    fn seek(&mut self, position: Duration) {
        self.player_sender
            .send(PlayerEvent::Command(PlayerCommand::Seek { position }))
            .unwrap();
    }
}

pub struct Delegate {
    event_sink: ExtEventSink,
    session: SessionDelegate,
    player: PlayerDelegate,
    web: Web,
    runtime: Runtime,
    image_cache: LruCache<String, ImageBuf>,
}

const IMAGE_CACHE_SIZE: usize = 2048;

impl Delegate {
    pub fn new(config: &Config, event_sink: ExtEventSink) -> Self {
        let runtime = Runtime::new().unwrap();
        let image_cache = LruCache::new(IMAGE_CACHE_SIZE);
        let session = {
            let sink = event_sink.clone();
            SessionDelegate::new(config, sink)
        };
        let player = {
            let session = session.handle.clone();
            let sink = event_sink.clone();
            PlayerDelegate::new(session, sink)
        };
        let web = {
            let session = session.handle.clone();
            Web::new(session)
        };

        Self {
            event_sink,
            session,
            player,
            web,
            runtime,
            image_cache,
        }
    }

    fn navigate(&mut self, data: &mut State, nav: Navigation) {
        log::info!("navigating to {:?}", nav);

        match nav {
            Navigation::Home => {
                data.route = Route::Home;
            }
            Navigation::SearchResults(query) => {
                self.event_sink
                    .submit_command(GOTO_SEARCH_RESULTS, query, Target::Auto)
                    .unwrap();
            }
            Navigation::AlbumDetail(id) => {
                self.event_sink
                    .submit_command(GOTO_ALBUM_DETAIL, id, Target::Auto)
                    .unwrap();
            }
            Navigation::ArtistDetail(id) => {
                self.event_sink
                    .submit_command(GOTO_ARTIST_DETAIL, id, Target::Auto)
                    .unwrap();
            }
            Navigation::PlaylistDetail(playlist) => {
                self.event_sink
                    .submit_command(GOTO_PLAYLIST_DETAIL, playlist, Target::Auto)
                    .unwrap();
            }
            Navigation::Library => {
                self.event_sink
                    .submit_command(GOTO_LIBRARY, (), Target::Auto)
                    .unwrap();
            }
        }
    }
}

impl AppDelegate<State> for Delegate {
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        _window_id: WindowId,
        event: Event,
        data: &mut State,
        _env: &Env,
    ) -> Option<Event> {
        match &event {
            //
            // Global hotkeys
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "1").matches(k_e) => {
                data.route = Route::Home;
                None
            }
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "2").matches(k_e) => {
                ctx.submit_command(GOTO_LIBRARY);
                None
            }
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "3").matches(k_e) => {
                data.route = Route::SearchResults;
                None
            }
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "l").matches(k_e) => {
                ctx.submit_command(SET_FOCUS.to(consts::WIDGET_SEARCH_INPUT));
                None
            }
            _ => Some(event),
        }
    }

    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> Handled {
        //
        // Common
        //
        if let Some(text) = cmd.get(COPY_TO_CLIPBOARD).cloned() {
            Application::global().clipboard().put_string(text);
            Handled::Yes
        //
        // remote_image
        //
        } else if let Some(location) = cmd.get(remote_image::REQUEST_DATA).cloned() {
            let sink = self.event_sink.clone();
            if let Some(image_buf) = self.image_cache.get_mut(&location) {
                let payload = remote_image::DataPayload {
                    image_buf: image_buf.clone(),
                    location,
                };
                sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                    .unwrap();
            } else {
                let web = self.web.clone();
                self.runtime.spawn(async move {
                    let dyn_image = web.load_image(&location).await.unwrap();
                    let image_buf = ImageBuf::from_dynamic_image(dyn_image);
                    let payload = remote_image::DataPayload {
                        location,
                        image_buf,
                    };
                    sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                        .unwrap();
                });
            }
            Handled::Yes
        } else if let Some(payload) = cmd.get(remote_image::PROVIDE_DATA) {
            self.image_cache
                .insert(payload.location.clone(), payload.image_buf.clone());
            Handled::No
        //
        // Session
        //
        } else if cmd.is(SESSION_CONNECTED) {
            self.event_sink
                .submit_command(LOAD_PLAYLISTS, (), Target::Auto)
                .unwrap();
            Handled::No
        //
        // Navigation
        //
        } else if let Some(nav) = cmd.get(NAVIGATE_TO) {
            data.nav_stack.push_back(nav.clone());
            self.navigate(data, nav.clone());
            Handled::Yes
        } else if cmd.is(NAVIGATE_BACK) {
            data.nav_stack.pop_back();
            let nav = data.nav_stack.last().cloned().unwrap_or(Navigation::Home);
            self.navigate(data, nav);
            Handled::Yes
        //
        // Playlists
        //
        } else if cmd.is(LOAD_PLAYLISTS) {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            data.library.playlists.defer_default();
            self.runtime.spawn(async move {
                let result = web.load_playlists().await;
                sink.submit_command(UPDATE_PLAYLISTS, result, Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some(result) = cmd.get(UPDATE_PLAYLISTS).cloned() {
            data.library.playlists.resolve_or_reject(result);
            Handled::Yes
        } else if let Some(playlist) = cmd.get(GOTO_PLAYLIST_DETAIL).cloned() {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            let playlist_id = playlist.id.clone();
            data.route = Route::PlaylistDetail;
            data.playlist.playlist.resolve(playlist);
            data.playlist.tracks.defer(playlist_id.clone());
            self.runtime.spawn(async move {
                let result = web.load_playlist_tracks(&playlist_id).await;
                sink.submit_command(UPDATE_PLAYLIST_TRACKS, (playlist_id, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((playlist_id, result)) = cmd.get(UPDATE_PLAYLIST_TRACKS).cloned() {
            if data.playlist.tracks.is_deferred(&playlist_id) {
                data.playlist.tracks.resolve_or_reject(result);
            }
            Handled::Yes
        //
        // Library, saved albums and tracks
        //
        } else if cmd.is(GOTO_LIBRARY) {
            data.route = Route::Library;
            if data.library.saved_albums.is_empty() || data.library.saved_albums.is_rejected() {
                data.library.saved_albums.defer_default();
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let result = web.load_saved_albums().await;
                    sink.submit_command(UPDATE_SAVED_ALBUMS, result, Target::Auto)
                        .unwrap();
                });
            }
            if data.library.saved_tracks.is_empty() || data.library.saved_tracks.is_rejected() {
                data.library.saved_tracks.defer_default();
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let result = web.load_saved_tracks().await;
                    sink.submit_command(UPDATE_SAVED_TRACKS, result, Target::Auto)
                        .unwrap();
                });
            }
            Handled::Yes
        } else if let Some(result) = cmd.get(UPDATE_SAVED_ALBUMS).cloned() {
            data.library.saved_albums.resolve_or_reject(result);
            Handled::Yes
        } else if let Some(result) = cmd.get(UPDATE_SAVED_TRACKS).cloned() {
            data.library.saved_tracks.resolve_or_reject(result);
            Handled::Yes
        //
        // Album detail
        //
        } else if let Some(album_id) = cmd.get(GOTO_ALBUM_DETAIL).cloned() {
            data.route = Route::AlbumDetail;
            data.album.id = album_id.clone();
            data.album.album.defer(album_id.clone());
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_album(&album_id).await;
                sink.submit_command(UPDATE_ALBUM_DETAIL, (album_id, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((album_id, result)) = cmd.get(UPDATE_ALBUM_DETAIL).cloned() {
            if data.album.album.is_deferred(&album_id) {
                data.album.album.resolve_or_reject(result);
            }
            Handled::Yes
        //
        // Artist detail
        //
        } else if let Some(artist_id) = cmd.get(GOTO_ARTIST_DETAIL) {
            data.route = Route::ArtistDetail;
            data.artist.id = artist_id.clone();
            data.artist.artist.defer(artist_id.clone());
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist(&id).await;
                sink.submit_command(UPDATE_ARTIST_DETAIL, (id, result), Target::Auto)
                    .unwrap();
            });
            data.artist.top_tracks.defer(artist_id.clone());
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist_albums(&id).await;
                sink.submit_command(UPDATE_ARTIST_ALBUMS, (id, result), Target::Auto)
                    .unwrap();
            });
            data.artist.albums.defer(artist_id.clone());
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist_top_tracks(&id).await;
                sink.submit_command(UPDATE_ARTIST_TOP_TRACKS, (id, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((artist_id, result)) = cmd.get(UPDATE_ARTIST_DETAIL).cloned() {
            if data.artist.artist.is_deferred(&artist_id) {
                data.artist.artist.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((artist_id, result)) = cmd.get(UPDATE_ARTIST_ALBUMS).cloned() {
            if data.artist.albums.is_deferred(&artist_id) {
                data.artist.albums.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((artist_id, result)) = cmd.get(UPDATE_ARTIST_TOP_TRACKS).cloned() {
            if data.artist.top_tracks.is_deferred(&artist_id) {
                data.artist.top_tracks.resolve_or_reject(result);
            }
            Handled::Yes
        //
        // Search
        //
        } else if let Some(query) = cmd.get(GOTO_SEARCH_RESULTS).cloned() {
            data.route = Route::SearchResults;
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            data.search.results.defer(query.clone());
            self.runtime.spawn(async move {
                let result = web.search(&query).await;
                sink.submit_command(UPDATE_SEARCH_RESULTS, result, Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some(result) = cmd.get(UPDATE_SEARCH_RESULTS).cloned() {
            data.search.results.resolve_or_reject(result);
            Handled::Yes
        //
        // Playback status
        //
        } else if let Some(report) = cmd.get(PLAYBACK_PROGRESS).cloned() {
            data.playback.is_playing = true;
            data.playback.progress = Some(report.progress);
            data.playback.item = self.player.get_track(&report.item);
            Handled::Yes
        } else if cmd.is(PLAYBACK_PAUSED) {
            data.playback.is_playing = false;
            Handled::No
        } else if cmd.is(PLAYBACK_STOPPED) {
            data.playback.is_playing = false;
            data.playback.progress = None;
            data.playback.item = None;
            Handled::No
        //
        // Playback control
        //
        } else if let Some(pb_ctx) = cmd.get(PLAY_TRACKS).cloned() {
            self.player.set_tracks(pb_ctx.tracks);
            self.player.play(pb_ctx.position);
            Handled::Yes
        } else if cmd.is(PLAY_PAUSE) {
            self.player.pause();
            Handled::Yes
        } else if cmd.is(PLAY_RESUME) {
            self.player.resume();
            Handled::Yes
        } else if cmd.is(PLAY_PREVIOUS) {
            self.player.previous();
            Handled::Yes
        } else if cmd.is(PLAY_NEXT) {
            self.player.next();
            Handled::Yes
        } else if let Some(frac) = cmd.get(SEEK_TO_FRACTION) {
            if let Some(track) = &data.playback.item {
                log::info!("seeking to {}", frac);
                let position = Duration::from_secs_f64(track.duration.as_secs_f64() * frac);
                self.player.seek(position);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
