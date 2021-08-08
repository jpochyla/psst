use std::sync::Arc;

use druid::{LensExt, Selector, Widget, WidgetExt};

use crate::{
    data::{AppState, Ctx, Recommend, Recommendations, RecommendationsRequest, WithCtx},
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, spinner_widget},
};

pub const LOAD_RESULTS: Selector<Arc<RecommendationsRequest>> =
    Selector::new("app.recommend.load-results");

pub fn results_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, track_results_widget, error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::recommend.then(Recommend::results),
            )
            .then(Ctx::in_promise()),
        )
        .on_cmd_async(
            LOAD_RESULTS,
            |d| WebApi::global().get_recommendations(d),
            |_, data, d| data.recommend.results.defer(d),
            |_, data, r| data.recommend.results.update(r),
        )
}

fn track_results_widget() -> impl Widget<WithCtx<Recommendations>> {
    tracklist_widget(TrackDisplay {
        title: true,
        artist: true,
        album: true,
        ..TrackDisplay::empty()
    })
}
