use std::sync::Arc;

use druid::{im::Vector, widget::ListIter, Data, Lens};
use serde::{Deserialize, Serialize};

use crate::data::{Album, Cached, Image, Promise};

#[derive(Clone, Data, Lens)]
pub struct ArtistDetail {
    pub artist: Promise<Artist, ArtistLink>,
    pub albums: Promise<ArtistAlbums, ArtistLink>,
    pub overview: Promise<Cached<ArtistOverview>, ArtistLink>,
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

/// One section of an artist's catalogue, as shown on the artist page:
/// a header (`title`) above its releases.
#[derive(Clone, Data, Lens)]
pub struct ArtistAlbumsSection {
    pub title: Arc<str>,
    pub albums: Vector<Arc<Album>>,
}

impl ArtistAlbums {
    /// The catalogue sections that hold at least one release, in the order
    /// they are shown on the artist page.  Sections with no releases are
    /// left out, so the page does not render a header for them.
    pub fn sections(&self) -> Vector<ArtistAlbumsSection> {
        [
            ("Albums", &self.albums),
            ("Singles", &self.singles),
            ("Compilations", &self.compilations),
            ("Appears On", &self.appears_on),
        ]
        .into_iter()
        .filter(|(_, albums)| !albums.is_empty())
        .map(|(title, albums)| ArtistAlbumsSection {
            title: title.into(),
            albums: albums.clone(),
        })
        .collect()
    }
}

/// Iterates over the non-empty catalogue sections only, so widgets built
/// on `List` (like the artist page) never see an empty section.
impl ListIter<ArtistAlbumsSection> for ArtistAlbums {
    fn for_each(&self, mut cb: impl FnMut(&ArtistAlbumsSection, usize)) {
        for (index, section) in self.sections().iter().enumerate() {
            cb(section, index);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut ArtistAlbumsSection, usize)) {
        // Sections are a filtered view over the four fields above; mutate a
        // clone of each non-empty field and write it back afterwards.  The
        // field order and titles must match `sections()`.
        let mut index = 0;
        for (title, albums) in [
            ("Albums", &mut self.albums),
            ("Singles", &mut self.singles),
            ("Compilations", &mut self.compilations),
            ("Appears On", &mut self.appears_on),
        ] {
            if albums.is_empty() {
                continue;
            }
            let mut section = ArtistAlbumsSection {
                title: title.into(),
                albums: albums.clone(),
            };
            cb(&mut section, index);
            index += 1;
            *albums = section.albums;
        }
    }

    fn data_len(&self) -> usize {
        self.sections().len()
    }
}

/// Everything the artist page reads out of a single `queryArtistOverview`
/// response: the info panel and the related artists shown below the albums.
#[derive(Clone, Data, Lens)]
pub struct ArtistOverview {
    pub info: ArtistInfo,
    pub related: Vector<Artist>,
}

#[derive(Clone, Data, Lens)]
pub struct ArtistInfo {
    pub main_image: Arc<str>,
    pub stats: ArtistStats,
    pub bio: String,
    pub artist_links: Vector<String>,
}

#[derive(Clone, Data, Lens)]
pub struct ArtistStats {
    pub followers: i64,
    pub monthly_listeners: i64,
    pub world_rank: i64,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn album(name: &str) -> Arc<Album> {
        Arc::new(Album {
            id: name.into(),
            name: name.into(),
            album_type: Default::default(),
            images: Vector::new(),
            artists: Vector::new(),
            copyrights: Vector::new(),
            label: "".into(),
            tracks: Vector::new(),
            release_date: None,
            release_date_precision: None,
        })
    }

    fn catalogue(
        albums: Vector<Arc<Album>>,
        singles: Vector<Arc<Album>>,
        compilations: Vector<Arc<Album>>,
        appears_on: Vector<Arc<Album>>,
    ) -> ArtistAlbums {
        ArtistAlbums {
            albums,
            singles,
            compilations,
            appears_on,
        }
    }

    fn section_titles(catalogue: &ArtistAlbums) -> Vec<String> {
        catalogue
            .sections()
            .iter()
            .map(|section| section.title.to_string())
            .collect()
    }

    #[test]
    fn sections_skip_empty_sections() {
        let catalogue = catalogue(
            Vector::from_iter([album("album")]),
            Vector::new(),
            Vector::new(),
            Vector::from_iter([album("guest spot")]),
        );
        assert_eq!(section_titles(&catalogue), ["Albums", "Appears On"]);
        assert_eq!(catalogue.data_len(), 2);
    }

    #[test]
    fn sections_are_empty_for_artist_without_releases() {
        let catalogue = catalogue(Vector::new(), Vector::new(), Vector::new(), Vector::new());
        assert!(catalogue.sections().is_empty());
        assert_eq!(catalogue.data_len(), 0);
    }

    #[test]
    fn sections_keep_all_sections_in_display_order() {
        let catalogue = catalogue(
            Vector::from_iter([album("album")]),
            Vector::from_iter([album("single")]),
            Vector::from_iter([album("compilation")]),
            Vector::from_iter([album("guest spot")]),
        );
        assert_eq!(
            section_titles(&catalogue),
            ["Albums", "Singles", "Compilations", "Appears On"]
        );
        assert_eq!(catalogue.data_len(), 4);
    }

    #[test]
    fn sections_carry_the_releases_of_their_field() {
        let catalogue = catalogue(
            Vector::new(),
            Vector::from_iter([album("single")]),
            Vector::new(),
            Vector::new(),
        );
        let sections = catalogue.sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title.as_ref(), "Singles");
        assert_eq!(sections[0].albums.len(), 1);
        assert_eq!(sections[0].albums[0].name.as_ref(), "single");
    }
}
