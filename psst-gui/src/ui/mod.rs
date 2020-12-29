use crate::{
    cmd,
    data::{Navigation, State},
    widget::{icons, Empty, HoverExt, ViewDispatcher},
};
use druid::{
    widget::{CrossAxisAlignment, Either, Flex, Label, Scroll, SizedBox, Split, ViewSwitcher},
    Widget, WidgetExt, WindowDesc,
};
use icons::SvgIcon;

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
    // .debug_invalidation()
    // .debug_widget_id()
    // .debug_paint_layout()
}

pub fn make_nav() -> impl Widget<State> {
    Flex::column()
        .with_default_spacer()
        .with_child(make_nav_button("Home", Navigation::Home))
        .with_child(make_nav_button("Library", Navigation::Library))
        .with_child(make_nav_search())
}

fn make_nav_button(title: &str, nav: Navigation) -> impl Widget<State> {
    Label::new(title)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .hover()
        .env_scope({
            let nav = nav.clone();
            move |env, state: &State| {
                if nav == state.route {
                    env.set(theme::HOVER_COLD_COLOR, theme::MENU_BUTTON_BG_ACTIVE);
                    env.set(theme::LABEL_COLOR, theme::MENU_BUTTON_FG_ACTIVE);
                } else {
                    env.set(theme::HOVER_COLD_COLOR, theme::MENU_BUTTON_BG_INACTIVE);
                    env.set(theme::LABEL_COLOR, theme::MENU_BUTTON_FG_INACTIVE);
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
        |route: &Navigation, _, _| match route {
            Navigation::Home => make_home().boxed(),
            Navigation::SearchResults(_) => search::make_results().boxed(),
            Navigation::AlbumDetail(_) => album::make_detail().boxed(),
            Navigation::ArtistDetail(_) => artist::make_detail().boxed(),
            Navigation::PlaylistDetail(_) => playlist::make_detail().boxed(),
            Navigation::Library => library::make_detail().boxed(),
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
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(make_route_title())
        .with_spacer(theme::grid(0.5))
        .with_child(make_route_icon())
}

fn make_route_icon() -> impl Widget<State> {
    ViewSwitcher::new(
        |state: &State, _| state.route.clone(),
        |route: &Navigation, _, _| {
            let icon = |icon: &SvgIcon| icon.scale(theme::ICON_SIZE);
            match &route {
                Navigation::Home => Empty.boxed(),
                Navigation::Library => Empty.boxed(),
                Navigation::SearchResults(_) => icon(&icons::SEARCH).boxed(),
                Navigation::AlbumDetail(_) => icon(&icons::ALBUM).boxed(),
                Navigation::ArtistDetail(_) => icon(&icons::ARTIST).boxed(),
                Navigation::PlaylistDetail(_) => icon(&icons::PLAYLIST).boxed(),
            }
        },
    )
}

fn make_route_title() -> impl Widget<State> {
    Label::dynamic(|state: &State, _| match &state.route {
        Navigation::Home => "".to_string(),
        Navigation::Library => "Library".to_string(),
        Navigation::SearchResults(query) => query.clone(),
        Navigation::AlbumDetail(link) => link.name.to_string(),
        Navigation::ArtistDetail(link) => link.name.to_string(),
        Navigation::PlaylistDetail(link) => link.name.to_string(),
    })
    .with_font(theme::UI_FONT_MEDIUM)
}
