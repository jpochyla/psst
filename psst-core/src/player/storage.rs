use std::{
    fs::File,
    io,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::{Condvar, Mutex};
use rangemap::RangeSet;
use tempfile::NamedTempFile;

pub enum StreamRequest {
    Preload { offset: u64, length: u64 },
    Blocked { offset: u64 },
}

pub struct StreamStorage {
    file: StreamFile,
    data_map: Arc<StreamDataMap>,
    req_receiver: Receiver<StreamRequest>,
    req_sender: Sender<StreamRequest>,
}

pub struct StreamReader {
    reader: File,
    data_map: Arc<StreamDataMap>,
    req_sender: Sender<StreamRequest>,
}

pub struct StreamWriter {
    writer: File,
    data_map: Arc<StreamDataMap>,
}

impl StreamStorage {
    pub fn new(total_size: u64) -> io::Result<StreamStorage> {
        // Use a temporary file for the backing storage, stretched to the full size, so
        // we can seek freely.
        let tmp_file = NamedTempFile::new()?;
        tmp_file.as_file().set_len(total_size)?;

        // Create a channel for requesting downloads of data.
        let (data_req_sender, data_req_receiver) = unbounded();

        Ok(StreamStorage {
            file: StreamFile::Temporary(tmp_file),
            req_receiver: data_req_receiver,
            req_sender: data_req_sender,
            data_map: Arc::new(StreamDataMap {
                total_size,
                downloaded: Mutex::new(RangeSet::new()),
                requested: Mutex::new(RangeSet::new()),
                condvar: Condvar::new(),
            }),
        })
    }

    pub fn from_complete_file(path: PathBuf) -> io::Result<StreamStorage> {
        // Query for the total file size.
        let total_size = path.metadata()?.len();

        // Create the data channel even though it will not be used, as the file should
        // be complete.  We could also turn these into `Option`s.
        let (data_req_sender, data_req_receiver) = unbounded();

        // Because the file is complete, let's mark the full range of data as
        // downloaded.  We mark it as requested as well, because the downloaded set is
        // always ⊆ the requested set.
        let mut downloaded_set = RangeSet::new();
        downloaded_set.insert(0..total_size);
        let requested_set = downloaded_set.clone();

        Ok(StreamStorage {
            file: StreamFile::Persisted(path),
            req_receiver: data_req_receiver,
            req_sender: data_req_sender,
            data_map: Arc::new(StreamDataMap {
                total_size,
                downloaded: Mutex::new(downloaded_set),
                requested: Mutex::new(requested_set),
                condvar: Condvar::new(),
            }),
        })
    }

    pub fn reader(&self) -> io::Result<StreamReader> {
        Ok(StreamReader {
            reader: self.file.reopen()?, // Re-opened files have a starting seek position.
            data_map: self.data_map.clone(),
            req_sender: self.req_sender.clone(),
        })
    }

    pub fn writer(&self) -> io::Result<StreamWriter> {
        Ok(StreamWriter {
            writer: self.file.reopen()?, // Re-opened files have a starting seek position.
            data_map: self.data_map.clone(),
        })
    }

    pub fn receiver(&self) -> &Receiver<StreamRequest> {
        &self.req_receiver
    }

    pub fn path(&self) -> &Path {
        self.file.path()
    }
}

enum StreamFile {
    Temporary(NamedTempFile),
    Persisted(PathBuf),
}

impl StreamFile {
    fn reopen(&self) -> io::Result<File> {
        match self {
            StreamFile::Temporary(tmp_file) => tmp_file.reopen(),
            StreamFile::Persisted(path) => File::open(path),
        }
    }

    fn path(&self) -> &Path {
        match self {
            StreamFile::Temporary(tmp_file) => tmp_file.path(),
            StreamFile::Persisted(path) => path,
        }
    }
}

impl StreamWriter {
    pub fn is_complete(&self) -> bool {
        self.data_map.is_complete()
    }

    pub fn mark_as_not_requested(&self, offset: u64, length: u64) {
        self.data_map.mark_as_not_requested(offset, length);
    }
}

impl Write for StreamWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let position = self.writer.stream_position()?;
        let written = self.writer.write(buf)?;
        self.data_map.mark_as_downloaded(position, written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Seek for StreamWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.writer.seek(pos)
    }
}

const MINIMUM_READ_LENGTH: u64 = 1024 * 64;
const PREFETCH_READ_LENGTH: u64 = 1024 * 256;

