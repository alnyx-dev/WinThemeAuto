use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveTime};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Fixed,
    Sun,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auto_enabled: bool,
    pub mode: Mode,
    pub light_at: NaiveTime,
    pub dark_at: NaiveTime,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub light_offset_min: i32,
    pub dark_offset_min: i32,
    pub change_apps: bool,
    pub change_system: bool,
    pub light_theme: String,
    pub dark_theme: String,
    pub light_wallpaper: String,
    pub dark_wallpaper: String,
    pub lockscreen_enabled: bool,
    pub light_lockscreen: String,
    pub dark_lockscreen: String,
    pub accent_enabled: bool,
    pub light_accent: String,
    pub dark_accent: String,
    pub language: String,
    pub close_hint_acked: bool,
    pub manual_hold: Option<Theme>,
    pub manual_hold_until: Option<DateTime<Local>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_enabled: false,
            mode: Mode::Fixed,
            light_at: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
            dark_at: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            lat: None,
            lon: None,
            light_offset_min: 0,
            dark_offset_min: 0,
            change_apps: true,
            change_system: true,
            light_theme: String::new(),
            dark_theme: String::new(),
            light_wallpaper: String::new(),
            dark_wallpaper: String::new(),
            lockscreen_enabled: false,
            light_lockscreen: String::new(),
            dark_lockscreen: String::new(),
            accent_enabled: false,
            light_accent: "0078D4".to_string(),
            dark_accent: "0078D4".to_string(),
            language: "en".to_string(),
            close_hint_acked: false,
            manual_hold: None,
            manual_hold_until: None,
        }
    }
}

fn parse_time_flex(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M:%S")
        .ok()
        .or_else(|| NaiveTime::parse_from_str(s, "%H:%M").ok())
}

fn normalize_lon(mut lon: f64) -> f64 {
    if !lon.is_finite() {
        return 0.0;
    }
    lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
    if lon == -180.0 {
        180.0
    } else {
        lon
    }
}

fn unique_backup(path: &std::path::Path) -> PathBuf {
    let base = path.with_extension("json.corrupt.bak");
    if !base.exists() {
        return base;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    path.with_extension(format!("json.corrupt.{stamp}.bak"))
}

fn sanitize(cfg: &mut Config) {
    cfg.lat = cfg
        .lat
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(-90.0, 90.0));
    cfg.lon = cfg.lon.filter(|v| v.is_finite()).map(normalize_lon);
    cfg.light_offset_min = cfg.light_offset_min.clamp(-180, 180);
    cfg.dark_offset_min = cfg.dark_offset_min.clamp(-180, 180);
    cfg.light_at = truncate_secs(cfg.light_at);
    cfg.dark_at = truncate_secs(cfg.dark_at);
    let lang = crate::i18n::Lang::from_code(&cfg.language);
    cfg.language = lang.code().to_string();
    if cfg
        .manual_hold_until
        .map(|u| u <= Local::now())
        .unwrap_or(false)
    {
        cfg.manual_hold = None;
        cfg.manual_hold_until = None;
    }
    if cfg.manual_hold.is_none() {
        cfg.manual_hold_until = None;
    }
}

fn truncate_secs(t: NaiveTime) -> NaiveTime {
    use chrono::Timelike;
    t.with_second(0)
        .unwrap_or(t)
        .with_nanosecond(0)
        .unwrap_or(t)
}

fn json_num(v: &serde_json::Value) -> Option<f64> {
    if let Some(f) = v.as_f64() {
        return Some(f);
    }
    v.as_str()
        .map(|s| s.trim().replace(',', "."))
        .and_then(|s| s.parse::<f64>().ok())
}

