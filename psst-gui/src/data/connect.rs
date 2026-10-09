use super::{Promise, Track};
use druid::{im::Vector, Data, Lens};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct Device {
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub is_active: bool,
    pub is_restricted: bool,
    pub volume_percent: Option<u32>,
}

#[derive(Clone, Data, Lens, Default)]
pub struct ConnectState {
    pub devices: Promise<Vector<Device>>,
    pub selected: Option<Device>,
    pub epoch: u64,
    pub busy: bool,
    pub polling: bool,
    pub pending_start: bool,
    pub local_volume: Option<f64>,
    pub status: String,
    pub native_status: String,
    pub native_ready: bool,
}

#[derive(Clone, Data)]
pub struct RemoteRequest {
    pub device_id: String,
    pub epoch: u64,
    pub kind: String,
    pub body: String,
    pub value: String,
}

#[derive(Clone, Data, Deserialize)]
pub struct RemotePlayback {
    pub device: Device,
    pub is_playing: bool,
    pub progress_ms: Option<u64>,
    pub item: Option<Arc<Track>>,
}
