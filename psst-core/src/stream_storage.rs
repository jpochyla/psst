use iset::IntervalSet;
use std::{
    fs::File,
    io,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Arc, Condvar, Mutex,
    },
};
use tempfile::NamedTempFile;

pub struct StreamStorage {
    file: StreamFile,
    data_map: Arc<StreamDataMap>,
    data_req_receiver: Receiver<(u64, u64)>,
    data_req_sender: Sender<(u64, u64)>,
}

pub struct StreamReader {
    reader: File,
    data_map: Arc<StreamDataMap>,
    data_req_sender: Sender<(u64, u64)>,
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
        let (data_req_sender, data_req_receiver) = mpsc::channel();

        Ok(StreamStorage {
            file: StreamFile::Temporary(tmp_file),
            data_req_receiver,
            data_req_sender,
            data_map: Arc::new(StreamDataMap {
                total_size,
                downloaded: Mutex::new(IntervalSet::new()),
                requested: Mutex::new(IntervalSet::new()),
                condvar: Condvar::new(),
            }),
        })
    }

    pub fn from_complete_file(path: PathBuf) -> io::Result<StreamStorage> {
        // Query for the total file size.
        let total_size = path.metadata()?.len();

        // Create the data channel even though it will not be used, as the file should
        // be complete.  We could also turn these into `Option`s.
        let (data_req_sender, data_req_receiver) = mpsc::channel();

        // Because the file is complete, let's mark the full range of data as
        // downloaded.  We mark it as requested as well, because the downloaded set is
        // always ⊆ the requested set.
        let mut downloaded_set = IntervalSet::new();
        downloaded_set.insert(0..total_size);
        let requested_set = downloaded_set.clone();

        Ok(StreamStorage {
            file: StreamFile::Persisted(path),
            data_req_receiver,
            data_req_sender,
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
            data_req_sender: self.data_req_sender.clone(),
        })
    }

    pub fn writer(&self) -> io::Result<StreamWriter> {
        Ok(StreamWriter {
            writer: self.file.reopen()?, // Re-opened files have a starting seek position.
            data_map: self.data_map.clone(),
        })
    }

    pub fn receiver(&mut self) -> &mut Receiver<(u64, u64)> {
        &mut self.data_req_receiver
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
}

impl Write for StreamWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let position = self.writer.seek(SeekFrom::Current(0))?;
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

const MINIMUM_READ_LENGTH: u64 = 1024 * 128;
const PREFETCH_READ_LENGTH: u64 = 1024 * 256;

impl Read for StreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let position = self.reader.seek(SeekFrom::Current(0))?;
        let remaining_len = self.data_map.remaining(position);
        if remaining_len == 0 {
            return Ok(0); // We're at the end of the file.
        }
        let needed_len = remaining_len.min(buf.len() as u64);

        // Make sure that at least `PREFETCH_READ_LENGTH` bytes in front of the reading
        // head is requested.
        let prefetch_len = needed_len.max(PREFETCH_READ_LENGTH).min(remaining_len);
        for (pos, len) in self.data_map.not_yet_requested(position, prefetch_len) {
            let req_pos = round_down_to_multiple(pos, 4);
            let req_len = round_up_to_multiple(len, 4).max(MINIMUM_READ_LENGTH);
            self.data_map.mark_as_requested(req_pos, req_len);
            self.data_req_sender
                .send((req_pos, req_len))
                .expect("Data request channel was closed");
        }

        // Block and wait until at least a part of the range is available, and read it.
        let ready_to_read_len = self.data_map.wait_for(position);
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

fn round_down_to_multiple(n: u64, m: u64) -> u64 {
    n - n % m
}

fn round_up_to_multiple(n: u64, m: u64) -> u64 {
    n + (m - n % m)
}

#[derive(Debug)]
struct StreamDataMap {
    total_size: u64,
    // Contains ranges of data sure to be present in the backing storage.  Always a subset of the
    // requested ranges.
    downloaded: Mutex<IntervalSet<u64>>,
    // Contains ranges of data requested from the server.  Downloaded ranges are not removed from
    // this set.
    requested: Mutex<IntervalSet<u64>>,
    condvar: Condvar,
}

impl StreamDataMap {
    /// Return the number of bytes from offset until the end of the file.
    fn remaining(&self, offset: u64) -> u64 {
        self.total_size.saturating_sub(offset)
    }

    /// Mark the range as downloaded and notify the `self.condvar`, so tasks
    /// currently blocked in `self.wait_for` are woken up.
    fn mark_as_downloaded(&self, offset: u64, length: u64) {
        self.downloaded
            .lock()
            .expect("Failed to acquire data map lock")
            .insert(offset..offset + length);
        self.condvar.notify_all();
    }

    /// Block, waiting until at least some data at given offset is downloaded.
    /// Returns length that is available.  See `self.mark_as_downloaded`.
    fn wait_for(&self, offset: u64) -> u64 {
        let mut printed_warning = false;
        let mut available_len = 0; // Resulting length.

        let downloaded = self
            .downloaded
            .lock()
            .expect("Failed to acquire data map lock");

        let _mutex_guard = self
            .condvar
            .wait_while(downloaded, |downloaded| {
                if let Some(range) = downloaded.overlap(offset).next() {
                    let (over_ofs, over_len) = range_to_offset_and_length(range);
                    let offset_from_overlapping = offset - over_ofs;
                    available_len = over_len - offset_from_overlapping;
                    // There is `available_len` bytes of data downloaded, stop waiting.
                    false
                } else {
                    // Log we are waiting for the network, but only the first time.
                    if !printed_warning {
                        log::info!("blocked at {}", offset);
                        printed_warning = true;
                    }
                    // There are no overlaps, wait.
                    true
                }
            })
            .expect("Failed to acquire data map lock");
        available_len
    }

    /// Mark given range as requested from the backend, so we can avoid
    /// requesting it more than once.
    fn mark_as_requested(&self, offset: u64, length: u64) {
        self.requested
            .lock()
            .expect("Failed to acquire data map lock")
            .insert(offset..offset + length);
    }

    /// Return an iterator of sub-ranges of `offset..offset+length` that have
    /// not yet been requested from the backend.
    fn not_yet_requested(&self, offset: u64, length: u64) -> impl IntoIterator<Item = (u64, u64)> {
        let requested = self
            .requested
            .lock()
            .expect("Failed to acquire data map lock");
        let overlaps = requested.iter(offset..offset + length);
        interval_difference(offset..offset + length, overlaps)
            .into_iter()
            .map(range_to_offset_and_length)
    }

    // Returns true if data is completely downloaded.
    fn is_complete(&self) -> bool {
        let downloaded = self
            .downloaded
            .lock()
            .expect("Failed to acquire data map lock");
        let overlaps = downloaded.iter(0..self.total_size);
        interval_difference(0..self.total_size, overlaps).is_empty()
    }
}

fn range_to_offset_and_length(range: Range<u64>) -> (u64, u64) {
    (range.start, range.end - range.start)
}

/// Return all sub-ranges of `range` that are not covered by any of the
/// `sorted_intervals`.
fn interval_difference(
    range: Range<u64>,
    sorted_intervals: impl IntoIterator<Item = Range<u64>>,
) -> Vec<Range<u64>> {
    let mut acc = Vec::new();
    let mut end = range.start;
    for i in sorted_intervals {
        if !i.contains(&end) {
            acc.push(end..i.start);
        }
        end = i.end;
    }
    if range.contains(&end) {
        acc.push(end..range.end);
    }
    acc
}
