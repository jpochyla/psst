use crate::{
    cmd,
    data::{CommonCtx, Ctx, Library, Nav, Playlist, PlaylistDetail, State},
    ui::{
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    webapi::WebApi,
    widget::{Async, AsyncAction, HoverExt},
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List},
    Insets, LensExt, MouseButton, Widget, WidgetExt,
};

pub fn list_widget() -> impl Widget<State> {
    Async::new(
        || spinner_widget(),
        || {
            List::new(|| {
                Label::raw()
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(theme::TEXT_SIZE_SMALL)
                    .lens(Playlist::name)
                    .expand_width()
                    .padding(Insets::uniform_xy(theme::grid(2.0), theme::grid(0.6)))
                    .hover()
                    .on_click(|ctx, playlist, _| {
                        let nav = Nav::PlaylistDetail(playlist.link());
                        ctx.submit_command(cmd::NAVIGATE.with(nav));
                    })
            })
        },
        || error_widget(),
    )
    .controller(AsyncAction::new(|_| WebApi::global().get_playlists()))
    .lens(State::library.then(Library::playlists.in_arc()))
}

pub fn playlist_widget() -> impl Widget<Ctx<CommonCtx, Playlist>> {
    let playlist_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Playlist::name);

    let track_count = Label::dynamic(|&track_count, _| match track_count {
        0 => format!("Empty"),
        1 => format!("1 track"),
        n => format!("{} tracks", n),
    })
    .with_text_color(theme::PLACEHOLDER_COLOR)
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .lens(Playlist::track_count);

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(playlist_name)
        .with_spacer(2.0)
        .with_child(track_count)
        .padding(theme::grid(1.0))
        .hover()
        .on_ex_click(
            move |ctx, event, playlist: &mut Playlist, _| match event.button {
                MouseButton::Left => {
                    let nav = Nav::PlaylistDetail(playlist.link());
                    ctx.submit_command(cmd::NAVIGATE.with(nav));
                }
                _ => {}
            },
        )
        .lens(Ctx::data())
}

pub fn detail_widget() -> impl Widget<State> {
    Async::new(
        || spinner_widget(),
        || {
            tracklist_widget(TrackDisplay {
                title: true,
                artist: true,
                album: true,
                ..TrackDisplay::empty()
            })
        },
        || error_widget().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::playlist.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
    )
}
