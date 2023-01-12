use std::{
    convert::TryFrom,
    fmt, hash,
    sync::Arc,
    time::{Duration, SystemTime},
};

use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Deserializer, Serialize};
use time::{Date, Month};

#[derive(Clone, Data, Lens)]
pub struct Cached<T: Data> {
    pub data: T,
    #[data(ignore)]
    pub cached_at: Option<SystemTime>,
}

impl<T: Data> Cached<T> {
    pub fn new(data: T, at: SystemTime) -> Self {
        Self {
            data,
            cached_at: Some(at),
        }
    }

    pub fn fresh(data: T) -> Self {
        Self {
            data,
            cached_at: None,
        }
    }

    pub fn map<U: Data>(self, f: impl Fn(T) -> U) -> Cached<U> {
        Cached {
            data: f(self.data),
            cached_at: self.cached_at,
        }
    }
}

#[derive(Deserialize)]
pub struct Page<T: Clone> {
    pub items: Vector<T>,
    pub limit: usize,
    pub offset: usize,
    pub total: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Data, Deserialize, Serialize)]
pub struct Image {
    pub url: Arc<str>,
    pub width: Option<usize>,
    pub height: Option<usize>,
}

impl Image {
    pub fn fits(&self, width: f64, height: f64) -> bool {
        if let (Some(w), Some(h)) = (self.width, self.height) {
            (w as f64) < width && (h as f64) < height
        } else {
            true // Unknown dimensions, treat as fitting.
        }
    }

    pub fn at_least_of_size(images: &Vector<Self>, width: f64, height: f64) -> Option<&Self> {
        images
            .iter()
            .rev()
            .find(|img| !img.fits(width, height))
            .or_else(|| images.back())
    }
}

pub fn default_str() -> Arc<str> {
    "".into()
}

#[derive(Copy, Clone, Default, Debug, Data, Deserialize)]
pub struct Float64(pub f64);

impl PartialEq for Float64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Float64 {}

impl hash::Hash for Float64 {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state)
    }
}

impl fmt::Display for Float64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<f64> for Float64 {
    fn from(f: f64) -> Self {
        Self(f)
    }
}

impl From<Float64> for f64 {
    fn from(f: Float64) -> Self {
        f.0
    }
}

pub fn deserialize_secs<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = f64::deserialize(deserializer)?;
    let duration = Duration::from_secs_f64(secs);
    Ok(duration)
}

pub fn deserialize_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    // Sometimes spotify will provide a negative number for a duration
    let millis = u64::deserialize(deserializer).unwrap_or(0);
    let duration = Duration::from_millis(millis);
    Ok(duration)
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: Deserializer<'de>,
{
    let date = String::deserialize(deserializer)?;
    let mut parts = date.splitn(3, '-');
    let year = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let month: u8 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let month = Month::try_from(month).unwrap_or(Month::January);
    let day = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);

    Date::from_calendar_date(year, month, day)
        .map_err(|_err| serde::de::Error::custom("Invalid date"))
}

pub fn deserialize_date_option<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "deserialize_date")] Date);

    Ok(Option::deserialize(deserializer)?.map(|Wrapper(val)| val))
}

pub fn deserialize_first_page<'de, D, T>(deserializer: D) -> Result<Vector<T>, D::Error>
where
    T: Clone,
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let page = Page::<T>::deserialize(deserializer)?;
    Ok(page.items)
}

pub fn deserialize_null_arc_str<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_else(default_str))
}
