use std::{
    fmt::Display,
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
    thread,
    time::Duration,
};

use druid::{
    im::Vector, image::{self, ImageFormat}, kurbo::MinDistance, Data, ImageBuf
};
use itertools::Itertools;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use ureq::{Agent, Request, Response};

use psst_core::{
    session::{access_token::TokenProvider, SessionService},
    util::default_ureq_agent_builder,
};

use crate::{
    data::{
        self, library_derived_lenses::playlists, Album, AlbumType, Artist, ArtistAlbums, AudioAnalysis, Cached, Episode, EpisodeId, EpisodeLink, MixedView, Nav, Page, Playlist, PublicUser, Range, Recommendations, RecommendationsRequest, SearchResults, SearchTopic, Show, SpotifyUrl, Track, UserProfile 
    },
    error::Error,
};

use super::{cache::WebApiCache, local::LocalTrackManager};

pub struct WebApi {
    session: SessionService,
    agent: Agent,
    cache: WebApiCache,
    token_provider: TokenProvider,
    local_track_manager: Mutex<LocalTrackManager>,
    paginated_limit: usize,
}

impl WebApi {
    pub fn new(
        session: SessionService,
        proxy_url: Option<&str>,
        cache_base: Option<PathBuf>,
        paginated_limit: usize,
    ) -> Self {
        let agent = default_ureq_agent_builder(proxy_url).unwrap().build();
        Self {
            session,
            agent,
            cache: WebApiCache::new(cache_base),
            token_provider: TokenProvider::new(),
            local_track_manager: Mutex::new(LocalTrackManager::new()),
            paginated_limit,
        }
    }

    fn access_token(&self) -> Result<String, Error> {
        let token = self
            .token_provider
            .get(&self.session)
            .map_err(|err| Error::WebApiError(err.to_string()))?;
        Ok(token.token)
    }

    fn build_request(&self, 
        method: &str, 
        base_url: &str,
        path: impl Display,
    ) -> Result<Request, Error> {
        let token = self.access_token()?;
        let request = self
            .agent
            .request(method, &format!("https://{}/{}", base_url, path))
            .set("Authorization", &format!("Bearer {}", &token));
        Ok(request)
    }

    fn request(&self, method: &str, base_url: &str, path: impl Display) -> Result<Request, Error> {
        self.build_request(method, base_url, path)
    }

    fn get(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("GET", base_url.unwrap_or("api.spotify.com"), path)    }

    fn put(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("GET", base_url.unwrap_or("api.spotify.com"), path)
    }

    fn post(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("GET", base_url.unwrap_or("api.spotify.com"), path)
    }

    fn delete(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("GET", base_url.unwrap_or("api.spotify.com"), path)
    }

    fn with_retry(f: impl Fn() -> Result<Response, Error>) -> Result<Response, Error> {
        loop {
            let response = f()?;
            match response.status() {
                429 => {
                    let retry_after_secs = response
                        .header("Retry-After")
                        .and_then(|secs| secs.parse().ok())
                        .unwrap_or(2);
                    thread::sleep(Duration::from_secs(retry_after_secs));
                }
                _ => {
                    break Ok(response);
                }
            }
        }
    }

    /// Send a request with a empty JSON object, throw away the response body.
    /// Use for POST/PUT/DELETE requests.
    fn send_empty_json(&self, request: Request) -> Result<(), Error> {
        let _response = Self::with_retry(|| Ok(request.clone().send_string("{}")?))?;
        Ok(())
    }

    /// Send a request and return the deserialized JSON body.  Use for GET
    /// requests.
    fn load<T: DeserializeOwned>(&self, request: Request) -> Result<T, Error> {
        let response = Self::with_retry(|| Ok(request.clone().call()?))?;
        let result = response.into_json()?;
        Ok(result)
    }

