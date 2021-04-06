use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_channel::Sender;
use druid::{
    im::Vector,
    widget::{prelude::*, Controller},
    ExtEventSink,
};
use psst_core::{
    audio_normalize::NormalizationLevel,
    audio_output::AudioOutput,
    audio_player::{PlaybackConfig, PlaybackItem, Player, PlayerCommand, PlayerEvent},
    cache::Cache,
    cdn::Cdn,
    session::SessionHandle,
};

#[cfg(target_os = "macos")]
use souvlaki::platform::macos::MediaControlsExtMacOs;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback};

use crate::{
    cmd,
    data::{
        Config, Playback, PlaybackOrigin, PlaybackState, QueueBehavior, QueuedTrack, State, TrackId,
    },
};

pub struct PlaybackController {
    sender: Option<Sender<PlayerEvent>>,
    thread: Option<JoinHandle<()>>,
    output_thread: Option<JoinHandle<()>>,
    media_controls: Option<MediaControls>,
}

impl PlaybackController {
    pub fn new() -> Self {
        Self {
            sender: None,
            thread: None,
            output_thread: None,
            media_controls: None,
        }
    }

    fn open_audio_output_and_start_threads(
        &mut self,
        session: SessionHandle,
        config: PlaybackConfig,
        event_sink: ExtEventSink,
        widget_id: WidgetId,
    ) {
        let output = AudioOutput::open().unwrap();
        let remote = output.remote();

        let cache_dir = Config::cache_dir().unwrap();
        let proxy_url = Config::proxy();
        let player = Player::new(
            session.clone(),
            Cdn::new(session, proxy_url.as_deref()).unwrap(),
            Cache::new(cache_dir).unwrap(),
            config,
            remote,
        );
        let sender = player.event_sender();
        let source = player.audio_source();

        let thread = thread::spawn(move || {
            Self::service_events(player, event_sink, widget_id);
        });
        let output_thread = thread::spawn(move || {
            output.start_playback(source).expect("Playback failed");
        });

        let mut media_controls = MediaControls::create().unwrap();
        media_controls.attach({
            let sender = sender.clone();
            move |event| {
                Self::handle_media_control_event(event, &sender);
            }
        });

        self.sender.replace(sender);
        self.thread.replace(thread);
        self.output_thread.replace(output_thread);
        self.media_controls.replace(media_controls);
    }

    fn service_events(mut player: Player, event_sink: ExtEventSink, widget_id: WidgetId) {
        for event in player.event_receiver() {
            // Forward events that affect the UI state to the UI thread.
            match &event {
                PlayerEvent::Loading { item } => {
                    let item: TrackId = item.item_id.into();
                    event_sink
                        .submit_command(cmd::PLAYBACK_LOADING, item, widget_id)
                        .unwrap();
                }
                PlayerEvent::Playing { path, duration } => {
                    let item: TrackId = path.item_id.into();
                    let progress = duration.to_owned();
                    event_sink
                        .submit_command(cmd::PLAYBACK_PLAYING, (item, progress), widget_id)
                        .unwrap();
                }
                PlayerEvent::Pausing { .. } => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_PAUSING, (), widget_id)
                        .unwrap();
                }
                PlayerEvent::Resuming { .. } => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_RESUMING, (), widget_id)
                        .unwrap();
                }
                PlayerEvent::Progress { duration, .. } => {
                    let progress = duration.to_owned();
                    event_sink
                        .submit_command(cmd::PLAYBACK_PROGRESS, progress, widget_id)
                        .unwrap();
                }
                PlayerEvent::Blocked => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_BLOCKED, (), widget_id)
                        .unwrap();
                }
                PlayerEvent::Stopped => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_STOPPED, (), widget_id)
                        .unwrap();
                }
                _ => {}
            }

            // Let the player react to its internal events.
            player.handle(event);
        }
    }

    fn handle_media_control_event(event: MediaControlEvent, sender: &Sender<PlayerEvent>) {
        let cmd = match event {
            MediaControlEvent::Play => PlayerEvent::Command(PlayerCommand::Resume),
            MediaControlEvent::Pause => PlayerEvent::Command(PlayerCommand::Pause),
            MediaControlEvent::Toggle => PlayerEvent::Command(PlayerCommand::PauseOrResume),
            MediaControlEvent::Next => PlayerEvent::Command(PlayerCommand::Next),
            MediaControlEvent::Previous => PlayerEvent::Command(PlayerCommand::Previous),
        };
        sender.send(cmd).unwrap();
    }

    fn update_media_controls(&mut self, playback: &Playback) {
        if let Some(media_controls) = self.media_controls.as_mut() {
            media_controls.set_playback(match playback.state {
                PlaybackState::Loading | PlaybackState::Stopped => MediaPlayback::Stopped,
                PlaybackState::Playing => MediaPlayback::Playing,
                PlaybackState::Paused => MediaPlayback::Paused,
            });
            let title = playback.now_playing.as_ref().map(|c| c.item.name.clone());
            let album = playback.now_playing.as_ref().map(|c| c.item.album_name());
            let artist = playback.now_playing.as_ref().map(|c| c.item.artist_name());
            media_controls.set_metadata(MediaMetadata {
                title: title.as_deref(),
                album: album.as_deref(),
                artist: artist.as_deref(),
            });
        }
    }

    fn send(&mut self, event: PlayerEvent) {
        self.sender.as_mut().unwrap().send(event).unwrap();
    }

    fn play(&mut self, items: &Vector<QueuedTrack>, position: usize) {
        let items = items
            .iter()
            .map(|queued| PlaybackItem {
                item_id: *queued.track.id,
                norm_level: match queued.origin {
                    PlaybackOrigin::Album(_) => NormalizationLevel::Album,
                    _ => NormalizationLevel::Track,
                },
            })
            .collect();
        self.send(PlayerEvent::Command(PlayerCommand::LoadQueue {
            items,
            position,
        }));
    }

    fn pause(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Pause));
    }

    fn resume(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Resume));
    }

    fn previous(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Previous));
    }

    fn next(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Next))
    }

    fn stop(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Stop));
    }

    fn seek(&mut self, position: Duration) {
        self.send(PlayerEvent::Command(PlayerCommand::Seek { position }));
    }

    fn set_queue_behavior(&mut self, behavior: QueueBehavior) {
        self.send(PlayerEvent::Command(PlayerCommand::SetQueueBehavior {
            behavior: match behavior {
                QueueBehavior::Sequential => psst_core::audio_queue::QueueBehavior::Sequential,
                QueueBehavior::Random => psst_core::audio_queue::QueueBehavior::Random,
                QueueBehavior::LoopTrack => psst_core::audio_queue::QueueBehavior::LoopTrack,
                QueueBehavior::LoopAll => psst_core::audio_queue::QueueBehavior::LoopAll,
            },
        }));
    }
}

