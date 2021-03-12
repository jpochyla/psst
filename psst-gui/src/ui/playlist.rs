use crate::{
    cmd,
    data::{CommonCtx, Ctx, Library, Nav, Playlist, PlaylistDetail, State},
    ui::{
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    widget::{Async, HoverExt},
};
use druid::{
    widget::{Label, LineBreaking, List},
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
                        ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
                    })
            })
        },
        || error_widget(),
    )
    .lens(State::library.then(Library::playlists.in_arc()))
}

pub fn playlist_widget() -> impl Widget<Ctx<CommonCtx, Playlist>> {
    let playlist_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Playlist::name);

    let playlist = playlist_name.padding(theme::grid(1.0)).lens(Ctx::data());

    playlist.hover().on_ex_click(
        move |ctx, event, playlist: &mut Ctx<CommonCtx, Playlist>, _| match event.button {
            MouseButton::Left => {
                let nav = Nav::PlaylistDetail(playlist.data.link());
                ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
            }
            _ => {}
        },
    )
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
