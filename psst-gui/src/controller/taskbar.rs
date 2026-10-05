//! Windows thumbnail controls. All COM/subclass operations stay on the UI thread.
use crate::{
    cmd,
    data::{AppState, PlaybackState},
};
use druid::{widget::prelude::*, widget::Controller, ExtEventSink, Target};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
        UI::{
            Shell::{
                DefSubclassProc, ITaskbarList3, RemoveWindowSubclass, SetWindowSubclass,
                TaskbarList, THBF_DISABLED, THBF_ENABLED, THBN_CLICKED, THB_FLAGS, THB_ICON,
                THB_TOOLTIP, THUMBBUTTON,
            },
            WindowsAndMessaging::{
                CreateIcon, DestroyIcon, RegisterWindowMessageW, HICON, WM_COMMAND, WM_NCDESTROY,
            },
        },
    },
};

const READY: druid::Selector = druid::Selector::new("app.taskbar.ready");
const SUBCLASS: usize = 0x58504F54;
const PREVIOUS: u32 = 0x7101;
const TOGGLE: u32 = 0x7102;
const NEXT: u32 = 0x7103;

struct Icon(HICON);
impl Drop for Icon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

fn icon(kind: usize) -> windows::core::Result<Icon> {
    // Small white transport glyphs with a transparent background; no disk assets.
    let mut mask = [255u8; 24 * 4];
    let mut rgba = [0u8; 24 * 24 * 4];
    for y in 0usize..24 {
        for x in 0usize..24 {
            let pixel = match kind {
                0 => {
                    (5..8).contains(&x) && (5..19).contains(&y)
                        || (8..19).contains(&x) && y.abs_diff(12) <= (x - 8) * 7 / 11
                }
                1 => (7..19).contains(&x) && y.abs_diff(12) <= (19 - x) * 7 / 12,
                2 => ((6..10).contains(&x) || (14..18).contains(&x)) && (5..19).contains(&y),
                _ => {
                    (16..19).contains(&x) && (5..19).contains(&y)
                        || (5..16).contains(&x) && y.abs_diff(12) <= (16 - x) * 7 / 11
                }
            };
            if pixel {
                mask[y * 4 + x / 8] &= !(0x80 >> (x % 8));
                rgba[(y * 24 + x) * 4..(y * 24 + x + 1) * 4].fill(255);
            }
        }
    }
    unsafe { CreateIcon(None, 24, 24, 1, 32, mask.as_ptr(), rgba.as_ptr()).map(Icon) }
}

struct Hook {
    sink: ExtEventSink,
    created_message: u32,
    destroyed: AtomicBool,
    available: AtomicBool,
}

struct Toolbar {
    hwnd: HWND,
    taskbar: Option<ITaskbarList3>,
    icons: Vec<Icon>,
    hook: Arc<Hook>,
    installed: bool,
    hook_reference: Option<*const Hook>,
    playing: bool,
    available: bool,
    com_initialized: bool,
}

impl Toolbar {
    fn buttons(&self) -> [THUMBBUTTON; 3] {
        let mut buttons = [THUMBBUTTON::default(); 3];
        for (i, (id, glyph, tip)) in [
            (PREVIOUS, 0, "Anterior"),
            (
                TOGGLE,
                if self.playing { 2 } else { 1 },
                if self.playing { "Pausar" } else { "Reproducir" },
            ),
            (NEXT, 3, "Siguiente"),
        ]
        .into_iter()
        .enumerate()
        {
            buttons[i].dwMask = THB_ICON | THB_TOOLTIP | THB_FLAGS;
            buttons[i].iId = id;
            buttons[i].hIcon = self.icons[glyph].0;
            buttons[i].dwFlags = if self.available {
                THBF_ENABLED
            } else {
                THBF_DISABLED
            };
            for (dst, ch) in buttons[i].szTip.iter_mut().zip(tip.encode_utf16()) {
                *dst = ch;
            }
        }
        buttons
    }

    fn sync(&mut self) {
        let Some(taskbar) = &self.taskbar else { return };
        if self.hook.destroyed.load(Ordering::Relaxed) {
            return;
        }
        unsafe {
            let result = if self.installed {
                taskbar.ThumbBarUpdateButtons(self.hwnd, &self.buttons())
            } else {
                taskbar.ThumbBarAddButtons(self.hwnd, &self.buttons())
            };
            if result.is_ok() {
                if !self.installed {
                    log::info!("Windows taskbar transport controls installed");
                }
                self.installed = true;
            } else {
                log::debug!("Taskbar toolbar not ready: {result:?}");
            }
        }
    }
}

