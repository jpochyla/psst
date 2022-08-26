use std::{
    collections::HashMap,
    convert::TryInto,
    fs::File,
    io::{self, Cursor, Read},
    path::PathBuf,
    str,
    sync::Arc,
    time::Duration,
    vec::Vec,
};

use druid::im::Vector;
use serde::Deserialize;
use serde_json::Value;

use crate::data::{config::Config, AlbumLink, ArtistLink, Image, Track, TrackId};
use psst_core::item_id::ItemId;

/**
 * All local files registered by the Spotify file can be found in the file
 * located at: <Spotify config>/Users/<username>-user/local-files.bnk
 *
 * While this is not a complete reverse engineering of the way it is stored,
 * it suffices for now. The file appears to be saved as a custom fork of
 * Google's ProtoBuf format.
 *
 * The file starts with "SPCO" as the magic followed by 0x13, 0x00*4 --
 * couldn't tell what these bytes do.
 *
 * After that there is 0x11 and "LocalFilesStorage". Note the 0x11
 * corresponds to the strings length and I can only assume this is somehow
 * used to load the correct proto definition.
 *
 * Interestingly enough, Spotify bnk files are chunked. After the above
 * there is a little endian encoded value of the number of bytes in a chunk
 * maxing out at 0x1FE3. After exactly this many bytes another chunk size
 * will be present. This can even interrupt strings however it does not
 * include the 0x00, 0x00 that ends every file (I think).
 *
 * Following the first chunk size is 0x60, which I think is used to signify
 * an array in this ProtoBuf-like language. Doesn't matter too much. The
 * number of elements in the array then follows encoded as a varint
 * (https://developers.google.com/protocol-buffers/docs/encoding).
 *
 * The bytes 0x94 0x00 seem to be present in every file I have used, I don't
 * know what they represent.
 *
 * Then a track entry is presented as: `Title -> Artist -> Album ->
 * Local Path -> Trailer`. I have no clue what the trailer represents other
 * than directly after the `Local Path` the bytes `0x08 -> <varint encoding
 * of track length>`, not relevant so far. The title, artist, etc. are each
 * encoded in the following format `0x09(string identifier) -> <varint
 * string size> -> string`. If the varint size is zero this is a null
 * string.
 *
 * The end of a trailer for a given track is signified by 0x78, 0x04 from
 * what I can tell.
 */

const MAGIC_BYTES: &[u8] = b"SPCO";
const FILE_TYPE: &[u8] = b"LocalFilesStorage";

const ARRAY_SIGNATURE: u8 = 0x60;
const STRING_SIGNATURE: u8 = 0x09;
const TRAILER_END: [u8; 2] = [0x78u8, 0x04u8];

#[derive(Clone, Debug)]
pub struct LocalTrack {
    title: Arc<str>,
    path: Arc<str>,
    album: Arc<str>,
    artist: Arc<str>,
}

pub struct LocalTrackManager {
    tracks: HashMap<Arc<str>, Vec<LocalTrack>>,
}

