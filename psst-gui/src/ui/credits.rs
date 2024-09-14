use crate::{
    data::AppState,
    ui::theme,
    webapi::{CreditArtist, RoleCredit},
};
use druid::{
    widget::{Flex, Label, List, Scroll},
    LensExt, Widget, WidgetExt, WindowDesc,
};

pub fn credits_window(track_name: &str) -> WindowDesc<AppState> {
    WindowDesc::new(credits_widget())
        .title(format!("Credits for {}", track_name))
        .window_size((400.0, 600.0))
}

fn credits_widget() -> impl Widget<AppState> {
    Scroll::new(
        Flex::column()
            .with_child(
                Label::new(|data: &AppState, _: &_| {
                    data.credits.as_ref().map_or("Credits".to_string(), |c| {
                        format!("Credits for {}", c.track_name)
                    })
                })
                .with_font(theme::UI_FONT_MEDIUM)
                .padding(10.0),
            )
            .with_child(
                List::new(|| {
                    Flex::column()
                        .with_child(
                            Label::new(|item: &RoleCredit, _: &_| item.role_title.clone())
                                .with_font(theme::UI_FONT_MEDIUM),
                        )
                        .with_child(
                            List::new(|| {
                                Label::new(|artist: &CreditArtist, _: &_| artist.name.clone())
                            })
                            .lens(RoleCredit::artists),
                        )
                        .padding(5.0)
                })
                .lens(AppState::credits.map(
                    |credits| {
                        credits
                            .as_ref()
                            .map(|c| c.role_credits.clone())
                            .unwrap_or_default()
                    },
                    |_, _| {},
                )),
            ),
    )
    .vertical()
    .expand()
}
