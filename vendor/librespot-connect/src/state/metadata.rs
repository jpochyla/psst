use crate::{
    protocol::{context::Context, context_track::ContextTrack, player::ProvidedTrack},
    state::context::StateContext,
};
use std::collections::HashMap;
use std::fmt::Display;

const CONTEXT_URI: &str = "context_uri";
const ENTITY_URI: &str = "entity_uri";
const IS_QUEUED: &str = "is_queued";
const IS_AUTOPLAY: &str = "autoplay.is_autoplay";
const HIDDEN: &str = "hidden";
const ITERATION: &str = "iteration";

const CUSTOM_CONTEXT_INDEX: &str = "context_index";
const CUSTOM_SHUFFLE_SEED: &str = "shuffle_seed";
const CUSTOM_INITIAL_TRACK: &str = "initial_track";

macro_rules! metadata_entry {
    ( $get:ident, $set:ident, $clear:ident ($key:ident: $entry:ident)) => {
        metadata_entry!( $get use get, $set, $clear ($key: $entry) -> Option<&String> );
    };
    ( $get_key:ident use $get:ident, $set:ident, $clear:ident ($key:ident: $entry:ident) -> $ty:ty ) => {
        fn $get_key (&self) -> $ty {
            self.$get($entry)
        }


        fn $set (&mut self, $key: impl Display) {
            self.metadata_mut().insert($entry.to_string(), $key.to_string());
        }

        fn $clear(&mut self) {
            self.metadata_mut().remove($entry);
        }
    };
}

/// Allows easy access of known metadata fields
#[allow(dead_code)]
pub(super) trait Metadata {
    fn metadata(&self) -> &HashMap<String, String>;

    fn metadata_mut(&mut self) -> &mut HashMap<String, String>;

    fn get_bool(&self, entry: &str) -> bool {
        matches!(self.metadata().get(entry), Some(entry) if entry.eq("true"))
    }

    fn get_usize(&self, entry: &str) -> Option<usize> {
        self.metadata().get(entry)?.parse().ok()
    }

    fn get(&self, entry: &str) -> Option<&String> {
        self.metadata().get(entry)
    }

    metadata_entry!(is_from_queue use get_bool, set_from_queue, remove_from_queue (is_queued: IS_QUEUED) -> bool);
    metadata_entry!(is_from_autoplay use get_bool, set_from_autoplay, remove_from_autoplay (is_autoplay: IS_AUTOPLAY) -> bool);
    metadata_entry!(is_hidden use get_bool, set_hidden, remove_hidden (is_hidden: HIDDEN) -> bool);

    metadata_entry!(get_context_index use get_usize, set_context_index, remove_context_index (context_index: CUSTOM_CONTEXT_INDEX) -> Option<usize>);
    metadata_entry!(get_context_uri, set_context_uri, remove_context_uri (context_uri: CONTEXT_URI));
    metadata_entry!(get_entity_uri, set_entity_uri, remove_entity_uri (entity_uri: ENTITY_URI));
    metadata_entry!(get_iteration, set_iteration, remove_iteration (iteration: ITERATION));
    metadata_entry!(get_shuffle_seed, set_shuffle_seed, remove_shuffle_seed (shuffle_seed: CUSTOM_SHUFFLE_SEED));
    metadata_entry!(get_initial_track, set_initial_track, remove_initial_track (initial_track: CUSTOM_INITIAL_TRACK));
}

macro_rules! impl_metadata {
    ($impl_for:ident) => {
        impl Metadata for $impl_for {
            fn metadata(&self) -> &HashMap<String, String> {
                &self.metadata
            }

            fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
                &mut self.metadata
            }
        }
    };
}

impl_metadata!(ContextTrack);
impl_metadata!(ProvidedTrack);
impl_metadata!(Context);
impl_metadata!(StateContext);
