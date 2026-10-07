//! Native Connect receiver. No browser, official desktop client or Web API polling.
use crate::{
    cmd,
    data::{
        AlbumLink, AppState, ArtistLink, Config, Image, Playable, PlaybackOrigin, PlaybackPayload,
        PlaybackState, PlaylistLink, QueueBehavior, QueueEntry, Track, TrackId,
    },
};
use druid::{
    widget::Controller, Data, Env, Event, EventCtx, ExtEventSink, LifeCycle, LifeCycleCtx,
    Selector, TimerToken, UpdateCtx, Widget,
};
use futures_util::StreamExt;
use librespot_connect::{
    ConnectConfig, LoadContextOptions, LoadRequest, LoadRequestOptions, Options, PlayingTrack,
    Spirc,
};
use librespot_core::{
    authentication::Credentials, config::DeviceType, dealer::protocol::Message, Session,
    SessionConfig,
};
use librespot_metadata::audio::item::{AudioItem, UniqueFields};
use librespot_playback::{
    audio_backend::{Sink, SinkError, SinkResult},
    config::{Bitrate, PlayerConfig},
    convert::Converter,
    decoder::AudioPacket,
    mixer::{self, MixerConfig},
    player::{Player, PlayerEvent},
};
use librespot_protocol::{
    authentication::AuthenticationType,
    connect::{Cluster, ClusterUpdate},
    player::ProvidedTrack,
};
use psst_core::audio::{
    output::{AudioOutput, AudioSink, DefaultAudioOutput},
    resample::ResamplingQuality,
    source::{AudioSource, ResampledSource, StereoMappedSource},
};
use rb::{Consumer, Producer, RbConsumer, RbProducer, SpscRb, RB};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

pub const TOGGLE: Selector = Selector::new("native-connect.toggle");
pub const RETRY: Selector = Selector::new("native-connect.retry");
const NOTICE: Selector<(u64, Notice)> = Selector::new("native-connect.notice");

enum Notice {
    Ready,
    Failed(String),
    CommandFailed(String),
    Player(PlayerEvent),
    Cluster(ClusterUpdate),
    LocalQueue(ClusterUpdate),
}
enum Action {
    PlayQueued { uid: String, uri: String },
    Add(String),
    Load(LoadRequest),
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Seek(u32),
    Volume(u16),
    Behavior(QueueBehavior),
    Suspend,
    Quit,
}

// Feed decoded Connect audio into the fork's existing native output. The real-time
// callback never waits; the decoder has bounded backpressure and shutdown latency.
struct BufferedAudio {
    consumer: Consumer<f32>,
}
impl AudioSource for BufferedAudio {
    fn write(&mut self, output: &mut [f32]) -> usize {
        self.consumer.read(output).unwrap_or(0)
    }
    fn channel_count(&self) -> usize {
        2
    }
    fn sample_rate(&self) -> u32 {
        44_100
    }
}
struct NativeSink {
    output: Arc<DefaultAudioOutput>,
    producer: Option<Producer<f32>>,
    stopping: Arc<std::sync::atomic::AtomicBool>,
}
impl Sink for NativeSink {
    fn start(&mut self) -> SinkResult<()> {
        let buffer = SpscRb::new(44_100);
        self.producer = Some(buffer.producer());
        let source = BufferedAudio {
            consumer: buffer.consumer(),
        };
        let sink = self.output.sink();
        if sink.sample_rate() == 44_100 && sink.channel_count() == 2 {
            sink.play(source);
        } else {
            let source = ResampledSource::new(
                source,
                sink.sample_rate(),
                ResamplingQuality::SincMediumQuality,
            );
            sink.play(StereoMappedSource::new(source, sink.channel_count()));
        }
        sink.resume();
        Ok(())
    }
    fn stop(&mut self) -> SinkResult<()> {
        self.output.sink().stop();
        self.producer = None;
        Ok(())
    }
    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let converted = converter.f64_to_f32(samples);
        let producer = self
            .producer
            .as_ref()
            .ok_or_else(|| SinkError::NotConnected("Audio en pausa".into()))?;
        write_buffer(producer, converted.as_slice(), &self.stopping)
    }
}

fn write_buffer(
    producer: &Producer<f32>,
    mut samples: &[f32],
    stopping: &std::sync::atomic::AtomicBool,
) -> SinkResult<()> {
    let mut last_write = Instant::now();
    while !samples.is_empty() {
        if stopping.load(std::sync::atomic::Ordering::Acquire) {
            return Err(SinkError::NotConnected("Receptor cerrado".into()));
        }
        let written = match producer.write(samples) {
            Ok(written) => written,
            Err(rb::RbError::Full) => 0,
            Err(error) => return Err(SinkError::OnWrite(error.to_string())),
        };
        samples = &samples[written..];
        if written == 0 {
            if last_write.elapsed() > Duration::from_secs(2) {
                return Err(SinkError::NotConnected(
                    "La salida de audio se desconectó".into(),
                ));
            }
            thread::sleep(Duration::from_millis(2));
        } else {
            last_write = Instant::now();
        }
    }
    Ok(())
}

struct Worker {
    quality: tokio::sync::watch::Sender<usize>,
    sender: tokio::sync::mpsc::Sender<Action>,
    epoch: u64,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.sender.try_send(Action::Quit);
    }
}

