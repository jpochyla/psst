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
        let (items, position) = match position {
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
        self.config.last_playback = Some(ResumeSnapshot {
            items,
            position,
            progress_ms: np
                .progress
                .as_millis()
                .min(current.duration.as_millis().saturating_sub(1))
                as u64,
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
