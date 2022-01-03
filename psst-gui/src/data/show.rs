use std::sync::Arc;

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Serialize};

use crate::data::{Album, Cached, Image, Promise, Track};

// #[derive(Clone, Data, Lens)]
// pub struct ArtistDetail {
//     pub artist: Promise<Artist, ArtistLink>,
//     pub albums: Promise<ArtistAlbums, ArtistLink>,
//     pub top_tracks: Promise<ArtistTracks, ArtistLink>,
//     pub related_artists: Promise<Cached<Vector<Artist>>, ArtistLink>,
// }

#[derive(Clone, Data, Lens, Deserialize)]
pub struct Show {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
    pub publisher: Arc<str>
}

impl Show {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        Image::at_least_of_size(&self.images, width, height)
    }

    pub fn link(&self) -> ShowLink {
        ShowLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

// #[derive(Clone, Data, Lens)]
// pub struct ArtistAlbums {
//     pub albums: Vector<Arc<Album>>,
//     pub singles: Vector<Arc<Album>>,
//     pub compilations: Vector<Arc<Album>>,
//     pub appears_on: Vector<Arc<Album>>,
// }

#[derive(Clone, Data, Lens)]
pub struct ShowEpisodes {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub episodes: Vector<Arc<Track>>,
}

impl ShowEpisodes {
    pub fn link(&self) -> ShowLink {
        ShowLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Data, Lens, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ShowLink {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

impl ShowLink {
    pub fn url(&self) -> String {
        format!("https://open.spotify.com/show/{id}", id = self.id)
    }
}
