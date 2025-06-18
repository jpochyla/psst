use std::sync::Arc;

use crate::data::{Album, Cached, Image, Promise, Track};
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
   // pub images: Vector<Image>, // Leads to error 
}

#[derive(Clone, Data, Lens)]
pub struct UserDetail {
    pub artist: Promise<PublicUser, UserLink>,
    pub albums: Promise<UserAlbums, UserLink>,
}

impl PublicUser {
  //  pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
  //      Image::at_least_of_size(&self.images, width, height)
  //  }

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
        format!("https://open.spotify.com/user/{id}", id = self.id)
    }
}
