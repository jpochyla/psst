use std::sync::Arc;

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Serialize};

use crate::data::{Album, Cached, Image, Promise, Track};

#[derive(Clone, Data, Lens)]
pub struct ArtistDetail {
    pub artist: Promise<Artist, ArtistLink>,
    pub albums: Promise<ArtistAlbums, ArtistLink>,
    pub top_tracks: Promise<ArtistTracks, ArtistLink>,
    pub related_artists: Promise<Cached<Vector<Artist>>, ArtistLink>,
}

#[derive(Clone, Data, Lens, Deserialize)]
pub struct Artist {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
}

impl Artist {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        Image::at_least_of_size(&self.images, width, height)
    }

    pub fn link(&self) -> ArtistLink {
        ArtistLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct ArtistAlbums {
    pub albums: Vector<Arc<Album>>,
    pub singles: Vector<Arc<Album>>,
    pub compilations: Vector<Arc<Album>>,
    pub appears_on: Vector<Arc<Album>>,
}

#[derive(Clone, Data, Lens)]
pub struct ArtistTracks {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub tracks: Vector<Arc<Track>>,
}

impl ArtistTracks {
    pub fn link(&self) -> ArtistLink {
        ArtistLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Data, Lens, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ArtistLink {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

impl ArtistLink {
    pub fn url(&self) -> String {
        format!("https://open.spotify.com/artist/{id}", id = self.id)
    }
}
