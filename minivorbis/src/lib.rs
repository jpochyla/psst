#![allow(clippy::all)]

use std::{
    convert::TryInto,
    error, fmt,
    fmt::Formatter,
    io::{self, Read, Seek},
    mem,
    os::raw,
    slice,
};

pub struct Decoder<R: Read + Seek> {
    data: Box<State<R>>,
    pub sample_rate: u32,
    pub channels: u8,
}

pub const TYPICAL_PACKET_CAP: usize = 2048;

impl<R: Read + Seek> Decoder<R> {
    pub fn new(reader: R) -> Result<Decoder<R>, Error> {
        let mut data = Box::new(State {
            vorbis: unsafe { mem::zeroed() },
            read_error: None,
            bitstream: 0,
            reader,
        });

        unsafe {
            let datasource = data.as_mut() as *mut _ as *mut _;
            let res_code = minivorbis_sys::ov_open_callbacks(
                datasource,
                &mut data.vorbis,
                std::ptr::null(), // zero initial data
                0,                // zero initial data
                minivorbis_sys::ov_callbacks {
                    read_func: Some(read_func::<R>),
                    seek_func: Some(seek_func::<R>),
                    tell_func: Some(tell_func::<R>),
                    close_func: None,
                },
            );
            Error::from_code(res_code)?;
        }

        let info = unsafe {
            minivorbis_sys::ov_info(
                &mut data.vorbis,
                -1, // for current bitstream
            )
            .as_ref()
            .unwrap()
        };
        let channels = info.channels.try_into().unwrap();
        let sample_rate = info.rate.try_into().unwrap();

        Ok(Decoder {
            data,
            channels,
            sample_rate,
        })
    }

    pub fn seek_to_pcm(&mut self, frame: u64) -> Result<(), Error> {
        let res_code = unsafe {
            minivorbis_sys::ov_pcm_seek(&mut self.data.vorbis, frame as minivorbis_sys::ogg_int64_t)
        };
        Error::from_code(res_code)?;
        Ok(())
    }

    pub fn position_in_pcm(&mut self) -> Result<u64, Error> {
        let frame = unsafe { minivorbis_sys::ov_pcm_tell(&mut self.data.vorbis) };
        if frame < 0 {
            Err(Error::from_code(frame as raw::c_int).unwrap_err())
        } else {
            Ok(frame as u64)
        }
    }

    pub fn read_packet(&mut self, samples: &mut Vec<i16>) -> Result<usize, Error> {
        let buf = samples.as_mut_ptr();
        let buf_nbytes = samples.capacity() * 2; // 2 bytes in i16
        let previous_bitstream = self.data.bitstream;
        match unsafe {
            minivorbis_sys::ov_read(
                &mut self.data.vorbis,
                buf as *mut raw::c_char,
                buf_nbytes as raw::c_int,
                0, // little endian
                2, // 2 bytes in i16
                1, // signed data
                &mut self.data.bitstream,
            )
        } {
            0 => {
                // Either the underlying reader reached the EOF, or encountered an error.
                match self.data.read_error.take() {
                    Some(err) => Err(Error::ReadError(err)),
                    None => Ok(0), // EOF
                }
            }
            code if code < 0 => {
                // Return a `minivorbis` error.
                let err = Error::from_code(code as raw::c_int).unwrap_err();
                Err(err)
            }
            read_bytes => {
                if previous_bitstream != self.data.bitstream {
                    // The section has changed, assert that the number of channels and the sample
                    // rate is the same.
                    let info = unsafe {
                        minivorbis_sys::ov_info(
                            &mut self.data.vorbis,
                            -1, // for current bitstream
                        )
                        .as_ref()
                        .unwrap()
                    };
                    let channels: u8 = info.channels.try_into().unwrap();
                    let sample_rate: u32 = info.rate.try_into().unwrap();
                    assert_eq!(
                        channels, self.channels,
                        "the number of channels have changed between sections",
                    );
                    assert_eq!(
                        sample_rate, self.sample_rate,
                        "the sample rate have changed between sections",
                    );
                }
                // We have read `read_bytes` of bytes. Convert to the number of read samples,
                // adjust the `samples` length, and return.
                let read_samples = read_bytes as usize / 2; // 2 bytes in i16
                unsafe { samples.set_len(read_samples) };
                Ok(read_samples)
            }
        }
    }
}

