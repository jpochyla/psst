use std::time::Duration;

use crate::{
    audio::{
        decode::AudioDecoder, decrypt::AudioKey, normalize::NormalizationLevel, probe::TrackProbe,
    },
    cache::CacheHandle,
    cdn::CdnHandle,
    error::Error,
    item_id::{ItemId, ItemIdType, LocalItemRegistry},
    metadata::{Fetch, ToMediaPath},
    protocol::metadata::{Episode, Track},
    session::SessionService,
};

use super::{
    file::{AudioFormat, MediaFile, MediaPath},
    PlaybackConfig,
};

pub struct LoadedPlaybackItem {
    pub file: MediaFile,
    pub source: AudioDecoder,
    pub norm_factor: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PlaybackItem {
    pub item_id: ItemId,
    pub norm_level: NormalizationLevel,
}

impl PlaybackItem {
    pub fn load(
        &self,
        session: &SessionService,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: &PlaybackConfig,
    ) -> Result<LoadedPlaybackItem, Error> {
        let path = load_media_path(self.item_id, session, &cache, config)?;
        let (file, source, norm_data) = match self.item_id.id_type {
            ItemIdType::LocalFile => {
                let file = MediaFile::local(path);
                let (source, norm_data) = file.local_audio_source()?;
                (file, source, norm_data)
            }
            _ => {
                let key = load_audio_key(&path, session, &cache)?;
                let file = MediaFile::open(path, cdn, cache)?;
                let (source, norm_data) = file.remote_audio_source(key)?;
                (file, source, norm_data)
            }
        };
        let norm_factor = norm_data.factor_for_level(self.norm_level, config.pregain);
        Ok(LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        })
    }
}

fn load_media_path(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<MediaPath, Error> {
    match item_id.id_type {
        ItemIdType::Track => {
            load_media_path_from_track_or_alternative(item_id, session, cache, config)
        }
        ItemIdType::Podcast => load_media_path_from_episode(item_id, session, cache, config),
        ItemIdType::LocalFile => load_media_path_from_local(item_id),
        ItemIdType::Unknown => unimplemented!(),
    }
}

fn load_media_path_from_track_or_alternative(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<MediaPath, Error> {
    let track = load_track(item_id, session, cache)?;
    let country = get_country_code(session, cache);
    let path = match country {
        Some(user_country) if track.is_restricted_in_region(&user_country) => {
            // The track is regionally restricted and is unavailable.  Let's try to find an
            // alternative track.
            let alt_id = track
                .find_allowed_alternative(&user_country)
                .ok_or(Error::MediaFileNotFound)?;
            let alt_track = load_track(alt_id, session, cache)?;
            let alt_path = alt_track
                .to_media_path(config.bitrate)
                .ok_or(Error::MediaFileNotFound)?;
            // We've found an alternative track with a fitting audio file.  Let's cheat a
            // little and pretend we've obtained it from the requested track.
            // TODO: We should be honest and display the real track information.
            MediaPath {
                item_id,
                ..alt_path
            }
        }
        _ => {
            // Either we do not have a country code loaded or the track is available, return
            // it.
            track
                .to_media_path(config.bitrate)
                .ok_or(Error::MediaFileNotFound)?
        }
    };
    Ok(path)
}

fn load_media_path_from_episode(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<MediaPath, Error> {
    let episode = load_episode(item_id, session, cache)?;
    let country = get_country_code(session, cache);
    let path = match country {
        Some(user_country) if episode.is_restricted_in_region(&user_country) => {
            // Episode is restricted, and doesn't have any alternatives.
            return Err(Error::MediaFileNotFound);
        }
        _ => episode
            .to_media_path(config.bitrate)
            .ok_or(Error::MediaFileNotFound)?,
    };
    Ok(path)
}

fn load_media_path_from_local(item_id: ItemId) -> Result<MediaPath, Error> {
    let path = LocalItemRegistry::get(item_id.id).expect("valid local item ID");
    let probe = TrackProbe::new(&path)?;
    Ok(MediaPath {
        item_id,
        file_id: Default::default(),
        file_format: AudioFormat::from_codec(probe.codec),
        // It's possible (though unlikely) that we're unable to determine the track
        // duration from the codec params; in that case, default to 0 and let it
        // be calculated at runtime as we play the track.
        duration: probe.duration.unwrap_or(Duration::from_millis(0)),
    })
}

fn get_country_code(session: &SessionService, cache: &CacheHandle) -> Option<String> {
    if let Some(cached_country_code) = cache.get_country_code() {
        Some(cached_country_code)
    } else {
        let country_code = session.connected().ok()?.get_country_code()?;
        if let Err(err) = cache.save_country_code(&country_code) {
            log::warn!("failed to save country code to cache: {:?}", err);
        }
        Some(country_code)
    }
}

fn load_track(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<Track, Error> {
    if let Some(cached_track) = cache.get_track(item_id) {
        Ok(cached_track)
    } else {
        let track = Track::fetch(session, item_id)?;
        if let Err(err) = cache.save_track(item_id, &track) {
            log::warn!("failed to save track to cache: {:?}", err);
        }
        Ok(track)
    }
}

fn load_episode(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<Episode, Error> {
    if let Some(cached_episode) = cache.get_episode(item_id) {
        Ok(cached_episode)
    } else {
        let episode = Episode::fetch(session, item_id)?;
        if let Err(err) = cache.save_episode(item_id, &episode) {
            log::warn!("failed to save episode to cache: {:?}", err);
        }
        Ok(episode)
    }
}

fn load_audio_key(
    path: &MediaPath,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<AudioKey, Error> {
    if let Some(cached_key) = cache.get_audio_key(path.item_id, path.file_id) {
        Ok(cached_key)
    } else {
        let key = session
            .connected()?
            .get_audio_key(path.item_id, path.file_id)?;
        if let Err(err) = cache.save_audio_key(path.item_id, path.file_id, &key) {
            log::warn!("failed to save audio key to cache: {:?}", err);
        }
        Ok(key)
    }
}
