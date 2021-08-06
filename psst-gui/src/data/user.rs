use std::sync::Arc;

use druid::{Data, Lens};
use serde::Deserialize;

#[derive(Clone, Data, Lens, Deserialize)]
pub struct UserProfile {
    pub display_name: Arc<str>,
    pub email: Arc<str>,
}
