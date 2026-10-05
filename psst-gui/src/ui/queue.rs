use druid::{
    widget::{Button, CrossAxisAlignment, Either, Flex, Label, LineBreaking, List, Scroll},
    LensExt, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        AppState, Playable, Playback, PlaybackPayload, PlaybackState, QueueBehavior, QueueEntry,
    },
};

use super::{design, theme, utils};

fn artist(item: &Playable) -> String {
    match item {
        Playable::Track(track) => track.artist_name().to_string(),
        Playable::Episode(episode) => episode.show.name.to_string(),
    }
}

pub fn widget() -> impl Widget<AppState> {
    let current = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new("SUENA AHORA")
                .with_text_size(11.0)
                .with_text_color(theme::BLUE_200),
        )
        .with_spacer(12.0)
        .with_child(
            Label::dynamic(|s: &AppState, _| {
                s.playback
                    .now_playing
                    .as_ref()
                    .map(|np| np.item.name().to_string())
                    .unwrap_or_default()
            })
            .with_font(theme::UI_FONT_MEDIUM)
            .with_text_size(22.0)
            .with_line_break_mode(LineBreaking::Clip),
        )
        .with_spacer(6.0)
        .with_child(
            Label::dynamic(|s: &AppState, _| {
                s.playback
                    .now_playing
                    .as_ref()
                    .map(|np| artist(&np.item))
                    .unwrap_or_default()
            })
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_line_break_mode(LineBreaking::Clip),
        )
        .with_spacer(6.0)
        .with_child(
            Label::dynamic(|s: &AppState, _| {
                (match s.playback.state {
                    PlaybackState::Loading => "Cargando…",
                    PlaybackState::Playing => "Reproduciendo",
                    PlaybackState::Paused => "En pausa",
                    PlaybackState::Stopped => "Detenido",
                })
                .to_string()
            })
            .with_text_size(12.0)
            .with_text_color(theme::PLACEHOLDER_COLOR),
        );

    let header = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Either::new(
            |s: &AppState, _| s.playback.now_playing.is_some(),
            design::card(current).expand_width(),
            design::card(
                Label::new("No hay una canción en reproducción.")
                    .with_text_color(theme::PLACEHOLDER_COLOR),
            )
            .expand_width(),
        ))
        .with_spacer(24.0)
        .with_child(
            Flex::row()
                .with_flex_child(
                    Label::dynamic(|s: &AppState, _| {
                        format!("A continuación · {}", s.playback.up_next.len())
                    })
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_text_size(20.0)
                    .expand_width(),
                    1.0,
                )
                .with_child(Either::new(
                    |s: &AppState, _| {
                        s.playback.now_playing.is_none() && !s.playback.up_next.is_empty()
                    },
                    Button::new("Reproducir cola")
                        .on_click(|ctx, s: &mut AppState, _| {
                            if let Some(first) = s.playback.up_next.front() {
                                ctx.submit_command(
                                    cmd::PLAY_TRACKS.with(PlaybackPayload {
                                        origin: first.origin.clone(),
                                        items: s
                                            .playback
                                            .up_next
                                            .iter()
                                            .map(|entry| entry.item.clone())
                                            .collect(),
                                        position: 0,
                                    }),
                                );
                            }
                        })
                        .disabled_if(|s: &AppState, _| {
                            s.playback.up_next.is_empty() || s.playback.now_playing.is_some()
                        }),
                    crate::widget::Empty,
                )),
        )
        .with_spacer(8.0)
        .with_child(
            Label::dynamic(|s: &AppState, _| {
                (match s.playback.queue_behavior {
                    QueueBehavior::Sequential => "Orden de reproducción actual",
                    QueueBehavior::Random => "Orden aleatorio actual",
                    QueueBehavior::LoopTrack => "Se repetirá la canción actual",
                    QueueBehavior::LoopAll => "Repetir cola · se muestra la próxima vuelta",
                })
                .to_string()
            })
            .with_text_size(12.0)
            .with_text_color(theme::PLACEHOLDER_COLOR),
        );

    let list = List::new(|| {
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(
                        Label::dynamic(|entry: &QueueEntry, _| entry.item.name().to_string())
                            .with_font(theme::UI_FONT_MEDIUM)
                            .with_line_break_mode(LineBreaking::Clip),
                    )
                    .with_spacer(5.0)
                    .with_child(
                        Label::dynamic(|entry: &QueueEntry, _| artist(&entry.item))
                            .with_text_color(theme::PLACEHOLDER_COLOR)
                            .with_text_size(12.0)
                            .with_line_break_mode(LineBreaking::Clip),
                    )
                    .expand_width(),
                1.0,
            )
            .with_spacer(16.0)
            .with_child(
                Label::dynamic(|entry: &QueueEntry, _| {
                    utils::as_minutes_and_seconds(entry.item.duration())
                })
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .with_text_size(12.0),
            )
            .padding((16.0, 14.0))
            .expand_width()
            .background(theme::GREY_700)
            .border(theme::GREY_500, 1.0)
            .rounded(8.0)
            .padding((0.0, 0.0, 0.0, 8.0))
    })
    .lens(AppState::playback.then(Playback::up_next));

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header.expand_width())
        .with_spacer(18.0)
        .with_flex_child(Either::new(|s: &AppState, _| s.playback.up_next.is_empty(),
            Label::new("No hay más canciones en la cola.\nAñade canciones desde su menú o reproduce una playlist.")
                .with_line_break_mode(LineBreaking::WordWrap).with_text_color(theme::PLACEHOLDER_COLOR).align_left(),
            Scroll::new(list).vertical()).expand_width(), 1.0)
        .padding(24.0)
}
