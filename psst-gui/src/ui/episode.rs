use std::sync::Arc;

use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking},
    LensExt, LocalizedString, Menu, MenuItem, Size, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Episode, Library, Nav},
    widget::{FadeOut, MyWidgetExt, RemoteImage},
};

use super::{
    playable::{self, PlayRow},
    theme, utils,
};

pub fn playable_widget() -> impl Widget<PlayRow<Arc<Episode>>> {
    let cover = rounded_cover_widget(theme::grid(4.0)).lens(PlayRow::item);

    let name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(PlayRow::item.then(Episode::name.in_arc()));

    let description = Label::raw()
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(PlayRow::item.then(Episode::description.in_arc()));

    let description =
        FadeOut::bottom(description, theme::grid(3.5)).with_color(theme::BACKGROUND_LIGHT);

    let release = Label::<Arc<Episode>>::dynamic(|episode, _| episode.release())
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::GREY_300)
        .lens(PlayRow::item);

    let is_playing = playable::is_playing_marker_widget().lens(PlayRow::is_playing);

    let duration = Label::<Arc<Episode>>::dynamic(|episode, _| utils::as_human(episode.duration))
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(PlayRow::item);

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(release)
                .with_default_spacer()
                .with_flex_child(is_playing, 1.0)
                .with_default_spacer()
                .with_child(duration),
        )
        .with_default_spacer()
        .with_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(cover)
                .with_default_spacer()
                .with_flex_child(
                    Flex::column()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .with_child(name)
                        .with_default_spacer()
                        .with_child(description),
                    1.0,
                ),
        )
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, row, _| ctx.submit_notification(cmd::PLAY.with(row.position)))
        .context_menu(episode_row_menu)
}

fn cover_widget(size: f64) -> impl Widget<Arc<Episode>> {
    RemoteImage::new(
        utils::placeholder_widget(),
        move |episode: &Arc<Episode>, _| episode.image(size, size).map(|image| image.url.clone()),
    )
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Arc<Episode>> {
    // TODO: Take the radius from theme.
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

fn episode_row_menu(row: &PlayRow<Arc<Episode>>) -> Menu<AppState> {
    episode_menu(&row.item, &row.ctx.library)
}

pub fn episode_menu(episode: &Episode, _library: &Arc<Library>) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(LocalizedString::new("menu-item-show-show").with_placeholder("Go To Show"))
            .command(cmd::NAVIGATE.with(Nav::ShowDetail(episode.show.clone()))),
    );

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Episode"),
        )
        .command(cmd::COPY.with(episode.url())),
    );

    menu
}