impl Worker {
    fn start(config: Config, sink: ExtEventSink) -> Self {
        let epoch = rand::random();
        let (sender, mut commands) = tokio::sync::mpsc::channel(64);
        let (quality, mut quality_changes) = tokio::sync::watch::channel(config.playback().bitrate);
        thread::spawn(move || {
            let notify = |notice| {
                let _ = sink.submit_command(NOTICE, (epoch, notice), druid::Target::Global);
            };
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    notify(Notice::Failed(error.to_string()));
                    return;
                }
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(async {
                let original = config.session();
                let credentials = Credentials {
                    username: original.login_creds.username,
                    auth_data: original.login_creds.auth_data,
                    auth_type: match original.login_creds.auth_type as i32 {
                        0 => AuthenticationType::AUTHENTICATION_USER_PASS,
                        1 => AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
                        2 => AuthenticationType::AUTHENTICATION_STORED_FACEBOOK_CREDENTIALS,
                        3 => AuthenticationType::AUTHENTICATION_SPOTIFY_TOKEN,
                        _ => return Err("Tipo de credencial no compatible. Vuelve a conectar tu cuenta.".to_string()),
                    },
                };
                let session_config = SessionConfig {
                    device_id: config.connect_device_id.clone(),
                    proxy: original.proxy_url.as_deref().map(url::Url::parse).transpose().map_err(|e| e.to_string())?,
                    autoplay: Some(false),
                    ..SessionConfig::default()
                };
                let audio_dir = Config::cache_dir().map(|p| p.join("connect-audio"));
                let cache = librespot_core::cache::Cache::new(None::<&std::path::Path>, None::<&std::path::Path>, audio_dir.as_deref(), Some(2 * 1024 * 1024 * 1024)).map_err(|e| e.to_string())?;
                let session = Session::new(session_config, Some(cache));
                session.spclient().set_strategy(librespot_core::spclient::RequestStrategy::TryTimes(3));
                let mut clusters = session.dealer().listen_for("hm://connect-state/v1/cluster", Message::from_raw::<ClusterUpdate>).map_err(|e| e.to_string())?;
                let bitrate = match config.playback().bitrate { 96 => Bitrate::Bitrate96, 160 => Bitrate::Bitrate160, _ => Bitrate::Bitrate320 };
                let mixer = mixer::find(Some("softvol")).ok_or("Mezclador no disponible")?(MixerConfig::default()).map_err(|e| e.to_string())?;
                let output = Arc::new(DefaultAudioOutput::open().map_err(|e| e.to_string())?);
                let stopping = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let sink_stopping = stopping.clone();
                let player = Player::new(PlayerConfig {
                    bitrate,
                    normalisation: true,
                    normalisation_pregain_db: config.playback().pregain as f64,
                    position_update_interval: Some(Duration::from_millis(500)),
                    ..PlayerConfig::default()
                }, session.clone(), mixer.get_soft_volume(), move || Box::new(NativeSink { output, producer: None, stopping: sink_stopping }));
                let mut events = player.get_player_event_channel();
                let setup = Spirc::new(ConnectConfig {
                    name: format!("Xpotify · {}", std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into())),
                    device_type: DeviceType::Computer,
                    initial_volume: volume(config.volume),
                    ..ConnectConfig::default()
                }, session.clone(), credentials, player.clone(), mixer);
                let (spirc, task) = tokio::time::timeout(Duration::from_secs(35), setup).await.map_err(|_| "Spotify Connect tardó demasiado en conectar".to_string())?.map_err(|e| e.to_string())?;
                let mut local_queue = spirc.queue_state();
                // Stay inactive on startup: do not steal playback from a phone.
                let mut task = tokio::spawn(task);
                notify(Notice::Ready);
                log::info!("Native Spotify Connect receiver registered");
                let mut health_check = tokio::time::interval(Duration::from_secs(1));
                let mut last_progress: Option<Instant> = None;
                let failure;
                loop {
                    tokio::select! {
                        biased;
                        _ = &mut task => {
                            failure = Some("La conexión con Spotify se cerró. Reconectando…".to_string());
                            break;
                        }
                        _ = health_check.tick() => {
                            if session.is_invalid() || last_progress.is_some_and(|at| at.elapsed() > Duration::from_secs(25)) {
                                failure = Some("La reproducción se interrumpió. Reconectando…".to_string());
                                break;
                            }
                        }
                        changed = quality_changes.changed() => {
                            if changed.is_ok() {
                                let value = *quality_changes.borrow_and_update();
                                player.set_bitrate(match value { 96 => Bitrate::Bitrate96, 160 => Bitrate::Bitrate160, _ => Bitrate::Bitrate320 });
                                log::info!("Audio quality queued for next track: {value} kb/s");
                            }
                        }
                        action = commands.recv() => {
                            let result = match action {
                                Some(Action::Load(request)) => spirc.activate().and_then(|_| spirc.load(request)),
                                Some(Action::Add(uri)) => spirc.add_to_queue(uri),
                                Some(Action::PlayQueued { uid, uri }) => spirc.play_queued(uid, uri),
                                Some(Action::Play) => spirc.play(),
                                Some(Action::Pause) => spirc.pause(),
                                Some(Action::Toggle) => spirc.play_pause(),
                                Some(Action::Next) => spirc.next(),
                                Some(Action::Previous) => spirc.prev(),
                                Some(Action::Seek(ms)) => spirc.set_position_ms(ms),
                                Some(Action::Volume(value)) => spirc.set_volume(value),
                                Some(Action::Behavior(behavior)) => spirc.shuffle(behavior == QueueBehavior::Random)
                                    .and_then(|_| spirc.repeat(behavior == QueueBehavior::LoopAll))
                                    .and_then(|_| spirc.repeat_track(behavior == QueueBehavior::LoopTrack)),
                                Some(Action::Suspend) => spirc.disconnect(true),
                                Some(Action::Quit) | None => { failure = None; break; },
                            };
                            if let Err(error) = result { notify(Notice::CommandFailed(error.to_string())); }
                        }
                        event = events.recv() => {
                            if session.is_invalid() || event.is_none() {
                                failure = Some("El reproductor perdió la conexión. Reconectando…".to_string());
                                break;
                            }
                            if let Some(event) = event {
                                match &event {
                                    PlayerEvent::Playing { .. } | PlayerEvent::PositionChanged { .. }
                                    | PlayerEvent::PositionCorrection { .. } => last_progress = Some(Instant::now()),
                                    PlayerEvent::Paused { .. } | PlayerEvent::Stopped { .. }
                                    | PlayerEvent::EndOfTrack { .. } => last_progress = None,
                                    _ => {}
                                }
                                notify(Notice::Player(event));
                            }
                        }
                        changed = local_queue.changed() => {
                            if changed.is_ok() {
                                let player = local_queue.borrow_and_update().clone();
                                if let Some(player) = player {
                                    log::debug!("Native queue: current={} first={} upcoming={} shuffle={}",
                                        player.track.as_ref().map_or("", |track| track.uri.as_str()),
                                        player.next_tracks.first().map_or("", |track| track.uri.as_str()),
                                        player.next_tracks.len(), player.options.shuffling_context);
                                    let mut cluster = Cluster::new();
                                    cluster.active_device_id = config.connect_device_id.clone();
                                    cluster.player_state = Some(player).into();
                                    let mut update = ClusterUpdate::new();
                                    update.cluster = Some(cluster).into();
                                    notify(Notice::LocalQueue(update));
                                }
                            }
                        }
                        update = clusters.next() => { if let Some(Ok(update)) = update { notify(Notice::Cluster(update)); } }
                    }
                }
                stopping.store(true, std::sync::atomic::Ordering::Release);
                let _ = spirc.shutdown();
                if !task.is_finished() {
                    let _ = tokio::time::timeout(Duration::from_secs(2), &mut task).await;
                }
                task.abort();
                player.stop();
                session.shutdown();
                failure.map_or(Ok::<(), String>(()), Err)
            })
            }));
            match result {
                Ok(Err(error)) => notify(Notice::Failed(error)),
                Err(_) => notify(Notice::Failed(
                    "No se pudo iniciar el receptor o la salida de audio.".into(),
                )),
                _ => {}
            }
            runtime.shutdown_timeout(Duration::from_secs(2));
        });
        Self {
            sender,
            epoch,
            quality,
        }
    }
}

