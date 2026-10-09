//! Public lyrics lookup uses a separate HTTP client with no Spotify credentials.
use std::{num::NonZeroUsize, sync::OnceLock, time::Duration};

use druid::im::Vector;
use lru::LruCache;
use parking_lot::Mutex;
use serde::Deserialize;

use super::WebApi;
use crate::{
    data::{Config, Lyrics, Track, TrackLines},
    error::Error,
};

type LyricsCache = Mutex<LruCache<String, Lyrics>>;
static CACHE: OnceLock<LyricsCache> = OnceLock::new();

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(default)]
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

pub fn fetch(track: &Track) -> Result<Lyrics, Error> {
    let key = format!(
        "{}|{}|{}|{}",
        track.name,
        track.artist_name(),
        track.album_name(),
        track.duration.as_millis()
    );
    let cache = CACHE.get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));
    if let Some(lyrics) = cache.lock().get(&key).cloned() {
        return Ok(lyrics);
    }
    let result = fetch_public(track).or_else(|public_error| {
        let mut lines = WebApi::global()
            .get_lyrics(track.id.0.to_base62())
            .map_err(|_| public_error)?;
        finish_timestamps(&mut lines, track.duration.as_millis() as u64);
        Ok::<Lyrics, Error>(Lyrics {
            lines,
            notice: "Fuente: Spotify".into(),
        })
    })?;
    cache.lock().put(key, result.clone());
    Ok(result)
}

fn fetch_public(track: &Track) -> Result<Lyrics, Error> {
    let artist = track
        .artists
        .front()
        .map(|artist| artist.name.as_ref())
        .unwrap_or("");
    if artist.is_empty() {
        return Err(Error::WebApiError(
            "La canción no tiene artista para buscar su letra.".into(),
        ));
    }
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .max_redirects(0)
        .http_status_as_error(false);
    if let Some(proxy) = Config::proxy().and_then(|url| ureq::Proxy::new(&url).ok()) {
        config = config.proxy(Some(proxy));
    }
    let agent: ureq::Agent = config.build().into();
    // Exact title, artist and duration prevent lyrics from unrelated songs or versions.
    for with_album in [true, false] {
        let mut url = url::Url::parse("https://lrclib.net/api/get").unwrap();
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("track_name", &track.name)
                .append_pair("artist_name", artist)
                .append_pair("duration", &track.duration.as_secs().to_string());
            if with_album {
                query.append_pair("album_name", &track.album_name());
            }
        }
        let mut response = psst_core::util::retry_network_read(|| {
            agent
                .get(url.as_str())
                .header("User-Agent", "Xpotify/0.1 (native lyrics)")
                .call()
        })
        .map_err(|_| {
            Error::WebApiError(
                "No se pudo conectar con el servicio de letras. Intenta de nuevo.".into(),
            )
        })?;
        if response.status().as_u16() == 404 {
            continue;
        }
        if !response.status().is_success() {
            return Err(Error::WebApiError(format!(
                "El servicio de letras respondió HTTP {}.",
                response.status().as_u16()
            )));
        }
        let bytes = response
            .body_mut()
            .with_config()
            .limit(2 * 1024 * 1024)
            .read_to_vec()
            .map_err(|_| Error::WebApiError("No se pudo leer la letra.".into()))?;
        let record: Record = serde_json::from_slice(&bytes)
            .map_err(|_| Error::WebApiError("El servicio devolvió una letra inválida.".into()))?;
        if record.instrumental {
            return Ok(Lyrics {
                lines: Vector::new(),
                notice: "Fuente: LRCLIB · Canción instrumental, sin letra".into(),
            });
        }
        let mut lines = record
            .synced_lyrics
            .as_deref()
            .map(parse_lrc)
            .unwrap_or_default();
        let synced = !lines.is_empty();
        if synced {
            finish_timestamps(&mut lines, track.duration.as_millis() as u64);
        } else if let Some(plain) = record.plain_lyrics {
            lines = plain
                .lines()
                .map(|words| TrackLines {
                    start_time_ms: "-1".into(),
                    words: words.into(),
                    end_time_ms: "-1".into(),
                })
                .collect();
        }
        if !lines.is_empty() {
            return Ok(Lyrics {
                lines,
                notice: if synced {
                    "Fuente: LRCLIB · Letra sincronizada · Pulsa una línea para ir a ese momento"
                } else {
                    "Fuente: LRCLIB · Letra sin sincronización"
                }
                .into(),
            });
        }
    }
    Err(Error::WebApiError(
        "No hay letra disponible para esta versión de la canción.".into(),
    ))
}

