use crate::{cache::CacheHandle, cdn::CdnHandle, error::Error, item_id::FileId};
use std::io::{self, Read, Seek, SeekFrom};

pub(super) fn encrypted_file(
    id: FileId,
    cdn: &CdnHandle,
    cache: &CacheHandle,
) -> Result<(), Error> {
    const CHUNK: u64 = 1024 * 1024;
    const MAX_TRACK: u64 = 512 * 1024 * 1024;
    let mut file = tempfile::NamedTempFile::new()?;
    let mut url = cdn.resolve_audio_file_url(id)?;
    let mut offset = 0;
    let mut total = None;
    while total.is_none_or(|length| offset < length) {
        let mut completed = false;
        for attempt in 0..3 {
            if attempt > 0 || url.expires <= std::time::Instant::now() {
                url = cdn.resolve_audio_file_url(id)?;
            }
            let result = (|| -> Result<u64, Error> {
                let (length, reader) = cdn.fetch_file_range(&url.url, offset, CHUNK)?;
                if length == 0 || length > MAX_TRACK || total.is_some_and(|old| old != length) {
                    return Err(Error::ConfigError(
                        "Unexpected encrypted track length".into(),
                    ));
                }
                total = Some(length);
                let expected = CHUNK.min(length.saturating_sub(offset));
                file.seek(SeekFrom::Start(offset))?;
                let written = io::copy(&mut reader.take(expected), &mut file)?;
                if written != expected {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
                }
                Ok(written)
            })();
            match result {
                Ok(written) => {
                    offset += written;
                    completed = true;
                    break;
                }
                Err(Error::IoError(error))
                    if attempt < 2
                        && matches!(
                            error.kind(),
                            io::ErrorKind::TimedOut
                                | io::ErrorKind::UnexpectedEof
                                | io::ErrorKind::ConnectionReset
                        ) =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(250 * (attempt + 1)));
                }
                Err(error) => return Err(error),
            }
        }
        if !completed {
            return Err(Error::ConfigError("Download did not finish".into()));
        }
    }
    file.as_file().set_len(offset)?;
    cache.save_audio_file(id, file.path().to_path_buf())?;
    Ok(())
}
