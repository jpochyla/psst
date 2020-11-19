use crate::{
    cmd,
    data::{AudioDuration, Config, Navigation, Route, State, Track, TrackId},
    ui,
    web::{Web, WebCache},
    widget::remote_image,
};
use druid::{
    commands, im::Vector, AppDelegate, Application, Command, Data, DelegateCtx, Env, Event,
    ExtEventSink, Handled, ImageBuf, Selector, Target, WindowId,
};
use lru_cache::LruCache;
use psst_core::{
    audio_output::AudioOutput,
    audio_player::{PlaybackConfig, PlaybackItem, Player, PlayerCommand, PlayerEvent},
    cache::Cache,
    cdn::{Cdn, CdnHandle},
    connection::Credentials,
    session::SessionHandle,
};
use std::{
    collections::HashSet,
    future::Future,
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
    fn new(credentials: Credentials, event_sink: ExtEventSink) -> Self {
        let handle = SessionHandle::new();
        let thread = thread::spawn({
            let handle = handle.clone();
            move || Self::service(credentials, event_sink, handle)
        });
        Self { handle, thread }
    }

    fn service(credentials: Credentials, event_sink: ExtEventSink, handle: SessionHandle) {
        let connect_and_service_single_session = || {
            let session = handle.connect(credentials.clone())?;
            log::info!("session connected");
            event_sink
                .submit_command(cmd::SESSION_CONNECTED, (), Target::Auto)
                .unwrap();
            session.service()
        };
        loop {
            if let Err(err) = connect_and_service_single_session() {
                log::error!("connection error: {:?}", err);
                event_sink
                    .submit_command(cmd::SESSION_LOST, (), Target::Auto)
                    .unwrap();
                thread::sleep(RECONNECT_AFTER_DELAY);
            }
        }
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

impl PlayerDelegate {
    fn new(config: PlaybackConfig, session: SessionHandle, event_sink: ExtEventSink) -> Self {
        let cdn = Cdn::connect(session.clone());
        let cache = {
            let dir = Config::cache_dir().expect("Failed to find cache location");
            Cache::new(dir).expect("Failed to open cache")
        };

        let audio_output = AudioOutput::open().expect("Failed to open audio output");

        let (player, player_receiver) = {
            let session = session.clone();
            let cdn = cdn.clone();
            let ctrl = audio_output.controller();
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
            Self::service(player, player_receiver, event_sink);
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

    fn service(mut player: Player, player_events: Receiver<PlayerEvent>, sink: ExtEventSink) {
        for event in player_events {
            match &event {
                PlayerEvent::Started { path } => {
                    let item: TrackId = path.item_id.into();
                    sink.submit_command(cmd::PLAYBACK_PLAYING, item, Target::Auto)
                        .unwrap();
                }
                PlayerEvent::Playing { duration, .. } => {
                    let progress: AudioDuration = duration.to_owned().into();
                    sink.submit_command(cmd::PLAYBACK_PROGRESS, progress, Target::Auto)
                        .unwrap();
                }
                PlayerEvent::Paused { .. } => {
                    sink.submit_command(cmd::PLAYBACK_PAUSED, (), Target::Auto)
                        .unwrap();
                }
                PlayerEvent::Finished => {
                    // TODO:
                    //  We should clear current playback, but only at the end
                    //  of the queue.
                }
                _ => {}
            }
            player.handle(event);
        }
    }

    fn get_track(&self, id: &TrackId) -> Option<Arc<Track>> {
        self.player_queue
            .iter()
            .find(|track| track.id.same(id))
            .cloned()
    }

    fn set_tracks(&mut self, tracks: Vector<Arc<Track>>) {
        self.player_queue = tracks;
    }

    fn play(&mut self, position: usize) {
        let items = self
            .player_queue
            .iter()
            .map(|track| PlaybackItem { item_id: *track.id })
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

pub struct DelegateHolder {
    event_sink: ExtEventSink,
    delegate: Option<Delegate>,
    pub main_window: Option<WindowId>,
    pub config_window: Option<WindowId>,
    opened_windows: HashSet<WindowId>,
}

impl DelegateHolder {
    pub fn new(event_sink: ExtEventSink) -> Self {
        Self {
            event_sink,
            delegate: None,
            main_window: None,
            config_window: None,
            opened_windows: HashSet::new(),
        }
    }

    pub fn configure(&mut self, config: &Config) {
        if self.delegate.is_none() {
            self.delegate
                .replace(Delegate::new(config, self.event_sink.clone()));
        } else {
            log::warn!("already configured");
        }
    }
}

impl AppDelegate<State> for DelegateHolder {
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        window_id: WindowId,
        event: Event,
        data: &mut State,
        env: &Env,
    ) -> Option<Event> {
        if let Some(delegate) = self.delegate.as_mut() {
            delegate.event(ctx, window_id, event, data, env)
        } else {
            Some(event)
        }
    }

    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut State,
        env: &Env,
    ) -> Handled {
        if cmd.is(cmd::CONFIGURE) {
            self.configure(&data.config);
            Handled::Yes
        } else if cmd.is(cmd::SHOW_MAIN) {
            if self
                .main_window
                .as_ref()
                .map(|id| self.opened_windows.contains(id))
                .unwrap_or(false)
            {
                let win_id = self.config_window.unwrap();
                ctx.submit_command(commands::SHOW_WINDOW.to(win_id));
            } else {
                let win = ui::make_main_window();
                self.main_window.replace(win.id);
                ctx.new_window(win);
            }
            Handled::Yes
        } else if cmd.is(commands::SHOW_PREFERENCES) {
            if self
                .config_window
                .as_ref()
                .map(|id| self.opened_windows.contains(id))
                .unwrap_or(false)
            {
                let win_id = self.config_window.unwrap();
                ctx.submit_command(commands::SHOW_WINDOW.to(win_id));
            } else {
                let win = ui::make_config_window();
                self.config_window.replace(win.id);
                ctx.new_window(win);
            }
            Handled::Yes
        } else {
            self.delegate
                .as_mut()
                .map(|delegate| delegate.command(ctx, target, cmd, data, env))
                .unwrap_or(Handled::No)
        }
    }

    fn window_added(&mut self, id: WindowId, data: &mut State, env: &Env, ctx: &mut DelegateCtx) {
        self.opened_windows.insert(id);
        self.delegate
            .as_mut()
            .map(|delegate| delegate.window_added(id, data, env, ctx));
    }

    fn window_removed(&mut self, id: WindowId, data: &mut State, env: &Env, ctx: &mut DelegateCtx) {
        self.opened_windows.remove(&id);
        self.delegate
            .as_mut()
            .map(|delegate| delegate.window_removed(id, data, env, ctx));
    }
}

pub struct Delegate {
    event_sink: ExtEventSink,
    session: SessionDelegate,
    player: PlayerDelegate,
    web: Arc<Web>,
    runtime: Runtime,
    image_cache: LruCache<Arc<str>, ImageBuf>,
}

const IMAGE_CACHE_SIZE: usize = 2048;

impl Delegate {
    pub fn new(config: &Config, event_sink: ExtEventSink) -> Self {
        let runtime = Runtime::new().unwrap();
        let session = {
            let sink = event_sink.clone();
            let creds = config.credentials().expect("Missing session credentials");
            SessionDelegate::new(creds, sink)
        };
        let player = {
            let session = session.handle.clone();
            let sink = event_sink.clone();
            PlayerDelegate::new(config.playback(), session, sink)
        };
        let web = {
            let session = session.handle.clone();
            let path = Config::cache_dir().expect("Failed to find cache path location");
            let cache = WebCache::new(path).expect("Failed to create web API cache");
            Arc::new(Web::new(session, cache))
        };
        let image_cache = LruCache::new(IMAGE_CACHE_SIZE);

        Self {
            event_sink,
            session,
            player,
            web,
            runtime,
            image_cache,
        }
    }

    fn navigate_to(&mut self, data: &mut State, nav: Navigation) {
        data.history.push_back(nav.clone());
        self.navigate(data, nav);
    }

    fn navigate_back(&mut self, data: &mut State) {
        data.history.pop_back();
        self.navigate(
            data,
            data.history.last().cloned().unwrap_or(Navigation::Home),
        );
    }

    fn navigate(&mut self, data: &mut State, nav: Navigation) {
        match nav {
            Navigation::Home => {
                data.route = Route::Home;
            }
            Navigation::SearchResults(query) => {
                self.event_sink
                    .submit_command(cmd::GOTO_SEARCH_RESULTS, query, Target::Auto)
                    .unwrap();
            }
            Navigation::AlbumDetail(id) => {
                self.event_sink
                    .submit_command(cmd::GOTO_ALBUM_DETAIL, id, Target::Auto)
                    .unwrap();
            }
            Navigation::ArtistDetail(id) => {
                self.event_sink
                    .submit_command(cmd::GOTO_ARTIST_DETAIL, id, Target::Auto)
                    .unwrap();
            }
            Navigation::PlaylistDetail(playlist) => {
                self.event_sink
                    .submit_command(cmd::GOTO_PLAYLIST_DETAIL, playlist, Target::Auto)
                    .unwrap();
            }
            Navigation::Library => {
                self.event_sink
                    .submit_command(cmd::GOTO_LIBRARY, (), Target::Auto)
                    .unwrap();
            }
        }
    }

    fn submit_async<F>(&self, selector: Selector<F::Output>, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sink = self.event_sink.clone();
        self.runtime.spawn(async move {
            sink.submit_command(selector, future.await, Target::Auto)
                .unwrap();
        });
    }
}

impl AppDelegate<State> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> Handled {
        if let Some(text) = cmd.get(cmd::COPY) {
            Application::global().clipboard().put_string(&text);
            Handled::Yes
        } else if let Handled::Yes = self.command_image(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_session(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_nav(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_playlist(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_library(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_album(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_artist(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_search(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_playback(target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_playback_ctrl(target, cmd, data) {
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

impl Delegate {
    fn command_image(&mut self, target: Target, cmd: &Command, _data: &mut State) -> Handled {
        if let Some(location) = cmd.get(remote_image::REQUEST_DATA).cloned() {
            let sink = self.event_sink.clone();
            if let Some(image_buf) = self.image_cache.get_mut(&location).cloned() {
                let payload = remote_image::ImagePayload {
                    location,
                    image_buf,
                };
                sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                    .unwrap();
            } else {
                let web = self.web.clone();
                self.runtime.spawn(async move {
                    let dyn_image = web.load_image(&location).await.unwrap();
                    let image_buf = ImageBuf::from_dynamic_image(dyn_image);
                    let payload = remote_image::ImagePayload {
                        location,
                        image_buf,
                    };
                    sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                        .unwrap();
                });
            }
            Handled::Yes
        } else if let Some(payload) = cmd.get(remote_image::PROVIDE_DATA).cloned() {
            self.image_cache.insert(payload.location, payload.image_buf);
            Handled::No
        } else {
            Handled::No
        }
    }

    fn command_session(&mut self, _target: Target, cmd: &Command, _data: &mut State) -> Handled {
        if cmd.is(cmd::SESSION_CONNECTED) {
            self.event_sink
                .submit_command(cmd::LOAD_PLAYLISTS, (), Target::Auto)
                .unwrap();
            Handled::No
        } else {
            Handled::No
        }
    }

    fn command_nav(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(nav) = cmd.get(cmd::NAVIGATE_TO).cloned() {
            self.navigate_to(data, nav);
            Handled::Yes
        } else if cmd.is(cmd::NAVIGATE_BACK) {
            self.navigate_back(data);
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_playlist(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if cmd.is(cmd::LOAD_PLAYLISTS) {
            let web = self.web.clone();
            data.library.playlists.defer_default();
            self.submit_async(
                cmd::UPDATE_PLAYLISTS,
                async move { web.load_playlists().await },
            );
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_PLAYLISTS).cloned() {
            data.library.playlists.resolve_or_reject(result);
            Handled::Yes
        } else if let Some(playlist) = cmd.get(cmd::GOTO_PLAYLIST_DETAIL).cloned() {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            let playlist_id = playlist.id.clone();
            data.route = Route::PlaylistDetail;
            data.playlist.playlist.resolve(playlist);
            data.playlist.tracks.defer(playlist_id.clone());
            self.runtime.spawn(async move {
                let result = web.load_playlist_tracks(&playlist_id).await;
                sink.submit_command(
                    cmd::UPDATE_PLAYLIST_TRACKS,
                    (playlist_id, result),
                    Target::Auto,
                )
                .unwrap();
            });
            Handled::Yes
        } else if let Some((playlist_id, result)) = cmd.get(cmd::UPDATE_PLAYLIST_TRACKS).cloned() {
            if data.playlist.tracks.is_deferred(&playlist_id) {
                data.playlist.tracks.resolve_or_reject(result);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_library(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if cmd.is(cmd::GOTO_LIBRARY) {
            data.route = Route::Library;
            if data.library.saved_albums.is_empty() || data.library.saved_albums.is_rejected() {
                data.library.saved_albums.defer_default();
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let result = web.load_saved_albums().await;
                    sink.submit_command(cmd::UPDATE_SAVED_ALBUMS, result, Target::Auto)
                        .unwrap();
                });
            }
            if data.library.saved_tracks.is_empty() || data.library.saved_tracks.is_rejected() {
                data.library.saved_tracks.defer_default();
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let result = web.load_saved_tracks().await;
                    sink.submit_command(cmd::UPDATE_SAVED_TRACKS, result, Target::Auto)
                        .unwrap();
                });
            }
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SAVED_ALBUMS).cloned() {
            data.library.saved_albums.resolve_or_reject(result);
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SAVED_TRACKS).cloned() {
            match result {
                Ok(tracks) => {
                    data.track_ctx.set_saved_tracks(&tracks);
                    data.library.saved_tracks.resolve(tracks);
                }
                Err(err) => {
                    data.track_ctx.set_saved_tracks(&Vector::new());
                    data.library.saved_tracks.reject(err);
                }
            };
            Handled::Yes
        } else if let Some(track_id) = cmd.get(cmd::SAVE_TRACK).cloned() {
            // TODO
            Handled::Yes
        } else if let Some(track_id) = cmd.get(cmd::UNSAVE_TRACK).cloned() {
            // TODO
            Handled::Yes
        } else if let Some(album_id) = cmd.get(cmd::SAVE_ALBUM).cloned() {
            // TODO
            Handled::Yes
        } else if let Some(album_id) = cmd.get(cmd::UNSAVE_ALBUM).cloned() {
            // TODO
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_album(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(album_id) = cmd.get(cmd::GOTO_ALBUM_DETAIL).cloned() {
            data.route = Route::AlbumDetail;
            data.album.id = album_id.clone();
            data.album.album.defer(album_id.clone());
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_album(&album_id).await;
                sink.submit_command(cmd::UPDATE_ALBUM_DETAIL, (album_id, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((album_id, result)) = cmd.get(cmd::UPDATE_ALBUM_DETAIL).cloned() {
            if data.album.album.is_deferred(&album_id) {
                data.album.album.resolve_or_reject(result);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_artist(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(artist_id) = cmd.get(cmd::GOTO_ARTIST_DETAIL) {
            data.route = Route::ArtistDetail;
            data.artist.id = artist_id.clone();
            data.artist.artist.defer(artist_id.clone());
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist(&id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_DETAIL, (id, result), Target::Auto)
                    .unwrap();
            });
            data.artist.top_tracks.defer(artist_id.clone());
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist_albums(&id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_ALBUMS, (id, result), Target::Auto)
                    .unwrap();
            });
            data.artist.albums.defer(artist_id.clone());
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist_top_tracks(&id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_TOP_TRACKS, (id, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((artist_id, result)) = cmd.get(cmd::UPDATE_ARTIST_DETAIL).cloned() {
            if data.artist.artist.is_deferred(&artist_id) {
                data.artist.artist.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((artist_id, result)) = cmd.get(cmd::UPDATE_ARTIST_ALBUMS).cloned() {
            if data.artist.albums.is_deferred(&artist_id) {
                data.artist.albums.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((artist_id, result)) = cmd.get(cmd::UPDATE_ARTIST_TOP_TRACKS).cloned() {
            if data.artist.top_tracks.is_deferred(&artist_id) {
                data.artist.top_tracks.resolve_or_reject(result);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_search(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(query) = cmd.get(cmd::GOTO_SEARCH_RESULTS).cloned() {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            data.route = Route::SearchResults;
            data.search.results.defer(query.clone());
            self.runtime.spawn(async move {
                let result = web.search(&query).await;
                sink.submit_command(cmd::UPDATE_SEARCH_RESULTS, result, Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SEARCH_RESULTS).cloned() {
            data.search.results.resolve_or_reject(result);
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_playback(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(item) = cmd.get(cmd::PLAYBACK_PLAYING) {
            if let Some(track) = self.player.get_track(item) {
                data.set_playback_playing(track);
            } else {
                log::warn!("played item not found in playback queue");
            }
            Handled::Yes
        } else if let Some(progress) = cmd.get(cmd::PLAYBACK_PROGRESS).cloned() {
            data.set_playback_progress(progress);
            Handled::Yes
        } else if cmd.is(cmd::PLAYBACK_PAUSED) {
            data.set_playback_paused();
            Handled::Yes
        } else if cmd.is(cmd::PLAYBACK_STOPPED) {
            data.set_playback_stopped();
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_playback_ctrl(
        &mut self,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if let Some(pb_ctx) = cmd.get(cmd::PLAY_TRACKS).cloned() {
            self.player.set_tracks(pb_ctx.tracks);
            self.player.play(pb_ctx.position);
            Handled::Yes
        } else if cmd.is(cmd::PLAY_PAUSE) {
            self.player.pause();
            Handled::Yes
        } else if cmd.is(cmd::PLAY_RESUME) {
            self.player.resume();
            Handled::Yes
        } else if cmd.is(cmd::PLAY_PREVIOUS) {
            self.player.previous();
            Handled::Yes
        } else if cmd.is(cmd::PLAY_NEXT) {
            self.player.next();
            Handled::Yes
        } else if let Some(frac) = cmd.get(cmd::SEEK_TO_FRACTION) {
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
