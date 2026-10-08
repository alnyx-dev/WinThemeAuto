use crate::{
    accent, autostart,
    config::{Config, Mode},
    i18n::{self, Lang},
    lockscreen, log, schedule,
    state::{
        clear_hold, resolve_lock_wallpaper_path, resolve_wallpaper_path, set_hold, theme_path,
        Shared, State,
    },
    theme::{self, Theme},
    tray,
    ui::{
        force_light_titlebar, refresh_lockscreen_preview, refresh_wallpaper_preview, update_lang_ui,
    },
    update, MainWindow,
};
use chrono::{Local, NaiveTime};
use slint::{ComponentHandle, Timer, TimerMode};
use std::path::PathBuf;
use std::{rc::Rc, sync::Arc, time::Duration};

pub(crate) fn tick_delay(state: &Shared) -> Duration {
    let st = state.lock().unwrap();
    if st.cfg.auto_enabled && (st.cfg.change_apps || st.cfg.change_system) {
        if let Some(sw) = schedule::next_switch(&st.cfg, Local::now()) {
            let ms = (sw.at - Local::now()).num_milliseconds().max(0) as u64;
            // Wake exactly at the switch, with a 60 s safety net for clock
            // changes, sleep/resume and external theme drift.
            return Duration::from_millis(ms.clamp(500, 60_000));
        }
        return Duration::from_secs(60);
    }
    Duration::from_secs(15)
}

pub(crate) fn arm_tick(
    timer: &Rc<Timer>,
    ui: &slint::Weak<MainWindow>,
    state: &Shared,
    tray: &Rc<tray::Tray>,
) {
    let delay = tick_delay(state);
    let (timer2, ui2, state2, tray2) = (timer.clone(), ui.clone(), state.clone(), tray.clone());
    timer.start(TimerMode::SingleShot, delay, move || {
        if let Some(upgraded) = ui2.upgrade() {
            tick(&upgraded, &state2, &tray2);
        }
        arm_tick(&timer2, &ui2, &state2, &tray2);
    });
}

pub(crate) fn tick(ui: &MainWindow, state: &Shared, tray: &tray::Tray) {
    let (apps_theme, sys_theme) = theme::current_pair();
    {
        let mut st = state.lock().unwrap();
        let now = Local::now();
        let lang = Lang::from_code(&st.cfg.language);

        if st.cfg.auto_enabled {
            if let Some(want) = schedule::desired_theme(&st.cfg, now) {
                if schedule::hold_active(&st.cfg, want, now) {
                    st.last_scheduled = Some(want);
                } else {
                    clear_hold(&mut st);
                    let need = (st.cfg.change_apps && apps_theme != want)
                        || (st.cfg.change_system && sys_theme != want);
                    if need {
                        match theme::apply(want, st.cfg.change_apps, st.cfg.change_system) {
                            Ok(()) => {
                                st.last_scheduled = Some(want);
                                log::info(format!(
                                    "auto switch: applied {}",
                                    if want == Theme::Dark { "dark" } else { "light" }
                                ));
                            }
                            Err(e) => {
                                log::error(format!("auto switch failed: {e:#}"));
                                ui.set_status(
                                    i18n::msg(lang, "switch_fail", &e.to_string()).into(),
                                );
                            }
                        }
                    } else {
                        st.last_scheduled = Some(want);
                    }
                    apply_wallpaper(ui, &mut st, want);
                    apply_lockscreen(ui, &mut st, want);
                    apply_accent(ui, &mut st, want);
                }
            } else {
                clear_hold(&mut st);
            }
        } else {
            clear_hold(&mut st);
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
            format!("WinThemeAuto — {head}")
        } else {
            format!("WinThemeAuto — {head} • {next}")
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

pub(crate) fn apply_accent(ui: &MainWindow, st: &mut State, want: Theme) {
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
            log::warn(format!("accent failed: {e:#}"));
            let lang = Lang::from_code(&st.cfg.language);
            ui.set_status(i18n::msg(lang, "accent", &e.to_string()).into());
        }
    }
}

pub(crate) fn apply_wallpaper(ui: &MainWindow, st: &mut State, want: Theme) {
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
            log::warn(format!("wallpaper failed: {e:#}"));
            let lang = Lang::from_code(&st.cfg.language);
            ui.set_status(i18n::msg(lang, "wallpaper", &e.to_string()).into());
        }
    }
}

