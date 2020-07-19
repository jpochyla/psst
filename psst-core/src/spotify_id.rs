use std::{convert::TryInto, fmt, ops::Deref};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpotifyIdType {
    Track,
    Podcast,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpotifyId {
    pub id: u128,
    pub id_type: SpotifyIdType,
}

const BASE62_DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const BASE16_DIGITS: &[u8] = b"0123456789abcdef";

impl SpotifyId {
    fn new(id: u128, id_type: SpotifyIdType) -> Self {
        Self { id, id_type }
    }

    pub fn from_base16(id: &str, id_type: SpotifyIdType) -> Option<Self> {
        let mut n = 0_u128;
        for c in id.as_bytes() {
            let d = BASE16_DIGITS.iter().position(|e| e == c)? as u128;
            n *= 16;
            n += d;
        }
        Some(Self::new(n, id_type))
    }

    pub fn from_base62(id: &str, id_type: SpotifyIdType) -> Option<Self> {
        let mut n = 0_u128;
        for c in id.as_bytes() {
            let d = BASE62_DIGITS.iter().position(|e| e == c)? as u128;
            n *= 62;
            n += d;
        }
        Some(Self::new(n, id_type))
    }

    pub fn from_raw(data: &[u8], id_type: SpotifyIdType) -> Option<Self> {
        let n = u128::from_be_bytes(data.try_into().ok()?);
        Some(Self::new(n, id_type))
    }

    pub fn from_uri(uri: &str) -> Option<Self> {
        let gid = uri.split(':').last()?;
        if uri.contains(":episode:") {
            Self::from_base62(gid, SpotifyIdType::Podcast)
        } else if uri.contains(":track:") {
            Self::from_base62(gid, SpotifyIdType::Track)
        } else {
            Self::from_base62(gid, SpotifyIdType::Unknown)
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
