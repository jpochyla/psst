//! A session-wide Windows lock, with activation queued until the UI is ready.
use druid::{ExtEventSink, Selector, Target};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};
use windows::{
    core::{w, BOOL, HSTRING},
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, WAIT_OBJECT_0,
        },
        System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE},
        UI::WindowsAndMessaging::{
            AllowSetForegroundWindow, EnumWindows, GetPropW, GetWindowThreadProcessId,
            SetForegroundWindow, SetPropW, ShowWindowAsync, SW_RESTORE,
        },
    },
};

pub const ACTIVATE: Selector = Selector::new("app.single-instance.activate");
const WINDOW_PROPERTY: windows::core::PCWSTR = w!("com.angelopol.xpotify.primary-window");

pub struct SingleInstance {
    mutex: HANDLE,
    event: HANDLE,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl SingleInstance {
    /// None means another process owns the instance; it has been asked to activate.
    pub fn acquire() -> windows::core::Result<Option<Self>> {
        Self::acquire_named("Local\\com.angelopol.xpotify", true)
    }

    fn acquire_named(name: &str, activate_window: bool) -> windows::core::Result<Option<Self>> {
        unsafe {
            let mutex_name = HSTRING::from(format!("{name}.mutex"));
            let event_name = HSTRING::from(format!("{name}.activate"));
            // Keep the event alive before publishing the mutex, so a simultaneous
            // second launch cannot create, signal and destroy it ahead of us.
            let event = CreateEventW(None, false, false, &event_name)?;
            let mutex = match CreateMutexW(None, false, &mutex_name) {
                Ok(mutex) => mutex,
                Err(error) => {
                    let _ = CloseHandle(event);
                    return Err(error);
                }
            };
            let exists = GetLastError() == ERROR_ALREADY_EXISTS;
            let instance = Self {
                mutex,
                event,
                stopping: Arc::new(AtomicBool::new(false)),
                worker: None,
            };
            if exists {
                // Grant the existing process foreground permission while this launch
                // still has the permission inherited from the user's click.
                let mut hwnd = HWND::default();
                if activate_window {
                    let _ =
                        EnumWindows(Some(find_window), LPARAM((&mut hwnd as *mut HWND) as isize));
                }
                if !hwnd.0.is_null() {
                    let mut pid = 0;
                    GetWindowThreadProcessId(hwnd, Some(&mut pid));
                    let _ = AllowSetForegroundWindow(pid);
                    restore(hwnd);
                }
                // An auto-reset event keeps a launch during startup pending until
                // the first process installs its listener. No polling or timeout.
                SetEvent(event)?;
                Ok(None)
            } else {
                Ok(Some(instance))
            }
        }
    }

    pub fn listen(&mut self, sink: ExtEventSink) -> std::io::Result<()> {
        let event = self.event.0 as usize;
        let stopping = self.stopping.clone();
        self.worker = Some(
            thread::Builder::new()
                .name("instance-activation".into())
                .spawn(move || {
                    let event = HANDLE(event as *mut _);
                    while unsafe { WaitForSingleObject(event, INFINITE) } == WAIT_OBJECT_0 {
                        if stopping.load(Ordering::Acquire) {
                            break;
                        }
                        if sink.submit_command(ACTIVATE, (), Target::Global).is_err() {
                            break;
                        }
                    }
                })?,
        );
        Ok(())
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            unsafe {
                let _ = SetEvent(self.event);
            }
            let _ = worker.join();
        }
        unsafe {
            let _ = CloseHandle(self.event);
            let _ = CloseHandle(self.mutex);
        }
    }
}

unsafe extern "system" fn find_window(hwnd: HWND, parameter: LPARAM) -> BOOL {
    if !unsafe { GetPropW(hwnd, WINDOW_PROPERTY) }.0.is_null() {
        unsafe {
            *(parameter.0 as *mut HWND) = hwnd;
        }
        return BOOL(0);
    }
    BOOL(1)
}

unsafe fn restore(hwnd: HWND) {
    unsafe {
        let _ = ShowWindowAsync(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

pub fn mark_window(handle: &druid::WindowHandle) {
    if let RawWindowHandle::Win32(handle) = handle.raw_window_handle() {
        unsafe {
            if let Err(error) = SetPropW(
                HWND(handle.hwnd),
                WINDOW_PROPERTY,
                Some(HANDLE(std::ptr::dangling_mut())),
            ) {
                log::error!("Cannot mark Xpotify window for activation: {error}");
            }
        }
    }
}

pub fn restore_window() {
    unsafe {
        let mut hwnd = HWND::default();
        let _ = EnumWindows(Some(find_window), LPARAM((&mut hwnd as *mut HWND) as isize));
        if !hwnd.0.is_null() {
            restore(hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_launch_is_signalled_even_before_listener_and_lock_releases_on_exit() {
        let name = format!("Local\\xpotify-instance-test-{}", std::process::id());
        let first = SingleInstance::acquire_named(&name, false)
            .unwrap()
            .unwrap();
        assert!(SingleInstance::acquire_named(&name, false)
            .unwrap()
            .is_none());
        assert_eq!(
            unsafe { WaitForSingleObject(first.event, 0) },
            WAIT_OBJECT_0
        );
        // The event is consumed, so it cannot repeatedly reactivate the window.
        assert_ne!(
            unsafe { WaitForSingleObject(first.event, 0) },
            WAIT_OBJECT_0
        );
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "single_instance::tests::secondary_process_checks_existing_instance",
            ])
            .env("XPOTIFY_TEST_INSTANCE_LOCK", &name)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stdout)
        );
        assert_eq!(
            unsafe { WaitForSingleObject(first.event, 0) },
            WAIT_OBJECT_0
        );
        drop(first);
        assert!(SingleInstance::acquire_named(&name, false)
            .unwrap()
            .is_some());
    }

    #[test]
    fn secondary_process_checks_existing_instance() {
        if let Ok(name) = std::env::var("XPOTIFY_TEST_INSTANCE_LOCK") {
            assert!(SingleInstance::acquire_named(&name, false)
                .unwrap()
                .is_none());
        }
    }
}
