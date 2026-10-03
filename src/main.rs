#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accent;
mod autostart;
mod cli;
mod config;
mod geo;
mod i18n;
mod schedule;
mod single_instance;
mod sun;
mod theme;
mod themes;
mod tray;
mod update;

use chrono::{Local, NaiveTime};
use config::{Config, Mode};
use i18n::Lang;
use slint::{
    CloseRequestResponse, ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc, time::Duration};
use theme::Theme;
use tray_icon::{menu::MenuEvent, TrayIconEvent};

slint::include_modules!();

struct State {
    cfg: Config,
    themes: Vec<themes::ThemeEntry>,
    last_scheduled: Option<Theme>,
    manual_hold: Option<Theme>,
    update_cancel: Arc<std::sync::atomic::AtomicBool>,
    last_titlebar_sys: Option<Theme>,
    last_wallpaper: Option<PathBuf>,
    last_accent: Option<u32>,
    last_tray_tooltip: Option<String>,
    last_tray_dark: Option<bool>,
}

type Shared = Rc<RefCell<State>>;

fn main() -> anyhow::Result<()> {
    match cli::parse(std::env::args()) {
        cli::Action::Ui { start_hidden } => run_ui(start_hidden),
        cli::Action::Toggle => run_cli(None),
        cli::Action::Light => run_cli(Some(Theme::Light)),
        cli::Action::Dark => run_cli(Some(Theme::Dark)),
        cli::Action::Status => run_status(),
        cli::Action::Help => {
            attach_console();
            print!("{}", cli::HELP);
            Ok(())
        }
        cli::Action::Invalid => {
            attach_console();
            eprintln!("Unknown flag. {}", cli::HELP);
            std::process::exit(2);
        }
    }
}

