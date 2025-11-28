use std::sync::Arc;

use druid::{im::Vector, Data, Lens};
use serde::Deserialize;

use crate::data::{Cached, Promise, PublicUser};

#[derive(Clone, Data, Lens)]
pub struct PublicUserDetail {
    pub info: Promise<Cached<Arc<PublicUserInformation>>, PublicUser>,
}

// Specialized structures for public user profile API response
#[derive(Clone, Data, Lens, Deserialize)]
pub struct PublicUserArtist {
    #[serde(default)]
    pub followers_count: i64,
    pub image_url: String,
    #[serde(default)]
    pub is_following: bool,
    pub name: String,
    pub uri: String,
}

#[derive(Clone, Data, Lens, Deserialize)]
pub struct PublicUserPlaylist {
    pub image_url: String,
    pub name: String,
    pub owner_name: String,
    pub owner_uri: String,
    pub uri: String,
    #[serde(default)]
    pub followers_count: Option<i64>,
    #[serde(default)]
    pub is_following: Option<bool>,
}

#[derive(Clone, Data, Lens, Deserialize)]
pub struct PublicUserInformation {
    #[serde(default)]
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub followers_count: i64,
    #[serde(default)]
    pub following_count: i64,
    #[serde(default)]
    pub is_following: Option<bool>,
    #[serde(default)]
    pub is_current_user: Option<bool>,
    #[serde(default)]
    pub recently_played_artists: Vector<PublicUserArtist>,
    #[serde(default)]
    pub public_playlists: Vector<PublicUserPlaylist>,
    #[serde(default)]
    pub total_public_playlists_count: i64,
    #[serde(default)]
    pub has_spotify_name: bool,
    #[serde(default)]
    pub has_spotify_image: bool,
    #[serde(default)]
    pub color: i64,
    #[serde(default)]
    pub allow_follows: bool,
    #[serde(default)]
    pub show_follows: bool,
}
