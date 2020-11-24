use crate::{
    data::{Album, AlbumType, Artist, ArtistAlbums, Image, Playlist, SearchResults, Track},
    error::Error,
};
use aspotify::{ItemType, Market, Page, PlaylistItemType, Response};
use druid::im::Vector;
use itertools::Itertools;
use psst_core::{access_token::TokenProvider, cache::mkdir_if_not_exists, session::SessionHandle};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::File,
    future::Future,
    io,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_compat_02::FutureExt;

struct CacheEntry<T> {
    path: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T> CacheEntry<T> {
    fn new(base: &Path, bucket: &str, id: &str) -> Self {
        Self {
            path: base.join(bucket).join(id),
            _phantom: PhantomData::default(),
        }
    }
}

impl<T: Serialize + DeserializeOwned> CacheEntry<T> {
    fn load(&self) -> Option<T> {
        serde_json::from_reader(File::open(&self.path).ok()?).ok()
    }

    fn store(&self, value: &T) -> Result<(), Error> {
        serde_json::to_writer(File::create(&self.path)?, &value)?;
        Ok(())
    }

    async fn load_or_store(
        &self,
        request: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error> {
        if let Some(item) = self.load() {
            Ok(item)
        } else {
            let item = request.await?;
            if let Err(err) = self.store(&item) {
                log::warn!("failed to save to cache: {:?}", err);
            }
            Ok(item)
        }
    }
}

pub struct WebCache {
    base: PathBuf,
}

const CACHE_ALBUM: &str = "album";

impl WebCache {
    pub fn new(base: PathBuf) -> Result<WebCache, Error> {
        // Create the cache structure.
        mkdir_if_not_exists(&base)?;
        mkdir_if_not_exists(&base.join(CACHE_ALBUM))?;

        Ok(Self { base })
    }

    fn album(&self, id: &str) -> CacheEntry<aspotify::Album> {
        CacheEntry::new(&self.base, CACHE_ALBUM, &id)
    }
}

pub struct Web {
    session: SessionHandle,
    token_provider: TokenProvider,
    cache: WebCache,
    spotify: aspotify::Client,
    image_client: reqwest::Client,
}

impl Web {
    pub fn new(session: SessionHandle, cache: WebCache) -> Self {
        // Web API access tokens are requested from the `TokenProvider`, not through the
        // usual Spotify Authorization process, but we still need to give _some_
        // credentials to `aspotify::Client`.
        let dummy_credentials = aspotify::ClientCredentials {
            id: String::new(),
            secret: String::new(),
        };
        let spotify = aspotify::Client::new(dummy_credentials);
        let image_client = reqwest::Client::new();
        let token_provider = TokenProvider::new();
        Self {
            session,
            image_client,
            cache,
            spotify,
            token_provider,
        }
    }

    async fn client(&self) -> Result<&aspotify::Client, Error> {
        let access_token = self
            .token_provider
            .get(&self.session)
            .map_err(|err| Error::WebApiError(err.to_string()))?;
        self.spotify
            .set_current_access_token(access_token.token, access_token.expires)
            .await;
        Ok(&self.spotify)
    }

    async fn with_paging<'a, PerFn, PerFut, MapFn, T, U>(
        &'a self,
        iter_fn: PerFn,
        map_fn: MapFn,
    ) -> Result<Vector<T>, Error>
    where
        PerFn: Fn(&'a aspotify::Client, usize, usize) -> PerFut,
        PerFut: Future<Output = Result<Response<Page<U>>, aspotify::Error>> + 'a,
        MapFn: Fn(U) -> T,
        T: Clone,
    {
        let mut results = Vector::new();
        let mut limit = 50;
        let mut offset = 0;
        loop {
            let page = iter_fn(self.client().await?, limit, offset).await?.data;

            results.extend(page.items.into_iter().map(&map_fn));

            if page.total > results.len() {
                limit = page.limit;
                offset = page.offset + page.limit;
            } else {
                break;
            }
        }
        Ok(results)
    }

    pub async fn load_album(&self, id: &str) -> Result<Album, Error> {
        Ok(self
            .cache
            .album(id)
            .load_or_store(async {
                Ok(self
                    .client()
                    .await?
                    .albums()
                    .get_album(id, Some(Market::FromToken))
                    .await?
                    .data)
            })
            .await?
            .into())
    }

    pub async fn load_artist(&self, id: &str) -> Result<Artist, Error> {
        let result = self
            .client()
            .await?
            .artists()
            .get_artist(id)
            .await?
            .data
            .into();
        Ok(result)
    }

    pub async fn load_saved_albums(&self) -> Result<Vector<Album>, Error> {
        let result = self
            .with_paging(
                |client, limit, offset| {
                    client
                        .library()
                        .get_saved_albums(limit, offset, Some(Market::FromToken))
                },
                |saved| saved.album.into(),
            )
            .await?;
        Ok(result)
    }

    pub async fn load_saved_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        let result = self
            .with_paging(
                |client, limit, offset| {
                    client
                        .library()
                        .get_saved_tracks(limit, offset, Some(Market::FromToken))
                },
                |saved| Arc::new(Track::from(saved.track)),
            )
            .await?;
        Ok(result)
    }

