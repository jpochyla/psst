use std::path::Path;
use std::ptr;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winapi::um::shellapi::{Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP};
use winapi::um::winuser::{
    CreatePopupMenu, AppendMenuW, TrackPopupMenu, SetForegroundWindow, 
    RegisterClassW, CreateWindowExW, DefWindowProcW, PostQuitMessage,
    LoadImageW, DestroyIcon, DestroyMenu, GetCursorPos, PostMessageW,
    WNDCLASSW, WM_USER, WM_COMMAND, WM_DESTROY, WM_RBUTTONUP, WM_LBUTTONUP,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, WS_OVERLAPPEDWINDOW, MF_STRING, MF_SEPARATOR,
    TPM_RIGHTBUTTON, TPM_BOTTOMALIGN, TPM_LEFTALIGN, IMAGE_ICON, LR_LOADFROMFILE,
};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::{HWND, HICON, HMENU, POINT};
use winapi::shared::minwindef::{UINT, DWORD, WPARAM, LPARAM, LRESULT};

const WM_TRAYICON: UINT = WM_USER + 1;
const TRAY_ID: UINT = 1001;

use std::sync::{Arc, Mutex};

pub struct WindowsTray {
    hwnd: HWND,
    hicon: HICON,
    hmenu: HMENU,
    left_click_callback: Option<Arc<Mutex<Box<dyn Fn() + Send + Sync>>>>,
    menu_callback: Option<Arc<Mutex<Box<dyn Fn(u32) + Send + Sync>>>>,
}

impl std::fmt::Debug for WindowsTray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsTray")
            .field("hwnd", &(self.hwnd as usize))
            .field("hicon", &(self.hicon as usize))
            .field("hmenu", &(self.hmenu as usize))
            .finish()
    }
}

static mut TRAY_INSTANCE: Option<Arc<Mutex<WindowsTray>>> = None;

impl WindowsTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let class_name = to_wide_string("PsstTrayWindow");
            let wnd_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                to_wide_string("Psst Tray").as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );

            if hwnd.is_null() {
                return Err("Failed to create window".into());
            }

            let hmenu = CreatePopupMenu();
            if hmenu.is_null() {
                return Err("Failed to create menu".into());
            }
            
            let prev_text = to_wide_string("Previous");
            let pause_text = to_wide_string("Play / Pause");
            let next_text = to_wide_string("Next");
            let exit_text = to_wide_string("Exit");

            if AppendMenuW(hmenu, MF_STRING, 1001, prev_text.as_ptr()) == 0 {
                return Err("Failed to add Previous menu item".into());
            }
            if AppendMenuW(hmenu, MF_STRING, 1002, pause_text.as_ptr()) == 0 {
                return Err("Failed to add Play/Pause menu item".into());
            }
            if AppendMenuW(hmenu, MF_STRING, 1003, next_text.as_ptr()) == 0 {
                return Err("Failed to add Next menu item".into());
            }
            if AppendMenuW(hmenu, MF_SEPARATOR, 0, ptr::null()) == 0 {
                return Err("Failed to add separator".into());
            }
            if AppendMenuW(hmenu, MF_STRING, 1004, exit_text.as_ptr()) == 0 {
                return Err("Failed to add Exit menu item".into());
            }

            Ok(WindowsTray {
                hwnd,
                hicon: ptr::null_mut(),
                hmenu,
                left_click_callback: None,
                menu_callback: None,
            })
        }
    }

    pub fn set_icon(&mut self, icon_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let icon_path_wide = to_wide_string(&icon_path.to_string_lossy());
            self.hicon = LoadImageW(
                ptr::null_mut(),
                icon_path_wide.as_ptr(),
                IMAGE_ICON,
                0, 0,
                LR_LOADFROMFILE,
            ) as HICON;

            if self.hicon.is_null() {
                return Err("Failed to load icon".into());
            }

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
            nid.hWnd = self.hwnd;
            nid.uID = TRAY_ID;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = self.hicon;

            let tooltip = to_wide_string("Psst - Right click for menu");
            let len = std::cmp::min(tooltip.len(), 127);
            nid.szTip[..len].copy_from_slice(&tooltip[..len]);

            if Shell_NotifyIconW(NIM_ADD, &mut nid) == 0 {
                return Err("Failed to add tray icon".into());
            }

            Ok(())
        }
    }

    pub fn set_callbacks<F1, F2>(&mut self, left_click: F1, menu_click: F2) -> Result<(), Box<dyn std::error::Error>>
    where
        F1: Fn() + Send + Sync + 'static,
        F2: Fn(u32) + Send + Sync + 'static,
    {
        self.left_click_callback = Some(Arc::new(Mutex::new(Box::new(left_click))));
        self.menu_callback = Some(Arc::new(Mutex::new(Box::new(menu_click))));
        Ok(())
    }

    pub fn set_as_global_instance(tray: Arc<Mutex<WindowsTray>>) {
        unsafe {
            TRAY_INSTANCE = Some(tray);
        }
    }

    pub fn clear_global_instance() {
        unsafe {
            TRAY_INSTANCE = None;
        }
    }
}

impl Drop for WindowsTray {
    fn drop(&mut self) {
        unsafe {
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
            nid.hWnd = self.hwnd;
            nid.uID = TRAY_ID;
            Shell_NotifyIconW(NIM_DELETE, &mut nid);

            if !self.hicon.is_null() {
                DestroyIcon(self.hicon);
            }
            if !self.hmenu.is_null() {
                DestroyMenu(self.hmenu);
            }
            
            WindowsTray::clear_global_instance();
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            match lparam as UINT {
                WM_LBUTTONUP => {
                    PostMessageW(hwnd, WM_USER + 100, 0, 0);
                }
                WM_RBUTTONUP => {
                    PostMessageW(hwnd, WM_USER + 101, 0, 0);
                }
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            let command_id = (wparam & 0xFFFF) as u32;
            println!("WM_COMMAND received: command_id = {}", command_id);
            
            if let Some(ref tray_arc) = TRAY_INSTANCE {
                if let Ok(tray) = tray_arc.lock() {
                    if let Some(ref callback) = tray.menu_callback {
                        if let Ok(cb) = callback.lock() {
                            println!("Calling menu callback with ID: {}", command_id);
                            cb(command_id);
                        }
                    }
                }
            }
            0
        }
        msg if msg == WM_USER + 100 => {
            if let Some(ref tray_arc) = TRAY_INSTANCE {
                if let Ok(tray) = tray_arc.lock() {
                    if let Some(ref callback) = tray.left_click_callback {
                        if let Ok(cb) = callback.lock() {
                            cb();
                        }
                    }
                }
            }
            0
        }
        msg if msg == WM_USER + 101 => {
            if let Some(ref tray_arc) = TRAY_INSTANCE {
                if let Ok(tray) = tray_arc.lock() {
                    let mut cursor_pos = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut cursor_pos);
                    SetForegroundWindow(hwnd);
                    
                    TrackPopupMenu(
                        tray.hmenu,
                        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                        cursor_pos.x,
                        cursor_pos.y,
                        0,
                        hwnd,
                        ptr::null(),
                    );
                    
                    println!("Context menu displayed");
                }
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
} 