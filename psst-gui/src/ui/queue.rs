use druid::{
    im::Vector,
    lens::Map,
    widget::{
        Button, CrossAxisAlignment, Either, Flex, Label, LineBreaking, List, Painter, Scroll,
    },
    RenderContext, Widget, WidgetExt,
};

use crate::{
    data::{AppState, Playable, PlaybackOrigin, QueueEntry},
    widget::{Border, MyWidgetExt, RemoteImage},
};

use super::{theme, utils};

fn artist(item: &Playable) -> String {
    match item {
        Playable::Track(track) => track.artist_name().to_string(),
        Playable::Episode(episode) => episode.show.name.to_string(),
    }
}

fn row(current: bool) -> impl Widget<QueueEntry> {
    let cover = RemoteImage::new(
        utils::placeholder_widget(),
        |entry: &QueueEntry, _| match &entry.item {
            Playable::Track(track) => track
                .album
                .as_ref()
                .or(match &entry.origin {
                    PlaybackOrigin::Album(album) => Some(album),
                    _ => None,
                })
                .and_then(|album| album.image(48.0, 48.0))
                .map(|image| image.url.clone()),
            Playable::Episode(episode) => episode.image(48.0, 48.0).map(|image| image.url.clone()),
        },
    )
    .fix_size(48.0, 48.0);
    Flex::row()
        .with_child(cover)
        .with_spacer(12.0)
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Label::dynamic(|entry: &QueueEntry, _| entry.item.name().to_string())
                        .with_font(theme::UI_FONT_MEDIUM)
                        .with_text_color(if current {
                            theme::BLUE_200
                        } else {
                            theme::TEXT_COLOR
                        })
                        .with_text_size(14.0)
                        .with_line_break_mode(LineBreaking::Clip),
                )
                .with_spacer(5.0)
                .with_child(
                    Label::dynamic(|entry: &QueueEntry, _| artist(&entry.item))
                        .with_text_size(12.0)
                        .with_text_color(theme::PLACEHOLDER_COLOR)
                        .with_line_break_mode(LineBreaking::Clip),
                )
                .expand_width(),
            1.0,
        )
        .padding((8.0, 8.0))
        .expand_width()
        .background(Painter::new(|ctx, _: &QueueEntry, env| {
            if ctx.is_hot() {
                let rect = ctx.size().to_rect().to_rounded_rect(6.0);
                ctx.fill(rect, &env.get(theme::GREY_700));
            }
        }))
}

pub fn widget() -> impl Widget<AppState> {
    let current = List::new(|| row(true)).lens(Map::new(
        |state: &AppState| {
            state
                .playback
                .now_playing
                .as_ref()
                .map(|np| {
                    Vector::unit(QueueEntry {
                        item: np.item.clone(),
                        origin: np.origin.clone(),
                    })
                })
                .unwrap_or_default()
        },
        |_, _| {},
    ));
    let upcoming = List::new(|| row(false)).lens(Map::new(
        |state: &AppState| {
            state
                .playback
                .up_next
                .iter()
                .take(state.queue_visible_count)
                .cloned()
                .collect::<Vector<_>>()
        },
        |_, _| {},
    ));
    let contents = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new("Suena ahora")
                .with_font(theme::UI_FONT_MEDIUM)
                .padding((8.0, 12.0)),
        )
        .with_child(Either::new(
            |state: &AppState, _| state.playback.now_playing.is_some(),
            current,
            Label::new("Elige una canción para empezar")
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .padding(8.0),
        ))
        .with_spacer(20.0)
        .with_child(
            Label::dynamic(|state: &AppState, _| {
                state
                    .playback
                    .now_playing
                    .as_ref()
                    .map(|np| match &np.origin {
                        PlaybackOrigin::Playlist(link) => format!("A continuación: {}", link.name),
                        PlaybackOrigin::Album(link) => format!("A continuación: {}", link.name),
                        _ => "A continuación".to_owned(),
                    })
                    .unwrap_or_else(|| "A continuación".to_owned())
            })
            .with_font(theme::UI_FONT_MEDIUM)
            .with_line_break_mode(LineBreaking::WordWrap)
            .padding((8.0, 12.0)),
        )
        .with_child(Either::new(
            |state: &AppState, _| state.playback.up_next.is_empty(),
            Label::new("La cola está vacía. Añade canciones desde su menú.")
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .padding(8.0),
            upcoming,
        ))
        .with_child(Either::new(
            |state: &AppState, _| state.queue_visible_count < state.playback.up_next.len(),
            Button::new("Cargar más canciones")
                .on_click(|_, state: &mut AppState, _| {
                    state.queue_visible_count = state.queue_visible_count.saturating_add(100);
                })
                .tooltip("Mostrar las siguientes 100 canciones de la cola")
                .padding((8.0, 16.0)),
            crate::widget::Empty,
        ))
        .expand_width();
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::row()
                .with_child(
                    Label::new("Cola")
                        .with_font(theme::UI_FONT_MEDIUM)
                        .padding((0.0, 10.0))
                        .background(Border::Bottom.with_color(theme::BLUE_200)),
                )
                .with_flex_spacer(1.0)
                .with_child(
                    Label::dynamic(|state: &AppState, _| {
                        format!("{} canciones", state.playback.up_next.len())
                    })
                    .with_text_size(11.0)
                    .with_text_color(theme::PLACEHOLDER_COLOR),
                )
                .with_spacer(8.0)
                .with_child(
                    Button::new("×")
                        .on_click(|_, state: &mut AppState, _| {
                            state.queue_panel_open = false;
                            state.config.show_now_playing = false;
                        })
                        .tooltip("Cerrar la cola"),
                )
                .padding((8.0, 4.0))
                .expand_width(),
        )
        .with_flex_child(Scroll::new(contents).vertical().expand_width(), 1.0)
        .padding(8.0)
        .background(theme::BACKGROUND_DARK)
}
