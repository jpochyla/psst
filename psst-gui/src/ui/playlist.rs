use std::{cell::RefCell, rc::Rc, sync::Arc};

use druid::{
    im::Vector,
    widget::{Button, Either, Flex, Label, LensWrap, LineBreaking, List, TextBox},
    Lens, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget, WidgetExt,
    WindowDesc,
};
use itertools::Itertools;

use crate::{
    cmd,
    data::{
        config::{SortCriteria, SortOrder},
        AppState, Ctx, Nav, Playlist, PlaylistAddTrack, PlaylistDetail, PlaylistLink,
        PlaylistRemoveTrack, PlaylistTracks, Track, WithCtx,
    },
    error::Error,
    ui::menu,
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage, ThemeScope},
};

use super::{playable, theme, track, utils};

pub fn picker_cover() -> impl Widget<Playlist> {
    RemoteImage::new(
        crate::widget::icons::PLAYLIST
            .scale((24.0, 24.0))
            .with_color(theme::PLACEHOLDER_COLOR)
            .center()
            .background(theme::GREY_700),
        |playlist: &Playlist, _| playlist.image(48.0, 48.0).map(|image| image.url.clone()),
    )
}

fn library_row_widget() -> impl Widget<WithCtx<Playlist>> {
    let cover = picker_cover()
        .lens(Ctx::data())
        .fix_size(48.0, 48.0)
        .clip(Size::new(48.0, 48.0).to_rounded_rect(5.0));
    let labels = Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(
            Label::raw()
                .with_font(theme::UI_FONT_MEDIUM)
                .with_line_break_mode(LineBreaking::Clip)
                .lens(Ctx::data().then(Playlist::name))
                .expand_width(),
        )
        .with_spacer(5.0)
        .with_child(
            Label::dynamic(|row: &WithCtx<Playlist>, _| {
                format!(
                    "Playlist · {}",
                    if row.data.owner.display_name.is_empty() {
                        &row.data.owner.id
                    } else {
                        &row.data.owner.display_name
                    }
                )
            })
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_line_break_mode(LineBreaking::Clip)
            .expand_width(),
        );
    Flex::row().with_child(cover).with_spacer(12.0).with_flex_child(labels, 1.0)
        .padding((10.0, 8.0)).expand_width().link().rounded(6.0)
        .active(|row: &WithCtx<Playlist>, _| matches!(&row.ctx.nav, Nav::PlaylistDetail(link) if link.id == row.data.id))
        .on_left_click(|ctx, _, row, _| ctx.submit_command(cmd::NAVIGATE.with(Nav::PlaylistDetail(row.data.link()))))
        .context_menu(playlist_menu_ctx)
}

pub const LOAD_LIST: Selector = Selector::new("app.playlist.load-list");
pub const LOAD_DETAIL: Selector<(PlaylistLink, AppState)> =
    Selector::new("app.playlist.load-detail");
pub const ADD_TRACK: Selector<PlaylistAddTrack> = Selector::new("app.playlist.add-track");
pub const REMOVE_TRACK: Selector<PlaylistRemoveTrack> = Selector::new("app.playlist.remove-track");
pub const REORDER_TRACK: Selector<crate::data::PlaylistReorder> =
    Selector::new("app.playlist.reorder-track");
const SET_SORT: Selector<(SortCriteria, SortOrder)> = Selector::new("app.playlist.set-sort");

pub const FOLLOW_PLAYLIST: Selector<Playlist> = Selector::new("app.playlist.follow");
pub const UNFOLLOW_PLAYLIST: Selector<PlaylistLink> = Selector::new("app.playlist.unfollow");
pub const UNFOLLOW_PLAYLIST_CONFIRM: Selector<PlaylistLink> =
    Selector::new("app.playlist.unfollow-confirm");

pub const RENAME_PLAYLIST: Selector<PlaylistLink> = Selector::new("app.playlist.rename");
pub const RENAME_PLAYLIST_CONFIRM: Selector<PlaylistLink> =
    Selector::new("app.playlist.rename-confirm");

