use std::{
    env::{self, VarError},
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

#[cfg(target_family = "unix")]
use std::os::unix::fs::OpenOptionsExt;

use druid::{Data, Lens, Size};
use platform_dirs::AppDirs;
use psst_core::{
    cache::{mkdir_if_not_exists, CacheHandle},
    connection::Credentials,
    oauth::{self, WebApiToken},
    player::PlaybackConfig,
    session::{SessionConfig, SessionConnection},
};
use serde::{Deserialize, Serialize};

use super::{Nav, Promise, QueueBehavior, SliderScrollScale};
use crate::ui::theme;

fn default_volume() -> f64 {
    1.0
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Preferences {
    pub active: PreferencesTab,
    #[data(ignore)]
    pub cache: Option<CacheHandle>,
    pub cache_size: Promise<u64, (), ()>,
    pub auth: Authentication,
    pub lastfm_auth_result: Option<String>,
}

impl Preferences {
    pub fn reset(&mut self) {
        self.cache_size.clear();
        self.auth.result.clear();
        self.auth.lastfm_api_key_input.clear();
        self.auth.lastfm_api_secret_input.clear();
    }

    pub fn measure_cache_usage() -> Option<u64> {
        Config::cache_dir().and_then(|path| get_dir_size(&path))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Data)]
pub enum PreferencesTab {
    General,
    Account,
    Cache,
    About,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Authentication {
    pub username: String,
    pub password: String,
    pub access_token: String,
    pub result: Promise<(), (), String>,
    #[data(ignore)]
    pub lastfm_api_key_input: String,
    #[data(ignore)]
    pub lastfm_api_secret_input: String,
}

impl Authentication {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            access_token: String::new(),
            result: Promise::Empty,
            lastfm_api_key_input: String::new(),
            lastfm_api_secret_input: String::new(),
        }
    }

    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            login_creds: if !self.access_token.is_empty() {
                Credentials::from_access_token(self.access_token.clone())
            } else {
                Credentials::from_username_and_password(
                    self.username.clone(),
                    self.password.clone(),
                )
            },
            proxy_url: Config::proxy(),
        }
    }

    pub fn authenticate_and_get_credentials(config: SessionConfig) -> Result<Credentials, String> {
        let connection = SessionConnection::open(config).map_err(|err| err.to_string())?;
        Ok(connection.credentials)
    }

    pub fn clear(&mut self) {
        self.username.clear();
        self.password.clear();
    }
}

const APP_NAME: &str = "Psst";
const CONFIG_FILENAME: &str = "config.json";
const PROXY_ENV_VAR: &str = "SOCKS_PROXY";

#[derive(Clone, Debug, Data, Lens, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[data(ignore)]
    credentials: Option<Credentials>,
    pub audio_quality: AudioQuality,
    pub theme: Theme,
    #[serde(default = "default_volume")]
    pub volume: f64,
    pub last_route: Option<Nav>,
    pub queue_behavior: QueueBehavior,
    pub show_track_cover: bool,
    pub window_size: Size,
    pub slider_scroll_scale: SliderScrollScale,
    pub sort_order: SortOrder,
    pub sort_criteria: SortCriteria,
    pub paginated_limit: usize,
    pub seek_duration: usize,
    pub lastfm_session_key: Option<String>,
    pub lastfm_api_key: Option<String>,
    pub lastfm_api_secret: Option<String>,
    pub lastfm_enable: bool,
    /// User-provided Spotify Developer Client ID for Web API calls.
    /// Register one at https://developer.spotify.com/dashboard
    pub webapi_client_id: Option<String>,
    /// Cached Web API OAuth token (access + refresh + expiry).
    #[data(ignore)]
    webapi_token: Option<WebApiToken>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            credentials: Default::default(),
            audio_quality: Default::default(),
            theme: Default::default(),
            volume: 1.0,
            last_route: Default::default(),
            queue_behavior: Default::default(),
            show_track_cover: Default::default(),
            window_size: Size::new(theme::grid(80.0), theme::grid(100.0)),
            slider_scroll_scale: Default::default(),
            sort_order: Default::default(),
            sort_criteria: Default::default(),
            paginated_limit: 500,
            seek_duration: 10,
            lastfm_session_key: None,
            lastfm_api_key: None,
            lastfm_api_secret: None,
            lastfm_enable: false,
            webapi_client_id: None,
            webapi_token: None,
        }
    }
}

