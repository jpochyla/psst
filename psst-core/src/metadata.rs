use std::time::Duration;

use quick_protobuf::MessageRead;

use crate::{
    error::Error,
    item_id::{FileId, ItemId, ItemIdType},
    player::file::{MediaFile, MediaPath},
    protocol::metadata::{Restriction, Track},
    session::SessionService,
};

pub trait Fetch: MessageRead<'static> {
    fn uri(id: ItemId) -> String;
    fn fetch(session: &SessionService, id: ItemId) -> Result<Self, Error> {
        session.connected()?.get_mercury_protobuf(Self::uri(id))
    }
}

impl Fetch for Track {
    fn uri(id: ItemId) -> String {
        format!("hm://metadata/3/track/{}", id.to_base16())
    }
}

pub trait ToMediaPath {
    fn is_restricted_in_region(&self, country: &str) -> bool;
    fn find_allowed_alternative(&self, country: &str) -> Option<ItemId>;
    fn to_media_path(&self, preferred_bitrate: usize) -> Option<MediaPath>;
}

impl ToMediaPath for Track {
    fn is_restricted_in_region(&self, country: &str) -> bool {
        self.restriction
            .iter()
            .any(|rest| is_restricted_in_region(rest, country))
    }

    fn find_allowed_alternative(&self, country: &str) -> Option<ItemId> {
        let alt_track = self
            .alternative
            .iter()
            .find(|alt_track| !alt_track.is_restricted_in_region(country))?;
        ItemId::from_raw(alt_track.gid.as_ref()?, ItemIdType::Track)
    }

    fn to_media_path(&self, preferred_bitrate: usize) -> Option<MediaPath> {
        let file = MediaFile::supported_audio_formats_for_bitrate(preferred_bitrate)
            .iter()
            .find_map(|&preferred_format| {
                self.file
                    .iter()
                    .find(|file| file.format == Some(preferred_format))
            })?;
        let file_format = file.format?;
        let item_id = ItemId::from_raw(self.gid.as_ref()?, ItemIdType::Track)?;
        let file_id = FileId::from_raw(file.file_id.as_ref()?)?;
        let duration = Duration::from_millis(self.duration? as u64);
        Some(MediaPath {
            item_id,
            file_id,
            file_format,
            duration,
        })
    }
}

fn is_restricted_in_region(restriction: &Restriction, country: &str) -> bool {
    if let Some(allowed) = &restriction.countries_allowed {
        return !is_country_in_list(allowed.as_bytes(), country.as_bytes());
    }
    if let Some(forbidden) = &restriction.countries_forbidden {
        return is_country_in_list(forbidden.as_bytes(), country.as_bytes());
    }
    false
}

fn is_country_in_list(countries: &[u8], country: &[u8]) -> bool {
    countries.chunks(2).any(|code| code == country)
}
