use crate::{audio_output::AudioSource, error::Error};
use lewton::inside_ogg::OggStreamReader;
use std::{io, vec};

pub struct VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    ogg: OggStreamReader<R>,
    packet: vec::IntoIter<i16>,
}

impl<R> VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    pub fn new(input: R) -> Result<Self, Error> {
        let mut ogg = OggStreamReader::new(input)?;
        let mut buf = Vec::new();

        // Prime the OGG reader so we are ready to return audio data in `self.next()`.
        buf.extend(read_packet(&mut ogg)?.unwrap_or_default());
        buf.extend(read_packet(&mut ogg)?.unwrap_or_default());

        Ok(Self {
            ogg,
            packet: buf.into_iter(),
        })
    }

    pub fn position(&self) -> u64 {
        self.ogg
            .get_last_absgp()
            .expect("Failed to retrieve current OGG stream position")
    }

    pub fn seek(&mut self, pcm_frame: u64) {
        self.ogg
            .seek_absgp_pg(pcm_frame)
            .expect("Failed to set current OGG stream position")
    }
}

impl<R> Iterator for VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if let Some(sample) = self.packet.next() {
            // Sample is available in this packet, return it.
            Some(sample)
        } else {
            // We're at the end of the packet, try to read the next one.
            match read_packet(&mut self.ogg) {
                Ok(Some(packet)) => {
                    self.packet = packet.into_iter();
                    self.packet.next()
                }
                Err(err) => {
                    log::error!("error while decoding: {:?}", err);
                    None // Signal an end of stream.
                }
                Ok(None) => {
                    None // End of stream.
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.packet.size_hint().0, None)
    }
}

impl<R> AudioSource for VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    fn channels(&self) -> u8 {
        self.ogg.ident_hdr.audio_channels
    }

    fn sample_rate(&self) -> u32 {
        self.ogg.ident_hdr.audio_sample_rate
    }
}

impl From<lewton::VorbisError> for Error {
    fn from(err: lewton::VorbisError) -> Error {
        Error::AudioDecodingError(Box::new(err))
    }
}

fn read_packet<R>(ogg: &mut OggStreamReader<R>) -> Result<Option<Vec<i16>>, Error>
where
    R: io::Read + io::Seek,
{
    use lewton::{
        audio::AudioReadError::AudioIsHeader,
        OggReadError::NoCapturePatternFound,
        VorbisError::{BadAudio, OggError},
    };

    loop {
        match ogg.read_dec_packet_itl() {
            Err(BadAudio(AudioIsHeader)) | Err(OggError(NoCapturePatternFound)) => {
                // Skip these and continue to next packet.
            }
            Err(err) => break Err(err.into()),
            Ok(Some(packet)) if packet.is_empty() => {
                // Skip empty packets, i.e. when we seek in the stream.
            }
            Ok(res) => break Ok(res),
        }
    }
}
