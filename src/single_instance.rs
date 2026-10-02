//! Single-instance guard.
//!
//! The first launch holds a named mutex for the whole process lifetime.
//! A second launch finds the already-running window, brings it forward
//! and exits — no duplicate trays or timers.

use anyhow::{bail, Result};
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

const MUTEX_NAME: &str = r"Local\WinThemeAuto-SingleInstance";
const WINDOW_TITLE: &str = "WinThemeAuto";

pub struct Guard(HANDLE);

impl Guard {
    /// Returns `Ok(None)` when another instance is already running.
    pub fn acquire() -> Result<Option<Guard>> {
        let name: Vec<u16> = std::ffi::OsStr::new(MUTEX_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: null security attributes, unnamed owner — returns a valid
        // handle or null on failure.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            bail!("cannot create single-instance mutex");
        }
        // SAFETY: reading the thread's last-error code.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            // SAFETY: handle is valid and owned by us.
            unsafe {
                CloseHandle(handle);
            }
            Ok(None)
        } else {
            Ok(Some(Guard(handle)))
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: handle is valid and owned by us.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Bring the already-running window forward. Best effort — returns false
/// when the window cannot be found.
pub fn focus_existing() -> bool {
    let title: Vec<u16> = std::ffi::OsStr::new(WINDOW_TITLE)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: null class matches any top-level window with this title.
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd.is_null() {
        return false;
    }
    // SAFETY: hwnd came from FindWindowW and is still checked for null.
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd) != 0
    }
}