fn from_value_merged(v: &serde_json::Value) -> Config {
    let mut cfg = Config::default();
    if let Some(b) = v.get("auto_enabled").and_then(|x| x.as_bool()) {
        cfg.auto_enabled = b;
    }
    if let Some(m) = v.get("mode").and_then(|x| x.as_str()) {
        cfg.mode = match m {
            "Sun" => Mode::Sun,
            "Fixed" => Mode::Fixed,
            _ => cfg.mode,
        };
    }
    if let Some(s) = v.get("light_at").and_then(|x| x.as_str()) {
        if let Some(t) = parse_time_flex(s) {
            cfg.light_at = t;
        }
    }
    if let Some(s) = v.get("dark_at").and_then(|x| x.as_str()) {
        if let Some(t) = parse_time_flex(s) {
            cfg.dark_at = t;
        }
    }
    if let Some(f) = v.get("lat").and_then(json_num) {
        cfg.lat = Some(f);
    }
    if let Some(f) = v.get("lon").and_then(json_num) {
        cfg.lon = Some(f);
    }
    if let Some(n) = v.get("light_offset_min").and_then(|x| x.as_i64()) {
        cfg.light_offset_min = n.clamp(-180, 180) as i32;
    }
    if let Some(n) = v.get("dark_offset_min").and_then(|x| x.as_i64()) {
        cfg.dark_offset_min = n.clamp(-180, 180) as i32;
    }
    if let Some(b) = v.get("change_apps").and_then(|x| x.as_bool()) {
        cfg.change_apps = b;
    }
    if let Some(b) = v.get("change_system").and_then(|x| x.as_bool()) {
        cfg.change_system = b;
    }
    if let Some(s) = v.get("light_theme").and_then(|x| x.as_str()) {
        cfg.light_theme = s.to_string();
    }
    if let Some(s) = v.get("dark_theme").and_then(|x| x.as_str()) {
        cfg.dark_theme = s.to_string();
    }
    if let Some(s) = v.get("light_wallpaper").and_then(|x| x.as_str()) {
        cfg.light_wallpaper = s.trim().to_string();
    }
    if let Some(s) = v.get("dark_wallpaper").and_then(|x| x.as_str()) {
        cfg.dark_wallpaper = s.trim().to_string();
    }
    if let Some(b) = v.get("lockscreen_enabled").and_then(|x| x.as_bool()) {
        cfg.lockscreen_enabled = b;
    }
    if let Some(s) = v.get("light_lockscreen").and_then(|x| x.as_str()) {
        cfg.light_lockscreen = s.trim().to_string();
    }
    if let Some(s) = v.get("dark_lockscreen").and_then(|x| x.as_str()) {
        cfg.dark_lockscreen = s.trim().to_string();
    }
    if let Some(b) = v.get("accent_enabled").and_then(|x| x.as_bool()) {
        cfg.accent_enabled = b;
    }
    if let Some(s) = v.get("light_accent").and_then(|x| x.as_str()) {
        cfg.light_accent = s.trim().to_string();
    }
    if let Some(s) = v.get("dark_accent").and_then(|x| x.as_str()) {
        cfg.dark_accent = s.trim().to_string();
    }
    if let Some(s) = v.get("language").and_then(|x| x.as_str()) {
        cfg.language = s.trim().to_string();
    }
    if let Some(b) = v.get("close_hint_acked").and_then(|x| x.as_bool()) {
        cfg.close_hint_acked = b;
    }
    if let Some(s) = v.get("manual_hold").and_then(|x| x.as_str()) {
        cfg.manual_hold = match s {
            "Light" => Some(Theme::Light),
            "Dark" => Some(Theme::Dark),
            _ => None,
        };
    }
    if let Some(s) = v.get("manual_hold_until").and_then(|x| x.as_str()) {
        if let Ok(dt) = s.parse::<DateTime<Local>>() {
            cfg.manual_hold_until = Some(dt);
        }
    }
    sanitize(&mut cfg);
    cfg
}

