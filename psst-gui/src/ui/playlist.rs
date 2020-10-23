use crate::{
    commands,
    ctx::Ctx,
    data::{Library, Navigation, Playlist, PlaylistDetail, State},
    ui::{
        theme,
        track::{make_tracklist, TrackDisplay},
    },
    widgets::{HoverExt, Promised},
};
use druid::{
    widget::{Label, LineBreaking, List},
    Insets, LensExt, Widget, WidgetExt,
};

pub fn make_list() -> impl Widget<State> {
    Promised::new(
        || Label::new("Loading"),
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
                        ctx.submit_command(commands::NAVIGATE_TO.with(nav));
                    })
            })
        },
        || Label::new("Error"),
    )
    .lens(Library::playlists)
    .lens(State::library)
}

pub fn make_detail() -> impl Widget<State> {
    Promised::new(
        || Label::new("Loading"),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: true,
                album: true,
            })
        },
        || Label::new("Error"),
    )
    .lens(
        Ctx::make(
            State::track_context(),
            State::playlist.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
    )
}
