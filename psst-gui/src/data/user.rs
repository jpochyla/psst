use std::sync::Arc;

use crate::data::{Album, Image, Promise, Track};
use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Data, Lens, Deserialize)]
pub struct UserProfile {
    pub display_name: Arc<str>,
    pub email: Arc<str>,
    pub id: Arc<str>,
}

#[derive(Clone, Data, Lens, Deserialize, Debug)]
pub struct PublicUser {
    pub display_name: Arc<str>,
    pub id: Arc<str>,
}

#[derive(Default, Debug, Clone, Lens, PartialEq, Deserialize, Data)]
pub struct PublicUserDetail {
    pub uri: String,
    pub name: String,
    pub image_url: String,
    pub followers_count: i64,
    pub following_count: i64,
    pub is_following: bool,
    pub recently_played_artists: Vec<RecentlyPlayedArtist>,
    pub public_playlists: Vec<PublicPlaylist>,
    pub total_public_playlists_count: i64,
    pub allow_follows: bool,
    pub show_follows: bool,
}

#[derive(Default, Debug, Clone, Lens, PartialEq, Deserialize, Data)]
pub struct RecentlyPlayedArtist {
    pub uri: String,
    pub name: String,
    pub image_url: String,
    pub followers_count: i64,
    pub is_following: Option<bool>,
}

#[derive(Default, Debug, Clone, Lens, PartialEq, Deserialize)]
pub struct PublicPlaylist {
    pub uri: String,
    pub name: String,
    pub image_url: String,
    pub owner_name: String,
    pub owner_uri: String,
    pub followers_count: Option<i64>,
    pub is_following: Option<bool>,
}

#[derive(Clone, Data, Lens)]
pub struct UserDetail {
    pub artist: Promise<PublicUser, UserLink>,
    pub albums: Promise<UserAlbums, UserLink>,
    pub top_tracks: Promise<UserTracks, UserLink>,
    pub user_info: Promise<UserInfo, UserLink>,
}

impl PublicUser {
    pub fn link(&self) -> UserLink {
        UserLink {
            id: self.id.clone(),
            name: self.display_name.clone(),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct UserAlbums {
    pub albums: Vector<Arc<Album>>,
}
#[derive(Clone, Data, Lens)]
pub struct UserInfo {
    pub main_image: Arc<str>,
    pub stats: UserStats,
}

#[derive(Clone, Data, Lens)]
pub struct UserStats {
    pub followers: i64,
    pub following: i64,
}

#[derive(Clone, Data, Lens)]
pub struct UserTracks {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub tracks: Vector<Arc<Track>>,
}

impl UserTracks {
    pub fn link(&self) -> UserLink {
        UserLink {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Data, Lens, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct UserLink {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

impl UserLink {
    pub fn url(&self) -> String {
        format!("https://open.spotify.com/users/{id}", id = self.id)
    }
}
