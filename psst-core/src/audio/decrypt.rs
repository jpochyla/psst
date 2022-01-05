use std::{convert::TryInto, io};

use aes::{
    cipher::{generic_array::GenericArray, NewCipher, StreamCipher, StreamCipherSeek},
    Aes128Ctr,
};

const AUDIO_AESIV: [u8; 16] = [
    0x72, 0xe0, 0x67, 0xfb, 0xdd, 0xcb, 0xcf, 0x77, 0xeb, 0xe8, 0xbc, 0x64, 0x3f, 0x63, 0x0d, 0x93,
];

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct AudioKey(pub [u8; 16]);

impl AudioKey {
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        Some(AudioKey(data.try_into().ok()?))
    }
}

pub struct AudioDecrypt<T> {
    cipher: Aes128Ctr,
    reader: T,
}

impl<T: io::Read> AudioDecrypt<T> {
    pub fn new(key: AudioKey, reader: T) -> AudioDecrypt<T> {
        let cipher = Aes128Ctr::new(
            GenericArray::from_slice(&key.0),
            GenericArray::from_slice(&AUDIO_AESIV),
        );
        AudioDecrypt { cipher, reader }
    }
}

impl<T: io::Read> io::Read for AudioDecrypt<T> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(output)?;

        self.cipher.apply_keystream(&mut output[..len]);

        Ok(len)
    }
}

impl<T: io::Read + io::Seek> io::Seek for AudioDecrypt<T> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let newpos = self.reader.seek(pos)?;

        self.cipher.seek(newpos);

        Ok(newpos)
    }
}
