use std::sync::Arc;

use druid::{LensExt, Widget, WidgetExt};

use crate::{
    data::{AppState, CommonCtx, Ctx, Recommend, Recommendations},
    widget::Async,
};

use super::{
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, spinner_widget},
};

pub fn results_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, track_results_widget, || {
        error_widget().lens(Ctx::data())
    })
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::recommend.then(Recommend::results),
        )
        .then(Ctx::in_promise()),
    )
}

fn track_results_widget() -> impl Widget<Ctx<Arc<CommonCtx>, Recommendations>> {
    tracklist_widget(TrackDisplay {
        title: true,
        artist: true,
        album: true,
        ..TrackDisplay::empty()
    })
}
