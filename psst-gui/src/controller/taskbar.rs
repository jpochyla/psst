//! Windows Taskbar Thumbnail Toolbar integration
//!
//! This module provides functionality to display playback control buttons (https://github.com/jpochyla/psst/issues/659)
//! (play/pause, next, previous) in the Windows taskbar thumbnail preview.

#[cfg(windows)]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

#[cfg(windows)]
use druid::{ExtEventSink, Target, WidgetId};

#[cfg(windows)]
use windows::{
    core::*, Win32::Foundation::*, Win32::System::Com::*, Win32::UI::Shell::*,
    Win32::UI::WindowsAndMessaging::*,
};

#[cfg(windows)]
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32WindowHandle};

#[cfg(windows)]
use crate::{cmd, data::PlaybackState};

#[cfg(windows)]
const WM_COMMAND_OFFSET: u32 = 0x8000;
#[cfg(windows)]
const CMD_PREVIOUS: u32 = WM_COMMAND_OFFSET + 1; // Premier bouton
#[cfg(windows)]
const CMD_PLAY_PAUSE: u32 = WM_COMMAND_OFFSET + 2; // Bouton du milieu
#[cfg(windows)]
const CMD_NEXT: u32 = WM_COMMAND_OFFSET + 3; // Dernier bouton

#[cfg(windows)]
static TASKBAR_CALLBACKS: OnceLock<Arc<Mutex<HashMap<isize, (ExtEventSink, WidgetId)>>>> =
    OnceLock::new();

#[cfg(windows)]
pub struct TaskbarManager {
    hwnd: HWND,
    taskbar_list: Option<ITaskbarList3>,
    event_sink: ExtEventSink,
    widget_id: WidgetId,
    is_initialized: bool,
}

#[cfg(not(windows))]
pub struct TaskbarManager;

impl TaskbarManager {
    #[cfg(windows)]
    pub fn new(
        window_handle: &dyn HasRawWindowHandle,
        event_sink: ExtEventSink,
        widget_id: WidgetId,
    ) -> Result<Self> {
        let hwnd = match window_handle.raw_window_handle() {
            RawWindowHandle::Win32(Win32WindowHandle { hwnd, .. }) => HWND(hwnd),
            _ => {
                log::error!("Failed to extract Win32 window handle");
                return Err(windows::core::Error::from_win32());
            }
        };

        let mut manager = Self {
            hwnd,
            taskbar_list: None,
            event_sink,
            widget_id,
            is_initialized: false,
        };

        match manager.initialize() {
            Ok(_) => Ok(manager),
            Err(e) => {
                log::error!("Failed to initialize TaskbarManager: {:?}", e);
                Err(e)
            }
        }
    }

    #[cfg(not(windows))]
    pub fn new(
        _window_handle: &dyn HasRawWindowHandle,
        _event_sink: ExtEventSink,
        _widget_id: WidgetId,
    ) -> Result<Self> {
        Ok(Self)
    }

    #[cfg(windows)]
    fn initialize(&mut self) -> Result<()> {
        unsafe {
            let com_result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if !com_result.is_ok() {
                log::warn!(
                    "COM initialization failed or already initialized: {:?}",
                    com_result
                );
            }

            let taskbar_list: ITaskbarList3 =
                match CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER) {
                    Ok(tl) => tl,
                    Err(e) => {
                        log::error!("Failed to create ITaskbarList3: {:?}", e);
                        return Err(e.into());
                    }
                };

            match taskbar_list.HrInit() {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to initialize ITaskbarList3: {:?}", e);
                    return Err(e.into());
                }
            }

            self.taskbar_list = Some(taskbar_list);

            let callbacks = TASKBAR_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
            callbacks.lock().unwrap().insert(
                self.hwnd.0 as isize,
                (self.event_sink.clone(), self.widget_id),
            );

