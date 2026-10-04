use crate::{config::Config, log, theme::Theme, themes};
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::Arc;

pub struct State {
    pub cfg: Config,
    pub themes: Vec<themes::ThemeEntry>,
    pub last_scheduled: Option<Theme>,
    pub update_cancel: Arc<std::sync::atomic::AtomicBool>,
    pub last_titlebar_sys: Option<Theme>,
    pub last_wallpaper: Option<PathBuf>,
    pub last_accent: Option<u32>,
    pub last_tray_tooltip: Option<String>,
    pub last_tray_dark: Option<bool>,
}

pub type Shared = Arc<std::sync::Mutex<State>>;

pub(crate) fn theme_index(themes: &[themes::ThemeEntry], path: &str) -> i32 {
    if path.is_empty() {
        return 0;
    }
    themes
        .iter()
        .position(|t| t.path.to_string_lossy() == path)
        .map(|i| i as i32 + 1)
        .unwrap_or(0)
}

pub(crate) fn theme_path(themes: &[themes::ThemeEntry], index: i32) -> String {
    if index <= 0 {
        return String::new();
    }
    themes
        .get(index as usize - 1)
        .map(|t| t.path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(crate) fn clear_hold(st: &mut State) {
    if st.cfg.manual_hold.is_some() {
        st.cfg.manual_hold = None;
        st.cfg.manual_hold_until = None;
        if let Err(e) = st.cfg.save() {
            log::error(format!("hold clear save failed: {e:#}"));
        }
    }
}

pub(crate) fn set_hold(state: &Shared, hold: Theme, until: DateTime<Local>) {
    let mut st = state.lock().unwrap();
    if st.cfg.manual_hold == Some(hold) && st.cfg.manual_hold_until == Some(until) {
        return;
    }
    st.cfg.manual_hold = Some(hold);
    st.cfg.manual_hold_until = Some(until);
    if let Err(e) = st.cfg.save() {
        log::error(format!("hold save failed: {e:#}"));
    }
}

pub(crate) fn resolve_wallpaper_path(
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
