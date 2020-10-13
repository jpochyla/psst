use aspotify::DatePrecision;
use chrono::NaiveDate;
use druid::{im::Vector, Data, Lens};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{fs::File, ops::Deref, sync::Arc, time::Duration};

#[derive(Clone, Debug, Default, Data, Serialize, Deserialize)]
pub struct Config {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Config {
    pub fn load() -> Option<Config> {
        if let Ok(file) = File::open("config.json") {
            Some(serde_json::from_reader(file).expect("Failed to read config"))
        } else {
            None
        }
    }

    pub fn save(&self) {
        let file = File::create("config.json").expect("Failed to open config");
        serde_json::to_writer_pretty(file, self).expect("Failed to write config");
    }
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct State {
    pub route: Route,
    pub nav_stack: Vector<Navigation>,
    pub config: Config,
    pub playback: Playback,
    pub search: Search,
    pub album: AlbumDetail,
    pub artist: ArtistDetail,
    pub playlist: PlaylistDetail,
    pub library: Library,
}

#[derive(Clone, Debug, Data, Eq, PartialEq, Hash)]
pub enum Route {
    Home,
    SearchResults,
    AlbumDetail,
    ArtistDetail,
    PlaylistDetail,
    Library,
}

impl Default for Route {
    fn default() -> Self {
        Route::Home
    }
}

#[derive(Clone, Debug, Data)]
pub enum Navigation {
    Home,
    SearchResults(String),
    AlbumDetail(String),
    ArtistDetail(String),
    PlaylistDetail(Playlist),
    Library,
}

impl Navigation {
    pub fn as_route(&self) -> Route {
        match self {
            Navigation::Home => Route::Home,
            Navigation::SearchResults(_) => Route::SearchResults,
            Navigation::AlbumDetail(_) => Route::AlbumDetail,
            Navigation::ArtistDetail(_) => Route::ArtistDetail,
            Navigation::PlaylistDetail(_) => Route::PlaylistDetail,
            Navigation::Library => Route::Library,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PlaybackReport {
    pub progress: AudioDuration,
    pub item: String,
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct Playback {
    pub is_playing: bool,
    pub progress: Option<AudioDuration>,
    pub item: Option<Arc<Track>>,
    pub analysis: Option<AudioAnalysis>,
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct Search {
    pub input: String,
    pub artists: Vector<Artist>,
    pub albums: Vector<Album>,
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct AlbumDetail {
    pub id: String,
    pub album: Option<Album>,
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct ArtistDetail {
    pub id: String,
    pub artist: Option<Artist>,
    pub albums: Option<Vector<Album>>,
    pub top_tracks: Option<Vector<Arc<Track>>>,
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct PlaylistDetail {
    pub playlist: Option<Playlist>,
    pub tracks: Option<Vector<Arc<Track>>>,
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct Library {
    pub saved_albums: Option<Vector<Album>>,
    pub saved_tracks: Option<Vector<Arc<Track>>>,
    pub playlists: Option<Vector<Playlist>>,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Artist {
    pub id: String,
    pub name: Arc<str>,
    pub images: Vector<Image>,
}

impl Artist {
    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        self.images
            .iter()
            .rev()
            .find(|img| !img.fits(width, height))
            .or_else(|| self.images.back())
    }
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Album {
    pub album_type: AlbumType,
    pub artists: Vector<Artist>,
    pub id: String,
    pub images: Vector<Image>,
    pub genres: Vector<Arc<str>>,
    pub name: Arc<str>,
    #[data(same_fn = "PartialEq::eq")]
    pub release_date: Option<NaiveDate>,
    #[data(same_fn = "PartialEq::eq")]
    pub release_date_precision: Option<DatePrecision>,
    pub tracks: Vector<Arc<Track>>,
}

impl Album {
    pub fn artist_list(&self) -> String {
        self.artists.iter().map(|artist| &artist.name).join(", ")
    }

    pub fn release(&self) -> String {
        self.format_release_date(match self.release_date_precision {
            Some(DatePrecision::Year) | None => "%Y",
            Some(DatePrecision::Month) => "%B %Y",
            Some(DatePrecision::Day) => "%x",
        })
    }

    pub fn release_year(&self) -> String {
        self.format_release_date("%Y")
    }

    fn format_release_date(&self, format: &str) -> String {
        self.release_date
            .as_ref()
            .map(|date| date.format(format).to_string())
            .unwrap_or_else(|| '-'.to_string())
    }

    pub fn image(&self, width: f64, height: f64) -> Option<&Image> {
        self.images
            .iter()
            .rev()
            .find(|img| !img.fits(width, height))
            .or_else(|| self.images.back())
    }
}

#[derive(Clone, Debug, Data, Eq, PartialEq)]
pub enum AlbumType {
    Album,
    Single,
    Compilation,
}

impl Default for AlbumType {
    fn default() -> Self {
        Self::Album
    }
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Track {
    pub album: Option<Album>,
    pub artists: Vector<Artist>,
    pub disc_number: usize,
    pub duration: AudioDuration,
    pub explicit: bool,
    pub id: Option<String>,
    pub is_local: bool,
    pub is_playable: Option<bool>,
    pub name: Arc<str>,
    pub popularity: Option<u32>,
    pub track_number: usize,
}

impl Track {
    pub fn artist_name(&self) -> String {
        self.artists
            .front()
            .map(|artist| artist.name.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn album_name(&self) -> String {
        self.album
            .as_ref()
            .map(|album| album.name.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn link(&self) -> String {
        format!(
            "https://open.spotify.com/track/{id}",
            id = self.id.as_ref().unwrap()
        )
    }
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct Playlist {
    pub id: String,
    pub images: Vector<Image>,
    pub name: String,
}

#[derive(Clone, Debug, Default, Data)]
pub struct Image {
    pub url: String,
    pub width: Option<usize>,
    pub height: Option<usize>,
}

impl Image {
    pub fn fits(&self, width: f64, height: f64) -> bool {
        if let (Some(w), Some(h)) = (self.width, self.height) {
            (w as f64) < width && (h as f64) < height
        } else {
            true // Unknown dimensions, treat as fitting.
        }
    }
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct AudioAnalysis {
    pub segments: Vector<AudioAnalysisSegment>,
}

impl AudioAnalysis {
    pub fn get_minmax_loudness(&self) -> (f64, f64) {
        self.segments
            .iter()
            .map(|segment| segment.loudness_max)
            .minmax()
            .into_option()
            .unwrap_or((0.0, 0.0))
    }
}

#[derive(Clone, Debug, Default, Data)]
pub struct AudioAnalysisSegment {
    pub start: AudioDuration,
    pub duration: AudioDuration,
    pub confidence: f64,
    pub loudness_start: f64,
    pub loudness_max_time: f64,
    pub loudness_max: f64,
    pub pitches: Vector<f64>,
    pub timbre: Vector<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct AudioDuration(Duration);

impl Data for AudioDuration {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Deref for AudioDuration {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Duration> for AudioDuration {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl AudioDuration {
    pub fn as_minutes_and_seconds(&self) -> String {
        let minutes = self.as_secs() / 60;
        let seconds = self.as_secs() % 60;
        format!("{}:{:02}", minutes, seconds)
    }
}
