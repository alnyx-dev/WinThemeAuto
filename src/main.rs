#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod config;
mod geo;
mod schedule;
mod sun;
mod theme;
mod tray;

use chrono::{Local, NaiveTime};
use config::{Config, Mode};
use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};
use std::{cell::RefCell, rc::Rc, time::Duration};
use theme::Theme;
use tray_icon::{menu::MenuEvent, TrayIconEvent};

slint::include_modules!();

struct State {
    cfg: Config,
    last_scheduled: Option<Theme>,
    last_titlebar_sys: Option<Theme>,
}

type Shared = Rc<RefCell<State>>;

fn main() -> anyhow::Result<()> {
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
        last_scheduled: None,
        last_titlebar_sys: None,
    }));

    let ui = MainWindow::new()?;
    load_into_ui(&ui, &state.borrow().cfg);
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
        ui.on_size_changed(move || {
            if let Some(ui) = w.upgrade() {
                fit_height(&ui);
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

fn fit_height(ui: &MainWindow) {
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(100), move || {
        if let Some(ui) = weak.upgrade() {
            let s = ui.window().size();
            ui.window().set_size(slint::WindowSize::Physical(slint::PhysicalSize::new(
                s.width, 1,
            )));
        }
    });
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

fn load_into_ui(ui: &MainWindow, cfg: &Config) {
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
}

fn tick(ui: &MainWindow, state: &Shared) {
    let (apps_theme, sys_theme) = theme::current_pair();
    let mut shrink = false;
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
        }

        let info = if st.cfg.mode == Mode::Sun {
            schedule::sun_info(&st.cfg, now.date_naive())
        } else {
            String::new()
        };
        let next = schedule::next_switch_info(&st.cfg, now);
        if (ui.get_sun_info().as_str().is_empty() != info.is_empty())
            || (ui.get_next_switch().as_str().is_empty() != next.is_empty())
        {
            shrink = true;
        }
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
    if shrink {
        fit_height(ui);
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

    let mode_changed = state.borrow().cfg.mode != mode;
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

    let had_status = !ui.get_status().is_empty();
    ui.set_status("".into());
    tick(ui, state);
    if mode_changed || had_status {
        fit_height(ui);
    }
}
