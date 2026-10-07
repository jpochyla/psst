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
    core::{w, GUID, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, PROPERTYKEY, WPARAM},
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, StructuredStorage::PROPVARIANT,
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            },
            Variant::VT_LPWSTR,
        },
        UI::{
            Shell::{
                DefSubclassProc, ITaskbarList3,
                PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
                RemoveWindowSubclass, SHStrDupW, SetWindowSubclass, TaskbarList, THBF_DISABLED,
                THBF_ENABLED, THBN_CLICKED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON,
            },
            WindowsAndMessaging::{
                CreateIcon, DestroyIcon, RegisterWindowMessageW, HICON, WM_COMMAND, WM_NCDESTROY,
            },
        },
    },
};

use windows::Win32::UI::{
    Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    },
    Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    },
    WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, WM_APP, WM_HOTKEY, WM_LBUTTONDBLCLK,
        WM_RBUTTONUP,
    },
};
const TRAY_MESSAGE: u32 = WM_APP + 74;
const HOTKEY_ID: i32 = 0x5850;
const RESUME_HOTKEY: druid::Selector = druid::Selector::new("app.desktop.resume-hotkey");
const DESKTOP_SYNC: druid::Selector<bool> = druid::Selector::new("app.desktop.sync");

pub fn startup_in_tray(config: &crate::data::Config) -> bool {
    config.start_in_tray && std::env::args().any(|arg| arg == "--autostart")
}

fn set_autostart(enabled: bool) -> std::io::Result<()> {
    use windows_sys::Win32::System::Registry::{
        RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
    };
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let name: Vec<u16> = "Xpotify\0".encode_utf16().collect();
    let status = if enabled {
        let exe = std::env::current_exe()?;
        let value: Vec<u16> = format!("\"{}\" --autostart\0", exe.display())
            .encode_utf16()
            .collect();
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) }
    };
    if status == 0 || (!enabled && status == 2) {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(status as i32))
    }
}

const READY: druid::Selector = druid::Selector::new("app.taskbar.ready");
const SUBCLASS: usize = 0x58504F54;
const PREVIOUS: u32 = 0x7101;
const TOGGLE: u32 = 0x7102;
const NEXT: u32 = 0x7103;

fn set_application_identity(hwnd: HWND, executable: &std::path::Path) -> windows::core::Result<()> {
    let store: IPropertyStore = unsafe { SHGetPropertyStoreForWindow(hwnd)? };
    let executable = executable.to_string_lossy();
    // Relaunch properties must be supplied together, with an explicit window AppID.
    // Resource 101 is defined in build.rs; the existing executable icon is retained.
    for (pid, text) in [
        (2, format!("\"{executable}\"")),
        (4, format!("@{executable},-101")),
        (5, "com.angelopol.xpotify".to_owned()),
    ] {
        let key = PROPERTYKEY {
            fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
            pid,
        };
        let mut value = PROPVARIANT::default();
        // PROPVARIANT's Rust Drop calls PropVariantClear. Give it a COM-owned
        // allocation from SHStrDupW, never a borrowed Rust string buffer.
        unsafe {
            let text = SHStrDupW(&HSTRING::from(text))?;
            let inner = &mut *value.Anonymous.Anonymous;
            inner.vt = VT_LPWSTR;
            inner.Anonymous.pwszVal = text;
            store.SetValue(&key, &value)?;
        }
    }
    // Window property stores apply values immediately; Commit is unnecessary.
    Ok(())
}

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
    explorer_message: u32,
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
    tray: bool,
    hotkey: bool,
}