const SHOW_RENAME_PLAYLIST_CONFIRM: Selector<PlaylistLink> =
    Selector::new("app.playlist.show-rename");
const SHOW_UNFOLLOW_PLAYLIST_CONFIRM: Selector<UnfollowPlaylist> =
    Selector::new("app.playlist.show-unfollow-confirm");

pub fn list_widget() -> impl Widget<AppState> {
    list_body(false)
}

pub fn compact_list_widget() -> impl Widget<AppState> {
    list_body(true)
}

fn compact_library_row() -> impl Widget<WithCtx<Playlist>> {
    druid::widget::ViewSwitcher::new(
        |row: &WithCtx<Playlist>, _| row.data.name.clone(),
        |_, row, _| {
            picker_cover()
                .lens(Ctx::data())
                .fix_size(48.0, 48.0)
                .clip(Size::new(48.0, 48.0).to_rounded_rect(5.0))
                .padding(4.0)
                .link()
                .rounded(7.0)
                .active(|row: &WithCtx<Playlist>, _| matches!(&row.ctx.nav, Nav::PlaylistDetail(link) if link.id == row.data.id))
                .tooltip(format!("{} · Playlist", row.data.name))
                .on_left_click(|ctx, _, row, _| ctx.submit_command(cmd::NAVIGATE.with(Nav::PlaylistDetail(row.data.link()))))
                .context_menu(playlist_menu_ctx)
                .boxed()
        },
    )
}

fn list_body(compact: bool) -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        move || {
            List::new(move || {
                if compact {
                    compact_library_row().boxed()
                } else {
                    library_row_widget().boxed()
                }
            })
        },
        utils::error_widget,
    )
    .lens(
        druid::lens::Map::new(super::folders::filtered_playlists, |_, _| {})
            .then(Ctx::in_promise()),
    )
}

/// Keep pending requests and playlist mutations alive when the rail changes mode.
pub fn list_controller(inner: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    inner
        .on_command_async(
            LOAD_LIST,
            |_| WebApi::global().get_playlists(),
            |_, data, d| data.with_library_mut(|l| l.playlists.defer(d)),
            |_, data, r| data.with_library_mut(|l| l.playlists.update(r)),
        )
        .on_command_async(
            ADD_TRACK,
            |d| {
                WebApi::global().add_track_to_playlist(
                    &d.link.id,
                    &d.track_id
                        .0
                        .to_uri()
                        .ok_or_else(|| Error::WebApiError("Item doesn't have URI".to_string()))?,
                )
            },
            |_, _, _| {},
            |ctx, data, (d, r)| {
                if let Err(err) = r {
                    data.error_alert(err);
                } else {
                    data.with_library_mut(|library| {
                        library.increment_playlist_track_count(&d.link)
                    });
                    data.info_alert("Added to playlist.");
                    if matches!(&data.nav, Nav::PlaylistDetail(link) if link.id == d.link.id) {
                        ctx.submit_command(cmd::NAVIGATE_REFRESH);
                    }
                }
            },
        )
        .on_command_async(
            UNFOLLOW_PLAYLIST,
            |link| WebApi::global().unfollow_playlist(link.id.as_ref()),
            |_, _, _| {},
            |_, data, (d, r)| {
                if let Err(err) = r {
                    data.error_alert(err);
                } else {
                    data.with_library_mut(|l| l.remove_from_playlist(&d.id));
                    data.info_alert("Playlist removed from library.");
                }
            },
        )
        .on_command_async(
            FOLLOW_PLAYLIST,
            |link| WebApi::global().follow_playlist(link.id.as_ref()),
            |_, _, _| {},
            |_, data: &mut AppState, (d, r)| {
                if let Err(err) = r {
                    data.error_alert(err);
                } else {
                    data.with_library_mut(|l| l.add_playlist(d));
                    data.info_alert("Playlist added to library.")
                }
            },
        )
        .on_command_async(
            RENAME_PLAYLIST,
            |link| WebApi::global().change_playlist_details(link.id.as_ref(), link.name.as_ref()),
            |_, _, _| {},
            |ctx, data: &mut AppState, (link, r)| {
                if let Err(err) = r {
                    data.error_alert(err);
                } else {
                    let current =
                        matches!(&data.nav, Nav::PlaylistDetail(current) if current.id == link.id);
                    data.with_library_mut(|l| l.rename_playlist(link));
                    data.info_alert("Playlist renamed.");
                    if current {
                        ctx.submit_command(cmd::NAVIGATE_REFRESH);
                    }
                }
            },
        )
        .on_command(SHOW_UNFOLLOW_PLAYLIST_CONFIRM, |ctx, msg, _| {
            let window = unfollow_confirm_window(msg.clone());
            ctx.new_window(window);
        })
        .on_command(SHOW_RENAME_PLAYLIST_CONFIRM, |ctx, link, _| {
            let window = rename_playlist_window(link.clone());
            ctx.new_window(window);
        })
        .on_command_async(
            REMOVE_TRACK,
            |d| WebApi::global().remove_track_from_playlist(&d.link.id, &d.track_uri),
            |_, _, _| {},
            |e, data, (p, r)| {
                if let Err(err) = r {
                    data.error_alert(err);
                } else {
                    data.with_library_mut(|library| {
                        library.decrement_playlist_track_count(&p.link)
                    });
                    data.info_alert("Removed from playlist.");
                }
                // Re-submit the `LOAD_DETAIL` command to reload the playlist data.
                e.submit_command(LOAD_DETAIL.with((p.link, data.clone())))
            },
        )
        .on_command_async(
            REORDER_TRACK,
            |move_track| WebApi::global().reorder_playlist_track(&move_track),
            |_, _, _| {},
            |ctx, data, (_, result)| match result {
                Ok(()) => {
                    data.info_alert("Orden de la playlist actualizado.");
                    ctx.submit_command(cmd::NAVIGATE_REFRESH);
                }
                Err(error) => data.error_alert(error),
            },
        )
}

