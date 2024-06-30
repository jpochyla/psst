use druid::{
    im::Vector, widget::{CrossAxisAlignment, Flex, Label, List, Scroll, ViewSwitcher}, Env, Menu, MenuItem, Widget, WidgetExt
};

use crate::{
    cmd,
    data::{
        AppState, QueueEntry
    },
    widget::{icons, Border, MyWidgetExt},
};

use super::theme;

// Is it the best idea to have this in its own file

pub fn queue_widget() -> Box<dyn Widget<AppState>> {
    // Theres possibly a better way to do this, we could probably use an if statemt and no viewswitcher?? 
    // What do you think?
    Box::new(ViewSwitcher::new(
        |data: &AppState, _env: &Env| data.config.window_size.width >= 700.0,
        move |&show_widget, _data, _env| {
            if show_widget {
                let header = Flex::row()
                    .with_child(Label::new("Queue")        
                        .with_font(theme::UI_FONT_MEDIUM)
                        .with_text_size(theme::TEXT_SIZE_LARGE))
                    .with_default_spacer()
                    .padding(theme::grid(1.0))
                    .background(Border::Bottom.with_color(theme::GREY_500));

                let widget = Flex::column()
                    .with_child(header)
                    .with_spacer(theme::grid(1.0))
                    .with_flex_child(
                        Scroll::new(queue_list_widget())
                            .vertical()
                            // The appstate added_queue automatically updates when its changed
                            // To do the handling of the queue we could just make methods directly handling this (how will we handle it after the song has been played? will it remain or disappear?)
                            .lens(AppState::added_queue)
                            .expand(),
                        1.0,
                    )
                    .with_spacer(theme::grid(1.0))
                    .fix_width(185.0)
                    .background(theme::BACKGROUND_DARK);
                Box::new(widget) as Box<dyn Widget<AppState>>
            } else {
                Box::new(Label::new("")) as Box<dyn Widget<AppState>> // Empty widget
            }
        },
    ))
}

fn queue_list_widget() -> impl Widget<Vector<QueueEntry>> {
    List::new(|| {
        Flex::row()
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Label::new(|item: &QueueEntry, _env: &Env| item.item.name().to_string())
                    .with_font(theme::UI_FONT_MEDIUM))
                .with_spacer(2.0)
                .with_child(Label::new(|item: &QueueEntry, _env: &Env| item.item.artist().to_string()).with_text_size(theme::TEXT_SIZE_SMALL)),
                /*.on_left_click(|ctx, _, row, _| {
                    // We need to make a function which takes the song index when clicked on then we need to skip by that amount.
                    ctx.submit_notification(TODO)
                })*/
                //.context_menu(queue_menu_widget(|item: &Vec<QueueEntry>, _env: &Env| item.len()),
            1.0,
        )
        .with_flex_spacer(3.0)
        .with_child(icons::CLOSE_CIRCLE
            .scale((24.0, 24.0))
            .padding(theme::grid(0.25))
            .link()
            .rounded(100.0)
            .on_click(move |ctx, _, _| {
                
                ctx.submit_command(cmd::REMOVE_FROM_QUEUE.with(0/* Add song index here */));
            })        
        )
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .padding(theme::grid(1.0))
    })
}