impl<W> Controller<State, W> for PlaybackController
where
    W: Widget<State>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut State,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_LOADING) => {
                let item = cmd.get_unchecked(cmd::PLAYBACK_LOADING);

                if let Some(queued) = data.queued_track(item) {
                    data.loading_playback(queued.track, queued.origin);
                    self.update_media_controls(&data.playback);
                } else {
                    log::warn!("loaded item not found in playback queue");
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_PLAYING) => {
                let (item, progress) = cmd.get_unchecked(cmd::PLAYBACK_PLAYING);
                log::info!("playing");

                if let Some(queued) = data.queued_track(item) {
                    data.start_playback(queued.track, queued.origin, progress.to_owned());
                    self.update_media_controls(&data.playback);
                } else {
                    log::warn!("played item not found in playback queue");
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_PROGRESS) => {
                let progress = cmd.get_unchecked(cmd::PLAYBACK_PROGRESS);
                data.progress_playback(progress.to_owned());
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_PAUSING) => {
                data.pause_playback();
                self.update_media_controls(&data.playback);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_RESUMING) => {
                data.resume_playback();
                self.update_media_controls(&data.playback);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_BLOCKED) => {
                data.block_playback();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_STOPPED) => {
                data.stop_playback();
                self.update_media_controls(&data.playback);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::UPDATE_AUDIO_ANALYSIS) => {
                let (track_id, result) = cmd.get_unchecked(cmd::UPDATE_AUDIO_ANALYSIS);
                data.playback.now_playing.as_mut().map(|current| {
                    if current.analysis.is_deferred(track_id) {
                        current.analysis.resolve_or_reject(result.to_owned());
                    }
                });
                ctx.set_handled();
            }
            //
            Event::Command(cmd) if cmd.is(cmd::PLAY_TRACKS) => {
                let payload = cmd.get_unchecked(cmd::PLAY_TRACKS);
                data.playback.queue = payload
                    .tracks
                    .iter()
                    .map(|track| QueuedTrack {
                        origin: payload.origin.to_owned(),
                        track: track.to_owned(),
                    })
                    .collect();
                self.play(&data.playback.queue, payload.position);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_PAUSE) => {
                self.pause();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_RESUME) => {
                self.resume();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_PREVIOUS) => {
                self.previous();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_NEXT) => {
                self.next();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_STOP) => {
                self.stop();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_QUEUE_BEHAVIOR) => {
                let behavior = cmd.get_unchecked(cmd::PLAY_QUEUE_BEHAVIOR);
                data.playback.queue_behavior = behavior.to_owned();
                self.set_queue_behavior(behavior.to_owned());
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_SEEK) => {
                let fraction = cmd.get_unchecked(cmd::PLAY_SEEK);
                data.playback.now_playing.as_ref().map(|current| {
                    let position =
                        Duration::from_secs_f64(current.item.duration.as_secs_f64() * fraction);
                    self.seek(position);
                });
                ctx.set_handled();
            }
            //
            _ => child.event(ctx, event, data, env),
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &State,
        env: &Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => {
                self.open_audio_output_and_start_threads(
                    data.session.clone(),
                    data.config.playback(),
                    ctx.get_external_handle(),
                    ctx.widget_id(),
                );
            }
            _ => {}
        }
        child.lifecycle(ctx, event, data, env);
    }
}
