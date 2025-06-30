use std::{cell::RefCell, cmp::Ordering, rc::Rc, sync::Arc};

use druid::{
    im::Vector,
    widget::{Button, Either, Flex, Label, LensWrap, LineBreaking, List, TextBox},
    Insets, Lens, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget,
    WidgetExt, WindowDesc,
};
use itertools::Itertools;

use crate::{
    cmd,
    data::{
        config::{SortCriteria, SortOrder},
        AppState, Ctx, Library, Nav, Playlist, PlaylistAddTrack, PlaylistDetail, PlaylistLink,
        PlaylistRemoveTrack, PlaylistTracks, Track, WithCtx,
    },
    error::Error,
    ui::menu,
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage, ThemeScope},
};

use super::{playable, theme, track, utils};

pub const LOAD_LIST: Selector = Selector::new("app.playlist.load-list");
pub const LOAD_DETAIL: Selector<(PlaylistLink, AppState)> =
    Selector::new("app.playlist.load-detail");
pub const ADD_TRACK: Selector<PlaylistAddTrack> = Selector::new("app.playlist.add-track");
pub const REMOVE_TRACK: Selector<PlaylistRemoveTrack> = Selector::new("app.playlist.remove-track");

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
    Async::new(
        utils::spinner_widget,
        || {
            List::new(|| {
                Label::raw()
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(theme::TEXT_SIZE_SMALL)
                    .lens(Ctx::data().then(Playlist::name))
                    .expand_width()
                    .padding(Insets::uniform_xy(theme::grid(2.0), theme::grid(0.6)))
                    .link()
                    .on_left_click(|ctx, _, playlist, _| {
                        ctx.submit_command(
                            cmd::NAVIGATE.with(Nav::PlaylistDetail(playlist.data.link())),
                        );
                    })
                    .context_menu(playlist_menu_ctx)
            })
        },
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::library.then(Library::playlists.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
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
        |_, data, d| {
            data.with_library_mut(|library| library.increment_playlist_track_count(&d.link))
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Added to playlist.");
            }
        },
    )
    .on_command_async(
        UNFOLLOW_PLAYLIST,
        |link| WebApi::global().unfollow_playlist(link.id.as_ref()),
        |_, data: &mut AppState, d| data.with_library_mut(|l| l.remove_from_playlist(&d.id)),
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Playlist removed from library.");
            }
        },
    )
    .on_command_async(
        FOLLOW_PLAYLIST,
        |link| WebApi::global().follow_playlist(link.id.as_ref()),
        |_, data: &mut AppState, d| data.with_library_mut(|l| l.add_playlist(d)),
        |_, data: &mut AppState, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Playlist added to library.")
            }
        },
    )
    .on_command_async(
        RENAME_PLAYLIST,
        |link| WebApi::global().change_playlist_details(link.id.as_ref(), link.name.as_ref()),
        |_, data: &mut AppState, link| data.with_library_mut(|l| l.rename_playlist(link)),
        |_, data: &mut AppState, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Playlist renamed.")
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
        |d| WebApi::global().remove_track_from_playlist(&d.link.id, d.track_pos),
        |_, data, d| {
            data.with_library_mut(|library| library.decrement_playlist_track_count(&d.link))
        },
        |e, data, (p, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Removed from playlist.");
            }
            // Re-submit the `LOAD_DETAIL` command to reload the playlist data.
            e.submit_command(LOAD_DETAIL.with((p.link, data.clone())))
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
                    .fix_size(theme::grid(16.0), theme::grid(8.0)),
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

fn async_playlist_info_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, playlist_info_widget, || Empty)
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
            format!("{} songs", count)
        }
    })
    .with_text_size(theme::TEXT_SIZE_SMALL);

    let description_widget = Either::new(
        |p: &Playlist, _| !p.description.is_empty(),
        Flex::column().with_default_spacer().with_child(
            Label::dynamic(|p: &Playlist, _| p.description.to_string())
                .with_line_break_mode(LineBreaking::WordWrap)
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
        .with_child(visibility_widget)
        .padding(theme::grid(1.0));

    Flex::row()
        .with_child(playlist_cover)
        .with_default_spacer()
        .with_child(playlist_info.lens(Ctx::data()))
}

fn async_tracks_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, tracks_widget, utils::error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::playlist_detail.then(PlaylistDetail::tracks),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |arg: (PlaylistLink, AppState)| {
                let d = arg.0;
                let data = arg.1;
                sort_playlist(&data, WebApi::global().get_playlist_tracks(&d.id))
            },
            |_, data, d| data.playlist_detail.tracks.defer(d.0),
            |_, data, (d, r)| {
                let tracks = PlaylistTracks {
                    id: d.0.id.clone(),
                    name: d.0.name.clone(),
                    tracks: r,
                };
                data.playlist_detail.tracks.update((d.0, Ok(tracks)))
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

fn sort_playlist(data: &AppState, result: Result<Vector<Arc<Track>>, Error>) -> Vector<Arc<Track>> {
    let sort_criteria = data.config.sort_criteria;
    let sort_order = data.config.sort_order;

    let playlist = result.unwrap_or_else(|_| Vector::new());

    let sorted_playlist: Vector<Arc<Track>> = playlist
        .into_iter()
        .sorted_by(|a, b| {
            let method = match sort_criteria {
                SortCriteria::Title => a.name.cmp(&b.name),
                SortCriteria::Artist => a.artist_name().cmp(&b.artist_name()),
                SortCriteria::Album => a.album_name().cmp(&b.album_name()),
                SortCriteria::Duration => a.duration.cmp(&b.duration),
                SortCriteria::DateAdded => Ordering::Equal,
            };

            if sort_order == SortOrder::Descending {
                method.reverse()
            } else {
                method
            }
        })
        .collect();

    if sort_criteria == SortCriteria::DateAdded && sort_order == SortOrder::Descending {
        sorted_playlist.into_iter().rev().collect()
    } else {
        sorted_playlist
    }
}

fn playlist_menu_ctx(playlist: &WithCtx<Playlist>) -> Menu<AppState> {
    let library = &playlist.ctx.library;
    let playlist = &playlist.data;

    let mut menu = Menu::empty();

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