fn unfollow_confirm_window(msg: UnfollowPlaylist) -> WindowDesc<AppState> {
    let win = WindowDesc::new(unfollow_playlist_confirm_widget(msg))
        .window_size((theme::grid(45.0), theme::grid(25.0)))
        .title("Unfollow playlist")
        .resizable(false)
        .show_title(false)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

fn unfollow_playlist_confirm_widget(msg: UnfollowPlaylist) -> impl Widget<AppState> {
    let link = msg.link;

    let information_section = if msg.created_by_user {
        information_section(
            format!("Delete {} from Library?", link.name).as_str(),
            "This will delete the playlist from Your Library",
        )
    } else {
        information_section(
            format!("Remove {} from Library?", link.name).as_str(),
            "We'll remove this playlist from Your Library, but you'll still be able to search for it on Spotify",
        )
    };

    let button_section = button_section(
        "Delete",
        UNFOLLOW_PLAYLIST_CONFIRM,
        Box::new(move || link.clone()),
    );

    ThemeScope::new(
        Flex::column()
            .with_child(information_section)
            .with_flex_spacer(2.0)
            .with_child(button_section)
            .with_flex_spacer(2.0)
            .background(theme::BACKGROUND_DARK),
    )
}

fn rename_playlist_window(link: PlaylistLink) -> WindowDesc<AppState> {
    let win = WindowDesc::new(rename_playlist_widget(link))
        .window_size((theme::grid(45.0), theme::grid(30.0)))
        .title("Rename playlist")
        .resizable(false)
        .show_title(false)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

#[derive(Clone, Lens)]
struct TextInput {
    input: Rc<RefCell<String>>,
}

impl Lens<AppState, String> for TextInput {
    fn with<V, F: FnOnce(&String) -> V>(&self, _data: &AppState, f: F) -> V {
        f(&self.input.borrow())
    }

    fn with_mut<V, F: FnOnce(&mut String) -> V>(&self, _data: &mut AppState, f: F) -> V {
        f(&mut self.input.borrow_mut())
    }
}

fn rename_playlist_widget(link: PlaylistLink) -> impl Widget<AppState> {
    let text_input = TextInput {
        input: Rc::new(RefCell::new(link.name.to_string())),
    };

    let information_section = information_section(
        "Rename playlist?",
        "Please enter a new name for your playlist",
    );
    let input_section = LensWrap::new(
        TextBox::new()
            .padding_horizontal(theme::grid(2.0))
            .expand_width(),
        text_input.clone(),
    );
    let button_section = button_section(
        "Rename",
        RENAME_PLAYLIST_CONFIRM,
        Box::new(move || PlaylistLink {
            id: link.id.clone(),
            name: Arc::from(text_input.input.borrow().clone().into_boxed_str()),
        }),
    );

    ThemeScope::new(
        Flex::column()
            .with_child(information_section)
            .with_child(input_section)
            .with_flex_spacer(2.0)
            .with_child(button_section)
            .with_flex_spacer(2.0)
            .background(theme::BACKGROUND_DARK),
    )
}

fn button_section(
    action_button_name: &str,
    selector: Selector<PlaylistLink>,
    link_extractor: Box<dyn Fn() -> PlaylistLink>,
) -> impl Widget<AppState> {
    let action_button = Button::new(action_button_name)
        .fix_height(theme::grid(5.0))
        .fix_width(theme::grid(9.0))
        .on_click(move |ctx, _, _| {
            ctx.submit_command(selector.with(link_extractor()));
            ctx.window().close();
        });
    let cancel_button = Button::new("Cancel")
        .fix_height(theme::grid(5.0))
        .fix_width(theme::grid(8.0))
        .padding_left(theme::grid(3.0))
        .padding_right(theme::grid(2.0))
        .on_click(|ctx, _, _| ctx.window().close());

    Flex::row()
        .with_child(action_button)
        .with_child(cancel_button)
        .align_right()
}

fn information_section(title_msg: &str, description_msg: &str) -> impl Widget<AppState> {
    let title_label = Label::new(title_msg)
        .with_text_size(theme::TEXT_SIZE_LARGE)
        .align_left()
        .padding(theme::grid(2.0));

    let description_label = Label::new(description_msg)
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(theme::TEXT_SIZE_NORMAL)
        .align_left()
        .padding(theme::grid(2.0));

    Flex::column()
        .with_child(title_label)
        .with_child(description_label)
}

pub fn playlist_widget(horizontal: bool) -> impl Widget<WithCtx<Playlist>> {
    let playlist_image_size = if horizontal {
        theme::grid(16.0)
    } else {
        theme::grid(6.0)
    };
    let playlist_image = rounded_cover_widget(playlist_image_size).lens(Ctx::data());

    let playlist_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Ctx::data().then(Playlist::name));

    let playlist_description = Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .lens(Ctx::data().then(Playlist::description));

    let (playlist_name, playlist_description) = if horizontal {
        (
            playlist_name.fix_width(playlist_image_size).align_left(),
            playlist_description
                .fix_width(playlist_image_size)
                .align_left(),
        )
    } else {
        (
            playlist_name.align_left(),
            playlist_description.align_left(),
        )
    };

    let playlist = if horizontal {
        Flex::column()
            .with_child(playlist_image)
            .with_default_spacer()
            .with_child(
                Flex::column()
                    .with_child(playlist_name)
                    .with_spacer(2.0)
                    .with_child(playlist_description)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(10.0))
                    .clip(Size::new(theme::grid(16.0), theme::grid(10.0)).to_rect()),
            )
            .padding(theme::grid(1.0))
    } else {
        Flex::row()
            .with_child(playlist_image)
            .with_default_spacer()
            .with_flex_child(
                Flex::column()
                    .with_child(playlist_name)
                    .with_spacer(2.0)
                    .with_child(playlist_description),
                1.0,
            )
            .padding(theme::grid(1.0))
    };

    playlist
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, playlist, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::PlaylistDetail(playlist.data.link())));
        })
        .context_menu(playlist_menu_ctx)
}

