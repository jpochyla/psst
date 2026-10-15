use std::time::{Duration, Instant};

pub(super) const TAP_INTERVAL: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MediaPress {
    Play,
    Pause,
    Toggle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GestureAction {
    Single(MediaPress),
    Next,
    Previous,
}

/// Resolve hardware taps before sending transport commands, so a triple tap
/// never briefly pauses playback or advances to the next song.
#[derive(Default)]
pub(super) struct MediaGesture {
    first: Option<MediaPress>,
    count: u8,
    deadline: Option<Instant>,
}

impl MediaGesture {
    pub fn press(&mut self, press: MediaPress, now: Instant) -> Option<GestureAction> {
        let expired = self.finish(now);
        self.first.get_or_insert(press);
        self.count += 1;
        if self.count == 3 {
            self.cancel();
            return Some(GestureAction::Previous);
        }
        self.deadline = Some(now + TAP_INTERVAL);
        expired
    }

    pub fn finish(&mut self, now: Instant) -> Option<GestureAction> {
        if self.deadline.is_none_or(|deadline| now < deadline) {
            return None;
        }
        let action = match self.count {
            1 => GestureAction::Single(self.first?),
            2 => GestureAction::Next,
            _ => return None,
        };
        self.cancel();
        Some(action)
    }

    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    pub fn cancel(&mut self) {
        self.first = None;
        self.count = 0;
        self.deadline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_press_preserves_windows_play_pause_and_toggle_commands() {
        for press in [MediaPress::Play, MediaPress::Pause, MediaPress::Toggle] {
            let mut gesture = MediaGesture::default();
            let now = Instant::now();
            assert_eq!(gesture.press(press, now), None);
            assert_eq!(
                gesture.finish(now + TAP_INTERVAL - Duration::from_millis(1)),
                None
            );
            assert_eq!(
                gesture.finish(now + TAP_INTERVAL),
                Some(GestureAction::Single(press))
            );
            assert_eq!(gesture.finish(now + TAP_INTERVAL), None);
        }
    }

    #[test]
    fn double_tap_waits_for_the_third_tap_then_advances_exactly_once() {
        let mut gesture = MediaGesture::default();
        let now = Instant::now();
        assert_eq!(gesture.press(MediaPress::Pause, now), None);
        let second = now + Duration::from_millis(250);
        assert_eq!(gesture.press(MediaPress::Play, second), None);
        assert_eq!(gesture.finish(now + TAP_INTERVAL), None);
        assert_eq!(
            gesture.finish(second + TAP_INTERVAL),
            Some(GestureAction::Next)
        );
        assert_eq!(gesture.finish(second + TAP_INTERVAL), None);
    }

    #[test]
    fn triple_tap_goes_back_without_first_executing_pause_or_next() {
        let mut gesture = MediaGesture::default();
        let now = Instant::now();
        assert_eq!(gesture.press(MediaPress::Toggle, now), None);
        assert_eq!(
            gesture.press(MediaPress::Toggle, now + Duration::from_millis(200)),
            None
        );
        assert_eq!(
            gesture.press(MediaPress::Toggle, now + Duration::from_millis(400)),
            Some(GestureAction::Previous)
        );
        assert_eq!(gesture.finish(now + Duration::from_secs(1)), None);
        assert_eq!(gesture.remaining(now), None);
    }

    #[test]
    fn late_timer_does_not_merge_separate_gestures() {
        let mut gesture = MediaGesture::default();
        let now = Instant::now();
        gesture.press(MediaPress::Pause, now);
        let later = now + TAP_INTERVAL;
        assert_eq!(
            gesture.press(MediaPress::Play, later),
            Some(GestureAction::Single(MediaPress::Pause))
        );
        assert_eq!(
            gesture.finish(later + TAP_INTERVAL),
            Some(GestureAction::Single(MediaPress::Play))
        );
    }

    #[test]
    fn explicit_transport_command_cancels_the_pending_gesture() {
        let mut gesture = MediaGesture::default();
        let now = Instant::now();
        gesture.press(MediaPress::Toggle, now);
        gesture.cancel();
        assert_eq!(gesture.finish(now + TAP_INTERVAL), None);
        assert_eq!(gesture.press(MediaPress::Pause, now + TAP_INTERVAL), None);
        assert_eq!(
            gesture.finish(now + TAP_INTERVAL * 2),
            Some(GestureAction::Single(MediaPress::Pause))
        );
    }
}