impl Read for StreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let position = self.reader.stream_position()?;
        let remaining_len = self.data_map.remaining(position);
        if remaining_len == 0 {
            return Ok(0); // We're at the end of the file.
        }
        let needed_len = remaining_len.min(buf.len() as u64);

        // Make sure that at least `PREFETCH_READ_LENGTH` bytes in front of the reading
        // head is requested.
        let prefetch_len = needed_len.max(PREFETCH_READ_LENGTH).min(remaining_len);
        for (pos, len) in self.data_map.not_yet_requested(position, prefetch_len) {
            let req_len = len.max(MINIMUM_READ_LENGTH);
            self.data_map.mark_as_requested(pos, req_len);
            self.req_sender
                .send(StreamRequest::Preload {
                    offset: pos,
                    length: req_len,
                })
                .expect("Data request channel was closed");
        }

        // Block and wait until at least a part of the range is available, and read it.
        let ready_to_read_len = self.data_map.wait_for(position, |offset| {
            // Notify the servicing thread we are blocked, so it can possibly prioritize the
            // blocked offset.
            self.req_sender
                .send(StreamRequest::Blocked { offset })
                .expect("Data request channel was closed");
        });
        assert!(ready_to_read_len > 0);
        self.reader
            .read(&mut buf[..ready_to_read_len.min(needed_len) as usize])
    }
}

impl Seek for StreamReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.reader.seek(pos)
    }
}

#[derive(Debug)]
struct StreamDataMap {
    total_size: u64,
    // Contains ranges of data requested from the server.  Downloaded ranges are not removed from
    // this set.
    requested: Mutex<RangeSet<u64>>,
    // Contains ranges of data sure to be present in the backing storage.  Always a subset of the
    // requested ranges.
    downloaded: Mutex<RangeSet<u64>>,
    condvar: Condvar,
}

impl StreamDataMap {
    /// Return the number of bytes from offset until the end of the file.
    fn remaining(&self, offset: u64) -> u64 {
        self.total_size.saturating_sub(offset)
    }

    /// Return a vector of sub-ranges of `offset..offset+length` that have not
    /// yet been requested from the backend.
    fn not_yet_requested(&self, offset: u64, length: u64) -> Vec<(u64, u64)> {
        self.requested
            .lock()
            .gaps(&(offset..offset + length))
            .into_iter()
            .map(|r| range_to_offset_and_length(&r))
            .collect()
    }

    /// Mark given range as requested from the backend, so we can avoid
    /// requesting it more than once.
    fn mark_as_requested(&self, offset: u64, length: u64) {
        self.requested.lock().insert(offset..offset + length);
    }

    /// Remove range previously marked as requested.
    fn mark_as_not_requested(&self, offset: u64, length: u64) {
        self.requested.lock().remove(offset..offset + length);
    }

    /// Mark the range as downloaded and notify the `self.condvar`, so tasks
    /// currently blocked in `self.wait_for` are woken up.
    fn mark_as_downloaded(&self, offset: u64, length: u64) {
        self.downloaded.lock().insert(offset..offset + length);
        self.condvar.notify_all();
    }

    /// Block, waiting until at least some data at given offset is downloaded.
    /// Returns length that is available.  See `self.mark_as_downloaded`.
    fn wait_for(&self, offset: u64, blocking_callback: impl Fn(u64)) -> u64 {
        let mut downloaded = self.downloaded.lock();
        let mut called_callback = false;
        loop {
            if let Some(range) = downloaded.get(&offset) {
                let (over_ofs, over_len) = range_to_offset_and_length(range);
                let offset_from_overlapping = offset - over_ofs;
                let available_len = over_len - offset_from_overlapping;
                // There is `available_len` bytes of data downloaded, stop waiting.
                break available_len;
            } else {
                // Call the blocking callback, but only the first time we are waiting.
                if !called_callback {
                    called_callback = true;
                    blocking_callback(offset);
                }
                // There are no overlaps, wait.
                self.condvar.wait(&mut downloaded);
            }
        }
    }

    // Returns true if data is completely downloaded.
    fn is_complete(&self) -> bool {
        self.downloaded
            .lock()
            .gaps(&(0..self.total_size))
            .next()
            .is_none()
    }
}

fn range_to_offset_and_length(range: &Range<u64>) -> (u64, u64) {
    (range.start, range.end - range.start)
}
