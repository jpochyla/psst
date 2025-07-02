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
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;

use psst_core::session::{access_token::TokenProvider, SessionService};
use ureq::{
    http::{Response, StatusCode},
    Agent, Body,
};

use crate::{
    data::{
        self, utils::sanitize_html_string, Album, AlbumType, Artist, ArtistAlbums, ArtistInfo,
        ArtistLink, ArtistStats, AudioAnalysis, Cached, Episode, EpisodeId, EpisodeLink, Image,
        MixedView, Nav, Page, Playlist, PublicUser, Range, Recommendations, RecommendationsRequest,
        SearchResults, SearchTopic, Show, SpotifyUrl, Track, TrackLines, UserProfile,
    },
    error::Error,
    ui::credits::TrackCredits,
};

use super::{cache::WebApiCache, local::LocalTrackManager};
use sanitize_html::rules::predefined::DEFAULT;
use sanitize_html::sanitize_str;

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
        let mut agent = Agent::config_builder().timeout_global(Some(Duration::from_secs(5)));
        if let Some(proxy_url) = proxy_url {
            let proxy = ureq::Proxy::new(proxy_url).ok();
            agent = agent.proxy(proxy);
        }
        Self {
            session,
            agent: agent.build().into(),
            cache: WebApiCache::new(cache_base),
            token_provider: TokenProvider::new(),
            local_track_manager: Mutex::new(LocalTrackManager::new()),
            paginated_limit,
        }
    }

    fn access_token(&self) -> Result<String, Error> {
        self.token_provider
            .get(&self.session)
            .map_err(|err| Error::WebApiError(err.to_string()))
            .map(|t| t.token)
    }

    fn request(&self, request: &RequestBuilder) -> Result<Response<Body>, Error> {
        let token = self.access_token()?;
        let request = request.clone().query("market", "from_token");

        match request.get_method() {
            Method::Get => {
                let mut req = self
                    .agent
                    .get(request.build())
                    .header("Authorization", &format!("Bearer {}", token));
                for header in request.get_headers() {
                    req = req.header(header.0, header.1);
                }

                req.call()
                    .map_err(|err| Error::WebApiError(err.to_string()))
            }
            Method::Post => self
                .agent
                .post(request.build())
                .header("Authorization", &format!("Bearer {}", token))
                .send_json(request.get_body())
                .map_err(|err| Error::WebApiError(err.to_string())),
            Method::Put => self
                .agent
                .put(request.build())
                .header("Authorization", &format!("Bearer {}", token))
                .send_json(request.get_body())
                .map_err(|err| Error::WebApiError(err.to_string())),
            Method::Delete => self
                .agent
                .delete(request.build())
                .header("Authorization", &format!("Bearer {}", token))
                .call()
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

    /// Send a request with a empty JSON object, throw away the response body.
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
            log::error!("failed to read local tracks: {}", err);
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
            uri: String,

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
                info!("Error loading home section: {}", e);
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
                    let uri = item.content.data.uri.clone();
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

/// Artist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-artist/
    pub fn get_artist(&self, id: &str) -> Result<Artist, Error> {
        let request = &RequestBuilder::new(format!("v1/artists/{}", id), Method::Get, None);
        let result = self.load_cached(request, "artist", id)?;
        Ok(result.data)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-an-artists-albums/
    pub fn get_artist_albums(&self, id: &str) -> Result<ArtistAlbums, Error> {
        let request = &RequestBuilder::new(format!("v1/artists/{}/albums", id), Method::Get, None)
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
        let request =
            &RequestBuilder::new(format!("v1/artists/{}/top-tracks", id), Method::Get, None)
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
        let request = &RequestBuilder::new(
            format!("v1/artists/{}/related-artists", id),
            Method::Get,
            None,
        );
        let result: Cached<Artists> = self.load_cached(request, "related-artists", id)?;
        Ok(result.map(|result| result.artists))
    }

    pub fn get_artist_info(&self, id: &str) -> Result<ArtistInfo, Error> {
        #[derive(Clone, Data, Deserialize)]
        pub struct Welcome {
            data: Data1,
        }

        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Data1 {
            artist_union: ArtistUnion,
        }

        #[derive(Clone, Data, Deserialize)]
        pub struct ArtistUnion {
            profile: Profile,
            stats: Stats,
            visuals: Visuals,
        }

        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Profile {
            biography: Biography,
            external_links: ExternalLinks,
        }

        #[derive(Clone, Data, Deserialize)]
        pub struct Biography {
            text: String,
        }

        #[derive(Clone, Data, Deserialize)]
        pub struct ExternalLinks {
            items: Vector<ExternalLinksItem>,
        }

        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Visuals {
            avatar_image: AvatarImage,
        }
        #[derive(Clone, Data, Deserialize)]
        pub struct AvatarImage {
            sources: Vector<Image>,
        }
        #[derive(Clone, Data, Deserialize)]
        pub struct ExternalLinksItem {
            url: String,
        }

        #[derive(Clone, Data, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Stats {
            followers: i64,
            monthly_listeners: i64,
            world_rank: i64,
        }

        let variables = json!( {
            "locale": "",
            "uri": format!("spotify:artist:{}", id),
        });
        let json = json!({
            "extensions": {
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": "1ac33ddab5d39a3a9c27802774e6d78b9405cc188c6f75aed007df2a32737c72"
                }
            },
            "operationName": "queryArtistOverview",
            "variables": variables,
        });

        let request =
            &RequestBuilder::new("pathfinder/v2/query".to_string(), Method::Post, Some(json))
                .set_base_uri("api-partner.spotify.com");

        let result: Cached<Welcome> = self.load_cached(request, "artist-info", id)?;

        let hrefs: Vector<String> = result
            .data
            .data
            .artist_union
            .profile
            .external_links
            .items
            .into_iter()
            .map(|link| link.url)
            .collect();

        Ok(ArtistInfo {
            main_image: Arc::from(
                result.data.data.artist_union.visuals.avatar_image.sources[0]
                    .url
                    .to_string(),
            ),
            stats: ArtistStats {
                followers: result.data.data.artist_union.stats.followers,
                monthly_listeners: result.data.data.artist_union.stats.monthly_listeners,
                world_rank: result.data.data.artist_union.stats.world_rank,
            },
            bio: {
                let sanitized_bio = sanitize_str(
                    &DEFAULT,
                    &result.data.data.artist_union.profile.biography.text,
                )
                .unwrap_or_default();
                sanitized_bio.replace("&amp;", "&")
            },

            artist_links: hrefs,
        })
    }
}

