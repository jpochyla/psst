use crate::data::State;
pub use druid::theme::*;
use druid::{Color, Env, FontDescriptor, FontFamily, FontWeight, Insets, Key, Size};

pub fn grid(m: f64) -> f64 {
    GRID * m
}

pub const GRID: f64 = 8.0;

pub const WHITE: Color = Color::WHITE;
pub const DARK_WHITE: Color = Color::rgb8(34, 40, 49);
pub const DARK_SELECTED: Color = Color::rgb8(57, 62, 70);
pub const BLACK: Color = Color::BLACK;
pub const GREY_1: Color = Color::grey8(0x33);
pub const GREY_2: Color = Color::grey8(0x4f);
pub const DARK_GREY_2: Color = Color::grey8(0xef);
pub const GREY_3: Color = Color::grey8(0x82);
pub const GREY_4: Color = Color::grey8(0xbd);
pub const GREY_5: Color = Color::grey8(0xe0);
pub const GREY_6: Color = Color::grey8(0xf2);
pub const DARK_GREY_6: Color = Color::grey8(0x32);
pub const BLUE_LIGHT: Color = Color::rgb8(0x5c, 0xc4, 0xff);
pub const BLUE_DARK: Color = Color::rgb8(0x00, 0x8d, 0xdd);

pub const MENU_BUTTON_BG_ACTIVE: Color = GREY_5;
pub const MENU_BUTTON_BG_INACTIVE: Color = GREY_6;
pub const MENU_BUTTON_FG_ACTIVE: Color = GREY_1;
pub const MENU_BUTTON_FG_INACTIVE: Color = GREY_2;

pub const UI_FONT_MEDIUM: Key<FontDescriptor> = Key::new("app.ui-font-medium");
pub const UI_FONT_MONO: Key<FontDescriptor> = Key::new("app.ui-font-mono");
pub const TEXT_SIZE_SMALL: Key<f64> = Key::new("app.text-size-small");

pub const ICON_COLOR: Key<Color> = Key::new("app.icon-color");
pub const ICON_SIZE: Size = Size::new(12.0, 12.0);

pub const HOVER_HOT_COLOR: Key<Color> = Key::new("app.hover-hot-color");
pub const HOVER_COLD_COLOR: Key<Color> = Key::new("app.hover-cold-color");

pub fn setup(env: &mut Env, _state: &State) {
    let dark_theme = true;

    if dark_theme {
        env.set(WINDOW_BACKGROUND_COLOR, DARK_WHITE);
        env.set(LABEL_COLOR, DARK_GREY_2);
        env.set(SELECTION_TEXT_COLOR, DARK_SELECTED);

        env.set(SELECTION_COLOR, BLUE_LIGHT);
        env.set(BACKGROUND_DARK, DARK_GREY_6);
        env.set(BUTTON_DARK, DARK_GREY_6);

        env.set(BUTTON_LIGHT, DARK_WHITE);

        env.set(HOVER_COLD_COLOR, MENU_BUTTON_BG_ACTIVE);
        env.set(LABEL_COLOR, DARK_GREY_2);

        env.set(BACKGROUND_LIGHT, DARK_WHITE);

        env.set(CURSOR_COLOR, GREY_3);
    } else {
        env.set(WINDOW_BACKGROUND_COLOR, WHITE);
        env.set(LABEL_COLOR, GREY_2);
        env.set(SELECTION_TEXT_COLOR, BLACK);

        env.set(SELECTION_COLOR, BLUE_LIGHT);
        env.set(BACKGROUND_DARK, GREY_6);
        env.set(BUTTON_DARK, GREY_6);

        env.set(BUTTON_LIGHT, WHITE);
        env.set(BACKGROUND_LIGHT, WHITE);

        env.set(CURSOR_COLOR, BLACK);
    }

    env.set(ICON_COLOR, GREY_3);
    env.set(PLACEHOLDER_COLOR, GREY_3);
    env.set(PRIMARY_LIGHT, BLUE_LIGHT);
    env.set(PRIMARY_DARK, BLUE_DARK);

    env.set(PROGRESS_BAR_RADIUS, 4.0);

    env.set(FOREGROUND_LIGHT, GREY_1);
    env.set(FOREGROUND_DARK, BLACK);

    env.set(BUTTON_BORDER_RADIUS, 4.0);
    env.set(BUTTON_BORDER_WIDTH, 1.0);

    env.set(BORDER_DARK, GREY_5);
    env.set(BORDER_LIGHT, GREY_4);

    env.set(
        UI_FONT,
        FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(13.0),
    );
    env.set(
        UI_FONT_MEDIUM,
        FontDescriptor::new(FontFamily::SYSTEM_UI)
            .with_size(13.0)
            .with_weight(FontWeight::MEDIUM),
    );
    env.set(
        UI_FONT_MONO,
        FontDescriptor::new(FontFamily::MONOSPACE).with_size(13.0),
    );
    env.set(TEXT_SIZE_SMALL, 12.0);
    env.set(TEXT_SIZE_NORMAL, 13.0);
    env.set(TEXT_SIZE_LARGE, 16.0);

    env.set(BASIC_WIDGET_HEIGHT, grid(3.0));
    env.set(WIDE_WIDGET_WIDTH, grid(12.0));
    env.set(BORDERED_WIDGET_HEIGHT, grid(4.0));

    env.set(TEXTBOX_BORDER_RADIUS, 4.0);
    env.set(TEXTBOX_BORDER_WIDTH, 1.0);
    env.set(TEXTBOX_INSETS, Insets::uniform_xy(grid(1.2), grid(1.0)));

    env.set(SCROLLBAR_COLOR, GREY_3);
    env.set(SCROLLBAR_BORDER_COLOR, GREY_3);
    env.set(SCROLLBAR_MAX_OPACITY, 0.8);
    env.set(SCROLLBAR_FADE_DELAY, 1500u64);
    env.set(SCROLLBAR_WIDTH, 6.0);
    env.set(SCROLLBAR_PAD, 2.0);
    env.set(SCROLLBAR_RADIUS, 5.0);
    env.set(SCROLLBAR_EDGE_WIDTH, 1.0);

    env.set(WIDGET_PADDING_VERTICAL, grid(0.5));
    env.set(WIDGET_PADDING_HORIZONTAL, grid(1.0));
    env.set(WIDGET_CONTROL_COMPONENT_PADDING, grid(1.0));

    env.set(HOVER_HOT_COLOR, Color::rgba(0.0, 0.0, 0.0, 0.05));
    env.set(HOVER_COLD_COLOR, Color::rgba(0.0, 0.0, 0.0, 0.0));
}
