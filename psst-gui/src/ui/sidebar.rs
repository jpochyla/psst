use druid::{
    commands,
    widget::{Flex, Scroll},
    Key, Widget, WidgetExt,
};

use super::{playlist, theme};
use crate::{
    cmd,
    data::{AppState, Nav},
    widget::{icons, MyWidgetExt},
};

pub const WIDTH: Key<f64> = Key::new("app.library-width");

pub fn toggle(compact: bool) -> impl Widget<AppState> {
    icons::LIBRARY_RAIL
        .scale((24.0, 24.0))
        .padding(8.0)
        .link()
        .rounded(6.0)
        .tooltip(if compact {
            "Expandir tu biblioteca"
        } else {
            "Minimizar tu biblioteca"
        })
        .on_left_click(|_, _, state: &mut AppState, _| {
            state.config.library_compact = !state.config.library_compact;
            state.config.save();
        })
}

fn destination(
    icon: &'static icons::SvgIcon,
    title: &'static str,
    nav: Nav,
) -> impl Widget<AppState> {
    let highlight = nav.clone();
    icon.scale((20.0, 20.0))
        .padding(8.0)
        .link()
        .rounded(6.0)
        .active(move |state: &AppState, _| {
            if highlight == Nav::Queue {
                state.queue_panel_open
            } else {
                state.nav == highlight
            }
        })
        .tooltip(title)
        .on_left_click(move |ctx, _, _, _| ctx.submit_command(cmd::NAVIGATE.with(nav.clone())))
}

pub fn compact() -> impl Widget<AppState> {
    let actions = Flex::column()
        .with_child(toggle(true))
        .with_spacer(4.0)
        .with_child(
            icons::CIRCLE_PLUS
                .scale((28.0, 28.0))
                .padding(6.0)
                .link()
                .rounded(20.0)
                .tooltip("Organizar una playlist con IA")
                .on_left_click(|ctx, _, _, _| {
                    ctx.submit_command(crate::splitify::OPEN.with(String::new()))
                }),
        )
        .with_spacer(8.0)
        .with_child(destination(
            &icons::MUSIC_NOTE,
            "Canciones guardadas",
            Nav::SavedTracks,
        ))
        .with_child(destination(
            &icons::ALBUM,
            "Álbumes guardados",
            Nav::SavedAlbums,
        ))
        .with_child(destination(&icons::PODCAST, "Podcasts", Nav::Shows))
        .with_child(destination(
            &icons::QUEUE,
            "Cola de reproducción",
            Nav::Queue,
        ))
        .with_spacer(8.0);
    let list = Scroll::new(playlist::compact_list_widget().padding((4.0, 4.0, 12.0, 12.0)))
        .vertical()
        .expand_height();
    Flex::column()
        .with_child(actions.padding((4.0, 8.0)))
        .with_flex_child(list, 1.0)
        .with_child(
            icons::PREFERENCES
                .scale((22.0, 22.0))
                .padding(12.0)
                .link()
                .rounded(6.0)
                .tooltip("Abrir ajustes")
                .on_left_click(|ctx, _, _, _| ctx.submit_command(commands::SHOW_PREFERENCES))
                .center()
                .fix_height(56.0),
        )
        .background(theme::BACKGROUND_DARK)
}
