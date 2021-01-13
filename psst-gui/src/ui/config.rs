use std::thread::{self, JoinHandle};

use crate::{
    cmd,
    data::{AudioQuality, Config, Preferences, PreferencesTab, State, Theme},
    ui::{icons::SvgIcon, theme, utils::Border},
    widget::{icons, HoverExt},
};
use druid::{
    commands,
    widget::{
        Button, Controller, CrossAxisAlignment, Flex, Label, LineBreaking, MainAxisAlignment,
        RadioGroup, TextBox, ViewSwitcher,
    },
    Env, Event, EventCtx, LifeCycle, LifeCycleCtx, Selector, Widget, WidgetExt,
};

pub fn make_config() -> impl Widget<State> {
    let tabs = make_config_tabs()
        .padding(theme::grid(2.0))
        .background(theme::BACKGROUND_LIGHT);

    let active = ViewSwitcher::new(
        |state: &State, _env| state.preferences.active,
        |active: &PreferencesTab, _state, _env| match active {
            PreferencesTab::General => make_config_general().boxed(),
            PreferencesTab::Cache => make_config_cache().boxed(),
        },
    )
    .padding((theme::grid(4.0), theme::grid(4.0)))
    .expand_width()
    .background(Border::Top.widget(theme::GREY_500));

    Flex::column()
        .must_fill_main_axis(true)
        .with_child(tabs)
        .with_child(active)
}

fn make_config_tabs() -> impl Widget<State> {
    let label = |text, icon: &SvgIcon, tab: PreferencesTab| {
        Flex::column()
            .with_child(icon.scale(theme::ICON_SIZE))
            .with_default_spacer()
            .with_child(Label::new(text).with_font(theme::UI_FONT_MEDIUM))
            .padding(theme::grid(1.0))
            .hover()
            .rounded(theme::BUTTON_BORDER_RADIUS)
            .env_scope({
                let tab = tab.clone();
                move |env, state: &State| {
                    if tab == state.preferences.active {
                        env.set(theme::HOVER_COLD_COLOR, env.get(theme::BACKGROUND_DARK));
                        env.set(theme::LABEL_COLOR, env.get(theme::FOREGROUND_LIGHT));
                    } else {
                        env.set(theme::HOVER_COLD_COLOR, env.get(theme::BACKGROUND_LIGHT));
                        env.set(theme::LABEL_COLOR, env.get(theme::LABEL_COLOR));
                    }
                }
            })
            .on_click(move |_ctx, state: &mut State, _env| {
                state.preferences.active = tab;
            })
    };
    Flex::row()
        .must_fill_main_axis(true)
        .main_axis_alignment(MainAxisAlignment::Center)
        .with_child(label(
            "General",
            &icons::PREFERENCES,
            PreferencesTab::General,
        ))
        .with_default_spacer()
        .with_child(label("Cache", &icons::STORAGE, PreferencesTab::Cache))
}

fn make_config_general() -> impl Widget<State> {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    // Theme
    col = col
        .with_child(Label::new("Theme").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            RadioGroup::new(vec![("Light", Theme::Light), ("Dark", Theme::Dark)])
                .lens(Config::theme),
        );

    // Credentials
    col = col
        .with_spacer(theme::grid(3.0))
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
        );

    // Audio quality
    col = col
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
        );

    // Save
    col = col.with_spacer(theme::grid(3.0)).with_child(
        Button::new("Save")
            .on_click(move |ctx, config: &mut Config, _env| {
                config.save();
                ctx.submit_command(cmd::CONFIGURE);
                ctx.submit_command(cmd::SHOW_MAIN);
                ctx.submit_command(commands::CLOSE_WINDOW);
            })
            .fix_width(theme::grid(10.0))
            .align_right(),
    );

    col.lens(State::config)
}

fn make_config_cache() -> impl Widget<State> {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    col = col
        .with_child(Label::new("Location").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            Label::dynamic(|_, _| {
                Config::cache_dir()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| "None".to_string())
            })
            .with_line_break_mode(LineBreaking::WordWrap),
        );

    col = col
        .with_spacer(theme::grid(3.0))
        .with_child(Label::new("Size").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(Label::dynamic(
            |preferences: &Preferences, _| match preferences.cache_size {
                None => {
                    format!("Unknown")
                }
                Some(0) => {
                    format!("Empty")
                }
                Some(b) => {
                    format!("{:.2} MB", b as f64 / 1e6 as f64)
                }
            },
        ));

    col.controller(MeasureCacheSize::new())
        .lens(State::preferences)
}

struct MeasureCacheSize {
    thread: Option<JoinHandle<()>>,
}

impl MeasureCacheSize {
    fn new() -> Self {
        Self { thread: None }
    }
}

impl MeasureCacheSize {
    const UPDATE_CACHE_SIZE: Selector<Option<u64>> = Selector::new("app.measure-cache-size");
}

impl<W: Widget<Preferences>> Controller<Preferences, W> for MeasureCacheSize {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Preferences,
        env: &Env,
    ) {
        match &event {
            Event::Command(cmd) if cmd.is(Self::UPDATE_CACHE_SIZE) => {
                self.thread.take();
                data.cache_size = cmd.get_unchecked(Self::UPDATE_CACHE_SIZE).to_owned();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Preferences,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = &event {
            let handle = thread::spawn({
                let widget_id = ctx.widget_id();
                let event_sink = ctx.get_external_handle();
                move || {
                    let size = Preferences::measure_cache_usage();
                    event_sink
                        .submit_command(Self::UPDATE_CACHE_SIZE, size, widget_id)
                        .unwrap();
                }
            });
            self.thread.replace(handle);
        }
        child.lifecycle(ctx, event, data, env);
    }
}
