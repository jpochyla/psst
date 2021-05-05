use crate::error::Error;
use std::io;
use std::slice;

pub struct VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    vorbis: minivorbis::Decoder<R>,
    // Buffer with enough capacity for `minivorbis` packets.
    packet: Vec<f32>,
    // Offset into `packet`, currently pending sample.
    pos: usize,
}

impl<R> VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    pub fn new(input: R) -> Result<Self, Error> {
        let vorbis = minivorbis::Decoder::new(input)?;

        Ok(Self {
            vorbis,
            packet: Vec::with_capacity(minivorbis::TYPICAL_PACKET_CAP),
            pos: 0, // Buffer is initially empty.
        })
    }

    pub fn seek(&mut self, pcm_frame: u64) {
        self.vorbis
            .seek_to_pcm(pcm_frame)
            .expect("Failed to set current OGG stream position")
    }

    fn read_next_packet(&mut self) -> Result<usize, minivorbis::Error> {
        loop {
            let packet = unsafe {
                slice::from_raw_parts_mut(self.packet.as_mut_ptr(), self.packet.capacity())
            };
            match self.vorbis.read_packet(packet) {
                Err(minivorbis::Error::Hole) => {
                    // Skip holes in decoding.
                    continue;
                }
                Ok(len) => {
                    unsafe { self.packet.set_len(len) };
                    return Ok(len);
                }
                other_result => {
                    return other_result;
                }
            }
        }
    }

    fn channels(&self) -> u8 {
        self.vorbis.channels
    }

    fn sample_rate(&self) -> u32 {
        self.vorbis.sample_rate
    }
}

impl<R> Iterator for VorbisDecoder<R>
where
    R: io::Read + io::Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.packet.len() {
            // We have reached the end of the packet, try to read the next one.
            match self.read_next_packet() {
                Err(err) => {
                    log::error!("error while decoding: {:?}", err);
                    return None; // Signal an end of stream.
                }
                Ok(0) => {
                    return None; // End of stream.
                }
                Ok(_) => {
                    // We have read next packet, reset the cursor and continue.
                    self.pos = 0;
                }
            }
        }
        // Sample is available in this packet, return it.
        let sample = self.packet[self.pos];
        self.pos += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.packet.len() - self.pos, None)
    }
}

impl From<minivorbis::Error> for Error {
    fn from(err: minivorbis::Error) -> Error {
        Error::AudioDecodingError(Box::new(err))
    }
}
