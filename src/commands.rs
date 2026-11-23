use crate::theme::Theme;
use anyhow::{bail, Result};
use std::{cell::RefCell, io, ptr::null_mut};
use windows_sys::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW, RegisterClassW,
            SendMessageTimeoutW, HWND_MESSAGE, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_APP, WNDCLASSW,
        },
    },
};

const SWITCH_THEME: u32 = WM_APP + 1;
const WINDOW_CLASS: windows_sys::core::PCWSTR = w!("WinThemeAuto.Commands.1");

type Handler = Box<dyn Fn(Option<Theme>) -> Option<Theme>>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = RefCell::new(None);
}

pub struct Receiver(HWND);

impl Receiver {
    pub fn create(handler: impl Fn(Option<Theme>) -> Option<Theme> + 'static) -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: WINDOW_CLASS,
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        let window = unsafe {
            CreateWindowExW(
                0,
                WINDOW_CLASS,
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                instance,
                std::ptr::null(),
            )
        };
        if window.is_null() {
            return Err(io::Error::last_os_error().into());
        }
        HANDLER.with(|slot| slot.replace(Some(Box::new(handler))));
        Ok(Self(window))
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        HANDLER.with(|slot| slot.replace(None));
        unsafe { DestroyWindow(self.0) };
    }
}

pub fn send(target: Option<Theme>) -> Result<Option<Theme>> {
    let window = unsafe { FindWindowExW(HWND_MESSAGE, null_mut(), WINDOW_CLASS, std::ptr::null()) };
    if window.is_null() {
        return Ok(None);
    }

    send_to(window, target).map(Some)
}

fn send_to(window: HWND, target: Option<Theme>) -> Result<Theme> {
    let mut reply = 0;
    let delivered = unsafe {
        SendMessageTimeoutW(
            window,
            SWITCH_THEME,
            encode(target),
            0,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            10_000,
            &mut reply,
        )
    };
    if delivered == 0 {
        bail!("The running instance did not respond to the theme command");
    }
    match reply {
        1 => Ok(Theme::Light),
        2 => Ok(Theme::Dark),
        _ => bail!("The running instance could not switch the theme. Check its settings and logs."),
    }
}

fn encode(theme: Option<Theme>) -> usize {
    match theme {
        None => 0,
        Some(Theme::Light) => 1,
        Some(Theme::Dark) => 2,
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg != SWITCH_THEME {
        return unsafe { DefWindowProcW(window, msg, wparam, lparam) };
    }
    let target = match wparam {
        0 => None,
        1 => Some(Theme::Light),
        2 => Some(Theme::Dark),
        _ => return 0,
    };
    HANDLER.with(|slot| {
        // Windows can dispatch sent messages inside a COM call. Reject reentry.
        let Ok(handler) = slot.try_borrow_mut() else {
            return 0;
        };
        encode(handler.as_ref().and_then(|run| run(target))) as LRESULT
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_NOREMOVE};

    #[test]
    fn routes_commands_to_receiver_thread_and_returns_applied_theme() {
        let owner = std::thread::current().id();
        let receiver = Receiver::create(move |target| {
            (std::thread::current().id() == owner).then_some(target.unwrap_or(Theme::Dark))
        })
        .unwrap();

        let window = receiver.0 as usize;
        let client = std::thread::spawn(move || {
            let window = window as HWND;
            assert_eq!(send_to(window, Some(Theme::Light)).unwrap(), Theme::Light);
            assert_eq!(send_to(window, None).unwrap(), Theme::Dark);
        });

        let deadline = Instant::now() + Duration::from_secs(3);
        while !client.is_finished() && Instant::now() < deadline {
            let mut msg = MSG::default();
            // PeekMessage dispatches synchronous messages sent by the client.
            unsafe { PeekMessageW(&mut msg, receiver.0, 0, 0, PM_NOREMOVE) };
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            client.is_finished(),
            "receiver did not process the commands"
        );
        client.join().unwrap();
    }
}
