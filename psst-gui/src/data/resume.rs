use super::{AppState, Nav, Playable, PlaybackOrigin, PlaybackState, QueueEntry, Track};
use druid::{im::Vector, Data};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Data, Serialize, Deserialize)]
pub struct ResumeEntry {
    pub track: Arc<Track>,
    pub origin: Nav,
}

#[derive(Clone, Debug, Data, Serialize, Deserialize)]
pub struct ResumeSnapshot {
    pub items: Vector<ResumeEntry>,
    pub position: usize,
    pub progress_ms: u64,
    #[serde(default)]
    #[data(ignore)]
    pub engine_queue: Option<psst_core::player::queue::QueueSnapshot>,
}

impl AppState {
    pub fn capture_resume(&mut self) {
        let Some(np) = &self.playback.now_playing else {
            return;
        };
        let Some(current) = np.item.track() else {
            return;
        };
        let queue = &self.playback.queue;
        let position = queue
            .iter()
            .position(|entry| entry.item.id() == current.id.0);
        let entries: Vector<_> = queue
            .iter()
            .filter_map(|entry| {
                entry.item.track().map(|track| ResumeEntry {
                    track: track.clone(),
                    origin: entry.origin.to_nav(),
                })
            })
            .take(5000)
            .collect();
        let (mut items, mut position) = match position {
            Some(position) if entries.len() == queue.len() && position < entries.len() => {
                (entries, position)
            }
            _ => (
                druid::im::vector![ResumeEntry {
                    track: current.clone(),
                    origin: np.origin.to_nav()
                }],
                0,
            ),
        };
        let mut engine_queue = None;
        if self.connect.selected.is_none() {
            if let Some(engine) = &self.engine_queue {
                let all = engine.items.iter().chain(engine.user_items.iter());
                let entries: Vector<_> = all
                    .filter_map(|item| self.queued_entry(item.item_id))
                    .filter_map(|entry| {
                        entry.item.track().map(|track| ResumeEntry {
                            track: track.clone(),
                            origin: entry.origin.to_nav(),
                        })
                    })
                    .collect();
                if !engine.items.is_empty()
                    && entries.len() == engine.items.len() + engine.user_items.len()
                    && entries.len() <= 5000
                {
                    if let Some(index) = engine.positions.get(engine.position) {
                        if entries
                            .get(*index)
                            .is_some_and(|entry| entry.track.id == current.id)
                        {
                            items = entries;
                            position = *index;
                            engine_queue = Some(engine.clone());
                        }
                    }
                }
            }
        }
        self.config.last_playback = Some(ResumeSnapshot {
            items,
            position,
            progress_ms: np
                .progress
                .as_millis()
                .min(current.duration.as_millis().saturating_sub(1))
                as u64,
            engine_queue,
        });
    }

    pub fn restore_resume(&mut self) {
        let Some(snapshot) = self.config.last_playback.clone() else {
            return;
        };
        let Some(entry) = snapshot.items.get(snapshot.position) else {
            return;
        };
        if snapshot.items.len() > 5000 {
            return;
        }
        let progress = Duration::from_millis(
            snapshot
                .progress_ms
                .min(entry.track.duration.as_millis().saturating_sub(1) as u64),
        );
        let origin = origin_from_nav(&entry.origin);
        self.playback.queue = snapshot
            .items
            .iter()
            .map(|entry| QueueEntry {
                item: Playable::Track(entry.track.clone()),
                origin: origin_from_nav(&entry.origin),
            })
            .collect();
        self.engine_queue = snapshot.engine_queue.filter(|engine| {
            let mut queue = psst_core::player::queue::Queue::new();
            queue.restore(engine.clone())
                && queue
                    .get_current()
                    .is_some_and(|item| item.item_id == entry.track.id.0)
                && engine
                    .items
                    .iter()
                    .chain(engine.user_items.iter())
                    .all(|item| {
                        snapshot
                            .items
                            .iter()
                            .any(|entry| entry.track.id.0 == item.item_id)
                    })
        });
        if let Some(saved) = &mut self.config.last_playback {
            saved.engine_queue = self.engine_queue.clone();
        }
        self.start_playback(Playable::Track(entry.track.clone()), origin, progress);
        self.playback.state = PlaybackState::Paused;
        self.nav = entry.origin.clone();
        self.common_ctx_mut().nav = self.nav.clone();
    }
}

