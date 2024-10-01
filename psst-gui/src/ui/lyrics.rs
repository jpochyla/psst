
use druid::widget::{Flex, Label, LineBreaking};
use druid::Insets;
use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::cmd;
use crate::data::{Ctx, NowPlaying, TrackLines};
use crate::{
    data::AppState,
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::theme;
use super::utils::spinner_widget;

pub const SHOW_LYRICS: Selector<NowPlaying> = Selector::new("app.home.show_lyrics");


pub fn lyrics_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(track_lyrics_widget())
}

fn track_lyrics_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || {List::new(|| {
            Label::raw()
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(Ctx::data().then(TrackLines::words))
                .expand_width()
                .padding(Insets::uniform_xy(theme::grid(2.0), theme::grid(0.6)))
                .link()
                .on_left_click(|ctx, _, c, _| ctx.submit_command(cmd::SKIP_TO_POSITION.with(c.data.start_time_ms.parse::<u64>().unwrap())))
            
        })},
        || Label::new("No lyrics for this song!"),
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::lyrics,
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        SHOW_LYRICS,
        |t| WebApi::global().get_lyrics(t.item.id().to_base62()),
        |_, data, _| data.lyrics.defer(()),
        |_, data, r| data.lyrics.update(((), r.1)),
    )
}