fn cover_widget(size: f64) -> impl Widget<Playlist> {
    RemoteImage::new(
        utils::placeholder_widget(),
        move |playlist: &Playlist, _| playlist.image(size, size).map(|image| image.url.clone()),
    )
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Playlist> {
    // TODO: Take the radius from theme.
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

pub fn detail_widget() -> impl Widget<AppState> {
    use druid::widget::CrossAxisAlignment;

    let playlist_top = async_playlist_info_widget().padding(theme::grid(1.0));

    let playlist_tracks = async_tracks_widget();

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_spacer(theme::grid(1.0))
        .with_child(playlist_top)
        .with_spacer(theme::grid(1.0))
        .with_child(playlist_tracks)
}

pub fn playlist_toolbar() -> impl Widget<AppState> {
    let search = TextBox::new()
        .with_placeholder("Buscar canción o artista en esta playlist")
        .expand_width()
        .lens(AppState::playlist_detail.then(PlaylistDetail::query));
    let clear = Button::new("Limpiar")
        .on_click(|_, data: &mut AppState, _| data.playlist_detail.query.clear())
        .disabled_if(|data, _| data.playlist_detail.query.is_empty());
    let sort = Button::dynamic(|data: &AppState, _| {
        format!("Orden: {} ▾", sort_label(data.config.sort_criteria))
    })
    .on_click(|ctx, data, _| {
        let mut menu: Menu<AppState> = Menu::empty();
        for criteria in [
            SortCriteria::Title,
            SortCriteria::Artist,
            SortCriteria::Album,
            SortCriteria::DateAdded,
            SortCriteria::Duration,
        ] {
            menu = menu.entry(
                MenuItem::new(sort_label(criteria))
                    .selected(criteria == data.config.sort_criteria)
                    .command(SET_SORT.with((criteria, data.config.sort_order))),
            );
        }
        ctx.show_context_menu(
            menu,
            ctx.window_origin() + druid::Vec2::new(0.0, ctx.size().height),
        );
    });
    let direction = Button::dynamic(|data: &AppState, _| {
        match (data.config.sort_criteria, data.config.sort_order) {
            (SortCriteria::DateAdded, SortOrder::Ascending) => "Más antiguas primero ↑",
            (SortCriteria::DateAdded, SortOrder::Descending) => "Más recientes primero ↓",
            (SortCriteria::Duration, SortOrder::Ascending) => "Menor duración ↑",
            (SortCriteria::Duration, SortOrder::Descending) => "Mayor duración ↓",
            (_, SortOrder::Ascending) => "A → Z ↑",
            (_, SortOrder::Descending) => "Z → A ↓",
        }
        .to_string()
    })
    .on_click(|ctx, data, _| {
        let order = if data.config.sort_order == SortOrder::Ascending {
            SortOrder::Descending
        } else {
            SortOrder::Ascending
        };
        ctx.submit_command(SET_SORT.with((data.config.sort_criteria, order)));
    });
    Flex::column()
        .with_child(
            Flex::row()
                .with_flex_child(search, 1.0)
                .with_spacer(8.0)
                .with_child(clear),
        )
        .with_spacer(8.0)
        .with_child(
            Flex::row()
                .with_child(sort)
                .with_spacer(8.0)
                .with_child(direction)
                .with_flex_spacer(1.0),
        )
        .expand_width()
        .on_command(SET_SORT, |_, (criteria, order), data| {
            data.config.sort_criteria = *criteria;
            data.config.sort_order = *order;
            if let Some(loaded) = data.playlist_detail.tracks.resolved().cloned() {
                if let Ok(sorted) = sort_playlist(data, Ok(loaded.tracks)) {
                    data.playlist_detail.tracks.resolved_mut().unwrap().tracks = sorted;
                }
            }
        })
}

fn sort_label(criteria: SortCriteria) -> &'static str {
    match criteria {
        SortCriteria::Title => "Título",
        SortCriteria::Artist => "Artista",
        SortCriteria::Album => "Álbum",
        SortCriteria::DateAdded => "Fecha de agregado",
        SortCriteria::Duration => "Duración",
    }
}

pub fn play_button() -> impl Widget<AppState> {
    Button::new("▶ Reproducir playlist")
        .on_click(|ctx, state: &mut AppState, _| {
            if let Some(payload) = playlist_playback(state) {
                ctx.submit_command(cmd::PLAY_TRACKS.with(payload));
            }
        })
        .disabled_if(|state, _| playlist_for_playback(state).is_none())
        .tooltip("Reproducir esta playlist desde el principio con el modo actual")
}

fn playlist_playback(state: &AppState) -> Option<crate::data::PlaybackPayload> {
    let tracks = playlist_for_playback(state)?;
    Some(crate::data::PlaybackPayload {
        items: tracks
            .tracks
            .iter()
            .cloned()
            .map(crate::data::Playable::Track)
            .collect(),
        position: 0,
        origin: crate::data::PlaybackOrigin::Playlist(tracks.link()),
    })
}

fn playlist_for_playback(state: &AppState) -> Option<&PlaylistTracks> {
    let Nav::PlaylistDetail(link) = &state.nav else {
        return None;
    };
    let tracks = state.playlist_detail.tracks.resolved()?;
    if tracks.id != link.id || tracks.tracks.is_empty() {
        return None;
    }
    Some(tracks)
}

#[cfg(test)]
mod playback_button_tests {
    use super::*;

    #[test]
    fn sorting_uses_dates_not_playlist_positions_and_handles_both_directions() {
        let mut state = AppState::default_with_config(crate::data::Config::default());
        let tracks: Vector<Arc<Track>> = [
            ("zebra", "Alpha", "Zulu", 120000, "2026-01-02T00:00:00Z"),
            ("Alpha", "zebra", "Alpha", 240000, "2026-01-01T00:00:00Z"),
        ]
        .into_iter()
        .enumerate()
        .map(|(position, (name, artist, album, duration, date))| {
            let mut track: Track = serde_json::from_value(serde_json::json!({
                "name":name, "artists":[{"id":"artist","name":artist}],
                "album":{"id":"album","name":album,"images":[]}, "duration_ms":duration,
                "disc_number":1,"track_number":1,"explicit":false,"is_local":false,
                "playlist_added_at":date,
            }))
            .unwrap();
            track.track_pos = position;
            Arc::new(track)
        })
        .collect();
        for (criteria, first_position) in [
            (SortCriteria::Title, 1),
            (SortCriteria::Artist, 0),
            (SortCriteria::Album, 1),
            (SortCriteria::Duration, 0),
            (SortCriteria::DateAdded, 1),
        ] {
            state.config.sort_criteria = criteria;
            for order in [SortOrder::Ascending, SortOrder::Descending] {
                state.config.sort_order = order;
                let sorted = sort_playlist(&state, Ok(tracks.clone())).unwrap();
                let expected = if order == SortOrder::Ascending {
                    first_position
                } else {
                    1 - first_position
                };
                assert_eq!(sorted[0].track_pos, expected, "{criteria:?} {order:?}");
                assert_eq!(
                    sort_playlist(&state, Ok(sorted)).unwrap()[0].track_pos,
                    expected
                );
            }
        }
    }

    #[test]
    fn playlist_button_plays_all_loaded_pages_and_rejects_stale_or_empty_data() {
        let mut state = AppState::default_with_config(crate::data::Config::default());
        let link = PlaylistLink {
            id: "playlist-a".into(),
            name: "Playlist A".into(),
        };
        state.nav = Nav::PlaylistDetail(link.clone());
        assert!(playlist_playback(&state).is_none());
        let track: Track = serde_json::from_value(serde_json::json!({
            "name":"Repeated song", "artists":[], "duration_ms":240000,
            "disc_number":1,"track_number":1,"explicit":false,"is_local":false
        }))
        .unwrap();
        let track = Arc::new(track);
        state.playlist_detail.tracks.resolve(
            link.clone(),
            PlaylistTracks {
                query: String::new(),
                id: link.id.clone(),
                name: link.name.clone(),
                tracks: (0..750).map(|_| track.clone()).collect(),
            },
        );
        let payload = playlist_playback(&state).unwrap();
        assert_eq!(payload.items.len(), 750);
        assert_eq!(payload.position, 0);
        assert!(
            matches!(payload.origin, crate::data::PlaybackOrigin::Playlist(origin) if origin.id == link.id)
        );
        state.nav = Nav::PlaylistDetail(PlaylistLink {
            id: "playlist-b".into(),
            name: "B".into(),
        });
        assert!(playlist_playback(&state).is_none());
        state.nav = Nav::PlaylistDetail(link);
        state
            .playlist_detail
            .tracks
            .resolved_mut()
            .unwrap()
            .tracks
            .clear();
        assert!(playlist_playback(&state).is_none());
    }
}

fn async_playlist_info_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        playlist_info_widget,
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::playlist_detail.then(PlaylistDetail::playlist),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_playlist(&d.0.id),
        |_, data, d| data.playlist_detail.playlist.defer(d.0),
        |_, data, (d, r)| data.playlist_detail.playlist.update((d.0, r)),
    )
}

