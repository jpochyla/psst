use crate::{
    data::{
        Album, AlbumType, Artist, ArtistAlbums, AudioAnalysis, Page, Playlist, SearchResults, Track,
    },
    error::Error,
};
use druid::{im::Vector, image};
use psst_core::{
    access_token::TokenProvider, session::SessionHandle, util::default_ureq_agent_builder,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    fmt::Display,
    io::{self, Read},
    sync::Arc,
    thread,
    time::Duration,
};
use ureq::{Agent, Request, Response};

pub struct WebApi {
    session: SessionHandle,
    agent: Agent,
    token_provider: TokenProvider,
}

impl WebApi {
    pub fn new(session: SessionHandle, proxy_url: Option<&str>) -> Self {
        let agent = default_ureq_agent_builder(proxy_url).unwrap().build();
        Self {
            session,
            agent,
            token_provider: TokenProvider::new(),
        }
    }

    fn access_token(&self) -> Result<String, Error> {
        let token = self
            .token_provider
            .get(&self.session)
            .map_err(|err| Error::WebApiError(err.to_string()))?;
        Ok(token.token)
    }

    fn request(&self, method: &str, path: impl Display) -> Result<Request, Error> {
        let token = self.access_token()?;
        let request = self
            .agent
            .request(method, &format!("https://api.spotify.com/{}", path))
            .set("Authorization", &format!("Bearer {}", &token));
        Ok(request)
    }

    fn get(&self, path: impl Display) -> Result<Request, Error> {
        self.request("GET", path)
    }

    fn put(&self, path: impl Display) -> Result<Request, Error> {
        self.request("PUT", path)
    }

    fn delete(&self, path: impl Display) -> Result<Request, Error> {
        self.request("DELETE", path)
    }

    fn with_retry(f: impl Fn() -> Result<Response, Error>) -> Result<Response, Error> {
        loop {
            let response = f()?;
            match response.status() {
                429 => {
                    //
                    let retry_after_secs = response
                        .header("Retry-After")
                        .and_then(|secs| secs.parse().ok())
                        .unwrap_or(2);
                    thread::sleep(Duration::from_secs(retry_after_secs));
                }
                _ => {
                    //
                    break Ok(response);
                }
            }
        }
    }

    fn load<T: DeserializeOwned>(&self, request: Request) -> Result<T, Error> {
        let response = Self::with_retry(|| {
            let response = request.clone().call()?;
            Ok(response)
        })?;
        let result = response.into_json()?;
        Ok(result)
    }

    fn send_empty_json(&self, request: Request) -> Result<(), Error> {
        Self::with_retry(|| {
            let response = request.clone().send_string("{}")?;
            Ok(response)
        })?;
        Ok(())
    }

    fn load_all_pages<T: DeserializeOwned + Clone>(
        &self,
        request: Request,
    ) -> Result<Vector<T>, Error> {
        // TODO: Some result sets, like very long playlists and saved tracks/albums can
        // be very big.  Implement virtualized scrolling and lazy-loading of results.
        const PAGED_ITEMS_LIMIT: usize = 200;

        let mut results = Vector::new();
        let mut limit = 50;
        let mut offset = 0;
        loop {
            let req = request
                .clone()
                .query("limit", &limit.to_string())
                .query("offset", &offset.to_string());
            let page: Page<T> = self.load(req)?;

            results.extend(page.items);

            if page.total > results.len() && results.len() < PAGED_ITEMS_LIMIT {
                limit = page.limit;
                offset = page.offset + page.limit;
            } else {
                break;
            }
        }
        Ok(results)
    }
}

/// Artist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/artists/get-artist/
    pub fn get_artist(&self, id: &str) -> Result<Artist, Error> {
        let request = self.get(format!("v1/artists/{}", id))?;
        let result = self.load(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/artists/get-artists-albums/
    pub fn get_artist_albums(&self, id: &str) -> Result<ArtistAlbums, Error> {
        let request = self
            .get(format!("v1/artists/{}/albums", id))?
            .query("market", "from_token");
        let result: Vector<Album> = self.load_all_pages(request)?;

        let mut artist_albums = ArtistAlbums {
            albums: Vector::new(),
            singles: Vector::new(),
            compilations: Vector::new(),
        };
        for album in result {
            match album.album_type {
                AlbumType::Album => artist_albums.albums.push_back(album),
                AlbumType::Single => artist_albums.singles.push_back(album),
                AlbumType::Compilation => artist_albums.compilations.push_back(album),
            }
        }
        Ok(artist_albums)
    }

    // https://developer.spotify.com/documentation/web-api/reference/artists/get-artists-top-tracks/
    pub fn get_artist_top_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Deserialize)]
        struct Tracks {
            tracks: Vector<Arc<Track>>,
        };

        let request = self
            .get(format!("v1/artists/{}/top-tracks", id))?
            .query("market", "from_token");
        let result: Tracks = self.load(request)?;
        Ok(result.tracks)
    }

    // https://developer.spotify.com/documentation/web-api/reference/artists/get-related-artists/
    pub fn get_related_artists(&self, id: &str) -> Result<Vector<Artist>, Error> {
        #[derive(Deserialize)]
        struct Artists {
            artists: Vector<Artist>,
        };

        let request = self.get(format!("v1/artists/{}/related-artists", id))?;
        let result: Artists = self.load(request)?;
        Ok(result.artists)
    }
}

