use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    audio::decrypt::AudioKey,
    error::Error,
    item_id::{FileId, ItemId},
    protocol::metadata::{Episode, Track},
    util::{deserialize_protobuf, serialize_protobuf},
};

pub type CacheHandle = Arc<Cache>;

pub struct Cache {
    base: PathBuf,
}

impl Cache {
    pub fn new(base: PathBuf) -> Result<CacheHandle, Error> {
        log::info!("using cache: {:?}", base);

        // Create the cache structure.
        mkdir_if_not_exists(&base)?;
        mkdir_if_not_exists(&base.join("track"))?;
        mkdir_if_not_exists(&base.join("episode"))?;
        mkdir_if_not_exists(&base.join("audio"))?;
        mkdir_if_not_exists(&base.join("key"))?;

        let cache = Self { base };
        Ok(Arc::new(cache))
    }
}

// Cache of `Track` protobuf structures.
impl Cache {
    pub fn get_track(&self, item_id: ItemId) -> Option<Track> {
        let buf = fs::read(self.track_path(item_id)).ok()?;
        deserialize_protobuf(&buf).ok()
    }

    pub fn save_track(&self, item_id: ItemId, track: &Track) -> Result<(), Error> {
        log::debug!("saving track to cache: {:?}", item_id);
        fs::write(self.track_path(item_id), &serialize_protobuf(track)?)?;
        Ok(())
    }

    fn track_path(&self, item_id: ItemId) -> PathBuf {
        self.base.join("track").join(item_id.to_base62())
    }
}

// Cache of `Episode` protobuf structures.
impl Cache {
    pub fn get_episode(&self, item_id: ItemId) -> Option<Episode> {
        let buf = fs::read(self.episode_path(item_id)).ok()?;
        deserialize_protobuf(&buf).ok()
    }

    pub fn save_episode(&self, item_id: ItemId, episode: &Episode) -> Result<(), Error> {
        log::debug!("saving episode to cache: {:?}", item_id);
        fs::write(self.episode_path(item_id), &serialize_protobuf(episode)?)?;
        Ok(())
    }

    fn episode_path(&self, item_id: ItemId) -> PathBuf {
        self.base.join("episode").join(item_id.to_base62())
    }
}

// Cache of `AudioKey`s.
impl Cache {
    pub fn get_audio_key(&self, item_id: ItemId, file_id: FileId) -> Option<AudioKey> {
        let buf = fs::read(self.audio_key_path(item_id, file_id)).ok()?;
        AudioKey::from_raw(&buf)
    }

    pub fn save_audio_key(
        &self,
        item_id: ItemId,
        file_id: FileId,
        key: &AudioKey,
    ) -> Result<(), Error> {
        log::debug!("saving audio key to cache: {:?}:{:?}", item_id, file_id);
        fs::write(self.audio_key_path(item_id, file_id), key.0)?;
        Ok(())
    }

    fn audio_key_path(&self, item_id: ItemId, file_id: FileId) -> PathBuf {
        let mut key_id = String::new();
        key_id += &item_id.to_base62()[..16];
        key_id += &file_id.to_base16()[..16];
        self.base.join("key").join(key_id)
    }
}

// Cache of encrypted audio file content.
impl Cache {
    pub fn audio_file_path(&self, file_id: FileId) -> PathBuf {
        self.base.join("audio").join(file_id.to_base16())
    }

    pub fn save_audio_file(&self, file_id: FileId, from_path: PathBuf) -> Result<(), Error> {
        log::debug!("saving audio file to cache: {:?}", file_id);
        fs::copy(from_path, self.audio_file_path(file_id))?;
        Ok(())
    }
}

// Cache of user country code.
impl Cache {
    pub fn get_country_code(&self) -> Option<String> {
        fs::read_to_string(self.country_code_path()).ok()
    }

    pub fn save_country_code(&self, country_code: &str) -> Result<(), Error> {
        fs::write(self.country_code_path(), country_code)?;
        Ok(())
    }

    fn country_code_path(&self) -> PathBuf {
        self.base.join("country_code")
    }
}

pub fn mkdir_if_not_exists(path: &Path) -> io::Result<()> {
    fs::create_dir(path).or_else(|err| {
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    })
}
