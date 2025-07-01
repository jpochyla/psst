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
    #[serde(skip)]
    pub images: Vector<Image>,
}

#[derive(Clone, Data, Lens)]
pub struct UserDetail {
    pub artist: Promise<PublicUser, UserLink>,
    pub albums: Promise<UserAlbums, UserLink>,
    pub top_tracks: Promise<UserTracks, UserLink>,
    pub user_info: Promise<UserInfo, UserLink>,
}

impl PublicUser {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        Image::at_least_of_size(&self.images, width, height)
    }

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
