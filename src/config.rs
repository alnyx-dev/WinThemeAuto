use anyhow::{Context, Result};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    /// Latitude in degrees, `None` = not set yet.
    /// `Some(0.0)` is a valid location (Gulf of Guinea), unlike the old
    /// `0.0 == unset` sentinel.
    pub lat: Option<f64>,
    /// Longitude in degrees, `None` = not set yet.
    pub lon: Option<f64>,
    pub light_offset_min: i32,
    pub dark_offset_min: i32,
    pub change_apps: bool,
    pub change_system: bool,
    /// Full `.theme` file applied on light switch ("" = flags only).
    pub light_theme: String,
    /// Full `.theme` file applied on dark switch ("" = flags only).
    pub dark_theme: String,
    /// Custom wallpaper for light mode ("" = use theme wallpaper).
    /// Wins over `light_theme` wallpaper when set.
    pub light_wallpaper: String,
    /// Custom wallpaper for dark mode ("" = use theme wallpaper).
    /// Wins over `dark_theme` wallpaper when set.
    pub dark_wallpaper: String,
    /// Sync the Windows accent color with the theme.
    pub accent_enabled: bool,
    /// Accent for light mode, hex `RRGGBB` (e.g. `"0078D4"`).
    pub light_accent: String,
    /// Accent for dark mode, hex `RRGGBB`.
    pub dark_accent: String,
    /// UI language code: `"en"` or `"ru"`.
    pub language: String,
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
            accent_enabled: false,
            light_accent: "0078D4".to_string(),
            dark_accent: "0078D4".to_string(),
            language: "en".to_string(),
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

fn sanitize(cfg: &mut Config) {
    // `None` = never set; `Some(0.0)` is a real place (Gulf of Guinea).
    cfg.lat = cfg
        .lat
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(-90.0, 90.0));
    cfg.lon = cfg.lon.filter(|v| v.is_finite()).map(normalize_lon);
    cfg.light_offset_min = cfg.light_offset_min.clamp(-180, 180);
    cfg.dark_offset_min = cfg.dark_offset_min.clamp(-180, 180);
    let lang = crate::i18n::Lang::from_code(&cfg.language);
    cfg.language = lang.code().to_string();
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
    if let Some(f) = v.get("lat").and_then(|x| x.as_f64()) {
        cfg.lat = Some(f);
    }
    if let Some(f) = v.get("lon").and_then(|x| x.as_f64()) {
        cfg.lon = Some(f);
    }
    if let Some(n) = v.get("light_offset_min").and_then(|x| x.as_i64()) {
        cfg.light_offset_min = (n as i32).clamp(-180, 180);
    }
    if let Some(n) = v.get("dark_offset_min").and_then(|x| x.as_i64()) {
        cfg.dark_offset_min = (n as i32).clamp(-180, 180);
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
        let bak = path.with_extension("json.corrupt.bak");
        let _ = std::fs::rename(&path, &bak);
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
        // Old configs without the fields still load.
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
        // Unset stays unset.
        let mut c = Config::default();
        sanitize(&mut c);
        assert_eq!(c.lat, None);
        assert_eq!(c.lon, None);
        // Explicit 0,0 (Gulf of Guinea) survives sanitize.
        let mut c = Config {
            lat: Some(0.0),
            lon: Some(0.0),
            ..Config::default()
        };
        sanitize(&mut c);
        assert_eq!(c.lat, Some(0.0));
        assert_eq!(c.lon, Some(0.0));
        // Non-finite becomes unset, not 0.0.
        let mut c = Config {
            lat: Some(f64::NAN),
            lon: Some(f64::INFINITY),
            ..Config::default()
        };
        sanitize(&mut c);
        assert_eq!(c.lat, None);
        assert_eq!(c.lon, None);
    }
}
