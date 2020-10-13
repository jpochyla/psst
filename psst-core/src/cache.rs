use crate::{
    audio_key::AudioKey,
    error::Error,
    spotify_id::{FileId, SpotifyId},
    util::{deserialize_protobuf, serialize_protobuf},
};
use psst_protocol::metadata::Track;
use std::{
    fs,
    fs::File,
    io,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

pub type CacheHandle = Arc<Cache>;

pub struct Cache {
    base: PathBuf,
}

impl Cache {
    pub fn new() -> Result<CacheHandle, Error> {
        let base = PathBuf::from("cache");

        // Create the cache structure.
        mkdir_if_not_exists(&base)?;
        mkdir_if_not_exists(&base.join("track"))?;
        mkdir_if_not_exists(&base.join("audio"))?;
        mkdir_if_not_exists(&base.join("key"))?;

        let cache = Self { base };
        Ok(Arc::new(cache))
    }
}

// Cache of `Track` protobuf structures.
impl Cache {
    pub fn get_track(&self, item_id: SpotifyId) -> Option<Track> {
        let mut file = File::open(self.track_path(item_id)).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        deserialize_protobuf(&buf).ok()
    }

    pub fn save_track(&self, item_id: SpotifyId, track: &Track) -> Result<(), Error> {
        log::debug!("saving track to cache: {:?}", item_id);
        let mut file = File::create(self.track_path(item_id))?;
        let buf = serialize_protobuf(track)?;
        file.write_all(&buf)?;
        Ok(())
    }

    fn track_path(&self, item_id: SpotifyId) -> PathBuf {
        self.base.join("track").join(item_id.to_base62())
    }
}

// Cache of `AudioKey`s.
impl Cache {
    pub fn get_audio_key(&self, item_id: SpotifyId, file_id: FileId) -> Option<AudioKey> {
        let mut file = File::open(self.audio_key_path(item_id, file_id)).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        AudioKey::from_raw(&buf)
    }

    pub fn save_audio_key(
        &self,
        item_id: SpotifyId,
        file_id: FileId,
        key: &AudioKey,
    ) -> Result<(), Error> {
        log::debug!("saving audio key to cache: {:?}:{:?}", item_id, file_id);
        let mut file = File::create(self.audio_key_path(item_id, file_id))?;
        file.write_all(&key.0)?;
        Ok(())
    }

    fn audio_key_path(&self, item_id: SpotifyId, file_id: FileId) -> PathBuf {
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

fn mkdir_if_not_exists(path: &Path) -> io::Result<()> {
    fs::create_dir(path).or_else(|err| {
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    })
}