pub(crate) fn apply_lockscreen(ui: &MainWindow, st: &mut State, want: Theme) {
    let Some(path) = resolve_lock_wallpaper_path(&st.cfg, &st.themes, want) else {
        return;
    };
    if st.last_lockscreen.as_ref() == Some(&path) {
        return;
    }
    if !path.exists() {
        return;
    }
    match lockscreen::set_image(&path) {
        Ok(()) => {
            st.last_lockscreen = Some(path.clone());
            log::info(format!("lock screen: applied {}", path.display()));
        }
        Err(e) => {
            log::warn(format!("lock screen failed: {e:#}"));
            let lang = Lang::from_code(&st.cfg.language);
            ui.set_status(i18n::msg(lang, "lockscreen", &format!("{e:#}")).into());
        }
    }
}

pub(crate) fn toggle(ui: &MainWindow, state: &Shared, tray: &Rc<tray::Tray>, timer: &Rc<Timer>) {
    let (apps, system, lang, auto_enabled) = {
        let st = state.lock().unwrap();
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
            log::info(format!(
                "manual switch: applied {}",
                if new == Theme::Dark { "dark" } else { "light" }
            ));
            ui.set_is_dark(new == Theme::Dark);
            apply_wallpaper(ui, &mut state.lock().unwrap(), new);
            apply_lockscreen(ui, &mut state.lock().unwrap(), new);
            apply_accent(ui, &mut state.lock().unwrap(), new);
            if auto_enabled {
                let now = Local::now();
                let (want, until) = {
                    let st = state.lock().unwrap();
                    (
                        schedule::desired_theme(&st.cfg, now),
                        schedule::next_switch(&st.cfg, now).map(|s| s.at),
                    )
                };
                match want {
                    Some(w) if w != new => {
                        let until = until.unwrap_or_else(|| now + chrono::Duration::days(1));
                        set_hold(state, new, until);
                        let next = schedule::next_switch_info(&state.lock().unwrap().cfg, now);
                        let text = if next.is_empty() {
                            i18n::msg(lang, "held_plain", "")
                        } else {
                            i18n::msg(lang, "held", &next)
                        };
                        ui.set_status(text.into());
                    }
                    _ => {
                        clear_hold(&mut state.lock().unwrap());
                        ui.set_status("".into());
                    }
                }
            } else {
                clear_hold(&mut state.lock().unwrap());
                ui.set_status("".into());
            }
            tick(ui, state, tray);
        }
        Err(e) => {
            log::error(format!("manual switch failed: {e:#}"));
            ui.set_status(i18n::msg(lang, "error", &e.to_string()).into());
            ui.set_is_dark(theme::effective_current(apps, system) == Theme::Dark);
        }
    }
    arm_tick(timer, &ui.as_weak(), state, tray);
}

pub(crate) fn check_updates(
    ui: &MainWindow,
    lang: Lang,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) {
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
                log::warn(format!("update check failed: {e:#}"));
                set(&i18n::msg(lang, "check_fail", &e.to_string()), true, false);
                return;
            }
        };
        if !info.is_newer {
            log::info(format!("update check: already latest ({})", info.latest));
            set(&i18n::msg(lang, "latest", &info.latest), true, true);
            return;
        }
        log::info(format!("update check: found {}", info.latest));

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
                    log::info("update download cancelled by user");
                    set(&i18n::msg(lang, "cancelled", ""), true, false);
                } else {
                    log::error(format!("update download failed: {e:#}"));
                    set(&i18n::msg(lang, "dl_fail", &err), true, false);
                }
                return;
            }
        };

        set(&i18n::msg(lang, "installing", &info.latest), false, false);
        match update::self_install(&new_exe) {
            Ok(()) => {
                log::info(format!("update installed, restarting into {}", info.latest));
                let _ = weak.upgrade_in_event_loop(|_| {
                    let _ = slint::quit_event_loop();
                });
            }
            Err(e) => {
                log::error(format!("update install failed: {e:#}"));
                set(
                    &i18n::msg(lang, "install_fail", &e.to_string()),
                    true,
                    false,
                )
            }
        }
    });
}

