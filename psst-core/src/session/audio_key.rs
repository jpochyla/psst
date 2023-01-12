use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use byteorder::{ReadBytesExt, BE};
use crossbeam_channel::Sender;

use crate::{
    audio::decrypt::AudioKey,
    connection::shannon_codec::ShannonMsg,
    error::Error,
    item_id::{FileId, ItemId},
    util::Sequence,
};

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

    pub fn enqueue_request(
        &mut self,
        track: ItemId,
        file: FileId,
        callback: Sender<Result<AudioKey, Error>>,
    ) -> ShannonMsg {
        let seq = self.sequence.advance();
        self.pending.insert(seq, callback);
        Self::make_key_request(seq, track, file)
    }

    fn make_key_request(seq: u32, track: ItemId, file: FileId) -> ShannonMsg {
        let mut buf = Vec::new();
        buf.extend(file.0);
        buf.extend(track.to_raw());
        buf.extend(seq.to_be_bytes());
        buf.extend(0_u16.to_be_bytes());
        ShannonMsg::new(ShannonMsg::REQUEST_KEY, buf)
    }

    pub fn handle_aes_key(&mut self, msg: ShannonMsg) {
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

    pub fn handle_aes_key_error(&mut self, msg: ShannonMsg) {
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
