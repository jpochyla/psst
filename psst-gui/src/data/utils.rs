use druid::Data;
use std::{ops::Deref, sync::Arc, time::Duration};

#[derive(Clone, Debug, Data)]
pub struct Image {
    pub url: Arc<str>,
    pub width: Option<usize>,
    pub height: Option<usize>,
}

impl Image {
    pub fn fits(&self, width: f64, height: f64) -> bool {
        if let (Some(w), Some(h)) = (self.width, self.height) {
            (w as f64) < width && (h as f64) < height
        } else {
            true // Unknown dimensions, treat as fitting.
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct AudioDuration(Duration);

impl Data for AudioDuration {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Deref for AudioDuration {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Duration> for AudioDuration {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl AudioDuration {
    pub fn as_minutes_and_seconds(&self) -> String {
        let minutes = self.as_secs() / 60;
        let seconds = self.as_secs() % 60;
        format!("{}:{:02}", minutes, seconds)
    }
}
