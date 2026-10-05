use rand::prelude::SliceRandom;

use super::PlaybackItem;

#[derive(Default, Debug, Clone)]
pub enum QueueBehavior {
    #[default]
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

#[derive(Clone)]
pub struct Queue {
    items: Vec<PlaybackItem>,
    user_items: Vec<PlaybackItem>,
    position: usize,
    user_items_position: usize,
    positions: Vec<usize>,
    behavior: QueueBehavior,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            user_items: Vec::new(),
            position: 0,
            user_items_position: 0,
            positions: Vec::new(),
            behavior: QueueBehavior::default(),
        }
    }

    pub fn clear(&mut self) {
        self.user_items.clear();
        self.user_items_position = 0;
        self.items.clear();
        self.positions.clear();
        self.position = 0;
    }

    pub fn fill(&mut self, items: Vec<PlaybackItem>, position: usize) {
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
            preview.skip_to_following();
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
            if self.positions.len() > 1 {
                let current = playlist_position.min(self.positions.len() - 1);
                self.positions.swap(0, current);
                self.positions[1..].shuffle(&mut rand::rng());
            }
            self.position = 0;
        } else {
            self.position = playlist_position;
        }
    }

    pub fn skip_to_previous(&mut self) {
        self.position = self.previous_position();
    }

    pub fn skip_to_next(&mut self) {
        let had_current = self.get_current().is_some();
        let inserted = self.handle_added_queue();
        self.position = if !had_current {
            inserted.unwrap_or_else(|| self.next_position())
        } else {
            self.next_position()
        };
    }

    pub fn skip_to_following(&mut self) {
        let had_current = self.get_current().is_some();
        let inserted = self.handle_added_queue();
        self.position = if !had_current {
            inserted.unwrap_or_else(|| self.following_position())
        } else {
            self.following_position()
        };
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
