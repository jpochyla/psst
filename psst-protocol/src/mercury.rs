// Automatically generated rust module for 'mercury.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use quick_protobuf::{MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MercuryMultiGetRequest {
    pub request: Vec<mercury::MercuryRequest>,
}

impl<'a> MessageRead<'a> for MercuryMultiGetRequest {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.request.push(r.read_message::<mercury::MercuryRequest>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MercuryMultiGetRequest {
    fn get_size(&self) -> usize {
        0
        + self.request.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.request { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MercuryMultiGetReply {
    pub reply: Vec<mercury::MercuryReply>,
}

impl<'a> MessageRead<'a> for MercuryMultiGetReply {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.reply.push(r.read_message::<mercury::MercuryReply>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MercuryMultiGetReply {
    fn get_size(&self) -> usize {
        0
        + self.reply.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.reply { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MercuryRequest {
    pub uri: Option<String>,
    pub content_type: Option<String>,
    pub body: Option<Vec<u8>>,
    pub etag: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for MercuryRequest {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.uri = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.content_type = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.body = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(34) => msg.etag = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MercuryRequest {
    fn get_size(&self) -> usize {
        0
        + self.uri.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.content_type.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.body.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.etag.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.uri { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.content_type { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.body { w.write_with_tag(26, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.etag { w.write_with_tag(34, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MercuryReply {
    pub status_code: Option<i32>,
    pub status_message: Option<String>,
    pub cache_policy: Option<mercury::mod_MercuryReply::CachePolicy>,
    pub ttl: Option<i32>,
    pub etag: Option<Vec<u8>>,
    pub content_type: Option<String>,
    pub body: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for MercuryReply {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.status_code = Some(r.read_sint32(bytes)?),
                Ok(18) => msg.status_message = Some(r.read_string(bytes)?.to_owned()),
                Ok(24) => msg.cache_policy = Some(r.read_enum(bytes)?),
                Ok(32) => msg.ttl = Some(r.read_sint32(bytes)?),
                Ok(42) => msg.etag = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(50) => msg.content_type = Some(r.read_string(bytes)?.to_owned()),
                Ok(58) => msg.body = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MercuryReply {
    fn get_size(&self) -> usize {
        0
        + self.status_code.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.status_message.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.cache_policy.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.ttl.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.etag.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.content_type.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.body.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.status_code { w.write_with_tag(8, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.status_message { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.cache_policy { w.write_with_tag(24, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.ttl { w.write_with_tag(32, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.etag { w.write_with_tag(42, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.content_type { w.write_with_tag(50, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.body { w.write_with_tag(58, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

pub mod mod_MercuryReply {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CachePolicy {
    CACHE_NO = 1,
    CACHE_PRIVATE = 2,
    CACHE_PUBLIC = 3,
}

impl Default for CachePolicy {
    fn default() -> Self {
        CachePolicy::CACHE_NO
    }
}

impl From<i32> for CachePolicy {
    fn from(i: i32) -> Self {
        match i {
            1 => CachePolicy::CACHE_NO,
            2 => CachePolicy::CACHE_PRIVATE,
            3 => CachePolicy::CACHE_PUBLIC,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for CachePolicy {
    fn from(s: &'a str) -> Self {
        match s {
            "CACHE_NO" => CachePolicy::CACHE_NO,
            "CACHE_PRIVATE" => CachePolicy::CACHE_PRIVATE,
            "CACHE_PUBLIC" => CachePolicy::CACHE_PUBLIC,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Header {
    pub uri: Option<String>,
    pub content_type: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<i32>,
    pub user_fields: Vec<mercury::UserField>,
}

impl<'a> MessageRead<'a> for Header {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.uri = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.content_type = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.method = Some(r.read_string(bytes)?.to_owned()),
                Ok(32) => msg.status_code = Some(r.read_sint32(bytes)?),
                Ok(50) => msg.user_fields.push(r.read_message::<mercury::UserField>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Header {
    fn get_size(&self) -> usize {
        0
        + self.uri.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.content_type.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.method.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.status_code.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.user_fields.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.uri { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.content_type { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.method { w.write_with_tag(26, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.status_code { w.write_with_tag(32, |w| w.write_sint32(*s))?; }
        for s in &self.user_fields { w.write_with_tag(50, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct UserField {
    pub key: Option<String>,
    pub value: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for UserField {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.key = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.value = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for UserField {
    fn get_size(&self) -> usize {
        0
        + self.key.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.value.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.key { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.value { w.write_with_tag(18, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

