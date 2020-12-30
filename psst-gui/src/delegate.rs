use crate::{
    cmd,
    data::{
        ArtistTracks, AudioDuration, Config, Nav, PlaybackOrigin, PlaylistTracks, SavedTracks,
        State, Track, TrackId,
    },
    ui,
    web::{Web, WebCache},
    widget::remote_image,
};
use crossbeam_channel::{Receiver, Sender};
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
    collections::HashSet, future::Future, sync::Arc, thread, thread::JoinHandle, time::Duration,
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
    player_queue: Vector<(PlaybackOrigin, Arc<Track>)>,
    player_sender: Sender<PlayerEvent>,
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
            let remote = audio_output.remote();
            Player::new(session, cdn, cache, config, remote)
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
                PlayerEvent::Loading { item } => {
                    let item: TrackId = item.item_id.into();
                    sink.submit_command(cmd::PLAYBACK_LOADING, item, Target::Auto)
                        .unwrap();
                }
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

    fn get_track(&self, id: &TrackId) -> Option<(PlaybackOrigin, Arc<Track>)> {
        self.player_queue
            .iter()
            .find(|(_origin, track)| track.id.same(id))
            .cloned()
    }

    fn set_tracks(&mut self, origin: PlaybackOrigin, tracks: Vector<Arc<Track>>) {
        self.player_queue = tracks
            .into_iter()
            .map(|track| (origin.clone(), track))
            .collect();
    }

    fn play(&mut self, position: usize) {
        let items = self
            .player_queue
            .iter()
            .map(|(_origin, track)| PlaybackItem { item_id: *track.id })
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

    fn navigate_to(&mut self, data: &mut State, nav: Nav) {
        data.history.push_back(nav.clone());
        self.navigate(data, nav);
    }

    fn navigate_back(&mut self, data: &mut State) {
        data.history.pop_back();
        self.navigate(data, data.history.last().cloned().unwrap_or(Nav::Home));
    }

    fn navigate(&mut self, data: &mut State, nav: Nav) {
        match nav {
            Nav::Home => {
                data.route = Nav::Home;
            }
            Nav::SearchResults(query) => {
                self.event_sink
                    .submit_command(cmd::GOTO_SEARCH_RESULTS, query, Target::Auto)
                    .unwrap();
            }
            Nav::AlbumDetail(link) => {
                self.event_sink
                    .submit_command(cmd::GOTO_ALBUM_DETAIL, link, Target::Auto)
                    .unwrap();
            }
            Nav::ArtistDetail(link) => {
                self.event_sink
                    .submit_command(cmd::GOTO_ARTIST_DETAIL, link, Target::Auto)
                    .unwrap();
            }
            Nav::PlaylistDetail(link) => {
                self.event_sink
                    .submit_command(cmd::GOTO_PLAYLIST_DETAIL, link, Target::Auto)
                    .unwrap();
            }
            Nav::Library => {
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
            data.library_mut().playlists.defer_default();
            self.submit_async(
                cmd::UPDATE_PLAYLISTS,
                async move { web.load_playlists().await },
            );
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_PLAYLISTS).cloned() {
            data.library_mut().playlists.resolve_or_reject(result);
            Handled::Yes
        } else if let Some(link) = cmd.get(cmd::GOTO_PLAYLIST_DETAIL).cloned() {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            data.route = Nav::PlaylistDetail(link.clone());
            data.playlist.playlist.defer(link.clone());
            data.playlist.tracks.defer(link.clone());
            self.runtime.spawn(async move {
                let result = web.load_playlist_tracks(&link.id).await;
                sink.submit_command(cmd::UPDATE_PLAYLIST_TRACKS, (link, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_PLAYLIST_TRACKS).cloned() {
            if data.playlist.tracks.is_deferred(&link) {
                data.playlist
                    .tracks
                    .resolve_or_reject(result.map(|tracks| PlaylistTracks {
                        id: link.id,
                        name: link.name,
                        tracks,
                    }));
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_library(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if cmd.is(cmd::GOTO_LIBRARY) {
            data.route = Nav::Library;
            if data.library.saved_albums.is_empty() || data.library.saved_albums.is_rejected() {
                data.library_mut().saved_albums.defer_default();
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let result = web.load_saved_albums().await;
                    sink.submit_command(cmd::UPDATE_SAVED_ALBUMS, result, Target::Auto)
                        .unwrap();
                });
            }
            if data.library.saved_tracks.is_empty() || data.library.saved_tracks.is_rejected() {
                data.library_mut().saved_tracks.defer_default();
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
            match result {
                Ok(albums) => {
                    data.common_ctx.set_saved_albums(&albums);
                    data.library_mut().saved_albums.resolve(albums);
                }
                Err(err) => {
                    data.common_ctx.set_saved_albums(&Vector::new());
                    data.library_mut().saved_albums.reject(err);
                }
            };
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SAVED_TRACKS).cloned() {
            match result {
                Ok(tracks) => {
                    data.common_ctx.set_saved_tracks(&tracks);
                    data.library_mut()
                        .saved_tracks
                        .resolve(SavedTracks { tracks });
                }
                Err(err) => {
                    data.common_ctx.set_saved_tracks(&Vector::new());
                    data.library_mut().saved_tracks.reject(err);
                }
            };
            Handled::Yes
        } else if let Some(track) = cmd.get(cmd::SAVE_TRACK).cloned() {
            let web = self.web.clone();
            let track_id = track.id.to_base62();
            data.save_track(track);
            self.runtime.spawn(async move {
                let result = web.save_track(&track_id).await;
                if result.is_err() {
                    // TODO: Refresh saved tracks.
                }
            });
            Handled::Yes
        } else if let Some(track_id) = cmd.get(cmd::UNSAVE_TRACK).cloned() {
            let web = self.web.clone();
            data.unsave_track(&track_id);
            self.runtime.spawn(async move {
                let result = web.unsave_track(&track_id.to_base62()).await;
                if result.is_err() {
                    // TODO: Refresh saved tracks.
                }
            });
            Handled::Yes
        } else if let Some(album) = cmd.get(cmd::SAVE_ALBUM).cloned() {
            let web = self.web.clone();
            let album_id = album.id.clone();
            data.save_album(album);
            self.runtime.spawn(async move {
                let result = web.save_album(&album_id).await;
                if result.is_err() {
                    // TODO: Refresh saved albums.
                }
            });
            Handled::Yes
        } else if let Some(link) = cmd.get(cmd::UNSAVE_ALBUM).cloned() {
            let web = self.web.clone();
            data.unsave_album(&link.id);
            self.runtime.spawn(async move {
                let result = web.unsave_album(&link.id).await;
                if result.is_err() {
                    // TODO: Refresh saved albums.
                }
            });
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_album(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(link) = cmd.get(cmd::GOTO_ALBUM_DETAIL).cloned() {
            data.route = Nav::AlbumDetail(link.clone());
            data.album.album.defer(link.clone());
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_album(&link.id).await;
                sink.submit_command(cmd::UPDATE_ALBUM_DETAIL, (link, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ALBUM_DETAIL).cloned() {
            if data.album.album.is_deferred(&link) {
                data.album.album.resolve_or_reject(result);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_artist(&mut self, _target: Target, cmd: &Command, data: &mut State) -> Handled {
        if let Some(album_link) = cmd.get(cmd::GOTO_ARTIST_DETAIL) {
            data.route = Nav::ArtistDetail(album_link.clone());
            // Load artist detail
            data.artist.artist.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist(&link.id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_DETAIL, (link, result), Target::Auto)
                    .unwrap();
            });
            // Load artist top tracks
            data.artist.top_tracks.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist_top_tracks(&link.id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_TOP_TRACKS, (link, result), Target::Auto)
                    .unwrap();
            });
            // Load artist's related artists
            data.artist.related_artists.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_related_artists(&link.id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_RELATED, (link, result), Target::Auto)
                    .unwrap();
            });
            // Load artist albums
            data.artist.albums.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let result = web.load_artist_albums(&link.id).await;
                sink.submit_command(cmd::UPDATE_ARTIST_ALBUMS, (link, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_DETAIL).cloned() {
            if data.artist.artist.is_deferred(&link) {
                data.artist.artist.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_ALBUMS).cloned() {
            if data.artist.albums.is_deferred(&link) {
                data.artist.albums.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_TOP_TRACKS).cloned() {
            if data.artist.top_tracks.is_deferred(&link) {
                data.artist
                    .top_tracks
                    .resolve_or_reject(result.map(|tracks| ArtistTracks {
                        id: link.id,
                        name: link.name,
                        tracks,
                    }));
            }
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_RELATED).cloned() {
            if data.artist.related_artists.is_deferred(&link) {
                data.artist.related_artists.resolve_or_reject(result);
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
            data.route = Nav::SearchResults(query.clone());
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
        if let Some(item) = cmd.get(cmd::PLAYBACK_LOADING) {
            if let Some((origin, track)) = self.player.get_track(item) {
                data.set_playback_loading(track, origin);
            } else {
                log::warn!("loaded item not found in playback queue");
            }
            Handled::Yes
        } else if let Some(item) = cmd.get(cmd::PLAYBACK_PLAYING) {
            if let Some((origin, track)) = self.player.get_track(item) {
                data.set_playback_playing(track, origin);
                data.playback.current.as_mut().map(|current| {
                    current.analysis.defer(item.clone());
                });
                let item = item.clone();
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let result = web.load_audio_analysis(&item.to_base62()).await;
                    sink.submit_command(cmd::UPDATE_AUDIO_ANALYSIS, (item, result), Target::Auto)
                        .unwrap();
                });
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
        } else if let Some((track_id, result)) = cmd.get(cmd::UPDATE_AUDIO_ANALYSIS).cloned() {
            data.playback.current.as_mut().map(|current| {
                if current.analysis.is_deferred(&track_id) {
                    current.analysis.resolve_or_reject(result);
                }
            });
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
        if let Some(payload) = cmd.get(cmd::PLAY_TRACKS).cloned() {
            self.player.set_tracks(payload.origin, payload.tracks);
            self.player.play(payload.position);
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
        } else if let Some(fraction) = cmd.get(cmd::SEEK_TO_FRACTION) {
            data.playback.current.as_ref().map(|current| {
                let position =
                    Duration::from_secs_f64(current.item.duration.as_secs_f64() * fraction);
                self.player.seek(position);
            });
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
