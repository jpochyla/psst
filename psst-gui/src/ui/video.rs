use crate::data::Track;

/// Open the provider's search instead of guessing an unverified video ID.
pub fn search_url(track: &Track) -> String {
    let mut url = url::Url::parse("https://www.youtube.com/results").unwrap();
    url.query_pairs_mut().append_pair(
        "search_query",
        &format!(
            "{} {} official music video",
            track.artist_name(),
            track.name
        ),
    );
    url.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn song_names_cannot_change_the_video_destination() {
        let track: Track = serde_json::from_value(serde_json::json!({
            "name": "Song & next=https://evil.example/#fragment", "artists": [{"id":"preview", "name":"Artist"}],
            "duration_ms":240000, "disc_number":1, "track_number":1, "explicit":false, "is_local":false, "is_playable":true
        })).unwrap();
        let url = url::Url::parse(&search_url(&track)).unwrap();
        assert_eq!(url.host_str(), Some("www.youtube.com"));
        assert!(url.fragment().is_none());
        assert_eq!(url.query_pairs().count(), 1);
        assert!(url.query_pairs().next().unwrap().1.contains(&*track.name));
    }
}
