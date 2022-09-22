use once_cell::sync::Lazy;
use std::{collections::HashMap, convert::TryInto, fmt, ops::Deref, path::PathBuf, sync::Mutex};

static LOCAL_REGISTRY: Lazy<Mutex<LocalItemRegistry>> =
    Lazy::new(|| Mutex::new(LocalItemRegistry::new()));

// LocalItemRegistry allows generating IDs for local music files, so they can be
// treated similarly to files hosted on Spotify's remote servers. IDs are
// easier to pass around since they implement `Copy`, as opposed to passing
// around a `PathBuf` or `File` pointing to the file.
//
// The registry stores two complementary maps for bi-directional lookup. This
// allows for quick registration of new tracks and quick lookup of existing
// tracks by ID, at the cost of increased memory usage. The ID-to-path lookup
// should be prioritized, as that is required to begin playback. Path-to-ID
// lookup is helpful to avoid registering the same path under multiple IDs,
// but is okay to be a bit slower since it's only done once per track when
// (when loading the list of local files from Spotify's config).
pub struct LocalItemRegistry {
    next_id: u128,
    path_to_id: HashMap<PathBuf, u128>,
    id_to_path: HashMap<u128, PathBuf>,
}

impl LocalItemRegistry {
    fn new() -> Self {
        Self {
            next_id: 1,
            path_to_id: HashMap::new(),
            id_to_path: HashMap::new(),
        }
    }

    pub fn get_or_insert(path: PathBuf) -> u128 {
        let mut registry = LOCAL_REGISTRY.lock().unwrap();
        registry
            .path_to_id
            .get(&path)
            .map(|id| *id)
            .unwrap_or_else(|| {
                let id = registry.next_id;
                registry.next_id += 1;
                registry.id_to_path.insert(id, path.clone());
                id
            })
    }

    pub fn get(id: u128) -> Option<PathBuf> {
        let registry = LOCAL_REGISTRY.lock().unwrap();
        registry.id_to_path.get(&id).map(|path| path.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemIdType {
    Track,
    Podcast,
    LocalFile,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId {
    pub id: u128,
    pub id_type: ItemIdType,
}

const BASE62_DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const BASE16_DIGITS: &[u8] = b"0123456789abcdef";

impl ItemId {
    pub const INVALID: Self = Self::new(0u128, ItemIdType::Unknown);

    pub const fn new(id: u128, id_type: ItemIdType) -> Self {
        Self { id, id_type }
    }

    pub fn from_base16(id: &str, id_type: ItemIdType) -> Option<Self> {
        let mut n = 0_u128;
        for c in id.as_bytes() {
            let d = BASE16_DIGITS.iter().position(|e| e == c)? as u128;
            n *= 16;
            n += d;
        }
        Some(Self::new(n, id_type))
    }

    pub fn from_base62(id: &str, id_type: ItemIdType) -> Option<Self> {
        let mut n = 0_u128;
        for c in id.as_bytes() {
            let d = BASE62_DIGITS.iter().position(|e| e == c)? as u128;
            n *= 62;
            n += d;
        }
        Some(Self::new(n, id_type))
    }

    pub fn from_raw(data: &[u8], id_type: ItemIdType) -> Option<Self> {
        let n = u128::from_be_bytes(data.try_into().ok()?);
        Some(Self::new(n, id_type))
    }

    pub fn from_uri(uri: &str) -> Option<Self> {
        let gid = uri.split(':').last()?;
        if uri.contains(":episode:") {
            Self::from_base62(gid, ItemIdType::Podcast)
        } else if uri.contains(":track:") {
            Self::from_base62(gid, ItemIdType::Track)
        } else {
            Self::from_base62(gid, ItemIdType::Unknown)
        }
    }

    /// Converts an ID to an URI as described in: https://developer.spotify.com/documentation/web-api/#spotify-uris-and-ids
    pub fn to_uri(&self) -> Option<String> {
        let b64 = self.to_base62();
        match self.id_type {
            ItemIdType::Track => Some(format!("spotify:track:{}", b64)),
            ItemIdType::Podcast => Some(format!("spotify:podcast:{}", b64)),
            // TODO: support adding local files to playlists
            ItemIdType::LocalFile => None,
            ItemIdType::Unknown => None,
        }
    }

    pub fn to_base16(&self) -> String {
        format!("{:032x}", self.id)
    }

    pub fn to_base62(&self) -> String {
        let mut n = self.id;
        let mut data = [0_u8; 22];
        for i in 0..22 {
            data[21 - i] = BASE62_DIGITS[(n % 62) as usize];
            n /= 62;
        }
        std::str::from_utf8(&data).unwrap().to_string()
    }

    pub fn to_raw(&self) -> [u8; 16] {
        self.id.to_be_bytes()
    }

    pub fn from_local(path: PathBuf) -> Self {
        Self::new(
            LocalItemRegistry::get_or_insert(path),
            ItemIdType::LocalFile,
        )
    }

    pub fn to_local(&self) -> PathBuf {
        match self.id_type {
            // local items should only be constructed with `from_local`
            ItemIdType::LocalFile => LocalItemRegistry::get(self.id).expect("valid item ID"),
            _ => panic!("expected local file"),
        }
    }
}

impl Default for ItemId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl From<ItemId> for String {
    fn from(id: ItemId) -> Self {
        id.to_base62()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FileId(pub [u8; 20]);

impl FileId {
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        Some(FileId(data.try_into().ok()?))
    }

    pub fn to_base16(&self) -> String {
        self.0
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .concat()
    }
}

impl Deref for FileId {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for FileId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("FileId").field(&self.to_base16()).finish()
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_base16())
    }
}