fn playlist_info_widget() -> impl Widget<WithCtx<Playlist>> {
    use druid::widget::CrossAxisAlignment;

    let size = theme::grid(10.0);
    let playlist_cover = cover_widget(size)
        .lens(Ctx::data())
        .clip(Size::new(size, size).to_rounded_rect(4.0))
        .context_menu(playlist_menu_ctx);

    let owner_label = Label::dynamic(|p: &Playlist, _| p.owner.display_name.as_ref().to_string());

    let track_count_label = Label::dynamic(|p: &Playlist, _| {
        let count = p.track_count.unwrap_or(0);
        if count == 1 {
            "1 song".to_string()
        } else {
            format!("{count} songs")
        }
    })
    .with_text_size(theme::TEXT_SIZE_SMALL);

    let description_widget = Either::new(
        |p: &Playlist, _| !p.description.is_empty(),
        Flex::column().with_default_spacer().with_child(
            Label::dynamic(|p: &Playlist, _| p.description.to_string())
                .with_line_break_mode(LineBreaking::Clip)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        ),
        Empty,
    );

    let visibility_widget = Either::new(
        |p: &Playlist, _| p.public.is_some() || p.collaborative,
        Flex::column().with_default_spacer().with_child(
            Label::dynamic(|p: &Playlist, _| {
                let mut parts = Vec::new();
                match p.public {
                    Some(true) => parts.push("Public"),
                    Some(false) => parts.push("Private"),
                    None => {}
                }
                if p.collaborative {
                    parts.push("Collaborative");
                }
                parts.join(" • ")
            })
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR),
        ),
        Empty,
    );

    let playlist_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(owner_label)
        .with_default_spacer()
        .with_child(track_count_label)
        .with_child(description_widget)
        .with_child(visibility_widget);

    Flex::row()
        .with_child(playlist_cover)
        .with_default_spacer()
        .with_flex_child(playlist_info.lens(Ctx::data()), 1.0)
}

