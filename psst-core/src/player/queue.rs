use super::PlaybackItem;
use rand::prelude::SliceRandom;

#[derive(Debug)]
pub enum QueueBehavior {
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

impl Default for QueueBehavior {
    fn default() -> Self {
        Self::Sequential
    }
}

pub struct Queue {
    items: Vec<PlaybackItem>,
    position: usize,
    positions: Vec<usize>,
    behavior: QueueBehavior,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            position: 0,
            positions: Vec::new(),
            behavior: QueueBehavior::default(),
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.positions.clear();
        self.position = 0;
    }

    pub fn fill(&mut self, items: &Vec<PlaybackItem>, position: usize) {
        let items = items.to_vec();
        self.items = items;
        self.positions = (0..items.len()).collect();
        self.position = position;
    }

    pub fn set_behaviour(&mut self, behavior: QueueBehavior) {
        self.behavior = behavior;
        self.positions = self.compute_positions();
    }

    fn compute_positions(&self) -> Vec<usize> {
        let mut positions = vec![0; self.items.len()];

        let playlist_position = if self.positions.len() > 1 {
            self.positions[self.position]
        } else {
            self.position
        };

        if let QueueBehavior::Random = self.behavior {
            if positions.len() > 1 {
                positions.swap(0, self.position);
                positions[1..].shuffle(&mut rand::thread_rng());
            }
            positions[0] = playlist_position;
        } else {
            for i in 0..positions.len() {
                positions[i] = i;
            }
            positions[playlist_position] = self.position;
        }

        positions
    }

    pub fn skip_to_previous(&mut self) {
        self.position = self.previous_position();
    }

    pub fn skip_to_next(&mut self) {
        self.position = self.next_position();
    }

    pub fn skip_to_following(&mut self) {
        self.position = self.following_position();
    }

    pub fn get_current(&self) -> Option<&PlaybackItem> {
        let position = self.positions.get(self.position).copied()?;
        self.items.get(position)
    }

    pub fn get_following(&self) -> Option<&PlaybackItem> {
        let position = self.positions.get(self.following_position()).copied()?;
        self.items.get(position)
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
            QueueBehavior::LoopAll => (self.position + 1) % self.items.len(),
        }
    }

    fn following_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential | QueueBehavior::Random => self.position + 1,
            QueueBehavior::LoopTrack => self.position,
            QueueBehavior::LoopAll => (self.position + 1) % self.items.len(),
        }
    }
}