impl<R: Read + Seek> Drop for Decoder<R> {
    fn drop(&mut self) {
        unsafe {
            minivorbis_sys::ov_clear(&mut self.data.vorbis);
        }
    }
}

struct State<R: Read + Seek> {
    reader: R,
    vorbis: minivorbis_sys::OggVorbis_File,
    bitstream: raw::c_int,
    read_error: Option<io::Error>,
}

unsafe impl<R: Read + Seek + Send> Send for State<R> {}

extern "C" fn read_func<R: Read + Seek>(
    ptr: *mut raw::c_void,
    size: usize,
    nmemb: usize,
    datasource: *mut raw::c_void,
) -> usize {
    let data = unsafe { &mut *(datasource as *mut State<R>) };

    // In practice `minivorbis` always sets size to 1. This assumption makes things
    // much simpler.
    assert_eq!(size, 1);

    let buffer = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, nmemb as usize) };

    loop {
        match data.reader.read(buffer) {
            Ok(read_bytes) => {
                return read_bytes as usize;
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                // Ignore and try again.
            }
            Err(err) => {
                // Keep the error and pick it up later in `Decoder::read_packet`.
                data.read_error.replace(err);

                // Correctly, we should set `errno` to indicate a reading error to `minivorbis`,
                // but because interfacing `errno` is a hassle in Rust, let's pretend we have an
                // EOF.
                return 0;
            }
        }
    }
}

extern "C" fn seek_func<R: Read + Seek>(
    datasource: *mut raw::c_void,
    offset: minivorbis_sys::ogg_int64_t,
    whence: raw::c_int,
) -> raw::c_int {
    let data = unsafe { &mut *(datasource as *mut State<R>) };

    let pos = match whence as u32 {
        minivorbis_sys::SEEK_SET => io::SeekFrom::Start(offset as u64),
        minivorbis_sys::SEEK_CUR => io::SeekFrom::Current(offset),
        minivorbis_sys::SEEK_END => io::SeekFrom::End(offset),
        _ => unreachable!(),
    };
    match data.reader.seek(pos) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

extern "C" fn tell_func<R: Read + Seek>(datasource: *mut raw::c_void) -> raw::c_long {
    let data = unsafe { &mut *(datasource as *mut State<R>) };

    data.reader
        .seek(io::SeekFrom::Current(0))
        .map(|v| v as raw::c_long)
        .unwrap_or(-1)
}

#[derive(Debug)]
pub enum Error {
    ReadError(io::Error),
    NotVorbis,
    VersionMismatch,
    BadHeader,
    InvalidSetup,
    Hole,
    Unimplemented,
}

impl Error {
    fn from_code(code: raw::c_int) -> Result<(), Self> {
        match code {
            0 => Ok(()),
            minivorbis_sys::OV_ENOTVORBIS => Err(Self::NotVorbis),
            minivorbis_sys::OV_EVERSION => Err(Self::VersionMismatch),
            minivorbis_sys::OV_EBADHEADER => Err(Self::BadHeader),
            minivorbis_sys::OV_EINVAL => Err(Self::InvalidSetup),
            minivorbis_sys::OV_HOLE => Err(Self::Hole),
            minivorbis_sys::OV_EIMPL => Err(Self::Unimplemented),
            minivorbis_sys::OV_EFAULT => {
                panic!("Internal vorbis error");
            }
            unknown_code => {
                panic!("Unknown vorbis error: {}", unknown_code);
            }
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self {
            Error::ReadError(err) => err.fmt(f),
            Error::NotVorbis => write!(f, "OV_ENOTVORBIS"),
            Error::VersionMismatch => write!(f, "OV_EVERSION"),
            Error::BadHeader => write!(f, "OV_EBADHEADER"),
            Error::InvalidSetup => write!(f, "OV_EINVAL"),
            Error::Hole => write!(f, "OV_HOLE"),
            Error::Unimplemented => write!(f, "OV_EIMPL"),
        }
    }
}