fn timestamp(value: &str) -> Option<u64> {
    let (minutes, seconds) = value.split_once(':')?;
    let minutes: u64 = minutes.parse().ok()?;
    let seconds: f64 = seconds.parse().ok()?;
    if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    minutes
        .checked_mul(60_000)?
        .checked_add((seconds * 1000.0).round() as u64)
}

fn parse_lrc(text: &str) -> Vector<TrackLines> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim_start();
        let mut times = Vec::new();
        while let Some(tag) = rest.strip_prefix('[') {
            let Some((value, tail)) = tag.split_once(']') else {
                break;
            };
            if let Some(time) = timestamp(value) {
                times.push(time);
            }
            rest = tail;
        }
        for time in times {
            lines.push(TrackLines {
                start_time_ms: time.to_string(),
                words: rest.trim().into(),
                end_time_ms: "-1".into(),
            });
        }
    }
    lines.sort_by_key(|line| line.start_time_ms.parse::<u64>().unwrap_or(0));
    lines.into_iter().collect()
}

fn finish_timestamps(lines: &mut Vector<TrackLines>, duration: u64) {
    for index in 0..lines.len() {
        let end = lines
            .iter()
            .skip(index + 1)
            .filter_map(|line| line.start_time_ms.parse::<u64>().ok())
            .find(|next| {
                *next
                    > lines[index]
                        .start_time_ms
                        .parse::<u64>()
                        .unwrap_or(u64::MAX)
            })
            .unwrap_or(duration);
        lines[index].end_time_ms = end.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_response_cannot_replace_the_new_song_lyrics() {
        let mut pending = crate::data::Promise::<Lyrics, String>::Empty;
        pending.defer("first".into());
        pending.defer("second".into());
        pending.update((
            "first".into(),
            Ok(Lyrics {
                lines: Vector::new(),
                notice: "old".into(),
            }),
        ));
        assert!(pending.is_deferred(&"second".into()));
        pending.update((
            "second".into(),
            Ok(Lyrics {
                lines: Vector::new(),
                notice: "new".into(),
            }),
        ));
        assert_eq!(pending.resolved().unwrap().notice, "new");
    }

    #[test]
    #[ignore = "Calls the public LRCLIB service; run explicitly for integration validation"]
    fn live_public_lookup_without_spotify_credentials() {
        let track: Track = serde_json::from_value(serde_json::json!({
            "name":"Instant Crush", "artists":[{"id":"preview","name":"Daft Punk"}],
            "duration_ms":337000, "disc_number":1, "track_number":1,
            "explicit":false, "is_local":false, "is_playable":true
        }))
        .unwrap();
        let lyrics = fetch_public(&track).unwrap();
        assert!(lyrics.lines.len() > 10);
        assert!(lyrics.notice.contains("LRCLIB"));
        assert!(lyrics
            .lines
            .iter()
            .any(|line| line.start_time_ms.parse::<u64>().is_ok()));
    }
    #[test]
    fn lrc_handles_multiple_timestamps_metadata_and_malformed_lines() {
        let mut lines = parse_lrc(
            "[ar:Demo]\n[00:02.5][00:04.125]Example\n[bad]Ignore\n[00:00.00]Intro\n[01:99]Ignore",
        );
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start_time_ms, "0");
        assert_eq!(lines[1].start_time_ms, "2500");
        assert_eq!(lines[2].start_time_ms, "4125");
        finish_timestamps(&mut lines, 10_000);
        assert_eq!(lines[0].end_time_ms, "2500");
        assert_eq!(lines[2].end_time_ms, "10000");
        assert_eq!(lines[1].words, "Example");
    }
    #[test]
    fn invalid_timestamps_cannot_seek_or_overflow() {
        for value in [
            "bad",
            "0:NaN",
            "0:inf",
            "0:-1",
            "0:60",
            "18446744073709551615:00",
        ] {
            assert_eq!(timestamp(value), None);
        }
    }
}
