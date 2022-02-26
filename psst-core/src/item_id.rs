use std::{convert::TryInto, fmt, ops::Deref};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemIdType {
    Track,
    Podcast,
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
