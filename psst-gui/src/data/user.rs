use std::sync::Arc;

use druid::{Data, Lens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Data, Lens, Deserialize)]
pub struct UserProfile {
    pub display_name: Arc<str>,
    pub email: Arc<str>,
    pub id: Arc<str>,
}

#[derive(Clone, Data, Lens, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct PublicUser {
    #[serde(default)]
    pub id: Arc<str>,
    #[serde(default)]
    pub display_name: Arc<str>,
    // Extended profile fields (optional, for detailed responses)
    #[serde(default)]
    pub uri: Option<Arc<str>>,
    #[serde(default)]
    pub name: Option<Arc<str>>,
    #[serde(default)]
    pub image_url: Option<Arc<str>>,
    #[serde(default)]
    pub followers_count: Option<i64>,
    #[serde(default)]
    pub is_following: Option<bool>,
    #[serde(default)]
    pub color: Option<i64>,
}

impl PublicUser {
    pub fn get_display_name(&self) -> Arc<str> {
        if self.display_name.is_empty() {
            self.name.clone().unwrap_or_default()
        } else {
            self.display_name.clone()
        }
    }
}