fn volume(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * u16::MAX as f64).round() as u16
}
fn id(uri: &str, kind: &str) -> Option<String> {
    let value = uri.strip_prefix(&format!("spotify:{kind}:"))?;
    (value.len() == 22 && value.bytes().all(|b| b.is_ascii_alphanumeric()))
        .then(|| value.to_string())
}
fn load(
    payload: &PlaybackPayload,
    progress: Duration,
    playing: bool,
    behavior: QueueBehavior,
) -> Result<LoadRequest, String> {
    let current = payload
        .items
        .get(payload.position)
        .ok_or("No hay una canción seleccionada")?;
    if payload
        .items
        .iter()
        .any(|item| matches!(item, Playable::Track(t) if t.is_local))
    {
        return Err("Los archivos locales usan el motor sin Connect. Desactiva Connect nativo en Dispositivos para escucharlos.".into());
    }
    let options = LoadRequestOptions {
        start_playing: playing,
        seek_to: progress.as_millis().min(u32::MAX as u128) as u32,
        playing_track: Some(PlayingTrack::Index(payload.position as u32)),
        context_options: Some(LoadContextOptions::Options(Options {
            shuffle: behavior == QueueBehavior::Random,
            repeat: behavior == QueueBehavior::LoopAll,
            repeat_track: behavior == QueueBehavior::LoopTrack,
        })),
    };
    let uris: Vec<_> = payload
        .items
        .iter()
        .filter_map(|item| item.id().to_uri())
        .collect();
    if uris.len() != payload.items.len() || current.id().to_uri().is_none() {
        return Err("La lista contiene identificadores no compatibles".into());
    }
    Ok(LoadRequest::from_tracks(uris, options))
}

fn from_provided(track: &ProvidedTrack) -> Option<Playable> {
    let track_id = id(&track.uri, "track")?;
    let metadata = &track.metadata;
    let name = metadata
        .get("title")
        .or_else(|| metadata.get("name"))
        .cloned()
        .unwrap_or_else(|| "Canción en la cola de Spotify".into());
    let artist = metadata
        .get("artist_name")
        .cloned()
        .unwrap_or_else(|| "Spotify".into());
    let album = id(&track.album_uri, "album").map(|album_id| serde_json::json!({"id":album_id,"name":metadata.get("album_title").cloned().unwrap_or_default(),"images":[]}));
    let artists = id(&track.artist_uri, "artist")
        .map(|artist_id| vec![serde_json::json!({"id":artist_id,"name":artist})])
        .unwrap_or_default();
    serde_json::from_value::<Track>(serde_json::json!({"id":track_id,"name":name,"album":album,"artists":artists,"duration_ms":metadata.get("duration").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0),"disc_number":1,"track_number":1,"explicit":false,"is_local":false})).ok().map(|t| Playable::Track(Arc::new(t)))
}

fn from_audio(item: &AudioItem, album_id: Option<String>) -> Option<Playable> {
    let UniqueFields::Track {
        artists,
        album,
        popularity,
        number,
        disc_number,
        ..
    } = &item.unique_fields
    else {
        return None;
    };
    let track_id: TrackId = item.track_id.to_id().ok()?.try_into().ok()?;
    let images = item
        .covers
        .iter()
        .map(|cover| Image {
            url: cover.url.clone().into(),
            width: usize::try_from(cover.width).ok(),
            height: usize::try_from(cover.height).ok(),
        })
        .collect();
    Some(Playable::Track(Arc::new(Track {
        id: track_id,
        name: item.name.clone().into(),
        album: album_id.map(|id| AlbumLink {
            id: id.into(),
            name: album.clone().into(),
            images,
        }),
        artists: artists
            .iter()
            .filter_map(|artist| {
                Some(ArtistLink {
                    id: artist.id.to_id().ok()?.into(),
                    name: artist.name.clone().into(),
                })
            })
            .collect(),
        duration: Duration::from_millis(item.duration_ms as u64),
        disc_number: *disc_number as usize,
        track_number: *number as usize,
        explicit: item.is_explicit,
        is_local: false,
        local_path: None,
        is_playable: Some(true),
        popularity: Some(*popularity as u32),
        track_pos: 0,
        lyrics: None,
    })))
}

struct PendingLoad {
    payload: PlaybackPayload,
    progress: Duration,
    playing: bool,
}

pub struct NativeConnectController {
    worker: Option<Worker>,
    timer: TimerToken,
    next_retry: Instant,
    failures: u32,
    restore_pending: bool,
    pending_load: Option<PendingLoad>,
    in_flight_load: Option<PendingLoad>,
    audio_item: Option<Box<AudioItem>>,
    cluster: Option<ClusterUpdate>,
    last_volume: Option<u16>,
    local_queue_observed: bool,
    next_audio_check: Instant,
    #[cfg(feature = "cpal")]
    audio_device: Option<String>,
}
impl Default for NativeConnectController {
    fn default() -> Self {
        Self {
            worker: None,
            timer: TimerToken::INVALID,
            next_retry: Instant::now(),
            failures: 0,
            restore_pending: true,
            pending_load: None,
            in_flight_load: None,
            audio_item: None,
            cluster: None,
            last_volume: None,
            local_queue_observed: false,
            next_audio_check: Instant::now(),
            #[cfg(feature = "cpal")]
            audio_device: DefaultAudioOutput::devices().0,
        }
    }
}
impl NativeConnectController {
    fn connection_failed(&mut self, state: &mut AppState) {
        // Preserve a newer user selection, and never resume another device's playback.
        if state.connect.selected.is_none() {
            if self.pending_load.is_none() {
                if let Some(pending) = self.in_flight_load.take() {
                    self.pending_load = Some(pending);
                } else if let Some((payload, progress)) = Self::payload(state) {
                    self.pending_load = Some(PendingLoad {
                        payload,
                        progress,
                        playing: state.playback.state == PlaybackState::Playing,
                    });
                }
            }
            state.playback.state = PlaybackState::Paused;
        }
        self.worker = None;
        self.in_flight_load = None;
        state.connect.native_ready = false;
        self.restore_pending = true;
        self.failures = (self.failures + 1).min(5);
        self.next_retry = Instant::now() + Duration::from_secs((1 << self.failures).min(15));
    }

