use std::{
    fs, io,
    io::{Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
    thread,
    thread::JoinHandle,
    time::Duration,
};

use symphonia::core::codecs::CodecType;

use crate::{
    audio::{
        decode::{AudioCodecFormat, AudioDecoder},
        decrypt::{AudioDecrypt, AudioKey},
        normalize::NormalizationData,
    },
    cache::CacheHandle,
    cdn::{CdnHandle, CdnUrl},
    error::Error,
    item_id::{FileId, ItemId},
    protocol::metadata::mod_AudioFile::Format,
    util::OffsetFile,
};

use super::storage::{StreamRequest, StreamStorage, StreamWriter};

#[derive(Debug, Clone, Copy)]
pub struct MediaPath {
    pub item_id: ItemId,
    pub file_id: FileId,
    pub file_format: AudioFormat,
    pub duration: Duration,
}

// possibly should be combined with AudioCodecFormat?
#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Mp3,
    OggVorbis,
    Unsupported,
}

impl AudioFormat {
    pub fn from_protocol(format: Format) -> Self {
        use Format::*;
        match format {
            MP3_256 | MP3_320 | MP3_160 | MP3_96 | MP3_160_ENC => Self::Mp3,
            OGG_VORBIS_96 | OGG_VORBIS_160 | OGG_VORBIS_320 => Self::OggVorbis,
            _ => Self::Unsupported,
        }
    }

    pub fn from_codec(codec: CodecType) -> Self {
        use symphonia::core::codecs::*;
        if codec == CODEC_TYPE_MP3 {
            return Self::Mp3;
        } else if codec == CODEC_TYPE_VORBIS {
            return Self::OggVorbis;
        } else {
            return Self::Unsupported;
        }
    }
}

pub enum MediaFile {
    Streamed {
        streamed_file: Arc<StreamedFile>,
        servicing_handle: JoinHandle<()>,
    },
    Cached {
        cached_file: CachedFile,
    },
    Local {
        path: MediaPath,
    },
}

impl MediaFile {
    pub fn supported_audio_formats_for_bitrate(bitrate: usize) -> &'static [Format] {
        match bitrate {
            96 => &[
                Format::OGG_VORBIS_96,
                Format::MP3_96,
                Format::OGG_VORBIS_160,
                Format::MP3_160,
                Format::MP3_160_ENC,
                Format::MP3_256,
                Format::OGG_VORBIS_320,
                Format::MP3_320,
            ],
            160 => &[
                Format::OGG_VORBIS_160,
                Format::MP3_160,
                Format::MP3_160_ENC,
                Format::MP3_256,
                Format::OGG_VORBIS_320,
                Format::MP3_320,
                Format::OGG_VORBIS_96,
                Format::MP3_96,
            ],
            320 => &[
                Format::OGG_VORBIS_320,
                Format::MP3_320,
                Format::MP3_256,
                Format::OGG_VORBIS_160,
                Format::MP3_160,
                Format::MP3_160_ENC,
                Format::OGG_VORBIS_96,
                Format::MP3_96,
            ],
            _ => unreachable!(),
        }
    }

    pub fn open(path: MediaPath, cdn: CdnHandle, cache: CacheHandle) -> Result<Self, Error> {
        let cached_path = cache.audio_file_path(path.file_id);
        if cached_path.exists() {
            let cached_file = CachedFile::open(path, cached_path)?;
            Ok(Self::Cached { cached_file })
        } else {
            let streamed_file = Arc::new(StreamedFile::open(path, cdn, cache)?);
            let servicing_handle = thread::spawn({
                let streamed_file = Arc::clone(&streamed_file);
                move || {
                    streamed_file
                        .service_streaming()
                        .expect("Streaming thread failed");
                }
            });
            Ok(Self::Streamed {
                streamed_file,
                servicing_handle,
            })
        }
    }

    pub fn local(path: MediaPath) -> Self {
        Self::Local { path }
    }

    pub fn path(&self) -> MediaPath {
        match self {
            Self::Streamed { streamed_file, .. } => streamed_file.path,
            Self::Cached { cached_file, .. } => cached_file.path,
            Self::Local { path } => *path,
        }
    }

    pub fn storage(&self) -> Option<&StreamStorage> {
        match self {
            Self::Streamed { streamed_file, .. } => Some(&streamed_file.storage),
            Self::Cached { cached_file, .. } => Some(&cached_file.storage),
            Self::Local { .. } => None,
        }
    }

    pub fn remote_audio_source(
        &self,
        key: AudioKey,
    ) -> Result<(AudioDecoder, NormalizationData), Error> {
        let reader = self
            .storage()
            .expect("storage always set for remote files")
            .reader()?;
        let mut decrypted = AudioDecrypt::new(key, reader);
        let normalization = NormalizationData::parse(&mut decrypted)?;
        let encoded = OffsetFile::new(decrypted, self.header_length())?;
        let decoded = AudioDecoder::new(encoded, self.codec_format())?;
        Ok((decoded, normalization))
    }

    pub fn local_audio_source(&self) -> Result<(AudioDecoder, NormalizationData), Error> {
        let mut reader = fs::File::open(self.path().item_id.to_local())?;
        let normalization = NormalizationData::parse(&mut reader)?;
        let encoded = OffsetFile::new(reader, self.header_length())?;
        let decoded = AudioDecoder::new(encoded, self.codec_format())?;
        Ok((decoded, normalization))
    }

    fn header_length(&self) -> u64 {
        match self.path().file_format {
            AudioFormat::OggVorbis => 167,
            _ => 0,
        }
    }

    fn codec_format(&self) -> AudioCodecFormat {
        match self.path().file_format {
            AudioFormat::OggVorbis => AudioCodecFormat::OggVorbis,
            AudioFormat::Mp3 => AudioCodecFormat::Mp3,
            AudioFormat::Unsupported => unreachable!("unsupported codec"),
        }
    }
}