pub fn origin_from_nav(nav: &Nav) -> PlaybackOrigin {
    match nav {
        Nav::PlaylistDetail(link) => PlaybackOrigin::Playlist(link.clone()),
        Nav::AlbumDetail(link, _) => PlaybackOrigin::Album(link.clone()),
        Nav::SavedTracks => PlaybackOrigin::Library,
        Nav::ShowDetail(link) => PlaybackOrigin::Show(link.clone()),
        Nav::SearchResults(query) => PlaybackOrigin::Search(query.clone()),
        Nav::Recommendations(request) => PlaybackOrigin::Recommendations(request.clone()),
        _ => PlaybackOrigin::Home,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_snapshot_keeps_manual_tracks_and_exact_engine_order() {
        use psst_core::{
            audio::normalize::NormalizationLevel,
            item_id::{ItemId, ItemIdType},
            player::{
                item::PlaybackItem,
                queue::{Queue, QueueBehavior},
            },
        };
        let tracks: Vec<_> = (1..=3)
            .map(|index| {
                let mut track: Track = serde_json::from_value(serde_json::json!({
                    "id":"5lfWrciYtohtIMVDVZd0Rf","name":format!("Song {index}"),
                    "artists":[{"id":"example","name":"Artist"}],"duration_ms":240000,
                    "disc_number":1,"track_number":1,"explicit":false,"is_local":false
                }))
                .unwrap();
                track.id = super::super::TrackId(ItemId::new(index, ItemIdType::Track));
                Arc::new(track)
            })
            .collect();
        let entry = |index: usize| QueueEntry {
            item: Playable::Track(tracks[index].clone()),
            origin: PlaybackOrigin::Library,
        };
        let item = |index: usize| PlaybackItem {
            item_id: tracks[index].id.0,
            norm_level: NormalizationLevel::Track,
        };
        let mut state = AppState::default_with_config(super::super::Config::default());
        state.playback.queue = druid::im::vector![entry(0), entry(1)];
        state.added_queue = druid::im::vector![entry(2), entry(2)];
        let mut engine = Queue::new();
        engine.fill(vec![item(0), item(1)], 0);
        engine.set_behaviour(QueueBehavior::Random);
        engine.add(item(2));
        engine.add(item(2));
        engine.skip_to_following();
        state.engine_queue = Some(engine.snapshot());
        state.start_playback(
            Playable::Track(tracks[2].clone()),
            PlaybackOrigin::Library,
            Duration::from_secs(37),
        );
        state.capture_resume();
        let config = serde_json::from_str(&serde_json::to_string(&state.config).unwrap()).unwrap();
        let restored = AppState::default_with_config(config);
        assert_eq!(restored.playback.state, PlaybackState::Paused);
        assert_eq!(
            restored.playback.now_playing.as_ref().unwrap().item.id(),
            tracks[2].id.0
        );
        let mut resumed_engine = Queue::new();
        assert!(resumed_engine.restore(restored.engine_queue.unwrap()));
        assert_eq!(resumed_engine.upcoming_ids(), engine.upcoming_ids());
    }
    #[test]
    fn round_trip_restores_track_queue_position_and_pause_without_credentials() {
        let track: Track = serde_json::from_value(serde_json::json!({
            "id":"5lfWrciYtohtIMVDVZd0Rf", "name":"Example", "artists":[{"id":"example","name":"Artist"}],
            "duration_ms":240000, "disc_number":1,"track_number":1,"explicit":false,"is_local":false
        })).unwrap();
        let mut state = AppState::default_with_config(super::super::Config::default());
        state.playback.queue = druid::im::vector![QueueEntry {
            item: Playable::Track(Arc::new(track.clone())),
            origin: PlaybackOrigin::Library
        }];
        state.start_playback(
            Playable::Track(Arc::new(track)),
            PlaybackOrigin::Library,
            Duration::from_secs(37),
        );
        state.capture_resume();
        let json = serde_json::to_string(state.config.last_playback.as_ref().unwrap()).unwrap();
        let mut config = super::super::Config::default();
        config.last_playback = Some(serde_json::from_str(&json).unwrap());
        let restored = AppState::default_with_config(config);
        assert_eq!(restored.playback.state, PlaybackState::Paused);
        assert_eq!(
            restored.playback.now_playing.unwrap().progress,
            Duration::from_secs(37)
        );
        assert_eq!(restored.playback.queue.len(), 1);
        assert_eq!(restored.nav, Nav::SavedTracks);
    }
}
