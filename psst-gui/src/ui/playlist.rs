use crate::{
    cmd,
    data::{Ctx, Library, Navigation, Playlist, PlaylistDetail, State},
    ui::{
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader},
    },
    widget::{HoverExt, Promised},
};
use druid::{
    widget::{Label, LineBreaking, List},
    Insets, LensExt, Widget, WidgetExt,
};

pub fn make_list() -> impl Widget<State> {
    Promised::new(
        || make_loader(),
        || {
            List::new(|| {
                Label::dynamic(|playlist: &Playlist, _| playlist.name.clone())
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(theme::TEXT_SIZE_SMALL)
                    .expand_width()
                    .padding(Insets::uniform_xy(theme::grid(2.0), theme::grid(0.6)))
                    .hover()
                    .on_click(|ctx, playlist, _| {
                        let nav = Navigation::PlaylistDetail(playlist.clone());
                        ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
                    })
            })
        },
        || make_error(),
    )
    .lens(Library::playlists)
    .lens(State::library)
}

pub fn make_detail() -> impl Widget<State> {
    Promised::new(
        || make_loader(),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: true,
                album: true,
            })
        },
        || make_error(),
    )
    .lens(
        Ctx::make(
            State::track_ctx,
            State::playlist.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
    )
}
