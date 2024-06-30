use druid::{
    im::Vector, widget::{CrossAxisAlignment, Flex, Label, List, Scroll, ViewSwitcher}, Env, Menu, MenuItem, Widget, WidgetExt
};

use crate::{
    cmd,
    data::{
        AppState, QueueEntry
    },
    widget::{Border, MyWidgetExt},
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
                //.context_menu(queue_menu_widget()),
            1.0,
        )
        .with_default_spacer()
        .padding(theme::grid(1.0))
        .link()
        
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .padding(theme::grid(1.0))
    })
}
fn queue_menu_widget() -> Menu<AppState> {
    //.with_child(queue_menu_item_widget("Clear Queue", Command::ClearQueue))
    //.with_child(queue_menu_item_widget("Remove from queue", Command::RemoveFromQueue))
    let mut menu = Menu::new("Queue");

    // Create menu items for sorting options
    let sort_by_title = MenuItem::new("Title").command(cmd::SORT_BY_TITLE);
    let sort_by_album = MenuItem::new("Album").command(cmd::SORT_BY_ALBUM);
    let sort_by_date_added = MenuItem::new("Date Added").command(cmd::SORT_BY_DATE_ADDED);
    let sort_by_duration = MenuItem::new("Duration").command(cmd::SORT_BY_DURATION);
    let sort_by_artist = MenuItem::new("Artist").command(cmd::SORT_BY_ARTIST);


    // Add the items and checkboxes to the menu
    menu = menu.entry(sort_by_album);
    menu = menu.entry(sort_by_artist);
    menu = menu.entry(sort_by_date_added);
    menu = menu.entry(sort_by_duration);
    menu = menu.entry(sort_by_title);

    menu
}