pub(crate) fn apply_settings(
    ui: &MainWindow,
    state: &Shared,
    tray: &Rc<tray::Tray>,
    timer: &Rc<Timer>,
) {
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
        let st = state.lock().unwrap();
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

    let lockscreen_enabled = ui.get_lockscreen_enabled();
    let light_lockscreen = ui.get_light_lockscreen().trim().to_string();
    let dark_lockscreen = ui.get_dark_lockscreen().trim().to_string();
    if lockscreen_enabled {
        for (light, p) in [(true, &light_lockscreen), (false, &dark_lockscreen)] {
            if !p.is_empty() && !PathBuf::from(p).is_file() {
                ui.set_tab_index(1);
                ui.set_status(i18n::lock_missing(lang, light, p).into());
                return;
            }
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
        lockscreen_enabled,
        light_lockscreen,
        dark_lockscreen,
        accent_enabled,
        light_accent,
        dark_accent,
        language: lang.code().to_string(),
        close_hint_acked: state.lock().unwrap().cfg.close_hint_acked,
        manual_hold: None,
        manual_hold_until: None,
    };
    if let Err(e) = new_cfg.save() {
        log::error(format!("settings save failed: {e:#}"));
        ui.set_status(i18n::msg(lang, "save_fail", &e.to_string()).into());
        return;
    }
    {
        let mut st = state.lock().unwrap();
        st.cfg = new_cfg;
        st.last_scheduled = None;
        st.last_wallpaper = None;
        st.last_lockscreen = None;
        st.last_accent = None;
    }

    if let Err(e) = autostart::set(ui.get_autostart()) {
        log::warn(format!("autostart failed: {e:#}"));
        ui.set_autostart(autostart::is_enabled());
        ui.set_status(i18n::msg(lang, "autostart_fail", &e.to_string()).into());
        return;
    }

    let status_before = ui.get_status();
    if !state.lock().unwrap().cfg.auto_enabled {
        let cur = {
            let st = state.lock().unwrap();
            theme::effective_current(st.cfg.change_apps, st.cfg.change_system)
        };
        apply_wallpaper(ui, &mut state.lock().unwrap(), cur);
        apply_lockscreen(ui, &mut state.lock().unwrap(), cur);
        apply_accent(ui, &mut state.lock().unwrap(), cur);
    }

    update_lang_ui(ui, state, lang);

    refresh_wallpaper_preview(ui, true);
    refresh_wallpaper_preview(ui, false);
    refresh_lockscreen_preview(ui, true);
    refresh_lockscreen_preview(ui, false);

    ui.set_dirty(false);
    if ui.get_status().as_str() == status_before.as_str() {
        ui.set_status("".into());
    }
    tick(ui, state, tray);
    arm_tick(timer, &ui.as_weak(), state, tray);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(cfg: Config) -> Shared {
        Arc::new(std::sync::Mutex::new(State {
            cfg,
            themes: Vec::new(),
            last_scheduled: None,
            update_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_titlebar_sys: None,
            last_wallpaper: None,
            last_lockscreen: None,
            last_accent: None,
            last_tray_tooltip: None,
            last_tray_dark: None,
        }))
    }

    #[test]
    fn idle_polls_slowly() {
        let s = state_with(Config::default());
        assert_eq!(tick_delay(&s), Duration::from_secs(15));
    }

    #[test]
    fn no_targets_or_no_switch_polls() {
        let cfg = Config {
            auto_enabled: true,
            change_apps: false,
            change_system: false,
            ..Config::default()
        };
        assert_eq!(tick_delay(&state_with(cfg)), Duration::from_secs(15));

        let cfg = Config {
            auto_enabled: true,
            mode: Mode::Sun,
            ..Config::default()
        };
        assert_eq!(tick_delay(&state_with(cfg)), Duration::from_secs(60));
    }

    #[test]
    fn wakes_at_next_switch() {
        let now = Local::now();
        let in_half_minute = (now + chrono::Duration::seconds(30)).time();
        let later = (now + chrono::Duration::hours(3)).time();
        let cfg = Config {
            auto_enabled: true,
            light_at: in_half_minute,
            dark_at: later,
            ..Config::default()
        };
        let ms = tick_delay(&state_with(cfg)).as_millis();
        assert!(
            (20_000..=31_000).contains(&ms),
            "expected ~30 s wake, got {ms} ms"
        );
    }
}