    fn send(&mut self, state: &mut AppState, action: Action) {
        let was_playing = self
            .in_flight_load
            .as_ref()
            .map_or(state.playback.state == PlaybackState::Playing, |pending| {
                pending.playing
            });
        let playing = match &action {
            Action::Pause => Some(false),
            Action::Play => Some(true),
            Action::Toggle => Some(!was_playing),
            _ => None,
        };
        let seek = match &action {
            Action::Seek(ms) => Some(Duration::from_millis(*ms as u64)),
            _ => None,
        };
        let update = |pending: &mut PendingLoad| {
            if let Some(playing) = playing {
                pending.playing = playing;
            }
            if let Some(progress) = seek {
                pending.progress = progress;
            }
        };
        if let Some(worker) = &self.worker {
            match worker.sender.try_send(action) {
                Ok(()) => {
                    // The decoder can be blocked waiting for the network. Retain
                    // transport intent even before its acknowledgement arrives.
                    if let Some(playing) = playing {
                        state.playback.state = if playing {
                            PlaybackState::Playing
                        } else {
                            PlaybackState::Paused
                        };
                    }
                    if let Some(pending) = &mut self.in_flight_load {
                        update(pending);
                    }
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    state.info_alert("Spotify está procesando los controles. Espera un momento.");
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    self.connection_failed(state);
                    if let Some(pending) = &mut self.pending_load {
                        update(pending);
                    }
                    self.next_retry = Instant::now();
                    state.info_alert("Reconectando con Spotify…");
                }
            }
        } else {
            state.info_alert("Conectando con Spotify… Consulta el estado en Dispositivos.");
        }
    }
    fn payload(state: &AppState) -> Option<(PlaybackPayload, Duration)> {
        let current = state.playback.now_playing.as_ref()?;
        let position = state
            .playback
            .queue
            .iter()
            .position(|q| q.item.id() == current.item.id());
        let (items, position) = match position {
            Some(p) => (
                state
                    .playback
                    .queue
                    .iter()
                    .map(|q| q.item.clone())
                    .collect(),
                p,
            ),
            None => (druid::im::vector![current.item.clone()], 0),
        };
        Some((
            PlaybackPayload {
                items,
                position,
                origin: current.origin.clone(),
            },
            current.progress,
        ))
    }
    fn load(
        &mut self,
        state: &mut AppState,
        payload: &PlaybackPayload,
        progress: Duration,
        playing: bool,
    ) {
        match load(payload, progress, playing, state.playback.queue_behavior) {
            Ok(request) => {
                let sent = if state.connect.native_ready {
                    self.worker
                        .as_ref()
                        .map(|worker| worker.sender.try_send(Action::Load(request)))
                } else {
                    None
                };
                if !matches!(sent, Some(Ok(()))) {
                    let first = self.pending_load.is_none();
                    if matches!(
                        sent,
                        Some(Err(tokio::sync::mpsc::error::TrySendError::Closed(_)))
                    ) {
                        self.worker = None;
                        state.connect.native_ready = false;
                    }
                    self.pending_load = Some(PendingLoad {
                        payload: payload.clone(),
                        progress,
                        playing,
                    });
                    if first {
                        if self.worker.is_none() {
                            self.next_retry = Instant::now();
                        }
                        state.info_alert("Conectando con Spotify… Tu selección se reproducirá cuando esté listo.");
                    }
                    return;
                }
                self.pending_load = None;
                self.in_flight_load = Some(PendingLoad {
                    payload: payload.clone(),
                    progress,
                    playing,
                });
                self.restore_pending = false;
                state.engine_queue = None;
                state.playback.queue = payload
                    .items
                    .iter()
                    .map(|item| QueueEntry {
                        item: item.clone(),
                        origin: payload.origin.clone(),
                    })
                    .collect();
                state.playback.up_next = state
                    .playback
                    .queue
                    .iter()
                    .skip(payload.position + 1)
                    .cloned()
                    .collect();
            }
            Err(error) => state.error_alert(error),
        }
    }
    fn flush_pending(&mut self, state: &mut AppState) {
        if state.connect.selected.is_some() {
            self.pending_load = None;
            self.in_flight_load = None;
        } else if state.connect.native_ready {
            if let Some(pending) = self.pending_load.as_ref() {
                let payload = pending.payload.clone();
                let progress = pending.progress;
                let playing = pending.playing;
                self.load(state, &payload, progress, playing);
            }
        }
    }
    fn notice(&mut self, ctx: &mut EventCtx, state: &mut AppState, notice: &Notice) {
        match notice {
            Notice::CommandFailed(error) => state.error_alert(format!("Connect: {error}")),
            Notice::Ready => {
                self.local_queue_observed = false;
                state.connect.native_ready = true;
                self.failures = 0;
                state.connect.native_status = "Connect nativo listo. Selecciona Xpotify en Spotify del teléfono. Las escuchas del motor nativo no se reportan al historial de Spotify.".into();
                self.flush_pending(state);
            }
            Notice::Failed(error) => {
                self.connection_failed(state);
                state.connect.native_status = format!("{error} Puedes reintentar aquí.");
            }
            Notice::Cluster(update) | Notice::LocalQueue(update) => {
                let local = matches!(notice, Notice::LocalQueue(_));
                if !local
                    && self.local_queue_observed
                    && update.cluster.as_ref().is_some_and(|cluster| {
                        cluster.active_device_id == state.config.connect_device_id
                    })
                {
                    return; // A lagging server echo must not overwrite this engine's queue.
                }
                self.local_queue_observed = local;
                self.cluster = Some(update.clone());
                if let Some(cluster) = update.cluster.as_ref() {
                    if cluster.active_device_id != state.config.connect_device_id
                        && !cluster.active_device_id.is_empty()
                    {
                        self.pending_load = None;
                        self.in_flight_load = None;
                    }
                    if cluster.active_device_id == state.config.connect_device_id {
                        // A phone can transfer back to this receiver while paused.
                        // Retire requests targeting the previous remote device.
                        if state.connect.selected.is_some() && !state.connect.busy {
                            state.connect.selected = None;
                            state.connect.epoch += 1;
                            state.connect.pending_start = false;
                            state.connect.local_volume = None;
                        }
                        if let Some(player) = cluster.player_state.as_ref() {
                            state.engine_queue = None;
                            let origin = id(&player.context_uri, "playlist")
                                .map(|playlist_id| {
                                    PlaybackOrigin::Playlist(PlaylistLink {
                                        id: playlist_id.into(),
                                        name: player
                                            .context_metadata
                                            .get("context_description")
                                            .cloned()
                                            .unwrap_or_else(|| "Playlist de Spotify".into())
                                            .into(),
                                    })
                                })
                                .or_else(|| {
                                    id(&player.context_uri, "album").map(|album_id| {
                                        PlaybackOrigin::Album(AlbumLink {
                                            id: album_id.into(),
                                            name: player
                                                .context_metadata
                                                .get("context_description")
                                                .cloned()
                                                .unwrap_or_else(|| "Álbum de Spotify".into())
                                                .into(),
                                            images: druid::im::Vector::new(),
                                        })
                                    })
                                });
                            if let Some(options) = player.options.as_ref() {
                                let behavior = if options.repeating_track {
                                    QueueBehavior::LoopTrack
                                } else if options.shuffling_context {
                                    QueueBehavior::Random
                                } else if options.repeating_context {
                                    QueueBehavior::LoopAll
                                } else {
                                    QueueBehavior::Sequential
                                };
                                if state.playback.queue_behavior != behavior {
                                    state.set_queue_behavior(behavior);
                                }
                            }
                            let mut known = std::collections::HashMap::new();
                            for entry in state.playback.queue.iter().chain(state.added_queue.iter())
                            {
                                known
                                    .entry(entry.item.id())
                                    .or_insert_with(|| entry.clone());
                            }
                            state.playback.up_next = player
                                .next_tracks
                                .iter()
                                .filter_map(|provided| {
                                    let item_id =
                                        psst_core::item_id::ItemId::from_uri(&provided.uri)?;
                                    let mut entry = match known.get(&item_id) {
                                        Some(entry) => entry.clone(),
                                        None => QueueEntry {
                                            item: from_provided(provided)?,
                                            origin: PlaybackOrigin::Home,
                                        },
                                    };
                                    if let Some(origin) = &origin {
                                        entry.origin = origin.clone();
                                    }
                                    Some(entry)
                                })
                                .collect();
                            if let Some(track) = player.track.as_ref() {
                                if let Some(origin) = &origin {
                                    if let Some(current_id) =
                                        psst_core::item_id::ItemId::from_uri(&track.uri)
                                    {
                                        if let Some(index) = state
                                            .playback
                                            .queue
                                            .iter()
                                            .position(|q| q.item.id() == current_id)
                                        {
                                            let mut entry = state.playback.queue[index].clone();
                                            entry.origin = origin.clone();
                                            state.playback.queue.set(index, entry);
                                        }
                                        if let Some(np) = &mut state.playback.now_playing {
                                            if np.item.id() == current_id {
                                                np.origin = origin.clone();
                                            }
                                        }
                                        state.common_ctx_mut().playing_origin =
                                            Some(origin.clone());
                                    }
                                }
                                let album_id = id(&track.album_uri, "album");
                                if let Some(audio) =
                                    self.audio_item.as_ref().filter(|a| a.uri == track.uri)
                                {
                                    if let Some(item) = from_audio(audio, album_id) {
                                        if let Some(index) = state
                                            .playback
                                            .queue
                                            .iter()
                                            .position(|q| q.item.id() == item.id())
                                        {
                                            let mut entry = state.playback.queue[index].clone();
                                            if let (Playable::Track(old), Playable::Track(new)) =
                                                (&entry.item, &item)
                                            {
                                                let mut track = new.as_ref().clone();
                                                if track.album.is_none() {
                                                    track.album = old.album.clone();
                                                }
                                                entry.item = Playable::Track(Arc::new(track));
                                            }
                                            state.playback.queue.set(index, entry.clone());
                                            if let Some(np) = &mut state.playback.now_playing {
                                                if np.item.id() == entry.item.id() {
                                                    np.item = entry.item;
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(playable) = from_provided(track) {
                                    if state.queued_entry(playable.id()).is_none() {
                                        state.playback.queue.push_back(QueueEntry {
                                            item: playable,
                                            origin: PlaybackOrigin::Home,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Notice::Player(event) => match event {
                PlayerEvent::TrackChanged { audio_item } => {
                    let album_id = self
                        .cluster
                        .as_ref()
                        .and_then(|u| u.cluster.as_ref())
                        .and_then(|c| c.player_state.as_ref())
                        .and_then(|p| p.track.as_ref())
                        .filter(|t| t.uri == audio_item.uri)
                        .and_then(|t| id(&t.album_uri, "album"));
                    if let Some(playable) = from_audio(audio_item, album_id) {
                        if let Some(index) = state
                            .playback
                            .queue
                            .iter()
                            .position(|entry| entry.item.id() == playable.id())
                        {
                            let mut entry = state.playback.queue[index].clone();
                            if let (Playable::Track(old), Playable::Track(new)) =
                                (&entry.item, &playable)
                            {
                                let mut track = new.as_ref().clone();
                                if track.album.is_none() {
                                    track.album = old.album.clone();
                                }
                                entry.item = Playable::Track(Arc::new(track));
                            } else {
                                entry.item = playable;
                            }
                            state.playback.queue.set(index, entry);
                        } else {
                            state.playback.queue.push_back(QueueEntry {
                                item: playable,
                                origin: PlaybackOrigin::Home,
                            });
                        }
                    }
                    self.audio_item = Some(audio_item.clone());
                }
                PlayerEvent::Playing {
                    track_id,
                    position_ms,
                    ..
                }
                | PlayerEvent::Paused {
                    track_id,
                    position_ms,
                    ..
                } => {
                    self.restore_pending = false;
                    self.in_flight_load = None;
                    if matches!(event, PlayerEvent::Playing { .. })
                        && state.connect.selected.is_some()
                    {
                        state.connect.selected = None;
                        state.connect.epoch += 1;
                        state.connect.busy = false;
                        state.connect.pending_start = false;
                        state.connect.local_volume = None;
                    }
                    if state.connect.selected.is_none() {
                        if let Some(item_id) = track_id
                            .to_uri()
                            .ok()
                            .and_then(|uri| psst_core::item_id::ItemId::from_uri(&uri))
                        {
                            ctx.submit_command(
                                cmd::PLAYBACK_PLAYING
                                    .with((item_id, Duration::from_millis(*position_ms as u64))),
                            );
                            if matches!(event, PlayerEvent::Paused { .. }) {
                                ctx.submit_command(cmd::PLAYBACK_PAUSING);
                            }
                        }
                    }
                }
                PlayerEvent::PositionChanged { position_ms, .. }
                | PlayerEvent::PositionCorrection { position_ms, .. }
                | PlayerEvent::Seeked { position_ms, .. }
                    if state.connect.selected.is_none() =>
                {
                    ctx.submit_command(
                        cmd::PLAYBACK_PROGRESS.with(Duration::from_millis(*position_ms as u64)),
                    );
                }
                PlayerEvent::VolumeChanged { volume } if state.connect.selected.is_none() => {
                    self.last_volume = Some(*volume);
                    state.playback.volume =
                        (*volume as f64 / u16::MAX as f64 * 10_000.0).round() / 10_000.0;
                }
                PlayerEvent::Stopped { .. } if state.connect.selected.is_none() => {
                    ctx.submit_command(cmd::PLAYBACK_PAUSING);
                }
                PlayerEvent::Unavailable { .. } => {
                    state.error_alert("Spotify no pudo cargar esta canción en Connect.");
                }
                _ => {}
            },
        }
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for NativeConnectController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Timer(token) = event {
            if *token == self.timer {
                self.flush_pending(state);
                #[cfg(feature = "cpal")]
                if state.config.native_connect && Instant::now() >= self.next_audio_check {
                    self.next_audio_check = Instant::now() + Duration::from_secs(2);
                    let current = DefaultAudioOutput::devices().0;
                    if current != self.audio_device {
                        self.worker = None;
                        state.connect.native_ready = false;
                        self.restore_pending = true;
                        self.next_retry = Instant::now();
                        state.playback.state = PlaybackState::Paused;
                        state.capture_resume();
                        state.config.save();
                        if state.connect.selected.is_none() {
                            state.info_alert("Salida de audio cambiada. Reproducción en pausa.");
                        }
                        self.audio_device = current;
                    }
                }
                if state.config.native_connect
                    && state.config.has_credentials()
                    && self.worker.is_none()
                    && Instant::now() >= self.next_retry
                {
                    state.connect.native_status = "Conectando receptor nativo con Spotify…".into();
                    state.connect.native_ready = false;
                    // Stop the legacy sink before creating the Connect sink.
                    ctx.submit_command(cmd::SUSPEND_LOCAL_PLAYBACK);
                    state.config.save();
                    self.worker = Some(Worker::start(
                        state.config.clone(),
                        ctx.get_external_handle(),
                    ));
                }
                self.timer = ctx.request_timer(Duration::from_secs(1));
            }
        }
        if let Event::WindowDisconnected = event {
            self.pending_load = None;
            self.worker = None;
        }
        if let Event::Command(command) = event {
            // The legacy engine shuts down its sink when Connect takes ownership.
            // Its Stop notification must not erase the restored paused selection.
            if state.config.native_connect && command.is(cmd::PLAYBACK_STOPPED) {
                ctx.set_handled();
                return;
            }
            if let Some((epoch, notice)) = command.get(NOTICE) {
                if self.worker.as_ref().is_some_and(|w| w.epoch == *epoch) {
                    self.notice(ctx, state, notice);
                }
                ctx.set_handled();
                return;
            }
            if command.is(TOGGLE) {
                self.pending_load = None;
                self.in_flight_load = None;
                state.config.native_connect = !state.config.native_connect;
                state.config.save();
                self.worker = None;
                state.connect.native_ready = false;
                self.next_retry = Instant::now();
                self.restore_pending = true;
                if !state.config.native_connect {
                    ctx.submit_command(cmd::RESTORE_LOCAL_PLAYBACK);
                    state.connect.native_status = "Connect nativo desactivado. El motor local no es controlable desde el teléfono.".into();
                }
                ctx.set_handled();
                return;
            }
            if command.is(RETRY) {
                self.worker = None;
                state.connect.native_ready = false;
                self.next_retry = Instant::now();
                self.restore_pending = true;
                ctx.set_handled();
                return;
            }
            if state.config.native_connect && command.is(cmd::SUSPEND_LOCAL_PLAYBACK) {
                if state.connect.selected.is_some() {
                    self.pending_load = None;
                }
                if state.connect.native_ready && self.worker.is_some() {
                    self.send(state, Action::Suspend);
                }
                child.event(ctx, event, state, env);
                return;
            }
            if state.config.native_connect && state.connect.selected.is_none() {
                if (command.is(cmd::PLAY_RESUME) || command.is(cmd::PLAY_TOGGLE))
                    && self.pending_load.is_some()
                {
                    if let Some(pending) = &mut self.pending_load {
                        pending.playing = command.is(cmd::PLAY_RESUME) || !pending.playing;
                    }
                    self.flush_pending(state);
                    ctx.set_handled();
                    return;
                }
                if (command.is(cmd::PLAY_PAUSE) || command.is(cmd::PLAY_STOP))
                    && self.pending_load.is_some()
                {
                    if command.is(cmd::PLAY_STOP) {
                        self.pending_load = None;
                    } else if let Some(pending) = &mut self.pending_load {
                        pending.playing = false;
                    }
                    state.playback.state = PlaybackState::Paused;
                    ctx.set_handled();
                    return;
                }
                if command.is(cmd::RESTORE_LOCAL_PLAYBACK) {
                    self.restore_pending = true;
                    state.playback.state = PlaybackState::Paused;
                    ctx.set_handled();
                    return;
                }
                if let Some(payload) = command.get(cmd::PLAY_TRACKS) {
                    self.load(state, payload, Duration::ZERO, true);
                    ctx.set_handled();
                    return;
                }
                if let Some((index, expected)) = command.get(cmd::PLAY_UPCOMING) {
                    if state
                        .playback
                        .up_next
                        .get(*index)
                        .is_some_and(|q| q.item.id() == *expected)
                    {
                        if !self.restore_pending {
                            let occurrence = self
                                .cluster
                                .as_ref()
                                .and_then(|update| update.cluster.as_ref())
                                .and_then(|cluster| cluster.player_state.as_ref())
                                .and_then(|player| {
                                    player
                                        .next_tracks
                                        .iter()
                                        .filter(|provided| {
                                            psst_core::item_id::ItemId::from_uri(&provided.uri)
                                                .is_some_and(|id| {
                                                    state.queued_entry(id).is_some()
                                                        || from_provided(provided).is_some()
                                                })
                                        })
                                        .nth(*index)
                                })
                                .filter(|provided| {
                                    expected.to_uri().as_deref() == Some(provided.uri.as_str())
                                })
                                .map(|provided| (provided.uid.clone(), provided.uri.clone()));
                            if let Some((uid, uri)) = occurrence {
                                self.send(state, Action::PlayQueued { uid, uri });
                            } else {
                                state
                                    .error_alert("La cola cambió. Selecciona de nuevo la canción.");
                            }
                            ctx.set_handled();
                            return;
                        }
                        let items = state
                            .playback
                            .up_next
                            .iter()
                            .skip(*index)
                            .map(|q| q.item.clone())
                            .collect();
                        self.load(
                            state,
                            &PlaybackPayload {
                                items,
                                position: 0,
                                origin: PlaybackOrigin::Home,
                            },
                            Duration::ZERO,
                            true,
                        );
                    } else {
                        state.error_alert("La cola cambió. Selecciona de nuevo la canción.");
                    }
                    ctx.set_handled();
                    return;
                }
                if command.is(cmd::PLAY_RESUME) || command.is(cmd::PLAY_TOGGLE) {
                    if self.restore_pending {
                        if let Some((payload, progress)) = Self::payload(state) {
                            self.load(state, &payload, progress, true);
                        }
                    } else {
                        self.send(
                            state,
                            if command.is(cmd::PLAY_TOGGLE) {
                                Action::Toggle
                            } else {
                                Action::Play
                            },
                        );
                    }
                    ctx.set_handled();
                    return;
                }
                let action = if command.is(cmd::PLAY_PAUSE) || command.is(cmd::PLAY_STOP) {
                    Some(Action::Pause)
                } else if command.is(cmd::PLAY_NEXT) {
                    Some(Action::Next)
                } else if command.is(cmd::PLAY_PREVIOUS) {
                    Some(Action::Previous)
                } else if let Some(position) = command.get(cmd::SKIP_TO_POSITION) {
                    Some(Action::Seek((*position).min(u32::MAX as u64) as u32))
                } else if let Some(fraction) = command.get(cmd::PLAY_SEEK) {
                    state.playback.now_playing.as_ref().map(|np| {
                        Action::Seek(
                            (np.item.duration().as_millis() as f64 * fraction.clamp(0.0, 1.0))
                                .min(u32::MAX as f64) as u32,
                        )
                    })
                } else if let Some(behavior) = command.get(cmd::PLAY_QUEUE_BEHAVIOR) {
                    state.set_queue_behavior(*behavior);
                    if self.restore_pending || !state.connect.native_ready {
                        ctx.set_handled();
                        return;
                    }
                    Some(Action::Behavior(*behavior))
                } else {
                    None
                };
                if let Some(action) = action {
                    self.send(state, action);
                    ctx.set_handled();
                    return;
                }
                if let Some((entry, _)) = command.get(cmd::ADD_TO_QUEUE) {
                    if !self.restore_pending {
                        let manual_count = self
                            .cluster
                            .as_ref()
                            .and_then(|update| update.cluster.as_ref())
                            .and_then(|cluster| cluster.player_state.as_ref())
                            .map_or(0, |player| {
                                player
                                    .next_tracks
                                    .iter()
                                    .filter(|track| track.provider == "queue")
                                    .count()
                            });
                        if manual_count >= 80 {
                            state.error_alert("La cola manual del motor Connect está llena (80 canciones). Reproduce alguna antes de añadir más.");
                            ctx.set_handled();
                            return;
                        }
                        if let Some(uri) = entry.item.id().to_uri() {
                            state.add_queued_entry(entry.clone());
                            self.send(state, Action::Add(uri));
                        }
                        ctx.set_handled();
                        return;
                    }
                    if let Some(current) = state.playback.now_playing.as_ref() {
                        let mut items = druid::im::vector![current.item.clone()];
                        items.extend(state.playback.up_next.iter().map(|q| q.item.clone()));
                        items.push_back(entry.item.clone());
                        let progress = current.progress;
                        let playing = state.playback.state == PlaybackState::Playing;
                        self.load(
                            state,
                            &PlaybackPayload {
                                items,
                                position: 0,
                                origin: current.origin.clone(),
                            },
                            progress,
                            playing,
                        );
                    }
                    ctx.set_handled();
                    return;
                }
            }
        }
        child.event(ctx, event, state, env);
    }
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        state: &AppState,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.timer = ctx.request_timer(Duration::from_millis(200));
        }
        child.lifecycle(ctx, event, state, env);
    }
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old: &AppState,
        state: &AppState,
        env: &Env,
    ) {
        if state.config.native_connect
            && state.connect.selected.is_none()
            && state.connect.native_ready
            && !old.playback.volume.same(&state.playback.volume)
        {
            if let Some(worker) = &self.worker {
                let value = volume(state.playback.volume);
                if self.last_volume != Some(value) {
                    self.last_volume = Some(value);
                    let _ = worker.sender.try_send(Action::Volume(value));
                }
            }
        }
        if old.config.audio_quality != state.config.audio_quality {
            if let Some(worker) = &self.worker {
                worker.quality.send_replace(state.config.playback().bitrate);
            }
        }
        if old.config.has_credentials() != state.config.has_credentials()
            || old.config.username() != state.config.username()
        {
            self.worker = None;
            self.pending_load = None;
            self.next_retry = Instant::now();
            self.restore_pending = true;
            ctx.submit_command(RETRY);
        }
        child.update(ctx, old, state, env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_payload(name: &str) -> PlaybackPayload {
        let track: Track = serde_json::from_value(serde_json::json!({
            "id":"0123456789012345678901", "name":name, "artists":[],
            "duration_ms":240000, "disc_number":1, "track_number":1,
            "explicit":false, "is_local":false
        }))
        .unwrap();
        PlaybackPayload {
            items: druid::im::vector![Playable::Track(Arc::new(track))],
            position: 0,
            origin: PlaybackOrigin::Home,
        }
    }

    #[test]
    fn reconnect_restores_playing_track_and_position_after_repeated_failures() {
        let mut state = AppState::default_with_config(Config::default());
        let payload = pending_payload("Interrupted");
        state.start_playback(
            payload.items[0].clone(),
            payload.origin.clone(),
            Duration::from_secs(73),
        );
        let mut controller = NativeConnectController::default();
        controller.connection_failed(&mut state);
        controller.connection_failed(&mut state);
        assert_eq!(state.playback.state, PlaybackState::Paused);
        assert!(controller.next_retry <= Instant::now() + Duration::from_secs(15));
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        controller.worker = Some(Worker {
            sender,
            epoch: 1,
            quality: tokio::sync::watch::channel(320).0,
        });
        state.connect.native_ready = true;
        controller.flush_pending(&mut state);
        match receiver.try_recv().unwrap() {
            Action::Load(request) => {
                assert!(request.start_playing);
                assert_eq!(request.seek_to, 73_000);
            }
            _ => panic!("expected recovery load"),
        }
        controller.flush_pending(&mut state);
        assert!(receiver.try_recv().is_err());
        // Registration can succeed while the subsequent audio download still fails.
        controller.connection_failed(&mut state);
        let pending = controller.pending_load.as_ref().unwrap();
        assert!(pending.playing);
        assert_eq!(pending.progress, Duration::from_secs(73));
    }

    #[test]
    fn reconnect_preserves_pause_and_newer_user_selection() {
        let mut state = AppState::default_with_config(Config::default());
        let payload = pending_payload("Interrupted");
        state.start_playback(
            payload.items[0].clone(),
            payload.origin.clone(),
            Duration::from_secs(73),
        );
        state.pause_playback();
        let mut controller = NativeConnectController::default();
        controller.connection_failed(&mut state);
        assert!(!controller.pending_load.as_ref().unwrap().playing);
        controller.load(
            &mut state,
            &pending_payload("New selection"),
            Duration::from_secs(12),
            false,
        );
        controller.connection_failed(&mut state);
        let pending = controller.pending_load.as_ref().unwrap();
        assert_eq!(pending.payload.items[0].name().as_ref(), "New selection");
        assert_eq!(pending.progress, Duration::from_secs(12));
        assert!(!pending.playing);
    }

    #[test]
    fn pause_on_closed_connection_does_not_autoresume() {
        let mut state = AppState::default_with_config(Config::default());
        let payload = pending_payload("Interrupted");
        state.start_playback(
            payload.items[0].clone(),
            payload.origin.clone(),
            Duration::from_secs(73),
        );
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        drop(receiver);
        let mut controller = NativeConnectController {
            worker: Some(Worker {
                sender,
                epoch: 1,
                quality: tokio::sync::watch::channel(320).0,
            }),
            ..Default::default()
        };
        controller.send(&mut state, Action::Pause);
        assert!(!controller.pending_load.as_ref().unwrap().playing);
    }

    #[test]
    fn pause_waiting_for_network_acknowledgement_does_not_autoresume() {
        let mut state = AppState::default_with_config(Config::default());
        let payload = pending_payload("Interrupted");
        state.start_playback(
            payload.items[0].clone(),
            payload.origin,
            Duration::from_secs(73),
        );
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let mut controller = NativeConnectController {
            worker: Some(Worker {
                sender,
                epoch: 1,
                quality: tokio::sync::watch::channel(320).0,
            }),
            ..Default::default()
        };
        controller.send(&mut state, Action::Pause);
        assert!(matches!(receiver.try_recv(), Ok(Action::Pause)));
        controller.connection_failed(&mut state);
        assert!(!controller.pending_load.as_ref().unwrap().playing);
    }

    #[test]
    fn connection_failure_does_not_interrupt_remote_playback() {
        let mut state = AppState::default_with_config(Config::default());
        state.connect.selected = Some(crate::data::connect::Device {
            id: Some("phone".into()),
            name: "Phone".into(),
            kind: "Smartphone".into(),
            is_active: true,
            is_restricted: false,
            volume_percent: Some(50),
        });
        let payload = pending_payload("Remote");
        state.start_playback(
            payload.items[0].clone(),
            payload.origin,
            Duration::from_secs(73),
        );
        let mut controller = NativeConnectController::default();
        controller.connection_failed(&mut state);
        assert!(controller.pending_load.is_none());
        assert_eq!(state.playback.state, PlaybackState::Playing);
    }

    #[test]
    fn startup_retains_only_latest_selection_and_delivers_it_once_with_pause_and_seek() {
        let mut state = AppState::default_with_config(Config::default());
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let mut controller = NativeConnectController {
            worker: Some(Worker {
                sender,
                epoch: 1,
                quality: tokio::sync::watch::channel(320).0,
            }),
            ..Default::default()
        };
        controller.load(&mut state, &pending_payload("First"), Duration::ZERO, true);
        controller.load(
            &mut state,
            &pending_payload("Latest"),
            Duration::from_secs(12),
            false,
        );
        assert!(receiver.try_recv().is_err()); // No command before receiver registration.
        assert_eq!(
            controller.pending_load.as_ref().unwrap().payload.items[0]
                .name()
                .as_ref(),
            "Latest"
        );
        state.connect.native_ready = true;
        controller.flush_pending(&mut state);
        match receiver.try_recv().unwrap() {
            Action::Load(request) => {
                assert!(!request.start_playing);
                assert_eq!(request.seek_to, 12_000);
            }
            _ => panic!("expected a load"),
        }
        assert_eq!(state.playback.queue[0].item.name().as_ref(), "Latest");
        assert!(controller.pending_load.is_none());
        controller.flush_pending(&mut state);
        assert!(receiver.try_recv().is_err());
        assert!(!controller.restore_pending);
    }

    #[test]
    fn full_channel_retries_without_duplicate_alerts_and_closed_channel_reconnects() {
        let mut state = AppState::default_with_config(Config::default());
        state.connect.native_ready = true;
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        assert!(sender.try_send(Action::Pause).is_ok());
        let mut controller = NativeConnectController {
            worker: Some(Worker {
                sender,
                epoch: 1,
                quality: tokio::sync::watch::channel(320).0,
            }),
            ..Default::default()
        };
        controller.load(
            &mut state,
            &pending_payload("Selected"),
            Duration::ZERO,
            true,
        );
        let alerts = state.alerts.len();
        controller.flush_pending(&mut state);
        assert_eq!(state.alerts.len(), alerts);
        assert!(matches!(receiver.try_recv(), Ok(Action::Pause)));
        controller.flush_pending(&mut state);
        assert!(matches!(receiver.try_recv(), Ok(Action::Load(_))));
        drop(receiver);
        controller.load(
            &mut state,
            &pending_payload("After disconnect"),
            Duration::ZERO,
            true,
        );
        assert!(controller.worker.is_none());
        assert!(!state.connect.native_ready);
        assert!(controller.pending_load.is_some());
        assert!(controller.next_retry <= Instant::now());
    }

    #[test]
    fn switching_to_remote_cancels_deferred_local_playback() {
        let mut state = AppState::default_with_config(Config::default());
        let mut controller = NativeConnectController::default();
        controller.load(
            &mut state,
            &pending_payload("Selected"),
            Duration::ZERO,
            true,
        );
        state.connect.selected = Some(crate::data::connect::Device {
            id: Some("phone".into()),
            name: "Phone".into(),
            kind: "Smartphone".into(),
            is_active: true,
            is_restricted: false,
            volume_percent: Some(50),
        });
        controller.flush_pending(&mut state);
        assert!(controller.pending_load.is_none());
    }
    #[test]
    fn full_audio_buffer_waits_for_output_instead_of_failing() {
        let buffer = SpscRb::new(4);
        let producer = buffer.producer();
        let consumer = buffer.consumer();
        producer.write(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        let writer = thread::spawn(move || {
            send.send(write_buffer(
                &producer,
                &[5.0, 6.0, 7.0, 8.0],
                &std::sync::atomic::AtomicBool::new(false),
            ))
            .unwrap()
        });
        assert!(matches!(
            receive.recv_timeout(Duration::from_millis(20)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        let mut samples = [0.0; 4];
        assert_eq!(consumer.read(&mut samples).unwrap(), 4);
        assert_eq!(samples, [1.0, 2.0, 3.0, 4.0]);
        receive
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        writer.join().unwrap();
        assert_eq!(consumer.read(&mut samples).unwrap(), 4);
        assert_eq!(samples, [5.0, 6.0, 7.0, 8.0]);
    }
    #[test]
    fn closing_receiver_interrupts_a_full_audio_buffer() {
        let buffer = SpscRb::new(4);
        let producer = buffer.producer();
        producer.write(&[1.0; 4]).unwrap();
        let stopping = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = stopping.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let writer = thread::spawn(move || {
            send.send(write_buffer(&producer, &[2.0; 4], &signal))
                .unwrap()
        });
        assert!(matches!(
            receive.recv_timeout(Duration::from_millis(20)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        stopping.store(true, std::sync::atomic::Ordering::Release);
        assert!(matches!(
            receive.recv_timeout(Duration::from_secs(1)).unwrap(),
            Err(SinkError::NotConnected(_))
        ));
        writer.join().unwrap();
    }
    #[test]
    fn device_identity_and_native_choice_survive_configuration_reload() {
        let config = Config::default();
        let device = config.connect_device_id.clone();
        let saved = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.connect_device_id, device);
        assert!(restored.native_connect);
        assert_eq!(device.len(), 40);
    }
    #[test]
    fn ids_are_strict_and_typed() {
        assert_eq!(
            id("spotify:track:0123456789012345678901", "track").as_deref(),
            Some("0123456789012345678901")
        );
        assert!(id("spotify:episode:0123456789012345678901", "track").is_none());
        assert!(id("spotify:track:short", "track").is_none());
    }
    #[test]
    fn volume_is_bounded() {
        assert_eq!(volume(-1.0), 0);
        assert_eq!(volume(5.0), u16::MAX);
        assert_eq!(volume(0.5), 32768);
    }
    #[test]
    fn unknown_queue_tracks_keep_spotify_identity() {
        let track = ProvidedTrack {
            uri: "spotify:track:0123456789012345678901".into(),
            ..ProvidedTrack::default()
        };
        let playable = from_provided(&track).unwrap();
        assert_eq!(playable.id().to_uri().as_deref(), Some(track.uri.as_str()));
        assert!(from_provided(&ProvidedTrack::default()).is_none());
    }
}
