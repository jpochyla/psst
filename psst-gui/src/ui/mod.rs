use crate::{
    cmd,
    data::{Navigation, Promise, Route, State},
    widget::{icons, Empty, HoverExt, Icon, ViewDispatcher},
};
use druid::{
    widget::{CrossAxisAlignment, Either, Flex, Label, Scroll, SizedBox, Split},
    Widget, WidgetExt, WindowDesc,
};

pub mod album;
pub mod artist;
pub mod config;
pub mod library;
pub mod menu;
pub mod playback;
pub mod playlist;
pub mod search;
pub mod theme;
pub mod track;
pub mod utils;

pub fn make_main_window() -> WindowDesc<State> {
    WindowDesc::new(make_root)
        .title("Psst")
        .menu(menu::make_menu())
        .with_min_size((theme::grid(25.0), theme::grid(25.0)))
        .window_size((theme::grid(125.0), theme::grid(100.0)))
}

pub fn make_config_window() -> WindowDesc<State> {
    WindowDesc::new(make_config)
        .title("Preferences")
        .menu(menu::make_menu())
        .window_size((theme::grid(45.0), theme::grid(50.0)))
        .resizable(false)
}

fn make_config() -> impl Widget<State> {
    config::make_config()
        .center()
        .background(theme::BACKGROUND_DARK)
        .expand()
}

pub fn make_root() -> impl Widget<State> {
    let playlists = Scroll::new(playlist::make_list()).vertical();
    let sidebar = Flex::column()
        .must_fill_main_axis(true)
        .with_child(make_nav())
        .with_default_spacer()
        .with_flex_child(playlists, 1.0)
        .background(theme::BACKGROUND_DARK);

    let topbar = Flex::row()
        .with_child(make_back_button())
        .with_default_spacer()
        .with_child(make_title());

    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(topbar)
        .with_flex_child(make_route(), 1.0)
        .with_child(playback::make_panel());

    Split::columns(sidebar, main)
        .split_point(0.2)
        .bar_size(1.0)
        .min_size(150.0, 0.0)
        .min_bar_area(1.0)
        .solid_bar(true)
}

pub fn make_nav() -> impl Widget<State> {
    Flex::column()
        .with_default_spacer()
        .with_child(make_nav_button(
            "Home",
            icons::TIME.scale(theme::MENU_BUTTON_ICON_SIZE),
            Navigation::Home,
        ))
        .with_child(make_nav_button(
            "Library",
            icons::HEART.scale(theme::MENU_BUTTON_ICON_SIZE),
            Navigation::Library,
        ))
        .with_child(make_nav_search())
}

fn make_nav_button(title: &str, icon: Icon, nav: Navigation) -> impl Widget<State> {
    let label = Label::new(title);

    Flex::row()
        .with_child(icon)
        .with_spacer(theme::grid(0.5))
        .with_child(label)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .hover()
        .env_scope({
            let nav = nav.clone();
            move |env, state: &State| {
                if nav.as_route() == state.route {
                    env.set(theme::HOVER_COLD_COLOR, theme::MENU_BUTTON_BG_ACTIVE);
                    env.set(theme::LABEL_COLOR, theme::MENU_BUTTON_FG_ACTIVE);
                    env.set(theme::ICON_COLOR, theme::MENU_BUTTON_ICON_ACTIVE);
                } else {
                    env.set(theme::HOVER_COLD_COLOR, theme::MENU_BUTTON_BG_INACTIVE);
                    env.set(theme::LABEL_COLOR, theme::MENU_BUTTON_FG_INACTIVE);
                    env.set(theme::ICON_COLOR, theme::MENU_BUTTON_ICON_INACTIVE);
                };
            }
        })
        .on_click(move |ctx, _, _| ctx.submit_command(cmd::NAVIGATE_TO.with(nav.clone())))
}

fn make_nav_search() -> impl Widget<State> {
    search::make_input().padding((theme::grid(1.0), theme::grid(1.0)))
}

pub fn make_route() -> impl Widget<State> {
    let switcher = ViewDispatcher::new(
        |state: &State, _| state.route.clone(),
        |route: &Route, _, _| match route {
            Route::Home => make_home().boxed(),
            Route::SearchResults => search::make_results().boxed(),
            Route::AlbumDetail => album::make_detail().boxed(),
            Route::ArtistDetail => artist::make_detail().boxed(),
            Route::PlaylistDetail => playlist::make_detail().boxed(),
            Route::Library => library::make_detail().boxed(),
        },
    )
    .padding(theme::grid(1.0));

    Scroll::new(switcher).vertical().expand()
}

pub fn make_home() -> impl Widget<State> {
    Empty
}

pub fn make_back_button() -> impl Widget<State> {
    let icon_width = 10.0;
    let icon_height = theme::grid(2.0);
    let empty_icon = SizedBox::empty().width(icon_width).height(icon_height);
    let back_icon = icons::BACK
        .scale((icon_width, icon_height))
        .padding(theme::grid(1.0))
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _state, _env| {
            ctx.submit_command(cmd::NAVIGATE_BACK);
        });
    Either::new(
        |state: &State, _| state.history.is_empty(),
        empty_icon,
        back_icon,
    )
    .padding(theme::grid(1.0))
}

pub fn make_title() -> impl Widget<State> {
    let category = Label::dynamic(|state: &State, _| get_route_category(state))
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL);
    let title =
        Label::dynamic(|state: &State, _| get_route_title(state)).with_font(theme::UI_FONT_MEDIUM);
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Baseline)
        .with_child(category)
        .with_child(title)
}

fn get_route_category(state: &State) -> String {
    match state.route {
        Route::Home => "".to_string(),
        Route::Library => "".to_string(),
        Route::SearchResults => "Search ".to_string(),
        Route::AlbumDetail => "Album ".to_string(),
        Route::ArtistDetail => "Artist ".to_string(),
        Route::PlaylistDetail => "Playlist ".to_string(),
    }
}

fn get_route_title(state: &State) -> String {
    match state.route {
        Route::Home => "".to_string(),
        Route::Library => "Library".to_string(),
        Route::SearchResults => state.search.input.clone(),
        Route::AlbumDetail => match &state.album.album {
            Promise::Empty | Promise::Deferred(_) => "...".to_string(),
            Promise::Resolved(album) => album.name.to_string(),
            Promise::Rejected(err) => err.to_string(),
        },
        Route::ArtistDetail => match &state.artist.artist {
            Promise::Empty | Promise::Deferred(_) => "...".to_string(),
            Promise::Resolved(artist) => artist.name.to_string(),
            Promise::Rejected(err) => err.to_string(),
        },
        Route::PlaylistDetail => match &state.playlist.playlist {
            Promise::Empty | Promise::Deferred(_) => "...".to_string(),
            Promise::Resolved(playlist) => playlist.name.to_string(),
            Promise::Rejected(err) => err.to_string(),
        },
    }
}
