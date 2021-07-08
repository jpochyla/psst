use crate::{
    connection::codec::{ShannonEncoder, ShannonMessage},
    error::Error,
    protocol::mercury::Header,
    util::{deserialize_protobuf, serialize_protobuf, Sequence},
};
use byteorder::{ReadBytesExt, BE};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    collections::HashMap,
    io,
    io::{Cursor, Read},
    net::TcpStream,
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

    pub fn request(
        &mut self,
        encoder: &mut ShannonEncoder<TcpStream>,
        req: MercuryRequest,
    ) -> io::Result<Receiver<Response>> {
        let (res_sender, res_receiver) = unbounded();
        // Save a new pending ticket.
        let seq = self.sequence.advance();
        self.pending.insert(
            seq,
            Pending {
                messages: Vec::new(),
                callback: Some(res_sender),
            },
        );
        // Send the request message.
        encoder.encode(ShannonMessage::new(
            ShannonMessage::MERCURY_REQ,
            req.encode_to_mercury_message(seq),
        ))?;
        Ok(res_receiver)
    }

    pub fn handle_mercury_req(&mut self, shannon_msg: ShannonMessage) {
        let msg = MercuryMessage::decode(shannon_msg.payload);
        let msg_flags = msg.flags;
        let msg_seq = msg.seq;
        let mut pending = self.pending.remove(&msg_seq).unwrap_or_default();

        pending.messages.push(msg);

        if msg_flags == MercuryMessage::FINAL {
            // This is the final message.  Aggregate all pending parts and process further.
            let parts = MercuryMessage::collect(pending.messages);
            let response = Response::decode_from_parts(parts);
            if let Some(callback) = pending.callback {
                // Send the response.  If the response channel is closed, ignore it.
                let _ = callback.send(response);
            }
        } else {
            // This is not the final message of this sequence, but it back as pending.
            self.pending.insert(msg_seq, pending);
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
        let msg = MercuryMessage::new(seq, MercuryMessage::FINAL, self.encode_to_parts());
        msg.encode()
    }

    fn encode_to_parts(self) -> Vec<Vec<u8>> {
        let header = Header {
            uri: Some(self.uri),
            method: Some(self.method),
            ..Header::default()
        };
        let header_part = serialize_protobuf(&header).expect("Failed to serialize message header");
        let mut payload = self.payload;
        payload.insert(0, header_part);
        payload
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub uri: String,
    pub status_code: i32,
    pub payload: Vec<Vec<u8>>,
}

impl Response {
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

#[derive(Debug, Default)]
struct Pending {
    messages: Vec<MercuryMessage>,
    callback: Option<Sender<Response>>,
}

#[derive(Debug, Default)]
struct MercuryMessage {
    seq: u64,
    flags: u8,
    count: u16,
    parts: Vec<Vec<u8>>,
}

impl MercuryMessage {
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

    fn collect(msgs: impl IntoIterator<Item = Self>) -> Vec<Vec<u8>> {
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
