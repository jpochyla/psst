use crate::data::{Album, Image, Promise, Track};
use druid::{im::Vector, Data, Lens};
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens)]
pub struct ArtistDetail {
    pub artist: Promise<Artist, ArtistLink>,
    pub albums: Promise<ArtistAlbums, ArtistLink>,
    pub top_tracks: Promise<ArtistTracks, ArtistLink>,
    pub related_artists: Promise<Vector<Artist>, ArtistLink>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Artist {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub images: Vector<Image>,
}

impl Artist {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        self.images
            .iter()
            .rev()
            .find(|img| !img.fits(width, height))
            .or_else(|| self.images.back())
    }

    pub fn link(&self) -> ArtistLink {
        ArtistLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Data, Lens)]
pub struct ArtistAlbums {
    pub albums: Vector<Album>,
    pub singles: Vector<Album>,
    pub compilations: Vector<Album>,
}

#[derive(Clone, Debug, Data, Lens)]
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

#[derive(Clone, Debug, Data, Lens, Eq, PartialEq, Hash)]
pub struct ArtistLink {
    pub id: Arc<str>,
    pub name: Arc<str>,
}
