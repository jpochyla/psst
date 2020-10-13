use crate::{
    commands::*,
    consts,
    data::{Config, Navigation, Playback, PlaybackReport, Route, State, Track},
    database::Web,
    widgets::remote_image,
};
use druid::{
    im::Vector, AppDelegate, Application, Command, DelegateCtx, Env, Event, ExtEventSink, HotKey,
    ImageBuf, SysMods, Target, WindowId,
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
            PlayerEvent::Playing { path, duration, .. } => {
                let item = path.item_id.to_base62();
                let progress = duration.clone().into();
                let report = PlaybackReport { item, progress };
                sink.submit_command(PLAYBACK_PLAYING, report, Target::Auto)
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
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> bool {
        //
        // Common
        //
        if let Some(text) = cmd.get(COPY_TO_CLIPBOARD).cloned() {
            Application::global().clipboard().put_string(text);
            false
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
            false
        } else if let Some(payload) = cmd.get(remote_image::PROVIDE_DATA) {
            self.image_cache
                .insert(payload.location.clone(), payload.image_buf.clone());
            true
        //
        // Session
        //
        } else if cmd.is(SESSION_CONNECTED) {
            self.event_sink
                .submit_command(LOAD_PLAYLISTS, (), Target::Auto)
                .unwrap();
            true
        //
        // Navigation
        //
        } else if let Some(nav) = cmd.get(NAVIGATE_TO) {
            data.nav_stack.push_back(nav.clone());
            self.navigate(data, nav.clone());
            true
        } else if cmd.is(NAVIGATE_BACK) {
            data.nav_stack.pop_back();
            let nav = data.nav_stack.last().cloned().unwrap_or(Navigation::Home);
            self.navigate(data, nav);
            true
        //
        // Playlists
        //
        } else if cmd.is(LOAD_PLAYLISTS) {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let payload = web.load_playlists().await.unwrap();
                sink.submit_command(UPDATE_PLAYLISTS, payload, Target::Auto)
                    .unwrap();
            });
            false
        } else if let Some(playlists) = cmd.get(UPDATE_PLAYLISTS).cloned() {
            data.library.playlists = Some(playlists);
            false
        } else if let Some(playlist) = cmd.get(GOTO_PLAYLIST_DETAIL).cloned() {
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            let playlist_id = playlist.id.clone();
            data.route = Route::PlaylistDetail;
            data.playlist.tracks = None;
            data.playlist.playlist = Some(playlist);
            self.runtime.spawn(async move {
                let payload = web.load_playlist_tracks(&playlist_id).await.unwrap();
                sink.submit_command(UPDATE_PLAYLIST_TRACKS, payload, Target::Auto)
                    .unwrap();
            });
            false
        } else if let Some(tracks) = cmd.get(UPDATE_PLAYLIST_TRACKS).cloned() {
            data.playlist.tracks = Some(tracks);
            false
        //
        // Library, saved albums and tracks
        //
        } else if cmd.is(GOTO_LIBRARY) {
            data.route = Route::Library;
            if data.library.saved_albums.is_none() {
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let payload = web.load_saved_albums().await.unwrap();
                    sink.submit_command(UPDATE_SAVED_ALBUMS, payload, Target::Auto)
                        .unwrap();
                });
            }
            if data.library.saved_tracks.is_none() {
                let web = self.web.clone();
                let sink = self.event_sink.clone();
                self.runtime.spawn(async move {
                    let payload = web.load_saved_tracks().await.unwrap();
                    sink.submit_command(UPDATE_SAVED_TRACKS, payload, Target::Auto)
                        .unwrap();
                });
            }
            false
        } else if let Some(saved_albums) = cmd.get(UPDATE_SAVED_ALBUMS).cloned() {
            data.library.saved_albums = Some(saved_albums);
            false
        } else if let Some(saved_tracks) = cmd.get(UPDATE_SAVED_TRACKS).cloned() {
            data.library.saved_tracks = Some(saved_tracks);
            false
        //
        // Album detail
        //
        } else if let Some(album_id) = cmd.get(GOTO_ALBUM_DETAIL).cloned() {
            data.route = Route::AlbumDetail;
            data.album.id = album_id.clone();
            data.album.album = None;
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let payload = web.load_album(&album_id).await.unwrap();
                sink.submit_command(UPDATE_ALBUM_DETAIL, payload, Target::Auto)
                    .unwrap();
            });
            false
        } else if let Some(album) = cmd.get(UPDATE_ALBUM_DETAIL).cloned() {
            data.album.album = Some(album);
            false
        //
        // Artist detail
        //
        } else if let Some(artist_id) = cmd.get(GOTO_ARTIST_DETAIL) {
            data.route = Route::ArtistDetail;
            data.artist.id = artist_id.clone();
            data.artist.artist = None;
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let payload = web.load_artist(&id).await.unwrap();
                sink.submit_command(UPDATE_ARTIST_DETAIL, payload, Target::Auto)
                    .unwrap();
            });
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let payload = web.load_artist_albums(&id).await.unwrap();
                sink.submit_command(UPDATE_ARTIST_ALBUMS, payload, Target::Auto)
                    .unwrap();
            });
            let id = artist_id.clone();
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let payload = web.load_artist_top_tracks(&id).await.unwrap();
                sink.submit_command(UPDATE_ARTIST_TOP_TRACKS, payload, Target::Auto)
                    .unwrap();
            });
            false
        } else if let Some(artist) = cmd.get(UPDATE_ARTIST_DETAIL).cloned() {
            data.artist.artist = Some(artist);
            false
        } else if let Some(albums) = cmd.get(UPDATE_ARTIST_ALBUMS).cloned() {
            data.artist.albums = Some(albums);
            false
        } else if let Some(top_tracks) = cmd.get(UPDATE_ARTIST_TOP_TRACKS).cloned() {
            data.artist.top_tracks = Some(top_tracks);
            false
        //
        // Search
        //
        } else if let Some(query) = cmd.get(GOTO_SEARCH_RESULTS).cloned() {
            data.route = Route::SearchResults;
            let web = self.web.clone();
            let sink = self.event_sink.clone();
            self.runtime.spawn(async move {
                let payload = web.search(&query).await.unwrap();
                sink.submit_command(UPDATE_SEARCH_RESULTS, payload, Target::Auto)
                    .unwrap();
            });
            false
        } else if let Some((artists, albums, tracks)) = cmd.get(UPDATE_SEARCH_RESULTS).cloned() {
            data.search.artists = artists;
            data.search.albums = albums;
            data.search.tracks = tracks;
            false
        //
        // Playback status
        //
        } else if let Some(report) = cmd.get(PLAYBACK_PLAYING).cloned() {
            let updated_playback = Playback {
                is_playing: true,
                progress: Some(report.progress),
                item: self.player.get_track(&report.item),
                analysis: None,
            };
            let current_track_id = data
                .playback
                .item
                .as_ref()
                .and_then(|track| track.id.as_ref());
            let updated_track_id = updated_playback
                .item
                .as_ref()
                .and_then(|track| track.id.as_ref());
            if current_track_id != updated_track_id {
                if let Some(id) = updated_track_id {
                    ctx.submit_command(LOAD_AUDIO_ANALYSIS.with(id.clone()));
                }
                data.playback.analysis = None;
            }
            data.playback = updated_playback;
            false
        } else if cmd.is(PLAYBACK_PAUSED) {
            data.playback.is_playing = false;
            false
        //
        // Audio analysis
        //
        } else if let Some(_track_id) = cmd.get(LOAD_AUDIO_ANALYSIS).cloned() {
            // let web = self.web.clone();
            // let sink = self.event_sink.clone();
            // self.runtime.spawn(async move {
            //     let payload = web.analyze_track(&track_id).await.unwrap();
            //     sink.submit_command(UPDATE_AUDIO_ANALYSIS, payload, Target::Auto)
            //         .unwrap();
            // });
            false
        } else if let Some(analysis) = cmd.get(UPDATE_AUDIO_ANALYSIS).cloned() {
            data.playback.analysis = Some(analysis);
            false
        //
        // Playback control
        //
        } else if let Some((tracks, position)) = cmd.get(PLAY_TRACKS).cloned() {
            self.player.set_tracks(tracks);
            self.player.play(position);
            false
        } else if cmd.is(PLAY_PAUSE) {
            self.player.pause();
            false
        } else if cmd.is(PLAY_RESUME) {
            self.player.resume();
            false
        } else if cmd.is(PLAY_PREVIOUS) {
            self.player.previous();
            false
        } else if cmd.is(PLAY_NEXT) {
            self.player.next();
            false
        } else if let Some(frac) = cmd.get(SEEK_TO_FRACTION) {
            if let Some(track) = &data.playback.item {
                log::info!("seeking to {}", frac);
                let position = Duration::from_secs_f64(track.duration.as_secs_f64() * frac);
                self.player.seek(position);
            }
            false
        } else {
            true
        }
    }
}
