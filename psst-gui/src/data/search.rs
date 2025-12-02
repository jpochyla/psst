use std::sync::Arc;

use druid::{im::Vector, Data, Lens};

use crate::data::{Album, Artist, Playlist, Promise, Show, Track};

#[derive(Clone, Data, Lens)]
pub struct Search {
    pub input: String,
    pub topic: Option<SearchTopic>,
    pub results: Promise<SearchResults, (Arc<str>, Option<SearchTopic>)>,
}

#[derive(Copy, Clone, Data, Eq, PartialEq)]
pub enum SearchTopic {
    Artist,
    Album,
    Track,
    Playlist,
    Show,
}

impl SearchTopic {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchTopic::Artist => "artist",
            SearchTopic::Album => "album",
            SearchTopic::Track => "track",
            SearchTopic::Playlist => "playlist",
            SearchTopic::Show => "show",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SearchTopic::Artist => "Artists",
            SearchTopic::Album => "Albums",
            SearchTopic::Track => "Tracks",
            SearchTopic::Playlist => "Playlists",
            SearchTopic::Show => "Podcasts",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Artist,
            Self::Album,
            Self::Track,
            Self::Playlist,
            Self::Show,
        ]
    }
}

#[derive(Clone, Data, Lens)]
pub struct SearchResults {
    pub query: Arc<str>,
    pub topic: Option<SearchTopic>,
    pub artists: Vector<Artist>,
    pub albums: Vector<Arc<Album>>,
    pub tracks: Vector<Arc<Track>>,
    pub playlists: Vector<Playlist>,
    pub shows: Vector<Arc<Show>>,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.artists.is_empty()
            && self.albums.is_empty()
            && self.tracks.is_empty()
            && self.playlists.is_empty()
            && self.shows.is_empty()
    }
}
