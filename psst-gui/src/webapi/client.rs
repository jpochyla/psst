use std::{
    fmt::Display, io::{self, Read}, path::PathBuf, sync::Arc, thread, time::Duration
};

use druid::{
    im::Vector,
    image::{self, ImageFormat},
    Data, ImageBuf,
};
use itertools::Itertools;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use sanitize_html::rules::predefined::DEFAULT;
use sanitize_html::sanitize_str;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use ureq::{Agent, Request, Response};

use psst_core::{
    session::{access_token::TokenProvider, SessionService},
    util::default_ureq_agent_builder,
};

use crate::{
    data::{
        self, Album, AlbumType, Artist, ArtistAlbums, ArtistInfo, ArtistLink, ArtistStats, AudioAnalysis, Cached, Episode, EpisodeId, EpisodeLink, Image, MixedView, Nav, Page, Playlist, PublicUser, Range, Recommendations, RecommendationsRequest, SearchResults, SearchTopic, Show, SpotifyUrl, Track, UserProfile
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

    fn build_request(
        &self,
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
        self.request("GET", base_url.unwrap_or("api.spotify.com"), path)
    }

    fn put(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("PUT", base_url.unwrap_or("api.spotify.com"), path)
    }

    fn post(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("POST", base_url.unwrap_or("api.spotify.com"), path)
    }

    fn delete(&self, path: impl Display, base_url: Option<&str>) -> Result<Request, Error> {
        self.request("DELETE", base_url.unwrap_or("api.spotify.com"), path)
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

    fn load_and_return_home_section(&self, request: Request) -> Result<MixedView, Error> {
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
        let result: Welcome = self.load(request)?;

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
                    let id = uri.split(':').last().unwrap_or("").to_string();

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
                                    let desc = sanitize_str(
                                        &DEFAULT,
                                        item.content
                                            .data
                                            .description
                                            .as_deref()
                                            .unwrap_or_default(),
                                    )
                                    .unwrap_or_default();
                                    // This is roughly 3 lines of description, truncated if too long
                                    if desc.chars().count() > 55 {
                                        desc.chars().take(52).collect::<String>() + "..."
                                    } else {
                                        desc
                                    }
                                    .into()
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
                                    display_name: item
                                        .content
                                        .data
                                        .owner_v2
                                        .as_ref()
                                        .map(|owner| Arc::from(owner.data.name.as_str()))
                                        .unwrap_or_else(|| Arc::from("")),
                                },
                                collaborative: false,
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
                                                    .last()
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
                                item.content.data.publisher.as_ref().unwrap().name.clone(),
                            ),
                            description: "".into(),
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
        let request = self.get("v1/me", None)?;
        let result = self.load(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/get-users-top-artists-and-tracks
    pub fn get_user_top_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        let request = self
            .get("v1/me/top/tracks", None)?
            .query("market", "from_token");

        let result: Vector<Arc<Track>> = self.load_some_pages(request, 30)?;

        Ok(result)
    }

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

        let extensions = json!({
            "persistedQuery": {
                "version": 1,
                // From https://github.com/spicetify/cli/blob/bb767a9059143fe183c1c577acff335dc6a462b7/Extensions/shuffle%2B.js#L373 keep and eye on this and change accordingly
                "sha256Hash": "35648a112beb1794e39ab931365f6ae4a8d45e65396d641eeda94e4003d41497"
            }
        });
        let extensions_json = serde_json::to_string(&extensions);
        
        let variables = json!( {
            "uri": format!("spotify:artist:{}", id),
            "locale": "",
            "includePrerelease": true,  // Assuming this returns a Result<String, Error>
        });
        let variables_json = serde_json::to_string(&variables);

        let request = self.get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "queryArtistOverview")
            .query("variables", &variables_json.unwrap().to_string())
            .query("extensions", &extensions_json.unwrap().to_string());

        let result: Cached<Welcome> = self.load_cached(request, "artist-info", id)?;

        let hrefs: Vector<String> = result.data.data.artist_union.profile.external_links.items
        .into_iter()
        .map(|link| link.url)
        .collect();

        Ok(ArtistInfo {
            main_image: Arc::from(result.data.data.artist_union.visuals.avatar_image.sources[0].url.to_string()),
            stats: ArtistStats{
                followers: result.data.data.artist_union.stats.followers.to_string(),
                monthly_listeners: result.data.data.artist_union.stats.monthly_listeners.to_string(),
                world_rank: result.data.data.artist_union.stats.world_rank.to_string()
            },
            bio: {
                let desc = sanitize_str(
                    &DEFAULT,
                    &result.data
                        .data
                        .artist_union.profile.biography.text,
                )
                .unwrap_or_default();
                // This is roughly 3 lines of description, truncated if too long
                if desc.chars().count() > 255 {
                    desc.chars().take(254).collect::<String>() + "..."
                } else {
                    desc
                }
                .into()
            },
            
            artist_links: hrefs.into()
        })
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

        let request = self
            .get("v1/me/albums", None)?
            .query("market", "from_token");

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

        let request = self
            .get("v1/me/tracks", None)?
            .query("market", "from_token");

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

/// View endpoints.
impl WebApi {
    pub fn get_user_info(&self) -> Result<(String, String), Error> {
        #[derive(Deserialize, Clone, Data)]
        struct User {
            region: String,
            timezone: String,
        }
        let token = self.access_token()?;
        let request = self
            .agent
            .request("GET", &format!("http://{}/{}", "ip-api.com", "json"))
            .query("fields", "260")
            .set("Authorization", &format!("Bearer {}", &token));

        let result: Cached<User> = self.load_cached(request, "User_info", "usrinfo")?;

        Ok((result.data.region.clone(), result.data.timezone.clone()))
    }

    fn build_home_request(&self, section_uri: &str) -> (String, String) {
        let extensions = json!({
            "persistedQuery": {
                "version": 1,
                // From https://github.com/KRTirtho/spotube/blob/9b024120601c0d381edeab4460cb22f87149d0f8/lib%2Fservices%2Fcustom_spotify_endpoints%2Fspotify_endpoints.dart keep and eye on this and change accordingly
                "sha256Hash": "eb3fba2d388cf4fc4d696b1757a58584e9538a3b515ea742e9cc9465807340be"
            }
        });

        let variables = json!( {
            "uri": section_uri,
            "timeZone": self.get_user_info().unwrap().0,
            "sp_t": self.access_token().unwrap(),  // Assuming this returns a Result<String, Error>
            "country": self.get_user_info().unwrap().1,
            "sectionItemsOffset": 0,
            "sectionItemsLimit": 20,
        });

        let variables_json = serde_json::to_string(&variables);
        let extensions_json = serde_json::to_string(&extensions);

        (variables_json.unwrap(), extensions_json.unwrap())
    }

    pub fn get_made_for_you(&self) -> Result<MixedView, Error> {
        // 0JQ5DAUnp4wcj0bCb3wh3S -> Daily mixes
        let json_query = self.build_home_request("spotify:section:0JQ5DAUnp4wcj0bCb3wh3S");
        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    pub fn get_top_mixes(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu89 -> Top mixes
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu89");
        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    pub fn recommended_stations(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3R -> Recommended stations
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu3R");

        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    pub fn uniquely_yours(&self) -> Result<MixedView, Error> {
        // 0JQ5DAqAJXkJGsa2DyEjKi -> Uniquely yours
        let json_query = self.build_home_request("spotify:section:0JQ5DAqAJXkJGsa2DyEjKi");

        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    pub fn best_of_artists(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3n -> Best of artists
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu3n");
        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    // Need to make a mix of it!
    pub fn jump_back_in(&self) -> Result<MixedView, Error> {
        // 0JQ5DAIiKWzVFULQfUm85X -> Jump back in
        let json_query = self.build_home_request("spotify:section:0JQ5DAIiKWzVFULQfUm85X");
        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    // Shows
    pub fn your_shows(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3N -> Your shows
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu3N");
        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    pub fn shows_that_you_might_like(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3P -> Shows that you might like
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu3P");
        let request = self
            .get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    /*
    // TODO: Episodes for you, implement this to redesign the podcast page
    pub fn new_episodes(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu3K -> New episodes
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu3K");
        let request = self.get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;

        Ok(result)
    }

    // Episodes for you, this needs to have its own thing or be part of a mixed view as it is in episode form
    pub fn episode_for_you(&self) -> Result<MixedView, Error> {
        // 0JQ5DAnM3wGh0gz1MXnu9e -> Episodes for you
        let json_query = self.build_home_request("spotify:section:0JQ5DAnM3wGh0gz1MXnu9e");
        let request = self.get("pathfinder/v1/query", Some("api-partner.spotify.com"))?
            .query("operationName", "homeSection")
            .query("variables", &json_query.0.to_string())
            .query("extensions", &json_query.1.to_string());

        // Extract the playlists
        let result = self.load_and_return_home_section(request)?;
        Ok(result)
    }
    */
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
