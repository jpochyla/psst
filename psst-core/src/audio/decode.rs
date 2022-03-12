use std::{io, time::Duration};

use symphonia::{
    core::{
        audio::{SampleBuffer, SignalSpec},
        codecs::{CodecParameters, Decoder, DecoderOptions},
        conv::ConvertibleSample,
        errors::Error as SymphoniaError,
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions},
        units::TimeStamp,
    },
    default::{
        codecs::{Mp3Decoder, VorbisDecoder},
        formats::{Mp3Reader, OggReader},
    },
};

use crate::{error::Error, util::FileWithConstSize};

pub enum AudioCodecFormat {
    Mp3,
    OggVorbis,
}

impl AudioCodecFormat {
    fn format_reader(
        &self,
        mss: MediaSourceStream,
    ) -> Result<Box<dyn FormatReader>, SymphoniaError> {
        match self {
            Self::Mp3 => Ok(Box::new(Mp3Reader::try_new(
                mss,
                &FormatOptions::default(),
            )?)),
            Self::OggVorbis => Ok(Box::new(OggReader::try_new(
                mss,
                &FormatOptions::default(),
            )?)),
        }
    }

    fn decoder(&self, codec_params: &CodecParameters) -> Result<Box<dyn Decoder>, SymphoniaError> {
        match self {
            Self::Mp3 => Ok(Box::new(Mp3Decoder::try_new(
                codec_params,
                &DecoderOptions::default(),
            )?)),
            Self::OggVorbis => Ok(Box::new(VorbisDecoder::try_new(
                codec_params,
                &DecoderOptions::default(),
            )?)),
        }
    }
}

pub struct AudioDecoder {
    track_id: u32, // Internal track index.
    decoder: Box<dyn Decoder>,
    format: Box<dyn FormatReader>,
}

impl AudioDecoder {
    pub fn new<T>(input: T, codec: AudioCodecFormat) -> Result<Self, Error>
    where
        T: io::Read + io::Seek + Send + Sync + 'static,
    {
        let mss = MediaSourceStream::new(
            Box::new(FileWithConstSize::new(input)),
            MediaSourceStreamOptions::default(),
        );
        let format = codec.format_reader(mss)?;
        let track = format.default_track().unwrap();
        let decoder = codec.decoder(&track.codec_params)?;

        Ok(Self {
            track_id: track.id,
            decoder,
            format,
        })
    }

    pub fn codec_params(&self) -> &CodecParameters {
        self.decoder.codec_params()
    }

    pub fn signal_spec(&self) -> SignalSpec {
        SignalSpec {
            rate: self.codec_params().sample_rate.unwrap(),
            channels: self.codec_params().channels.unwrap(),
        }
    }

    pub fn seek(&mut self, time: Duration) -> Result<TimeStamp, Error> {
        let seeked_to = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: time.as_secs_f64().into(),
                track_id: Some(self.track_id),
            },
        )?;
        Ok(seeked_to.actual_ts)
    }

    /// Read a next packet of audio from this decoder.  Returns `None` in case
    /// of EOF or internal error.
    pub fn read_packet<S>(&mut self, samples: &mut SampleBuffer<S>) -> Option<TimeStamp>
    where
        S: ConvertibleSample,
    {
        loop {
            // Demux an encoded packet from the media format.
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(io)) if io.kind() == io::ErrorKind::UnexpectedEof => {
                    return None; // End of this stream.
                }
                Err(err) => {
                    log::error!("format error: {}", err);
                    return None; // We cannot recover from format errors, quit.
                }
            };
            while !self.format.metadata().is_latest() {
                // Consume any new metadata that has been read since the last
                // packet.
            }
            // If the packet does not belong to the selected track, skip over it.
            if packet.track_id() != self.track_id {
                continue;
            }
            // Decode the packet into an audio buffer.
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    // Interleave the samples into the buffer.
                    samples.copy_interleaved_ref(decoded);
                    return Some(packet.ts());
                }
                Err(SymphoniaError::IoError(err)) => {
                    // The packet failed to decode due to an IO error, skip the packet.
                    log::error!("io decode error: {}", err);
                    continue;
                }
                Err(SymphoniaError::DecodeError(err)) => {
                    // The packet failed to decode due to invalid data, skip the packet.
                    log::error!("decode error: {}", err);
                    continue;
                }
                Err(err) => {
                    log::error!("fatal decode error: {}", err);
                    return None;
                }
            };
        }
    }
}

impl<T> MediaSource for FileWithConstSize<T>
where
    T: io::Read + io::Seek + Send + Sync,
{
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len())
    }
}

impl From<SymphoniaError> for Error {
    fn from(err: SymphoniaError) -> Error {
        Error::AudioDecodingError(Box::new(err))
    }
}