impl LocalTrackManager {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
        }
    }

    pub fn load_tracks_for_user(&mut self, username: &str) -> io::Result<()> {
        let file_path =
            Config::spotify_local_files_file(username).ok_or(io::ErrorKind::NotFound)?;
        let local_file = File::open(&file_path)?;
        let mut reader = LocalTracksReader::new(local_file)?;

        log::info!("parsing local tracks: {:?}", file_path);

        // Start reading the track array.
        let num_tracks = reader.read_array()?;
        if num_tracks > 0 {
            reader.advance(2)?; // Skip `0x94 0x00`.
        }

        self.tracks.clear();

        for n in 1..=num_tracks {
            let title = reader.read_string()?;
            let artist = reader.read_string()?;
            let album = reader.read_string()?;
            let path = reader.read_string()?;
            let track = LocalTrack {
                title: title.into(),
                path: path.into(),
                album: album.into(),
                artist: artist.into(),
            };
            self.tracks
                .entry(track.title.clone())
                .or_default()
                .push(track);
            if reader.advance_until(&TRAILER_END).is_err() {
                if n != num_tracks {
                    log::warn!("found EOF but missing {} tracks", num_tracks - n);
                }
                break;
            }
        }

        Ok(())
    }

    pub fn find_local_track(&self, track_json: Value) -> Option<Arc<Track>> {
        let local_track: LocalTrackJson = match serde_json::from_value(track_json) {
            Ok(t) => t,
            Err(e) => {
                log::error!("error parsing track {:?}", e);
                return None;
            }
        };

        let matching_tracks = self.tracks.get(&local_track.name)?;

        for parsed_track in matching_tracks {
            let path: PathBuf = match (&*parsed_track.path).try_into() {
                Ok(t) => t,
                Err(e) => {
                    log::error!("error loading local file {:?}", e);
                    continue;
                }
            };

            if Self::is_matching_in_addition_to_title(parsed_track, &local_track) {
                return Some(Arc::new(Track {
                    id: TrackId(ItemId::from_local(path)),
                    name: local_track.name,
                    album: local_track.album.map(|local_album| {
                        AlbumLink {
                            id: local_album.id.unwrap_or_else(|| "null".into()), // TODO: Invalid ID
                            name: local_album.name,
                            images: local_album.images,
                        }
                    }),
                    artists: local_track
                        .artists
                        .into_iter()
                        .map(|artist| ArtistLink {
                            id: artist.id.unwrap_or_else(|| "null".into()), // TODO: Invalid ID
                            name: artist.name,
                        })
                        .collect(),
                    duration: local_track.duration,
                    disc_number: local_track.disc_number,
                    track_number: local_track.track_number,
                    explicit: local_track.explicit,
                    is_local: local_track.is_local,
                    local_path: Some(parsed_track.path.clone()),
                    // TODO: Change this to true once playback is supported.
                    is_playable: Some(false),
                    popularity: local_track.popularity,
                }));
            }
        }

        None
    }

    fn is_matching_in_addition_to_title(t1: &LocalTrack, t2: &LocalTrackJson) -> bool {
        // TODO: More checks on if a local track may return multiple artists from
        // Spotify's web facing API.
        let artist_mismatch = t2
            .artists
            .iter()
            .next()
            .map_or(false, |t2_artist| t2_artist.name != t1.artist);
        let album_mismatch = t2
            .album
            .as_ref()
            .map_or(false, |t2_album| t2_album.name != t1.album);
        !(artist_mismatch || album_mismatch)
    }
}

// Spotify can do some weird stuff with local track APIs so serializing with
// `serde` requires a good amount of workarounds.  The following structs reflect
// the ones in the `data` module, with modifications to allow for null values.

#[derive(Clone, Debug, Deserialize)]
struct LocalAlbumLinkJson {
    #[serde(default)]
    pub id: Option<Arc<str>>,
    pub name: Arc<str>,
    #[serde(default)]
    pub images: Vector<Image>,
}

#[derive(Clone, Debug, Deserialize)]
struct LocalArtistLinkJson {
    #[serde(default)]
    pub id: Option<Arc<str>>,
    pub name: Arc<str>,
}

#[derive(Deserialize)]
struct LocalTrackJson {
    #[serde(default)]
    pub id: Option<TrackId>,
    pub name: Arc<str>,
    #[serde(default)]
    pub album: Option<LocalAlbumLinkJson>,
    pub artists: Vector<LocalArtistLinkJson>,
    #[serde(rename = "duration_ms")]
    #[serde(deserialize_with = "crate::data::utils::deserialize_millis")]
    pub duration: Duration,
    pub disc_number: usize,
    pub track_number: usize,
    pub explicit: bool,
    pub is_local: bool,
    pub is_playable: Option<bool>,
    pub popularity: Option<u32>,
}

struct LocalTracksReader {
    chunked: ChunkedReader,
}

impl LocalTracksReader {
    fn new(file: File) -> io::Result<Self> {
        Ok(Self {
            chunked: Self::parse_file(file)?,
        })
    }

