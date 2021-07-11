use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use byteorder::{ReadBytesExt, BE};
use crossbeam_channel::Sender;

use crate::{
    connection::shannon_codec::ShannonMsg,
    error::Error,
    protocol::mercury::Header,
    util::{deserialize_protobuf, serialize_protobuf, Sequence},
};

pub struct MercuryDispatcher {
    sequence: Sequence<u64>,
    pending: HashMap<u64, Pending>,
}

impl MercuryDispatcher {
    pub fn new() -> Self {
        Self {
            sequence: Sequence::new(0),
            pending: HashMap::new(),
        }
    }

    pub fn enqueue_request(
        &mut self,
        req: MercuryRequest,
        callback: Sender<MercuryResponse>,
    ) -> ShannonMsg {
        let seq = self.sequence.advance();
        self.pending.insert(
            seq,
            Pending {
                callback,
                messages: Vec::new(),
            },
        );
        ShannonMsg::new(ShannonMsg::MERCURY_REQ, req.encode_to_mercury_message(seq))
    }

    pub fn handle_mercury_req(&mut self, shannon_msg: ShannonMsg) {
        let msg = Msg::decode(shannon_msg.payload);
        let msg_flags = msg.flags;
        let msg_seq = msg.seq;
        if let Some(mut pending) = self.pending.remove(&msg_seq) {
            pending.messages.push(msg);
            if msg_flags == Msg::FINAL {
                // This is the final message.  Aggregate all pending parts and process further.
                let parts = Msg::aggregate(pending.messages);
                let response = MercuryResponse::decode_from_parts(parts);
                // Send the response.  If the response channel is closed, ignore it.
                let _ = pending.callback.send(response);
            } else {
                // This is not the final message of this sequence, but it back as pending.
                self.pending.insert(msg_seq, pending);
            }
        } else {
            log::warn!("received unexpected mercury msg, seq: {}", msg_seq);
        }
    }
}

#[derive(Debug)]
pub struct MercuryRequest {
    pub uri: String,
    pub method: String,
    pub payload: Vec<Vec<u8>>,
}

impl MercuryRequest {
    pub fn get(uri: String) -> Self {
        Self {
            uri,
            method: "GET".to_string(),
            payload: Vec::new(),
        }
    }

    pub fn send(uri: String, data: Vec<u8>) -> Self {
        Self {
            uri,
            method: "SEND".to_string(),
            payload: vec![data],
        }
    }

    fn encode_to_mercury_message(self, seq: u64) -> Vec<u8> {
        let parts = self.encode_to_parts();
        let msg = Msg::new(seq, Msg::FINAL, parts);
        msg.encode()
    }

    fn encode_to_parts(self) -> Vec<Vec<u8>> {
        let header = Header {
            uri: Some(self.uri),
            method: Some(self.method),
            ..Header::default()
        };
        let header_part = serialize_protobuf(&header).expect("Failed to serialize message header");
        let mut parts = self.payload;
        parts.insert(0, header_part);
        parts
    }
}

#[derive(Debug, Clone)]
pub struct MercuryResponse {
    pub uri: String,
    pub status_code: i32,
    pub payload: Vec<Vec<u8>>,
}

impl MercuryResponse {
    fn decode_from_parts(mut parts: Vec<Vec<u8>>) -> Self {
        let header_part = parts.remove(0);
        let header: Header =
            deserialize_protobuf(&header_part).expect("Failed to deserialize message header");
        Self {
            uri: header.uri.unwrap(),
            status_code: header.status_code.unwrap(),
            payload: parts,
        }
    }
}

#[derive(Debug)]
struct Pending {
    messages: Vec<Msg>,
    callback: Sender<MercuryResponse>,
}

#[derive(Debug, Default)]
struct Msg {
    seq: u64,
    flags: u8,
    count: u16,
    parts: Vec<Vec<u8>>,
}

impl Msg {
    const FINAL: u8 = 0x01;
    const PARTIAL: u8 = 0x02;

    fn new(seq: u64, flags: u8, parts: Vec<Vec<u8>>) -> Self {
        let count = parts.len() as u16;
        Self {
            seq,
            flags,
            count,
            parts,
        }
    }

    fn decode(buf: Vec<u8>) -> Self {
        let mut buf = Cursor::new(buf);
        let seq_len = buf.read_u16::<BE>().unwrap();
        let seq = buf.read_uint::<BE>(seq_len.into()).unwrap();
        let flags = buf.read_u8().unwrap();
        let count = buf.read_u16::<BE>().unwrap();
        let mut parts = Vec::with_capacity(count.into());
        for _ in 0..count {
            let part_len = buf.read_u16::<BE>().unwrap();
            let mut part = vec![0_u8; part_len.into()];
            buf.read_exact(&mut part).unwrap();
            parts.push(part);
        }
        Self {
            seq,
            flags,
            count,
            parts,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend(8_u16.to_be_bytes()); // Sequence length.
        buf.extend(self.seq.to_be_bytes());
        buf.push(self.flags);
        buf.extend(self.count.to_be_bytes());
        for part in &self.parts {
            let len = part.len() as u16;
            buf.extend(len.to_be_bytes());
            buf.extend(part);
        }
        buf
    }

    fn aggregate(msgs: impl IntoIterator<Item = Self>) -> Vec<Vec<u8>> {
        let mut results = Vec::new();
        let mut partial: Option<Vec<u8>> = None;

        for msg in msgs {
            for (i, mut part) in msg.parts.into_iter().enumerate() {
                // If we have a partial data left from the last message, append to it.
                if let Some(mut partial) = partial.take() {
                    partial.extend(part);
                    part = partial;
                }

                // Save the last part of partial messages for later.
                let is_last_part = i as u16 == msg.count - 1;
                if msg.flags == Self::PARTIAL && is_last_part {
                    partial = Some(part);
                } else {
                    results.push(part);
                }
            }
        }

        results
    }
}

impl From<quick_protobuf::Error> for Error {
    fn from(err: quick_protobuf::Error) -> Self {
        Error::IoError(err.into())
    }
}
