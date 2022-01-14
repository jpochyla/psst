use std::sync::Arc;

use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking},
    LensExt, LocalizedString, Menu, MenuItem, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Episode, Library},
    widget::{MyWidgetExt, RemoteImage},
};

use super::{playable::PlayRow, theme, utils};

pub fn playable_widget() -> impl Widget<PlayRow<Arc<Episode>>> {
    let cover = episode_cover_widget(theme::grid(4.0));

    let name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(Episode::name.in_arc());

    let description = Label::raw()
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(Episode::description.in_arc());

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
        )
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .lens(PlayRow::item)
        .on_click(|ctx, row, _| ctx.submit_notification(cmd::PLAY.with(row.position)))
        .context_menu(episode_row_menu)
}

fn episode_cover_widget(size: f64) -> impl Widget<Arc<Episode>> {
    RemoteImage::new(
        utils::placeholder_widget(),
        move |episode: &Arc<Episode>, _| episode.image(size, size).map(|image| image.url.clone()),
    )
    .fix_size(size, size)
}

fn episode_row_menu(row: &PlayRow<Arc<Episode>>) -> Menu<AppState> {
    episode_menu(&row.item, &row.ctx.library)
}

pub fn episode_menu(episode: &Episode, _library: &Arc<Library>) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Episode"),
        )
        .command(cmd::COPY.with(episode.url())),
    );

    menu
}
