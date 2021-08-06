use std::sync::Arc;

use druid::{im::Vector, Data, Lens};

use crate::data::{Album, Artist, Playlist, Promise, Track};

#[derive(Clone, Data, Lens)]
pub struct Search {
    pub input: String,
    pub results: Promise<SearchResults, Arc<str>>,
}

#[derive(Clone, Data, Lens)]
pub struct SearchResults {
    pub query: Arc<str>,
    pub artists: Vector<Artist>,
    pub albums: Vector<Arc<Album>>,
    pub tracks: Vector<Arc<Track>>,
    pub playlists: Vector<Playlist>,
}
