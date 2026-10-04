use crate::{
    autostart,
    config::{Config, Mode},
    geo,
    i18n::{self, Lang},
    log,
    state::{theme_index, theme_path, Shared},
    theme, themes, update, MainWindow,
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::path::PathBuf;

pub(crate) fn show_window(ui: &MainWindow, state: &Shared) {
    let _ = ui.show();
    force_light_titlebar(ui);
    state.lock().unwrap().last_titlebar_sys = Some(theme::current_system());
}

pub(crate) fn force_light_titlebar(ui: &MainWindow) {
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

pub(crate) fn load_into_ui(ui: &MainWindow, cfg: &Config, themes: &[themes::ThemeEntry]) {
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

pub(crate) fn apply_lang(ui: &MainWindow, lang: Lang) {
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
    ui.set_t_logs(s.logs.into());
    ui.set_t_onboard(s.onboard.into());
    ui.set_t_cancel(s.cancel.into());
    ui.set_t_close_title(s.close_title.into());
    ui.set_t_close_body(s.close_body.into());
    ui.set_t_close_minimize(s.close_minimize.into());
    ui.set_t_close_quit(s.close_quit.into());
    ui.set_t_close_never(s.close_never.into());
}

pub(crate) fn update_lang_ui(ui: &MainWindow, state: &Shared, lang: Lang) {
    apply_lang(ui, lang);
    ui.set_lang_index(lang.index());
    ui.set_update_info("".into());
    ui.set_geo_hint("".into());
    let st = state.lock().unwrap();
    let names: Vec<SharedString> =
        std::iter::once(SharedString::from(i18n::ui(lang).system_default))
            .chain(st.themes.iter().map(|t| themes::display_name(t).into()))
            .collect();
    ui.set_light_themes(ModelRc::new(VecModel::from(names.clone())));
    ui.set_dark_themes(ModelRc::new(VecModel::from(names)));
}

pub(crate) fn apply_language_instant(ui: &MainWindow, state: &Shared) {
    let lang = Lang::from_index(ui.get_lang_index());
    {
        let mut st = state.lock().unwrap();
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

pub(crate) fn apply_autostart_instant(ui: &MainWindow) {
    let want = ui.get_autostart();
    if let Err(e) = autostart::set(want) {
        ui.set_autostart(autostart::is_enabled());
        let lang = Lang::from_index(ui.get_lang_index());
        ui.set_status(i18n::msg(lang, "autostart_fail", &e.to_string()).into());
    }
}

pub(crate) fn refresh_themes(ui: &MainWindow, state: &Shared) {
    let lang = Lang::from_code(&state.lock().unwrap().cfg.language);
    ui.set_status(i18n::msg(lang, "rescanning", "").into());
    let weak = ui.as_weak();
    let state = state.clone();
    std::thread::spawn(move || {
        let fresh = themes::enumerate();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let (light_path, dark_path) = {
                let st = state.lock().unwrap();
                (
                    theme_path(&st.themes, ui.get_light_theme_index()),
                    theme_path(&st.themes, ui.get_dark_theme_index()),
                )
            };
            let light_idx = theme_index(&fresh, &light_path);
            let dark_idx = theme_index(&fresh, &dark_path);
            state.lock().unwrap().themes = fresh;
            let st = state.lock().unwrap();
            log::info(format!("themes rescanned: {} entries", st.themes.len()));
            let lang = Lang::from_code(&st.cfg.language);
            let names: Vec<SharedString> =
                std::iter::once(SharedString::from(i18n::ui(lang).system_default))
                    .chain(st.themes.iter().map(|t| themes::display_name(t).into()))
                    .collect();
            ui.set_light_themes(ModelRc::new(VecModel::from(names.clone())));
            ui.set_dark_themes(ModelRc::new(VecModel::from(names)));
            ui.set_light_theme_index(light_idx);
            ui.set_dark_theme_index(dark_idx);
            if (light_idx == 0 && !light_path.is_empty())
                || (dark_idx == 0 && !dark_path.is_empty())
            {
                ui.set_status(i18n::msg(lang, "theme_gone", "").into());
            } else if ui.get_status().as_str() == i18n::msg(lang, "rescanning", "").as_str() {
                ui.set_status("".into());
            }
        });
    });
}

pub(crate) fn detect_location(ui: &MainWindow, lang: Lang) {
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
                    log::warn(format!("geolocation failed: {e}"));
                }
            }
        });
    });
}

pub(crate) fn refresh_wallpaper_preview(ui: &MainWindow, light: bool) {
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

pub(crate) fn browse_wallpaper(ui: &MainWindow, light: bool, lang: Lang) {
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
