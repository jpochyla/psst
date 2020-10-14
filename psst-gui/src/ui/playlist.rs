use crate::{
    commands,
    data::{Library, Navigation, Playlist, PlaylistDetail, State},
    ui::{
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::make_placeholder,
    },
    widgets::{HoverExt, Maybe},
};
use druid::{
    widget::{Label, LineBreaking, List},
    Insets, Widget, WidgetExt,
};

pub fn make_list() -> impl Widget<State> {
    Maybe::or_empty(|| {
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
    })
    .lens(Library::playlists)
    .lens(State::library)
}

pub fn make_detail() -> impl Widget<State> {
    Maybe::new(
        || {
            make_tracklist(TrackDisplay {
                title: true,
                artist: true,
                album: true,
            })
        },
        make_placeholder,
    )
    .lens(PlaylistDetail::tracks)
    .lens(State::playlist)
}
