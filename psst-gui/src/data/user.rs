use std::sync::Arc;

use druid::{Data, Lens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Data, Lens, Deserialize)]
pub struct UserProfile {
    #[serde(alias = "name")]
    pub display_name: Arc<str>,
    pub email: Arc<str>,
    pub id: Arc<str>,
}

#[derive(Clone, Data, Lens, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct PublicUser {
    #[serde(default)]
    pub id: Arc<str>,
    #[serde(default, alias = "name")]
    pub display_name: Arc<str>,
    // Extended profile fields (optional, for detailed responses)
    #[serde(default)]
    pub uri: Option<Arc<str>>,
    #[serde(default)]
    pub image_url: Option<Arc<str>>,
    #[serde(default)]
    pub followers_count: Option<i64>,
    #[serde(default)]
    pub is_following: Option<bool>,
}

impl PublicUser {
    /// Get the user ID, extracting from URI if `id` is empty.
    /// URI format is "spotify:user:abc123", extracts "abc123".
    pub fn get_id(&self) -> Arc<str> {
        if self.id.is_empty() {
            self.uri
                .as_ref()
                .and_then(|uri| uri.split(':').nth(2).map(Arc::from))
                .unwrap_or_default()
        } else {
            self.id.clone()
        }
    }
}
