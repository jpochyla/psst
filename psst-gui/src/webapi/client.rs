use std::{
    collections::HashMap,
    fmt::Display,
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
    thread,
    time::Duration,
};

use druid::{
    im::Vector,
    image::{self, ImageFormat},
    Data, ImageBuf,
};

use itertools::Itertools;
use log::info;
use parking_lot::Mutex;
use psst_core::{
    oauth::{self, WebApiToken},
    session::{
        client_token::{ClientTokenProvider, ClientTokenProviderHandle},
        login5::Login5,
        SessionService,
    },
    system_info::{OS, SPOTIFY_SEMANTIC_VERSION},
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use std::sync::OnceLock;
use time::{Date, Month};
use ureq::{
    http::{Response, StatusCode},
    Agent, Body,
};

use crate::{
    data::{
        self, utils::sanitize_html_string, Album, AlbumType, Artist, ArtistAlbums, ArtistInfo,
        ArtistLink, ArtistStats, AudioAnalysis, Cached, DatePrecision, Episode, EpisodeId,
        EpisodeLink, Image, MixedView, Nav, Page, Playlist, PublicUser, Range, Recommendations,
        RecommendationsRequest, SearchResults, SearchTopic, Show, SpotifyUrl, Track, TrackLines,
        UserProfile,
    },
    error::Error,
    ui::credits::TrackCredits,
};

use super::{cache::WebApiCache, local::LocalTrackManager};
use sanitize_html::{rules::predefined::DEFAULT, sanitize_str};

pub struct WebApi {
    agent: Agent,
    cache: WebApiCache,
    local_track_manager: Mutex<LocalTrackManager>,
    paginated_limit: usize,
    webapi_token: Mutex<Option<WebApiToken>>,
    webapi_client_id: Mutex<Option<String>>,
    // First-party session credentials, used for `api-partner.spotify.com`
    // (pathfinder GraphQL) calls, which reject the Web API OAuth token.
    session: Mutex<Option<SessionService>>,
    login5: Login5,
    client_token_provider: ClientTokenProviderHandle,
}

impl WebApi {
    pub fn new(
        proxy_url: Option<&str>,
        cache_base: Option<PathBuf>,
        paginated_limit: usize,
    ) -> Self {
        let mut agent = Agent::config_builder().timeout_global(Some(Duration::from_secs(5)));
        if let Some(proxy_url) = proxy_url {
            let proxy = ureq::Proxy::new(proxy_url).ok();
            agent = agent.proxy(proxy);
        }
        let client_token_provider = ClientTokenProvider::new_shared(proxy_url);
        Self {
            agent: agent.build().into(),
            cache: WebApiCache::new(cache_base),
            local_track_manager: Mutex::new(LocalTrackManager::new()),
            paginated_limit,
            webapi_token: Mutex::new(None),
            webapi_client_id: Mutex::new(None),
            session: Mutex::new(None),
            login5: Login5::new(Some(Arc::clone(&client_token_provider)), proxy_url),
            client_token_provider,
        }
    }

    // Similar to how librespot does this https://github.com/librespot-org/librespot/blob/dev/core/src/version.rs
    fn user_agent() -> String {
        let platform = match OS {
            "macos" => "OSX",
            "windows" => "Win32",
            _ => "Linux",
        };
        format!(
            "Spotify/{} {}/0 (psst/{})",
            SPOTIFY_SEMANTIC_VERSION,
            platform,
            env!("CARGO_PKG_VERSION")
        )
    }

    /// Update the cached Web API token and client ID (called from GUI on config changes).
    pub fn set_webapi_credentials(&self, client_id: Option<String>, token: Option<WebApiToken>) {
        *self.webapi_client_id.lock() = client_id;
        *self.webapi_token.lock() = token;
    }

    /// Install the authenticated core session, used to mint the first-party
    /// tokens that `api-partner.spotify.com` requires.
    pub fn set_session(&self, session: SessionService) {
        *self.session.lock() = Some(session);
    }

    /// Mint the `(bearer, client-token)` pair accepted by `api-partner`.  Uses
    /// the Login5 access token from the core session plus a protobuf client
    /// token — the same credentials the web player and `Cdn` use.
    fn partner_token(&self) -> Result<(String, String), Error> {
        let session = self
            .session
            .lock()
            .clone()
            .ok_or_else(|| Error::WebApiError("No active session for api-partner".to_string()))?;
        let access_token = self
            .login5
            .get_access_token(&session)
            .map_err(|err| Error::WebApiError(err.to_string()))?;
        let client_token = self
            .client_token_provider
            .get()
            .map_err(|err| Error::WebApiError(err.to_string()))?;
        Ok((access_token.access_token, client_token))
    }

    fn access_token(&self) -> Result<String, Error> {
        let mut token_guard = self.webapi_token.lock();
        if let Some(ref token) = *token_guard {
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }
            // Try to refresh
            let client_id_guard = self.webapi_client_id.lock();
            if let (Some(ref client_id), Some(ref refresh_token)) =
                (&*client_id_guard, &token.refresh_token)
            {
                log::info!("Web API token expired, attempting refresh...");
                match oauth::refresh_webapi_token(client_id, refresh_token) {
                    Ok(new_token) => {
                        let access_token = new_token.access_token.clone();
                        // NOTE: only updates the in-memory cache. The durable
                        // copy in config.json is refreshed by Config on the
                        // next save; if Spotify rotates the refresh token and
                        // the process exits before then, the user must re-login.
                        *token_guard = Some(new_token);
                        return Ok(access_token);
                    }
                    Err(e) => {
                        log::error!("Failed to refresh Web API token: {e}");
                    }
                }
            }
        }
        Err(Error::WebApiError(
            "No valid Web API token available. Did you enable Web API for the Spotify Developer Client?".to_string(),
        ))
    }

    fn request(&self, request: &RequestBuilder) -> Result<Response<Body>, Error> {
        // `api-partner.spotify.com` rejects the Web API OAuth token, so those
        // requests carry the first-party Login5 bearer + client-token instead.
        let (token, client_token) = if request.partner_auth {
            let (bearer, client_token) = self.partner_token()?;
            (bearer, Some(client_token))
        } else {
            (self.access_token()?, None)
        };
        let url = request.build();

        fn configure_request<B>(
            req_builder: ureq::RequestBuilder<B>,
            token: &str,
            client_token: Option<&str>,
            headers: &HashMap<String, String>,
        ) -> ureq::RequestBuilder<B> {
            let mut req = req_builder.header("Authorization", &format!("Bearer {token}"));
            if let Some(client_token) = client_token {
                req = req.header("client-token", client_token);
            }
            headers.iter().fold(
                req, |current_req, (k, v)| current_req.header(k, v)
            )
        }

        let ct = client_token.as_deref();
        let headers = request.get_headers();
        match request.get_method() {
            Method::Get => configure_request(self.agent.get(&url), &token, ct, headers)
                .call()
                .map_err(|err| Error::WebApiError(err.to_string())),
            Method::Post => configure_request(self.agent.post(&url), &token, ct, headers)
                .send_json(request.get_body())
                .map_err(|err| Error::WebApiError(err.to_string())),
            Method::Put => configure_request(self.agent.put(&url), &token, ct, headers)
                .send_json(request.get_body())
                .map_err(|err| Error::WebApiError(err.to_string())),
            Method::Delete => configure_request(self.agent.delete(&url), &token, ct, headers)
                .force_send_body()
                .send_json(request.get_body())
                .map_err(|err| Error::WebApiError(err.to_string())),
        }
    }

    fn with_retry(f: impl Fn() -> Result<Response<Body>, Error>) -> Result<Response<Body>, Error> {
        loop {
            let response = f()?;
            match response.status() {
                StatusCode::TOO_MANY_REQUESTS => {
                    let retry_after_secs = response
                        .headers()
                        .get("Retry-After")
                        .and_then(|secs| secs.to_str().ok());
                    let secs = retry_after_secs.unwrap_or("2").parse::<u64>().unwrap_or(2);
                    thread::sleep(Duration::from_secs(secs));
                }
                _ => {
                    break Ok(response);
                }
            }
        }
    }

    /// Send a request with an empty JSON object, throw away the response body.
    /// Use for POST/PUT/DELETE requests.
    fn send_empty_json(&self, request: &RequestBuilder) -> Result<(), Error> {
        Self::with_retry(|| self.request(request)).map(|_| ())
    }

    /// Send a request and return the deserialized JSON body.  Use for GET
    /// requests.
    fn load<T: DeserializeOwned>(&self, request: &RequestBuilder) -> Result<T, Error> {
        let mut response = Self::with_retry(|| self.request(request))?;
        response
            .body_mut()
            .read_json()
            .map_err(|err| Error::WebApiError(err.to_string()))
    }

    /// Send a request using `self.load()`, but only if it isn't already present
    /// in cache.
    fn load_cached<T: Data + DeserializeOwned>(
        &self,
        request: &RequestBuilder,
        bucket: &str,
        key: &str,
    ) -> Result<Cached<T>, Error> {
        if let Some(file) = self.cache.get(bucket, key) {
            let cached_at = file.metadata()?.modified()?;
            let value = serde_json::from_reader(file)?;
            Ok(Cached::new(value, cached_at))
        } else {
            let response = Self::with_retry(|| self.request(request))?;
            let body = {
                let mut reader = response.into_body().into_reader();
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
        request: &RequestBuilder,
        mut func: impl FnMut(Page<T>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        // TODO: Some result sets, like very long playlists and saved tracks/albums can
        // be very big.  Implement virtualized scrolling and lazy-loading of results.
        let mut limit = 50;
        let mut offset = 0;
        loop {
            let req = request
                .clone()
                .query("limit".to_string(), limit.to_string())
                .query("offset".to_string(), offset.to_string());
            let page: Page<T> = self.load(&req)?;

            let page_total = page.total;
            let page_offset = page.offset;
            let page_limit = page.limit;
            func(page)?;

            if page_total > offset && offset < self.paginated_limit {
                limit = page_limit;
                offset = page_offset + page_limit;
            } else {
                break Ok(());
            }
        }
    }

    /// Very similar to `for_all_pages`, but only returns a certain number of results
    fn for_some_pages<T: DeserializeOwned + Clone>(
        &self,
        request: &RequestBuilder,
        lim: usize,
        mut func: impl FnMut(Page<T>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut limit = 50;
        let mut offset = 0;
        if lim < limit {
            limit = lim;
            let req = request
                .clone()
                .query("limit".to_string(), limit.to_string())
                .query("offset".to_string(), offset.to_string());

            let page: Page<T> = self.load(&req)?;

            func(page)?;
        } else {
            loop {
                let req = request
                    .clone()
                    .query("limit".to_string(), limit.to_string())
                    .query("offset".to_string(), offset.to_string());

                let page: Page<T> = self.load(&req)?;

                let page_total = limit / lim;
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
        request: &RequestBuilder,
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
        request: &RequestBuilder,
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
            log::error!("failed to read local tracks: {err}");
        }
    }

    fn load_and_return_home_section(&self, request: &RequestBuilder) -> Result<MixedView, Error> {
        #[derive(Deserialize)]
        pub struct Welcome {
            data: WelcomeData,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct WelcomeData {
            home_sections: HomeSections,
        }

        #[derive(Deserialize)]
        pub struct HomeSections {
            sections: Vec<Section>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Section {
            data: SectionData,
            section_items: SectionItems,
        }

        #[derive(Deserialize)]
        pub struct SectionData {
            title: Title,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Title {
            text: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct SectionItems {
            items: Vec<Item>,
        }

        #[derive(Deserialize)]
        pub struct Item {
            content: Content,
        }

        #[derive(Deserialize)]
        pub struct Content {
            data: ContentData,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct ContentData {
            #[serde(rename = "__typename")]
            typename: DataTypename,
            name: Option<String>,
            uri: Option<String>,

            // Playlist-specific fields
            attributes: Option<Vec<Attribute>>,
            description: Option<String>,
            images: Option<Images>,
            owner_v2: Option<OwnerV2>,

            // Artist-specific fields
            artists: Option<Artists>,
            profile: Option<Profile>,
            visuals: Option<Visuals>,

            // Show-specific fields
            cover_art: Option<CoverArt>,
            publisher: Option<Publisher>,
            total_episodes: Option<usize>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Visuals {
            avatar_image: CoverArt,
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
        #[serde(rename_all = "camelCase")]
        pub struct CoverArt {
            sources: Vec<Source>,
        }

        #[derive(Deserialize)]
        pub struct Source {
            url: String,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        pub enum MediaType {
            #[serde(rename = "AUDIO")]
            Audio,
            #[serde(rename = "MIXED")]
            Mixed,
        }

        #[derive(Deserialize)]
        pub struct Publisher {
            name: String,
        }

        #[derive(Deserialize)]
        pub enum DataTypename {
            Podcast,
            Playlist,
            Artist,
            Album,
            NotFound,
        }

        #[derive(Deserialize)]
        pub struct Images {
            items: Vec<ImagesItem>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct ImagesItem {
            sources: Vec<Source>,
        }

        #[derive(Deserialize)]
        pub struct OwnerV2 {
            data: OwnerV2Data,
        }

        #[derive(Deserialize)]
        pub struct OwnerV2Data {
            #[serde(rename = "__typename")]
            name: String,
        }

        // Extract the playlists
        let result: Welcome = match self.load(request) {
            Ok(res) => res,
            Err(e) => {
                info!("Error loading home section: {e}");
                return Err(e);
            }
        };

        let mut title: Arc<str> = Arc::from("");
        let mut playlist: Vector<Playlist> = Vector::new();
        let mut album: Vector<Arc<Album>> = Vector::new();
        let mut artist: Vector<Artist> = Vector::new();
        let mut show: Vector<Arc<Show>> = Vector::new();

        result
            .data
            .home_sections
            .sections
            .iter()
            .for_each(|section| {
                title = section.data.title.text.clone().into();

                section.section_items.items.iter().for_each(|item| {
                    let Some(uri) = &item.content.data.uri else {
                        return;
                    };
                    let id = uri.split(':').next_back().unwrap_or("").to_string();

                    match item.content.data.typename {
                        DataTypename::Playlist => {
                            playlist.push_back(Playlist {
                                id: id.into(),
                                name: Arc::from(item.content.data.name.clone().unwrap()),
                                images: Some(item.content.data.images.as_ref().map_or_else(
                                    Vector::new,
                                    |images| {
                                        images
                                            .items
                                            .iter()
                                            .map(|img| data::utils::Image {
                                                url: Arc::from(
                                                    img.sources
                                                        .first()
                                                        .map(|s| s.url.as_str())
                                                        .unwrap_or_default(),
                                                ),
                                                width: None,
                                                height: None,
                                            })
                                            .collect()
                                    },
                                )),
                                description: {
                                    let desc = sanitize_html_string(
                                        item.content
                                            .data
                                            .description
                                            .as_deref()
                                            .unwrap_or_default(),
                                    );

                                    // This is roughly 3 lines of description, truncated if too long
                                    if desc.chars().count() > 55 {
                                        Arc::from(desc.chars().take(52).collect::<String>() + "...")
                                    } else {
                                        desc
                                    }
                                },
                                track_count: item.content.data.attributes.as_ref().and_then(
                                    |attrs| {
                                        attrs
                                            .iter()
                                            .find(|attr| attr.key == "track_count")
                                            .and_then(|attr| attr.value.parse().ok())
                                    },
                                ),
                                owner: PublicUser {
                                    id: Arc::from(""),
                                    display_name: Arc::from(
                                        item.content
                                            .data
                                            .owner_v2
                                            .as_ref()
                                            .map(|owner| owner.data.name.as_str())
                                            .unwrap_or_default(),
                                    ),
                                },
                                collaborative: false,
                                public: None,
                            });
                        }
                        DataTypename::Artist => artist.push_back(Artist {
                            id: id.into(),
                            name: Arc::from(
                                item.content.data.profile.as_ref().unwrap().name.clone(),
                            ),
                            images: item.content.data.visuals.as_ref().map_or_else(
                                Vector::new,
                                |images| {
                                    images
                                        .avatar_image
                                        .sources
                                        .iter()
                                        .map(|img| data::utils::Image {
                                            url: Arc::from(img.url.as_str()),
                                            width: None,
                                            height: None,
                                        })
                                        .collect()
                                },
                            ),
                        }),
                        DataTypename::Album => album.push_back(Arc::new(Album {
                            id: id.into(),
                            name: Arc::from(item.content.data.name.clone().unwrap()),
                            album_type: AlbumType::Album,
                            images: item.content.data.cover_art.as_ref().map_or_else(
                                Vector::new,
                                |images| {
                                    images
                                        .sources
                                        .iter()
                                        .map(|src| data::utils::Image {
                                            url: Arc::from(src.url.clone()),
                                            width: None,
                                            height: None,
                                        })
                                        .collect()
                                },
                            ),
                            artists: item.content.data.artists.as_ref().map_or_else(
                                Vector::new,
                                |artists| {
                                    artists
                                        .items
                                        .iter()
                                        .map(|artist| ArtistLink {
                                            id: Arc::from(
                                                artist
                                                    .uri
                                                    .split(':')
                                                    .next_back()
                                                    .unwrap_or("")
                                                    .to_string(),
                                            ),
                                            name: Arc::from(artist.profile.name.clone()),
                                        })
                                        .collect()
                                },
                            ),
                            copyrights: Vector::new(),
                            label: "".into(),
                            tracks: Vector::new(),
                            release_date: None,
                            release_date_precision: None,
                        })),
                        DataTypename::Podcast => show.push_back(Arc::new(Show {
                            id: id.into(),
                            name: Arc::from(item.content.data.name.clone().unwrap()),
                            images: item.content.data.cover_art.as_ref().map_or_else(
                                Vector::new,
                                |images| {
                                    images
                                        .sources
                                        .iter()
                                        .map(|src| data::utils::Image {
                                            url: Arc::from(src.url.clone()),
                                            width: None,
                                            height: None,
                                        })
                                        .collect()
                                },
                            ),
                            publisher: Arc::from(
                                item.content
                                    .data
                                    .publisher
                                    .as_ref()
                                    .map(|p| p.name.as_str())
                                    .unwrap_or(""),
                            ),
                            description: Arc::from(
                                item.content.data.description.as_deref().unwrap_or(""),
                            ),
                            total_episodes: item.content.data.total_episodes,
                        })),
                        // For section items we don't cover yet
                        DataTypename::NotFound => {}
                    }
                });
            });

        Ok(MixedView {
            title,
            playlists: playlist,
            artists: artist,
            albums: album,
            shows: show,
        })
    }
}

static GLOBAL_WEBAPI: OnceLock<Arc<WebApi>> = OnceLock::new();

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
        let result = self.load(&RequestBuilder::new("v1/me".to_string(), Method::Get, None))?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-users-top-artists-and-tracks
    pub fn get_user_top_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        let request = &RequestBuilder::new("v1/me/top/tracks".to_string(), Method::Get, None)
            .query("market", "from_token");
        let result: Vector<Arc<Track>> = self.load_some_pages(request, 30)?;

        Ok(result)
    }

    pub fn get_user_top_artist(&self) -> Result<Vector<Artist>, Error> {
        #[derive(Clone, Data, Deserialize)]
        #[allow(dead_code)]
        struct Artists {
            artists: Artist,
        }
        let request = &RequestBuilder::new("v1/me/top/artists", Method::Get, None);

        Ok(self
            .load_some_pages(request, 10)?
            .into_iter()
            .map(|item: Artist| item)
            .collect())
    }
}

/// Persisted-query hashes for the pathfinder discography operations, extracted
/// from the web player bundle.  Albums, singles and compilations all share one
/// document (selected by `operationName`); appears-on is a separate one.
const DISCOGRAPHY_HASH: &str = "5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599";
const APPEARS_ON_HASH: &str = "9a4bb7a20d6720fe52d7b47bc001cfa91940ddf5e7113761460b4a288d18a4c1";

// Shape of the `queryArtistDiscography*` / `queryArtistAppearsOn` responses.
// Each operation populates a different field, so all are optional and the
// caller selects the one it wants.
#[derive(Deserialize)]
struct DiscographyResponse {
    data: DiscographyData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscographyData {
    artist_union: DiscographyUnion,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscographyUnion {
    #[serde(default)]
    discography: Discography,
    #[serde(default)]
    related_content: RelatedContent,
}

#[derive(Default, Deserialize)]
struct Discography {
    #[serde(default)]
    albums: Option<DiscographySection>,
    #[serde(default)]
    singles: Option<DiscographySection>,
    #[serde(default)]
    compilations: Option<DiscographySection>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelatedContent {
    #[serde(default)]
    appears_on: Option<DiscographySection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscographySection {
    #[serde(default)]
    total_count: i64,
    #[serde(default)]
    items: Vec<ReleaseHolder>,
}

#[derive(Deserialize)]
struct ReleaseHolder {
    releases: Releases,
}

#[derive(Deserialize)]
struct Releases {
    #[serde(default)]
    items: Vec<Release>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Release {
    id: Arc<str>,
    name: Arc<str>,
    #[serde(default)]
    cover_art: CoverArt,
    #[serde(default)]
    date: Option<ReleaseDate>,
    #[serde(rename = "type", default)]
    release_type: Option<String>,
    #[serde(default)]
    artists: ReleaseArtists,
}

#[derive(Default, Deserialize)]
struct CoverArt {
    #[serde(default)]
    sources: Vector<Image>,
}

#[derive(Deserialize)]
struct ReleaseDate {
    #[serde(default)]
    year: Option<i32>,
    #[serde(default)]
    month: Option<u8>,
    #[serde(default)]
    day: Option<u8>,
    #[serde(default)]
    precision: Option<String>,
}

#[derive(Default, Deserialize)]
struct ReleaseArtists {
    #[serde(default)]
    items: Vec<ReleaseArtist>,
}

#[derive(Deserialize)]
struct ReleaseArtist {
    uri: String,
    profile: ReleaseArtistProfile,
}

#[derive(Deserialize)]
struct ReleaseArtistProfile {
    name: Arc<str>,
}

impl Release {
    fn to_album(&self, default_type: AlbumType) -> Album {
        let album_type = match self.release_type.as_deref() {
            Some("ALBUM") => AlbumType::Album,
            Some("SINGLE") => AlbumType::Single,
            Some("COMPILATION") => AlbumType::Compilation,
            _ => default_type,
        };
        let (release_date, release_date_precision) = self
            .date
            .as_ref()
            .map(ReleaseDate::to_date)
            .unwrap_or((None, None));

        Album {
            id: self.id.clone(),
            name: self.name.clone(),
            album_type,
            images: self.cover_art.sources.clone(),
            artists: self
                .artists
                .items
                .iter()
                .map(|artist| ArtistLink {
                    id: artist.uri.rsplit(':').next().unwrap_or_default().into(),
                    name: artist.profile.name.clone(),
                })
                .collect(),
            copyrights: Vector::new(),
            label: "".into(),
            tracks: Vector::new(),
            release_date,
            release_date_precision,
        }
    }
}

impl ReleaseDate {
    fn to_date(&self) -> (Option<Date>, Option<DatePrecision>) {
        let precision = match self.precision.as_deref() {
            Some("DAY") => Some(DatePrecision::Day),
            Some("MONTH") => Some(DatePrecision::Month),
            Some("YEAR") => Some(DatePrecision::Year),
            _ => None,
        };
        // Only the year is required; month/day default to 1 so we can still form
        // a valid `Date` even when precision is coarser than a full day.
        let date = self.year.and_then(|year| {
            let month = self
                .month
                .and_then(|month| Month::try_from(month).ok())
                .unwrap_or(Month::January);
            let day = self.day.filter(|day| *day >= 1).unwrap_or(1);
            Date::from_calendar_date(year, month, day).ok()
        });
        (date, precision)
    }
}

/// Artist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-artist/
    pub fn get_artist(&self, id: &str) -> Result<Artist, Error> {
        let request = &RequestBuilder::new(format!("v1/artists/{id}"), Method::Get, None);
        let result = self.load_cached(request, "artist", id)?;
        Ok(result.data)
    }

    // Albums, singles, compilations and appears-on each come from a separate
    // pathfinder discography operation on `api-partner.spotify.com`, paginated
    // independently.
    pub fn get_artist_albums(&self, id: &str) -> Result<ArtistAlbums, Error> {
        Ok(ArtistAlbums {
            albums: self.artist_releases(
                id,
                "queryArtistDiscographyAlbums",
                DISCOGRAPHY_HASH,
                true,
                AlbumType::Album,
                |union| union.discography.albums.as_ref(),
            )?,
            singles: self.artist_releases(
                id,
                "queryArtistDiscographySingles",
                DISCOGRAPHY_HASH,
                true,
                AlbumType::Single,
                |union| union.discography.singles.as_ref(),
            )?,
            compilations: self.artist_releases(
                id,
                "queryArtistDiscographyCompilations",
                DISCOGRAPHY_HASH,
                true,
                AlbumType::Compilation,
                |union| union.discography.compilations.as_ref(),
            )?,
            appears_on: self.artist_releases(
                id,
                "queryArtistAppearsOn",
                APPEARS_ON_HASH,
                false,
                AlbumType::Album,
                |union| union.related_content.appears_on.as_ref(),
            )?,
        })
    }

    /// Fetch one discography section, paging until `totalCount` is exhausted.
    /// `select` picks the section out of the response, `default_type` fills in
    /// releases that omit their `type`, and `order` sends `DATE_DESC` (which the
    /// appears-on operation rejects).
    fn artist_releases(
        &self,
        id: &str,
        operation: &str,
        hash: &str,
        order: bool,
        default_type: AlbumType,
        select: impl Fn(&DiscographyUnion) -> Option<&DiscographySection>,
    ) -> Result<Vector<Arc<Album>>, Error> {
        const PAGE: usize = 50;

        let mut releases = Vector::new();
        let mut offset = 0;
        loop {
            let mut variables = json!({
                "uri": format!("spotify:artist:{id}"),
                "offset": offset,
                "limit": PAGE,
            });
            if order {
                variables["order"] = json!("DATE_DESC");
            }
            let json = json!({
                "operationName": operation,
                "variables": variables,
                "extensions": {
                    "persistedQuery": { "version": 1, "sha256Hash": hash }
                },
            });
            let request =
                &RequestBuilder::new("pathfinder/v2/query".to_string(), Method::Post, Some(json))
                    .set_base_uri("api-partner.spotify.com")
                    .header("User-Agent", Self::user_agent())
                    .partner_auth();

            let response: DiscographyResponse = self.load(request)?;
            let union = response.data.artist_union;
            let Some(section) = select(&union) else {
                break;
            };

            for holder in &section.items {
                if let Some(release) = holder.releases.items.first() {
                    releases.push_back(Arc::new(release.to_album(default_type.clone())));
                }
            }

            offset += PAGE;
            if section.items.is_empty() || offset >= section.total_count.max(0) as usize {
                break;
            }
        }
        Ok(releases)
    }

    fn artist_overview_request(&self, id: &str) -> RequestBuilder {
        let json = json!({
            "extensions": {
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": "1ac33ddab5d39a3a9c27802774e6d78b9405cc188c6f75aed007df2a32737c72"
                }
            },
            "operationName": "queryArtistOverview",
            "variables": {
                "locale": "",
                "uri": format!("spotify:artist:{id}"),
            },
        });
        RequestBuilder::new("pathfinder/v2/query".to_string(), Method::Post, Some(json))
            .set_base_uri("api-partner.spotify.com")
            .header("User-Agent", Self::user_agent())
            .partner_auth()
    }

    // Related artists, sourced from `artistUnion.relatedContent` in the same
    // `queryArtistOverview` response
    pub fn get_related_artists(&self, id: &str) -> Result<Cached<Vector<Artist>>, Error> {
        #[derive(Clone, Data, Deserialize)]
        struct Welcome {
            data: WelcomeData,
        }
        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WelcomeData {
            artist_union: ArtistUnion,
        }
        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ArtistUnion {
            #[serde(default)]
            related_content: Option<RelatedContent>,
        }
        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RelatedContent {
            #[serde(default)]
            related_artists: RelatedArtists,
        }
        #[derive(Clone, Data, Default, Deserialize)]
        struct RelatedArtists {
            #[serde(default)]
            items: Vector<RelatedArtist>,
        }
        #[derive(Clone, Data, Deserialize)]
        struct RelatedArtist {
            id: Arc<str>,
            profile: RelatedProfile,
            #[serde(default)]
            visuals: Option<RelatedVisuals>,
        }
        #[derive(Clone, Data, Deserialize)]
        struct RelatedProfile {
            name: Arc<str>,
        }
        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RelatedVisuals {
            #[serde(default)]
            avatar_image: Option<RelatedAvatar>,
        }
        #[derive(Clone, Data, Deserialize)]
        struct RelatedAvatar {
            #[serde(default)]
            sources: Vector<Image>,
        }

        let request = &self.artist_overview_request(id);
        let result: Cached<Welcome> = self.load_cached(request, "related-artists", id)?;
        Ok(result.map(|welcome| {
            welcome
                .data
                .artist_union
                .related_content
                .map(|content| content.related_artists.items)
                .unwrap_or_default()
                .into_iter()
                .map(|artist| Artist {
                    id: artist.id,
                    name: artist.profile.name,
                    images: artist
                        .visuals
                        .and_then(|visuals| visuals.avatar_image)
                        .map(|image| image.sources)
                        .unwrap_or_default(),
                })
                .collect()
        }))
    }

    // Artist bio, stats, image and external links from the pathfinder GraphQL
    // `queryArtistOverview` operation (there is no REST equivalent).
    pub fn get_artist_info(&self, id: &str) -> Result<ArtistInfo, Error> {
        #[derive(Clone, Data, Deserialize)]
        struct Welcome {
            data: WelcomeData,
        }
        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WelcomeData {
            artist_union: ArtistUnion,
        }
        // Sparse artists (no monthly listeners yet) omit `profile`, `stats` and
        // `visuals` entirely, so every branch must tolerate their absence.
        #[derive(Clone, Data, Deserialize)]
        struct ArtistUnion {
            #[serde(default)]
            profile: Option<Profile>,
            #[serde(default)]
            stats: Option<Stats>,
            #[serde(default)]
            visuals: Option<Visuals>,
        }
        #[derive(Clone, Data, Default, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Profile {
            #[serde(default)]
            biography: Option<Biography>,
            #[serde(default)]
            external_links: ExternalLinks,
        }
        #[derive(Clone, Data, Deserialize)]
        struct Biography {
            #[serde(default)]
            text: Option<String>,
        }
        #[derive(Clone, Data, Default, Deserialize)]
        struct ExternalLinks {
            #[serde(default)]
            items: Vector<ExternalLinksItem>,
        }
        #[derive(Clone, Data, Deserialize)]
        struct ExternalLinksItem {
            url: String,
        }
        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Visuals {
            #[serde(default)]
            avatar_image: Option<AvatarImage>,
        }
        #[derive(Clone, Data, Deserialize)]
        struct AvatarImage {
            #[serde(default)]
            sources: Vector<Image>,
        }
        // Individual counters can also be null even when `stats` is present.
        #[derive(Clone, Data, Default, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Stats {
            #[serde(default)]
            followers: Option<i64>,
            #[serde(default)]
            monthly_listeners: Option<i64>,
            #[serde(default)]
            world_rank: Option<i64>,
        }

        let request = &self.artist_overview_request(id);
        let result: Cached<Welcome> = self.load_cached(request, "artist-info", id)?;
        let union = result.data.data.artist_union;

        let main_image = union
            .visuals
            .and_then(|visuals| visuals.avatar_image)
            .and_then(|image| image.sources.into_iter().next())
            .map(|source| source.url.clone())
            .unwrap_or_else(|| Arc::from(""));

        let profile = union.profile.unwrap_or_default();
        let stats = union.stats.unwrap_or_default();

        let bio = profile
            .biography
            .and_then(|biography| biography.text)
            .map(|text| {
                let sanitized = sanitize_str(&DEFAULT, &text).unwrap_or_default();
                sanitized.replace("&amp;", "&")
            })
            .unwrap_or_default();

        let artist_links = profile
            .external_links
            .items
            .into_iter()
            .map(|link| link.url)
            .collect();

        Ok(ArtistInfo {
            main_image,
            stats: ArtistStats {
                followers: stats.followers.unwrap_or(0),
                monthly_listeners: stats.monthly_listeners.unwrap_or(0),
                world_rank: stats.world_rank.unwrap_or(0),
            },
            bio,
            artist_links,
        })
    }
}

/// Album endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-an-album/
    pub fn get_album(&self, id: &str) -> Result<Cached<Arc<Album>>, Error> {
        let request = &RequestBuilder::new(format!("v1/albums/{id}"), Method::Get, None)
            .query("market", "from_token");
        let result = self.load_cached(request, "album", id)?;
        Ok(result)
    }
}

/// Show endpoints. (Podcasts)
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-a-show/Add commentMore actions
    pub fn get_show(&self, id: &str) -> Result<Cached<Arc<Show>>, Error> {
        let request = &RequestBuilder::new(format!("v1/shows/{id}"), Method::Get, None)
            .query("market", "from_token");

        let result = self.load_cached(request, "show", id)?;

        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-multiple-episodes
    pub fn get_episodes(
        &self,
        ids: impl IntoIterator<Item = EpisodeId>,
    ) -> Result<Vector<Arc<Episode>>, Error> {
        #[derive(Deserialize)]
        struct Episodes {
            episodes: Vector<Arc<Episode>>,
        }

        let request = &RequestBuilder::new("v1/episodes", Method::Get, None)
            .query("ids", ids.into_iter().map(|id| id.0.to_base62()).join(","))
            .query("market", "from_token");
        let result: Episodes = self.load(request)?;
        Ok(result.episodes)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-a-shows-episodes
    pub fn get_show_episodes(&self, id: &str) -> Result<Vector<Arc<Episode>>, Error> {
        let request = &RequestBuilder::new(format!("v1/shows/{id}/episodes"), Method::Get, None)
            .query("market", "from_token");

        let mut results = Vector::new();
        self.for_all_pages(request, |page: Page<Option<EpisodeLink>>| {
            if !page.items.is_empty() {
                let ids = page
                    .items
                    .into_iter()
                    .filter_map(|link| link.map(|link| link.id));
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
        let request = &RequestBuilder::new(format!("v1/tracks/{id}"), Method::Get, None)
            .query("market", "from_token");
        self.load(request)
    }

    pub fn get_track_credits(&self, track_id: &str) -> Result<TrackCredits, Error> {
        let request = &RequestBuilder::new(
            format!("track-credits-view/v0/experimental/{track_id}/credits"),
            Method::Get,
            None,
        )
        .set_base_uri("spclient.wg.spotify.com");
        let result: TrackCredits = self.load(request)?;
        Ok(result)
    }

    pub fn get_lyrics(&self, track_id: String) -> Result<Vector<TrackLines>, Error> {
        #[derive(Default, Debug, Clone, PartialEq, Deserialize, Data)]
        #[serde(rename_all = "camelCase")]
        pub struct Root {
            pub lyrics: Lyrics,
        }

        #[derive(Default, Debug, Clone, PartialEq, Deserialize, Data)]
        #[serde(rename_all = "camelCase")]
        pub struct Lyrics {
            pub lines: Vector<TrackLines>,
            pub provider: String,
            pub provider_lyrics_id: String,
        }

        let request = &RequestBuilder::new(
            format!("color-lyrics/v2/track/{track_id}"),
            Method::Get,
            None,
        )
        .set_base_uri("spclient.wg.spotify.com")
        .query("format", "json")
        .query("vocalRemoval", "false")
        .query("market", "from_token")
        .header("app-platform", "WebPlayer");

        let lyrics: Cached<Root> = self.load_cached(request, "lyrics", &track_id)?;
        Ok(lyrics.data.lyrics.lines)
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

        let request =
            &RequestBuilder::new("v1/me/albums", Method::Get, None).query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedAlbum| item.album)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-to-library/
    pub fn save_album(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/library", Method::Put, None)
            .set_body(Some(json!({"uris": [format!("spotify:album:{id}")]})));
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-from-library/
    pub fn unsave_album(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/library", Method::Delete, None)
            .set_body(Some(json!({"uris": [format!("spotify:album:{id}")]})));
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-users-saved-tracks/
    pub fn get_saved_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedTrack {
            track: Arc<Track>,
        }
        let request =
            &RequestBuilder::new("v1/me/tracks", Method::Get, None).query("market", "from_token");
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

        let request =
            &RequestBuilder::new("v1/me/shows", Method::Get, None).query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedShow| item.show)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-to-library/
    pub fn save_track(&self, id: &str) -> Result<(), Error> {
        // Spotify's /v1/me/tracks takes the base62 ids as a query param, not a
        // uris body.
        let request = &RequestBuilder::new("v1/me/tracks", Method::Put, None).query("ids", id);
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-from-library/
    pub fn unsave_track(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/tracks", Method::Delete, None).query("ids", id);
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-to-library/
    pub fn save_show(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/library", Method::Put, None)
            .set_body(Some(json!({"uris": [format!("spotify:show:{id}")]})));
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-from-library/
    pub fn unsave_show(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/library", Method::Delete, None)
            .set_body(Some(json!({"uris": [format!("spotify:show:{id}")]})));
        self.send_empty_json(request)
    }
}

/// View endpoints.
impl WebApi {
    pub fn get_user_info(&self) -> Result<(String, String), Error> {
        #[derive(Deserialize, Clone, Data)]
        struct User {
            region: String,
            timezone: String,
        }
        let token = self.access_token()?;

        let request = &RequestBuilder::new("json".to_string(), Method::Get, None)
            .set_protocol("http")
            .set_base_uri("ip-api.com")
            .query("fields", "260")
            .header("Authorization", format!("Bearer {token}"));

        let result: Cached<User> = self.load_cached(request, "user-info", "usrinfo")?;

        Ok((result.data.region, result.data.timezone))
    }

    pub fn get_section(&self, section_uri: &str) -> Result<MixedView, Error> {
        let (country, time_zone) = self.get_user_info()?;
        let access_token = self.access_token()?;

        let json = json!({
            "extensions": {
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": "eb3fba2d388cf4fc4d696b1757a58584e9538a3b515ea742e9cc9465807340be"
                }
            },
            "operationName": "homeSection",
            "variables":  {
                "sectionItemsLimit": 20,
                "sectionItemsOffset": 0,
                "sp_t": access_token,
                "timeZone": time_zone,
                "country": country,
                "uri": section_uri
            },
        });

        let request =
            &RequestBuilder::new("pathfinder/v2/query".to_string(), Method::Post, Some(json))
                .set_base_uri("api-partner.spotify.com")
                .header("User-Agent", Self::user_agent())
                .partner_auth();

        // Extract the playlists
        self.load_and_return_home_section(request)
    }

    pub fn get_made_for_you(&self) -> Result<MixedView, Error> {
        // 0JQ5DAUnp4wcj0bCb3wh3S -> Made for you
        self.get_section("spotify:section:0JQ5DAUnp4wcj0bCb3wh3S")
    }

    pub fn get_top_mixes(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu89 -> Top mixes
        self.get_section("spotify:section:0JQ5DAnM3wGh0gz1MXnu89")
    }

    pub fn recommended_stations(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3R -> Recommended stations
        self.get_section("spotify:section:0JQ5DAnM3wGh0gz1MXnu3R")
    }

    pub fn uniquely_yours(&self) -> Result<MixedView, Error> {
        // 0JQ5DAUnp4wcj0bCb3wh3S -> Uniquely yours
        self.get_section("spotify:section:0JQ5DAUnp4wcj0bCb3wh3S")
    }

    pub fn best_of_artists(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3n -> Best of artists
        self.get_section("spotify:section:0JQ5DAnM3wGh0gz1MXnu3n")
    }

    // Need to make a mix of it!
    pub fn jump_back_in(&self) -> Result<MixedView, Error> {
        // 0JQ5DAIiKWzVFULQfUm85X -> Jump back in
        self.get_section("spotify:section:0JQ5DAIiKWzVFULQfUm85X")
    }

    // Shows
    pub fn your_shows(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3N -> Your shows
        self.get_section("spotify:section:0JQ5DAnM3wGh0gz1MXnu3N")
    }

    pub fn shows_that_you_might_like(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3P -> Shows that you might like
        self.get_section("spotify:section:0JQ5DAnM3wGh0gz1MXnu3P")
    }
}

/// Playlist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-a-list-of-current-users-playlists
    pub fn get_playlists(&self) -> Result<Vector<Playlist>, Error> {
        let request = &RequestBuilder::new("v1/me/playlists", Method::Get, None);
        let result: Vector<Playlist> = self.load_all_pages(request)?;
        Ok(result)
    }

    pub fn follow_playlist(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/library", Method::Put, None)
            .set_body(Some(json!({"uris": [format!("spotify:playlist:{id}")]})));
        self.send_empty_json(request)?;
        Ok(())
    }

    pub fn unfollow_playlist(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/library", Method::Delete, None)
            .set_body(Some(json!({"uris": [format!("spotify:playlist:{id}")]})));
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-playlist
    pub fn get_playlist(&self, id: &str) -> Result<Playlist, Error> {
        let request = &RequestBuilder::new(format!("v1/playlists/{id}"), Method::Get, None);
        let result: Playlist = self.load(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-playlist-items
    pub fn get_playlist_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct PlaylistItem {
            #[serde(default)]
            item: Option<OptionalTrack>,
            #[serde(default)]
            track: Option<OptionalTrack>,
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

        let request = &RequestBuilder::new(format!("v1/playlists/{id}/items"), Method::Get, None)
            .query("marker", "from_token")
            .query("additional_types", "track");

        let result: Vector<PlaylistItem> = self.load_all_pages(request)?;

        let local_track_manager = self.local_track_manager.lock();

        Ok(result
            .into_iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let track_source = item.item.or(item.track);
                let mut track = match track_source {
                    Some(OptionalTrack::Track(track)) => track,
                    Some(OptionalTrack::Json(json)) => {
                        local_track_manager.find_local_track(json)?
                    }
                    None => return None,
                };
                Arc::make_mut(&mut track).track_pos = index;
                Some(track)
            })
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/change-playlist-details
    pub fn change_playlist_details(&self, id: &str, name: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new(format!("v1/playlists/{id}"), Method::Put, None)
            .set_body(Some(json!({ "name": name })));
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/add-items-to-playlist
    pub fn add_track_to_playlist(&self, playlist_id: &str, track_uri: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new(
            format!("v1/playlists/{playlist_id}/items"),
            Method::Post,
            Some(json!({"uris": [track_uri]})),
        );
        self.request(request).map(|_| ())
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-playlist-items
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        track_uri: &str,
    ) -> Result<(), Error> {
        let request = &RequestBuilder::new(
            format!("v1/playlists/{playlist_id}/items"),
            Method::Delete,
            Some(json!({ "items": [{ "uri": track_uri }] })),
        );
        self.request(request).map(|_| ())
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
            // Spotify returns `null` items for region-blocked/removed playlists.
            playlists: Option<Page<Option<Playlist>>>,
            shows: Option<Page<Arc<Show>>>,
        }

        let encoded_query = urlencoding::encode(query);
        let type_query_param = topics.iter().map(SearchTopic::as_str).join(",");
        let request = &RequestBuilder::new("v1/search", Method::Get, None)
            .query("q", encoded_query)
            .query("type", &type_query_param)
            .query("limit", limit.to_string())
            .query("marker", "from_token");

        let result: ApiSearchResults = self.load(request)?;

        let artists = result.artists.map_or_else(Vector::new, |page| page.items);
        let albums = result.albums.map_or_else(Vector::new, |page| page.items);
        let tracks = result.tracks.map_or_else(Vector::new, |page| page.items);
        let playlists = result
            .playlists
            .map_or_else(Vector::new, |p| p.items.into_iter().flatten().collect());
        let shows = result.shows.map_or_else(Vector::new, |page| page.items);
        let topic = (topics.len() == 1).then_some(topics[0]);

        Ok(SearchResults {
            query: query.into(),
            topic,
            artists,
            albums,
            tracks,
            playlists,
            shows,
        })
    }

    pub fn load_spotify_link(&self, link: &SpotifyUrl) -> Result<Nav, Error> {
        let nav = match link {
            SpotifyUrl::Playlist(id) => Nav::PlaylistDetail(self.get_playlist(id)?.link()),
            SpotifyUrl::Artist(id) => Nav::ArtistDetail(self.get_artist(id)?.link()),
            SpotifyUrl::Album(id) => Nav::AlbumDetail(self.get_album(id)?.data.link(), None),
            SpotifyUrl::Show(id) => Nav::ShowDetail(self.get_show(id)?.data.link()),
            SpotifyUrl::Track(id) => {
                let track = self.get_track(id)?;
                let album = track.album.clone().ok_or_else(|| {
                    Error::WebApiError("Track was found but has no album".to_string())
                })?;
                Nav::AlbumDetail(album, Some(track.id))
            }
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

        let mut request = RequestBuilder::new("v1/recommendations", Method::Get, None)
            .query("marker", "from_token")
            .query("limit", "100")
            .query("seed_artists", &seed_artists)
            .query("seed_tracks", &seed_tracks);

        fn add_range_param(
            req: RequestBuilder,
            r: Range<impl ToString>,
            s: &str,
        ) -> RequestBuilder {
            let mut req = req;
            if let Some(v) = r.min {
                req = req.query(format!("min_{s}"), v.to_string());
            }
            if let Some(v) = r.max {
                req = req.query(format!("max_{s}"), v.to_string());
            }
            if let Some(v) = r.target {
                req = req.query(format!("target_{s}"), v.to_string());
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

        let mut result: Recommendations = self.load(&request)?;
        result.request = data;
        Ok(result)
    }
}

/// Track endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-audio-analysis/
    pub fn _get_audio_analysis(&self, track_id: &str) -> Result<AudioAnalysis, Error> {
        let request =
            &RequestBuilder::new(format!("v1/audio-analysis/{track_id}"), Method::Get, None);
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

        // Split the URI into its components
        let uri_clone = uri.clone();
        let parsed = url::Url::parse(&uri_clone).unwrap();

        let protocol = parsed.scheme();
        let base_uri = parsed.host_str().unwrap();
        let path = parsed.path().trim_start_matches('/');

        let mut queries = std::collections::HashMap::new();
        for (k, v) in parsed.query_pairs() {
            queries.insert(k.to_string(), v.to_string());
        }

        let request = RequestBuilder::new(path, Method::Get, None)
            .set_protocol(protocol)
            .set_base_uri(base_uri);

        let response = self.request(&request)?;
        let mut body = Vec::new();
        response.into_body().into_reader().read_to_end(&mut body)?;

        let format = match infer::get(body.as_slice()) {
            Some(kind) if kind.mime_type() == "image/jpeg" => Some(ImageFormat::Jpeg),
            Some(kind) if kind.mime_type() == "image/png" => Some(ImageFormat::Png),
            Some(kind) if kind.mime_type() == "image/webp" => Some(ImageFormat::WebP),
            _ => None,
        };

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

#[derive(Debug, Clone)]
enum Method {
    Post,
    Put,
    Delete,
    Get,
}

// Creating a new URI builder so aid in the creation of uris with extendable queries.
#[derive(Debug, Clone)]
struct RequestBuilder {
    protocol: String,
    base_uri: String,
    path: String,
    queries: HashMap<String, String>,
    headers: HashMap<String, String>,
    method: Method,
    body: Option<serde_json::Value>,
    // When set, authenticate with the first-party Login5 bearer + client-token
    // (for `api-partner.spotify.com`) instead of the Web API OAuth token.
    partner_auth: bool,
}

impl RequestBuilder {
    // By default, we use https and the api.spotify.com
    fn new(path: impl Display, method: Method, body: Option<serde_json::Value>) -> Self {
        Self {
            protocol: "https".to_string(),
            base_uri: "api.spotify.com".to_string(),
            path: path.to_string(),
            queries: HashMap::new(),
            headers: HashMap::new(),
            method,
            body,
            partner_auth: false,
        }
    }

    /// Authenticate this request with the first-party Login5 bearer +
    /// client-token, as required by `api-partner.spotify.com`.
    fn partner_auth(mut self) -> Self {
        self.partner_auth = true;
        self
    }

    fn query(mut self, key: impl Display, value: impl Display) -> Self {
        self.queries.insert(key.to_string(), value.to_string());
        self
    }

    fn header(mut self, key: impl Display, value: impl Display) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    fn set_protocol(mut self, protocol: impl Display) -> Self {
        self.protocol = protocol.to_string();
        self
    }
    fn get_headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
    fn get_body(&self) -> Option<&serde_json::Value> {
        self.body.as_ref()
    }
    fn set_body(mut self, body: Option<serde_json::Value>) -> Self {
        self.body = body;
        self
    }
    fn get_method(&self) -> &Method {
        &self.method
    }
    #[allow(dead_code)]
    fn set_method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }
    fn set_base_uri(mut self, url: impl Display) -> Self {
        self.base_uri = url.to_string();
        self
    }
    fn build(&self) -> String {
        let mut url = format!("{}://{}/{}", self.protocol, self.base_uri, self.path);
        if !self.queries.is_empty() {
            url.push('?');
            url.push_str(
                &self
                    .queries
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&"),
            );
        }
        url
    }
}
