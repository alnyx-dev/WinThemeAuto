//! Single-instance guard.
//!
//! The first launch holds a named mutex for the whole process lifetime.
//! A second launch finds the already-running window, brings it forward
//! and exits — no duplicate trays or timers.

use anyhow::{bail, Result};
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, SetForegroundWindow, ShowWindow,
    SW_RESTORE,
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
///
/// Matches by window title *and* owning executable: a plain `FindWindowW`
/// would steal focus from any unrelated window (browser tab, Explorer)
/// that happens to share the title.
pub fn focus_existing() -> bool {
    let Some(hwnd) = find_own_window() else {
        return false;
    };
    // SAFETY: hwnd was just enumerated and validated.
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd) != 0
    }
}

struct Search {
    current_exe: String,
    found: HWND,
}

/// Enumerate top-level windows, keeping the first `WinThemeAuto` title
/// whose process image matches our own exe.
fn find_own_window() -> Option<HWND> {
    let current_exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut search = Search {
        current_exe,
        found: std::ptr::null_mut(),
    };
    // SAFETY: callback receives a valid `Search` pointer for the call duration.
    unsafe {
        EnumWindows(Some(enum_cb), &mut search as *mut Search as LPARAM);
    }
    if search.found.is_null() {
        None
    } else {
        Some(search.found)
    }
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
    // TRUE (1) = continue, FALSE (0) = stop.
    const TRUE: i32 = 1;
    const FALSE: i32 = 0;
    let search = &mut *(lparam as *mut Search);
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len <= 0 {
        return TRUE;
    }
    let title: Vec<u16> = std::ffi::OsStr::new(WINDOW_TITLE).encode_wide().collect();
    if (len as usize) != title.len() || &buf[..len as usize] != title.as_slice() {
        return TRUE;
    }
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 || pid == std::process::id() {
        return TRUE;
    }
    if !exe_matches(pid, &search.current_exe) {
        return TRUE;
    }
    search.found = hwnd;
    FALSE
}

/// `true` when `pid` runs the same exe file as us (case-insensitive).
fn exe_matches(pid: u32, current_exe: &str) -> bool {
    // SAFETY: plain PID lookup; handle closed below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    // SAFETY: `handle` is valid, buffer sized by `size`.
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    // SAFETY: handle owned by us.
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        return false;
    }
    let other = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
    !current_exe.is_empty() && other == current_exe
}
