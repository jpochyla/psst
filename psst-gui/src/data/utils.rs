use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use druid::{im::Vector, Data, Lens};
use serde::{Deserialize, Deserializer};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

#[derive(Clone, Data, Lens)]
pub struct Cached<T: Data> {
    pub data: T,
    #[data(ignore)]
    pub cached_at: Option<NaiveDateTime>,
}

impl<T: Data> Cached<T> {
    pub fn fresh(data: T) -> Self {
        Self {
            data,
            cached_at: None,
        }
    }

    pub fn cached(data: T, at: SystemTime) -> Self {
        let datetime: DateTime<Utc> = at.into();
        Self {
            data,
            cached_at: Some(datetime.naive_utc()),
        }
    }

    pub fn is_cached(&self) -> bool {
        self.cached_at.is_some()
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Data, Deserialize)]
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
}

pub fn default_str() -> Arc<str> {
    "".into()
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
    let millis = u64::deserialize(deserializer)?;
    let duration = Duration::from_millis(millis);
    Ok(duration)
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let date = String::deserialize(deserializer)?;
    let mut parts = date.splitn(3, '-');
    let year = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let month = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let day = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    NaiveDate::from_ymd_opt(year, month, day).ok_or(serde::de::Error::custom("Invalid date"))
}

pub(crate) fn deserialize_date_option<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "deserialize_date")] NaiveDate);

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