pub struct StreamedFile {
    path: MediaPath,
    storage: StreamStorage,
    url: CdnUrl,
    cdn: CdnHandle,
    cache: CacheHandle,
}

impl StreamedFile {
    fn open(path: MediaPath, cdn: CdnHandle, cache: CacheHandle) -> Result<StreamedFile, Error> {
        // First, we need to resolve URL of the file contents.
        let url = cdn.resolve_audio_file_url(path.file_id)?;
        log::debug!("resolved file URL: {:?}", url.url);

        // How many bytes we request in the first chunk.
        const INITIAL_REQUEST_LENGTH: u64 = 1024 * 6;

        // Send the initial request, that gives us the total file length and the
        // beginning of the contents.  Use the total length for creating the backing
        // data storage.
        let (total_length, mut initial_data) =
            cdn.fetch_file_range(&url.url, 0, INITIAL_REQUEST_LENGTH)?;
        let storage = StreamStorage::new(total_length)?;

        // Pipe the initial data from the request body into storage.
        io::copy(&mut initial_data, &mut storage.writer()?)?;

        Ok(StreamedFile {
            path,
            storage,
            url,
            cdn,
            cache,
        })
    }

    fn service_streaming(&self) -> Result<(), Error> {
        let mut last_url = self.url.clone();
        let mut fresh_url = || -> Result<CdnUrl, Error> {
            if last_url.is_expired() {
                last_url = self.cdn.resolve_audio_file_url(self.path.file_id)?;
            }
            Ok(last_url.clone())
        };
        let mut download_range = |offset, length| -> Result<(), Error> {
            let thread_name = format!(
                "cdn-{}-{}..{}",
                self.path.file_id.to_base16(),
                offset,
                offset + length
            );
            // TODO: We spawn threads here without any accounting.  Seems wrong.
            thread::Builder::new().name(thread_name).spawn({
                let url = fresh_url()?.url;
                let cdn = self.cdn.clone();
                let cache = self.cache.clone();
                let mut writer = self.storage.writer()?;
                let file_path = self.storage.path().to_path_buf();
                let file_id = self.path.file_id;
                move || {
                    match load_range(&mut writer, &cdn, &url, offset, length) {
                        Ok(_) => {
                            // If the file is completely downloaded, copy it to cache.
                            if writer.is_complete() && !cache.audio_file_path(file_id).exists() {
                                // TODO: We should do this atomically.
                                if let Err(err) = cache.save_audio_file(file_id, file_path) {
                                    log::warn!("failed to save audio file to cache: {:?}", err);
                                }
                            }
                        }
                        Err(err) => {
                            log::error!("failed to download: {}", err);
                            // Range failed to download, remove it from the requested set.
                            writer.mark_as_not_requested(offset, length);
                        }
                    }
                }
            })?;

            Ok(())
        };

        while let Ok(req) = self.storage.receiver().recv() {
            match req {
                StreamRequest::Preload { offset, length } => {
                    if let Err(err) = download_range(offset, length) {
                        log::error!("failed to request audio range: {:?}", err);
                    }
                }
                StreamRequest::Blocked { offset } => {
                    log::info!("blocked at {}", offset);
                }
            }
        }
        Ok(())
    }
}

pub struct CachedFile {
    path: MediaPath,
    storage: StreamStorage,
}

impl CachedFile {
    fn open(path: MediaPath, file_path: PathBuf) -> Result<Self, Error> {
        Ok(Self {
            path,
            storage: StreamStorage::from_complete_file(file_path)?,
        })
    }
}

fn load_range(
    writer: &mut StreamWriter,
    cdn: &CdnHandle,
    url: &str,
    offset: u64,
    length: u64,
) -> Result<(), Error> {
    log::trace!("downloading {}..{}", offset, offset + length);

    // Download range of data from the CDN.  Block until we a have reader of the
    // request body.
    let (_total_length, mut reader) = cdn.fetch_file_range(url, offset, length)?;

    // Pipe it into storage. Blocks until fully written, but readers sleeping on
    // this file should be notified as soon as their offset is covered.
    writer.seek(SeekFrom::Start(offset))?;
    io::copy(&mut reader, writer)?;

    Ok(())
}
