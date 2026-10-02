use anyhow::Result;
use serde::{Deserialize, Serialize};
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, PostMessageW, SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG,
    WM_SETTINGCHANGE, WM_THEMECHANGED,
};
use windows_sys::w;
use winreg::{enums::*, RegKey};

const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn toggled(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    fn reg_value(self) -> u32 {
        match self {
            Theme::Light => 1,
            Theme::Dark => 0,
        }
    }
}

pub fn current_pair() -> (Theme, Theme) {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(KEY).ok();
    let get = |name: &str| {
        key.as_ref()
            .and_then(|k| k.get_value::<u32, _>(name).ok())
            .map(|v| if v == 0 { Theme::Dark } else { Theme::Light })
            .unwrap_or(Theme::Light)
    };
    (get("AppsUseLightTheme"), get("SystemUsesLightTheme"))
}

pub fn current() -> Theme {
    current_pair().0
}

pub fn current_system() -> Theme {
    current_pair().1
}

pub fn effective_current(apps: bool, system: bool) -> Theme {
    if system && !apps {
        current_system()
    } else {
        current()
    }
}

pub fn display_theme(apps_theme: Theme, sys_theme: Theme, change_apps: bool, change_system: bool) -> Theme {
    if change_apps {
        apps_theme
    } else if change_system {
        sys_theme
    } else {
        apps_theme
    }
}

pub fn apply(theme: Theme, apps: bool, system: bool) -> Result<()> {
    if !apps && !system {
        return Ok(());
    }
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(KEY)?;
    let v = theme.reg_value();
    if apps {
        key.set_value("AppsUseLightTheme", &v)?;
    }
    if system {
        key.set_value("SystemUsesLightTheme", &v)?;
    }
    std::thread::spawn(|| {
        broadcast_change();
        refresh_taskbars();
        // A second broadcast shortly after: some apps only pick up the
        // change once their message queue has settled.
        std::thread::sleep(std::time::Duration::from_millis(200));
        broadcast_change();
    });
    Ok(())
}


fn broadcast_change() {
    let param: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            param.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            1000,
            std::ptr::null_mut(),
        );
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_THEMECHANGED,
            0,
            0,
            SMTO_ABORTIFHUNG,
            1000,
            std::ptr::null_mut(),
        );
    }
}

fn refresh_taskbars() {
    const CLASSES: [*const u16; 2] = [w!("Shell_TrayWnd"), w!("Shell_SecondaryTrayWnd")];
    unsafe {
        for class in CLASSES {
            let mut prev = std::ptr::null_mut();
            loop {
                let hwnd = FindWindowExW(std::ptr::null_mut(), prev, class, std::ptr::null());
                if hwnd.is_null() {
                    break;
                }
                PostMessageW(hwnd, WM_THEMECHANGED, 0, 0);
                InvalidateRect(hwnd, std::ptr::null(), 1);
                prev = hwnd;
            }
        }
    }
}
