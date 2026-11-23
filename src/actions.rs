use crate::{
    accent,
    appearance::Trigger,
    autostart,
    config::{Config, Mode},
    i18n::{self, Lang},
    log, schedule, slideshow,
    state::{clear_hold, set_hold, theme_path, Shared},
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
use std::{rc::Rc, time::Duration};

pub(crate) fn tick_delay(state: &Shared) -> Duration {
    let st = state.lock().unwrap();
    let now = Local::now();
    if let Some(until) = schedule::paused_until(&st.cfg, now) {
        return Duration::from_millis((until - now).num_milliseconds().clamp(500, 60_000) as u64);
    }
    if st.cfg.auto_enabled && (st.cfg.change_apps || st.cfg.change_system) {
        if let Some(sw) = schedule::next_switch(&st.cfg, now) {
            let ms = (sw.at - now).num_milliseconds().max(0) as u64;
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
    tick_inner(ui, state, tray, None);
}

fn tick_inner(ui: &MainWindow, state: &Shared, tray: &tray::Tray, manual: Option<Theme>) {
    let (mut apps_theme, mut sys_theme) = theme::current_pair();
    {
        let mut st = state.lock().unwrap();
        let now = Local::now();
        let lang = Lang::from_code(&st.cfg.language);

        if st.cfg.auto_enabled && schedule::paused_until(&st.cfg, now).is_none() {
            if let Some(want) = schedule::desired_theme(&st.cfg, now) {
                // A genuine theme change since the previous tick: only then
                // may an already-active slideshow advance (safety-net ticks
                // must not spin it).
                let prev_scheduled = st.last_scheduled;
                let switched = matches!(prev_scheduled, Some(p) if p != want);
                if schedule::hold_active(&st.cfg, want, now) {
                    st.last_scheduled = Some(want);
                } else {
                    clear_hold(&mut st);
                    let need = (st.cfg.change_apps && apps_theme != want)
                        || (st.cfg.change_system && sys_theme != want);
                    if need {
                        let result = theme::apply(want, st.cfg.change_apps, st.cfg.change_system);
                        (apps_theme, sys_theme) = theme::current_pair();
                        match result {
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
                    if manual.is_none() {
                        st.apply_appearance(ui, want, Trigger::Scheduled { switched });
                    }
                }
            } else {
                clear_hold(&mut st);
            }
        } else if !st.cfg.auto_enabled {
            clear_hold(&mut st);
        }

        if let Some(target) = manual {
            st.apply_appearance(ui, target, Trigger::Manual);
        }

        let info = if st.cfg.mode == Mode::Sun {
            schedule::sun_info(&st.cfg, now.date_naive())
        } else {
            String::new()
        };
        let schedule_enabled = st.cfg.auto_enabled && (st.cfg.change_apps || st.cfg.change_system);
        let has_next_switch = schedule::next_switch(&st.cfg, now).is_some();
        let schedule_paused = schedule::paused_until(&st.cfg, now).is_some();
        tray.update_pause(lang, schedule_enabled, has_next_switch, schedule_paused);
        ui.set_schedule_enabled(schedule_enabled);
        ui.set_has_next_switch(has_next_switch);
        ui.set_schedule_paused(schedule_paused);
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

pub(crate) fn switch_theme(
    ui: &MainWindow,
    state: &Shared,
    tray: &Rc<tray::Tray>,
    timer: &Rc<Timer>,
    target: Option<Theme>,
) -> Option<Theme> {
    let (apps, system, lang) = {
        let st = state.lock().unwrap();
        (
            st.cfg.change_apps,
            st.cfg.change_system,
            Lang::from_code(&st.cfg.language),
        )
    };
    if !apps && !system {
        ui.set_status(i18n::msg(lang, "nothing_toggle", "").into());
        return None;
    }
    let target = target.unwrap_or_else(|| theme::effective_current(apps, system).toggled());
    if let Err(e) = theme::apply(target, apps, system) {
        log::error(format!("manual switch failed: {e:#}"));
        ui.set_status(i18n::msg(lang, "error", &e.to_string()).into());
        return None;
    }

    let now = Local::now();
    let until = schedule::manual_hold_until(&state.lock().unwrap().cfg, target, now);
    if let Some(until) = until {
        set_hold(state, target, until);
        let next = schedule::next_switch_info(&state.lock().unwrap().cfg, now);
        ui.set_status(if next.is_empty() {
            i18n::msg(lang, "held_plain", "").into()
        } else {
            i18n::msg(lang, "held", &next).into()
        });
    } else {
        clear_hold(&mut state.lock().unwrap());
        ui.set_status("".into());
    }
    log::info(format!("manual switch: applied {target:?}"));
    tick_inner(ui, state, tray, Some(target));
    arm_tick(timer, &ui.as_weak(), state, tray);
    Some(target)
}

pub(crate) fn set_pause(
    ui: &MainWindow,
    state: &Shared,
    tray: &Rc<tray::Tray>,
    timer: &Rc<Timer>,
    action: tray::PauseAction,
) {
    let now = Local::now();
    {
        let mut st = state.lock().unwrap();
        let mut cfg = st.cfg.clone();
        cfg.paused_until = match action {
            tray::PauseAction::Hour => Some(now + chrono::Duration::hours(1)),
            tray::PauseAction::NextSwitch => {
                let Some(next) = schedule::next_switch(&cfg, now) else {
                    return;
                };
                Some(next.at)
            }
            tray::PauseAction::Resume => None,
        };
        cfg.manual_hold = None;
        cfg.manual_hold_until = None;
        if let Err(e) = cfg.save() {
            let lang = Lang::from_code(&cfg.language);
            log::error(format!("pause save failed: {e:#}"));
            ui.set_status(i18n::msg(lang, "save_fail", &e.to_string()).into());
            return;
        }
        st.cfg = cfg;
    }
    ui.set_status("".into());
    tick(ui, state, tray);
    arm_tick(timer, &ui.as_weak(), state, tray);
}

pub(crate) fn check_updates(ui: &MainWindow, lang: Lang, state: &Shared) {
    if ui.get_checking_update() {
        return;
    }
    state.lock().unwrap().available_update = None;
    ui.set_available_version("".into());
    ui.set_checking_update(true);
    ui.set_update_info(i18n::msg(lang, "checking_updates", "").into());
    ui.set_update_ok(false);

    let weak = ui.as_weak();
    let state = state.clone();
    std::thread::spawn(move || {
        let result = update::check();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let mut st = state.lock().unwrap();
            let lang = Lang::from_code(&st.cfg.language);
            match result {
                Ok(info) if info.is_newer => {
                    log::info(format!("update check: found {}", info.latest));
                    ui.set_available_version(info.latest.trim_start_matches('v').into());
                    ui.set_update_info("".into());
                    st.available_update = Some(info);
                }
                Ok(info) => {
                    log::info(format!("update check: already latest ({})", info.latest));
                    ui.set_update_info(i18n::msg(lang, "latest", &info.latest).into());
                    ui.set_update_ok(true);
                }
                Err(e) => {
                    log::warn(format!("update check failed: {e:#}"));
                    ui.set_update_info(i18n::msg(lang, "check_fail", &e.to_string()).into());
                }
            }
            ui.set_checking_update(false);
        });
    });
}

pub(crate) fn install_update(ui: &MainWindow, state: &Shared) {
    use std::sync::atomic::Ordering;
    if ui.get_checking_update() {
        return;
    }
    let (Some(info), lang, cancel) = ({
        let st = state.lock().unwrap();
        (
            st.available_update.clone(),
            Lang::from_code(&st.cfg.language),
            st.update_cancel.clone(),
        )
    }) else {
        return;
    };
    cancel.store(false, Ordering::SeqCst);
    ui.set_checking_update(true);
    ui.set_downloading_update(true);
    ui.set_update_ok(false);

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let set = |text: &str, done: bool, downloading: bool| {
            let text = text.to_string();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_update_info(text.into());
                ui.set_downloading_update(downloading);
                if done {
                    ui.set_checking_update(false);
                }
            });
        };

        set(&i18n::dl_progress(lang, &info.latest, 0, None), false, true);
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

        if cancel.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&new_exe);
            set(&i18n::msg(lang, "cancelled", ""), true, false);
            return;
        }

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
        if p.is_empty() {
            continue;
        }
        let fs_path = PathBuf::from(p);
        if fs_path.is_file() {
            continue;
        }
        if fs_path.is_dir() {
            if slideshow::collect_images(&fs_path).is_empty() {
                ui.set_tab_index(1);
                ui.set_status(i18n::slideshow_no_images(lang, light, p).into());
                return;
            }
            continue;
        }
        ui.set_tab_index(1);
        ui.set_status(i18n::wp_missing(lang, light, p).into());
        return;
    }

    let Some(slideshow_interval_min) =
        slideshow::parse_interval_minutes(ui.get_slideshow_interval().as_str())
    else {
        ui.set_tab_index(1);
        ui.set_status(i18n::msg(lang, "bad_slideshow_interval", "").into());
        return;
    };
    let slideshow_shuffle = ui.get_slideshow_shuffle();

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

    let (close_hint_acked, paused_until) = {
        let st = state.lock().unwrap();
        let keep_pause = ui.get_auto_enabled() && (ui.get_change_apps() || ui.get_change_system());
        let pause = keep_pause
            .then(|| schedule::paused_until(&st.cfg, Local::now()))
            .flatten();
        (st.cfg.close_hint_acked, pause)
    };
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
        slideshow_interval_min,
        slideshow_shuffle,
        lockscreen_enabled,
        light_lockscreen,
        dark_lockscreen,
        accent_enabled,
        light_accent,
        dark_accent,
        language: lang.code().to_string(),
        close_hint_acked,
        paused_until,
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
        st.appearance.invalidate();
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
        state
            .lock()
            .unwrap()
            .apply_appearance(ui, cur, Trigger::Scheduled { switched: false });
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
    use crate::state::State;
    use std::sync::Arc;

    fn state_with(cfg: Config) -> Shared {
        Arc::new(std::sync::Mutex::new(State {
            cfg,
            themes: Vec::new(),
            last_scheduled: None,
            available_update: None,
            update_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_titlebar_sys: None,
            appearance: crate::appearance::Worker::default(),
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