fn async_tracks_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, tracks_widget, utils::error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                druid::lens::Map::new(
                    |data: &AppState| {
                        let mut tracks = data.playlist_detail.tracks.clone();
                        if let Some(tracks) = tracks.resolved_mut() {
                            tracks.query = data.playlist_detail.query.clone();
                        }
                        tracks
                    },
                    |_, _| {},
                ),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |arg: (PlaylistLink, AppState)| WebApi::global().get_playlist_tracks(&arg.0.id),
            |_, data, d| {
                if data
                    .playlist_detail
                    .tracks
                    .deferred()
                    .is_none_or(|link| link.id != d.0.id)
                {
                    data.playlist_detail.query.clear();
                }
                data.playlist_detail.tracks.defer(d.0);
            },
            |_, data, (d, r)| {
                let tracks = sort_playlist(data, r).map(|tracks| PlaylistTracks {
                    query: String::new(),
                    id: d.0.id.clone(),
                    name: d.0.name.clone(),
                    tracks,
                });
                data.playlist_detail.tracks.update((d.0, tracks))
            },
        )
}

fn tracks_widget() -> impl Widget<WithCtx<PlaylistTracks>> {
    playable::list_widget_with_find(
        playable::Display {
            track: track::Display {
                title: true,
                artist: true,
                album: true,
                cover: true,
                ..track::Display::empty()
            },
        },
        cmd::FIND_IN_PLAYLIST,
    )
}

