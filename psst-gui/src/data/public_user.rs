use std::sync::Arc;

use druid::{im::Vector, Data, Lens};

use crate::data::{Cached, MixedView, Promise, PublicUser};

#[derive(Clone, Data, Lens)]
pub struct PublicUserDetail {
    pub info: Promise<Cached<Arc<PublicUserInformation>>, PublicUser>,
}

#[derive(Clone, Data, Lens)]
pub struct PublicUserInformation {
    pub uri: String,
    pub name: String,
    pub image_url: Option<String>,
    pub followers_count: i64,
    pub following_count: i64,
    pub is_following: Option<bool>,
    pub is_current_user: Option<bool>,
    pub recently_played_artists: MixedView,
    pub public_playlists: MixedView,
    pub allow_follows: bool,
    pub followers: Vector<PublicUser>,
    pub following: Vector<PublicUser>,
}
