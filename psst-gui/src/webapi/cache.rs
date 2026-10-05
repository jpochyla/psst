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

pub struct WebApiCache {
    base: Option<PathBuf>,
    images: Mutex<LruCache<Arc<str>, ImageBuf>>,
    writes: Mutex<()>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_invalidation_preserves_audio_images_and_release_quota() {
        let path = std::env::temp_dir().join(format!(
            "xpotify-cache-test-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let cache = WebApiCache::new(Some(path.clone()));
        cache.set("responses", "request", br#"{"name":"old"}"#);
        cache.set("responses", "request", br#"{"name":"new"}"#);
        let value: serde_json::Value =
            serde_json::from_reader(cache.get("responses", "request").unwrap()).unwrap();
        assert_eq!(value["name"], "new");
        for bucket in ["audio", "images", "artist-releases", "request-limits"] {
            cache.set(bucket, "preserve", b"unchanged");
        }
        cache.invalidate_metadata().unwrap();
        assert!(cache.get("responses", "request").is_none());
        for bucket in ["audio", "images", "artist-releases", "request-limits"] {
            assert!(cache.get(bucket, "preserve").is_some());
        }
        fs::remove_dir_all(path).unwrap();
    }
}

impl WebApiCache {
    pub fn new(base: Option<PathBuf>) -> Self {
        const IMAGE_CACHE_SIZE: usize = 256;
        Self {
            base,
            images: Mutex::new(LruCache::new(NonZeroUsize::new(IMAGE_CACHE_SIZE).unwrap())),
            writes: Mutex::new(()),
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
        let _guard = self.writes.lock();
        if let Some(path) = self.bucket(bucket) {
            if let Err(err) = fs::create_dir_all(&path) {
                log::error!("failed to create WebAPI cache bucket: {err:?}");
            }
        }
        if let Some(path) = self.key(bucket, key) {
            let temporary = path.with_extension("tmp");
            if let Err(err) = fs::write(&temporary, value).and_then(|_| fs::rename(temporary, path))
            {
                log::error!("failed to save to WebAPI cache: {err:?}");
            }
        }
    }

    pub fn invalidate_metadata(&self) -> std::io::Result<()> {
        let _guard = self.writes.lock();
        for bucket in [
            "responses",
            "artist",
            "artist-overview",
            "album",
            "show",
            "lyrics",
            "audio-analysis",
        ] {
            if let Some(path) = self.bucket(bucket) {
                match fs::read_dir(path) {
                    Ok(entries) => {
                        for entry in entries {
                            let entry = entry?;
                            if entry.file_type()?.is_file() {
                                fs::remove_file(entry.path())?;
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    fn bucket(&self, bucket: &str) -> Option<PathBuf> {
        self.base.as_ref().map(|path| path.join(bucket))
    }

    fn key(&self, bucket: &str, key: &str) -> Option<PathBuf> {
        self.bucket(bucket).map(|path| path.join(key))
    }
}