impl Toolbar {
    fn tray_data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: self.icons[1].0,
            ..Default::default()
        };
        for (dst, ch) in data
            .szTip
            .iter_mut()
            .zip("Xpotify - doble clic para abrir".encode_utf16())
        {
            *dst = ch;
        }
        data
    }
    fn add_tray(&mut self) {
        self.tray = unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.tray_data()).as_bool()
                || Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_MODIFY, &self.tray_data())
                    .as_bool()
        };
        if !self.tray {
            log::warn!("Could not create tray icon");
        }
    }
    fn configure_hotkey(&mut self, enabled: bool) -> windows::core::Result<()> {
        if self.hotkey && !enabled {
            unsafe {
                UnregisterHotKey(Some(self.hwnd), HOTKEY_ID)?;
            }
            self.hotkey = false;
        }
        if enabled && !self.hotkey {
            unsafe {
                RegisterHotKey(
                    Some(self.hwnd),
                    HOTKEY_ID,
                    MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                    u32::from(b'P'),
                )?;
            }
            self.hotkey = true;
        }
        Ok(())
    }

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
            if self.tray {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.tray_data());
            }
            if self.hotkey {
                let _ = UnregisterHotKey(Some(self.hwnd), HOTKEY_ID);
            }
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
    if message == WM_HOTKEY && wp.0 == HOTKEY_ID as usize {
        let _ = hook.sink.submit_command(RESUME_HOTKEY, (), Target::Global);
        return LRESULT(0);
    } else if message == TRAY_MESSAGE {
        if lp.0 as u32 == WM_LBUTTONDBLCLK || lp.0 as u32 == WM_RBUTTONUP {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
        }
        return LRESULT(0);
    } else if message == hook.created_message || message == hook.explorer_message {
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

pub struct TaskbarController {
    toolbar: Option<Box<Toolbar>>,
    hide_timer: druid::TimerToken,
}

impl Default for TaskbarController {
    fn default() -> Self {
        Self {
            toolbar: None,
            hide_timer: druid::TimerToken::INVALID,
        }
    }
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
        if let Event::Timer(token) = event {
            if *token == self.hide_timer {
                if let Some(toolbar) = &self.toolbar {
                    unsafe {
                        let _ = ShowWindow(
                            toolbar.hwnd,
                            if toolbar.tray { SW_HIDE } else { SW_RESTORE },
                        );
                    }
                }
            }
        }
        if let Event::Command(command) = event {
            if command.is(RESUME_HOTKEY) {
                if data.config.resume_hotkey {
                    if data.config.resume_hotkey_next {
                        if let Some(entry) = data.playback.up_next.front() {
                            ctx.submit_command(cmd::PLAY_UPCOMING.with((0, entry.item.id())));
                        } else {
                            data.error_alert("No hay una cancion siguiente en la cola guardada.");
                        }
                    } else if data.playback.now_playing.is_some() {
                        ctx.submit_command(cmd::PLAY_RESUME);
                    } else {
                        data.error_alert("Todavia no hay una cancion guardada para reanudar.");
                    }
                }
                ctx.set_handled();
                return;
            }
            if command.is(DESKTOP_SYNC) {
                if *command.get_unchecked(DESKTOP_SYNC) {
                    if let Err(error) = set_autostart(data.config.start_with_windows) {
                        log::error!("Windows startup setting failed: {error}");
                        data.error_alert(format!(
                            "No se pudo configurar el inicio con Windows: {error}"
                        ));
                    }
                }
                if let Some(toolbar) = &mut self.toolbar {
                    if let Err(error) = toolbar.configure_hotkey(data.config.resume_hotkey) {
                        log::warn!("Global hotkey registration failed: {error}");
                        data.error_alert(
                            "No se pudo registrar Ctrl + Alt + P. Puede estar en uso por otra app.",
                        );
                    }
                }
                ctx.set_handled();
                return;
            }
            if command.is(READY) {
                if let Some(toolbar) = &mut self.toolbar {
                    toolbar.installed = false;
                    toolbar.add_tray();
                    if !toolbar.tray {
                        unsafe {
                            let _ = ShowWindow(toolbar.hwnd, SW_RESTORE);
                        }
                    }
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
                if let Ok(executable) = std::env::current_exe() {
                    if let Err(error) = set_application_identity(HWND(handle.hwnd), &executable) {
                        log::warn!("Could not set Xpotify taskbar identity: {error}");
                    }
                }
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
                            explorer_message: unsafe {
                                RegisterWindowMessageW(w!("TaskbarCreated"))
                            },
                            destroyed: AtomicBool::new(false),
                            available: AtomicBool::new(data.playback.now_playing.is_some()),
                        }),
                        installed: false,
                        hook_reference: None,
                        playing: data.playback.state == PlaybackState::Playing,
                        available: data.playback.now_playing.is_some(),
                        com_initialized: initialized,
                        tray: false,
                        hotkey: false,
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
                            toolbar.add_tray();
                            if let Err(error) = toolbar.configure_hotkey(data.config.resume_hotkey)
                            {
                                log::warn!("Global hotkey registration failed: {error}");
                                data.error_alert("No se pudo registrar Ctrl + Alt + P. Puede estar en uso por otra app.");
                            }
                            if startup_in_tray(&data.config) {
                                self.hide_timer =
                                    ctx.request_timer(std::time::Duration::from_millis(100));
                            }
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
        if old.config.start_with_windows != data.config.start_with_windows
            || old.config.resume_hotkey != data.config.resume_hotkey
        {
            ctx.submit_command(
                DESKTOP_SYNC.with(old.config.start_with_windows != data.config.start_with_windows),
            );
        }
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
