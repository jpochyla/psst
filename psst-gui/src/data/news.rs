use super::{Album, Promise};
use druid::{im::Vector, Data, Lens};
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
pub struct Release {
    pub album: Arc<Album>,
    pub artist: String,
    pub artist_id: String,
    pub date: String,
    pub unread: bool,
}

#[derive(Clone, Data, Lens)]
pub struct NewsFeed {
    pub releases: Vector<Release>,
    pub followed_count: usize,
    pub failed_count: usize,
    pub notice: String,
}

#[derive(Clone, Data, Lens, Default)]
pub struct NewsState {
    pub feed: Promise<NewsFeed>,
}

#[derive(Clone, Data, Lens)]
pub struct ArtistReleases {
    pub id: String,
    pub name: String,
    pub releases: Vector<Release>,
}

pub const RECENT_DAYS: i64 = 14;

pub fn is_recent(date: Option<time::Date>, today: time::Date) -> bool {
    date.is_some_and(|date| date >= today - time::Duration::days(RECENT_DAYS) && date <= today)
}

pub fn group_by_artist(releases: &Vector<Release>) -> Vector<ArtistReleases> {
    let mut groups: Vector<ArtistReleases> = Vector::new();
    let mut positions = std::collections::HashMap::new();
    // The feed is newest first: preserve that order for both artists and releases.
    for release in releases {
        let index = *positions
            .entry(release.artist_id.clone())
            .or_insert_with(|| {
                groups.push_back(ArtistReleases {
                    id: release.artist_id.clone(),
                    name: release.artist.clone(),
                    releases: Vector::new(),
                });
                groups.len() - 1
            });
        groups[index].releases.push_back(release.clone());
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_week_window_includes_boundary_and_excludes_old_and_future_releases() {
        let today = time::Date::from_calendar_date(2026, time::Month::October, 9).unwrap();
        assert!(is_recent(Some(today), today));
        assert!(is_recent(Some(today - time::Duration::days(14)), today));
        assert!(!is_recent(Some(today - time::Duration::days(15)), today));
        assert!(!is_recent(Some(today + time::Duration::days(1)), today));
        assert!(!is_recent(None, today));
    }
    #[test]
    fn grouping_preserves_latest_first_and_keeps_different_artists_with_the_same_name_separate() {
        let album: Album = serde_json::from_value(serde_json::json!({
            "id":"release", "name":"Release", "album_type":"single", "release_date":"2026-10-09", "release_date_precision":"day"
        })).unwrap();
        let releases = ["a", "b", "a"]
            .into_iter()
            .enumerate()
            .map(|(i, id)| Release {
                album: Arc::new(album.clone()),
                artist: "Same name".into(),
                artist_id: id.into(),
                date: format!("2026-10-0{}", 9 - i),
                unread: i == 0,
            })
            .collect();
        let groups = group_by_artist(&releases);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, "a");
        assert_eq!(groups[0].releases.len(), 2);
        assert_eq!(groups[0].releases[1].date, "2026-10-07");
        assert!(groups[0].releases[0].unread);
        assert!(!groups[0].releases[1].unread);
        assert_eq!(groups[1].id, "b");
    }
}