/// Album endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-an-album/
    pub fn get_album(&self, id: &str) -> Result<Cached<Arc<Album>>, Error> {
        let request = &RequestBuilder::new(format!("v1/albums/{}", id), Method::Get, None)
            .query("market", "from_token");
        let result = self.load_cached(request, "album", id)?;
        Ok(result)
    }
}

/// Show endpoints. (Podcasts)
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/get-a-show/Add commentMore actions
    pub fn get_show(&self, id: &str) -> Result<Cached<Arc<Show>>, Error> {
        let request = &RequestBuilder::new(format!("v1/shows/{}", id), Method::Get, None)
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
        let request = &RequestBuilder::new(format!("v1/shows/{}/episodes", id), Method::Get, None)
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
            format!("track-credits-view/v0/experimental/{}/credits", track_id),
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

    // https://developer.spotify.com/documentation/web-api/reference/save-albums-user/
    pub fn save_album(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/albums", Method::Put, None).query("ids", id);

        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-albums-user/
    pub fn unsave_album(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/albums", Method::Delete, None).query("ids", id);
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

    // https://developer.spotify.com/documentation/web-api/reference/save-tracks-user/
    pub fn save_track(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/tracks", Method::Get, None).query("ids", id);
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-tracks-user/
    pub fn unsave_track(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/tracks", Method::Get, None).query("ids", id);
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/save-shows-user
    pub fn save_show(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/shows", Method::Put, None).query("ids", id);
        self.send_empty_json(request)
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-shows-user
    pub fn unsave_show(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new("v1/me/shows", Method::Delete, None).query("ids", id);
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
                    // From https://github.com/KRTirtho/spotube/blob/9b024120601c0d381edeab4460cb22f87149d0f8/lib%2Fservices%2Fcustom_spotify_endpoints%2Fspotify_endpoints.dart
                    // Keep and eye on this and change accordingly
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
                .header("app-platform", "WebPlayer")
                .header("Content-Type", "application/json");

        // Extract the playlists
        self.load_and_return_home_section(request)
    }

    pub fn get_made_for_you(&self) -> Result<MixedView, Error> {
        // 0JQ5DAqAJXkJGsa2DyEjKi -> Made for you
        self.get_section("spotify:section:0JQ5DAqAJXkJGsa2DyEjKi")
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
        // 0JQ5DAqAJXkJGsa2DyEjKi -> Uniquely yours
        self.get_section("spotify:section:0JQ5DAqAJXkJGsa2DyEjKi")
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
        let request =
            &RequestBuilder::new(format!("v1/playlists/{}/followers", id), Method::Put, None)
                .set_body(Some(json!({"public": false})));
        self.request(request)?;
        Ok(())
    }

    pub fn unfollow_playlist(&self, id: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new(
            format!("v1/playlists/{}/followers", id),
            Method::Delete,
            None,
        );
        self.request(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-playlist
    pub fn get_playlist(&self, id: &str) -> Result<Playlist, Error> {
        let request = &RequestBuilder::new(format!("v1/playlists/{}", id), Method::Get, None);
        let result: Playlist = self.load(request)?;
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

        let request =
            &RequestBuilder::new(format!("v1/playlists/{}/tracks", id), Method::Get, None)
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
        let request =
            &RequestBuilder::new(format!("v1/playlists/{}/tracks", id), Method::Get, None)
                .set_body(Some(json!({ "name": name })));
        self.request(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/add-tracks-to-playlist
    pub fn add_track_to_playlist(&self, playlist_id: &str, track_uri: &str) -> Result<(), Error> {
        let request = &RequestBuilder::new(
            format!("v1/playlists/{}/tracks", playlist_id),
            Method::Post,
            None,
        )
        .query("uris", track_uri);
        self.request(request).map(|_| ())
    }

    // https://developer.spotify.com/documentation/web-api/reference/remove-tracks-playlist
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        track_pos: usize,
    ) -> Result<(), Error> {
        let request = &RequestBuilder::new(
            format!("v1/playlists/{}/tracks", playlist_id),
            Method::Delete,
            None,
        )
        .set_body(Some(json!({ "positions": [track_pos] })));
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
            playlists: Option<Page<Playlist>>,
            shows: Option<Page<Arc<Show>>>,
        }

        let topics = topics.iter().map(SearchTopic::as_str).join(",");
        let request = &RequestBuilder::new("v1/search", Method::Get, None)
            .query("q", query.replace(" ", "%20"))
            .query("type", &topics)
            .query("limit", limit.to_string())
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
            &RequestBuilder::new(format!("v1/audio-analysis/{}", track_id), Method::Get, None);
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
}

impl RequestBuilder {
    // By default we use https and the api.spotify.com
    fn new(path: impl Display, method: Method, body: Option<serde_json::Value>) -> Self {
        Self {
            protocol: "https".to_string(),
            base_uri: "api.spotify.com".to_string(),
            path: path.to_string(),
            queries: HashMap::new(),
            headers: HashMap::new(),
            method,
            body,
        }
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
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&"),
            );
        }
        url
    }
}