fn attach_console() {
    use windows_sys::Win32::System::Console::AttachConsole;
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn run_cli(target: Option<Theme>) -> anyhow::Result<()> {
    attach_console();
    let cfg = Config::load();
    if !cfg.change_apps && !cfg.change_system {
        eprintln!("Nothing to switch: enable Apps or System in the app first");
        std::process::exit(1);
    }
    let want = target
        .unwrap_or_else(|| theme::effective_current(cfg.change_apps, cfg.change_system).toggled());
    if let Err(e) = theme::apply(want, cfg.change_apps, cfg.change_system) {
        eprintln!("Failed to switch theme: {e}");
        std::process::exit(1);
    }
    let themes = themes::enumerate();
    if let Some(path) = resolve_wallpaper_path(&cfg, &themes, want) {
        if path.exists() {
            if let Err(e) = theme::set_wallpaper(&path) {
                eprintln!("Wallpaper: {e}");
            }
        }
    }
    if cfg.accent_enabled {
        let hex = if want == Theme::Light {
            cfg.light_accent.clone()
        } else {
            cfg.dark_accent.clone()
        };
        match accent::parse_hex(&hex) {
            Some(color) => {
                if let Err(e) = accent::apply(color) {
                    eprintln!("Accent: {e}");
                }
            }
            None => eprintln!("Accent: invalid hex {hex}"),
        }
    }
    println!("{}", if want == Theme::Dark { "dark" } else { "light" });
    if cfg.auto_enabled {
        if let Some(scheduled) = schedule::desired_theme(&cfg, Local::now()) {
            if scheduled != want {
                eprintln!(
                    "Note: auto-switch is on and wants {} вЂ” the GUI will revert this shortly. Use the window/tray Switch for a hold until the next switch.",
                    if scheduled == Theme::Dark {
                        "dark"
                    } else {
                        "light"
                    }
                );
            }
        }
    }
    Ok(())
}

fn run_status() -> anyhow::Result<()> {
    attach_console();
    let cfg = Config::load();
    let lang = Lang::from_code(&cfg.language);
    let s = i18n::ui(lang);
    let (apps_theme, sys_theme) = theme::current_pair();
    let shown = theme::display_theme(apps_theme, sys_theme, cfg.change_apps, cfg.change_system);
    let now = Local::now();
    let next = schedule::next_switch_info(&cfg, now);
    let head = if shown == Theme::Dark {
        s.dark_now
    } else {
        s.light_now
    };
    if next.is_empty() {
        println!("{head}");
    } else {
        println!("{head} вЂў {next}");
    }
    Ok(())
}

fn run_ui(start_hidden: bool) -> anyhow::Result<()> {
    let _guard = match single_instance::Guard::acquire() {
        Ok(Some(g)) => g,
        Ok(None) => {
            single_instance::focus_existing();
            return Ok(());
        }
        Err(e) => {
            show_fatal(&format!(
                "WinThemeAuto: single-instance guard failed: {e:#}"
            ));
            return Err(e);
        }
    };
    if let Err(e) = run_loop(start_hidden) {
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
        MessageBoxW(
            std::ptr::null_mut(),
            w.as_ptr(),
            t.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn run_loop(start_hidden: bool) -> anyhow::Result<()> {
    let state: Shared = Rc::new(RefCell::new(State {
        cfg: Config::load(),
        themes: themes::enumerate(),
        last_scheduled: None,
        manual_hold: None,
        update_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        last_titlebar_sys: None,
        last_wallpaper: None,
        last_accent: None,
        last_tray_tooltip: None,
        last_tray_dark: None,
    }));

    let ui = MainWindow::new()?;
    let tray: Rc<tray::Tray> = Rc::new({
        let cfg = state.borrow().cfg.clone();
        let lang = Lang::from_code(&cfg.language);
        let s = i18n::ui(lang);
        let (apps_theme, sys_theme) = theme::current_pair();
        let shown = theme::display_theme(apps_theme, sys_theme, cfg.change_apps, cfg.change_system);
        tray::create(
            &tray::TrayText {
                tooltip: "WinThemeAuto",
                open: s.tray_open,
                switch_to_light: s.to_light,
                switch_to_dark: s.to_dark,
                quit: s.tray_exit,
            },
            shown == Theme::Dark,
        )?
    });
    load_into_ui(&ui, &state.borrow().cfg, &state.borrow().themes);
    tick(&ui, &state, &tray);

    {
        let w = ui.as_weak();
        let s = state.clone();
        ui.window().on_close_requested(move || {
            if s.borrow().cfg.close_hint_acked {
                return CloseRequestResponse::HideWindow;
            }
            if let Some(ui) = w.upgrade() {
                ui.set_close_never_ask(false);
                ui.set_show_close_dialog(true);
            }
            CloseRequestResponse::KeepWindowShown
        });
    }
    {
        let w = ui.as_weak();
        let s = state.clone();
        ui.on_close_minimize(move || {
            if let Some(ui) = w.upgrade() {
                if ui.get_close_never_ask() {
                    let mut st = s.borrow_mut();
                    st.cfg.close_hint_acked = true;
                    let _ = st.cfg.save();
                }
                ui.set_show_close_dialog(false);
                let _ = ui.hide();
            }
        });
    }
    {
        let w = ui.as_weak();
        let s = state.clone();
        ui.on_close_quit(move || {
            if let Some(ui) = w.upgrade() {
                if ui.get_close_never_ask() {
                    let mut st = s.borrow_mut();
                    st.cfg.close_hint_acked = true;
                    let _ = st.cfg.save();
                }
                ui.set_show_close_dialog(false);
            }
            let _ = slint::quit_event_loop();
        });
    }

    {
        let (w, s, t) = (ui.as_weak(), state.clone(), tray.clone());
        ui.on_toggle_theme(move || {
            if let Some(ui) = w.upgrade() {
                toggle(&ui, &s, &t);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_settings_changed(move || {
            if let Some(ui) = w.upgrade() {
                ui.set_dirty(true);
            }
        });
    }
    {
        let (w, s, t) = (ui.as_weak(), state.clone(), tray.clone());
        ui.on_apply_settings(move || {
            if let Some(ui) = w.upgrade() {
                apply_settings(&ui, &s, &t);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_language_changed(move || {
            if let Some(ui) = w.upgrade() {
                apply_language_instant(&ui, &s);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_autostart_changed(move || {
            if let Some(ui) = w.upgrade() {
                apply_autostart_instant(&ui);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_detect_location(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.borrow().cfg.language);
                detect_location(&ui, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_light_wallpaper(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.borrow().cfg.language);
                browse_wallpaper(&ui, true, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_dark_wallpaper(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.borrow().cfg.language);
                browse_wallpaper(&ui, false, lang);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_clear_light_wallpaper(move || {
            if let Some(ui) = w.upgrade() {
                ui.set_light_wallpaper("".into());
                ui.set_dirty(true);
                refresh_wallpaper_preview(&ui, true);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_clear_dark_wallpaper(move || {
            if let Some(ui) = w.upgrade() {
                ui.set_dark_wallpaper("".into());
                ui.set_dirty(true);
                refresh_wallpaper_preview(&ui, false);
            }
        });
    }
    {
        ui.on_open_releases(move || {
            update::open_releases_page();
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_check_updates(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.borrow().cfg.language);
                let cancel = s.borrow().update_cancel.clone();
                check_updates(&ui, lang, cancel);
            }
        });
    }
    {
        let s = state.clone();
        ui.on_cancel_update(move || {
            s.borrow()
                .update_cancel
                .store(true, std::sync::atomic::Ordering::SeqCst);
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_refresh_themes(move || {
            if let Some(ui) = w.upgrade() {
                refresh_themes(&ui, &s);
            }
        });
    }

    let schedule_timer = Timer::default();
    {
        let (w, s, t) = (ui.as_weak(), state.clone(), tray.clone());
        schedule_timer.start(TimerMode::Repeated, Duration::from_secs(5), move || {
            if let Some(ui) = w.upgrade() {
                tick(&ui, &s, &t);
            }
        });
    }

    let tray_timer = Timer::default();
    {
        let (w, s, t) = (ui.as_weak(), state.clone(), tray.clone());
        let (open_id, toggle_id, quit_id) = (
            tray.open_id.clone(),
            tray.toggle_id.clone(),
            tray.quit_id.clone(),
        );

        tray_timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
            while let Ok(ev) = MenuEvent::receiver().try_recv() {
                if let Some(ui) = w.upgrade() {
                    if ev.id == open_id {
                        show_window(&ui, &s);
                    } else if ev.id == toggle_id {
                        toggle(&ui, &s, &t);
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
    if let Some(lat) = cfg.lat {
        ui.set_lat(lat.to_string().into());
    }
    if let Some(lon) = cfg.lon {
        ui.set_lon(lon.to_string().into());
    }
    ui.set_light_offset(cfg.light_offset_min.to_string().into());
    ui.set_dark_offset(cfg.dark_offset_min.to_string().into());
    ui.set_change_apps(cfg.change_apps);
    ui.set_change_system(cfg.change_system);
    ui.set_autostart(autostart::is_enabled());
    ui.set_app_version(update::current_version().into());

    let lang = Lang::from_code(&cfg.language);
    apply_lang(ui, lang);
    ui.set_lang_index(lang.index());

    let names: Vec<SharedString> =
        std::iter::once(SharedString::from(i18n::ui(lang).system_default))
            .chain(themes.iter().map(|t| themes::display_name(t).into()))
            .collect();
    ui.set_light_themes(ModelRc::new(VecModel::from(names.clone())));
    ui.set_dark_themes(ModelRc::new(VecModel::from(names)));
    ui.set_light_theme_index(theme_index(themes, &cfg.light_theme));
    ui.set_dark_theme_index(theme_index(themes, &cfg.dark_theme));
    ui.set_light_wallpaper(cfg.light_wallpaper.clone().into());
    ui.set_dark_wallpaper(cfg.dark_wallpaper.clone().into());
    ui.set_accent_enabled(cfg.accent_enabled);
    ui.set_light_accent(cfg.light_accent.clone().into());
    ui.set_dark_accent(cfg.dark_accent.clone().into());
    ui.set_dirty(false);
    ui.set_show_close_dialog(false);
    ui.set_close_never_ask(false);
    refresh_wallpaper_preview(ui, true);
    refresh_wallpaper_preview(ui, false);
}

fn apply_lang(ui: &MainWindow, lang: Lang) {
    let s = i18n::ui(lang);
    ui.set_t_tab_auto(s.tab_auto.into());
    ui.set_t_tab_appearance(s.tab_appearance.into());
    ui.set_t_tab_settings(s.tab_settings.into());
    ui.set_t_auto_title(s.auto_title.into());
    ui.set_t_mode_time(s.mode_time.into());
    ui.set_t_mode_sun(s.mode_sun.into());
    ui.set_t_light_from(s.light_from.into());
    ui.set_t_dark_from(s.dark_from.into());
    ui.set_t_lat_ph(s.lat_ph.into());
    ui.set_t_lon_ph(s.lon_ph.into());
    ui.set_t_detect(s.detect.into());
    ui.set_t_light_pm(s.light_pm.into());
    ui.set_t_dark_pm(s.dark_pm.into());
    ui.set_t_apply_to(s.apply_to.into());
    ui.set_t_apps(s.apps.into());
    ui.set_t_system(s.system.into());
    ui.set_t_light_label(s.light_label.into());
    ui.set_t_dark_label(s.dark_label.into());
    ui.set_t_light_wp_ph(s.light_wp_ph.into());
    ui.set_t_dark_wp_ph(s.dark_wp_ph.into());
    ui.set_t_wp_hint(s.wp_hint.into());
    ui.set_t_accent_check(s.accent_check.into());
    ui.set_t_accent_hint(s.accent_hint.into());
    ui.set_t_rescan(s.rescan.into());
    ui.set_t_autostart(s.autostart.into());
    ui.set_t_check_updates(s.check_updates.into());
    ui.set_t_checking(s.checking.into());
    ui.set_t_apply(s.apply.into());
    ui.set_t_to_light(s.to_light.into());
    ui.set_t_to_dark(s.to_dark.into());
    ui.set_t_light_now(s.light_now.into());
    ui.set_t_dark_now(s.dark_now.into());
    ui.set_t_language(s.language.into());
    ui.set_t_releases(s.releases.into());
    ui.set_t_onboard(s.onboard.into());
    ui.set_t_cancel(s.cancel.into());
    ui.set_t_close_title(s.close_title.into());
    ui.set_t_close_body(s.close_body.into());
    ui.set_t_close_minimize(s.close_minimize.into());
    ui.set_t_close_quit(s.close_quit.into());
    ui.set_t_close_never(s.close_never.into());
}

fn update_lang_ui(ui: &MainWindow, state: &Shared, lang: Lang) {
    apply_lang(ui, lang);
    ui.set_lang_index(lang.index());
    ui.set_update_info("".into());
    ui.set_geo_hint("".into());
    let st = state.borrow();
    let names: Vec<SharedString> =
        std::iter::once(SharedString::from(i18n::ui(lang).system_default))
            .chain(st.themes.iter().map(|t| themes::display_name(t).into()))
            .collect();
    ui.set_light_themes(ModelRc::new(VecModel::from(names.clone())));
    ui.set_dark_themes(ModelRc::new(VecModel::from(names)));
}

fn apply_language_instant(ui: &MainWindow, state: &Shared) {
    let lang = Lang::from_index(ui.get_lang_index());
    {
        let mut st = state.borrow_mut();
        if Lang::from_code(&st.cfg.language) == lang {
        } else {
            let mut next = st.cfg.clone();
            next.language = lang.code().to_string();
            if let Err(e) = next.save() {
                ui.set_lang_index(Lang::from_code(&st.cfg.language).index());
                ui.set_status(i18n::msg(lang, "save_fail", &e.to_string()).into());
                return;
            }
            st.cfg = next;
        }
    }
    update_lang_ui(ui, state, lang);
}

fn apply_autostart_instant(ui: &MainWindow) {
    let want = ui.get_autostart();
    if let Err(e) = autostart::set(want) {
        ui.set_autostart(autostart::is_enabled());
        let lang = Lang::from_index(ui.get_lang_index());
        ui.set_status(i18n::msg(lang, "autostart_fail", &e.to_string()).into());
    }
}

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

fn theme_path(themes: &[themes::ThemeEntry], index: i32) -> String {
    if index <= 0 {
        return String::new();
    }
    themes
        .get(index as usize - 1)
        .map(|t| t.path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn refresh_themes(ui: &MainWindow, state: &Shared) {
    let (light_path, dark_path) = {
        let st = state.borrow();
        (
            theme_path(&st.themes, ui.get_light_theme_index()),
            theme_path(&st.themes, ui.get_dark_theme_index()),
        )
    };
    let fresh = themes::enumerate();
    let light_idx = theme_index(&fresh, &light_path);
    let dark_idx = theme_index(&fresh, &dark_path);
    {
        state.borrow_mut().themes = fresh;
    }
    let st = state.borrow();
    let lang = Lang::from_code(&st.cfg.language);
    let names: Vec<SharedString> =
        std::iter::once(SharedString::from(i18n::ui(lang).system_default))
            .chain(st.themes.iter().map(|t| themes::display_name(t).into()))
            .collect();
    ui.set_light_themes(ModelRc::new(VecModel::from(names.clone())));
    ui.set_dark_themes(ModelRc::new(VecModel::from(names)));
    ui.set_light_theme_index(light_idx);
    ui.set_dark_theme_index(dark_idx);
    if (light_idx == 0 && !light_path.is_empty()) || (dark_idx == 0 && !dark_path.is_empty()) {
        ui.set_status(i18n::msg(lang, "theme_gone", "").into());
    }
}

fn tick(ui: &MainWindow, state: &Shared, tray: &tray::Tray) {
    let (apps_theme, sys_theme) = theme::current_pair();
    {
        let mut st = state.borrow_mut();
        let now = Local::now();
        let lang = Lang::from_code(&st.cfg.language);

        if st.cfg.auto_enabled {
            if let Some(want) = schedule::desired_theme(&st.cfg, now) {
                if let Some(held) = st.manual_hold {
                    if want == held {
                        st.manual_hold = None;
                        ui.set_status("".into());
                    } else {
                        st.last_scheduled = Some(want);
                    }
                }
                let holding = st.manual_hold.is_some() && Some(want) != st.manual_hold;
                if !holding {
                    let need = (st.cfg.change_apps && apps_theme != want)
                        || (st.cfg.change_system && sys_theme != want);
                    if need {
                        match theme::apply(want, st.cfg.change_apps, st.cfg.change_system) {
                            Ok(()) => st.last_scheduled = Some(want),
                            Err(e) => {
                                ui.set_status(
                                    i18n::msg(lang, "switch_fail", &e.to_string()).into(),
                                );
                            }
                        }
                    } else {
                        st.last_scheduled = Some(want);
                    }
                    apply_wallpaper(ui, &mut st, want);
                    apply_accent(ui, &mut st, want);
                }
            } else {
                st.manual_hold = None;
            }
        } else {
            st.manual_hold = None;
        }

        let info = if st.cfg.mode == Mode::Sun {
            schedule::sun_info(&st.cfg, now.date_naive())
        } else {
            String::new()
        };
        let next = schedule::next_switch_info(&st.cfg, now);
        ui.set_sun_info(info.into());
        ui.set_next_switch(next.clone().into());

        let shown = theme::display_theme(
            apps_theme,
            sys_theme,
            st.cfg.change_apps,
            st.cfg.change_system,
        );
        let is_dark = shown == Theme::Dark;
        ui.set_is_dark(is_dark);

        if st.last_titlebar_sys != Some(sys_theme) {
            st.last_titlebar_sys = Some(sys_theme);
            force_light_titlebar(ui);
        }

        let s = i18n::ui(lang);
        let head = if is_dark { s.dark_now } else { s.light_now };
        let tooltip = if next.is_empty() {
            format!("WinThemeAuto вЂ” {head}")
        } else {
            format!("WinThemeAuto вЂ” {head} вЂў {next}")
        };
        if st.last_tray_tooltip.as_ref() != Some(&tooltip) || st.last_tray_dark != Some(is_dark) {
            st.last_tray_tooltip = Some(tooltip.clone());
            st.last_tray_dark = Some(is_dark);
            let s = i18n::ui(lang);
            tray.update(
                is_dark,
                &tray::TrayText {
                    tooltip: &tooltip,
                    open: s.tray_open,
                    switch_to_light: s.to_light,
                    switch_to_dark: s.to_dark,
                    quit: s.tray_exit,
                },
            );
        }
    }
}

fn apply_accent(ui: &MainWindow, st: &mut State, want: Theme) {
    if !st.cfg.accent_enabled {
        return;
    }
    let hex = if want == Theme::Light {
        st.cfg.light_accent.clone()
    } else {
        st.cfg.dark_accent.clone()
    };
    let Some(color) = accent::parse_hex(&hex) else {
        return;
    };
    if st.last_accent == Some(color) {
        return;
    }
    match accent::apply(color) {
        Ok(()) => st.last_accent = Some(color),
        Err(e) => {
            let lang = Lang::from_code(&st.cfg.language);
            ui.set_status(i18n::msg(lang, "accent", &e.to_string()).into());
        }
    }
}

fn apply_wallpaper(ui: &MainWindow, st: &mut State, want: Theme) {
    let Some(path) = resolve_wallpaper_path(&st.cfg, &st.themes, want) else {
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
        Err(e) => {
            let lang = Lang::from_code(&st.cfg.language);
            ui.set_status(i18n::msg(lang, "wallpaper", &e.to_string()).into());
        }
    }
}

fn resolve_wallpaper_path(
    cfg: &Config,
    themes: &[themes::ThemeEntry],
    want: Theme,
) -> Option<PathBuf> {
    let custom = if want == Theme::Light {
        &cfg.light_wallpaper
    } else {
        &cfg.dark_wallpaper
    };
    if !custom.is_empty() {
        return Some(PathBuf::from(custom));
    }
    let configured = if want == Theme::Light {
        &cfg.light_theme
    } else {
        &cfg.dark_theme
    };
    if configured.is_empty() {
        return None;
    }
    themes
        .iter()
        .find(|t| t.path.to_string_lossy() == *configured)
        .and_then(|t| t.wallpaper.clone())
}

fn toggle(ui: &MainWindow, state: &Shared, tray: &tray::Tray) {
    let (apps, system, lang, auto_enabled) = {
        let st = state.borrow();
        (
            st.cfg.change_apps,
            st.cfg.change_system,
            Lang::from_code(&st.cfg.language),
            st.cfg.auto_enabled,
        )
    };
    if !apps && !system {
        ui.set_status(i18n::msg(lang, "nothing_toggle", "").into());
        return;
    }
    let new = theme::effective_current(apps, system).toggled();
    match theme::apply(new, apps, system) {
        Ok(()) => {
            ui.set_is_dark(new == Theme::Dark);
            apply_wallpaper(ui, &mut state.borrow_mut(), new);
            apply_accent(ui, &mut state.borrow_mut(), new);
            if auto_enabled {
                let want = {
                    let st = state.borrow();
                    schedule::desired_theme(&st.cfg, Local::now())
                };
                match want {
                    Some(w) if w != new => {
                        state.borrow_mut().manual_hold = Some(new);
                        let next = schedule::next_switch_info(&state.borrow().cfg, Local::now());
                        let text = if next.is_empty() {
                            i18n::msg(lang, "held_plain", "")
                        } else {
                            i18n::msg(lang, "held", &next)
                        };
                        ui.set_status(text.into());
                    }
                    _ => {
                        state.borrow_mut().manual_hold = None;
                        ui.set_status("".into());
                    }
                }
            } else {
                state.borrow_mut().manual_hold = None;
                ui.set_status("".into());
            }
            tick(ui, state, tray);
        }
        Err(e) => {
            ui.set_status(i18n::msg(lang, "error", &e.to_string()).into());
            ui.set_is_dark(theme::effective_current(apps, system) == Theme::Dark);
        }
    }
}

fn detect_location(ui: &MainWindow, lang: Lang) {
    ui.set_locating(true);
    ui.set_geo_hint(i18n::msg(lang, "locating", "").into());

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let result = geo::detect_by_ip().map_err(|e| e.to_string());

        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.set_locating(false);
            match result {
                Ok(loc) => {
                    ui.set_lat(loc.lat.to_string().into());
                    ui.set_lon(loc.lon.to_string().into());
                    ui.set_dirty(true);
                    let place = if loc.place.is_empty() {
                        String::new()
                    } else {
                        format!("{}: ", loc.place)
                    };
                    let detail = format!("{place}{:.4}, {:.4}", loc.lat, loc.lon);
                    ui.set_geo_hint(i18n::msg(lang, "geo_ok", &detail).into());
                }
                Err(e) => {
                    ui.set_geo_hint(i18n::msg(lang, "geo_fail", &e).into());
                }
            }
        });
    });
}

fn refresh_wallpaper_preview(ui: &MainWindow, light: bool) {
    let path = if light {
        ui.get_light_wallpaper()
    } else {
        ui.get_dark_wallpaper()
    };
    let path = path.trim().to_string();
    let has = !path.is_empty() && PathBuf::from(&path).is_file();
    let img = if has {
        slint::Image::load_from_path(std::path::Path::new(&path)).unwrap_or_default()
    } else {
        slint::Image::default()
    };
    if light {
        ui.set_light_wp_preview(img);
        ui.set_light_wp_has(has);
    } else {
        ui.set_dark_wp_preview(img);
        ui.set_dark_wp_has(has);
    }
}

fn browse_wallpaper(ui: &MainWindow, light: bool, lang: Lang) {
    let title = i18n::msg(lang, if light { "pick_light" } else { "pick_dark" }, "");
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let picked = rfd::FileDialog::new()
            .set_title(title)
            .add_filter("Images", &["jpg", "jpeg", "png", "bmp"])
            .pick_file();

        let _ = weak.upgrade_in_event_loop(move |ui| {
            if let Some(path) = picked {
                let s: SharedString = path.to_string_lossy().into_owned().into();
                if light {
                    ui.set_light_wallpaper(s);
                } else {
                    ui.set_dark_wallpaper(s);
                }
                ui.set_dirty(true);
                refresh_wallpaper_preview(&ui, light);
            }
        });
    });
}

fn check_updates(ui: &MainWindow, lang: Lang, cancel: Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::atomic::Ordering;
    if ui.get_checking_update() {
        return;
    }
    cancel.store(false, Ordering::SeqCst);
    ui.set_checking_update(true);
    ui.set_update_info(i18n::msg(lang, "checking_updates", "").into());
    ui.set_update_ok(false);

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let info = update::check();
        let set = |text: &str, done: bool, ok: bool| {
            let text = text.to_string();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_update_info(text.into());
                ui.set_update_ok(ok);
                if done {
                    ui.set_checking_update(false);
                }
            });
        };

        let info = match info {
            Ok(i) => i,
            Err(e) => {
                set(&i18n::msg(lang, "check_fail", &e.to_string()), true, false);
                return;
            }
        };
        if !info.is_newer {
            set(&i18n::msg(lang, "latest", &info.latest), true, true);
            return;
        }

        set(
            &i18n::dl_progress(lang, &info.latest, 0, None),
            false,
            false,
        );
        let prog_weak = weak.clone();
        let prog_state = std::sync::Mutex::new((std::time::Instant::now(), 0u64));
        let latest = info.latest.clone();
        let on_progress = move |done: u64, total: Option<u64>| {
            let forward = {
                let mut st = prog_state.lock().unwrap_or_else(|e| e.into_inner());
                let msgs_pct = |d: u64, t: Option<u64>| match t {
                    Some(t) if t > 0 => d * 100 / t,
                    _ => u64::MAX,
                };
                let pct = msgs_pct(done, total);
                if pct != msgs_pct(st.1, total) || st.0.elapsed().as_millis() > 500 {
                    *st = (std::time::Instant::now(), done);
                    true
                } else {
                    false
                }
            };
            if forward {
                let text = i18n::dl_progress(lang, &latest, done, total);
                let _ = prog_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_update_info(text.into());
                });
            }
        };
        let new_exe = match update::download(&info, &cancel, on_progress) {
            Ok(p) => p,
            Err(e) => {
                let err = e.to_string();
                if err.contains("cancelled by user") {
                    set(&i18n::msg(lang, "cancelled", ""), true, false);
                } else {
                    set(&i18n::msg(lang, "dl_fail", &err), true, false);
                }
                return;
            }
        };

        set(&i18n::msg(lang, "installing", &info.latest), false, false);
        match update::self_install(&new_exe) {
            Ok(()) => {
                let _ = weak.upgrade_in_event_loop(|_| {
                    let _ = slint::quit_event_loop();
                });
            }
            Err(e) => set(
                &i18n::msg(lang, "install_fail", &e.to_string()),
                true,
                false,
            ),
        }
    });
}

fn apply_settings(ui: &MainWindow, state: &Shared, tray: &tray::Tray) {
    let parse_time = |s: slint::SharedString| NaiveTime::parse_from_str(s.trim(), "%H:%M");
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
        let lang = Lang::from_index(ui.get_lang_index());
        ui.set_tab_index(0);
        ui.set_status(i18n::msg(lang, "bad_time", "").into());
        return;
    };
    if light_at == dark_at {
        let lang = Lang::from_index(ui.get_lang_index());
        ui.set_tab_index(0);
        ui.set_status(i18n::msg(lang, "same_time", "").into());
        return;
    }

    let (Some(light_offset_min), Some(dark_offset_min)) = (
        parse_off(ui.get_light_offset()),
        parse_off(ui.get_dark_offset()),
    ) else {
        let lang = Lang::from_index(ui.get_lang_index());
        ui.set_tab_index(0);
        ui.set_status(i18n::msg(lang, "bad_offset", "").into());
        return;
    };

    let mode = if ui.get_mode_index() == 1 {
        Mode::Sun
    } else {
        Mode::Fixed
    };
    let (light_theme, dark_theme) = {
        let st = state.borrow();
        (
            theme_path(&st.themes, ui.get_light_theme_index()),
            theme_path(&st.themes, ui.get_dark_theme_index()),
        )
    };
    let parse_coord = |s: slint::SharedString, min: f64, max: f64| -> Option<Option<f64>> {
        let t = s.trim().replace(',', ".");
        if t.is_empty() {
            Some(None)
        } else {
            match t.parse::<f64>() {
                Ok(v) if (min..=max).contains(&v) => Some(Some(v)),
                _ => None,
            }
        }
    };
    let lat_parsed = parse_coord(ui.get_lat(), -90.0, 90.0);
    let lon_parsed = parse_coord(ui.get_lon(), -180.0, 180.0);

    let (lat, lon) = match mode {
        Mode::Sun => match (lat_parsed, lon_parsed) {
            (Some(Some(la)), Some(Some(lo))) => (Some(la), Some(lo)),
            _ => {
                let lang = Lang::from_index(ui.get_lang_index());
                ui.set_tab_index(0);
                ui.set_status(i18n::msg(lang, "bad_coords", "").into());
                return;
            }
        },
        Mode::Fixed => {
            let typed = |s: slint::SharedString| !s.trim().is_empty();
            if (typed(ui.get_lat()) && lat_parsed.is_none())
                || (typed(ui.get_lon()) && lon_parsed.is_none())
            {
                let lang = Lang::from_index(ui.get_lang_index());
                ui.set_tab_index(0);
                ui.set_status(i18n::msg(lang, "bad_coords", "").into());
                return;
            }
            (lat_parsed.unwrap_or(None), lon_parsed.unwrap_or(None))
        }
    };

    let lang = Lang::from_index(ui.get_lang_index());
    let light_wallpaper = ui.get_light_wallpaper().trim().to_string();
    let dark_wallpaper = ui.get_dark_wallpaper().trim().to_string();
    for (light, p) in [(true, &light_wallpaper), (false, &dark_wallpaper)] {
        if !p.is_empty() && !PathBuf::from(p).is_file() {
            ui.set_tab_index(1);
            ui.set_status(i18n::wp_missing(lang, light, p).into());
            return;
        }
    }

    let accent_enabled = ui.get_accent_enabled();
    let light_accent = ui.get_light_accent().trim().to_string();
    let dark_accent = ui.get_dark_accent().trim().to_string();
    if accent_enabled {
        for (light, h) in [(true, &light_accent), (false, &dark_accent)] {
            if accent::parse_hex(h).is_none() {
                let label = i18n::mode_word(lang, light);
                ui.set_tab_index(1);
                ui.set_status(i18n::msg(lang, "accent_hex", label).into());
                return;
            }
        }
    }

    if ui.get_auto_enabled() && !ui.get_change_apps() && !ui.get_change_system() {
        ui.set_tab_index(0);
        ui.set_status(i18n::msg(lang, "need_target", "").into());
        return;
    }

    let new_cfg = Config {
        auto_enabled: ui.get_auto_enabled(),
        mode,
        light_at,
        dark_at,
        lat,
        lon,
        light_offset_min,
        dark_offset_min,
        change_apps: ui.get_change_apps(),
        change_system: ui.get_change_system(),
        light_theme,
        dark_theme,
        light_wallpaper,
        dark_wallpaper,
        accent_enabled,
        light_accent,
        dark_accent,
        language: lang.code().to_string(),
        close_hint_acked: state.borrow().cfg.close_hint_acked,
    };
    if let Err(e) = new_cfg.save() {
        ui.set_status(i18n::msg(lang, "save_fail", &e.to_string()).into());
        return;
    }
    {
        let mut st = state.borrow_mut();
        st.cfg = new_cfg;
        st.last_scheduled = None;
        st.manual_hold = None;
        st.last_wallpaper = None;
        st.last_accent = None;
    }

    if let Err(e) = autostart::set(ui.get_autostart()) {
        ui.set_autostart(autostart::is_enabled());
        ui.set_status(i18n::msg(lang, "autostart_fail", &e.to_string()).into());
        return;
    }

    let status_before = ui.get_status();
    if !state.borrow().cfg.auto_enabled {
        let cur = {
            let st = state.borrow();
            theme::effective_current(st.cfg.change_apps, st.cfg.change_system)
        };
        apply_wallpaper(ui, &mut state.borrow_mut(), cur);
        apply_accent(ui, &mut state.borrow_mut(), cur);
    }

    update_lang_ui(ui, state, lang);

    refresh_wallpaper_preview(ui, true);
    refresh_wallpaper_preview(ui, false);

    ui.set_dirty(false);
    if ui.get_status().as_str() == status_before.as_str() {
        ui.set_status("".into());
    }
    tick(ui, state, tray);
}