/// Album endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/albums/get-album/
    pub fn get_album(&self, id: &str) -> Result<Album, Error> {
        let request = self
            .get(format!("v1/albums/{}", id))?
            .query("market", "from_token");
        let result = self.load(request)?;
        Ok(result)
    }
}

/// Library endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/library/get-users-saved-albums/
    pub fn get_saved_albums(&self) -> Result<Vector<Album>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedAlbum {
            album: Album,
        };

        let request = self.get("v1/me/albums")?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedAlbum| item.album)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/save-albums-user/
    pub fn save_album(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/albums")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/remove-albums-user/
    pub fn unsave_album(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/albums")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/get-users-saved-tracks/
    pub fn get_saved_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedTrack {
            track: Arc<Track>,
        };

        let request = self.get("v1/me/tracks")?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedTrack| item.track)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/save-tracks-user/
    pub fn save_track(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/tracks")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/remove-tracks-user/
    pub fn unsave_track(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/tracks")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }
}

/// Playlist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/playlists/get-a-list-of-current-users-playlists/
    pub fn get_playlists(&self) -> Result<Vector<Playlist>, Error> {
        let request = self.get("v1/me/playlists")?;
        let result = self.load_all_pages(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/playlists/get-playlist-tracks/
    pub fn get_playlist_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct PlaylistItem {
            track: Arc<Track>,
        }

        let request = self
            .get(format!("v1/playlists/{}/tracks", id))?
            .query("marker", "from_token")
            .query("additional_types", "track");
        let result: Vector<PlaylistItem> = self.load_all_pages(request)?;

        Ok(result.into_iter().map(|item| item.track).collect())
    }
}

/// Search endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/search/
    pub fn search(&self, query: &str) -> Result<SearchResults, Error> {
        #[derive(Deserialize)]
        struct ApiSearchResults {
            artists: Option<Page<Artist>>,
            albums: Option<Page<Album>>,
            tracks: Option<Page<Arc<Track>>>,
            playlists: Option<Page<Playlist>>,
        }

        let request = self
            .get("v1/search")?
            .query("q", query)
            .query("type", "artist,album,track,playlist")
            .query("marker", "from_token");
        let result: ApiSearchResults = self.load(request)?;

        let artists = result.artists.map_or_else(Vector::new, |page| page.items);
        let albums = result.albums.map_or_else(Vector::new, |page| page.items);
        let tracks = result.tracks.map_or_else(Vector::new, |page| page.items);
        let playlists = result.playlists.map_or_else(Vector::new, |page| page.items);
        Ok(SearchResults {
            query: query.to_string(),
            artists,
            albums,
            tracks,
            playlists,
        })
    }
}

/// Track endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/tracks/get-audio-analysis/
    pub fn get_audio_analysis(&self, track_id: &str) -> Result<AudioAnalysis, Error> {
        let request = self.get(format!("v1/audio-analysis/{}", track_id))?;
        let result = self.load(request)?;
        Ok(result)
    }
}

/// Image endpoints.
impl WebApi {
    pub fn get_image(
        &self,
        uri: &str,
        format: image::ImageFormat,
    ) -> Result<image::DynamicImage, Error> {
        let mut image_bytes = Vec::new();
        self.agent
            .get(uri)
            .call()?
            .into_reader()
            .read_to_end(&mut image_bytes)?;
        let image = image::load_from_memory_with_format(&image_bytes, format)?;
        Ok(image)
    }
}

const LOCAL_ARTIST_ID: &str = "local_artist";
const LOCAL_ALBUM_ID: &str = "local_album";

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<image::ImageError> for Error {
    fn from(err: image::ImageError) -> Self {
        Error::WebApiError(err.to_string())
    }
}