            self.subclass_window()?;
            self.is_initialized = true;
        }

        Ok(())
    }

    #[cfg(not(windows))]
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    fn subclass_window(&self) -> Result<()> {
        unsafe {
            let result = SetWindowSubclass(self.hwnd, Some(taskbar_subclass_proc), 1, 0);

            if result.as_bool() {
                Ok(())
            } else {
                log::error!("Failed to subclass window");
                Err(windows::core::Error::from_win32())
            }
        }
    }

    #[cfg(windows)]
    pub fn setup_buttons(&self, playback_state: PlaybackState) -> Result<()> {
        if !self.is_initialized {
            log::warn!("Taskbar manager not initialized, skipping button setup");
            return Ok(());
        }

        let taskbar_list = match &self.taskbar_list {
            Some(tl) => tl,
            None => {
                log::error!("ITaskbarList3 interface is None");
                return Err(windows::core::Error::from_win32());
            }
        };

        unsafe {
            let play_icon = taskbar_icons::create_play_icon();
            let pause_icon = taskbar_icons::create_pause_icon();
            let prev_icon = taskbar_icons::create_previous_icon();
            let next_icon = taskbar_icons::create_next_icon();

            let (play_pause_icon, play_pause_tooltip) = match playback_state {
                PlaybackState::Playing => (pause_icon, "Pause"),
                _ => (play_icon, "Play"),
            };

            let mut buttons = [
                THUMBBUTTON {
                    iId: CMD_PREVIOUS,
                    hIcon: prev_icon,
                    szTip: [0; 260],
                    dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                    dwFlags: THBF_ENABLED,
                    ..Default::default()
                },
                THUMBBUTTON {
                    iId: CMD_PLAY_PAUSE,
                    hIcon: play_pause_icon,
                    szTip: [0; 260],
                    dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                    dwFlags: THBF_ENABLED,
                    ..Default::default()
                },
                THUMBBUTTON {
                    iId: CMD_NEXT,
                    hIcon: next_icon,
                    szTip: [0; 260],
                    dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                    dwFlags: THBF_ENABLED,
                    ..Default::default()
                },
            ];

            self.set_button_tooltip(&mut buttons[0], "Previous")?;
            self.set_button_tooltip(&mut buttons[1], play_pause_tooltip)?;
            self.set_button_tooltip(&mut buttons[2], "Next")?;

            let result = taskbar_list.ThumbBarAddButtons(self.hwnd, &buttons);

            match result {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "Failed to add taskbar buttons: HRESULT 0x{:08x}",
                        e.code().0
                    );
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    pub fn setup_buttons(&self, _playback_state: PlaybackState) -> Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    pub fn update_play_pause_button(&self, playback_state: PlaybackState) -> Result<()> {
        if !self.is_initialized {
            log::warn!("Taskbar manager not initialized, skipping button update");
            return Ok(());
        }

        let taskbar_list = match &self.taskbar_list {
            Some(tl) => tl,
            None => {
                log::error!("ITaskbarList3 interface is None during update");
                return Err(windows::core::Error::from_win32());
            }
        };

        unsafe {
            let (icon, tooltip) = match playback_state {
                PlaybackState::Playing => {
                    let pause_icon = taskbar_icons::create_pause_icon();
                    (pause_icon, "Pause")
                }
                _ => {
                    let play_icon = taskbar_icons::create_play_icon();
                    (play_icon, "Play")
                }
            };

            let mut button = THUMBBUTTON {
                iId: CMD_PLAY_PAUSE,
                hIcon: icon,
                szTip: [0; 260],
                dwMask: THB_ICON | THB_TOOLTIP,
                dwFlags: THBF_ENABLED,
                ..Default::default()
            };

            self.set_button_tooltip(&mut button, tooltip)?;

            match taskbar_list.ThumbBarUpdateButtons(self.hwnd, &[button]) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to update play/pause button: {:?}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    pub fn update_play_pause_button(&self, _playback_state: PlaybackState) -> Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    fn set_button_tooltip(&self, button: &mut THUMBBUTTON, tooltip: &str) -> Result<()> {
        let tooltip_bytes = tooltip.as_bytes();
        let len = tooltip_bytes.len().min(259);

        button.szTip.fill(0);

        for (i, &byte) in tooltip_bytes.iter().take(len).enumerate() {
            button.szTip[i] = byte as u16;
        }

        Ok(())
    }
}

impl Drop for TaskbarManager {
    #[cfg(windows)]
    fn drop(&mut self) {
        if self.is_initialized {
            unsafe {
                let _ = RemoveWindowSubclass(self.hwnd, Some(taskbar_subclass_proc), 1);

                if let Some(callbacks) = TASKBAR_CALLBACKS.get() {
                    callbacks.lock().unwrap().remove(&(self.hwnd.0 as isize));
                }

                CoUninitialize();
            }
        }
    }

    #[cfg(not(windows))]
    fn drop(&mut self) {}
}

#[cfg(windows)]
unsafe extern "system" fn taskbar_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uid_subclass: usize,
    _dw_ref_data: usize,
) -> LRESULT {
    if msg == WM_COMMAND {
        let command_id = (wparam.0 & 0xFFFF) as u32;

        if let Some(callbacks) = TASKBAR_CALLBACKS.get() {
            if let Some((event_sink, widget_id)) =
                callbacks.lock().unwrap().get(&(hwnd.0 as isize)).cloned()
            {
                match command_id {
                    CMD_PLAY_PAUSE => {
                        if let Err(e) = event_sink.submit_command(
                            cmd::PLAY_PAUSE_OR_RESUME,
                            (),
                            Target::Widget(widget_id),
                        ) {
                            log::error!("Failed to submit play/pause command: {:?}", e);
                        }
                    }
                    CMD_PREVIOUS => {
                        if let Err(e) = event_sink.submit_command(
                            cmd::PLAY_PREVIOUS,
                            (),
                            Target::Widget(widget_id),
                        ) {
                            log::error!("Failed to submit previous command: {:?}", e);
                        }
                    }
                    CMD_NEXT => {
                        if let Err(e) =
                            event_sink.submit_command(cmd::PLAY_NEXT, (), Target::Widget(widget_id))
                        {
                            log::error!("Failed to submit next command: {:?}", e);
                        }
                    }
                    _ => {}
                }
                return LRESULT(0);
            }
        }
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

#[cfg(windows)]
mod taskbar_icons {
    use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

    const SZ: i32 = 32;
    unsafe fn new_argb_bitmap() -> (HBITMAP, *mut u32) {
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hdc = GetDC(HWND(std::ptr::null_mut()));
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SZ,
                biHeight: -SZ,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let hbm = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .expect("CreateDIBSection");
        ReleaseDC(HWND(std::ptr::null_mut()), hdc);
        (hbm, bits.cast::<u32>())
    }

    unsafe fn new_and_mask() -> HBITMAP {
        CreateBitmap(SZ, SZ, 1, 1, None)
    }

    fn inside(px: f32, py: f32, poly: &[(i32, i32)]) -> bool {
        let mut inside = false;
        let mut j = poly.len() - 1;
        for (i, &(xi, yi)) in poly.iter().enumerate() {
            let (xj, yj) = (poly[j].0, poly[j].1);
            if ((yi > py as i32) != (yj > py as i32))
                && (px < (xj - xi) as f32 * (py - yi as f32) / (yj - yi) as f32 + xi as f32)
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    unsafe fn rasterise_polys(bits: *mut u32, polys: &[&[(i32, i32)]]) {
        for y in 0..SZ {
            for x in 0..SZ {
                let opaque = polys
                    .iter()
                    .any(|poly| inside(x as f32 + 0.5, y as f32 + 0.5, poly));
                let p = bits.add((y * SZ + x) as usize);
                if opaque {
                    *p = 0xFFFFFFFF;
                } else {
                    *p = 0x00000000;
                }
            }
        }
    }

    unsafe fn icon_from_polys(polys: &[&[(i32, i32)]]) -> HICON {
        let (hbm_color, bits) = new_argb_bitmap();
        rasterise_polys(bits, polys);

        let hbm_mask = new_and_mask();
        let icon_info = ICONINFO {
            fIcon: TRUE,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };
        let hicon = CreateIconIndirect(&icon_info).expect("CreateIconIndirect");

        let _ = DeleteObject(hbm_mask);
        let _ = DeleteObject(hbm_color);
        hicon
    }

    pub fn create_play_icon() -> HICON {
        unsafe { icon_from_polys(&[&[(22, 16), (10, 7), (10, 25)]]) }
    }

    pub fn create_pause_icon() -> HICON {
        let bar_l = &[(11, 7), (14, 7), (14, 25), (11, 25)];
        let bar_r = &[(18, 7), (21, 7), (21, 25), (18, 25)];
        unsafe { icon_from_polys(&[bar_l, bar_r]) }
    }

    pub fn create_next_icon() -> HICON {
        let bar = &[(6, 7), (10, 7), (10, 25), (6, 25)];
        let tri = &[(20, 16), (12, 7), (12, 25)];
        unsafe { icon_from_polys(&[bar, tri]) }
    }

    pub fn create_previous_icon() -> HICON {
        let tri = &[(12, 16), (20, 7), (20, 25)];
        let bar = &[(22, 7), (26, 7), (26, 25), (22, 25)];
        unsafe { icon_from_polys(&[tri, bar]) }
    }
}
