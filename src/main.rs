#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accent;
mod actions;
mod appearance;
mod autostart;
mod cli;
mod commands;
mod config;
mod geo;
mod i18n;
mod lockscreen;
mod log;
mod schedule;
mod single_instance;
mod slideshow;
mod state;
mod sun;
mod theme;
mod themes;
mod thumb;
mod tray;
mod ui;
mod update;

use actions::{
    apply_settings, arm_tick, check_updates, install_update, set_pause, switch_theme, tick,
};
use chrono::Local;
use config::Config;
use i18n::Lang;
use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};
use state::{resolve_lock_wallpaper_path, resolve_wallpaper_path, Shared, State};
use std::{rc::Rc, sync::Arc, time::Duration};
use theme::Theme;
use tray_icon::{menu::MenuEvent, TrayIconEvent};
use ui::{
    apply_autostart_instant, apply_language_instant, browse_lockscreen, browse_slideshow_folder,
    browse_wallpaper, detect_location, load_into_ui, refresh_lockscreen_preview, refresh_themes,
    refresh_wallpaper_preview, show_window,
};

slint::include_modules!();

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
    let _guard = {
        let mut guard = None;
        for attempt in 0..40 {
            if let Some(acquired) = single_instance::Guard::acquire()? {
                guard = Some(acquired);
                break;
            }
            if let Some(applied) = commands::send(target)? {
                println!(
                    "{}",
                    if applied == Theme::Dark {
                        "dark"
                    } else {
                        "light"
                    }
                );
                return Ok(());
            }
            if attempt < 39 {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        guard.ok_or_else(|| {
            anyhow::anyhow!("The running instance is unavailable for theme commands")
        })?
    };
    let mut cfg = Config::load();
    if !cfg.change_apps && !cfg.change_system {
        eprintln!("Nothing to switch: enable Apps or System in the app first");
        std::process::exit(1);
    }
    let want = target
        .unwrap_or_else(|| theme::effective_current(cfg.change_apps, cfg.change_system).toggled());
    if let Err(e) = theme::apply(want, cfg.change_apps, cfg.change_system) {
        log::error(format!("cli switch failed: {e:#}"));
        eprintln!("Failed to switch theme: {e}");
        std::process::exit(1);
    }
    log::info(format!(
        "cli switch: applied {}",
        if want == Theme::Dark { "dark" } else { "light" }
    ));
    let themes = themes::enumerate();
    if let Some(path) = resolve_wallpaper_path(&cfg, &themes, want).filter(|path| path.exists()) {
        let interval = slideshow::clamp_interval(cfg.slideshow_interval_min);
        if let Err(e) = slideshow::apply_explicit(&path, interval, cfg.slideshow_shuffle) {
            eprintln!("Wallpaper: {e}");
        }
    }
    if let Some(resolved) = resolve_lock_wallpaper_path(&cfg, &themes, want) {
        let path = if resolved.is_dir() {
            slideshow::first_image(&resolved)
        } else {
            Some(resolved)
        };
        if let Some(path) = path {
            if path.exists() {
                if let Err(e) = lockscreen::set_image(&path) {
                    eprintln!("Lock screen: {e}");
                }
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
    let until = schedule::manual_hold_until(&cfg, want, Local::now());
    if until.is_some() || cfg.manual_hold.is_some() {
        cfg.manual_hold = until.map(|_| want);
        cfg.manual_hold_until = until;
        if let Err(e) = cfg.save() {
            eprintln!("Warning: cannot persist manual hold: {e}");
        }
        if let Some(until) = until {
            eprintln!(
                "Holding the manual theme until {}.",
                until.format("%d.%m %H:%M")
            );
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
        println!("{head} • {next}");
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
    let state: Shared = Arc::new(std::sync::Mutex::new(State {
        cfg: Config::load(),
        themes: themes::enumerate(),
        last_scheduled: None,
        available_update: None,
        update_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        last_titlebar_sys: None,
        appearance: appearance::Worker::default(),
        last_tray_tooltip: None,
        last_tray_dark: None,
    }));

    let ui = MainWindow::new()?;
    let tray: Rc<tray::Tray> = Rc::new({
        let cfg = state.lock().unwrap().cfg.clone();
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
    {
        let st = state.lock().unwrap();
        load_into_ui(&ui, &st.cfg, &st.themes);
    }
    tick(&ui, &state, &tray);
    {
        let st = state.lock().unwrap();
        let now = Local::now();
        if let Some(want) = schedule::desired_theme(&st.cfg, now) {
            if schedule::hold_active(&st.cfg, want, now) {
                log::info("startup: manual hold active, auto-switch paused until next switch");
            }
        }
    }

    let tick_timer: Rc<Timer> = Rc::new(Timer::default());

    {
        let w = ui.as_weak();
        let s = state.clone();
        ui.window().on_close_requested(move || {
            if s.lock().unwrap().cfg.close_hint_acked {
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
                    let mut st = s.lock().unwrap();
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
                    let mut st = s.lock().unwrap();
                    st.cfg.close_hint_acked = true;
                    let _ = st.cfg.save();
                }
                ui.set_show_close_dialog(false);
            }
            let _ = slint::quit_event_loop();
        });
    }

    {
        let (w, s, t, timer) = (
            ui.as_weak(),
            state.clone(),
            tray.clone(),
            tick_timer.clone(),
        );
        ui.on_toggle_theme(move || {
            if let Some(ui) = w.upgrade() {
                switch_theme(&ui, &s, &t, &timer, None);
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
        let (w, s, t, timer) = (
            ui.as_weak(),
            state.clone(),
            tray.clone(),
            tick_timer.clone(),
        );
        ui.on_apply_settings(move || {
            if let Some(ui) = w.upgrade() {
                apply_settings(&ui, &s, &t, &timer);
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
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                detect_location(&ui, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_light_wallpaper(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                browse_wallpaper(&ui, true, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_dark_wallpaper(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                browse_wallpaper(&ui, false, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_light_slideshow(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                browse_slideshow_folder(&ui, true, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_dark_slideshow(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                browse_slideshow_folder(&ui, false, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_light_lockscreen(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                browse_lockscreen(&ui, true, lang);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_browse_dark_lockscreen(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                browse_lockscreen(&ui, false, lang);
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
        let w = ui.as_weak();
        ui.on_clear_light_lockscreen(move || {
            if let Some(ui) = w.upgrade() {
                ui.set_light_lockscreen("".into());
                ui.set_dirty(true);
                refresh_lockscreen_preview(&ui, true);
            }
        });
    }
    {
        let w = ui.as_weak();
        ui.on_clear_dark_lockscreen(move || {
            if let Some(ui) = w.upgrade() {
                ui.set_dark_lockscreen("".into());
                ui.set_dirty(true);
                refresh_lockscreen_preview(&ui, false);
            }
        });
    }
    {
        ui.on_open_releases(move || {
            update::open_releases_page();
        });
    }
    {
        ui.on_open_logs(move || {
            log::open_logs();
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_check_updates(move || {
            if let Some(ui) = w.upgrade() {
                let lang = Lang::from_code(&s.lock().unwrap().cfg.language);
                check_updates(&ui, lang, &s);
            }
        });
    }
    {
        let (w, s) = (ui.as_weak(), state.clone());
        ui.on_install_update(move || {
            if let Some(ui) = w.upgrade() {
                install_update(&ui, &s);
            }
        });
    }
    {
        let s = state.clone();
        ui.on_cancel_update(move || {
            s.lock()
                .unwrap()
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

    for action in [
        tray::PauseAction::Hour,
        tray::PauseAction::NextSwitch,
        tray::PauseAction::Resume,
    ] {
        let (w, s, t, timer) = (
            ui.as_weak(),
            state.clone(),
            tray.clone(),
            tick_timer.clone(),
        );
        let run = move || {
            if let Some(ui) = w.upgrade() {
                set_pause(&ui, &s, &t, &timer, action);
            }
        };
        match action {
            tray::PauseAction::Hour => ui.on_pause_hour(run),
            tray::PauseAction::NextSwitch => ui.on_pause_next(run),
            tray::PauseAction::Resume => ui.on_resume_schedule(run),
        }
    }

    arm_tick(&tick_timer, &ui.as_weak(), &state, &tray);

    let tray_timer = Timer::default();
    {
        let (w, s, t, timer) = (
            ui.as_weak(),
            state.clone(),
            tray.clone(),
            tick_timer.clone(),
        );
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
                        switch_theme(&ui, &s, &t, &timer, None);
                    } else if let Some(action) = t.pause_action(&ev.id) {
                        set_pause(&ui, &s, &t, &timer, action);
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

    let _commands = {
        let (w, s, t, timer) = (
            ui.as_weak(),
            state.clone(),
            tray.clone(),
            tick_timer.clone(),
        );
        commands::Receiver::create(move |target| {
            // A Windows API invoked by another callback may pump sent messages.
            drop(s.try_lock().ok()?);
            switch_theme(&w.upgrade()?, &s, &t, &timer, target)
        })?
    };

    if !start_hidden {
        show_window(&ui, &state);
    }

    slint::run_event_loop_until_quit()?;

    drop(tray);
    Ok(())
}
