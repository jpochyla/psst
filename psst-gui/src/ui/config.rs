use crate::data::{AudioQuality, Config, State};
use crate::ui::theme;
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, RadioGroup, TextBox};
use druid::{Widget, WidgetExt};

pub fn make_config() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        // Credentials
        .with_child(Label::new("Device credentials").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            TextBox::new()
                .with_placeholder("Username")
                .lens(Config::username),
        )
        .with_spacer(theme::grid(1.0))
        .with_child(
            TextBox::new()
                .with_placeholder("Password")
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
            Button::new("Save").on_click(|_, config: &mut Config, _env| {
                config.save();
            }),
        )
        .padding(theme::grid(2.0))
        .lens(State::config)
}
