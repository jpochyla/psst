use crate::{
    cmd,
    controller::{NavController, PlaybackController, SessionController},
    data::{Nav, State},
    ui::utils::Border,
    widget::{icons, Empty, HoverExt, ThemeScope, ViewDispatcher},
};
use druid::{
    commands,
    widget::{CrossAxisAlignment, Either, Flex, Label, Scroll, Split, ViewSwitcher},
    Widget, WidgetExt, WindowDesc, WindowLevel,
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
pub mod utils;

pub fn main_window() -> WindowDesc<State> {
    let mut win = WindowDesc::new(root_widget()).title("Psst");
    win = win
        .with_min_size((theme::grid(25.0), theme::grid(25.0)))
        .window_size((theme::grid(100.0), theme::grid(100.0)));
    if cfg!(target_os = "macos") {
        win = win.menu(menu::main_menu());
    }
    win
}

pub fn preferences_window() -> WindowDesc<State> {
    let mut win = WindowDesc::new(preferences_widget()).title("Preferences");
    win = win
        .set_level(WindowLevel::Modal)
        .window_size((theme::grid(50.0), theme::grid(69.0)))
        .resizable(false);
    if cfg!(target_os = "macos") {
        win = win.menu(menu::main_menu());
    }
    win
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
        .with_flex_child(playlists, 1.0)
        .background(theme::BACKGROUND_DARK);

    let topbar = Flex::row()
        .must_fill_main_axis(true)
        .with_child(back_button_widget())
        .with_default_spacer()
        .with_child(title_widget())
        .with_flex_child(
            Flex::row()
                .with_child(is_online_widget())
                .with_default_spacer()
                .with_child(preferences_button_widget())
                .align_right(),
            1.0,
        )
        .background(Border::Bottom.widget(theme::BACKGROUND_DARK));

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
}

fn menu_widget() -> impl Widget<State> {
    Flex::column()
        .with_default_spacer()
        .with_child(menu_item_widget("Home", Nav::Home))
        .with_child(menu_item_widget("Library", Nav::Library))
        .with_child(menu_search_widget())
}

fn menu_item_widget(title: &str, nav: Nav) -> impl Widget<State> {
    Label::new(title)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .hover()
        .env_scope({
            let nav = nav.clone();
            move |env, state: &State| {
                env.set(
                    theme::HOVER_COLD_COLOR,
                    if nav == state.route {
                        env.get(theme::MENU_BUTTON_BG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_BG_INACTIVE)
                    },
                );
                env.set(
                    theme::LABEL_COLOR,
                    if nav == state.route {
                        env.get(theme::MENU_BUTTON_FG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_FG_INACTIVE)
                    },
                );
            }
        })
        .on_click(move |ctx, _, _| {
            ctx.submit_command(cmd::NAVIGATE_TO.with(nav.clone()));
        })
}

fn menu_search_widget() -> impl Widget<State> {
    search::input_widget().padding((theme::grid(1.0), theme::grid(1.0)))
}

fn route_widget() -> impl Widget<State> {
    ViewDispatcher::new(
        |state: &State, _| state.route.clone(),
        |route: &Nav, _, _| match route {
            Nav::Home => home_widget().padding(theme::grid(1.0)).boxed(),
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
            Nav::Library => Scroll::new(library::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
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
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _state, _env| {
            ctx.submit_command(cmd::NAVIGATE_BACK);
        });
    Either::new(
        |state: &State, _| state.history.is_empty(),
        disabled,
        enabled,
    )
    .padding(theme::grid(1.0))
}

fn title_widget() -> impl Widget<State> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(route_title_widget())
        .with_spacer(theme::grid(0.5))
        .with_child(route_icon_widget())
}

fn route_icon_widget() -> impl Widget<State> {
    ViewSwitcher::new(
        |state: &State, _| state.route.clone(),
        |route: &Nav, _, _| {
            let icon = |icon: &SvgIcon| icon.scale(theme::ICON_SIZE);
            match &route {
                Nav::Home => Empty.boxed(),
                Nav::Library => Empty.boxed(),
                Nav::SearchResults(_) => icon(&icons::SEARCH).boxed(),
                Nav::AlbumDetail(_) => icon(&icons::ALBUM).boxed(),
                Nav::ArtistDetail(_) => icon(&icons::ARTIST).boxed(),
                Nav::PlaylistDetail(_) => icon(&icons::PLAYLIST).boxed(),
            }
        },
    )
}

fn route_title_widget() -> impl Widget<State> {
    Label::dynamic(|state: &State, _| match &state.route {
        Nav::Home => "Home".to_string(),
        Nav::Library => "Library".to_string(),
        Nav::SearchResults(query) => query.clone(),
        Nav::AlbumDetail(link) => link.name.to_string(),
        Nav::ArtistDetail(link) => link.name.to_string(),
        Nav::PlaylistDetail(link) => link.name.to_string(),
    })
    .with_font(theme::UI_FONT_MEDIUM)
}

fn preferences_button_widget() -> impl Widget<State> {
    icons::PREFERENCES
        .scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::GREY_400)
        .padding(theme::grid(1.0))
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _state, _env| {
            ctx.submit_command(commands::SHOW_PREFERENCES);
        })
        .padding(theme::grid(1.0))
}

fn is_online_widget() -> impl Widget<State> {
    Either::new(
        // TODO: Avoid the locking here.
        |state: &State, _| state.session.is_connected(),
        Empty,
        Label::new("Offline"),
    )
    .padding(theme::grid(1.0))
}
