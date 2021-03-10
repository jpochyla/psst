use crate::{
    cmd,
    data::{CommonCtx, Ctx, Library, Nav, Playlist, PlaylistDetail, State},
    ui::{
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader},
    },
    widget::{Async, HoverExt},
};
use druid::{
    widget::{Label, LineBreaking, List},
    Insets, LensExt, MouseButton, Widget, WidgetExt,
};

pub fn make_list() -> impl Widget<State> {
    Async::new(
        || make_loader(),
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
        || make_error(),
    )
    .lens(State::library.then(Library::playlists.in_arc()))
}

pub fn make_playlist() -> impl Widget<Ctx<CommonCtx, Playlist>> {
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

pub fn make_detail() -> impl Widget<State> {
    Async::new(
        || make_loader(),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: true,
                album: true,
            })
        },
        || make_error().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::playlist.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
    )
}
