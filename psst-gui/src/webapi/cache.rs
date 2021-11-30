use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
};

use druid::ImageBuf;
use lru_cache::LruCache;
use parking_lot::Mutex;
use psst_core::cache::mkdir_if_not_exists;

pub struct WebApiCache {
    base: Option<PathBuf>,
    images: Mutex<LruCache<Arc<str>, ImageBuf>>,
}

impl WebApiCache {
    pub fn new(base: Option<PathBuf>) -> Self {
        const IMAGE_CACHE_SIZE: usize = 256;
        Self {
            base,
            images: Mutex::new(LruCache::new(IMAGE_CACHE_SIZE)),
        }
    }

    pub fn get_image(&self, uri: &Arc<str>) -> Option<ImageBuf> {
        self.images.lock().get_mut(uri).cloned()
    }

    pub fn set_image(&self, uri: Arc<str>, image: ImageBuf) {
        self.images.lock().insert(uri, image);
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
}
