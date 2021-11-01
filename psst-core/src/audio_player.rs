use std::{
    mem,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    thread,
    thread::JoinHandle,
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use symphonia::core::audio::{SampleBuffer, SignalSpec};

use crate::{
    actor::{Actor, ActorHandle, ActorOp},
    audio_decode::AudioDecoder,
    audio_file::{AudioFile, AudioPath},
    audio_key::AudioKey,
    audio_normalize::NormalizationLevel,
    audio_output::{AudioOutput, AudioSink},
    audio_queue::{Queue, QueueBehavior},
    audio_resample::{AudioResampler, ResamplingAlgo, ResamplingSpec},
    cache::CacheHandle,
    cdn::CdnHandle,
    error::Error,
    item_id::{ItemId, ItemIdType},
    metadata::{Fetch, ToAudioPath},
    protocol::metadata::Track,
    session::SessionService,
};

const PREVIOUS_TRACK_THRESHOLD: Duration = Duration::from_secs(3);
const STOP_AFTER_CONSECUTIVE_LOADING_FAILURES: usize = 3;

#[derive(Clone)]
pub struct PlaybackConfig {
    pub bitrate: usize,
    pub pregain: f32,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            bitrate: 320,
            pregain: 3.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PlaybackItem {
    pub item_id: ItemId,
    pub norm_level: NormalizationLevel,
}

impl PlaybackItem {
    fn load(
        &self,
        session: &SessionService,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: &PlaybackConfig,
    ) -> Result<LoadedPlaybackItem, Error> {
        let path = load_audio_path(self.item_id, session, &cache, config)?;
        let key = load_audio_key(&path, session, &cache)?;
        let file = AudioFile::open(path, cdn, cache)?;
        let (source, norm_data) = file.audio_source(key)?;
        let norm_factor = norm_data.factor_for_level(self.norm_level, config.pregain);
        Ok(LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        })
    }
}

fn load_audio_path(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<AudioPath, Error> {
    match item_id.id_type {
        ItemIdType::Track => {
            load_audio_path_from_track_or_alternative(item_id, session, cache, config)
        }
        ItemIdType::Podcast | ItemIdType::Unknown => unimplemented!(),
    }
}

fn load_audio_path_from_track_or_alternative(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<AudioPath, Error> {
    let track = load_track(item_id, session, cache)?;
    let country = get_country_code(session, cache);
    let path = match country {
        Some(user_country) if track.is_restricted_in_region(&user_country) => {
            // The track is regionally restricted and is unavailable.  Let's try to find an
            // alternative track.
            let alt_id = track
                .find_allowed_alternative(&user_country)
                .ok_or(Error::AudioFileNotFound)?;
            let alt_track = load_track(alt_id, session, cache)?;
            let alt_path = alt_track
                .to_audio_path(config.bitrate)
                .ok_or(Error::AudioFileNotFound)?;
            // We've found an alternative track with a fitting audio file.  Let's cheat a
            // little and pretend we've obtained it from the requested track.
            // TODO: We should be honest and display the real track information.
            AudioPath {
                item_id,
                ..alt_path
            }
        }
        _ => {
            // Either we do not have a country code loaded or the track is available, return
            // it.
            track
                .to_audio_path(config.bitrate)
                .ok_or(Error::AudioFileNotFound)?
        }
    };
    Ok(path)
}

fn get_country_code(session: &SessionService, cache: &CacheHandle) -> Option<String> {
    if let Some(cached_country_code) = cache.get_country_code() {
        Some(cached_country_code)
    } else {
        let country_code = session.connected().ok()?.get_country_code()?;
        if let Err(err) = cache.save_country_code(&country_code) {
            log::warn!("failed to save country code to cache: {:?}", err);
        }
        Some(country_code)
    }
}

fn load_track(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<Track, Error> {
    if let Some(cached_track) = cache.get_track(item_id) {
        Ok(cached_track)
    } else {
        let track = Track::fetch(session, item_id)?;
        if let Err(err) = cache.save_track(item_id, &track) {
            log::warn!("failed to save track to cache: {:?}", err);
        }
        Ok(track)
    }
}

fn load_audio_key(
    path: &AudioPath,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<AudioKey, Error> {
    if let Some(cached_key) = cache.get_audio_key(path.item_id, path.file_id) {
        Ok(cached_key)
    } else {
        let key = session
            .connected()?
            .get_audio_key(path.item_id, path.file_id)?;
        if let Err(err) = cache.save_audio_key(path.item_id, path.file_id, &key) {
            log::warn!("failed to save audio key to cache: {:?}", err);
        }
        Ok(key)
    }
}

pub struct LoadedPlaybackItem {
    file: AudioFile,
    source: AudioDecoder,
    norm_factor: f32,
}

pub struct Player {
    state: PlayerState,
    preload: PreloadState,
    session: SessionService,
    cdn: CdnHandle,
    cache: CacheHandle,
    config: PlaybackConfig,
    queue: Queue,
    event_sender: Sender<PlayerEvent>,
    event_receiver: Receiver<PlayerEvent>,
    audio_output_sink: AudioSink<f32>,
    audio_volume: VolumeLevel,
    consecutive_loading_failures: usize,
}

impl Player {
    pub fn new(
        session: SessionService,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: PlaybackConfig,
        audio_output: &AudioOutput,
    ) -> Self {
        let (event_sender, event_receiver) = unbounded();
        Self {
            session,
            cdn,
            cache,
            config,
            event_sender,
            event_receiver,
            audio_output_sink: audio_output.sink(),
            audio_volume: VolumeLevel::new(),
            state: PlayerState::Stopped,
            preload: PreloadState::None,
            queue: Queue::new(),
            consecutive_loading_failures: 0,
        }
    }

    pub fn event_sender(&self) -> Sender<PlayerEvent> {
        self.event_sender.clone()
    }

    pub fn event_receiver(&self) -> Receiver<PlayerEvent> {
        self.event_receiver.clone()
    }

    pub fn handle(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::Command(cmd) => {
                self.handle_command(cmd);
            }
            PlayerEvent::Loaded { item, result } => {
                self.handle_loaded(item, result);
            }
            PlayerEvent::Preloaded { item, result } => {
                self.handle_preloaded(item, result);
            }
            PlayerEvent::Position { position, path } => {
                self.handle_position(position, path);
            }
            PlayerEvent::EndOfTrack { .. } => {
                self.handle_end_of_track();
            }
            PlayerEvent::Loading { .. }
            | PlayerEvent::Playing { .. }
            | PlayerEvent::Pausing { .. }
            | PlayerEvent::Resuming { .. }
            | PlayerEvent::Stopped { .. }
            | PlayerEvent::Blocked { .. } => {}
        };
    }

    fn handle_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::LoadQueue { items, position } => self.load_queue(items, position),
            PlayerCommand::LoadAndPlay { item } => self.load_and_play(item),
            PlayerCommand::Preload { item } => self.preload(item),
            PlayerCommand::Pause => self.pause(),
            PlayerCommand::Resume => self.resume(),
            PlayerCommand::PauseOrResume => self.pause_or_resume(),
            PlayerCommand::Previous => self.previous(),
            PlayerCommand::Next => self.next(),
            PlayerCommand::Stop => self.stop(),
            PlayerCommand::Seek { position } => self.seek(position),
            PlayerCommand::Configure { config } => self.configure(config),
            PlayerCommand::SetQueueBehavior { behavior } => self.queue.set_behaviour(behavior),
            PlayerCommand::SetVolume { volume } => self.set_volume(volume),
        }
    }

    fn handle_loaded(&mut self, item: PlaybackItem, result: Result<LoadedPlaybackItem, Error>) {
        match self.state {
            PlayerState::Loading {
                item: requested_item,
                ..
            } if item == requested_item => match result {
                Ok(loaded_item) => {
                    self.consecutive_loading_failures = 0;
                    self.play_loaded(loaded_item);
                }
                Err(err) => {
                    self.consecutive_loading_failures += 1;
                    if self.consecutive_loading_failures < STOP_AFTER_CONSECUTIVE_LOADING_FAILURES {
                        log::error!("skipping, error while loading: {}", err);
                        self.next();
                    } else {
                        log::error!("stopping, error while loading: {}", err);
                        self.stop();
                    }
                }
            },
            _ => {
                log::info!("stale load result received, ignoring");
            }
        }
    }

    fn handle_preloaded(&mut self, item: PlaybackItem, result: Result<LoadedPlaybackItem, Error>) {
        match self.preload {
            PreloadState::Preloading {
                item: requested_item,
                ..
            } if item == requested_item => match result {
                Ok(loaded_item) => {
                    log::info!("preloaded audio file");
                    self.preload = PreloadState::Preloaded { item, loaded_item };
                }
                Err(err) => {
                    log::error!("failed to preload audio file, error while opening: {}", err);
                    self.preload = PreloadState::None;
                }
            },
            _ => {
                log::info!("stale preload result received, ignoring");
            }
        }
    }

    fn handle_position(&mut self, new_position: Duration, path: AudioPath) {
        match &mut self.state {
            PlayerState::Playing { position, .. } | PlayerState::Paused { position, .. } => {
                *position = new_position;
            }
            _ => {
                log::warn!("received unexpected position report");
            }
        }
        const PRELOAD_BEFORE_END_OF_TRACK: Duration = Duration::from_secs(30);
        if let Some(&item_to_preload) = self.queue.get_following() {
            let time_until_end_of_track =
                path.duration.checked_sub(new_position).unwrap_or_default();
            if time_until_end_of_track <= PRELOAD_BEFORE_END_OF_TRACK {
                self.preload(item_to_preload);
            }
        }
    }

    fn handle_end_of_track(&mut self) {
        self.queue.skip_to_following();
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn load_queue(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.queue.fill(items, position);
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn load_and_play(&mut self, item: PlaybackItem) {
        // Check if the item is already preloaded, and if so, take it out of the
        // preloader state, and start the playback.
        match mem::replace(&mut self.preload, PreloadState::None) {
            PreloadState::Preloaded {
                item: preloaded_item,
                loaded_item,
            } if preloaded_item == item => {
                self.play_loaded(loaded_item);
                return;
            }
            preloading_or_none => {
                // Restore the preloader to the previous state.
                // TODO: If the item is being preloaded, extract the loading handle.
                self.preload = preloading_or_none;
            }
        }
        // Item is not preloaded yet, load it in a background thread.
        let loading_handle = thread::spawn({
            let event_sender = self.event_sender.clone();
            let session = self.session.clone();
            let cdn = self.cdn.clone();
            let cache = self.cache.clone();
            let config = self.config.clone();
            move || {
                let result = item.load(&session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Loaded { item, result })
                    .expect("Failed to send PlayerEvent::Loaded");
            }
        });
        self.event_sender
            .send(PlayerEvent::Loading { item })
            .expect("Failed to send PlayerEvent::Loading");
        self.state = PlayerState::Loading {
            item,
            _loading_handle: loading_handle,
        };
    }

    fn preload(&mut self, item: PlaybackItem) {
        if self.is_in_preload(item) {
            return;
        }
        let loading_handle = thread::spawn({
            let event_sender = self.event_sender.clone();
            let session = self.session.clone();
            let cdn = self.cdn.clone();
            let cache = self.cache.clone();
            let config = self.config.clone();
            move || {
                let result = item.load(&session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Preloaded { item, result })
                    .expect("Failed to send PlayerEvent::Preloaded");
            }
        });
        self.preload = PreloadState::Preloading {
            item,
            _loading_handle: loading_handle,
        };
    }

    fn set_volume(&mut self, volume: f64) {
        self.audio_volume.set(volume as _);
    }

    fn play_loaded(&mut self, loaded_item: LoadedPlaybackItem) {
        log::info!("starting playback");
        let path = loaded_item.file.path();
        let position = Duration::default();
        let worker = PlaybackWorker {
            actor: Decoding::spawn_default({
                let events = self.event_sender.clone();
                let sink = self.audio_output_sink.clone();
                let volume = self.audio_volume.clone();
                move |this| Decoding::new(loaded_item, events, this, sink, volume)
            }),
        };
        worker.start();
        self.state = PlayerState::Playing {
            path,
            position,
            worker,
        };
        self.event_sender
            .send(PlayerEvent::Playing { path, position })
            .expect("Failed to send PlayerEvent::Playing");
    }

    fn pause(&mut self) {
        match mem::replace(&mut self.state, PlayerState::Invalid) {
            PlayerState::Playing {
                path,
                position,
                worker,
            }
            | PlayerState::Paused {
                path,
                position,
                worker,
            } => {
                log::info!("pausing playback");
                worker.stop();
                self.event_sender
                    .send(PlayerEvent::Pausing { path, position })
                    .expect("Failed to send PlayerEvent::Paused");
                self.state = PlayerState::Paused {
                    path,
                    position,
                    worker,
                };
            }
            _ => {
                log::warn!("invalid state transition");
            }
        }
    }

    fn resume(&mut self) {
        match mem::replace(&mut self.state, PlayerState::Invalid) {
            PlayerState::Playing {
                path,
                position,
                worker,
            }
            | PlayerState::Paused {
                path,
                position,
                worker,
            } => {
                log::info!("resuming playback");
                worker.start();
                self.event_sender
                    .send(PlayerEvent::Resuming { path, position })
                    .expect("Failed to send PlayerEvent::Resuming");
                self.state = PlayerState::Playing {
                    path,
                    position,
                    worker,
                };
            }
            _ => {
                log::warn!("invalid state transition");
            }
        }
    }

    fn pause_or_resume(&mut self) {
        match &self.state {
            PlayerState::Playing { .. } => self.pause(),
            PlayerState::Paused { .. } => self.resume(),
            _ => {
                // Do nothing.
            }
        }
    }

    fn previous(&mut self) {
        if self.is_near_playback_start() {
            self.queue.skip_to_previous();
            if let Some(&item) = self.queue.get_current() {
                self.load_and_play(item);
            } else {
                self.stop();
            }
        } else {
            self.seek(Duration::default());
        }
    }

    fn next(&mut self) {
        self.queue.skip_to_next();
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn stop(&mut self) {
        self.event_sender
            .send(PlayerEvent::Stopped)
            .expect("Failed to send PlayerEvent::Stopped");
        self.state = PlayerState::Stopped;
        self.queue.clear();
        self.consecutive_loading_failures = 0;
    }

    fn seek(&mut self, position: Duration) {
        if let PlayerState::Playing { worker, .. } | PlayerState::Paused { worker, .. } =
            &mut self.state
        {
            worker.seek(position);
        }
    }

    fn configure(&mut self, config: PlaybackConfig) {
        self.config = config;
    }

    fn is_near_playback_start(&self) -> bool {
        match self.state {
            PlayerState::Playing { position, .. } | PlayerState::Paused { position, .. } => {
                position < PREVIOUS_TRACK_THRESHOLD
            }
            _ => false,
        }
    }

    fn is_in_preload(&self, item: PlaybackItem) -> bool {
        match self.preload {
            PreloadState::Preloading { item: p_item, .. }
            | PreloadState::Preloaded { item: p_item, .. } => p_item == item,
            _ => false,
        }
    }
}

pub enum PlayerCommand {
    LoadQueue {
        items: Vec<PlaybackItem>,
        position: usize,
    },
    LoadAndPlay {
        item: PlaybackItem,
    },
    Preload {
        item: PlaybackItem,
    },
    Pause,
    Resume,
    PauseOrResume,
    Previous,
    Next,
    Stop,
    Seek {
        position: Duration,
    },
    Configure {
        config: PlaybackConfig,
    },
    SetQueueBehavior {
        behavior: QueueBehavior,
    },
    /// Change playback volume to a value in 0.0..=1.0 range.
    SetVolume {
        volume: f64,
    },
}

pub enum PlayerEvent {
    Command(PlayerCommand),
    /// Track has started loading.  `Loaded` follows.
    Loading {
        item: PlaybackItem,
    },
    /// Track loading either succeeded or failed.  `Playing` follows in case of
    /// success.
    Loaded {
        item: PlaybackItem,
        result: Result<LoadedPlaybackItem, Error>,
    },
    /// Next item in queue has been either successfully preloaded or failed to
    /// preload.
    Preloaded {
        item: PlaybackItem,
        result: Result<LoadedPlaybackItem, Error>,
    },
    /// Player has started playing new track.  `Position` events will follow.
    Playing {
        path: AudioPath,
        position: Duration,
    },
    /// Player is in a paused state.  `Resuming` might follow.
    Pausing {
        path: AudioPath,
        position: Duration,
    },
    /// Player is resuming playback of a track.  `Position` events will follow.
    Resuming {
        path: AudioPath,
        position: Duration,
    },
    /// Position of the playback head has changed.
    Position {
        path: AudioPath,
        position: Duration,
    },
    /// Player would like to continue playing, but is blocked, waiting for I/O.
    Blocked {
        path: AudioPath,
        position: Duration,
    },
    /// Player has finished playing a track.  `Loading` or `Playing` might
    /// follow if the queue is not empty, `Stopped` will follow if it is.
    EndOfTrack,
    /// The queue is empty.
    Stopped,
}

enum PlayerState {
    Loading {
        item: PlaybackItem,
        _loading_handle: JoinHandle<()>,
    },
    Playing {
        path: AudioPath,
        position: Duration,
        worker: PlaybackWorker,
    },
    Paused {
        path: AudioPath,
        position: Duration,
        worker: PlaybackWorker,
    },
    Stopped,
    Invalid,
}

enum PreloadState {
    Preloading {
        item: PlaybackItem,
        _loading_handle: JoinHandle<()>,
    },
    Preloaded {
        item: PlaybackItem,
        loaded_item: LoadedPlaybackItem,
    },
    None,
}

struct PlaybackWorker {
    actor: ActorHandle<Decode>,
}

impl PlaybackWorker {
    fn start(&self) {
        self.actor.sender().send(Decode::Start).unwrap();
    }

    fn stop(&self) {
        self.actor.sender().send(Decode::Stop).unwrap();
    }

    fn seek(&self, pos: Duration) {
        self.actor.sender().send(Decode::Seek(pos)).unwrap();
    }

    fn quit(&self) {
        let _ = self.actor.sender().send(Decode::Quit);
    }
}

impl Drop for PlaybackWorker {
    fn drop(&mut self) {
        self.quit();
    }
}

const REPORT_POSITION_EACH: Duration = Duration::from_millis(1000);

enum Decode {
    Start,
    Stop,
    Seek(Duration),
    ReadPacket,
    FlushPacket,
    Quit,
}

enum DecState {
    Started,
    Stopped,
}

struct Decoding {
    file: AudioFile,
    source: AudioDecoder,
    norm_factor: f32,
    resampler: AudioResampler,
    samples: SampleBuffer<f32>,
    events: Sender<PlayerEvent>,
    this: Sender<Decode>,
    sink: AudioSink<f32>,
    volume: VolumeLevel,
    state: DecState,
    last_reported_position: Duration,
}

impl Decoding {
    fn new(
        loaded: LoadedPlaybackItem,
        events: Sender<PlayerEvent>,
        this: Sender<Decode>,
        sink: AudioSink<f32>,
        volume: VolumeLevel,
    ) -> Self {
        let LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        } = loaded;
        let resampler = AudioResampler::new(
            // TODO: Make the quality configurable.
            ResamplingAlgo::SincMediumQuality,
            ResamplingSpec {
                channels: source.channels().unwrap().count(),
                from_rate: source.sample_rate().unwrap() as usize,
                to_rate: sink.sample_rate() as usize,
            },
            1024 * 8,
        )
        .unwrap();
        let samples = {
            let max_frames = source.max_frames_per_packet().unwrap_or(1024 * 8);
            let channels = source.channels().unwrap();
            let rate = source.sample_rate().unwrap();
            SampleBuffer::new(max_frames, SignalSpec { rate, channels })
        };
        Self {
            file,
            source,
            norm_factor,
            resampler,
            samples,
            events,
            this,
            sink,
            volume,
            state: DecState::Stopped,
            last_reported_position: Duration::ZERO,
        }
    }

    fn frames_to_duration(&self, frames: u64) -> Duration {
        Duration::from_secs_f64(frames as f64 / self.source.sample_rate().unwrap() as f64)
    }

    fn report_position(&mut self, position: Duration) {
        self.events
            .send(PlayerEvent::Position {
                path: self.file.path(),
                position,
            })
            .unwrap();
        self.last_reported_position = position;
    }

    fn report_current_position(&mut self) {
        let position = self.frames_to_duration(self.source.current_frame());
        self.report_position(position);
    }

    fn report_current_position_if_neeeded(&mut self) {
        let position = self.frames_to_duration(self.source.current_frame());
        if position.saturating_sub(self.last_reported_position) > REPORT_POSITION_EACH {
            self.report_position(position);
        }
    }

    fn is_started(&self) -> bool {
        matches!(self.state, DecState::Started)
    }
}

impl Actor for Decoding {
    type Message = Decode;
    type Error = Error;

    fn handle(&mut self, msg: Self::Message) -> Result<ActorOp, Self::Error> {
        match msg {
            Decode::Start if !self.is_started() => {
                self.this.send(Decode::ReadPacket)?;
                self.state = DecState::Started;
                Ok(ActorOp::Continue)
            }
            Decode::Stop if self.is_started() => {
                self.state = DecState::Stopped;
                Ok(ActorOp::Continue)
            }
            Decode::Seek(pos) => self.handle_seek(pos),
            Decode::ReadPacket => self.handle_read_packet(),
            Decode::FlushPacket => self.handle_flush_packet(),
            Decode::Quit => Ok(ActorOp::Shutdown),
            _ => Ok(ActorOp::Continue),
        }
    }
}

impl Decoding {
    fn handle_seek(&mut self, position: Duration) -> Result<ActorOp, Error> {
        if let Err(err) = self.source.seek(position) {
            log::error!("failed to seek: {}", err);
        } else {
            self.report_current_position();
        }
        Ok(ActorOp::Continue)
    }

    fn handle_read_packet(&mut self) -> Result<ActorOp, Error> {
        if self.is_started() {
            if let Some(packet) = self.source.next_packet() {
                self.samples.copy_interleaved_ref(packet);
                self.report_current_position_if_neeeded();
                self.this.send(Decode::FlushPacket)?;
            } else {
                self.events.send(PlayerEvent::EndOfTrack)?;
                return Ok(ActorOp::Shutdown);
            }
        }
        Ok(ActorOp::Continue)
    }

    fn handle_flush_packet(&mut self) -> Result<ActorOp, Error> {
        let samples = self.samples.samples();

        // Resample the sample buffer into a rate that the audio output supports.
        let resampled = self.resampler.resample(samples)?;

        // Apply the global volume level and the normalization factor.
        let factor = self.norm_factor * self.volume.get();
        for sample in resampled.iter_mut() {
            *sample *= factor;
        }

        // Write into the sink, block until all samples are committed to the ring buffer.
        self.sink.write_blocking(resampled)?;

        if self.is_started() {
            self.this.send(Decode::ReadPacket)?;
        }
        Ok(ActorOp::Continue)
    }
}

#[derive(Clone)]
struct VolumeLevel {
    volume: Arc<AtomicU32>,
}

impl VolumeLevel {
    fn new() -> Self {
        Self {
            volume: Arc::new(AtomicU32::new(0)),
        }
    }

    fn set(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Ordering::Relaxed)
    }

    fn get(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
}
