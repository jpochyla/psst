use psst_protocol::metadata::Track;

use crate::{
    audio::{decode::AudioDecoder, decrypt::AudioKey, normalize::NormalizationLevel},
    cache::CacheHandle,
    cdn::CdnHandle,
    error::Error,
    item_id::{ItemId, ItemIdType},
    metadata::{Fetch, ToMediaPath},
    session::SessionService,
};

use super::{
    file::{MediaFile, MediaPath},
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
        let path = load_audio_path(self.item_id, session, &cache, config)?;
        let key = load_audio_key(&path, session, &cache)?;
        let file = MediaFile::open(path, cdn, cache)?;
        let (source, norm_data) = file.audio_source(key)?;
        let norm_factor = norm_data.factor_for_level(self.norm_level, config.pregain);
        Ok(LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        })
    }
}

fn load_audio_path(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<MediaPath, Error> {
    match item_id.id_type {
        ItemIdType::Track => {
            load_audio_path_from_track_or_alternative(item_id, session, cache, config)
        }
        ItemIdType::Podcast | ItemIdType::Unknown => unimplemented!(),
    }
}

fn load_audio_path_from_track_or_alternative(
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