    pub async fn load_playlists(&self) -> Result<Vector<Playlist>, Error> {
        let result = self
            .with_paging(
                |client, limit, offset| client.playlists().current_users_playlists(limit, offset),
                |playlist| playlist.into(),
            )
            .await?;
        Ok(result)
    }

    pub async fn load_playlist_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        let result = self
            .with_paging(
                |client, limit, offset| {
                    client.playlists().get_playlists_items(
                        id,
                        limit,
                        offset,
                        Some(Market::FromToken),
                    )
                },
                |item| match item.item {
                    PlaylistItemType::Track(track) => Arc::new(Track::from(track)),
                    PlaylistItemType::Episode(_) => unimplemented!(),
                },
            )
            .await?;
        Ok(result)
    }

    pub async fn load_artist_albums(&self, id: &str) -> Result<ArtistAlbums, Error> {
        let result: Vector<Album> = self
            .with_paging(
                |client, limit, offset| {
                    client.artists().get_artist_albums(
                        id,
                        None,
                        limit,
                        offset,
                        Some(Market::FromToken),
                    )
                },
                |artists_album| artists_album.into(),
            )
            .await?;
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

    pub async fn load_artist_top_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        let result = self
            .client()
            .await?
            .artists()
            .get_artist_top(id, Market::FromToken)
            .await?
            .data
            .into_iter()
            .map(|track| Arc::new(Track::from(track)))
            .collect();
        Ok(result)
    }

    pub async fn load_related_artists(&self, id: &str) -> Result<Vector<Artist>, Error> {
        let result = self
            .client()
            .await?
            .artists()
            .get_related_artists(id)
            .await?
            .data
            .into_iter()
            .map_into()
            .collect();
        Ok(result)
    }

    pub async fn load_image(&self, uri: &str) -> Result<image::DynamicImage, Error> {
        let req = self.image_client.get(uri).build()?;
        let res = self.image_client.execute(req).compat().await?;
        let img_bytes = res.bytes().await?;
        let img = image::load_from_memory(&img_bytes)?;
        Ok(img)
    }

    pub async fn search(&self, query: &str) -> Result<SearchResults, Error> {
        let results = self
            .client()
            .await?
            .search()
            .search(
                query,
                [ItemType::Artist, ItemType::Album, ItemType::Track]
                    .iter()
                    .copied(),
                false,
                25,
                0,
                Some(Market::FromToken),
            )
            .await?
            .data;
        let artists = results
            .artists
            .map_or_else(Vec::new, |page| page.items)
            .into_iter()
            .map_into()
            .collect();
        let albums = results
            .albums
            .map_or_else(Vec::new, |page| page.items)
            .into_iter()
            .map_into()
            .collect();
        let tracks = results
            .tracks
            .map_or_else(Vec::new, |page| page.items)
            .into_iter()
            .map(|track| Arc::new(Track::from(track)))
            .collect();
        Ok(SearchResults {
            artists,
            albums,
            tracks,
        })
    }
}

impl From<aspotify::ArtistSimplified> for Artist {
    fn from(artist: aspotify::ArtistSimplified) -> Self {
        Self {
            id: artist.id.unwrap().into(),
            name: artist.name.into(),
            images: Vector::new(),
        }
    }
}

impl From<aspotify::Artist> for Artist {
    fn from(artist: aspotify::Artist) -> Self {
        Self {
            id: artist.id.into(),
            name: artist.name.into(),
            images: artist.images.into_iter().map_into().collect(),
        }
    }
}

