use rand::prelude::SliceRandom;

use super::PlaybackItem;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum QueueBehavior {
    #[default]
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueueSnapshot {
    pub items: Vec<PlaybackItem>,
    pub user_items: Vec<PlaybackItem>,
    pub position: usize,
    pub user_items_position: usize,
    pub positions: Vec<usize>,
    pub behavior: QueueBehavior,
    #[serde(default)]
    pub consumed: Vec<crate::item_id::ItemId>,
    #[serde(default)]
    pub manual_indices: Vec<usize>,
}

#[derive(Clone)]
pub struct Queue {
    items: Vec<PlaybackItem>,
    user_items: Vec<PlaybackItem>,
    position: usize,
    user_items_position: usize,
    positions: Vec<usize>,
    behavior: QueueBehavior,
    consumed: Vec<crate::item_id::ItemId>,
    manual_indices: Vec<usize>,
}

impl Queue {
    pub fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            items: self.items.clone(),
            user_items: self.user_items.clone(),
            position: self.position,
            user_items_position: self.user_items_position,
            positions: self.positions.clone(),
            behavior: self.behavior.clone(),
            consumed: self.consumed.clone(),
            manual_indices: self.manual_indices.clone(),
        }
    }

    pub fn restore(&mut self, snapshot: QueueSnapshot) -> bool {
        if snapshot.items.len() + snapshot.user_items.len() > 5000
            || snapshot.positions.len() != snapshot.items.len()
            || snapshot.user_items_position > snapshot.user_items.len()
            || snapshot.position > snapshot.positions.len()
            || snapshot
                .manual_indices
                .iter()
                .any(|&i| i >= snapshot.items.len())
        {
            return false;
        }
        let mut indices = snapshot.positions.clone();
        indices.sort_unstable();
        if indices
            .iter()
            .enumerate()
            .any(|(index, value)| *value != index)
        {
            return false;
        }
        self.items = snapshot.items;
        self.user_items = snapshot.user_items;
        self.position = snapshot.position;
        self.user_items_position = snapshot.user_items_position;
        self.positions = snapshot.positions;
        self.behavior = snapshot.behavior;
        self.consumed = snapshot.consumed;
        self.manual_indices = snapshot.manual_indices;
        true
    }
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            user_items: Vec::new(),
            position: 0,
            user_items_position: 0,
            positions: Vec::new(),
            behavior: QueueBehavior::default(),
            consumed: Vec::new(),
            manual_indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.manual_indices.clear();
        self.consumed.clear();
        self.user_items.clear();
        self.user_items_position = 0;
        self.items.clear();
        self.positions.clear();
        self.position = 0;
    }

    pub fn fill(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.manual_indices.clear();
        self.consumed.clear();
        self.user_items.clear();
        self.user_items_position = 0;
        self.positions.clear();
        self.items = items;
        self.position = position;
        self.compute_positions();
    }

    pub fn add(&mut self, item: PlaybackItem) {
        self.user_items.push(item);
    }

    fn handle_added_queue(&mut self) -> Option<usize> {
        if self.user_items.len() <= self.user_items_position {
            return None;
        }
        let insertion = if self.get_current().is_some() {
            self.position + 1
        } else {
            self.positions.len()
        };
        let index = self.items.len();
        self.items.push(self.user_items[self.user_items_position]);
        self.manual_indices.push(index);
        self.positions.insert(insertion, index);
        self.user_items_position += 1;
        Some(insertion)
    }

    /// Preview one traversal without changing playback or reshuffling the queue.
    pub fn upcoming_ids(&self) -> Vec<crate::item_id::ItemId> {
        if self.get_current().is_none() {
            return self.user_items[self.user_items_position..]
                .iter()
                .map(|item| item.item_id)
                .collect();
        }
        let count = match self.behavior {
            QueueBehavior::LoopTrack => 1,
            _ => {
                self.items.len()
                    + self
                        .user_items
                        .len()
                        .saturating_sub(self.user_items_position)
            }
        };
        let mut preview = self.clone();
        let mut upcoming = Vec::new();
        for _ in 0..count {
            preview.advance(false, false);
            let Some(item) = preview.get_current() else {
                break;
            };
            upcoming.push(item.item_id);
        }
        upcoming
    }

    pub fn set_behaviour(&mut self, behavior: QueueBehavior) {
        self.behavior = behavior;
        self.compute_positions();
    }

    /// Jump within the displayed traversal, preserving shuffle and manual additions.
    /// Validate the item ID to reject a stale click after the queue changes.
    pub fn select_upcoming(&mut self, index: usize, expected: crate::item_id::ItemId) -> bool {
        if index >= 5000 || self.upcoming_ids().get(index) != Some(&expected) {
            return false;
        }
        let mut selected = self.clone();
        if !matches!(selected.behavior, QueueBehavior::LoopTrack) {
            let before = selected.upcoming_ids();
            if !selected.move_upcoming(index, 0, &before) {
                return false;
            }
        }
        selected.skip_to_following();
        if selected
            .get_current()
            .is_none_or(|item| item.item_id != expected)
        {
            return false;
        }
        *self = selected;
        true
    }

    /// Move an occurrence in the visible traversal without restarting playback.
    pub fn move_upcoming(
        &mut self,
        from: usize,
        to: usize,
        expected: &[crate::item_id::ItemId],
    ) -> bool {
        if from >= expected.len() || to >= expected.len() || self.upcoming_ids() != expected {
            return false;
        }
        if from == to {
            return true;
        }
        if self.get_current().is_none() {
            let offset = self.user_items_position;
            let moved = self.user_items.remove(offset + from);
            self.user_items.insert(offset + to, moved);
            return true;
        }
        if matches!(self.behavior, QueueBehavior::LoopTrack) {
            return false;
        }
        let mut edited = self.clone();
        let insertion = if edited.get_current().is_some() {
            edited.position + 1
        } else {
            edited.positions.len()
        };
        let pending: Vec<_> = edited.user_items[edited.user_items_position..].to_vec();
        let start = edited.items.len();
        edited.items.extend(pending.iter().copied());
        edited.manual_indices.extend(start..edited.items.len());
        edited
            .positions
            .splice(insertion..insertion, start..edited.items.len());
        edited.user_items_position = edited.user_items.len();
        let mut preview = edited.clone();
        let mut slots = Vec::new();
        for expected_id in expected {
            preview.advance(false, false);
            if preview.get_current().map(|item| item.item_id) != Some(*expected_id) {
                return false;
            }
            slots.push(preview.position);
        }
        // Repeat-all previews end at the currently playing occurrence.
        if slots[from] == edited.position || slots[to] == edited.position {
            return false;
        }
        let mut order: Vec<_> = slots.iter().map(|&slot| edited.positions[slot]).collect();
        let moved = order.remove(from);
        order.insert(to, moved);
        for (slot, item) in slots.into_iter().zip(order) {
            edited.positions[slot] = item;
        }
        *self = edited;
        true
    }

    fn compute_positions(&mut self) {
        // In the case of switching away from shuffle, the position should be set back to
        // where it appears in the actual playlist order.
        let playlist_position = self
            .positions
            .get(self.position)
            .copied()
            .unwrap_or(self.position);
        // Start with an ordered 1:1 mapping.
        self.positions = (0..self.items.len()).collect();

        if let QueueBehavior::Random = self.behavior {
            // Swap the current position with the first item, so we will start from the
            // beginning, with the full queue ahead of us.  Then shuffle the rest of the
            // items and set the position to 0.
            let current = playlist_position.min(self.positions.len().saturating_sub(1));
            self.positions
                .retain(|&i| i != current && self.consumed.contains(&self.items[i].item_id));
            self.position = self.positions.len();
            if !self.items.is_empty() {
                self.positions.push(current);
            }
            let mut remaining: Vec<_> = (0..self.items.len())
                .filter(|&i| i != current && !self.consumed.contains(&self.items[i].item_id))
                .collect();
            remaining.shuffle(&mut rand::rng());
            self.positions.extend(remaining);
        } else {
            self.position = playlist_position;
        }
    }

    pub fn skip_to_previous(&mut self) {
        self.position = self.previous_position();
    }

    pub fn skip_to_next(&mut self) {
        self.advance(true, true);
    }

    pub fn skip_to_following(&mut self) {
        self.advance(false, true);
    }

    fn advance(&mut self, explicit_next: bool, restart: bool) {
        if let Some(item) = self.get_current() {
            let id = item.item_id;
            if !self.consumed.contains(&id) {
                self.consumed.push(id);
            }
        }
        let had_current = self.get_current().is_some();
        let inserted = self.handle_added_queue();
        self.position = if !had_current {
            inserted.unwrap_or_else(|| self.following_position())
        } else if explicit_next {
            self.next_position()
        } else {
            self.following_position()
        };
        if matches!(self.behavior, QueueBehavior::Random) && inserted.is_none() {
            while self
                .get_current()
                .is_some_and(|i| self.consumed.contains(&i.item_id))
                && !self
                    .positions
                    .get(self.position)
                    .is_some_and(|i| self.manual_indices.contains(i))
            {
                self.position += 1;
            }
            if restart && self.position >= self.positions.len() && !self.items.is_empty() {
                let last = self.consumed.last().copied();
                self.consumed.clear();
                self.positions.shuffle(&mut rand::rng());
                if self.positions.len() > 1 && Some(self.items[self.positions[0]].item_id) == last {
                    if let Some(i) = self
                        .positions
                        .iter()
                        .position(|&i| Some(self.items[i].item_id) != last)
                    {
                        self.positions.swap(0, i);
                    }
                }
                self.position = 0;
            }
        }
    }

    pub fn get_current(&self) -> Option<&PlaybackItem> {
        let position = self.positions.get(self.position).copied()?;
        self.items.get(position)
    }

    pub fn get_following(&self) -> Option<&PlaybackItem> {
        if let Some(position) = self.positions.get(self.position).copied() {
            if let Some(item) = self.items.get(position) {
                return Some(item);
            }
        } else {
            return self.user_items.first();
        }
        None
    }

    fn previous_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential
            | QueueBehavior::Random
            | QueueBehavior::LoopTrack
            | QueueBehavior::LoopAll => self.position.saturating_sub(1),
        }
    }

    fn next_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential | QueueBehavior::Random | QueueBehavior::LoopTrack => {
                self.position + 1
            }
            QueueBehavior::LoopAll if !self.items.is_empty() => {
                (self.position + 1) % self.items.len()
            }
            QueueBehavior::LoopAll => 0,
        }
    }

    fn following_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential | QueueBehavior::Random => self.position + 1,
            QueueBehavior::LoopTrack => self.position,
            QueueBehavior::LoopAll if !self.items.is_empty() => {
                (self.position + 1) % self.items.len()
            }
            QueueBehavior::LoopAll => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        audio::normalize::NormalizationLevel,
        item_id::{ItemId, ItemIdType},
    };
    fn item(id: u128) -> PlaybackItem {
        PlaybackItem {
            item_id: ItemId::new(id, ItemIdType::Track),
            norm_level: NormalizationLevel::Track,
        }
    }
    fn ids(queue: &Queue) -> Vec<u128> {
        queue.upcoming_ids().iter().map(|id| id.id).collect()
    }
    #[test]
    fn moving_occurrences_preserves_current_duplicates_and_persisted_order() {
        for behavior in [
            QueueBehavior::Sequential,
            QueueBehavior::Random,
            QueueBehavior::LoopAll,
        ] {
            let mut queue = Queue::new();
            queue.fill((1..=8).map(item).collect(), 0);
            queue.set_behaviour(behavior);
            queue.add(item(90));
            queue.add(item(90));
            let current = queue.get_current().copied();
            let before = queue.upcoming_ids();
            let mut expected = before.clone();
            let moved = expected.remove(1);
            expected.insert(4, moved);
            assert!(queue.move_upcoming(1, 4, &before));
            assert_eq!(queue.get_current().copied(), current);
            assert_eq!(queue.upcoming_ids(), expected);
            assert!(!queue.move_upcoming(1, 4, &before));
            let mut restored = Queue::new();
            assert!(restored.restore(queue.snapshot()));
            assert_eq!(restored.upcoming_ids(), expected);
            for next in expected {
                restored.skip_to_following();
                assert_eq!(restored.get_current().unwrap().item_id, next);
            }
        }
    }
    #[test]
    fn shuffle_continues_in_unique_cycles_and_remembers_skips() {
        let mut queue = Queue::new();
        queue.fill((1..=150).map(item).collect(), 0);
        queue.set_behaviour(QueueBehavior::Random);
        let initial = ids(&queue);
        let skipped: Vec<_> = initial[..49].to_vec();
        assert!(queue.select_upcoming(48, item(skipped[48]).item_id));
        queue.set_behaviour(QueueBehavior::Random);
        let rest = ids(&queue);
        assert_eq!(rest.len(), 148);
        assert!(rest.iter().all(|id| *id != 1 && *id != skipped[48]));
        for id in rest {
            queue.skip_to_following();
            assert_eq!(queue.get_current().unwrap().item_id.id, id);
        }
        for _ in 0..3 {
            let mut seen = std::collections::HashSet::new();
            for _ in 0..150 {
                queue.skip_to_following();
                assert!(seen.insert(queue.get_current().unwrap().item_id));
            }
        }
    }
    #[test]
    fn selecting_upcoming_preserves_the_remaining_shuffle_and_manual_order() {
        let mut queue = Queue::new();
        queue.fill((1..=10).map(item).collect(), 0);
        queue.set_behaviour(QueueBehavior::Random);
        queue.add(item(99));
        queue.add(item(99));
        let before = queue.upcoming_ids();
        let mut remaining = before.clone();
        remaining.remove(3);
        assert!(queue.select_upcoming(3, before[3]));
        assert_eq!(queue.get_current().unwrap().item_id, before[3]);
        assert_eq!(queue.upcoming_ids(), remaining);
        let saved = serde_json::to_string(&queue.snapshot()).unwrap();
        assert!(!queue.select_upcoming(0, item(999).item_id));
        assert!(!queue.select_upcoming(5000, item(99).item_id));
        assert_eq!(saved, serde_json::to_string(&queue.snapshot()).unwrap());
        let mut idle = Queue::new();
        idle.add(item(1));
        idle.add(item(2));
        assert!(idle.select_upcoming(1, item(2).item_id));
        assert_eq!(idle.get_current(), Some(&item(2)));
    }
    #[test]
    fn persisted_queue_restores_shuffle_and_manual_duplicates_exactly() {
        let mut queue = Queue::new();
        queue.fill((1..12).map(item).collect(), 4);
        queue.set_behaviour(QueueBehavior::Random);
        queue.add(item(90));
        queue.add(item(90));
        queue.add(item(91));
        queue.skip_to_following();
        let expected = ids(&queue);
        let current = queue.get_current().copied();
        let json = serde_json::to_string(&queue.snapshot()).unwrap();
        let mut restored = Queue::new();
        assert!(restored.restore(serde_json::from_str(&json).unwrap()));
        assert_eq!(restored.get_current().copied(), current);
        assert_eq!(ids(&restored), expected);
        for id in expected {
            restored.skip_to_following();
            assert_eq!(restored.get_current().unwrap().item_id.id, id);
        }
        let mut invalid = queue.snapshot();
        invalid.positions[1] = invalid.positions[0];
        assert!(!restored.restore(invalid));
        let large = item(u128::MAX - 7);
        queue.fill(vec![large], 0);
        let json = serde_json::to_string(&queue.snapshot()).unwrap();
        assert!(restored.restore(serde_json::from_str(&json).unwrap()));
        assert_eq!(restored.get_current(), Some(&large));
    }
    #[test]
    fn preview_preserves_current_and_manual_fifo_order_with_duplicates() {
        let mut queue = Queue::new();
        queue.fill(vec![item(1), item(2), item(3)], 0);
        queue.add(item(9));
        queue.add(item(9));
        queue.add(item(8));
        assert_eq!(ids(&queue), [9, 9, 8, 2, 3]);
        assert_eq!(queue.get_current(), Some(&item(1)));
        for id in [9, 9, 8, 2, 3] {
            assert_eq!(
                queue.upcoming_ids().first().copied(),
                Some(item(id).item_id)
            );
            queue.skip_to_following();
            assert_eq!(queue.get_current(), Some(&item(id)));
        }
        assert!(ids(&queue).is_empty());
    }
    #[test]
    fn preview_matches_actual_shuffle_without_reshuffling() {
        let mut queue = Queue::new();
        queue.fill((1..9).map(item).collect(), 3);
        queue.set_behaviour(QueueBehavior::Random);
        let upcoming = ids(&queue);
        assert_eq!(queue.get_current(), Some(&item(4)));
        assert_eq!(upcoming, ids(&queue));
        for id in upcoming {
            queue.skip_to_following();
            assert_eq!(queue.get_current(), Some(&item(id)));
        }
    }
    #[test]
    fn changing_shuffle_keeps_current_track_and_preview_matches_new_order() {
        let mut queue = Queue::new();
        queue.fill((1..9).map(item).collect(), 3);
        queue.set_behaviour(QueueBehavior::Random);
        queue.skip_to_next();
        let current = *queue.get_current().unwrap();
        queue.set_behaviour(QueueBehavior::Random);
        assert_eq!(queue.get_current(), Some(&current));
        queue.set_behaviour(QueueBehavior::Sequential);
        assert_eq!(queue.get_current(), Some(&current));
    }
    #[test]
    fn repeat_modes_have_bounded_previews_and_empty_queue_is_safe() {
        let mut queue = Queue::new();
        queue.fill(vec![item(1), item(2), item(3)], 1);
        queue.set_behaviour(QueueBehavior::LoopAll);
        assert_eq!(ids(&queue), [3, 1, 2]);
        queue.set_behaviour(QueueBehavior::LoopTrack);
        assert_eq!(ids(&queue), [2]);
        queue.clear();
        queue.set_behaviour(QueueBehavior::LoopAll);
        queue.skip_to_next();
        assert!(ids(&queue).is_empty());
    }
    #[test]
    fn next_from_idle_starts_pending_tracks_safely() {
        let mut queue = Queue::new();
        queue.add(item(9));
        queue.add(item(8));
        assert_eq!(ids(&queue), [9, 8]);
        queue.skip_to_next();
        assert_eq!(queue.get_current(), Some(&item(9)));
        assert_eq!(ids(&queue), [8]);
        queue.skip_to_following();
        assert_eq!(queue.get_current(), Some(&item(8)));
        assert!(ids(&queue).is_empty());
    }

    #[test]
    fn replacing_or_clearing_queue_removes_old_manual_items() {
        let mut queue = Queue::new();
        queue.add(item(9));
        assert_eq!(ids(&queue), [9]);
        queue.fill(vec![item(1), item(2)], 0);
        assert_eq!(ids(&queue), [2]);
        queue.add(item(8));
        queue.clear();
        assert!(ids(&queue).is_empty());
    }
}
