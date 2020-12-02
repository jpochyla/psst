use druid::{Data, Lens};
use platform_dirs::AppDirs;
use psst_core::{
    audio_player::PlaybackConfig, cache::mkdir_if_not_exists, connection::Credentials,
};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};

const APP_NAME: &str = "Psst";
const CONFIG_FILENAME: &str = "config.json";

#[derive(Clone, Debug, Default, Data, Lens, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub audio_quality: AudioQuality,
}

impl Config {
    fn app_dirs() -> Option<AppDirs> {
        const USE_XDG_ON_MACOS: bool = false;

        AppDirs::new(Some(APP_NAME), USE_XDG_ON_MACOS)
    }

    pub fn cache_dir() -> Option<PathBuf> {
        Self::app_dirs().map(|dirs| dirs.cache_dir)
    }

    pub fn config_dir() -> Option<PathBuf> {
        Self::app_dirs().map(|dirs| dirs.config_dir)
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join(CONFIG_FILENAME))
    }

    pub fn load() -> Option<Config> {
        let path = Self::config_path().expect("Failed to get config path");
        if let Ok(file) = File::open(&path) {
            log::info!("loading config: {:?}", &path);
            Some(serde_json::from_reader(file).expect("Failed to read config"))
        } else {
            None
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir().expect("Failed to get config dir");
        let path = Self::config_path().expect("Failed to get config path");
        mkdir_if_not_exists(&dir).expect("Failed to create config dir");
        let file = File::create(path).expect("Failed to create config");
        serde_json::to_writer_pretty(file, self).expect("Failed to write config");
    }

    pub fn has_credentials(&self) -> bool {
        !self.username.is_empty() && !self.password.is_empty()
    }

    pub fn credentials(&self) -> Option<Credentials> {
        if self.has_credentials() {
            Some(Credentials::from_username_and_password(
                self.username.to_owned(),
                self.password.to_owned(),
            ))
        } else {
            None
        }
    }

    pub fn playback(&self) -> PlaybackConfig {
        PlaybackConfig {
            bitrate: self.audio_quality.as_bitrate(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize)]
pub enum AudioQuality {
    Low,
    Normal,
    High,
}

impl AudioQuality {
    fn as_bitrate(self) -> usize {
        match self {
            AudioQuality::Low => 96,
            AudioQuality::Normal => 160,
            AudioQuality::High => 320,
        }
    }
}

impl Default for AudioQuality {
    fn default() -> Self {
        Self::High
    }
}