fn sort_playlist(
    data: &AppState,
    result: Result<Vector<Arc<Track>>, Error>,
) -> Result<Vector<Arc<Track>>, Error> {
    let sort_criteria = data.config.sort_criteria;
    let sort_order = data.config.sort_order;

    let playlist = result?;

    let sorted_playlist: Vector<Arc<Track>> = playlist
        .into_iter()
        .sorted_by(|a, b| {
            let method = match sort_criteria {
                SortCriteria::Title => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortCriteria::Artist => a
                    .artist_name()
                    .to_lowercase()
                    .cmp(&b.artist_name().to_lowercase()),
                SortCriteria::Album => a
                    .album_name()
                    .to_lowercase()
                    .cmp(&b.album_name().to_lowercase()),
                SortCriteria::Duration => a.duration.cmp(&b.duration),
                SortCriteria::DateAdded => a.playlist_added_at.cmp(&b.playlist_added_at),
            }
            .then_with(|| a.track_pos.cmp(&b.track_pos));

            if sort_order == SortOrder::Descending {
                method.reverse()
            } else {
                method
            }
        })
        .collect();

    Ok(sorted_playlist)
}

fn playlist_menu_ctx(playlist: &WithCtx<Playlist>) -> Menu<AppState> {
    let library = &playlist.ctx.library;
    let playlist = &playlist.data;

    let mut menu = Menu::empty().entry(
        MenuItem::new("Organizar en carpeta...")
            .command(super::folders::ASSIGN_WINDOW.with(playlist.link())),
    );

    menu = menu.entry(
        MenuItem::new("Dividir con Splitify IA")
            .command(crate::splitify::OPEN.with(playlist.id.to_string())),
    );

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Playlist"),
        )
        .command(cmd::COPY.with(playlist.url())),
    );

    if library.contains_playlist(playlist) {
        let created_by_user = library.is_created_by_user(playlist);

        if created_by_user {
            let unfollow_msg = UnfollowPlaylist {
                link: playlist.link(),
                created_by_user,
            };
            menu = menu.entry(
                MenuItem::new(
                    LocalizedString::new("menu-unfollow-playlist")
                        .with_placeholder("Delete playlist"),
                )
                .command(SHOW_UNFOLLOW_PLAYLIST_CONFIRM.with(unfollow_msg)),
            );
            menu = menu.entry(
                MenuItem::new(
                    LocalizedString::new("menu-rename-playlist")
                        .with_placeholder("Rename playlist"),
                )
                .command(SHOW_RENAME_PLAYLIST_CONFIRM.with(playlist.link())),
            );
        } else {
            let unfollow_msg = UnfollowPlaylist {
                link: playlist.link(),
                created_by_user,
            };
            menu = menu.entry(
                MenuItem::new(
                    LocalizedString::new("menu-unfollow-playlist")
                        .with_placeholder("Remove playlist from Your Library"),
                )
                .command(SHOW_UNFOLLOW_PLAYLIST_CONFIRM.with(unfollow_msg)),
            );
        }
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-follow-playlist").with_placeholder("Follow Playlist"),
            )
            .command(FOLLOW_PLAYLIST.with(playlist.clone())),
        );
    }

    menu
}

#[derive(Clone)]
struct UnfollowPlaylist {
    link: PlaylistLink,
    created_by_user: bool,
}
