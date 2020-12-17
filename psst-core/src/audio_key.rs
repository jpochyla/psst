use crate::{
    connection::codec::{ShannonEncoder, ShannonMessage},
    error::Error,
    item_id::{FileId, ItemId},
    util::Sequence,
};
use byteorder::{ReadBytesExt, BE};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    collections::HashMap,
    convert::TryInto,
    io,
    io::{Cursor, Read},
    net::TcpStream,
};

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct AudioKey(pub [u8; 16]);

impl AudioKey {
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        Some(AudioKey(data.try_into().ok()?))
    }
}

pub struct AudioKeyDispatcher {
    sequence: Sequence<u32>,
    pending: HashMap<u32, Sender<Result<AudioKey, Error>>>,
}

impl AudioKeyDispatcher {
    pub fn new() -> Self {
        Self {
            sequence: Sequence::new(0),
            pending: HashMap::new(),
        }
    }

    pub fn request(
        &mut self,
        encoder: &mut ShannonEncoder<TcpStream>,
        track: ItemId,
        file: FileId,
    ) -> io::Result<Receiver<Result<AudioKey, Error>>> {
        let (res_sender, res_receiver) = unbounded();
        let seq = self.sequence.advance();
        self.pending.insert(seq, res_sender);
        encoder.encode(self.make_key_request(seq, track, file))?;
        Ok(res_receiver)
    }

    fn make_key_request(&self, seq: u32, track: ItemId, file: FileId) -> ShannonMessage {
        let mut buf = Vec::new();
        buf.extend_from_slice(&file);
        buf.extend_from_slice(&track.to_raw());
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(&0_u16.to_be_bytes());
        ShannonMessage::new(ShannonMessage::REQUEST_KEY, buf)
    }

    pub fn handle_aes_key(&mut self, msg: ShannonMessage) {
        let mut payload = Cursor::new(msg.payload);
        let seq = payload.read_u32::<BE>().unwrap();

        if let Some(tx) = self.pending.remove(&seq) {
            let mut key = [0_u8; 16];
            payload.read_exact(&mut key).unwrap();

            if tx.send(Ok(AudioKey(key))).is_err() {
                log::warn!("missing receiver for audio key, seq: {}", seq);
            }
        } else {
            log::warn!("received unexpected audio key msg, seq: {}", seq);
        }
    }

    pub fn handle_aes_key_error(&mut self, msg: ShannonMessage) {
        let mut payload = Cursor::new(msg.payload);
        let seq = payload.read_u32::<BE>().unwrap();

        if let Some(tx) = self.pending.remove(&seq) {
            log::error!("audio key error");
            if tx.send(Err(Error::UnexpectedResponse)).is_err() {
                log::warn!("missing receiver for audio key error, seq: {}", seq);
            }
        } else {
            log::warn!("received unknown audio key, seq: {}", seq);
        }
    }
}
