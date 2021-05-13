use crate::{
    cmd,
    controller::{NavController, PlaybackController, SessionController},
    data::{Nav, State},
    ui::utils::Border,
    widget::{icons, Empty, LinkExt, ThemeScope, ViewDispatcher},
};
use druid::{
    lens::Unit,
    widget::{CrossAxisAlignment, Either, Flex, Label, Scroll, Split, ViewSwitcher},
    Insets, Menu, MenuItem, MouseButton, Widget, WidgetExt, WindowDesc, WindowLevel,
};
use icons::SvgIcon;

pub mod album;
pub mod artist;
pub mod library;
pub mod menu;
pub mod playback;
pub mod playlist;
pub mod preferences;
pub mod search;
pub mod theme;
pub mod track;
pub mod user;
pub mod utils;

pub fn main_window() -> WindowDesc<State> {
    let win = WindowDesc::new(root_widget())
        .title("Psst")
        .with_min_size((theme::grid(25.0), theme::grid(25.0)))
        .window_size((theme::grid(80.0), theme::grid(100.0)))
        .show_title(false)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

pub fn preferences_window() -> WindowDesc<State> {
    let win = WindowDesc::new(preferences_widget())
        .title("Preferences")
        .window_size((theme::grid(50.0), theme::grid(69.0)))
        .resizable(false)
        .show_title(false)
        .transparent_titlebar(true)
        .set_level(WindowLevel::Modal);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

fn preferences_widget() -> impl Widget<State> {
    ThemeScope::new(
        preferences::preferences_widget()
            .background(theme::BACKGROUND_DARK)
            .expand(),
    )
}

fn root_widget() -> impl Widget<State> {
    let playlists = Scroll::new(playlist::list_widget()).vertical();
    let sidebar = Flex::column()
        .must_fill_main_axis(true)
        .with_child(logo_widget())
        .with_child(menu_widget())
        .with_default_spacer()
        .with_flex_child(playlists.expand_height(), 1.0)
        .with_child(user::user_widget())
        .padding(if cfg!(target_os = "macos") {
            Insets::new(0.0, 24.0, 0.0, 0.0)
        } else {
            Insets::ZERO
        })
        .background(theme::BACKGROUND_DARK);

    let topbar = Flex::row()
        .must_fill_main_axis(true)
        .with_child(back_button_widget())
        .with_child(title_widget())
        .background(Border::Bottom.with_color(theme::BACKGROUND_DARK));

    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(topbar)
        .with_flex_child(route_widget(), 1.0)
        .with_child(playback::panel_widget())
        .background(theme::BACKGROUND_LIGHT);

    let split = Split::columns(sidebar, main)
        .split_point(0.2)
        .bar_size(1.0)
        .min_size(150.0, 0.0)
        .min_bar_area(1.0)
        .solid_bar(true);

    let themed = ThemeScope::new(split);

    let controlled = themed
        .controller(PlaybackController::new())
        .controller(SessionController::new())
        .controller(NavController);

    controlled
    // .debug_invalidation()
    // .debug_widget_id()
    // .debug_paint_layout()
}

fn logo_widget() -> impl Widget<State> {
    icons::LOGO
        .scale((29.0, 32.0))
        .with_color(theme::GREY_500)
        .padding((0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
        .center()
        .lens(Unit)
}

fn menu_widget() -> impl Widget<State> {
    Flex::column()
        .with_default_spacer()
        .with_child(menu_link_widget("Home", Nav::Home))
        .with_child(menu_link_widget("Tracks", Nav::SavedTracks))
        .with_child(menu_link_widget("Albums", Nav::SavedAlbums))
        .with_child(menu_search_widget())
}

fn menu_link_widget(title: &str, nav: Nav) -> impl Widget<State> {
    Label::new(title)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .link()
        .env_scope({
            let nav = nav.clone();
            move |env, route: &Nav| {
                env.set(
                    theme::LINK_COLD_COLOR,
                    if &nav == route {
                        env.get(theme::MENU_BUTTON_BG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_BG_INACTIVE)
                    },
                );
                env.set(
                    theme::TEXT_COLOR,
                    if &nav == route {
                        env.get(theme::MENU_BUTTON_FG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_FG_INACTIVE)
                    },
                );
            }
        })
        .on_click(move |ctx, _, _| {
            ctx.submit_command(cmd::NAVIGATE.with(nav.clone()));
        })
        .lens(State::route)
}

fn menu_search_widget() -> impl Widget<State> {
    search::input_widget().padding((theme::grid(1.0), theme::grid(1.0)))
}

fn route_widget() -> impl Widget<State> {
    ViewDispatcher::new(
        |state: &State, _| state.route.clone(),
        |route: &Nav, _, _| match route {
            Nav::Home => home_widget().padding(theme::grid(1.0)).boxed(),
            Nav::SavedTracks => {
                Scroll::new(library::saved_tracks_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::SavedAlbums => {
                Scroll::new(library::saved_albums_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::SearchResults(_) => {
                Scroll::new(search::results_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::AlbumDetail(_) => Scroll::new(album::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Nav::ArtistDetail(_) => Scroll::new(artist::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Nav::PlaylistDetail(_) => {
                Scroll::new(playlist::detail_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
        },
    )
    .expand()
}

fn home_widget() -> impl Widget<State> {
    Empty
}

fn back_button_widget() -> impl Widget<State> {
    let icon = icons::BACK.scale((10.0, theme::grid(2.0)));
    let disabled = icon
        .clone()
        .with_color(theme::GREY_600)
        .padding(theme::grid(1.0));
    let enabled = icon
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_ex_click(|ctx, event, state, _env| match event.button {
            MouseButton::Left => {
                ctx.submit_command(cmd::NAVIGATE_BACK.with(1));
            }
            MouseButton::Right => {
                ctx.show_context_menu(history_menu(state), event.window_pos);
            }
            _ => {}
        });
    Either::new(
        |state: &State, _| state.history.is_empty(),
        disabled,
        enabled,
    )
    .padding(theme::grid(1.0))
}

fn history_menu(state: &State) -> Menu<State> {
    let mut menu = Menu::empty();
    for (index, history) in state.history.iter().rev().take(10).enumerate() {
        let skip_back_in_history_n_times = index + 1;
        menu = menu.entry(
            MenuItem::new(history.to_full_title())
                .command(cmd::NAVIGATE_BACK.with(skip_back_in_history_n_times)),
        );
    }
    menu
}

fn title_widget() -> impl Widget<State> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(route_title_widget())
        .with_spacer(theme::grid(0.5))
        .with_child(route_icon_widget())
        .lens(State::route)
}

fn route_icon_widget() -> impl Widget<Nav> {
    ViewSwitcher::new(
        |route: &Nav, _| route.clone(),
        |route: &Nav, _, _| {
            let icon = |icon: &SvgIcon| icon.scale(theme::ICON_SIZE);
            match &route {
                Nav::Home => Empty.boxed(),
                Nav::SavedTracks => Empty.boxed(),
                Nav::SavedAlbums => Empty.boxed(),
                Nav::SearchResults(_) => icon(&icons::SEARCH).boxed(),
                Nav::AlbumDetail(_) => icon(&icons::ALBUM).boxed(),
                Nav::ArtistDetail(_) => icon(&icons::ARTIST).boxed(),
                Nav::PlaylistDetail(_) => icon(&icons::PLAYLIST).boxed(),
            }
        },
    )
}

fn route_title_widget() -> impl Widget<Nav> {
    Label::dynamic(|route: &Nav, _| route.to_title())
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_size(theme::TEXT_SIZE_LARGE)
}
