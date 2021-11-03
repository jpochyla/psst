use std::{io, time::Duration};

use symphonia::{
    core::{
        audio::{AudioBufferRef, Channels},
        codecs::Decoder,
        errors::Error as SymphoniaError,
        formats::{FormatReader, SeekMode, SeekTo},
        io::{MediaSource, MediaSourceStream},
        units::TimeStamp,
    },
    default::{codecs::VorbisDecoder, formats::OggReader},
};

use crate::{error::Error, util::FileWithConstSize};

impl<T> MediaSource for FileWithConstSize<T>
where
    T: io::Read + io::Seek + Send,
{
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len())
    }
}

pub struct AudioDecoder {
    track_id: u32, // Internal OGG track index.
    decoder: Box<dyn Decoder>,
    format: Box<dyn FormatReader>,
    position: TimeStamp,
}

impl AudioDecoder {
    pub fn new<T>(input: T) -> Result<Self, Error>
    where
        T: io::Read + io::Seek + Send + 'static,
    {
        let mss_opts = Default::default();
        let mss = MediaSourceStream::new(Box::new(FileWithConstSize::new(input)), mss_opts);

        let format_opts = Default::default();
        let format = OggReader::try_new(mss, &format_opts)?;

        let track = format.default_track().unwrap();
        let decoder_opts = Default::default();
        let decoder = VorbisDecoder::try_new(&track.codec_params, &decoder_opts)?;

        Ok(Self {
            track_id: track.id,
            decoder: Box::new(decoder),
            format: Box::new(format),
            position: 0,
        })
    }

    pub fn channels(&self) -> Option<Channels> {
        self.decoder.codec_params().channels
    }

    pub fn sample_rate(&self) -> Option<u32> {
        self.decoder.codec_params().sample_rate
    }

    pub fn max_frames_per_packet(&self) -> Option<u64> {
        self.decoder.codec_params().max_frames_per_packet
    }

    pub fn seek(&mut self, time: Duration) -> Result<TimeStamp, Error> {
        let seeked_to = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: time.as_secs_f64().into(),
                track_id: Some(self.track_id),
            },
        )?;
        self.position = seeked_to.actual_ts;
        Ok(self.position)
    }

    pub fn next_packet(&mut self) -> Option<AudioBufferRef> {
        let packet = match self.format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(io)) if io.kind() == io::ErrorKind::UnexpectedEof => {
                return None;
            }
            Err(err) => {
                log::error!("format error: {}", err);
                return None;
            }
        };
        match self.decoder.decode(&packet) {
            Ok(packet) => {
                self.position += packet.frames() as TimeStamp;
                Some(packet)
            }
            // TODO: Handle non-fatal decoding errors and retry.
            Err(err) => {
                log::error!("fatal decode error: {}", err);
                None
            }
        }
    }

    pub fn current_frame(&self) -> TimeStamp {
        self.position
    }
}

impl From<SymphoniaError> for Error {
    fn from(err: SymphoniaError) -> Error {
        Error::AudioDecodingError(Box::new(err))
    }
}
