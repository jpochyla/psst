use crate::{
    cmd,
    data::{AppState, QueueEntry},
    ui::Vector,
    widget::{icons, Border, Empty, MyWidgetExt},
};

use druid::{
    widget::{CrossAxisAlignment, Either, Flex, Label, LineBreaking, List, Scroll}, Env, Widget, WidgetExt
};
use druid_shell::Cursor;

use super::theme;

pub fn queue_widget() -> impl Widget<AppState> {
    Either::new(
        |data: &AppState, _env: &Env| data.config.window_size.width >= 700.0,
            Flex::column()
                .with_child(queue_header_widget())
                .with_flex_child(
                    Scroll::new(queue_list_widget())
                        .vertical()
                        // (how will we handle it after the song has been played? will it remain or disappear?)
                        .lens(AppState::added_queue),
                    1.0,
                )
                .fix_width(185.0)
                .background(Border::Left.with_color(theme::GREY_500)),
            Empty
    )
}

fn queue_header_widget() -> impl Widget<AppState> {
    Flex::row()
        .with_flex_child(
            Label::new("Queue")
                .with_font(theme::UI_FONT_MEDIUM)
                .with_text_size(theme::TEXT_SIZE_LARGE)
                .center(),
            1.0,
        )
        .fix_height(32.0)
        .padding(theme::grid(1.0))
        .expand_width()
        .background(Border::Bottom.with_color(theme::GREY_500))
}

fn queue_list_widget() -> impl Widget<Vector<QueueEntry>> {
    List::new(|| {
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(
                        Label::new(|item: &QueueEntry, _env: &Env| item.item.name().to_string())
                            .with_font(theme::UI_FONT_MEDIUM)
                            .with_line_break_mode(LineBreaking::Clip)
                            .expand_width(),
                    )
                    .with_spacer(2.0)
                    .with_child(
                        Label::new(|item: &QueueEntry, _env: &Env| item.item.artist().to_string())
                            .with_text_size(theme::TEXT_SIZE_SMALL)
                            .with_text_color(theme::PLACEHOLDER_COLOR)
                            .with_line_break_mode(LineBreaking::Clip)
                            .expand_width(),
                    )
                .on_click(|ctx, data: &mut QueueEntry, _| {
                    ctx.submit_command(
                        cmd::SKIP_TO_PLACE_IN_QUEUE.with(data.clone())
                    );
                }), 
                1.0,
            )
            .with_default_spacer()
            .with_child(
                icons::CLOSE_CIRCLE
                    .scale((16.0, 16.0))
                    .link()
                    .rounded(100.0)
                    .on_click(|ctx, data: &mut QueueEntry, _| {
                        ctx.submit_command(
                            cmd::REMOVE_FROM_QUEUE.with(data.clone())
                        );
                    })
                    .with_cursor(Cursor::Pointer),
            )
            .padding(theme::grid(1.0))
            .link()
            .rounded(theme::BUTTON_BORDER_RADIUS)
    })
    .padding(theme::grid(1.0))
}
