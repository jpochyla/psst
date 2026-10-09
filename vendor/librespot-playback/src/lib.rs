#[macro_use]
extern crate log;

use librespot_audio as audio;
use librespot_core as core;
use librespot_metadata as metadata;

pub mod audio_backend;
pub mod config;
pub mod convert;
pub mod decoder;
pub mod dither;
mod local_file;
pub mod mixer;
pub mod player;
mod symphonia_util;

pub const SAMPLE_RATE: u32 = 44100;
pub const NUM_CHANNELS: u8 = 2;
pub const SAMPLES_PER_SECOND: u32 = SAMPLE_RATE * NUM_CHANNELS as u32;
pub const PAGES_PER_MS: f64 = SAMPLE_RATE as f64 / 1000.0;
pub const MS_PER_PAGE: f64 = 1000.0 / SAMPLE_RATE as f64;
