#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod config;
mod geo;
mod schedule;
mod single_instance;
mod sun;
mod theme;
mod themes;
mod tray;
mod update;

use chrono::{Local, NaiveTime};
use config::{Config, Mode};
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};
use theme::Theme;
use tray_icon::{menu::MenuEvent, TrayIconEvent};

slint::include_modules!();

struct State {
    cfg: Config,
    themes: Vec<themes::ThemeEntry>,
    last_scheduled: Option<Theme>,
    last_titlebar_sys: Option<Theme>,
    last_wallpaper: Option<PathBuf>,
}

type Shared = Rc<RefCell<State>>;

fn main() -> anyhow::Result<()> {
    // Held for the whole process lifetime. `None` means either another
    // instance is running or the mutex itself failed — step aside only
    // when the running window is actually found.
    let guard = single_instance::Guard::acquire().ok().flatten();
    if guard.is_none() && single_instance::focus_existing() {
        return Ok(());
    }
    let _guard = guard;
    if let Err(e) = run() {
        show_fatal(&format!("WinThemeAuto: {e:#}"));
        return Err(e);
    }
    Ok(())
}

fn show_fatal(text: &str) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let w: Vec<u16> = std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let t: Vec<u16> = "WinThemeAuto\0".encode_utf16().collect();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), w.as_ptr(), t.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

