use std::{convert::TryInto, io};

use shannon::Shannon;

#[derive(Debug)]
pub struct ShannonMsg {
    pub cmd: u8,
    pub payload: Vec<u8>,
}

impl ShannonMsg {
    pub const SECRET_BLOCK: u8 = 0x02;
    pub const PING: u8 = 0x04;
    pub const STREAM_CHUNK: u8 = 0x08;
    pub const STREAM_CHUNK_RES: u8 = 0x09;
    pub const CHANNEL_ERROR: u8 = 0x0a;
    pub const CHANNEL_ABORT: u8 = 0x0b;
    pub const REQUEST_KEY: u8 = 0x0c;
    pub const AES_KEY: u8 = 0x0d;
    pub const AES_KEY_ERROR: u8 = 0x0e;
    pub const IMAGE: u8 = 0x19;
    pub const COUNTRY_CODE: u8 = 0x1b;
    pub const PONG: u8 = 0x49;
    pub const PONG_ACK: u8 = 0x4a;
    pub const PAUSE: u8 = 0x4b;
    pub const PRODUCT_INFO: u8 = 0x50;
    pub const LEGACY_WELCOME: u8 = 0x69;
    pub const LICENSE_VERSION: u8 = 0x76;
    pub const LOGIN: u8 = 0xab;
    pub const AP_WELCOME: u8 = 0xac;
    pub const AUTH_FAILURE: u8 = 0xad;
    pub const MERCURY_REQ: u8 = 0xb2;
    pub const MERCURY_SUB: u8 = 0xb3;
    pub const MERCURY_UNSUB: u8 = 0xb4;
    pub const MERCURY_PUB: u8 = 0xb5;

    pub fn new(cmd: u8, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            cmd,
            payload: payload.into(),
        }
    }
}

const MAC_SIZE: usize = 4;
const HEADER_SIZE: usize = 3;

pub struct ShannonEncoder<T> {
    inner: T,
    nonce: u32,
    cipher: Shannon,
}

impl<T> ShannonEncoder<T>
where
    T: io::Write,
{
    pub fn new(inner: T, send_key: &[u8]) -> Self {
        Self {
            inner,
            nonce: 0,
            cipher: Shannon::new(send_key),
        }
    }

    pub fn encode(&mut self, item: ShannonMsg) -> io::Result<()> {
        // Buffer up the whole message.
        let mut buf = Vec::with_capacity(HEADER_SIZE + item.payload.len() + MAC_SIZE);
        let len_u16: u16 = item.payload.len().try_into().unwrap();
        buf.push(item.cmd);
        buf.extend(len_u16.to_be_bytes());
        buf.extend(item.payload);

        // Seed the cipher, rotate the nonce, and encrypt the header and payload.
        self.cipher.nonce_u32(self.nonce);
        self.nonce += 1;
        self.cipher.encrypt(&mut buf);

        // Compute the MAC and append it.
        let mut mac = [0_u8; MAC_SIZE];
        self.cipher.finish(&mut mac);
        buf.extend(mac);

        self.inner.write_all(&buf)
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

pub struct ShannonDecoder<T> {
    inner: T,
    nonce: u32,
    cipher: Shannon,
}

impl<T> ShannonDecoder<T>
where
    T: io::Read,
{
    pub fn new(inner: T, recv_key: &[u8]) -> Self {
        Self {
            inner,
            nonce: 0,
            cipher: Shannon::new(recv_key),
        }
    }

    pub fn decode(&mut self) -> io::Result<ShannonMsg> {
        // Seed the cipher and rotate the nonce.
        self.cipher.nonce_u32(self.nonce);
        self.nonce += 1;

        // Read the whole header.  Reading and decrypting byte by byte is not really
        // reliable, because of a bug in `shannon` crate.
        let mut header = [0_u8; HEADER_SIZE];
        self.inner.read_exact(&mut header)?;
        self.cipher.decrypt(&mut header);

        // Parse the header fields.
        let cmd = header[0];
        let size = u16::from_be_bytes([header[1], header[2]]) as usize;

        // Read and decrypt the payload.
        let mut payload = vec![0_u8; size];
        self.inner.read_exact(&mut payload)?;
        self.cipher.decrypt(&mut payload);

        // Read and check the MAC.
        let mut mac = [0_u8; MAC_SIZE];
        self.inner.read_exact(&mut mac)?;
        self.cipher.check_mac(&mac)?;

        Ok(ShannonMsg::new(cmd, payload))
    }

    pub fn as_inner(&self) -> &T {
        &self.inner
    }
}
