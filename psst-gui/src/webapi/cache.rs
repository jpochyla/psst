use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    path::PathBuf,
    sync::Arc,
};

use druid::image;
use druid::ImageBuf;
use lru::LruCache;
use parking_lot::Mutex;
use psst_core::cache::mkdir_if_not_exists;

use crate::data::utils::crop_to_square;

pub struct WebApiCache {
    base: Option<PathBuf>,
    images: Mutex<LruCache<Arc<str>, ImageBuf>>,
}

impl WebApiCache {
    pub fn new(base: Option<PathBuf>) -> Self {
        const IMAGE_CACHE_SIZE: usize = 256;
        Self {
            base,
            images: Mutex::new(LruCache::new(NonZeroUsize::new(IMAGE_CACHE_SIZE).unwrap())),
        }
    }

    pub fn get_image(&self, uri: &Arc<str>) -> Option<ImageBuf> {
        self.images.lock().get(uri).cloned()
    }

    pub fn set_image(&self, uri: Arc<str>, image: ImageBuf) {
        self.images.lock().put(uri, image);
    }

    pub fn get_image_from_disk(&self, uri: &Arc<str>) -> Option<ImageBuf> {
        let hash = Self::hash_uri(uri);
        self.key("images", &format!("{hash:016x}"))
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(crop_to_square)
            .map(ImageBuf::from_dynamic_image)
    }

    pub fn save_image_to_disk(&self, uri: &Arc<str>, data: &[u8]) {
        let hash = Self::hash_uri(uri);
        if let Some(path) = self.key("images", &format!("{hash:016x}")) {
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
                log::error!("failed to create WebAPI cache bucket: {err:?}");
            }
        }
        if let Some(path) = self.key(bucket, key) {
            if let Err(err) = fs::write(path, value) {
                log::error!("failed to save to WebAPI cache: {err:?}");
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
