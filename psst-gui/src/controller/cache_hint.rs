use crate::{
    data::{AppState, Nav},
    webapi::WebApi,
    widget::MyWidgetExt,
};
use druid::{widget::Controller, Data, Env, UpdateCtx, Widget, WidgetExt};
const UPDATE: druid::Selector<Nav> = druid::Selector::new("cache.hint.update");

pub fn widget(child: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    child
        .controller(CacheHint)
        .on_command(UPDATE, |_, nav, state| {
            if nav != &state.nav {
                return;
            }
            let cached = match nav {
                Nav::AlbumDetail(_, _) => state
                    .album_detail
                    .album
                    .resolved()
                    .and_then(|album| album.cached_at),
                Nav::ArtistDetail(_) => state
                    .artist_detail
                    .overview
                    .resolved()
                    .and_then(|artist| artist.cached_at),
                _ => WebApi::global().cache_origin(nav),
            };
            state.cache_notice = cached
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|duration| {
                    time::OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64).ok()
                })
                .map(|time| {
                    format!(
                        "Caché: {} {:02}:{:02} UTC",
                        time.date(),
                        time.hour(),
                        time.minute()
                    )
                })
                .unwrap_or_default();
        })
}

struct CacheHint;
impl<W: Widget<AppState>> Controller<AppState, W> for CacheHint {
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old: &AppState,
        state: &AppState,
        env: &Env,
    ) {
        if old.nav != state.nav
            || !old
                .playlist_detail
                .tracks
                .same(&state.playlist_detail.tracks)
            || !old.album_detail.album.same(&state.album_detail.album)
            || !old
                .artist_detail
                .overview
                .same(&state.artist_detail.overview)
            || !old.library.same(&state.library)
            || !old.search.results.same(&state.search.results)
        {
            ctx.submit_command(UPDATE.with(state.nav.clone()));
        }
        child.update(ctx, old, state, env);
    }
}