fn run() -> anyhow::Result<()> {
    let start_hidden = std::env::args().any(|a| a == "--tray");
    let state: Shared = Rc::new(RefCell::new(State {
        cfg: Config::load(),
        themes: themes::enumerate(),
        last_scheduled: None,
        last_titlebar_sys: None,
        last_wallpaper: None,
    }));

    let ui = MainWindow::new()?;
    load_into_ui(&ui, &state.borrow().cfg, &state.borrow().themes);
    tick(&ui, &state);

    ui.window().on_close_requested(|| CloseRequestResponse::HideWindow);

    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_toggle_theme(move || {
            if let Some(ui) = w.upgrade() {
                toggle(&ui, &s);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_settings_changed(move || {
            if let Some(ui) = w.upgrade() {
                apply_settings(&ui, &s);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_detect_location(move || {
            if let Some(ui) = w.upgrade() {
                detect_location(&ui);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_check_updates(move || {
            if let Some(ui) = w.upgrade() {
                check_updates(&ui);
            }
        });
    }

    let schedule_timer = Timer::default();
    {
        let (w, s) = (ui.as_weak(), state.clone());
        schedule_timer.start(TimerMode::Repeated, Duration::from_secs(5), move || {
            if let Some(ui) = w.upgrade() {
                tick(&ui, &s);
            }
        });
    }

    let tray = tray::create()?;
    let tray_timer = Timer::default();
    {
        let (w, s) = (ui.as_weak(), state.clone());
        let (open_id, toggle_id, quit_id) =
            (tray.open_id.clone(), tray.toggle_id.clone(), tray.quit_id.clone());

        tray_timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
            while let Ok(ev) = MenuEvent::receiver().try_recv() {
                if let Some(ui) = w.upgrade() {
                    if ev.id == open_id {
                        show_window(&ui, &s);
                    } else if ev.id == toggle_id {
                        toggle(&ui, &s);
                    } else if ev.id == quit_id {
                        let _ = slint::quit_event_loop();
                    }
                }
            }
            while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
                if matches!(ev, TrayIconEvent::DoubleClick { .. }) {
                    if let Some(ui) = w.upgrade() {
                        show_window(&ui, &s);
                    }
                }
            }
        });
    }

    if !start_hidden {
        show_window(&ui, &state);
    }

    slint::run_event_loop_until_quit()?;

    drop(tray);
    Ok(())
}

fn show_window(ui: &MainWindow, state: &Shared) {
    let _ = ui.show();
    force_light_titlebar(ui);
    state.borrow_mut().last_titlebar_sys = Some(theme::current_system());
}

fn force_light_titlebar(ui: &MainWindow) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};

    let wh = ui.window().window_handle();
    let Ok(handle) = wh.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(h) = handle.as_raw() else {
        return;
    };
    let hwnd = h.hwnd.get() as *mut std::ffi::c_void;
    let value: i32 = 0;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &value as *const _ as _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

fn load_into_ui(ui: &MainWindow, cfg: &Config, themes: &[themes::ThemeEntry]) {
    ui.set_auto_enabled(cfg.auto_enabled);
    ui.set_mode_index(if cfg.mode == Mode::Sun { 1 } else { 0 });
    ui.set_light_at(cfg.light_at.format("%H:%M").to_string().into());
    ui.set_dark_at(cfg.dark_at.format("%H:%M").to_string().into());
    if cfg.lat != 0.0 || cfg.lon != 0.0 {
        ui.set_lat(cfg.lat.to_string().into());
        ui.set_lon(cfg.lon.to_string().into());
    }
    ui.set_light_offset(cfg.light_offset_min.to_string().into());
    ui.set_dark_offset(cfg.dark_offset_min.to_string().into());
    ui.set_change_apps(cfg.change_apps);
    ui.set_change_system(cfg.change_system);
    ui.set_autostart(autostart::is_enabled());
    ui.set_app_version(update::current_version().into());

    let names: Vec<SharedString> = std::iter::once("(System default)".into())
        .chain(themes.iter().map(|t| themes::display_name(t).into()))
        .collect();
    ui.set_light_themes(ModelRc::new(VecModel::from(names.clone())));
    ui.set_dark_themes(ModelRc::new(VecModel::from(names)));
    ui.set_light_theme_index(theme_index(themes, &cfg.light_theme));
    ui.set_dark_theme_index(theme_index(themes, &cfg.dark_theme));
}

/// ComboBox index for a stored theme path (0 = flags only).
fn theme_index(themes: &[themes::ThemeEntry], path: &str) -> i32 {
    if path.is_empty() {
        return 0;
    }
    themes
        .iter()
        .position(|t| t.path.to_string_lossy() == path)
        .map(|i| i as i32 + 1)
        .unwrap_or(0)
}

/// Theme path for a ComboBox index ("" = flags only).
fn theme_path(themes: &[themes::ThemeEntry], index: i32) -> String {
    if index <= 0 {
        return String::new();
    }
    themes
        .get(index as usize - 1)
        .map(|t| t.path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn tick(ui: &MainWindow, state: &Shared) {
    let (apps_theme, sys_theme) = theme::current_pair();
    {
        let mut st = state.borrow_mut();
        let now = Local::now();

        if st.cfg.auto_enabled {
            let want = schedule::desired_theme(&st.cfg, now);
            if st.last_scheduled != Some(want) {
                let need = (st.cfg.change_apps && apps_theme != want)
                    || (st.cfg.change_system && sys_theme != want);
                if need {
                    match theme::apply(want, st.cfg.change_apps, st.cfg.change_system) {
                        Ok(()) => st.last_scheduled = Some(want),
                        Err(e) => {
                            ui.set_status(format!("Failed to switch theme: {e}").into());
                        }
                    }
                } else {
                    st.last_scheduled = Some(want);
                }
            }
            apply_wallpaper(ui, &mut st, want);
        }

        let info = if st.cfg.mode == Mode::Sun {
            schedule::sun_info(&st.cfg, now.date_naive())
        } else {
            String::new()
        };
        let next = schedule::next_switch_info(&st.cfg, now);
        ui.set_sun_info(info.into());
        ui.set_next_switch(next.into());

        if st.last_titlebar_sys != Some(sys_theme) {
            st.last_titlebar_sys = Some(sys_theme);
            force_light_titlebar(ui);
        }

        let shown =
            theme::display_theme(apps_theme, sys_theme, st.cfg.change_apps, st.cfg.change_system);
        ui.set_is_dark(shown == Theme::Dark);
    }
}

/// Swap the wallpaper to the one from the full theme configured for
/// `want`, if any. Silent when unconfigured or already applied.
fn apply_wallpaper(ui: &MainWindow, st: &mut State, want: Theme) {
    let configured = if want == Theme::Light {
        st.cfg.light_theme.clone()
    } else {
        st.cfg.dark_theme.clone()
    };
    if configured.is_empty() {
        return;
    }
    let wallpaper = st
        .themes
        .iter()
        .find(|t| t.path.to_string_lossy() == configured)
        .and_then(|t| t.wallpaper.clone());
    let Some(path) = wallpaper else {
        return;
    };
    if st.last_wallpaper.as_ref() == Some(&path) {
        return;
    }
    if !path.exists() {
        return;
    }
    match theme::set_wallpaper(&path) {
        Ok(()) => st.last_wallpaper = Some(path),
        Err(e) => ui.set_status(format!("Wallpaper: {e}").into()),
    }
}

fn toggle(ui: &MainWindow, state: &Shared) {
    let (apps, system) = {
        let st = state.borrow();
        (st.cfg.change_apps, st.cfg.change_system)
    };
    if !apps && !system {
        ui.set_status("Nothing to toggle: enable Apps or System".into());
        return;
    }
    let new = theme::effective_current(apps, system).toggled();
    match theme::apply(new, apps, system) {
        Ok(()) => {
            ui.set_status("".into());
            ui.set_is_dark(new == Theme::Dark);
            apply_wallpaper(ui, &mut state.borrow_mut(), new);
        }
        Err(e) => {
            ui.set_status(format!("Error: {e}").into());
            ui.set_is_dark(theme::effective_current(apps, system) == Theme::Dark);
        }
    }
}

fn detect_location(ui: &MainWindow) {
    ui.set_locating(true);
    ui.set_geo_hint("Locating…".into());

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let result = geo::detect_by_ip().map_err(|e| e.to_string());

        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.set_locating(false);
            match result {
                Ok(loc) => {
                    ui.set_lat(loc.lat.to_string().into());
                    ui.set_lon(loc.lon.to_string().into());
                    let place = if loc.place.is_empty() {
                        String::new()
                    } else {
                        format!("{}: ", loc.place)
                    };
                    ui.set_geo_hint(
                        format!(
                            "Found via IP — {place}{:.4}, {:.4}. Click Apply.",
                            loc.lat, loc.lon
                        )
                        .into(),
                    );
                }
                Err(e) => {
                    ui.set_geo_hint(
                        format!("Location failed: {e}. Enter coordinates manually.").into(),
                    );
                }
            }
        });
    });
}

fn check_updates(ui: &MainWindow) {
    if ui.get_checking_update() {
        return;
    }
    ui.set_checking_update(true);
    ui.set_update_info("Checking for updates…".into());

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let info = update::check();
        let set = |text: &str, done: bool| {
            let text = text.to_string();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_update_info(text.into());
                if done {
                    ui.set_checking_update(false);
                }
            });
        };

        let info = match info {
            Ok(i) => i,
            Err(e) => {
                set(&format!("Update check failed: {e}"), true);
                return;
            }
        };
        if !info.is_newer {
            set(
                &format!("You have the latest version (v{}).", info.latest),
                true,
            );
            return;
        }

        set(&format!("Found v{} — downloading…", info.latest), false);
        let new_exe = match update::download(&info.download_url) {
            Ok(p) => p,
            Err(e) => {
                set(&format!("Download failed: {e}"), true);
                return;
            }
        };

        set(
            &format!("Installing v{} — the app will restart…", info.latest),
            false,
        );
        match update::self_install(&new_exe) {
            Ok(()) => {
                let _ = weak.upgrade_in_event_loop(|_| {
                    let _ = slint::quit_event_loop();
                });
            }
            Err(e) => set(&format!("Install failed: {e}"), true),
        }
    });
}

