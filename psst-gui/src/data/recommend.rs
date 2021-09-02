use std::sync::Arc;

use druid::{
    im::{vector, Vector},
    Data, Lens,
};
use serde::{Deserialize, Serialize};

use super::{ArtistLink, Float64, Promise, Track, TrackId};

#[derive(Clone, Data, Lens)]
pub struct Recommend {
    pub knobs: Arc<RecommendationsKnobs>,
    pub results: Promise<Recommendations, Arc<RecommendationsRequest>>,
}

#[derive(Clone, Debug, Default, Data, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct RecommendationsRequest {
    pub seed_artists: Vector<ArtistLink>,
    pub seed_tracks: Vector<TrackId>,
    #[serde(skip)]
    pub params: RecommendationsParams,
}

impl RecommendationsRequest {
    pub fn for_track(id: TrackId) -> Self {
        Self {
            seed_tracks: vector![id],
            ..Self::default()
        }
    }

    pub fn with_params(mut self, params: RecommendationsParams) -> Self {
        self.params = params;
        self
    }
}

#[derive(Clone, Debug, Default, Data, Lens)]
pub struct RecommendationsKnobs {
    pub duration_ms: Toggled<u64>,
    pub popularity: Toggled<u64>,
    pub key: Toggled<u64>,
    pub mode: Toggled<u64>,
    pub tempo: Toggled<u64>,
    pub time_signature: Toggled<u64>,

    pub acousticness: Toggled<f64>,
    pub danceability: Toggled<f64>,
    pub energy: Toggled<f64>,
    pub instrumentalness: Toggled<f64>,
    pub liveness: Toggled<f64>,
    pub loudness: Toggled<f64>,
    pub speechiness: Toggled<f64>,
    pub valence: Toggled<f64>,
}

impl RecommendationsKnobs {
    pub fn as_params(&self) -> RecommendationsParams {
        RecommendationsParams {
            duration_ms: Range::new(None, None, self.duration_ms.into()),
            popularity: Range::new(None, None, self.popularity.into()),
            key: Range::new(None, None, self.key.into()),
            mode: Range::new(None, None, self.mode.into()),
            tempo: Range::new(None, None, self.tempo.into()),
            time_signature: Range::new(None, None, self.time_signature.into()),
            acousticness: Range::new(None, None, self.acousticness.into()),
            danceability: Range::new(None, None, self.danceability.into()),
            energy: Range::new(None, None, self.energy.into()),
            instrumentalness: Range::new(None, None, self.instrumentalness.into()),
            liveness: Range::new(None, None, self.liveness.into()),
            loudness: Range::new(None, None, self.loudness.into()),
            speechiness: Range::new(None, None, self.speechiness.into()),
            valence: Range::new(None, None, self.valence.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Data, Lens)]
pub struct Toggled<T> {
    pub enabled: bool,
    pub value: T,
}

impl From<Toggled<u64>> for Option<u64> {
    fn from(t: Toggled<u64>) -> Self {
        if t.enabled {
            Some(t.value)
        } else {
            None
        }
    }
}

impl From<Toggled<f64>> for Option<Float64> {
    fn from(t: Toggled<f64>) -> Self {
        if t.enabled {
            Some(t.value.into())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Data, Lens)]
pub struct RecommendationsParams {
    pub duration_ms: Range<u64>,
    pub popularity: Range<u64>,
    pub key: Range<u64>,
    pub mode: Range<u64>,
    pub tempo: Range<u64>,
    pub time_signature: Range<u64>,

    pub acousticness: Range<Float64>,
    pub danceability: Range<Float64>,
    pub energy: Range<Float64>,
    pub instrumentalness: Range<Float64>,
    pub liveness: Range<Float64>,
    pub loudness: Range<Float64>,
    pub speechiness: Range<Float64>,
    pub valence: Range<Float64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Data, Lens)]
pub struct Range<T> {
    pub min: Option<T>,
    pub max: Option<T>,
    pub target: Option<T>,
}

impl<T> Range<T> {
    pub fn new(min: Option<T>, max: Option<T>, target: Option<T>) -> Self {
        Self { min, max, target }
    }
}

#[derive(Clone, Data, Deserialize, Lens)]
pub struct Recommendations {
    #[serde(skip)]
    pub request: Arc<RecommendationsRequest>,
    pub seeds: Vector<RecommendationsSeed>,
    pub tracks: Vector<Arc<Track>>,
}

#[derive(Clone, Data, Deserialize, Lens)]
pub struct RecommendationsSeed {
    #[serde(default)]
    pub after_filtering_size: usize,
    #[serde(default)]
    pub after_relinking_size: usize,
    pub href: Option<Arc<str>>,
    pub id: Arc<str>,
    #[serde(default)]
    pub initial_pool_size: usize,
    #[serde(rename = "type")]
    pub _type: RecommendationsSeedType,
}

#[derive(Clone, Data, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecommendationsSeedType {
    Artist,
    Track,
    Genre,
}
