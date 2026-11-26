use crate::{
    core::{Error, SpotifyUri},
    protocol::player::ProvidedTrack,
    state::{
        ConnectState, SPOTIFY_MAX_NEXT_TRACKS_SIZE, SPOTIFY_MAX_PREV_TRACKS_SIZE, StateError,
        context::ContextType,
        metadata::Metadata,
        provider::{IsProvider, Provider},
    },
};
use protobuf::MessageField;
use rand::Rng;

// identifier used as part of the uid
pub const IDENTIFIER_DELIMITER: &str = "delimiter";

impl<'ct> ConnectState {
    /// Reorder an exact occurrence while leaving playback and history intact.
    pub fn move_queued(
        &mut self,
        uid: &str,
        uri: &str,
        target_uid: &str,
        target_uri: &str,
    ) -> Result<(), Error> {
        let mut tracks = self.local_queue_state().next_tracks;
        let from = tracks.iter().position(|t| t.uid == uid && t.uri == uri);
        let to = tracks
            .iter()
            .position(|t| t.uid == target_uid && t.uri == target_uri);
        let (Some(from), Some(to)) = (from, to) else {
            return Err(Error::failed_precondition(
                "La cola cambió. Intenta mover la canción otra vez.",
            ));
        };
        let track = tracks.remove(from);
        tracks.insert(to, track);
        let context_len = self.get_context(self.fill_up_context)?.tracks.len();
        self.update_context_index(self.fill_up_context, context_len)?;
        self.reordered_tracks = if tracks.len() > SPOTIFY_MAX_NEXT_TRACKS_SIZE {
            tracks.split_off(SPOTIFY_MAX_NEXT_TRACKS_SIZE)
        } else {
            Vec::new()
        };
        self.player_mut().next_tracks = tracks;
        self.update_queue_revision();
        Ok(())
    }

    fn new_delimiter(iteration: i64) -> ProvidedTrack {
        let mut delimiter = ProvidedTrack {
            uri: format!("spotify:{IDENTIFIER_DELIMITER}"),
            uid: format!("{IDENTIFIER_DELIMITER}{iteration}"),
            provider: Provider::Context.to_string(),
            ..Default::default()
        };
        delimiter.set_hidden(true);
        delimiter.set_iteration(iteration);

        delimiter
    }

    fn push_prev(&mut self, prev: ProvidedTrack) {
        let prev_tracks = self.prev_tracks_mut();
        // add prev track, while preserving a length of 10
        if prev_tracks.len() >= SPOTIFY_MAX_PREV_TRACKS_SIZE {
            // todo: O(n), but technically only maximal O(SPOTIFY_MAX_PREV_TRACKS_SIZE) aka O(10)
            let _ = prev_tracks.remove(0);
        }
        prev_tracks.push(prev)
    }

    fn get_next_track(&mut self) -> Option<ProvidedTrack> {
        if self.next_tracks().is_empty() {
            None
        } else {
            // todo: O(n), but technically only maximal O(SPOTIFY_MAX_NEXT_TRACKS_SIZE) aka O(80)
            Some(self.next_tracks_mut().remove(0))
        }
    }

    /// bottom => top, aka the last track of the list is the prev track
    fn prev_tracks_mut(&mut self) -> &mut Vec<ProvidedTrack> {
        &mut self.player_mut().prev_tracks
    }

    /// bottom => top, aka the last track of the list is the prev track
    pub(super) fn prev_tracks(&self) -> &Vec<ProvidedTrack> {
        &self.player().prev_tracks
    }

    /// top => bottom, aka the first track of the list is the next track
    fn next_tracks_mut(&mut self) -> &mut Vec<ProvidedTrack> {
        &mut self.player_mut().next_tracks
    }

    /// top => bottom, aka the first track of the list is the next track
    pub(super) fn next_tracks(&self) -> &Vec<ProvidedTrack> {
        &self.player().next_tracks
    }

    pub fn set_current_track_random(&mut self) -> Result<(), Error> {
        let max_tracks = self.get_context(self.active_context)?.tracks.len();
        let rng_track = rand::rng().random_range(0..max_tracks);
        self.set_current_track(rng_track)
    }

    pub fn set_current_track(&mut self, index: usize) -> Result<(), Error> {
        let context = self.get_context(self.active_context)?;

        let new_track = context
            .tracks
            .get(index)
            .ok_or(StateError::CanNotFindTrackInContext(
                Some(index),
                context.tracks.len(),
            ))?;

        debug!(
            "set track to: {} at {} of {} tracks",
            new_track.uri,
            index,
            context.tracks.len()
        );

        self.set_track(new_track.clone());

        self.update_current_index(|i| i.track = index as u32);

        Ok(())
    }

    /// Move to the next track
    ///
    /// Updates the current track to the next track. Adds the old track
    /// to prev tracks and fills up the next tracks from the current context
    pub fn next_track(&mut self) -> Result<Option<u32>, Error> {
        // when we skip in repeat track, we don't repeat the current track anymore
        if self.repeat_track() {
            self.set_repeat_track(false);
        }

        let uri = self.current_track(|t| t.uri.clone());
        self.shuffle_consumed.insert(uri);
        if self.shuffling_context() {
            let consumed = &self.shuffle_consumed;
            self.request
                .device
                .mut_or_insert_default()
                .player_state
                .mut_or_insert_default()
                .next_tracks
                .retain(|t| t.is_queue() || !consumed.contains(&t.uri));
            self.fill_up_next_tracks()?;
            if self.next_tracks().is_empty()
                && self
                    .get_context(ContextType::Default)?
                    .tracks
                    .iter()
                    .any(|t| !t.is_unavailable())
            {
                self.restart_shuffle_cycle()?;
            }
        }
        let old_track = self.player_mut().track.take();

        if let Some(old_track) = old_track {
            // only add songs from our context to our previous tracks
            if old_track.is_context() || old_track.is_autoplay() {
                self.push_prev(old_track)
            }
        }

        let new_track = loop {
            match self.get_next_track() {
                Some(next) if next.uid.starts_with(IDENTIFIER_DELIMITER) => {
                    self.push_prev(next);
                    continue;
                }
                Some(next) if next.is_unavailable() => continue,
                other => break other,
            };
        };

        let new_track = match new_track {
            None => return Ok(None),
            Some(t) => t,
        };

        let update_index = if new_track.is_queue() {
            None
        } else if new_track.is_autoplay() {
            self.set_active_context(ContextType::Autoplay);
            None
        } else {
            match new_track.get_context_index() {
                Some(new_index) => Some(new_index as u32),
                None => {
                    error!("the given context track had no set context_index");
                    None
                }
            }
        };

        if let Some(update_index) = update_index {
            self.update_current_index(|i| i.track = update_index)
        } else {
            self.player_mut().index.clear()
        }

        self.set_track(new_track);
        self.shuffle_consumed
            .insert(self.current_track(|t| t.uri.clone()));
        self.fill_up_next_tracks()?;
        self.update_restrictions();

        Ok(Some(self.player().index.track))
    }

    /// Move to the prev track
    ///
    /// Updates the current track to the prev track. Adds the old track
    /// to next tracks (when from the context) and fills up the prev tracks from the
    /// current context
    pub fn prev_track(&mut self) -> Result<Option<&MessageField<ProvidedTrack>>, Error> {
        let old_track = self.player_mut().track.take();

        if let Some(old_track) = old_track {
            if old_track.is_context() || old_track.is_autoplay() {
                // todo: O(n)
                self.next_tracks_mut().insert(0, old_track);
            }
        }

        // handle possible delimiter
        if matches!(self.prev_tracks().last(), Some(prev) if prev.uid.starts_with(IDENTIFIER_DELIMITER))
        {
            let delimiter = self
                .prev_tracks_mut()
                .pop()
                .expect("item that was prechecked");

            let next_tracks = self.next_tracks_mut();
            if next_tracks.len() >= SPOTIFY_MAX_NEXT_TRACKS_SIZE {
                let _ = next_tracks.pop();
            }
            // todo: O(n)
            next_tracks.insert(0, delimiter)
        }

        while self.next_tracks().len() > SPOTIFY_MAX_NEXT_TRACKS_SIZE {
            let _ = self.next_tracks_mut().pop();
        }

        let new_track = match self.prev_tracks_mut().pop() {
            None => return Ok(None),
            Some(t) => t,
        };

        if matches!(self.active_context, ContextType::Autoplay if new_track.is_context()) {
            // transition back to default context
            self.set_active_context(ContextType::Default);
        }

        self.fill_up_next_tracks()?;
        self.set_track(new_track);

        if self.player().index.track == 0 {
            warn!("prev: trying to skip into negative, index update skipped")
        } else {
            self.update_current_index(|i| i.track -= 1)
        }

        self.update_restrictions();

        Ok(Some(self.current_track(|t| t)))
    }

    pub fn current_track<F: Fn(&'ct MessageField<ProvidedTrack>) -> R, R>(
        &'ct self,
        access: F,
    ) -> R {
        access(&self.player().track)
    }

    pub fn set_track(&mut self, track: ProvidedTrack) {
        self.player_mut().track = MessageField::some(track)
    }

    pub fn set_next_tracks(&mut self, mut tracks: Vec<ProvidedTrack>) {
        self.reordered_tracks.clear();
        // mobile only sends a set_queue command instead of an add_to_queue command
        // in addition to handling the mobile add_to_queue handling, this should also handle
        // a mass queue addition
        tracks
            .iter_mut()
            .filter(|t| t.is_from_queue())
            .for_each(|t| {
                t.set_provider(Provider::Queue);
                // technically we could preserve the queue-uid here,
                // but it seems to work without that, so we just override it
                t.uid = format!("q{}", self.queue_count);
                self.queue_count += 1;
            });

        // when you drag 'n drop the current track in the queue view into the "Next from: ..."
        // section, it is only send as an empty item with just the provider and metadata, so we have
        // to provide set the uri from the current track manually
        tracks
            .iter_mut()
            .filter(|t| t.uri.is_empty())
            .for_each(|t| t.uri = self.current_track(|ct| ct.uri.clone()));

        self.player_mut().next_tracks = tracks;
    }

    pub fn set_prev_tracks(&mut self, tracks: Vec<ProvidedTrack>) {
        self.player_mut().prev_tracks = tracks;
    }

    pub fn clear_prev_track(&mut self) {
        self.prev_tracks_mut().clear()
    }

    pub fn clear_next_tracks(&mut self) {
        self.reordered_tracks.clear();
        // respect queued track and don't throw them out of our next played tracks
        let first_non_queued_track = self
            .next_tracks()
            .iter()
            .enumerate()
            .find(|(_, track)| !track.is_queue());

        if let Some((non_queued_track, _)) = first_non_queued_track {
            while self.next_tracks().len() > non_queued_track
                && self.next_tracks_mut().pop().is_some()
            {}
        }
    }

    pub fn fill_up_next_tracks(&mut self) -> Result<(), Error> {
        let ctx = self.get_context(self.fill_up_context)?;
        let mut new_index = ctx.index.track as usize;
        let mut iteration = ctx.index.page;

        while self.next_tracks().len() < SPOTIFY_MAX_NEXT_TRACKS_SIZE {
            if !self.reordered_tracks.is_empty() {
                let track = self.reordered_tracks.remove(0);
                self.next_tracks_mut().push(track);
                continue;
            }
            let ctx = self.get_context(self.fill_up_context)?;
            let track = match ctx.tracks.get(new_index) {
                None if self.repeat_context() && !self.shuffling_context() => {
                    let delimiter = Self::new_delimiter(iteration.into());
                    iteration += 1;
                    new_index = 0;
                    delimiter
                }
                None if !matches!(self.fill_up_context, ContextType::Autoplay)
                    && self.autoplay_context.is_some()
                    && !self.repeat_context() =>
                {
                    self.update_context_index(self.fill_up_context, new_index)?;

                    // transition to autoplay as fill up context
                    self.fill_up_context = ContextType::Autoplay;
                    new_index = self.get_context(ContextType::Autoplay)?.index.track as usize;

                    // add delimiter to only display the current context
                    Self::new_delimiter(iteration.into())
                }
                None if self.autoplay_context.is_some() => {
                    match self
                        .get_context(ContextType::Autoplay)?
                        .tracks
                        .get(new_index)
                    {
                        None => break,
                        Some(ct) => {
                            new_index += 1;
                            ct.clone()
                        }
                    }
                }
                None => break,
                Some(ct)
                    if ct.is_unavailable()
                        || self.is_skip_track(ct, Some(iteration))
                        || (self.shuffling_context()
                            && (self.shuffle_consumed.contains(&ct.uri)
                                || self.next_tracks().iter().any(|t| t.uri == ct.uri))) =>
                {
                    debug!(
                        "skipped track {} during fillup as it's unavailable or should be skipped",
                        ct.uri
                    );
                    new_index += 1;
                    continue;
                }
                Some(ct) => {
                    new_index += 1;
                    ct.clone()
                }
            };

            self.next_tracks_mut().push(track);
        }

        debug!(
            "finished filling up next_tracks ({})",
            self.next_tracks().len()
        );

        self.update_context_index(self.fill_up_context, new_index)?;

        // the web-player needs a revision update, otherwise the queue isn't updated in the ui
        self.update_queue_revision();

        Ok(())
    }

    pub fn preview_next_track(&mut self) -> Option<SpotifyUri> {
        let next = if self.repeat_track() {
            self.current_track(|t| &t.uri)
        } else {
            &self.next_tracks().first()?.uri
        };

        SpotifyUri::from_uri(next).ok()
    }

    /// The local UI can browse the full context; Spotify's wire queue stays capped at 80.
    pub fn local_queue_state(&self) -> crate::protocol::player::PlayerState {
        let mut player = self.player().clone();
        player
            .next_tracks
            .extend(self.reordered_tracks.iter().cloned());
        if let Ok(ctx) = self.get_context(self.fill_up_context) {
            let mut seen: std::collections::HashSet<_> = self.shuffle_consumed.clone();
            seen.insert(player.track.uri.clone());
            for track in &player.next_tracks {
                seen.insert(track.uri.clone());
            }
            player.next_tracks.extend(
                ctx.tracks
                    .iter()
                    .skip(ctx.index.track as usize)
                    .filter(|t| !t.is_unavailable() && !self.is_skip_track(t, Some(ctx.index.page)))
                    .filter(|t| !self.shuffling_context() || seen.insert(t.uri.clone()))
                    .cloned(),
            );
        }
        player
    }

    pub fn has_next_tracks(&self, min: Option<usize>) -> bool {
        if let Some(min) = min {
            self.next_tracks().len() >= min
        } else {
            !self.next_tracks().is_empty()
        }
    }

    pub fn recent_track_uris(&self) -> Vec<String> {
        let mut prev = self
            .prev_tracks()
            .iter()
            .map(|t| t.uri.clone())
            .collect::<Vec<_>>();

        prev.push(self.current_track(|t| t.uri.clone()));
        prev
    }

    pub fn mark_unavailable(&mut self, id: &SpotifyUri) -> Result<(), Error> {
        let uri = id.to_uri()?;

        debug!("marking {uri} as unavailable");

        let next_tracks = self.next_tracks_mut();
        while let Some(pos) = next_tracks.iter().position(|t| t.uri == uri) {
            let _ = next_tracks.remove(pos);
        }

        for next_track in next_tracks {
            Self::mark_as_unavailable_for_match(next_track, &uri)
        }

        let prev_tracks = self.prev_tracks_mut();
        while let Some(pos) = prev_tracks.iter().position(|t| t.uri == uri) {
            let _ = prev_tracks.remove(pos);
        }

        for prev_track in prev_tracks {
            Self::mark_as_unavailable_for_match(prev_track, &uri)
        }

        self.unavailable_uri.push(uri);
        self.fill_up_next_tracks()?;
        self.update_queue_revision();

        Ok(())
    }

    pub fn add_to_queue(&mut self, mut track: ProvidedTrack, rev_update: bool) {
        track.uid = format!("q{}", self.queue_count);
        self.queue_count += 1;

        track.set_provider(Provider::Queue);
        if !track.is_from_queue() {
            track.set_from_queue(true);
        }

        let next_tracks = self.next_tracks_mut();
        if let Some(next_not_queued_track) = next_tracks.iter().position(|t| !t.is_queue()) {
            next_tracks.insert(next_not_queued_track, track);
        } else {
            next_tracks.push(track)
        }

        let displaced = if next_tracks.len() > SPOTIFY_MAX_NEXT_TRACKS_SIZE {
            next_tracks.split_off(SPOTIFY_MAX_NEXT_TRACKS_SIZE)
        } else {
            Vec::new()
        };
        self.reordered_tracks.splice(0..0, displaced);

        if rev_update {
            self.update_queue_revision();
        }
        self.update_restrictions();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{protocol::connect::Device, state::context::StateContext};
    use std::collections::{HashMap, HashSet};

    fn state(count: usize, shuffle: bool) -> ConnectState {
        let mut state = ConnectState::default();
        state.request.device = MessageField::some(Device {
            player_state: MessageField::some(Default::default()),
            ..Default::default()
        });
        let tracks: Vec<_> = (0..count)
            .map(|i| ProvidedTrack {
                uri: format!("spotify:track:{i:022}"),
                uid: format!("track-{i}"),
                provider: Provider::Context.to_string(),
                ..Default::default()
            })
            .collect();
        state.context = Some(StateContext {
            tracks: tracks.into(),
            metadata: HashMap::new(),
            restrictions: None,
            index: Default::default(),
        });
        state.set_current_track(0).unwrap();
        state.set_shuffle(shuffle);
        if shuffle {
            state.shuffle_new().unwrap();
        } else {
            state.reset_playback_to_position(Some(0)).unwrap();
        }
        state
    }

    #[test]
    fn full_local_queue_is_selectable_beyond_spotify_window() {
        for shuffle in [false, true] {
            let mut state = state(240, shuffle);
            let displayed = state.local_queue_state().next_tracks;
            assert_eq!(state.next_tracks().len(), 80);
            assert_eq!(displayed.len(), 239);
            for expected in displayed {
                assert!(state.next_track().unwrap().is_some());
                assert_eq!(state.player().track.uid, expected.uid);
            }
            if shuffle {
                assert!(state.next_track().unwrap().is_some());
            } else {
                assert!(state.next_track().unwrap().is_none());
            }
        }
    }

    #[test]
    fn moving_and_selecting_beyond_wire_window_preserve_the_remaining_order() {
        for shuffle in [false, true] {
            let mut state = state(240, shuffle);
            let current = state.player().track.clone();
            let mut expected = state.local_queue_state().next_tracks;
            let source = expected[180].clone();
            let target = expected[4].clone();
            state
                .move_queued(&source.uid, &source.uri, &target.uid, &target.uri)
                .unwrap();
            let moved = expected.remove(180);
            expected.insert(4, moved);
            assert_eq!(state.player().track, current);
            assert_eq!(state.local_queue_state().next_tracks, expected);
            assert_eq!(state.next_tracks().len(), 80);
            let selected = expected.remove(130);
            let first = expected[0].clone();
            state
                .move_queued(&selected.uid, &selected.uri, &first.uid, &first.uri)
                .unwrap();
            state.next_track().unwrap();
            assert_eq!(state.player().track.uid, selected.uid);
            assert_eq!(state.local_queue_state().next_tracks, expected);
            for next in expected {
                state.next_track().unwrap();
                assert_eq!(state.player().track.uid, next.uid);
                assert!(state.next_tracks().len() <= 80);
            }
        }
    }

    #[test]
    fn manual_duplicates_and_new_additions_survive_reordering() {
        let mut state = state(160, true);
        let duplicate = ProvidedTrack {
            uri: "spotify:track:manual".into(),
            ..Default::default()
        };
        state.add_to_queue(duplicate.clone(), true);
        state.add_to_queue(duplicate.clone(), true);
        let before = state.local_queue_state().next_tracks;
        let source = before[1].clone();
        let target = before[120].clone();
        state
            .move_queued(&source.uid, &source.uri, &target.uid, &target.uri)
            .unwrap();
        let mut expected = before;
        let moved = expected.remove(1);
        expected.insert(120, moved);
        state.add_to_queue(
            ProvidedTrack {
                uri: "spotify:track:new".into(),
                ..Default::default()
            },
            true,
        );
        let actual = state.local_queue_state().next_tracks;
        assert_eq!(actual.len(), expected.len() + 1);
        assert_eq!(actual.iter().filter(|t| t.uri == duplicate.uri).count(), 2);
        let actual_without_new: Vec<_> = actual
            .into_iter()
            .filter(|t| t.uri != "spotify:track:new")
            .collect();
        assert_eq!(actual_without_new, expected);
    }

    #[test]
    fn shuffle_cycles_do_not_repeat_and_forward_skips_survive_reshuffle() {
        let mut state = state(150, true);
        let mut consumed = HashSet::new();
        consumed.insert(state.player().track.uri.clone());
        for _ in 0..49 {
            state.next_track().unwrap();
            assert!(consumed.insert(state.player().track.uri.clone()));
        }
        state.shuffle_new().unwrap();
        let upcoming = state.local_queue_state().next_tracks;
        assert_eq!(upcoming.len(), 100);
        assert!(upcoming.iter().all(|t| !consumed.contains(&t.uri)));
        for _ in 0..100 {
            state.next_track().unwrap();
            assert!(consumed.insert(state.player().track.uri.clone()));
        }
        let last = state.player().track.uri.clone();
        for cycle in 0..3 {
            let mut seen = HashSet::new();
            for i in 0..150 {
                assert!(state.next_track().unwrap().is_some());
                if cycle == 0 && i == 0 {
                    assert_ne!(state.player().track.uri, last);
                }
                assert!(seen.insert(state.player().track.uri.clone()));
            }
        }
    }

    #[test]
    fn duplicate_context_songs_play_once_per_shuffle_cycle() {
        let mut state = state(5, true);
        let duplicate = state.context.as_ref().unwrap().tracks[2].clone();
        state.context.as_mut().unwrap().tracks.push(duplicate);
        state.shuffle_new().unwrap();
        let mut seen = HashSet::new();
        seen.insert(state.player().track.uri.clone());
        for _ in 0..4 {
            state.next_track().unwrap();
            assert!(seen.insert(state.player().track.uri.clone()));
        }
        assert!(state.next_track().unwrap().is_some());
    }
}