impl Config {
    fn path() -> PathBuf {
        if let Some(dir) = dirs::config_dir() {
            return dir.join("WinThemeAuto").join("config.json");
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("config.json");
            }
        }
        PathBuf::from("config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        if let Ok(mut cfg) = serde_json::from_str::<Config>(&text) {
            sanitize(&mut cfg);
            return cfg;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            return from_value_merged(&v);
        }
        let bak = unique_backup(&path);
        let _ = std::fs::rename(&path, &bak);
        crate::log::warn(format!("config unparseable, moved to {}", bak.display()));
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("cannot create {}", dir.display()))?;
        }
        let data = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, data)?;
        if let Err(e) = std::fs::rename(&tmp, &path) {
            let _ = std::fs::remove_file(&path);
            std::fs::rename(&tmp, &path).map_err(|_| e)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_time() {
        assert!(parse_time_flex("07:30").is_some());
        assert!(parse_time_flex("07:30:00").is_some());
        assert!(parse_time_flex("xx").is_none());
    }

    #[test]
    fn merged_keeps_good_fields_on_bad_time() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"auto_enabled":true,"light_at":"oops"}"#).unwrap();
        let c = from_value_merged(&v);
        assert!(c.auto_enabled);
        assert_eq!(c.light_at.format("%H:%M").to_string(), "07:00");
    }

    #[test]
    fn merged_parses_custom_wallpapers() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"light_wallpaper":" C:\\a.jpg ","dark_wallpaper":"C:\\b.png"}"#,
        )
        .unwrap();
        let c = from_value_merged(&v);
        assert_eq!(c.light_wallpaper, "C:\\a.jpg");
        assert_eq!(c.dark_wallpaper, "C:\\b.png");
        let old: Config = serde_json::from_str(r#"{"auto_enabled":true}"#).unwrap();
        assert_eq!(old.light_wallpaper, "");
        assert_eq!(old.dark_wallpaper, "");
    }

    #[test]
    fn merged_parses_accent_fields() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"accent_enabled":true,"light_accent":" FF0000 ","dark_accent":"4cc2ff"}"#,
        )
        .unwrap();
        let c = from_value_merged(&v);
        assert!(c.accent_enabled);
        assert_eq!(c.light_accent, "FF0000");
        assert_eq!(c.dark_accent, "4cc2ff");
        let d = Config::default();
        assert!(!d.accent_enabled);
        assert_eq!(d.light_accent, "0078D4");
    }

    #[test]
    fn merged_parses_lockscreen_fields() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"lockscreen_enabled":true,"light_lockscreen":" C:\\l.jpg ","dark_lockscreen":"C:\\d.png"}"#,
        )
        .unwrap();
        let c = from_value_merged(&v);
        assert!(c.lockscreen_enabled);
        assert_eq!(c.light_lockscreen, "C:\\l.jpg");
        assert_eq!(c.dark_lockscreen, "C:\\d.png");
        let d = Config::default();
        assert!(!d.lockscreen_enabled);
        assert_eq!(d.light_lockscreen, "");
        assert_eq!(d.dark_lockscreen, "");
        let old: Config = serde_json::from_str(r#"{"auto_enabled":true}"#).unwrap();
        assert!(!old.lockscreen_enabled);
    }

    #[test]
    fn sanitize_clamps() {
        let mut c = Config {
            lat: Some(999.0),
            lon: Some(540.0),
            light_offset_min: 999,
            ..Config::default()
        };
        sanitize(&mut c);
        assert_eq!(c.lat, Some(90.0));
        assert_eq!(c.lon, Some(180.0));
        assert_eq!(c.light_offset_min, 180);
    }

    #[test]
    fn language_sanitizes() {
        let mut c = Config::default();
        assert_eq!(c.language, "en");
        c.language = "RU".to_string();
        sanitize(&mut c);
        assert_eq!(c.language, "ru");
        c.language = "de".to_string();
        sanitize(&mut c);
        assert_eq!(c.language, "en");
        let v: serde_json::Value = serde_json::from_str(r#"{"language":"ru"}"#).unwrap();
        assert_eq!(from_value_merged(&v).language, "ru");
    }

    #[test]
    fn none_means_unset_but_zero_is_valid() {
        let mut c = Config::default();
        sanitize(&mut c);
        assert_eq!(c.lat, None);
        assert_eq!(c.lon, None);
        let mut c = Config {
            lat: Some(0.0),
            lon: Some(0.0),
            ..Config::default()
        };
        sanitize(&mut c);
        assert_eq!(c.lat, Some(0.0));
        assert_eq!(c.lon, Some(0.0));
        let mut c = Config {
            lat: Some(f64::NAN),
            lon: Some(f64::INFINITY),
            ..Config::default()
        };
        sanitize(&mut c);
        assert_eq!(c.lat, None);
        assert_eq!(c.lon, None);
    }

    #[test]
    fn string_coords_and_huge_offsets() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"lat":"55,75","lon":"37.61"}"#).unwrap();
        let c = from_value_merged(&v);
        assert!((c.lat.unwrap() - 55.75).abs() < 1e-9);
        assert!((c.lon.unwrap() - 37.61).abs() < 1e-9);
        let v: serde_json::Value =
            serde_json::from_str(r#"{"light_offset_min":9999999999}"#).unwrap();
        assert_eq!(from_value_merged(&v).light_offset_min, 180);
    }

    #[test]
    fn seconds_truncate_on_sanitize() {
        let v: serde_json::Value = serde_json::from_str(r#"{"light_at":"07:00:45"}"#).unwrap();
        let c = from_value_merged(&v);
        assert_eq!(c.light_at.format("%H:%M:%S").to_string(), "07:00:00");
    }

    #[test]
    fn hold_roundtrip_and_expiry() {
        let future = (Local::now() + chrono::Duration::hours(2))
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();
        let v: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"manual_hold":"Dark","manual_hold_until":"{future}"}}"#
        ))
        .unwrap();
        let c = from_value_merged(&v);
        assert_eq!(c.manual_hold, Some(Theme::Dark));
        assert!(c.manual_hold_until.is_some());

        let past = (Local::now() - chrono::Duration::hours(2))
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();
        let v: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"manual_hold":"Light","manual_hold_until":"{past}"}}"#
        ))
        .unwrap();
        let c = from_value_merged(&v);
        assert_eq!(c.manual_hold, None);
        assert_eq!(c.manual_hold_until, None);

        let v: serde_json::Value = serde_json::from_str(r#"{"manual_hold":"Dark"}"#).unwrap();
        let c = from_value_merged(&v);
        assert_eq!(c.manual_hold, Some(Theme::Dark));
        assert_eq!(c.manual_hold_until, None);

        let v: serde_json::Value = serde_json::from_str(r#"{"manual_hold":"Neon"}"#).unwrap();
        assert_eq!(from_value_merged(&v).manual_hold, None);
    }
}
