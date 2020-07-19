// Automatically generated rust module for 'metadata.proto' file

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
pub struct TopTracks {
    pub country: Option<String>,
    pub track: Vec<metadata::Track>,
}

impl<'a> MessageRead<'a> for TopTracks {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.country = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.track.push(r.read_message::<metadata::Track>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for TopTracks {
    fn get_size(&self) -> usize {
        0
        + self.country.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.track.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.country { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        for s in &self.track { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ActivityPeriod {
    pub start_year: Option<i32>,
    pub end_year: Option<i32>,
    pub decade: Option<i32>,
}

impl<'a> MessageRead<'a> for ActivityPeriod {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.start_year = Some(r.read_sint32(bytes)?),
                Ok(16) => msg.end_year = Some(r.read_sint32(bytes)?),
                Ok(24) => msg.decade = Some(r.read_sint32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ActivityPeriod {
    fn get_size(&self) -> usize {
        0
        + self.start_year.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.end_year.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.decade.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.start_year { w.write_with_tag(8, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.end_year { w.write_with_tag(16, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.decade { w.write_with_tag(24, |w| w.write_sint32(*s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Artist {
    pub gid: Option<Vec<u8>>,
    pub name: Option<String>,
    pub popularity: Option<i32>,
    pub top_track: Vec<metadata::TopTracks>,
    pub album_group: Vec<metadata::AlbumGroup>,
    pub single_group: Vec<metadata::AlbumGroup>,
    pub compilation_group: Vec<metadata::AlbumGroup>,
    pub appears_on_group: Vec<metadata::AlbumGroup>,
    pub genre: Vec<String>,
    pub external_id: Vec<metadata::ExternalId>,
    pub portrait: Vec<metadata::Image>,
    pub biography: Vec<metadata::Biography>,
    pub activity_period: Vec<metadata::ActivityPeriod>,
    pub restriction: Vec<metadata::Restriction>,
    pub related: Vec<metadata::Artist>,
    pub is_portrait_album_cover: Option<bool>,
    pub portrait_group: Option<metadata::ImageGroup>,
}

impl<'a> MessageRead<'a> for Artist {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.gid = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(18) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(24) => msg.popularity = Some(r.read_sint32(bytes)?),
                Ok(34) => msg.top_track.push(r.read_message::<metadata::TopTracks>(bytes)?),
                Ok(42) => msg.album_group.push(r.read_message::<metadata::AlbumGroup>(bytes)?),
                Ok(50) => msg.single_group.push(r.read_message::<metadata::AlbumGroup>(bytes)?),
                Ok(58) => msg.compilation_group.push(r.read_message::<metadata::AlbumGroup>(bytes)?),
                Ok(66) => msg.appears_on_group.push(r.read_message::<metadata::AlbumGroup>(bytes)?),
                Ok(74) => msg.genre.push(r.read_string(bytes)?.to_owned()),
                Ok(82) => msg.external_id.push(r.read_message::<metadata::ExternalId>(bytes)?),
                Ok(90) => msg.portrait.push(r.read_message::<metadata::Image>(bytes)?),
                Ok(98) => msg.biography.push(r.read_message::<metadata::Biography>(bytes)?),
                Ok(106) => msg.activity_period.push(r.read_message::<metadata::ActivityPeriod>(bytes)?),
                Ok(114) => msg.restriction.push(r.read_message::<metadata::Restriction>(bytes)?),
                Ok(122) => msg.related.push(r.read_message::<metadata::Artist>(bytes)?),
                Ok(128) => msg.is_portrait_album_cover = Some(r.read_bool(bytes)?),
                Ok(138) => msg.portrait_group = Some(r.read_message::<metadata::ImageGroup>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Artist {
    fn get_size(&self) -> usize {
        0
        + self.gid.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.popularity.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.top_track.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.album_group.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.single_group.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.compilation_group.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.appears_on_group.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.genre.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.external_id.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.portrait.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.biography.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.activity_period.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.restriction.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.related.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.is_portrait_album_cover.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.portrait_group.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.gid { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.popularity { w.write_with_tag(24, |w| w.write_sint32(*s))?; }
        for s in &self.top_track { w.write_with_tag(34, |w| w.write_message(s))?; }
        for s in &self.album_group { w.write_with_tag(42, |w| w.write_message(s))?; }
        for s in &self.single_group { w.write_with_tag(50, |w| w.write_message(s))?; }
        for s in &self.compilation_group { w.write_with_tag(58, |w| w.write_message(s))?; }
        for s in &self.appears_on_group { w.write_with_tag(66, |w| w.write_message(s))?; }
        for s in &self.genre { w.write_with_tag(74, |w| w.write_string(&**s))?; }
        for s in &self.external_id { w.write_with_tag(82, |w| w.write_message(s))?; }
        for s in &self.portrait { w.write_with_tag(90, |w| w.write_message(s))?; }
        for s in &self.biography { w.write_with_tag(98, |w| w.write_message(s))?; }
        for s in &self.activity_period { w.write_with_tag(106, |w| w.write_message(s))?; }
        for s in &self.restriction { w.write_with_tag(114, |w| w.write_message(s))?; }
        for s in &self.related { w.write_with_tag(122, |w| w.write_message(s))?; }
        if let Some(ref s) = self.is_portrait_album_cover { w.write_with_tag(128, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.portrait_group { w.write_with_tag(138, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct AlbumGroup {
    pub album: Vec<metadata::Album>,
}

impl<'a> MessageRead<'a> for AlbumGroup {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.album.push(r.read_message::<metadata::Album>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for AlbumGroup {
    fn get_size(&self) -> usize {
        0
        + self.album.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.album { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Date {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
    pub hour: Option<i32>,
    pub minute: Option<i32>,
}

impl<'a> MessageRead<'a> for Date {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.year = Some(r.read_sint32(bytes)?),
                Ok(16) => msg.month = Some(r.read_sint32(bytes)?),
                Ok(24) => msg.day = Some(r.read_sint32(bytes)?),
                Ok(32) => msg.hour = Some(r.read_sint32(bytes)?),
                Ok(40) => msg.minute = Some(r.read_sint32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Date {
    fn get_size(&self) -> usize {
        0
        + self.year.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.month.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.day.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.hour.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.minute.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.year { w.write_with_tag(8, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.month { w.write_with_tag(16, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.day { w.write_with_tag(24, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.hour { w.write_with_tag(32, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.minute { w.write_with_tag(40, |w| w.write_sint32(*s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Album {
    pub gid: Option<Vec<u8>>,
    pub name: Option<String>,
    pub artist: Vec<metadata::Artist>,
    pub typ: Option<metadata::mod_Album::Type>,
    pub label: Option<String>,
    pub date: Option<metadata::Date>,
    pub popularity: Option<i32>,
    pub genre: Vec<String>,
    pub cover: Vec<metadata::Image>,
    pub external_id: Vec<metadata::ExternalId>,
    pub disc: Vec<metadata::Disc>,
    pub review: Vec<String>,
    pub copyright: Vec<metadata::Copyright>,
    pub restriction: Vec<metadata::Restriction>,
    pub related: Vec<metadata::Album>,
    pub sale_period: Vec<metadata::SalePeriod>,
    pub cover_group: Option<metadata::ImageGroup>,
}

impl<'a> MessageRead<'a> for Album {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.gid = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(18) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.artist.push(r.read_message::<metadata::Artist>(bytes)?),
                Ok(32) => msg.typ = Some(r.read_enum(bytes)?),
                Ok(42) => msg.label = Some(r.read_string(bytes)?.to_owned()),
                Ok(50) => msg.date = Some(r.read_message::<metadata::Date>(bytes)?),
                Ok(56) => msg.popularity = Some(r.read_sint32(bytes)?),
                Ok(66) => msg.genre.push(r.read_string(bytes)?.to_owned()),
                Ok(74) => msg.cover.push(r.read_message::<metadata::Image>(bytes)?),
                Ok(82) => msg.external_id.push(r.read_message::<metadata::ExternalId>(bytes)?),
                Ok(90) => msg.disc.push(r.read_message::<metadata::Disc>(bytes)?),
                Ok(98) => msg.review.push(r.read_string(bytes)?.to_owned()),
                Ok(106) => msg.copyright.push(r.read_message::<metadata::Copyright>(bytes)?),
                Ok(114) => msg.restriction.push(r.read_message::<metadata::Restriction>(bytes)?),
                Ok(122) => msg.related.push(r.read_message::<metadata::Album>(bytes)?),
                Ok(130) => msg.sale_period.push(r.read_message::<metadata::SalePeriod>(bytes)?),
                Ok(138) => msg.cover_group = Some(r.read_message::<metadata::ImageGroup>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Album {
    fn get_size(&self) -> usize {
        0
        + self.gid.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.artist.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.typ.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.label.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.date.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.popularity.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.genre.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.cover.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.external_id.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.disc.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.review.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.copyright.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.restriction.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.related.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.sale_period.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.cover_group.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.gid { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        for s in &self.artist { w.write_with_tag(26, |w| w.write_message(s))?; }
        if let Some(ref s) = self.typ { w.write_with_tag(32, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.label { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.date { w.write_with_tag(50, |w| w.write_message(s))?; }
        if let Some(ref s) = self.popularity { w.write_with_tag(56, |w| w.write_sint32(*s))?; }
        for s in &self.genre { w.write_with_tag(66, |w| w.write_string(&**s))?; }
        for s in &self.cover { w.write_with_tag(74, |w| w.write_message(s))?; }
        for s in &self.external_id { w.write_with_tag(82, |w| w.write_message(s))?; }
        for s in &self.disc { w.write_with_tag(90, |w| w.write_message(s))?; }
        for s in &self.review { w.write_with_tag(98, |w| w.write_string(&**s))?; }
        for s in &self.copyright { w.write_with_tag(106, |w| w.write_message(s))?; }
        for s in &self.restriction { w.write_with_tag(114, |w| w.write_message(s))?; }
        for s in &self.related { w.write_with_tag(122, |w| w.write_message(s))?; }
        for s in &self.sale_period { w.write_with_tag(130, |w| w.write_message(s))?; }
        if let Some(ref s) = self.cover_group { w.write_with_tag(138, |w| w.write_message(s))?; }
        Ok(())
    }
}

pub mod mod_Album {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Type {
    ALBUM = 1,
    SINGLE = 2,
    COMPILATION = 3,
    EP = 4,
}

impl Default for Type {
    fn default() -> Self {
        Type::ALBUM
    }
}

impl From<i32> for Type {
    fn from(i: i32) -> Self {
        match i {
            1 => Type::ALBUM,
            2 => Type::SINGLE,
            3 => Type::COMPILATION,
            4 => Type::EP,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Type {
    fn from(s: &'a str) -> Self {
        match s {
            "ALBUM" => Type::ALBUM,
            "SINGLE" => Type::SINGLE,
            "COMPILATION" => Type::COMPILATION,
            "EP" => Type::EP,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Track {
    pub gid: Option<Vec<u8>>,
    pub name: Option<String>,
    pub album: Option<metadata::Album>,
    pub artist: Vec<metadata::Artist>,
    pub number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration: Option<i32>,
    pub popularity: Option<i32>,
    pub explicit: Option<bool>,
    pub external_id: Vec<metadata::ExternalId>,
    pub restriction: Vec<metadata::Restriction>,
    pub file: Vec<metadata::AudioFile>,
    pub alternative: Vec<metadata::Track>,
    pub sale_period: Vec<metadata::SalePeriod>,
    pub preview: Vec<metadata::AudioFile>,
}

impl<'a> MessageRead<'a> for Track {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.gid = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(18) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.album = Some(r.read_message::<metadata::Album>(bytes)?),
                Ok(34) => msg.artist.push(r.read_message::<metadata::Artist>(bytes)?),
                Ok(40) => msg.number = Some(r.read_sint32(bytes)?),
                Ok(48) => msg.disc_number = Some(r.read_sint32(bytes)?),
                Ok(56) => msg.duration = Some(r.read_sint32(bytes)?),
                Ok(64) => msg.popularity = Some(r.read_sint32(bytes)?),
                Ok(72) => msg.explicit = Some(r.read_bool(bytes)?),
                Ok(82) => msg.external_id.push(r.read_message::<metadata::ExternalId>(bytes)?),
                Ok(90) => msg.restriction.push(r.read_message::<metadata::Restriction>(bytes)?),
                Ok(98) => msg.file.push(r.read_message::<metadata::AudioFile>(bytes)?),
                Ok(106) => msg.alternative.push(r.read_message::<metadata::Track>(bytes)?),
                Ok(114) => msg.sale_period.push(r.read_message::<metadata::SalePeriod>(bytes)?),
                Ok(122) => msg.preview.push(r.read_message::<metadata::AudioFile>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Track {
    fn get_size(&self) -> usize {
        0
        + self.gid.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.album.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.artist.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.number.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.disc_number.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.duration.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.popularity.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.explicit.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.external_id.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.restriction.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.file.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.alternative.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.sale_period.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.preview.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.gid { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.album { w.write_with_tag(26, |w| w.write_message(s))?; }
        for s in &self.artist { w.write_with_tag(34, |w| w.write_message(s))?; }
        if let Some(ref s) = self.number { w.write_with_tag(40, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.disc_number { w.write_with_tag(48, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.duration { w.write_with_tag(56, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.popularity { w.write_with_tag(64, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.explicit { w.write_with_tag(72, |w| w.write_bool(*s))?; }
        for s in &self.external_id { w.write_with_tag(82, |w| w.write_message(s))?; }
        for s in &self.restriction { w.write_with_tag(90, |w| w.write_message(s))?; }
        for s in &self.file { w.write_with_tag(98, |w| w.write_message(s))?; }
        for s in &self.alternative { w.write_with_tag(106, |w| w.write_message(s))?; }
        for s in &self.sale_period { w.write_with_tag(114, |w| w.write_message(s))?; }
        for s in &self.preview { w.write_with_tag(122, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Image {
    pub file_id: Option<Vec<u8>>,
    pub size: Option<metadata::mod_Image::Size>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

impl<'a> MessageRead<'a> for Image {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.file_id = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(16) => msg.size = Some(r.read_enum(bytes)?),
                Ok(24) => msg.width = Some(r.read_sint32(bytes)?),
                Ok(32) => msg.height = Some(r.read_sint32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Image {
    fn get_size(&self) -> usize {
        0
        + self.file_id.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.size.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.width.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.height.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.file_id { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.size { w.write_with_tag(16, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.width { w.write_with_tag(24, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.height { w.write_with_tag(32, |w| w.write_sint32(*s))?; }
        Ok(())
    }
}

pub mod mod_Image {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Size {
    DEFAULT = 0,
    SMALL = 1,
    LARGE = 2,
    XLARGE = 3,
}

impl Default for Size {
    fn default() -> Self {
        Size::DEFAULT
    }
}

impl From<i32> for Size {
    fn from(i: i32) -> Self {
        match i {
            0 => Size::DEFAULT,
            1 => Size::SMALL,
            2 => Size::LARGE,
            3 => Size::XLARGE,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Size {
    fn from(s: &'a str) -> Self {
        match s {
            "DEFAULT" => Size::DEFAULT,
            "SMALL" => Size::SMALL,
            "LARGE" => Size::LARGE,
            "XLARGE" => Size::XLARGE,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ImageGroup {
    pub image: Vec<metadata::Image>,
}

impl<'a> MessageRead<'a> for ImageGroup {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.image.push(r.read_message::<metadata::Image>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ImageGroup {
    fn get_size(&self) -> usize {
        0
        + self.image.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.image { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Biography {
    pub text: Option<String>,
    pub portrait: Vec<metadata::Image>,
    pub portrait_group: Vec<metadata::ImageGroup>,
}

impl<'a> MessageRead<'a> for Biography {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.text = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.portrait.push(r.read_message::<metadata::Image>(bytes)?),
                Ok(26) => msg.portrait_group.push(r.read_message::<metadata::ImageGroup>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Biography {
    fn get_size(&self) -> usize {
        0
        + self.text.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.portrait.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.portrait_group.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.text { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        for s in &self.portrait { w.write_with_tag(18, |w| w.write_message(s))?; }
        for s in &self.portrait_group { w.write_with_tag(26, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Disc {
    pub number: Option<i32>,
    pub name: Option<String>,
    pub track: Vec<metadata::Track>,
}

impl<'a> MessageRead<'a> for Disc {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.number = Some(r.read_sint32(bytes)?),
                Ok(18) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.track.push(r.read_message::<metadata::Track>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Disc {
    fn get_size(&self) -> usize {
        0
        + self.number.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.track.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.number { w.write_with_tag(8, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        for s in &self.track { w.write_with_tag(26, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Copyright {
    pub typ: Option<metadata::mod_Copyright::Type>,
    pub text: Option<String>,
}

impl<'a> MessageRead<'a> for Copyright {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.typ = Some(r.read_enum(bytes)?),
                Ok(18) => msg.text = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Copyright {
    fn get_size(&self) -> usize {
        0
        + self.typ.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.text.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.typ { w.write_with_tag(8, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.text { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

pub mod mod_Copyright {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Type {
    P = 0,
    C = 1,
}

impl Default for Type {
    fn default() -> Self {
        Type::P
    }
}

impl From<i32> for Type {
    fn from(i: i32) -> Self {
        match i {
            0 => Type::P,
            1 => Type::C,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Type {
    fn from(s: &'a str) -> Self {
        match s {
            "P" => Type::P,
            "C" => Type::C,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Restriction {
    pub catalogue: Vec<metadata::mod_Restriction::Catalogue>,
    pub countries_allowed: Option<String>,
    pub countries_forbidden: Option<String>,
    pub typ: Option<metadata::mod_Restriction::Type>,
    pub catalogue_str: Vec<String>,
}

impl<'a> MessageRead<'a> for Restriction {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.catalogue.push(r.read_enum(bytes)?),
                Ok(18) => msg.countries_allowed = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.countries_forbidden = Some(r.read_string(bytes)?.to_owned()),
                Ok(32) => msg.typ = Some(r.read_enum(bytes)?),
                Ok(42) => msg.catalogue_str.push(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Restriction {
    fn get_size(&self) -> usize {
        0
        + self.catalogue.iter().map(|s| 1 + sizeof_varint(*(s) as u64)).sum::<usize>()
        + self.countries_allowed.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.countries_forbidden.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.typ.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.catalogue_str.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.catalogue { w.write_with_tag(8, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.countries_allowed { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.countries_forbidden { w.write_with_tag(26, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.typ { w.write_with_tag(32, |w| w.write_enum(*s as i32))?; }
        for s in &self.catalogue_str { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

pub mod mod_Restriction {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Catalogue {
    AD = 0,
    SUBSCRIPTION = 1,
    CATALOGUE_ALL = 2,
    SHUFFLE = 3,
    COMMERCIAL = 4,
}

impl Default for Catalogue {
    fn default() -> Self {
        Catalogue::AD
    }
}

impl From<i32> for Catalogue {
    fn from(i: i32) -> Self {
        match i {
            0 => Catalogue::AD,
            1 => Catalogue::SUBSCRIPTION,
            2 => Catalogue::CATALOGUE_ALL,
            3 => Catalogue::SHUFFLE,
            4 => Catalogue::COMMERCIAL,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Catalogue {
    fn from(s: &'a str) -> Self {
        match s {
            "AD" => Catalogue::AD,
            "SUBSCRIPTION" => Catalogue::SUBSCRIPTION,
            "CATALOGUE_ALL" => Catalogue::CATALOGUE_ALL,
            "SHUFFLE" => Catalogue::SHUFFLE,
            "COMMERCIAL" => Catalogue::COMMERCIAL,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Type {
    STREAMING = 0,
}

impl Default for Type {
    fn default() -> Self {
        Type::STREAMING
    }
}

impl From<i32> for Type {
    fn from(i: i32) -> Self {
        match i {
            0 => Type::STREAMING,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Type {
    fn from(s: &'a str) -> Self {
        match s {
            "STREAMING" => Type::STREAMING,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Availability {
    pub catalogue_str: Vec<String>,
    pub start: Option<metadata::Date>,
}

impl<'a> MessageRead<'a> for Availability {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.catalogue_str.push(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.start = Some(r.read_message::<metadata::Date>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Availability {
    fn get_size(&self) -> usize {
        0
        + self.catalogue_str.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.start.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.catalogue_str { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.start { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct SalePeriod {
    pub restriction: Vec<metadata::Restriction>,
    pub start: Option<metadata::Date>,
    pub end: Option<metadata::Date>,
}

impl<'a> MessageRead<'a> for SalePeriod {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.restriction.push(r.read_message::<metadata::Restriction>(bytes)?),
                Ok(18) => msg.start = Some(r.read_message::<metadata::Date>(bytes)?),
                Ok(26) => msg.end = Some(r.read_message::<metadata::Date>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for SalePeriod {
    fn get_size(&self) -> usize {
        0
        + self.restriction.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.start.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.end.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.restriction { w.write_with_tag(10, |w| w.write_message(s))?; }
        if let Some(ref s) = self.start { w.write_with_tag(18, |w| w.write_message(s))?; }
        if let Some(ref s) = self.end { w.write_with_tag(26, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ExternalId {
    pub typ: Option<String>,
    pub id: Option<String>,
}

impl<'a> MessageRead<'a> for ExternalId {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.typ = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.id = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ExternalId {
    fn get_size(&self) -> usize {
        0
        + self.typ.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.id.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.typ { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.id { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct AudioFile {
    pub file_id: Option<Vec<u8>>,
    pub format: Option<metadata::mod_AudioFile::Format>,
}

impl<'a> MessageRead<'a> for AudioFile {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.file_id = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(16) => msg.format = Some(r.read_enum(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for AudioFile {
    fn get_size(&self) -> usize {
        0
        + self.file_id.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.format.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.file_id { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.format { w.write_with_tag(16, |w| w.write_enum(*s as i32))?; }
        Ok(())
    }
}

pub mod mod_AudioFile {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Format {
    OGG_VORBIS_96 = 0,
    OGG_VORBIS_160 = 1,
    OGG_VORBIS_320 = 2,
    MP3_256 = 3,
    MP3_320 = 4,
    MP3_160 = 5,
    MP3_96 = 6,
    MP3_160_ENC = 7,
    MP4_128_DUAL = 8,
    OTHER3 = 9,
    AAC_160 = 10,
    AAC_320 = 11,
    MP4_128 = 12,
    OTHER5 = 13,
}

impl Default for Format {
    fn default() -> Self {
        Format::OGG_VORBIS_96
    }
}

impl From<i32> for Format {
    fn from(i: i32) -> Self {
        match i {
            0 => Format::OGG_VORBIS_96,
            1 => Format::OGG_VORBIS_160,
            2 => Format::OGG_VORBIS_320,
            3 => Format::MP3_256,
            4 => Format::MP3_320,
            5 => Format::MP3_160,
            6 => Format::MP3_96,
            7 => Format::MP3_160_ENC,
            8 => Format::MP4_128_DUAL,
            9 => Format::OTHER3,
            10 => Format::AAC_160,
            11 => Format::AAC_320,
            12 => Format::MP4_128,
            13 => Format::OTHER5,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Format {
    fn from(s: &'a str) -> Self {
        match s {
            "OGG_VORBIS_96" => Format::OGG_VORBIS_96,
            "OGG_VORBIS_160" => Format::OGG_VORBIS_160,
            "OGG_VORBIS_320" => Format::OGG_VORBIS_320,
            "MP3_256" => Format::MP3_256,
            "MP3_320" => Format::MP3_320,
            "MP3_160" => Format::MP3_160,
            "MP3_96" => Format::MP3_96,
            "MP3_160_ENC" => Format::MP3_160_ENC,
            "MP4_128_DUAL" => Format::MP4_128_DUAL,
            "OTHER3" => Format::OTHER3,
            "AAC_160" => Format::AAC_160,
            "AAC_320" => Format::AAC_320,
            "MP4_128" => Format::MP4_128,
            "OTHER5" => Format::OTHER5,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct VideoFile {
    pub file_id: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for VideoFile {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.file_id = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for VideoFile {
    fn get_size(&self) -> usize {
        0
        + self.file_id.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.file_id { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Show {
    pub gid: Option<Vec<u8>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub deprecated_popularity: Option<i32>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub covers: Option<metadata::ImageGroup>,
    pub episode: Vec<metadata::Episode>,
    pub copyright: Vec<metadata::Copyright>,
    pub restriction: Vec<metadata::Restriction>,
    pub keyword: Vec<String>,
    pub media_type: Option<metadata::mod_Show::MediaType>,
    pub consumption_order: Option<metadata::mod_Show::ConsumptionOrder>,
    pub interpret_restriction_using_geoip: Option<bool>,
    pub availability: Vec<metadata::Availability>,
    pub country_of_origin: Option<String>,
    pub categories: Vec<metadata::Category>,
    pub passthrough: Option<metadata::mod_Show::PassthroughEnum>,
}

impl<'a> MessageRead<'a> for Show {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.gid = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(18) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(514) => msg.description = Some(r.read_string(bytes)?.to_owned()),
                Ok(520) => msg.deprecated_popularity = Some(r.read_sint32(bytes)?),
                Ok(530) => msg.publisher = Some(r.read_string(bytes)?.to_owned()),
                Ok(538) => msg.language = Some(r.read_string(bytes)?.to_owned()),
                Ok(544) => msg.explicit = Some(r.read_bool(bytes)?),
                Ok(554) => msg.covers = Some(r.read_message::<metadata::ImageGroup>(bytes)?),
                Ok(562) => msg.episode.push(r.read_message::<metadata::Episode>(bytes)?),
                Ok(570) => msg.copyright.push(r.read_message::<metadata::Copyright>(bytes)?),
                Ok(578) => msg.restriction.push(r.read_message::<metadata::Restriction>(bytes)?),
                Ok(586) => msg.keyword.push(r.read_string(bytes)?.to_owned()),
                Ok(592) => msg.media_type = Some(r.read_enum(bytes)?),
                Ok(600) => msg.consumption_order = Some(r.read_enum(bytes)?),
                Ok(608) => msg.interpret_restriction_using_geoip = Some(r.read_bool(bytes)?),
                Ok(626) => msg.availability.push(r.read_message::<metadata::Availability>(bytes)?),
                Ok(634) => msg.country_of_origin = Some(r.read_string(bytes)?.to_owned()),
                Ok(642) => msg.categories.push(r.read_message::<metadata::Category>(bytes)?),
                Ok(648) => msg.passthrough = Some(r.read_enum(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Show {
    fn get_size(&self) -> usize {
        0
        + self.gid.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.description.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.deprecated_popularity.as_ref().map_or(0, |m| 2 + sizeof_sint32(*(m)))
        + self.publisher.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.language.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.explicit.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.covers.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.episode.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.copyright.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.restriction.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.keyword.iter().map(|s| 2 + sizeof_len((s).len())).sum::<usize>()
        + self.media_type.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.consumption_order.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.interpret_restriction_using_geoip.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.availability.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.country_of_origin.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.categories.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.passthrough.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.gid { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.description { w.write_with_tag(514, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.deprecated_popularity { w.write_with_tag(520, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.publisher { w.write_with_tag(530, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.language { w.write_with_tag(538, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.explicit { w.write_with_tag(544, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.covers { w.write_with_tag(554, |w| w.write_message(s))?; }
        for s in &self.episode { w.write_with_tag(562, |w| w.write_message(s))?; }
        for s in &self.copyright { w.write_with_tag(570, |w| w.write_message(s))?; }
        for s in &self.restriction { w.write_with_tag(578, |w| w.write_message(s))?; }
        for s in &self.keyword { w.write_with_tag(586, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.media_type { w.write_with_tag(592, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.consumption_order { w.write_with_tag(600, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.interpret_restriction_using_geoip { w.write_with_tag(608, |w| w.write_bool(*s))?; }
        for s in &self.availability { w.write_with_tag(626, |w| w.write_message(s))?; }
        if let Some(ref s) = self.country_of_origin { w.write_with_tag(634, |w| w.write_string(&**s))?; }
        for s in &self.categories { w.write_with_tag(642, |w| w.write_message(s))?; }
        if let Some(ref s) = self.passthrough { w.write_with_tag(648, |w| w.write_enum(*s as i32))?; }
        Ok(())
    }
}

pub mod mod_Show {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MediaType {
    MIXED = 0,
    AUDIO = 1,
    VIDEO = 2,
}

impl Default for MediaType {
    fn default() -> Self {
        MediaType::MIXED
    }
}

impl From<i32> for MediaType {
    fn from(i: i32) -> Self {
        match i {
            0 => MediaType::MIXED,
            1 => MediaType::AUDIO,
            2 => MediaType::VIDEO,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for MediaType {
    fn from(s: &'a str) -> Self {
        match s {
            "MIXED" => MediaType::MIXED,
            "AUDIO" => MediaType::AUDIO,
            "VIDEO" => MediaType::VIDEO,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConsumptionOrder {
    SEQUENTIAL = 1,
    EPISODIC = 2,
    RECENT = 3,
}

impl Default for ConsumptionOrder {
    fn default() -> Self {
        ConsumptionOrder::SEQUENTIAL
    }
}

impl From<i32> for ConsumptionOrder {
    fn from(i: i32) -> Self {
        match i {
            1 => ConsumptionOrder::SEQUENTIAL,
            2 => ConsumptionOrder::EPISODIC,
            3 => ConsumptionOrder::RECENT,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ConsumptionOrder {
    fn from(s: &'a str) -> Self {
        match s {
            "SEQUENTIAL" => ConsumptionOrder::SEQUENTIAL,
            "EPISODIC" => ConsumptionOrder::EPISODIC,
            "RECENT" => ConsumptionOrder::RECENT,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PassthroughEnum {
    UNKNOWN = 0,
    NONE = 1,
    ALLOWED = 2,
}

impl Default for PassthroughEnum {
    fn default() -> Self {
        PassthroughEnum::UNKNOWN
    }
}

impl From<i32> for PassthroughEnum {
    fn from(i: i32) -> Self {
        match i {
            0 => PassthroughEnum::UNKNOWN,
            1 => PassthroughEnum::NONE,
            2 => PassthroughEnum::ALLOWED,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for PassthroughEnum {
    fn from(s: &'a str) -> Self {
        match s {
            "UNKNOWN" => PassthroughEnum::UNKNOWN,
            "NONE" => PassthroughEnum::NONE,
            "ALLOWED" => PassthroughEnum::ALLOWED,
            _ => Self::default(),
        }
    }
}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Episode {
    pub gid: Option<Vec<u8>>,
    pub name: Option<String>,
    pub duration: Option<i32>,
    pub popularity: Option<i32>,
    pub file: Vec<metadata::AudioFile>,
    pub description: Option<String>,
    pub number: Option<i32>,
    pub publish_time: Option<metadata::Date>,
    pub deprecated_popularity: Option<i32>,
    pub covers: Option<metadata::ImageGroup>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub show: Option<metadata::Show>,
    pub video: Vec<metadata::VideoFile>,
    pub video_preview: Vec<metadata::VideoFile>,
    pub audio_preview: Vec<metadata::AudioFile>,
    pub restriction: Vec<metadata::Restriction>,
    pub freeze_frame: Option<metadata::ImageGroup>,
    pub keyword: Vec<String>,
    pub suppress_monetization: Option<bool>,
    pub interpret_restriction_using_geoip: Option<bool>,
    pub allow_background_playback: Option<bool>,
    pub availability: Vec<metadata::Availability>,
    pub external_url: Option<String>,
    pub original_audio: Option<metadata::OriginalAudio>,
}

impl<'a> MessageRead<'a> for Episode {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.gid = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(18) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(56) => msg.duration = Some(r.read_sint32(bytes)?),
                Ok(64) => msg.popularity = Some(r.read_sint32(bytes)?),
                Ok(98) => msg.file.push(r.read_message::<metadata::AudioFile>(bytes)?),
                Ok(514) => msg.description = Some(r.read_string(bytes)?.to_owned()),
                Ok(520) => msg.number = Some(r.read_sint32(bytes)?),
                Ok(530) => msg.publish_time = Some(r.read_message::<metadata::Date>(bytes)?),
                Ok(536) => msg.deprecated_popularity = Some(r.read_sint32(bytes)?),
                Ok(546) => msg.covers = Some(r.read_message::<metadata::ImageGroup>(bytes)?),
                Ok(554) => msg.language = Some(r.read_string(bytes)?.to_owned()),
                Ok(560) => msg.explicit = Some(r.read_bool(bytes)?),
                Ok(570) => msg.show = Some(r.read_message::<metadata::Show>(bytes)?),
                Ok(578) => msg.video.push(r.read_message::<metadata::VideoFile>(bytes)?),
                Ok(586) => msg.video_preview.push(r.read_message::<metadata::VideoFile>(bytes)?),
                Ok(594) => msg.audio_preview.push(r.read_message::<metadata::AudioFile>(bytes)?),
                Ok(602) => msg.restriction.push(r.read_message::<metadata::Restriction>(bytes)?),
                Ok(610) => msg.freeze_frame = Some(r.read_message::<metadata::ImageGroup>(bytes)?),
                Ok(618) => msg.keyword.push(r.read_string(bytes)?.to_owned()),
                Ok(624) => msg.suppress_monetization = Some(r.read_bool(bytes)?),
                Ok(632) => msg.interpret_restriction_using_geoip = Some(r.read_bool(bytes)?),
                Ok(648) => msg.allow_background_playback = Some(r.read_bool(bytes)?),
                Ok(658) => msg.availability.push(r.read_message::<metadata::Availability>(bytes)?),
                Ok(666) => msg.external_url = Some(r.read_string(bytes)?.to_owned()),
                Ok(674) => msg.original_audio = Some(r.read_message::<metadata::OriginalAudio>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Episode {
    fn get_size(&self) -> usize {
        0
        + self.gid.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.duration.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.popularity.as_ref().map_or(0, |m| 1 + sizeof_sint32(*(m)))
        + self.file.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.description.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.number.as_ref().map_or(0, |m| 2 + sizeof_sint32(*(m)))
        + self.publish_time.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.deprecated_popularity.as_ref().map_or(0, |m| 2 + sizeof_sint32(*(m)))
        + self.covers.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.language.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.explicit.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.show.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.video.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.video_preview.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.audio_preview.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.restriction.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.freeze_frame.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.keyword.iter().map(|s| 2 + sizeof_len((s).len())).sum::<usize>()
        + self.suppress_monetization.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.interpret_restriction_using_geoip.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.allow_background_playback.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.availability.iter().map(|s| 2 + sizeof_len((s).get_size())).sum::<usize>()
        + self.external_url.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.original_audio.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.gid { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.duration { w.write_with_tag(56, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.popularity { w.write_with_tag(64, |w| w.write_sint32(*s))?; }
        for s in &self.file { w.write_with_tag(98, |w| w.write_message(s))?; }
        if let Some(ref s) = self.description { w.write_with_tag(514, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.number { w.write_with_tag(520, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.publish_time { w.write_with_tag(530, |w| w.write_message(s))?; }
        if let Some(ref s) = self.deprecated_popularity { w.write_with_tag(536, |w| w.write_sint32(*s))?; }
        if let Some(ref s) = self.covers { w.write_with_tag(546, |w| w.write_message(s))?; }
        if let Some(ref s) = self.language { w.write_with_tag(554, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.explicit { w.write_with_tag(560, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.show { w.write_with_tag(570, |w| w.write_message(s))?; }
        for s in &self.video { w.write_with_tag(578, |w| w.write_message(s))?; }
        for s in &self.video_preview { w.write_with_tag(586, |w| w.write_message(s))?; }
        for s in &self.audio_preview { w.write_with_tag(594, |w| w.write_message(s))?; }
        for s in &self.restriction { w.write_with_tag(602, |w| w.write_message(s))?; }
        if let Some(ref s) = self.freeze_frame { w.write_with_tag(610, |w| w.write_message(s))?; }
        for s in &self.keyword { w.write_with_tag(618, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.suppress_monetization { w.write_with_tag(624, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.interpret_restriction_using_geoip { w.write_with_tag(632, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.allow_background_playback { w.write_with_tag(648, |w| w.write_bool(*s))?; }
        for s in &self.availability { w.write_with_tag(658, |w| w.write_message(s))?; }
        if let Some(ref s) = self.external_url { w.write_with_tag(666, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.original_audio { w.write_with_tag(674, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Category {
    pub name: Option<String>,
    pub subcategories: Vec<metadata::Category>,
}

impl<'a> MessageRead<'a> for Category {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.subcategories.push(r.read_message::<metadata::Category>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Category {
    fn get_size(&self) -> usize {
        0
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.subcategories.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.name { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        for s in &self.subcategories { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct OriginalAudio {
    pub uuid: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for OriginalAudio {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.uuid = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for OriginalAudio {
    fn get_size(&self) -> usize {
        0
        + self.uuid.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.uuid { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

