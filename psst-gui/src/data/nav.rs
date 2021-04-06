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