impl Config {
    fn app_dirs() -> Option<AppDirs> {
        const USE_XDG_ON_MACOS: bool = false;

        AppDirs::new(Some(APP_NAME), USE_XDG_ON_MACOS)
    }

    pub fn spotify_local_files_file(username: &str) -> Option<PathBuf> {
        AppDirs::new(Some("spotify"), false).map(|dir| {
            let path = format!("Users/{username}-user/local-files.bnk");
            dir.config_dir.join(path)
        })
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
            log::info!("loading config: {:?}", path);
            let reader = BufReader::new(file);
            Some(serde_json::from_reader(reader).expect("Failed to read config"))
        } else {
            None
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir().expect("Failed to get config dir");
        let path = Self::config_path().expect("Failed to get config path");
        mkdir_if_not_exists(&dir).expect("Failed to create config dir");

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(target_family = "unix")]
        options.mode(0o600);

        let file = options.open(&path).expect("Failed to create config");
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self).expect("Failed to write config");
        log::info!("saved config: {:?}", path);
    }

    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    pub fn store_credentials(&mut self, credentials: Credentials) {
        self.credentials = Some(credentials);
    }

    pub fn clear_credentials(&mut self) {
        self.credentials = Default::default();
    }

    pub fn username(&self) -> Option<&str> {
        self.credentials
            .as_ref()
            .and_then(|c| c.username.as_deref())
    }

    pub fn session(&self) -> SessionConfig {
        SessionConfig {
            login_creds: self.credentials.clone().expect("Missing credentials"),
            proxy_url: Config::proxy(),
        }
    }

    pub fn webapi_client_id_value(&self) -> Option<&str> {
        self.webapi_client_id.as_deref().filter(|s| !s.is_empty())
    }

    pub fn store_webapi_token(&mut self, token: WebApiToken) {
        self.webapi_token = Some(token);
    }

    pub fn clear_webapi_token(&mut self) {
        self.webapi_token = None;
    }

    /// Try to get a valid Web API access token, refreshing if expired.
    /// Returns `Ok(token)` if successful, `Err(...)` if no token or refresh fails.
    pub fn get_or_refresh_webapi_token(&mut self) -> Result<WebApiToken, String> {
        let client_id = self
            .webapi_client_id_value()
            .ok_or_else(|| "No Web API Client ID configured".to_string())?
            .to_string();

        if let Some(ref token) = self.webapi_token {
            if !token.is_expired() {
                return Ok(token.clone());
            }

            // Token is expired, try to refresh
            if let Some(ref refresh_token) = token.refresh_token {
                log::info!("Web API token expired, attempting refresh...");
                match oauth::refresh_webapi_token(&client_id, refresh_token) {
                    Ok(new_token) => {
                        self.webapi_token = Some(new_token.clone());
                        self.save();
                        return Ok(new_token);
                    }
                    Err(e) => {
                        log::error!("Failed to refresh Web API token: {e}");
                    }
                }
            }
        }

        Err("No valid Web API token available. Browser-based authentication required.".to_string())
    }

    pub fn playback(&self) -> PlaybackConfig {
        PlaybackConfig {
            bitrate: self.audio_quality.as_bitrate(),
            ..PlaybackConfig::default()
        }
    }

    pub fn proxy() -> Option<String> {
        env::var(PROXY_ENV_VAR).map_or_else(
            |err| match err {
                VarError::NotPresent => None,
                VarError::NotUnicode(_) => {
                    log::error!("proxy URL is not a valid unicode");
                    None
                }
            },
            Some,
        )
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum AudioQuality {
    Low,
    Normal,
    #[default]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum SortCriteria {
    Title,
    Artist,
    Album,
    Duration,
    #[default]
    DateAdded,
}

fn get_dir_size(path: &Path) -> Option<u64> {
    fs::read_dir(path).ok()?.try_fold(0, |acc, entry| {
        let entry = entry.ok()?;
        let size = if entry.file_type().ok()?.is_dir() {
            get_dir_size(&entry.path())?
        } else {
            entry.metadata().ok()?.len()
        };
        Some(acc + size)
    })
}
