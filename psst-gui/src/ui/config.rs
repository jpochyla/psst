use crate::{
    cmd,
    data::{AudioQuality, Config, State},
    ui::theme,
    widget::HoverExt,
};
use druid::{
    commands,
    widget::{Button, CrossAxisAlignment, Flex, Label, RadioGroup, TextBox},
    Widget, WidgetExt,
};

pub fn make_config() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        // Credentials
        .with_child(Label::new("Device credentials").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            Flex::row()
                .with_child(
                    Label::new("You can set these up in your ")
                        .with_text_color(theme::PLACEHOLDER_COLOR)
                        .with_text_size(theme::TEXT_SIZE_SMALL),
                )
                .with_child(
                    Label::new("Spotify Account Settings.")
                        .with_text_size(theme::TEXT_SIZE_SMALL)
                        .hover()
                        .on_click(|_ctx, _data, _env| {
                            if let Err(err) =
                                open::that("https://www.spotify.com/account/set-device-password")
                            {
                                log::error!("error while opening url: {:?}", err);
                            }
                        }),
                ),
        )
        .with_spacer(theme::grid(2.0))
        .with_child(
            TextBox::new()
                .with_placeholder("Username")
                .env_scope(|env, _state| env.set(theme::WIDE_WIDGET_WIDTH, theme::grid(16.0)))
                .lens(Config::username),
        )
        .with_spacer(theme::grid(1.0))
        .with_child(
            TextBox::new()
                .with_placeholder("Password")
                .env_scope(|env, _state| env.set(theme::WIDE_WIDGET_WIDTH, theme::grid(16.0)))
                .lens(Config::password),
        )
        // Audio quality
        .with_spacer(theme::grid(3.0))
        .with_child(Label::new("Audio quality").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            RadioGroup::new(vec![
                ("Low (96kbit)", AudioQuality::Low),
                ("Normal (160kbit)", AudioQuality::Normal),
                ("High (320kbit)", AudioQuality::High),
            ])
            .lens(Config::audio_quality),
        )
        // Save
        .with_spacer(theme::grid(3.0))
        .with_child(
            Button::new("Save")
                .on_click(move |ctx, config: &mut Config, _env| {
                    config.save();
                    ctx.submit_command(cmd::CONFIGURE);
                    ctx.submit_command(cmd::SHOW_MAIN);
                    ctx.submit_command(commands::CLOSE_WINDOW);
                })
                .center(),
        )
        .padding((theme::grid(2.0), theme::grid(0.0)))
        .lens(State::config)
}
