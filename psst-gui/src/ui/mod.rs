use crate::{
    cmd,
    data::{Navigation, Promise, Route, State},
    widget::{button::HOVER_COLD_COLOR, icons, HoverExt, Icon, ViewDispatcher},
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, Scroll, SizedBox, Split, ViewSwitcher},
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
            icons::HOME.scale((theme::grid(2.0), theme::grid(2.0))),
            Navigation::Home,
        ))
        .with_child(make_nav_button(
            "Library",
            icons::LIBRARY.scale((theme::grid(2.0), theme::grid(2.0))),
            Navigation::Library,
        ))
        .with_child(make_nav_search())
}

fn make_nav_button(title: &str, icon: Icon, nav: Navigation) -> impl Widget<State> {
    let label = Label::new(title);

    Flex::row()
        .with_child(icon)
        .with_default_spacer()
        .with_child(label)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .hover()
        .env_scope({
            let nav = nav.clone();
            move |env, state: &State| {
                if nav.as_route() == state.route {
                    env.set(HOVER_COLD_COLOR, theme::MENU_BUTTON_BG_ACTIVE);
                    env.set(theme::LABEL_COLOR, theme::MENU_BUTTON_FG_ACTIVE);
                    env.set(icons::ICON_COLOR, theme::MENU_BUTTON_ICON_ACTIVE);
                } else {
                    env.set(HOVER_COLD_COLOR, theme::MENU_BUTTON_BG_INACTIVE);
                    env.set(theme::LABEL_COLOR, theme::MENU_BUTTON_FG_INACTIVE);
                    env.set(icons::ICON_COLOR, theme::MENU_BUTTON_ICON_INACTIVE);
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
    SizedBox::empty()
}

pub fn make_back_button() -> impl Widget<State> {
    ViewSwitcher::new(
        |state: &State, _| state.history.is_empty(),
        |&no_nav_history, _, _| {
            if no_nav_history {
                SizedBox::empty()
                    .width(10.0 + theme::grid(1.0))
                    .height(theme::grid(2.0) + theme::grid(1.0))
                    .boxed()
            } else {
                icons::BACK
                    .scale((10.0, theme::grid(2.0)))
                    .padding(theme::grid(1.0))
                    .hover()
                    .rounded(theme::BUTTON_BORDER_RADIUS)
                    .on_click(|ctx, _state, _env| {
                        ctx.submit_command(cmd::NAVIGATE_BACK);
                    })
                    .padding(theme::grid(1.0))
                    .boxed()
            }
        },
    )
}

pub fn make_title() -> impl Widget<State> {
    Label::dynamic(|state: &State, _| get_route_title(state)).with_font(theme::UI_FONT_MEDIUM)
}

fn get_route_title(state: &State) -> String {
    match state.route {
        Route::Home => "".to_string(),
        Route::Library => "Library".to_string(),
        Route::SearchResults => format!("Search: \"{}\"", state.search.input),
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
