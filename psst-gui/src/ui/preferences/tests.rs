use std::{cell::Cell, rc::Rc};

use druid::{
    tests::{harness::Harness, move_mouse},
    widget::Controller,
    Env, Event, EventCtx, KbKey, KeyEvent, Modifiers, MouseButton, Size, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Config, Promise, Theme},
    ui::account_setup_widget,
};

use super::Authenticate;

fn login_content_sizes() -> Vec<Size> {
    let mut sizes = vec![Size::new(400.0, 360.0)];
    if cfg!(target_os = "windows") {
        // Account for the native decorations inside the requested window size.
        sizes.extend([Size::new(384.0, 320.0), Size::new(384.0, 304.0)]);
    }
    sizes
}

fn click_login(
    harness: &mut Harness<'_, AppState>,
    size: Size,
    submitted: impl Fn(&Harness<'_, AppState>) -> bool,
) {
    harness.send_initial_events();
    harness.just_layout();
    harness.paint();

    // Search the visible center column instead of assuming a button position
    // that depends on font metrics and line wrapping. No scrolling is sent.
    for y in (0..size.height as usize).step_by(4) {
        let position = (size.width / 2.0, y as f64);
        harness.event(Event::MouseMove(move_mouse(position)));
        let mut mouse = move_mouse(position);
        mouse.button = MouseButton::Left;
        mouse.count = 1;
        mouse.buttons.insert(MouseButton::Left);
        harness.event(Event::MouseDown(mouse.clone()));
        mouse.buttons.remove(MouseButton::Left);
        harness.event(Event::MouseUp(mouse));
        if submitted(harness) {
            return;
        }
    }
    panic!("Login button did not submit within {size:?}");
}

fn assert_missing_client_id(harness: &Harness<'_, AppState>) {
    // Validation happens before any browser, network, or config-file access.
    assert!(matches!(
        &harness.data().preferences.auth.result,
        Promise::Rejected { err, .. }
            if err == "Please enter your Spotify Developer Client ID first."
    ));
}

#[test]
fn login_button_is_clickable_without_scrolling() {
    for size in login_content_sizes() {
        for selected_theme in [Theme::Light, Theme::Dark] {
            let mut state = AppState::default_with_config(Config::default());
            state.config.theme = selected_theme;
            Harness::create_with_render(
                state,
                account_setup_widget(),
                size,
                |harness| {
                    click_login(harness, size, |harness| {
                        matches!(
                            harness.data().preferences.auth.result,
                            Promise::Rejected { .. }
                        )
                    });
                    assert_missing_client_id(harness);
                },
                |_| {},
            );
        }
    }
}

struct CaptureLogin(Rc<Cell<bool>>);

impl<W: Widget<AppState>> Controller<AppState, W> for CaptureLogin {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(command) = event {
            if command.is(Authenticate::SPOTIFY_REQUEST) {
                self.0.set(true);
                ctx.set_handled();
                // Stop here so the test cannot start browser or network auth.
                return;
            }
        }
        child.event(ctx, event, data, env);
    }
}

#[test]
fn login_button_submits_with_a_configured_client_id() {
    const CLIENT_ID: &str = "0123456789abcdef0123456789abcdef";

    for size in login_content_sizes() {
        let mut state = AppState::default_with_config(Config::default());
        state.config.webapi_client_id = Some(CLIENT_ID.to_string());
        let submitted = Rc::new(Cell::new(false));
        Harness::create_with_render(
            state,
            account_setup_widget().controller(CaptureLogin(Rc::clone(&submitted))),
            size,
            |harness| {
                click_login(harness, size, |_| submitted.get());
                assert!(submitted.get());
                assert_eq!(
                    harness.data().config.webapi_client_id_value(),
                    Some(CLIENT_ID)
                );
            },
            |_| {},
        );
    }
}

#[test]
fn enter_in_client_id_input_submits_login() {
    Harness::create_simple(
        AppState::default_with_config(Config::default()),
        account_setup_widget(),
        |harness| {
            harness.send_initial_events();
            harness.just_layout();
            harness.submit_command(cmd::SET_FOCUS);
            harness.event(Event::KeyDown(KeyEvent::for_test(
                Modifiers::empty(),
                KbKey::Enter,
            )));

            assert_missing_client_id(harness);
        },
    );
}
