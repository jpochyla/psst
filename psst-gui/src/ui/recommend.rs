use std::{sync::Arc, time::Duration};

use druid::{
    widget::{CrossAxisAlignment, Flex, Slider},
    FontDescriptor, FontFamily, LensExt, Selector, Widget, WidgetExt,
};

use crate::{
    data::{
        AppState, Ctx, Recommend, Recommendations, RecommendationsKnobs, RecommendationsParams,
        RecommendationsRequest, Toggled, WithCtx,
    },
    webapi::WebApi,
    widget::{Async, Checkbox, MyWidgetExt},
};

use super::{
    theme,
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, spinner_widget},
};

const KNOBS_DEBOUNCE_DELAY: Duration = Duration::from_millis(500);

pub const UPDATE_PARAMS: Selector<RecommendationsParams> =
    Selector::new("app.recommend.update-params");
pub const LOAD_RESULTS: Selector<Arc<RecommendationsRequest>> =
    Selector::new("app.recommend.load-results");

pub fn results_widget() -> impl Widget<AppState> {
    let track_results = Async::new(spinner_widget, track_results_widget, error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::recommend.then(Recommend::results),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_RESULTS,
            |d| WebApi::global().get_recommendations(d),
            |_, data, d| data.recommend.results.defer(d),
            |_, data, r| data.recommend.results.update(r),
        )
        .on_command(UPDATE_PARAMS, |ctx, params, data| {
            if let Some(previous) = data.recommend.results.deferred() {
                let previous = (**previous).clone();
                let params = params.to_owned();
                let request = previous.with_params(params);
                ctx.submit_command(LOAD_RESULTS.with(Arc::new(request)));
            }
        });

    let param_knobs = params_widget()
        .on_debounce(KNOBS_DEBOUNCE_DELAY, |ctx, knobs, _| {
            ctx.submit_command(UPDATE_PARAMS.with(knobs.as_params()));
        })
        .lens(AppState::recommend.then(Recommend::knobs));

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(param_knobs)
        .with_default_spacer()
        .with_child(track_results)
}

fn params_widget() -> impl Widget<Arc<RecommendationsKnobs>> {
    let row = |label| {
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Checkbox::new(label)
                    .lens(Toggled::enabled)
                    .env_scope(|env, _| {
                        env.set(theme::BASIC_WIDGET_HEIGHT, theme::grid(1.5));
                        env.set(
                            theme::UI_FONT,
                            FontDescriptor::new(FontFamily::SYSTEM_UI)
                                .with_size(env.get(theme::TEXT_SIZE_SMALL)),
                        );
                    }),
            )
            .with_spacer(theme::grid(0.4))
            .with_child(
                Slider::new()
                    .lens(Toggled::value)
                    .disabled_if(|toggle, _| !toggle.enabled)
                    .padding_left(theme::grid(2.5))
                    .env_scope(|env, _| {
                        env.set(theme::BASIC_WIDGET_HEIGHT, theme::grid(1.5));
                        env.set(theme::FOREGROUND_LIGHT, env.get(theme::GREY_400));
                        env.set(theme::FOREGROUND_DARK, env.get(theme::GREY_400));
                        env.set(theme::DISABLED_FOREGROUND_LIGHT, env.get(theme::GREY_600));
                        env.set(theme::DISABLED_FOREGROUND_DARK, env.get(theme::GREY_600));
                    }),
            )
            .padding(theme::grid(0.5))
    };
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(row("Acousticness").lens(RecommendationsKnobs::acousticness.in_arc()))
                .with_child(row("Danceability").lens(RecommendationsKnobs::danceability.in_arc()))
                .with_child(row("Energy").lens(RecommendationsKnobs::energy.in_arc())),
            1.0,
        )
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    row("Instrumentalness").lens(RecommendationsKnobs::instrumentalness.in_arc()),
                )
                .with_child(row("Liveness").lens(RecommendationsKnobs::liveness.in_arc()))
                .with_child(row("Loudness").lens(RecommendationsKnobs::loudness.in_arc())),
            1.0,
        )
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(row("Speechiness").lens(RecommendationsKnobs::speechiness.in_arc()))
                .with_child(row("Valence").lens(RecommendationsKnobs::valence.in_arc())),
            1.0,
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
