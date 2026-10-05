use super::{theme, utils};
use crate::{
    cmd,
    data::{AppState, Nav, PlaybackState},
    widget::{MyWidgetExt, RemoteImage},
};
use druid::{
    widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking, Scroll},
    Widget, WidgetExt,
};

pub fn widget() -> impl Widget<AppState> {
    let cover = RemoteImage::new(utils::placeholder_widget(), |state: &AppState, _| {
        state
            .playback
            .now_playing
            .as_ref()
            .and_then(|np| np.cover_image_url(180.0, 180.0))
            .map(|url| url.into())
    })
    .fix_size(180.0, 180.0)
    .center()
    .padding((0.0, 16.0))
    .context_menu(super::queue::current_menu);
    let upcoming = super::queue::preview_widget();
    Scroll::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Flex::row()
                    .with_flex_child(
                        Label::new("Reproducción actual").with_font(theme::UI_FONT_MEDIUM),
                        1.0,
                    )
                    .with_child(
                        Button::new("×")
                            .on_click(|_, state: &mut AppState, _| {
                                state.config.show_now_playing = false
                            })
                            .tooltip("Ocultar panel de reproducción"),
                    ),
            )
            .with_child(cover)
            .with_child(
                Label::dynamic(|state: &AppState, _| {
                    state
                        .playback
                        .now_playing
                        .as_ref()
                        .map(|np| np.item.name().to_string())
                        .unwrap_or_else(|| "Elige una canción".into())
                })
                .with_text_size(20.0)
                .with_font(theme::UI_FONT_MEDIUM)
                .with_line_break_mode(LineBreaking::WordWrap)
                .context_menu(super::queue::current_menu),
            )
            .with_spacer(8.0)
            .with_child(
                Label::dynamic(|state: &AppState, _| {
                    state
                        .playback
                        .now_playing
                        .as_ref()
                        .map(|np| match &np.item {
                            crate::data::Playable::Track(track) => track.artist_name().to_string(),
                            crate::data::Playable::Episode(episode) => {
                                episode.show.name.to_string()
                            }
                        })
                        .unwrap_or_default()
                })
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .with_line_break_mode(LineBreaking::WordWrap),
            )
            .with_spacer(8.0)
            .with_child(
                Label::dynamic(|state: &AppState, _| {
                    match state.playback.state {
                        PlaybackState::Paused => "En pausa",
                        PlaybackState::Playing => "Reproduciendo",
                        PlaybackState::Loading => "Cargando…",
                        PlaybackState::Stopped => "Sin reproducción",
                    }
                    .to_owned()
                })
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .with_text_size(12.0),
            )
            .with_spacer(14.0)
            .with_child(
                Button::new("Letras")
                    .on_click(|ctx, _, _| ctx.submit_command(cmd::TOGGLE_LYRICS))
                    .tooltip("Ver las letras de esta canción"),
            )
            .with_spacer(8.0)
            .with_child(
                Button::new("Cola completa")
                    .on_click(|ctx, _, _| ctx.submit_command(cmd::NAVIGATE.with(Nav::Queue)))
                    .tooltip("Ver el orden de las próximas canciones"),
            )
            .with_spacer(8.0)
            .with_child(
                Button::new("Videoclip")
                    .on_click(|ctx, state: &mut AppState, _| {
                        if let Some(track) = state
                            .playback
                            .now_playing
                            .as_ref()
                            .and_then(|np| np.item.track())
                        {
                            ctx.submit_command(cmd::OPEN_MUSIC_VIDEO.with(track.clone()));
                        }
                    })
                    .tooltip("Buscar el videoclip en YouTube")
                    .disabled_if(|state, _| {
                        state
                            .playback
                            .now_playing
                            .as_ref()
                            .and_then(|np| np.item.track())
                            .is_none()
                    }),
            )
            .with_spacer(24.0)
            .with_child(Label::new("A continuación").with_font(theme::UI_FONT_MEDIUM))
            .with_child(upcoming)
            .padding(14.0)
            .expand_width(),
    )
    .vertical()
    .background(theme::BACKGROUND_DARK)
}
