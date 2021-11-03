pub mod file;
pub mod item;
pub mod queue;
mod storage;
mod worker;

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

use crate::{
    actor::Actor,
    audio::output::{AudioOutput, AudioSink},
    cache::CacheHandle,
    cdn::CdnHandle,
    error::Error,
    session::SessionService,
};

use self::{
    file::AudioPath,
    item::{LoadedPlaybackItem, PlaybackItem},
    queue::{Queue, QueueBehavior},
    worker::{Decode, Decoding, DecodingWorker},
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

pub struct Player {
    state: PlayerState,
    preload: PreloadState,
    session: SessionService,
    cdn: CdnHandle,
    cache: CacheHandle,
    config: PlaybackConfig,
    queue: Queue,
    sender: Sender<PlayerEvent>,
    receiver: Receiver<PlayerEvent>,
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
        let (sender, receiver) = unbounded();
        Self {
            session,
            cdn,
            cache,
            config,
            sender,
            receiver,
            audio_output_sink: audio_output.sink(),
            audio_volume: VolumeLevel::new(),
            state: PlayerState::Stopped,
            preload: PreloadState::None,
            queue: Queue::new(),
            consecutive_loading_failures: 0,
        }
    }

    pub fn sender(&self) -> Sender<PlayerEvent> {
        self.sender.clone()
    }

    pub fn receiver(&self) -> Receiver<PlayerEvent> {
        self.receiver.clone()
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
                log::warn!("received ununwraped position report");
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
            let event_sender = self.sender.clone();
            let session = self.session.clone();
            let cdn = self.cdn.clone();
            let cache = self.cache.clone();
            let config = self.config.clone();
            move || {
                let result = item.load(&session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Loaded { item, result })
                    .unwrap();
            }
        });
        self.sender.send(PlayerEvent::Loading { item }).unwrap();
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
            let event_sender = self.sender.clone();
            let session = self.session.clone();
            let cdn = self.cdn.clone();
            let cache = self.cache.clone();
            let config = self.config.clone();
            move || {
                let result = item.load(&session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Preloaded { item, result })
                    .unwrap();
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
        let worker = Decoding::spawn_default({
            let events = self.sender.clone();
            let sink = self.audio_output_sink.clone();
            let volume = self.audio_volume.clone();
            move |this| Decoding::new(loaded_item, events, this, sink, volume)
        });
        worker.send(Decode::Start).unwrap();
        self.state = PlayerState::Playing {
            path,
            position,
            worker: DecodingWorker { actor: worker },
        };
        self.sender
            .send(PlayerEvent::Playing { path, position })
            .unwrap();
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
                worker.actor.send(Decode::Stop).unwrap();
                self.sender
                    .send(PlayerEvent::Pausing { path, position })
                    .unwrap();
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
                worker.actor.send(Decode::Start).unwrap();
                self.sender
                    .send(PlayerEvent::Resuming { path, position })
                    .unwrap();
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
        self.sender.send(PlayerEvent::Stopped).unwrap();
        self.state = PlayerState::Stopped;
        self.queue.clear();
        self.consecutive_loading_failures = 0;
    }

    fn seek(&mut self, position: Duration) {
        if let PlayerState::Playing { worker, .. } | PlayerState::Paused { worker, .. } =
            &mut self.state
        {
            worker.actor.send(Decode::Seek(position)).unwrap();
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
        worker: DecodingWorker,
    },
    Paused {
        path: AudioPath,
        position: Duration,
        worker: DecodingWorker,
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

#[derive(Clone)]
pub struct VolumeLevel {
    volume: Arc<AtomicU32>,
}

impl VolumeLevel {
    pub fn new() -> Self {
        Self {
            volume: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn set(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Ordering::Relaxed)
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
}
