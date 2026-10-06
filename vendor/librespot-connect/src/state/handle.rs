use crate::{
    core::{Error, dealer::protocol::SetQueueCommand},
    state::{
        ConnectState,
        context::{ContextType, ResetContext},
        metadata::Metadata,
    },
};
use protobuf::MessageField;

impl ConnectState {
    pub fn handle_shuffle(&mut self, shuffle: bool) -> Result<(), Error> {
        self.set_shuffle(shuffle);

        if shuffle {
            return self.shuffle_new();
        }

        self.reset_context(ResetContext::DefaultIndex);

        if self.current_track(MessageField::is_none) {
            return Ok(());
        }

        match self.current_track(|t| t.get_context_index()) {
            Some(current_index) => self.reset_playback_to_position(Some(current_index)),
            None => {
                let ctx = self.get_context(ContextType::Default)?;
                let current_index = ConnectState::find_index_in_context(ctx, |c| {
                    self.current_track(|t| c.uri == t.uri)
                })?;
                self.reset_playback_to_position(Some(current_index))
            }
        }
    }

    pub fn handle_set_queue(&mut self, set_queue: SetQueueCommand) {
        self.set_next_tracks(set_queue.next_tracks);
        self.set_prev_tracks(set_queue.prev_tracks);
        self.update_queue_revision();
    }

    pub fn handle_set_repeat_context(&mut self, repeat: bool) -> Result<(), Error> {
        self.set_repeat_context(repeat);

        if repeat {
            if let ContextType::Autoplay = self.fill_up_context {
                self.fill_up_context = ContextType::Default;
            }
        }

        let ctx = self.get_context(ContextType::Default)?;
        let current_track =
            ConnectState::find_index_in_context(ctx, |t| self.current_track(|t| &t.uri) == &t.uri)?;
        self.reset_playback_to_position(Some(current_track))
    }
}
