use crate::data::{AlbumLink, ArtistLink, PlaylistLink};
use druid::Data;

#[derive(Clone, Debug, Data, Eq, PartialEq, Hash)]
pub enum Nav {
    Home,
    SavedTracks,
    SavedAlbums,
    SearchResults(String),
    ArtistDetail(ArtistLink),
    AlbumDetail(AlbumLink),
    PlaylistDetail(PlaylistLink),
}

impl Nav {
    pub fn to_title(&self) -> String {
        match self {
            Nav::Home => "Home".to_string(),
            Nav::SavedTracks => "Saved Tracks".to_string(),
            Nav::SavedAlbums => "Saved Albums".to_string(),
            Nav::SearchResults(query) => query.to_owned(),
            Nav::AlbumDetail(link) => link.name.to_string(),
            Nav::ArtistDetail(link) => link.name.to_string(),
            Nav::PlaylistDetail(link) => link.name.to_string(),
        }
    }

    pub fn to_full_title(&self) -> String {
        match self {
            Nav::Home => "Home".to_string(),
            Nav::SavedTracks => "Saved Tracks".to_string(),
            Nav::SavedAlbums => "Saved Albums".to_string(),
            Nav::SearchResults(query) => format!("Search “{}”", query),
            Nav::AlbumDetail(link) => format!("Album “{}”", link.name),
            Nav::ArtistDetail(link) => format!("Artist “{}”", link.name),
            Nav::PlaylistDetail(link) => format!("Playlist “{}”", link.name),
        }
    }
}
