use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use crate::data::TrackLines;
use druid::im::Vector;
use druid::image;
use druid::ImageBuf;
use lru_cache::LruCache;
use parking_lot::Mutex;
use psst_core::cache::mkdir_if_not_exists;

pub struct WebApiCache {
    base: Option<PathBuf>,
    images: Mutex<LruCache<Arc<str>, ImageBuf>>,
    lyrics: Mutex<LruCache<Arc<str>, Vector<TrackLines>>>,
}

impl WebApiCache {
    pub fn new(base: Option<PathBuf>) -> Self {
        const IMAGE_CACHE_SIZE: usize = 256;
        const LYRICS_CACHE_SIZE: usize = 100;
        Self {
            base,
            images: Mutex::new(LruCache::new(IMAGE_CACHE_SIZE)),
            lyrics: Mutex::new(LruCache::new(LYRICS_CACHE_SIZE)),
        }
    }

    pub fn get_image(&self, uri: &Arc<str>) -> Option<ImageBuf> {
        self.images.lock().get_mut(uri).cloned()
    }

    pub fn set_image(&self, uri: Arc<str>, image: ImageBuf) {
        self.images.lock().insert(uri, image);
    }

    pub fn get_image_from_disk(&self, uri: &Arc<str>) -> Option<ImageBuf> {
        let hash = Self::hash_uri(uri);
        self.key("images", &format!("{:016x}", hash))
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(ImageBuf::from_dynamic_image)
    }

    pub fn save_image_to_disk(&self, uri: &Arc<str>, data: &[u8]) {
        let hash = Self::hash_uri(uri);
        if let Some(path) = self.key("images", &format!("{:016x}", hash)) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, data);
        }
    }

    fn hash_uri(uri: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        uri.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get(&self, bucket: &str, key: &str) -> Option<File> {
        self.key(bucket, key).and_then(|path| File::open(path).ok())
    }

    pub fn set(&self, bucket: &str, key: &str, value: &[u8]) {
        if let Some(path) = self.bucket(bucket) {
            if let Err(err) = mkdir_if_not_exists(&path) {
                log::error!("failed to create WebAPI cache bucket: {:?}", err);
            }
        }
        if let Some(path) = self.key(bucket, key) {
            if let Err(err) = fs::write(path, value) {
                log::error!("failed to save to WebAPI cache: {:?}", err);
            }
        }
    }

    fn bucket(&self, bucket: &str) -> Option<PathBuf> {
        self.base.as_ref().map(|path| path.join(bucket))
    }

    fn key(&self, bucket: &str, key: &str) -> Option<PathBuf> {
        self.bucket(bucket).map(|path| path.join(key))
    }

    pub fn get_lyrics(&self, track_id: &Arc<str>) -> Option<Vector<TrackLines>> {
        let result = self.lyrics.lock().get_mut(track_id).cloned();
        result
    }

    pub fn set_lyrics(&self, track_id: Arc<str>, lyrics: Vector<TrackLines>) {
        self.lyrics.lock().insert(track_id, lyrics);
    }

    pub fn get_lyrics_from_disk(&self, track_id: &Arc<str>) -> Option<Vector<TrackLines>> {
        let result = self
            .key("lyrics", track_id)
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        result
    }

    pub fn save_lyrics_to_disk(&self, track_id: &Arc<str>, lyrics: &Vector<TrackLines>) {
        if let Some(path) = self.key("lyrics", track_id) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(data) = serde_json::to_vec(lyrics) {
                let _ = std::fs::write(path, data);
            }
        }
    }
}
