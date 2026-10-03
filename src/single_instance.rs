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
    pub fn acquire() -> Result<Option<Guard>> {
        let name: Vec<u16> = std::ffi::OsStr::new(MUTEX_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            bail!("cannot create single-instance mutex");
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
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
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn focus_existing() -> bool {
    let Some(hwnd) = find_own_window() else {
        return false;
    };
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd) != 0
    }
}

struct Search {
    current_exe: String,
    found: HWND,
    loose: bool,
}

fn find_own_window() -> Option<HWND> {
    let current_exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    for loose in [false, true] {
        let mut search = Search {
            current_exe: current_exe.clone(),
            found: std::ptr::null_mut(),
            loose,
        };
        unsafe {
            EnumWindows(Some(enum_cb), &mut search as *mut Search as LPARAM);
        }
        if !search.found.is_null() {
            return Some(search.found);
        }
    }
    None
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
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
    let ok = if search.loose {
        exe_matches_loose(pid)
    } else {
        exe_matches(pid, &search.current_exe)
    };
    if !ok {
        return TRUE;
    }
    search.found = hwnd;
    FALSE
}

fn exe_matches(pid: u32, current_exe: &str) -> bool {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        return false;
    }
    let other = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
    !current_exe.is_empty() && other == current_exe
}

fn exe_matches_loose(pid: u32) -> bool {
    let Some(name) = process_file_name(pid) else {
        return false;
    };
    name.contains("wintheme") || name.contains("themeauto") || name.contains("win-theme")
}

fn process_file_name(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        return None;
    }
    let full = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
    let name = full.rsplit(['\\', '/']).next().unwrap_or(&full).to_string();
    Some(name)
}