fn apply_settings(ui: &MainWindow, state: &Shared) {
    let parse_time = |s: slint::SharedString| NaiveTime::parse_from_str(s.trim(), "%H:%M");
    let parse_f = |s: slint::SharedString| s.trim().replace(',', ".").parse::<f64>().ok();
    let parse_off = |s: slint::SharedString| {
        let s = s.trim().to_string();
        if s.is_empty() {
            Some(0)
        } else {
            s.parse::<i32>().ok().filter(|v| (-180..=180).contains(v))
        }
    };

    let (Ok(light_at), Ok(dark_at)) = (parse_time(ui.get_light_at()), parse_time(ui.get_dark_at()))
    else {
        ui.set_status("Time must be HH:MM, e.g. 07:30".into());
        return;
    };
    if light_at == dark_at {
        ui.set_status("Light and dark times must differ".into());
        return;
    }

    let (Some(light_offset_min), Some(dark_offset_min)) =
        (parse_off(ui.get_light_offset()), parse_off(ui.get_dark_offset()))
    else {
        ui.set_status("Offset must be an integer from -180 to 180".into());
        return;
    };

    let mode = if ui.get_mode_index() == 1 { Mode::Sun } else { Mode::Fixed };
    let (light_theme, dark_theme) = {
        let st = state.borrow();
        (
            theme_path(&st.themes, ui.get_light_theme_index()),
            theme_path(&st.themes, ui.get_dark_theme_index()),
        )
    };
    let lat = parse_f(ui.get_lat()).filter(|v| (-90.0..=90.0).contains(v));
    let lon = parse_f(ui.get_lon()).filter(|v| (-180.0..=180.0).contains(v));

    if mode == Mode::Sun {
        match (lat, lon) {
            (Some(la), Some(lo)) if la != 0.0 || lo != 0.0 => {}
            _ => {
                ui.set_status(
                    "Enter coordinates: latitude -90..90, longitude -180..180".into(),
                );
                return;
            }
        }
    }

    let new_cfg = {
        let st = state.borrow();
        let (old_lat, old_lon) = (st.cfg.lat, st.cfg.lon);
        Config {
            auto_enabled: ui.get_auto_enabled(),
            mode,
            light_at,
            dark_at,
            lat: lat.unwrap_or(old_lat),
            lon: lon.unwrap_or(old_lon),
            light_offset_min,
            dark_offset_min,
            change_apps: ui.get_change_apps(),
            change_system: ui.get_change_system(),
            light_theme,
            dark_theme,
        }
    };
    if let Err(e) = new_cfg.save() {
        ui.set_status(format!("Failed to save settings: {e}").into());
        return;
    }
    {
        let mut st = state.borrow_mut();
        st.cfg = new_cfg;
        st.last_scheduled = None;
    }

    if let Err(e) = autostart::set(ui.get_autostart()) {
        ui.set_autostart(autostart::is_enabled());
        ui.set_status(format!("Autostart: {e}").into());
        return;
    }

    ui.set_status("".into());
    tick(ui, state);
}