impl Drop for Toolbar {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.destroyed.load(Ordering::Relaxed) {
                let _ = RemoveWindowSubclass(self.hwnd, Some(window_proc), SUBCLASS);
            }
            if let Some(reference) = self.hook_reference.take() {
                drop(Arc::from_raw(reference));
            }
            self.taskbar.take(); // Release COM before balancing initialization.
            if self.com_initialized {
                CoUninitialize();
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    reference: usize,
) -> LRESULT {
    // Only shared, immutable callback state is accessed during reentrant Win32 calls.
    let hook = unsafe { &*(reference as *const Hook) };
    if message == hook.created_message {
        let _ = hook.sink.submit_command(READY, (), Target::Global);
    } else if message == WM_COMMAND && (wp.0 >> 16) & 0xffff == THBN_CLICKED as usize {
        let command = match (wp.0 & 0xffff) as u32 {
            PREVIOUS => Some(cmd::PLAY_PREVIOUS),
            TOGGLE => Some(cmd::PLAY_TOGGLE),
            NEXT => Some(cmd::PLAY_NEXT),
            _ => None,
        };
        if let Some(command) = command {
            if hook.available.load(Ordering::Relaxed) {
                let _ = hook.sink.submit_command(command, (), Target::Global);
            }
            return LRESULT(0);
        }
    } else if message == WM_NCDESTROY {
        hook.destroyed.store(true, Ordering::Relaxed);
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(window_proc), SUBCLASS);
        }
    }
    unsafe { DefSubclassProc(hwnd, message, wp, lp) }
}

#[derive(Default)]
pub struct TaskbarController {
    toolbar: Option<Box<Toolbar>>,
}

impl<W: Widget<AppState>> Controller<AppState, W> for TaskbarController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(command) = event {
            if command.is(READY) {
                if let Some(toolbar) = &mut self.toolbar {
                    toolbar.installed = false;
                    toolbar.sync();
                }
                ctx.set_handled();
                return;
            }
        }
        if matches!(event, Event::WindowConnected) && self.toolbar.is_none() {
            if let RawWindowHandle::Win32(handle) = ctx.window().raw_window_handle() {
                // Druid and other media integrations may already initialize COM.
                let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
                let result = (|| -> windows::core::Result<Box<Toolbar>> {
                    let taskbar: ITaskbarList3 =
                        unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)? };
                    unsafe {
                        taskbar.HrInit()?;
                    }
                    let icons = (0..4)
                        .map(icon)
                        .collect::<windows::core::Result<Vec<_>>>()?;
                    Ok(Box::new(Toolbar {
                        hwnd: HWND(handle.hwnd),
                        taskbar: Some(taskbar),
                        icons,
                        hook: Arc::new(Hook {
                            sink: ctx.get_external_handle(),
                            created_message: unsafe {
                                RegisterWindowMessageW(w!("TaskbarButtonCreated"))
                            },
                            destroyed: AtomicBool::new(false),
                            available: AtomicBool::new(data.playback.now_playing.is_some()),
                        }),
                        installed: false,
                        hook_reference: None,
                        playing: data.playback.state == PlaybackState::Playing,
                        available: data.playback.now_playing.is_some(),
                        com_initialized: initialized,
                    }))
                })();
                match result {
                    Ok(mut toolbar) => {
                        let pointer = Arc::into_raw(toolbar.hook.clone());
                        toolbar.hook_reference = Some(pointer);
                        if unsafe {
                            SetWindowSubclass(
                                toolbar.hwnd,
                                Some(window_proc),
                                SUBCLASS,
                                pointer as usize,
                            )
                            .as_bool()
                        } {
                            toolbar.sync();
                            self.toolbar = Some(toolbar);
                        } else {
                            log::warn!("Could not attach taskbar controls");
                        }
                    }
                    Err(error) => {
                        if initialized {
                            unsafe {
                                CoUninitialize();
                            }
                        }
                        log::warn!("Taskbar controls unavailable: {error}");
                    }
                }
            }
        }
        child.event(ctx, event, data, env);
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if let Some(toolbar) = &mut self.toolbar {
            let playing = data.playback.state == PlaybackState::Playing;
            let available = data.playback.now_playing.is_some()
                && data.playback.state != PlaybackState::Loading;
            if toolbar.playing != playing || toolbar.available != available {
                toolbar.playing = playing;
                toolbar.available = available;
                toolbar.hook.available.store(available, Ordering::Relaxed);
                toolbar.sync();
            }
        }
        child.update(ctx, old, data, env);
    }
}