    /// Checks if `file` is in correct format and prepares it for reading.
    fn parse_file(mut file: File) -> io::Result<ChunkedReader> {
        // Validate the magic.
        let magic = read_bytes(&mut file, 4)?;
        if magic != MAGIC_BYTES {
            return Err(io::ErrorKind::InvalidData.into());
        }
        // Skip `0x13, 0x00*4`.
        advance(&mut file, 5)?;
        // Validate the file-type marker.
        let file_type = read_bytes(&mut file, 18)?;
        if file_type[0] != FILE_TYPE.len() as u8 || &file_type[1..] != FILE_TYPE {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(ChunkedReader::new(file))
    }

    fn advance(&mut self, len: usize) -> io::Result<()> {
        advance(&mut self.chunked, len)
    }

    fn advance_until(&mut self, bytes: &[u8]) -> io::Result<()> {
        advance_until(&mut self.chunked, bytes)
    }

    fn read_string(&mut self) -> io::Result<String> {
        let signature = read_u8(&mut self.chunked)?;
        if signature != STRING_SIGNATURE {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let str_size = read_uvarint(&mut self.chunked)?;
        let str_buf = read_utf8(&mut self.chunked, str_size as usize)?;
        Ok(str_buf)
    }

    fn read_array(&mut self) -> io::Result<usize> {
        let signature = read_u8(&mut self.chunked)?;
        if signature != ARRAY_SIGNATURE {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let num_entries = read_uvarint(&mut self.chunked)? as usize;
        Ok(num_entries)
    }
}

/// Implements a `Read` trait over the chunked file format described above.
struct ChunkedReader {
    inner: File,
    chunk: Cursor<Vec<u8>>,
}

impl ChunkedReader {
    fn new(inner: File) -> Self {
        Self {
            inner,
            chunk: Cursor::default(),
        }
    }

    fn read_next_chunk(&mut self) -> io::Result<()> {
        // Two LE bytes of chunk length.
        let size = read_u16_le(&mut self.inner)?;
        // Chunk content.
        let buf = read_bytes(&mut self.inner, size as usize)?;
        self.chunk = Cursor::new(buf);
        Ok(())
    }
}

impl Read for ChunkedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let n = self.chunk.read(buf)?;
            if n > 0 {
                break Ok(n);
            } else {
                // `self.chunk` is empty, read the next one.  Returns `Err` on EOF.
                self.read_next_chunk()?;
            }
        }
    }
}

/// Helper, reads a byte from `f` or returns `Err`.
fn read_u8(f: &mut impl io::Read) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    f.read_exact(&mut buf)?;
    Ok(buf[0])
}

/// Helper, reads little-endian `u16` or returns `Err`.
fn read_u16_le(f: &mut impl io::Read) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    f.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

/// Helper, reads ProtoBuf-style unsigned varint from `f` or returns `Err`.
fn read_uvarint(f: &mut impl io::Read) -> io::Result<u64> {
    let mut shift: u64 = 0;
    let mut ret: u64 = 0;

    loop {
        let byte = read_u8(f)?;
        let has_msb: bool = (byte & !0b01111111) != 0;
        ret |= ((byte & 0b01111111) as u64) << shift;

        if has_msb {
            shift += 7;
        } else {
            break;
        }
    }

    Ok(ret)
}

/// Helper, reads a `Vec<u8>` of length `len` from `f` or returns `Err`.
fn read_bytes(f: &mut impl io::Read, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// Helper, reads a UTF-8 string of length `len` from `f` or returns `Err`.
fn read_utf8(f: &mut impl io::Read, len: usize) -> io::Result<String> {
    let buf = read_bytes(f, len)?;
    String::from_utf8(buf).map_err(|_| io::ErrorKind::InvalidData.into())
}

/// Helper, skips `len` bytes of `f` or returns `Err`.
fn advance(f: &mut impl io::Read, len: usize) -> io::Result<()> {
    for _ in 0..len {
        read_u8(f)?;
    }
    Ok(())
}

/// Helper, skips bytes of `f` until an exact continuous `bytes` match is found,
/// or returns `Err`.
pub fn advance_until(f: &mut impl io::Read, bytes: &[u8]) -> io::Result<()> {
    let mut i = 0;
    while i < bytes.len() {
        loop {
            let r = read_u8(f)?;
            if r == bytes[i] {
                i += 1; // Match, continue with the next byte of `bytes`.
                break;
            } else {
                i = 0; // Mismatch, start at the beginning again.
                if r == bytes[i] {
                    i += 1;
                }
            }
        }
    }
    Ok(())
}