impl From<aspotify::AlbumSimplified> for Album {
    fn from(album: aspotify::AlbumSimplified) -> Self {
        Self {
            album_type: album.album_type.map(AlbumType::from).unwrap_or_default(),
            artists: album.artists.into_iter().map_into().collect(),
            id: album.id.unwrap().into(),
            images: album.images.into_iter().map_into().collect(),
            name: album.name.into(),
            release_date: album.release_date,
            release_date_precision: album.release_date_precision,
            genres: Vector::new(),
            copyrights: Vector::new(),
            tracks: Vector::new(),
            label: "".into(),
        }
    }
}

impl From<aspotify::Album> for Album {
    fn from(album: aspotify::Album) -> Self {
        Self {
            album_type: album.album_type.into(),
            artists: album.artists.into_iter().map_into().collect(),
            id: album.id.into(),
            images: album.images.into_iter().map_into().collect(),
            name: album.name.into(),
            release_date: Some(album.release_date),
            release_date_precision: Some(album.release_date_precision),
            genres: album.genres.into_iter().map_into().collect(),
            copyrights: album
                .copyrights
                .into_iter()
                .filter_map(|copyright| {
                    if copyright.performance_copyright {
                        Some(copyright.text.into())
                    } else {
                        None
                    }
                })
                .collect(),
            tracks: album
                .tracks
                .items
                .into_iter()
                .map(|track| Arc::new(Track::from(track)))
                .collect(),
            label: album.label.into(),
        }
    }
}

impl From<aspotify::ArtistsAlbum> for Album {
    fn from(album: aspotify::ArtistsAlbum) -> Self {
        Self {
            album_type: album.album_type.into(),
            artists: album.artists.into_iter().map_into().collect(),
            id: album.id.into(),
            images: album.images.into_iter().map_into().collect(),
            name: album.name.into(),
            release_date: Some(album.release_date),
            release_date_precision: Some(album.release_date_precision),
            genres: Vector::new(),
            copyrights: Vector::new(),
            tracks: Vector::new(),
            label: "".into(),
        }
    }
}

impl From<aspotify::AlbumType> for AlbumType {
    fn from(album: aspotify::AlbumType) -> Self {
        match album {
            aspotify::AlbumType::Album => AlbumType::Album,
            aspotify::AlbumType::Single => AlbumType::Single,
            aspotify::AlbumType::Compilation => AlbumType::Compilation,
        }
    }
}

impl From<aspotify::TrackSimplified> for Track {
    fn from(track: aspotify::TrackSimplified) -> Self {
        Self {
            album: None,
            artists: track.artists.into_iter().map_into().collect(),
            disc_number: track.disc_number,
            duration: track.duration.into(),
            explicit: track.explicit,
            // TODO: Local tracks do not have IDs, account for them.
            id: track.id.unwrap().parse().unwrap(),
            is_local: track.is_local,
            is_playable: None,
            name: track.name.into(),
            popularity: None,
            track_number: track.track_number,
        }
    }
}

impl From<aspotify::Track> for Track {
    fn from(track: aspotify::Track) -> Self {
        Self {
            album: Some(track.album.into()),
            artists: track.artists.into_iter().map_into().collect(),
            disc_number: track.disc_number,
            duration: track.duration.into(),
            explicit: track.explicit,
            // TODO: Local tracks do not have IDs, account for them.
            id: track.id.unwrap().parse().unwrap(),
            is_local: track.is_local,
            is_playable: track.is_playable,
            name: track.name.into(),
            popularity: Some(track.popularity),
            track_number: track.track_number,
        }
    }
}

impl From<aspotify::PlaylistSimplified> for Playlist {
    fn from(playlist: aspotify::PlaylistSimplified) -> Self {
        Self {
            id: playlist.id.into(),
            images: playlist.images.into_iter().map_into().collect(),
            name: playlist.name.into(),
        }
    }
}

impl From<aspotify::Image> for Image {
    fn from(image: aspotify::Image) -> Self {
        Self {
            url: image.url.into(),
            width: image.width,
            height: image.height,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<aspotify::Error> for Error {
    fn from(err: aspotify::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<image::ImageError> for Error {
    fn from(err: image::ImageError) -> Self {
        Error::WebApiError(err.to_string())
    }
}
