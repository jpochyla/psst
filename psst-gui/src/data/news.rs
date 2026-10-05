use super::{Album, Promise};
use druid::{im::Vector, Data, Lens};
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
pub struct Release {
    pub album: Arc<Album>,
    pub artist: String,
    pub date: String,
    pub unread: bool,
}

#[derive(Clone, Data, Lens)]
pub struct NewsFeed {
    pub releases: Vector<Release>,
    pub followed_count: usize,
    pub failed_count: usize,
    pub notice: String,
}

#[derive(Clone, Data, Lens, Default)]
pub struct NewsState {
    pub feed: Promise<NewsFeed>,
}