    /// Send a request using `self.load()`, but only if it isn't already present
    /// in cache.
    fn load_cached<T: Data + DeserializeOwned>(
        &self,
        request: Request,
        bucket: &str,
        key: &str,
    ) -> Result<Cached<T>, Error> {
        if let Some(file) = self.cache.get(bucket, key) {
            let cached_at = file.metadata()?.modified()?;
            let value = serde_json::from_reader(file)?;
            Ok(Cached::new(value, cached_at))
        } else {
            let response = Self::with_retry(|| Ok(request.clone().call()?))?;
            let body = {
                let mut reader = response.into_reader();
                let mut body = Vec::new();
                reader.read_to_end(&mut body)?;
                body
            };
            let value = serde_json::from_slice(&body)?;
            self.cache.set(bucket, key, &body);
            Ok(Cached::fresh(value))
        }
    }

    /// Iterate a paginated result set by sending `request` with added
    /// pagination parameters.  Mostly used through `load_all_pages`.
    fn for_all_pages<T: DeserializeOwned + Clone>(
        &self,
        request: Request,
        mut func: impl FnMut(Page<T>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        // TODO: Some result sets, like very long playlists and saved tracks/albums can
        // be very big.  Implement virtualized scrolling and lazy-loading of results.
        let mut limit = 50;
        let mut offset = 0;
        loop {
            let req = request
                .clone()
                .query("limit", &limit.to_string())
                .query("offset", &offset.to_string());
            let page: Page<T> = self.load(req)?;

            let page_total = page.total;
            let page_offset = page.offset;
            let page_limit = page.limit;
            func(page)?;

            if page_total > offset && offset < self.paginated_limit {
                limit = page_limit;
                offset = page_offset + page_limit;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Very similar to `for_all_pages`, but only returns a certain number of results
    /// TODO: test properly
    fn for_some_pages<T: DeserializeOwned + Clone>(
        &self,
        request: Request,
        lim: usize,
        mut func: impl FnMut(Page<T>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut limit = 50;
        let mut offset = 0;
        if lim < limit {
            limit = lim;
            let req = request
                .clone()
                .query("limit", &limit.to_string())
                .query("offset", &offset.to_string());

            let page: Page<T> = self.load(req)?;

            func(page)?;
        } else {
            loop {
                let req = request
                    .clone()
                    .query("limit", &limit.to_string())
                    .query("offset", &offset.to_string());
    
                let page: Page<T> = self.load(req)?;
                
                let page_total = limit/lim;
                let page_offset = page.offset;
                let page_limit = page.limit;
                func(page)?;
    
                if page_total > offset && offset < self.paginated_limit {
                    limit = page_limit;
                    offset = page_offset + page_limit;
                } else {
                    break;
                }
            }
        }
        Ok(())
    }
    /// Load a paginated result set by sending `request` with added pagination
    /// parameters and return the aggregated results.  Use with GET requests.
    fn load_all_pages<T: DeserializeOwned + Clone>(
        &self,
        request: Request,
    ) -> Result<Vector<T>, Error> {
        let mut results = Vector::new();

        self.for_all_pages(request, |page| {
            results.append(page.items);
            Ok(())
        })?;

        Ok(results)
    }

    /// Does a similar thing as `load_all_pages`, but limiting the number of results
    fn load_some_pages<T: DeserializeOwned + Clone>(
        &self,
        request: Request,
        number: usize,
    ) -> Result<Vector<T>, Error> {
        let mut results = Vector::new();

        self.for_some_pages(request, number, |page| {
            results.append(page.items);
            Ok(())
        })?;

        Ok(results)
    }

    /// Load local track files from the official client's database.
    pub fn load_local_tracks(&self, username: &str) {
        if let Err(err) = self
            .local_track_manager
            .lock()
            .load_tracks_for_user(username)
        {
            log::error!("failed to read local tracks: {}", err);
        }
    }
    pub fn transform_section_item_json_map(item: &serde_json::Value) -> Option<serde_json::Value> {
        item.get("data")
            .and_then(|data| data.get("homeSections"))
            .and_then(|home_sections| home_sections.get("sections"))
            .and_then(|sections| sections.get(0))
            .cloned()
    }
}

static GLOBAL_WEBAPI: OnceCell<Arc<WebApi>> = OnceCell::new();

/// Global instance.
impl WebApi {
    pub fn install_as_global(self) {
        GLOBAL_WEBAPI
            .set(Arc::new(self))
            .map_err(|_| "Cannot install more than once")
            .unwrap()
    }

    pub fn global() -> Arc<Self> {
        GLOBAL_WEBAPI.get().unwrap().clone()
    }
}

/// User endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-users-profile
    pub fn get_user_profile(&self) -> Result<UserProfile, Error> {
        let request = self.get("v1/me", None)?;
        let result = self.load(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-users-top-artists-and-tracks
    // TODO Cache this.
    pub fn get_user_top_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        let request = self.get("v1/me/top/tracks", None)?
            .query("market", "from_token");

        let result: Vector<Arc<Track>> = self.load_some_pages(request, 30)?;

        Ok(result)
    }

    // TODO Cache this.
    pub fn get_user_top_artist(&self) -> Result<Vector<Artist>, Error> {
        #[derive(Clone, Data, Deserialize)]
        struct Artists {
            artists: Artist,
        }

        let request = self.get("v1/me/top/artists", None)?;

        Ok(self
            .load_some_pages(request, 10)?
            .into_iter()
            .map(|item: Artist| item)
            .collect())
    }
}

/// Artist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-artist/
    pub fn get_artist(&self, id: &str) -> Result<Artist, Error> {
        let request = self.get(format!("v1/artists/{}", id), None)?;
        let result = self.load_cached(request, "artist", id)?;
        Ok(result.data)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-an-artists-albums/
    pub fn get_artist_albums(&self, id: &str) -> Result<ArtistAlbums, Error> {
        let request = self
            .get(format!("v1/artists/{}/albums", id), None)?
            .query("market", "from_token");
        let result: Vector<Arc<Album>> = self.load_all_pages(request)?;

        let mut artist_albums = ArtistAlbums {
            albums: Vector::new(),
            singles: Vector::new(),
            compilations: Vector::new(),
            appears_on: Vector::new(),
        };

        let mut last_album_release_year = usize::MAX;
        let mut last_single_release_year = usize::MAX;

        for album in result {
            match album.album_type {
                // Spotify is labeling albums and singles that should be labeled `appears_on` as `album` or `single`.
                // They are still ordered properly though, with the most recent first, then 'appears_on'.
                // So we just wait until they are no longer descending, then start putting them in the 'appears_on' Vec.
                // NOTE: This will break if an artist has released 'appears_on' albums/singles before their first actual album/single.
                AlbumType::Album => {
                    if album.release_year_int() > last_album_release_year {
                        artist_albums.appears_on.push_back(album)
                    } else {
                        last_album_release_year = album.release_year_int();
                        artist_albums.albums.push_back(album)
                    }
                }
                AlbumType::Single => {
                    if album.release_year_int() > last_single_release_year {
                        artist_albums.appears_on.push_back(album);
                    } else {
                        last_single_release_year = album.release_year_int();
                        artist_albums.singles.push_back(album);
                    }
                }
                AlbumType::Compilation => artist_albums.compilations.push_back(album),
                AlbumType::AppearsOn => artist_albums.appears_on.push_back(album),
            }
        }
        Ok(artist_albums)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-an-artists-top-tracks
    pub fn get_artist_top_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Deserialize)]
        struct Tracks {
            tracks: Vector<Arc<Track>>,
        }

        let request = self
            .get(format!("v1/artists/{}/top-tracks", id), None)?
            .query("market", "from_token");
        let result: Tracks = self.load(request)?;
        Ok(result.tracks)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-an-artists-related-artists
    pub fn get_related_artists(&self, id: &str) -> Result<Cached<Vector<Artist>>, Error> {
        #[derive(Clone, Data, Deserialize)]
        struct Artists {
            artists: Vector<Artist>,
        }

        let request = self.get(format!("v1/artists/{}/related-artists", id), None)?;
        let result: Cached<Artists> = self.load_cached(request, "related-artists", id)?;
        Ok(result.map(|result| result.artists))
    }
}

/// Album endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-an-album/
    pub fn get_album(&self, id: &str) -> Result<Cached<Arc<Album>>, Error> {
        let request = self
            .get(format!("v1/albums/{}", id), None)?
            .query("market", "from_token");
        let result = self.load_cached(request, "album", id)?;
        Ok(result)
    }
}

/// Show endpoints. (Podcasts)
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-multiple-episodes
    pub fn get_episodes(
        &self,
        ids: impl IntoIterator<Item = EpisodeId>,
    ) -> Result<Vector<Arc<Episode>>, Error> {
        #[derive(Deserialize)]
        struct Episodes {
            episodes: Vector<Arc<Episode>>,
        }

        let request = self
            .get("v1/episodes", None)?
            .query("ids", &ids.into_iter().map(|id| id.0.to_base62()).join(","))
            .query("market", "from_token");
        let result: Episodes = self.load(request)?;
        Ok(result.episodes)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-a-shows-episodes
    pub fn get_show_episodes(&self, id: &str) -> Result<Vector<Arc<Episode>>, Error> {
        let request = self
            .get(format!("v1/shows/{}/episodes", id), None)?
            .query("market", "from_token");
        let mut results = Vector::new();

        self.for_all_pages(request, |page: Page<EpisodeLink>| {
            if !page.items.is_empty() {
                let ids = page.items.into_iter().map(|link| link.id);
                let episodes = self.get_episodes(ids)?;
                results.append(episodes);
            }
            Ok(())
        })?;

        Ok(results)
    }
}

/// Track endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-track
    pub fn get_track(&self, id: &str) -> Result<Arc<Track>, Error> {
        let request = self
            .get(format!("v1/tracks/{}", id), None)?
            .query("market", "from_token");
        let result = self.load(request)?;
        Ok(result)
    }
}

/// Library endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-users-saved-albums/
    pub fn get_saved_albums(&self) -> Result<Vector<Arc<Album>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedAlbum {
            album: Arc<Album>,
        }

        let request = self.get("v1/me/albums", None)?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedAlbum| item.album)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-albums-user/
    pub fn save_album(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/albums", None)?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-albums-user/
    pub fn unsave_album(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/albums", None)?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-users-saved-tracks/
    pub fn get_saved_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedTrack {
            track: Arc<Track>,
        }

        let request = self.get("v1/me/tracks", None)?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedTrack| item.track)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-users-saved-shows
    pub fn get_saved_shows(&self) -> Result<Vector<Arc<Show>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedShow {
            show: Arc<Show>,
        }

        let request = self.get("v1/me/shows", None)?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedShow| item.show)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-tracks-user/
    pub fn save_track(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/tracks", None)?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-tracks-user/
    pub fn unsave_track(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/tracks", None)?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-shows-user
    pub fn save_show(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/shows", None)?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-shows-user
    pub fn unsave_show(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/shows", None)?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }
}


/* 
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/search/
    pub fn search(
        &self,
        query: &str,
        topics: &[SearchTopic],
        limit: usize,
    ) -> Result<SearchResults, Error> {
        #[derive(Deserialize)]
        struct ApiSearchResults {
            artists: Option<Page<Artist>>,
            albums: Option<Page<Arc<Album>>>,
            tracks: Option<Page<Arc<Track>>>,
            playlists: Option<Page<Playlist>>,
            shows: Option<Page<Arc<Show>>>,
        }

        let topics = topics.iter().map(SearchTopic::as_str).join(",");
        let request = self
            .get("v1/search")?
            .query("q", query)
            .query("type", &topics)
            .query("limit", &limit.to_string())
            .query("marker", "from_token");
        let result: ApiSearchResults = self.load(request)?;

        let artists = result.artists.map_or_else(Vector::new, |page| page.items);
        let albums = result.albums.map_or_else(Vector::new, |page| page.items);
        let tracks = result.tracks.map_or_else(Vector::new, |page| page.items);
        let playlists = result.playlists.map_or_else(Vector::new, |page| page.items);
        let shows = result.shows.map_or_else(Vector::new, |page| page.items);
        Ok(SearchResults {
            query: query.into(),
            artists,
            albums,
            tracks,
            playlists,
            shows,
        })
    }
}
*/
/// View endpoints.
impl WebApi {
    pub fn get_made_for_you(&self) -> Result<MixedView, Error> {
        #[derive(Deserialize)]
        pub struct Welcome {
            data: WelcomeData,
            extensions: Extensions,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct WelcomeData {
            home_sections: HomeSections,
        }
    
        #[derive(Deserialize)]
        pub struct HomeSections {
            #[serde(rename = "__typename")]
            typename: String,
            sections: Vec<Section>,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Section {
            #[serde(rename = "__typename")]
            typename: String,
            data: SectionData,
            section_items: SectionItems,
            uri: String,
        }
    
        #[derive(Deserialize)]
        pub struct SectionData {
            #[serde(rename = "__typename")]
            typename: String,
            subtitle: Subtitle,
            title: Title,
        }
    
        #[derive(Deserialize)]
        pub struct Subtitle {
            text: String,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Title {
            original_label: OriginalLabel,
            text: String,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct OriginalLabel {
            text_attributes: TextAttributes,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct TextAttributes {
            text_format_arguments: Vec<TextFormatArgument>,
        }
    
        #[derive(Deserialize)]
        pub struct TextFormatArgument {
            uri: Option<serde_json::Value>,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct SectionItems {
            items: Vec<SectionItemsItem>,
            paging_info: PagingInfo,
            total_count: i64,
        }
    
        #[derive(Deserialize)]
        pub struct SectionItemsItem {
            data: Option<serde_json::Value>,
            content: Content,
            uri: String,
        }
    
        #[derive(Deserialize)]
        pub struct Content {
            #[serde(rename = "__typename")]
            typename: String,
            data: ContentData,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct ContentData {
            // This needs to be variable, based off the type name
            #[serde(rename = "__typename")]
            typename: String,
            attributes: Option<Vec<Attribute>>,
            description: Option<String>,
            format: Option<String>,
            images: Option<Images>,
            name: Option<String>,
            owner_v2: Option<OwnerV2>,
            uri: String,
            artists: Option<Artists>,
            cover_art: Option<Images>,
            album_type: Option<String>,
            profile: Option<Profile>,
            media_type: Option<String>,
            publisher: Option<Profile>,
        }

        #[derive(Deserialize)]
        pub struct Artists {
            items: Vec<ArtistsItem>,
        }
        
        #[derive(Deserialize)]
        pub struct ArtistsItem {
            profile: Profile,
            uri: String,
        }
        
        #[derive(Deserialize)]
        pub struct Profile {
            name: String,
        }

        #[derive(Deserialize)]
        pub struct Attribute {
            key: String,
            value: String,
        }
    
        #[derive(Deserialize)]
        pub struct Images {
            items: Vec<ImagesItem>,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct ImagesItem {
            extracted_colors: ExtractedColors,
            sources: Vec<Source>,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct ExtractedColors {
            color_dark: ColorDark,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct ColorDark {
            hex: String,
            is_fallback: bool,
        }
    
        #[derive(Deserialize)]
        pub struct Source {
            height: Option<serde_json::Value>,
            url: String,
            width: Option<serde_json::Value>,
        }
    
        #[derive(Deserialize)]
        pub struct OwnerV2 {
            data: OwnerV2Data,
        }
    
        #[derive(Deserialize)]
        pub struct OwnerV2Data {
            #[serde(rename = "__typename")]
            typename: String,
            name: String,
            uri: String,
        }
    
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct PagingInfo {
            next_offset: Option<serde_json::Value>,
        }
    
        #[derive(Deserialize)]
        pub struct Extensions {}
    
        let request = self.get("pathfinder/v1/query?operationName=homeSection&variables=%7B%22uri%22%3A%22spotify%3Asection%3A0JQ5DAUnp4wcj0bCb3wh3S%22%2C%22timeZone%22%3A%22Europe%2FLondon%22%2C%22sp_t%22%3A%223c7e9795a8ab85165839a5e905d6f10c%22%2C%22country%22%3A%22GB%22%2C%22sectionItemsOffset%22%3A0%2C%22sectionItemsLimit%22%3A20%7D&extensions=%7B%22persistedQuery%22%3A%7B%22version%22%3A1%2C%22sha256Hash%22%3A%224da53a78e4e98d4f3fa55698af5b751fe05ca3a1a4a526ff8147e8866ccfa49f%22%7D%7D", Some("api-partner.spotify.com"))?;
        
        // Extract the playlists
        let result: Welcome = self.load(request)?;
    
        let playlist: Vector<Playlist> = result.data.home_sections.sections
        .iter()
        .flat_map(|section| {
            section.section_items.items.iter().map(|item| {
                let uri = item.uri.clone();
                let id = uri.split(':').last().unwrap_or("").to_string();

                Playlist {
                    id: id.into(),
                    name: item.content.data.name.clone().unwrap_or_default().into(),
                    images: item.content.data.images.as_ref().map(|images| 
                        images.items.iter().map(|img| data::utils::Image {
                            url: img.sources.first().map(|s| s.url.clone()).unwrap_or_default().into(),
                            width: None,
                            height: None,
                        }).collect()
                    ),
                    description: item.content.data.description.clone().unwrap_or_default().into(),
                    track_count: Some(10),
                    owner: PublicUser {
                        id: "".into(),

                        display_name: item.content.data.owner_v2.as_ref()
                            .map(|owner| owner.data.name.clone())
                            .unwrap_or_default()
                            .into(),
                    },
                    collaborative: false,
                }
            })
        })
        .collect();

        Ok(MixedView {
            playlists: playlist,
            artists: Vector::new(),
            albums: Vector::new(),
            shows: Vector::new(),
        })
    }

    pub fn podcasts_and_more(&self) -> Result<Vector<Show>, Error> {
        #[derive(Deserialize)]
        struct View {
            content: Page<Show>,
        }
        let request = self
            .get("v1/views/podcasts-and-more", None)?
            .query("types", "Show")
            .query("limit", "20")
            .query("offset", "0");
        let result: View = self.load(request)?;
        Ok(result.content.items)
    }
}

/// Playlist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-a-list-of-current-users-playlists
    pub fn get_playlists(&self) -> Result<Vector<Playlist>, Error> {
        let request = self.get("v1/me/playlists", None)?;
        let result = self.load_all_pages(request)?;
        Ok(result)
    }

    pub fn follow_playlist(&self, id: &str) -> Result<(), Error> {
        let request = self.put(format!("v1/playlists/{}/followers", id), None)?;
        request.send_json(json!({"public": false,}))?;
        Ok(())
    }

    pub fn unfollow_playlist(&self, id: &str) -> Result<(), Error> {
        let request = self.delete(format!("v1/playlists/{}/followers", id), None)?;
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-playlist
    pub fn get_playlist(&self, id: &str) -> Result<Playlist, Error> {
        let request = self.get(format!("v1/me/playlists/{}", id), None)?;
        let result = self.load(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-playlists-tracks
    pub fn get_playlist_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct PlaylistItem {
            track: OptionalTrack,
        }

        // Spotify API likes to return _really_ bogus data for local tracks. Much better
        // would be to ignore parsing this completely if `is_local` is true, but this
        // will do as well.
        #[derive(Clone, Deserialize)]
        #[serde(untagged)]
        enum OptionalTrack {
            Track(Arc<Track>),
            Json(serde_json::Value),
        }

        let request = self
            .get(format!("v1/playlists/{}/tracks", id), None)?
            .query("marker", "from_token")
            .query("additional_types", "track");
        let result: Vector<PlaylistItem> = self.load_all_pages(request)?;

        let local_track_manager = self.local_track_manager.lock();

        Ok(result
            .into_iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let mut track = match item.track {
                    OptionalTrack::Track(track) => track,
                    OptionalTrack::Json(json) => local_track_manager.find_local_track(json)?,
                };
                Arc::make_mut(&mut track).track_pos = index;
                Some(track)
            })
            .collect())
    }

    pub fn change_playlist_details(&self, id: &str, name: &str) -> Result<(), Error> {
        let request = self.put(format!("v1/playlists/{}", id), None)?;
        request.send_json(json!({ "name": name }))?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/add-tracks-to-playlist
    pub fn add_track_to_playlist(&self, playlist_id: &str, track_uri: &str) -> Result<(), Error> {
        let request = self
            .post(format!("v1/playlists/{}/tracks", playlist_id), None)?
            .query("uris", track_uri);
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-tracks-playlist
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        track_pos: usize,
    ) -> Result<(), Error> {
        self.delete(format!("v1/playlists/{}/tracks", playlist_id), None)?
            .send_json(ureq::json!({ "positions": [track_pos] }))?;
        Ok(())
    }
}

/// Search endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/search/
    pub fn search(
        &self,
        query: &str,
        topics: &[SearchTopic],
        limit: usize,
    ) -> Result<SearchResults, Error> {
        #[derive(Deserialize)]
        struct ApiSearchResults {
            artists: Option<Page<Artist>>,
            albums: Option<Page<Arc<Album>>>,
            tracks: Option<Page<Arc<Track>>>,
            playlists: Option<Page<Playlist>>,
            shows: Option<Page<Arc<Show>>>,
        }

        let topics = topics.iter().map(SearchTopic::as_str).join(",");
        let request = self
            .get("v1/search", None)?
            .query("q", query)
            .query("type", &topics)
            .query("limit", &limit.to_string())
            .query("marker", "from_token");
        let result: ApiSearchResults = self.load(request)?;

        let artists = result.artists.map_or_else(Vector::new, |page| page.items);
        let albums = result.albums.map_or_else(Vector::new, |page| page.items);
        let tracks = result.tracks.map_or_else(Vector::new, |page| page.items);
        let playlist = result.playlists.map_or_else(Vector::new, |page| page.items);
        let shows = result.shows.map_or_else(Vector::new, |page| page.items);
        Ok(SearchResults {
            query: query.into(),
            artists,
            albums,
            tracks,
            playlists: playlist,
            shows,
        })
    }

    pub fn load_spotify_link(&self, link: &SpotifyUrl) -> Result<Nav, Error> {
        let nav = match link {
            SpotifyUrl::Playlist(id) => Nav::PlaylistDetail(self.get_playlist(id)?.link()),
            SpotifyUrl::Artist(id) => Nav::ArtistDetail(self.get_artist(id)?.link()),
            SpotifyUrl::Album(id) => Nav::AlbumDetail(self.get_album(id)?.data.link()),
            SpotifyUrl::Show(id) => Nav::AlbumDetail(self.get_album(id)?.data.link()),
            SpotifyUrl::Track(id) => Nav::AlbumDetail(
                // TODO: We should highlight the exact track in the album.
                self.get_track(id)?.album.clone().ok_or_else(|| {
                    Error::WebApiError("Track was found but has no album".to_string())
                })?,
            ),
        };
        Ok(nav)
    }
}

/// Recommendation endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-recommendations
    pub fn get_recommendations(
        &self,
        data: Arc<RecommendationsRequest>,
    ) -> Result<Recommendations, Error> {
        let seed_artists = data.seed_artists.iter().map(|link| &link.id).join(", ");
        let seed_tracks = data
            .seed_tracks
            .iter()
            .map(|track| track.0.to_base62())
            .join(", ");

        let mut request = self
            .get("v1/recommendations", None)?
            .query("marker", "from_token")
            .query("limit", "100")
            .query("seed_artists", &seed_artists)
            .query("seed_tracks", &seed_tracks);

        fn add_range_param(mut req: Request, r: Range<impl ToString>, s: &str) -> Request {
            if let Some(v) = r.min {
                req = req.query(&format!("min_{}", s), &v.to_string());
            }
            if let Some(v) = r.max {
                req = req.query(&format!("max_{}", s), &v.to_string());
            }
            if let Some(v) = r.target {
                req = req.query(&format!("target_{}", s), &v.to_string());
            }
            req
        }
        request = add_range_param(request, data.params.duration_ms, "duration_ms");
        request = add_range_param(request, data.params.popularity, "popularity");
        request = add_range_param(request, data.params.key, "key");
        request = add_range_param(request, data.params.mode, "mode");
        request = add_range_param(request, data.params.tempo, "tempo");
        request = add_range_param(request, data.params.time_signature, "time_signature");
        request = add_range_param(request, data.params.acousticness, "acousticness");
        request = add_range_param(request, data.params.danceability, "danceability");
        request = add_range_param(request, data.params.energy, "energy");
        request = add_range_param(request, data.params.instrumentalness, "instrumentalness");
        request = add_range_param(request, data.params.liveness, "liveness");
        request = add_range_param(request, data.params.loudness, "loudness");
        request = add_range_param(request, data.params.speechiness, "speechiness");
        request = add_range_param(request, data.params.valence, "valence");

        let mut result: Recommendations = self.load(request)?;
        result.request = data;
        Ok(result)
    }
}

/// Track endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-audio-analysis/
    pub fn _get_audio_analysis(&self, track_id: &str) -> Result<AudioAnalysis, Error> {
        let request = self.get(format!("v1/audio-analysis/{}", track_id), None)?;
        let result = self.load_cached(request, "audio-analysis", track_id)?;
        Ok(result.data)
    }
}

/// Image endpoints.
impl WebApi {
    pub fn get_cached_image(&self, uri: &Arc<str>) -> Option<ImageBuf> {
        self.cache.get_image(uri)
    }

    pub fn get_image(&self, uri: Arc<str>) -> Result<ImageBuf, Error> {
        if let Some(cached_image) = self.cache.get_image(&uri) {
            return Ok(cached_image);
        }

        if let Some(disk_cached_image) = self.cache.get_image_from_disk(&uri) {
            self.cache.set_image(uri.clone(), disk_cached_image.clone());
            return Ok(disk_cached_image);
        }

        let response = self.agent.get(&uri).call()?;
        let format = match response.content_type() {
            "image/jpeg" => Some(ImageFormat::Jpeg),
            "image/png" => Some(ImageFormat::Png),
            _ => None,
        };
        let mut body = Vec::new();
        response.into_reader().read_to_end(&mut body)?;

        // Save raw image data to disk cache
        self.cache.save_image_to_disk(&uri, &body);

        let image = if let Some(format) = format {
            image::load_from_memory_with_format(&body, format)?
        } else {
            image::load_from_memory(&body)?
        };
        let image_buf = ImageBuf::from_dynamic_image(image);
        self.cache.set_image(uri, image_buf.clone());
        Ok(image_buf)
    }
}

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
