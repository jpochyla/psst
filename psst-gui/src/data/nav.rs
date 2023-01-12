use std::sync::Arc;

use druid::Data;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::data::{AlbumLink, ArtistLink, PlaylistLink, ShowLink};

use super::RecommendationsRequest;

#[derive(Copy, Clone, Debug, Data, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    SavedTracks,
    SavedAlbums,
    SavedShows,
    SearchResults,
    ArtistDetail,
    AlbumDetail,
    ShowDetail,
    PlaylistDetail,
    Recommendations,
}

#[derive(Clone, Debug, Data, PartialEq, Eq, Deserialize, Serialize)]
pub enum Nav {
    Home,
    SavedTracks,
    SavedAlbums,
    SavedShows,
    SearchResults(Arc<str>),
    ArtistDetail(ArtistLink),
    AlbumDetail(AlbumLink),
    ShowDetail(ShowLink),
    PlaylistDetail(PlaylistLink),
    Recommendations(Arc<RecommendationsRequest>),
}

impl Nav {
    pub fn route(&self) -> Route {
        match self {
            Nav::Home => Route::Home,
            Nav::SavedTracks => Route::SavedTracks,
            Nav::SavedAlbums => Route::SavedAlbums,
            Nav::SavedShows => Route::SavedShows,
            Nav::SearchResults(_) => Route::SearchResults,
            Nav::ArtistDetail(_) => Route::ArtistDetail,
            Nav::AlbumDetail(_) => Route::AlbumDetail,
            Nav::PlaylistDetail(_) => Route::PlaylistDetail,
            Nav::ShowDetail(_) => Route::ShowDetail,
            Nav::Recommendations(_) => Route::Recommendations,
        }
    }

    pub fn title(&self) -> String {
        match self {
            Nav::Home => "Home".to_string(),
            Nav::SavedTracks => "Saved Tracks".to_string(),
            Nav::SavedAlbums => "Saved Albums".to_string(),
            Nav::SavedShows => "Saved Podcasts".to_string(),
            Nav::SearchResults(query) => query.to_string(),
            Nav::AlbumDetail(link) => link.name.to_string(),
            Nav::ArtistDetail(link) => link.name.to_string(),
            Nav::PlaylistDetail(link) => link.name.to_string(),
            Nav::ShowDetail(link) => link.name.to_string(),
            Nav::Recommendations(_) => "Recommended".to_string(),
        }
    }

    pub fn full_title(&self) -> String {
        match self {
            Nav::Home => "Home".to_string(),
            Nav::SavedTracks => "Saved Tracks".to_string(),
            Nav::SavedAlbums => "Saved Albums".to_string(),
            Nav::SavedShows => "Saved Shows".to_string(),
            Nav::SearchResults(query) => format!("Search “{}”", query),
            Nav::AlbumDetail(link) => format!("Album “{}”", link.name),
            Nav::ArtistDetail(link) => format!("Artist “{}”", link.name),
            Nav::PlaylistDetail(link) => format!("Playlist “{}”", link.name),
            Nav::ShowDetail(link) => format!("Show “{}”", link.name),
            Nav::Recommendations(_) => "Recommended".to_string(),
        }
    }
}

#[derive(Clone, Debug, Data, Eq, PartialEq, Hash)]
pub enum SpotifyUrl {
    Playlist(Arc<str>),
    Artist(Arc<str>),
    Album(Arc<str>),
    Track(Arc<str>),
    Show(Arc<str>),
}

impl SpotifyUrl {
    pub fn parse(url: &str) -> Option<Self> {
        let url = Url::parse(url).ok()?;
        let mut segments = url.path_segments()?;
        let entity = segments.next()?;
        let id = segments.next()?;
        log::info!("url: {:?}", url);
        match entity {
            "playlist" => Some(Self::Playlist(id.into())),
            "artist" => Some(Self::Artist(id.into())),
            "album" => Some(Self::Album(id.into())),
            "track" => Some(Self::Track(id.into())),
            "show" => Some(Self::Show(id.into())),
            _ => None,
        }
    }

    pub fn id(&self) -> Arc<str> {
        match self {
            SpotifyUrl::Playlist(id) => id.clone(),
            SpotifyUrl::Artist(id) => id.clone(),
            SpotifyUrl::Album(id) => id.clone(),
            SpotifyUrl::Track(id) => id.clone(),
            SpotifyUrl::Show(id) => id.clone(),
        }
    }
}
