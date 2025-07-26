use rand::prelude::SliceRandom;

use super::PlaybackItem;

#[derive(Debug)]
pub enum ShuffleBehavior {
    Sequential,
    Random,
}

#[derive(Debug)]
pub enum LoopBehavior {
    Track,
    All,
    None,
}

impl Default for ShuffleBehavior {
    fn default() -> Self {
        Self::Sequential
    }
}

impl Default for LoopBehavior {
    fn default() -> Self {
        Self::None
    }
}

pub struct Queue {
    items: Vec<PlaybackItem>,
    user_items: Vec<PlaybackItem>,
    position: usize,
    user_items_position: usize,
    positions: Vec<usize>,
    shuffle_behavior: ShuffleBehavior,
    loop_behavior: LoopBehavior,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            user_items: Vec::new(),
            position: 0,
            user_items_position: 0,
            positions: Vec::new(),
            shuffle_behavior: ShuffleBehavior::default(),
            loop_behavior: LoopBehavior::default(),
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.positions.clear();
        self.position = 0;
    }

    pub fn fill(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.positions.clear();
        self.items = items;
        self.position = position;
        self.compute_positions();
    }

    pub fn add(&mut self, item: PlaybackItem) {
        self.user_items.push(item);
    }

    fn handle_added_queue(&mut self) {
        if self.user_items.len() > self.user_items_position {
            self.items.insert(
                self.positions.len(),
                self.user_items[self.user_items_position],
            );
            self.positions
                .insert(self.position + 1, self.positions.len());
            self.user_items_position += 1;
        }
    }

    pub fn set_shuffle_behaviour(&mut self, shuffle_behavior: ShuffleBehavior) {
        self.shuffle_behavior = shuffle_behavior;
        self.compute_positions();
    }

    pub fn set_loop_behaviour(&mut self, loop_behavior: LoopBehavior) {
        self.loop_behavior = loop_behavior;
        self.compute_positions();
    }

    fn compute_positions(&mut self) {
        // In the case of switching away from shuffle, the position should be set back to
        // where it appears in the actual playlist order.
        let playlist_position = if self.positions.len() > 1 {
            self.positions[self.position]
        } else {
            self.position
        };
        // Start with an ordered 1:1 mapping.
        self.positions = (0..self.items.len()).collect();

        if let ShuffleBehavior::Random = self.shuffle_behavior {
            // Swap the current position with the first item, so we will start from the
            // beginning, with the full queue ahead of us.  Then shuffle the rest of the
            // items and set the position to 0.
            if self.positions.len() > 1 {
                self.positions.swap(0, self.position);
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
        self.handle_added_queue();
        self.position = self.next_position();
    }

    pub fn skip_to_following(&mut self) {
        self.handle_added_queue();
        self.position = self.following_position();
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
        self.position.saturating_sub(1)
    }

    fn next_position(&self) -> usize {
        match self.loop_behavior {
            LoopBehavior::Track | LoopBehavior::None => self.position + 1,
            LoopBehavior::All => (self.position + 1) % self.items.len(),
        }
    }

    fn following_position(&self) -> usize {
        match self.loop_behavior {
            LoopBehavior::None => self.position + 1,
            LoopBehavior::Track => self.position,
            LoopBehavior::All => (self.position + 1) % self.items.len(),
        }
    }
}
