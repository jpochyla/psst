use std::sync::Arc;

use druid::{im::Vector, Data, Lens};

use crate::data::{Album, Artist, Playlist, Promise, Track};

#[derive(Clone, Data, Lens)]
pub struct Search {
    pub input: String,
    pub results: Promise<SearchResults, Arc<str>>,
}

#[derive(Copy, Clone, Data, Eq, PartialEq)]
pub enum SearchTopic {
    Artist,
    Album,
    Track,
    Playlist,
}

impl SearchTopic {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchTopic::Artist => "artist",
            SearchTopic::Album => "album",
            SearchTopic::Track => "track",
            SearchTopic::Playlist => "playlist",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Artist, Self::Album, Self::Track, Self::Playlist]
    }
}

#[derive(Clone, Data, Lens)]
pub struct SearchResults {
    pub query: Arc<str>,
    pub artists: Vector<Artist>,
    pub albums: Vector<Arc<Album>>,
    pub tracks: Vector<Arc<Track>>,
    pub playlists: Vector<Playlist>,
}
