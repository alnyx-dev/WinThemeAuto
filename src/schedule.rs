use crate::{
    config::{Config, Mode},
    i18n::{self, Lang},
    sun::{self, Sun},
    theme::Theme,
};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, Timelike, Utc};

pub fn theme_for(now: NaiveTime, light_at: NaiveTime, dark_at: NaiveTime) -> Theme {
    let is_light = if light_at <= dark_at {
        now >= light_at && now < dark_at
    } else {
        !(now >= dark_at && now < light_at)
    };
    if is_light {
        Theme::Light
    } else {
        Theme::Dark
    }
}

fn coords(cfg: &Config) -> Option<(f64, f64)> {
    match (cfg.lat, cfg.lon) {
        (Some(lat), Some(lon)) => Some((lat, lon)),
        _ => None,
    }
}

pub fn desired_theme(cfg: &Config, now: DateTime<Local>) -> Option<Theme> {
    let t = now.time();
    match cfg.mode {
        Mode::Fixed => Some(theme_for(t, cfg.light_at, cfg.dark_at)),
        Mode::Sun => {
            let (lat, lon) = coords(cfg)?;
            match sun::sun_events(lat, lon, now.date_naive()) {
                Sun::Normal { rise, set } => Some(theme_for(
                    t,
                    shifted(rise, cfg.light_offset_min),
                    shifted(set, cfg.dark_offset_min),
                )),
                Sun::PolarDay => Some(Theme::Light),
                Sun::PolarNight => Some(Theme::Dark),
            }
        }
    }
}

fn lang_of(cfg: &Config) -> Lang {
    Lang::from_code(&cfg.language)
}

/// Single-line summary (fits the fixed 14px info line, elided on overflow).
pub fn sun_info(cfg: &Config, date: NaiveDate) -> String {
    let lang = lang_of(cfg);
    let Some((lat, lon)) = coords(cfg) else {
        return i18n::msg(lang, "need_coords", "");
    };
    match sun::sun_events(lat, lon, date) {
        Sun::Normal { rise, set } => {
            let (today, shifted_pair) = match lang {
                Lang::En => (
                    format!(
                        "Today: sunrise {}, sunset {}",
                        hm(shifted(rise, 0)),
                        hm(shifted(set, 0))
                    ),
                    format!(
                        " • light {}, dark {}",
                        hm(shifted(rise, cfg.light_offset_min)),
                        hm(shifted(set, cfg.dark_offset_min))
                    ),
                ),
                Lang::Ru => (
                    format!(
                        "Сегодня: восход {}, закат {}",
                        hm(shifted(rise, 0)),
                        hm(shifted(set, 0))
                    ),
                    format!(
                        " • светлая {}, тёмная {}",
                        hm(shifted(rise, cfg.light_offset_min)),
                        hm(shifted(set, cfg.dark_offset_min))
                    ),
                ),
            };
            if cfg.light_offset_min != 0 || cfg.dark_offset_min != 0 {
                format!("{today}{shifted_pair}")
            } else {
                today
            }
        }
        Sun::PolarDay => match lang {
            Lang::En => "Polar day: sun never sets".into(),
            Lang::Ru => "Полярный день: солнце не заходит".into(),
        },
        Sun::PolarNight => match lang {
            Lang::En => "Polar night: sun never rises".into(),
            Lang::Ru => "Полярная ночь: солнце не восходит".into(),
        },
    }
}

pub fn next_switch_info(cfg: &Config, now: DateTime<Local>) -> String {
    if !cfg.auto_enabled {
        return String::new();
    }
    if !cfg.change_apps && !cfg.change_system {
        return String::new();
    }
    let t = now.time();
    match cfg.mode {
        Mode::Fixed => {
            let (light_t, dark_t) = (cfg.light_at, cfg.dark_at);
            if light_t == dark_t {
                return String::new();
            }
            switch_string(lang_of(cfg), t, light_t, dark_t)
        }
        Mode::Sun => sun_next_switch(cfg, now),
    }
}

fn sun_next_switch(cfg: &Config, now: DateTime<Local>) -> String {
    let lang = lang_of(cfg);
    let Some((lat, lon)) = coords(cfg) else {
        return i18n::msg(lang, "need_coords_short", "");
    };
    let today = now.date_naive();
    let t = now.time();
    let today_times = day_times(cfg, today);

    match today_times {
        None => {
            let polar_day = matches!(sun::sun_events(lat, lon, today), Sun::PolarDay);
            let tomorrow = today.succ_opt().unwrap_or(today);
            if let Some((l2, d2)) = day_times(cfg, tomorrow) {
                let (dark, light) = (
                    i18n::next_theme_word(lang, true),
                    i18n::next_theme_word(lang, false),
                );
                let (name, next_t, mins) = if polar_day {
                    (dark, d2, mins_until_tomorrow(t, d2))
                } else {
                    (light, l2, mins_until_tomorrow(t, l2))
                };
                return match lang {
                    Lang::En => format!(
                        "Next: {} at {} (in {})",
                        name,
                        hm(next_t),
                        i18n::duration_hm(lang, mins)
                    ),
                    Lang::Ru => format!(
                        "Далее: {} в {} (через {})",
                        name,
                        hm(next_t),
                        i18n::duration_hm(lang, mins)
                    ),
                };
            }
            if polar_day {
                match lang {
                    Lang::En => "No switches: polar day".into(),
                    Lang::Ru => "Нет переключений: полярный день".into(),
                }
            } else {
                match lang {
                    Lang::En => "No switches: polar night".into(),
                    Lang::Ru => "Нет переключений: полярная ночь".into(),
                }
            }
        }
        Some((light_t, dark_t)) => {
            if light_t == dark_t {
                return String::new();
            }
            let current = theme_for(t, light_t, dark_t);
            if current == Theme::Light || t < light_t {
                switch_string(lang, t, light_t, dark_t)
            } else {
                let tomorrow = today.succ_opt().unwrap_or(today);
                if let Some((l2, _)) = day_times(cfg, tomorrow) {
                    let mins = mins_until_tomorrow(t, l2);
                    let light = i18n::next_theme_word(lang, false);
                    let when = if mins == 0 {
                        match lang {
                            Lang::En => "now".to_string(),
                            Lang::Ru => "сейчас".to_string(),
                        }
                    } else {
                        match lang {
                            Lang::En => format!("in {}", i18n::duration_hm(lang, mins)),
                            Lang::Ru => format!("через {}", i18n::duration_hm(lang, mins)),
                        }
                    };
                    return match lang {
                        Lang::En => format!("Next: {light} at {} ({when})", hm(l2)),
                        Lang::Ru => format!("Далее: {light} в {} ({when})", hm(l2)),
                    };
                }
                switch_string(lang, t, light_t, dark_t)
            }
        }
    }
}

fn day_times(cfg: &Config, date: NaiveDate) -> Option<(NaiveTime, NaiveTime)> {
    let (lat, lon) = coords(cfg)?;
    match sun::sun_events(lat, lon, date) {
        Sun::Normal { rise, set } => Some((
            shifted(rise, cfg.light_offset_min),
            shifted(set, cfg.dark_offset_min),
        )),
        Sun::PolarDay | Sun::PolarNight => None,
    }
}

fn mins_until_tomorrow(t: NaiveTime, next_t: NaiveTime) -> i64 {
    let cur = t.hour() as i64 * 60 + t.minute() as i64;
    let nxt = next_t.hour() as i64 * 60 + next_t.minute() as i64;
    (24 * 60 - cur) + nxt
}

fn switch_string(lang: Lang, t: NaiveTime, light_t: NaiveTime, dark_t: NaiveTime) -> String {
    let current = theme_for(t, light_t, dark_t);
    let (next_dark, next_t) = if current == Theme::Light {
        (true, dark_t)
    } else {
        (false, light_t)
    };
    let name = i18n::next_theme_word(lang, next_dark);
    let mut mins = next_t.signed_duration_since(t).num_minutes();
    if mins < 0 {
        mins += 24 * 60;
    }
    let when = if mins == 0 {
        match lang {
            Lang::En => "now".to_string(),
            Lang::Ru => "сейчас".to_string(),
        }
    } else {
        match lang {
            Lang::En => format!("in {}", i18n::duration_hm(lang, mins)),
            Lang::Ru => format!("через {}", i18n::duration_hm(lang, mins)),
        }
    };
    match lang {
        Lang::En => format!("Next: {} at {} ({})", name, hm(next_t), when),
        Lang::Ru => format!("Далее: {} в {} ({})", name, hm(next_t), when),
    }
}

fn shifted(dt: DateTime<Utc>, minutes: i32) -> NaiveTime {
    (dt + Duration::minutes(minutes as i64))
        .with_timezone(&Local)
        .time()
}

fn hm(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn normal_day() {
        assert_eq!(theme_for(t(12, 0), t(7, 0), t(19, 0)), Theme::Light);
        assert_eq!(theme_for(t(23, 0), t(7, 0), t(19, 0)), Theme::Dark);
        assert_eq!(theme_for(t(3, 0), t(7, 0), t(19, 0)), Theme::Dark);
    }

    #[test]
    fn wrapped() {
        assert_eq!(theme_for(t(22, 0), t(20, 0), t(6, 0)), Theme::Light);
        assert_eq!(theme_for(t(12, 0), t(20, 0), t(6, 0)), Theme::Dark);
    }

    fn cfg_fixed(auto: bool, light: NaiveTime, dark: NaiveTime) -> Config {
        Config {
            auto_enabled: auto,
            mode: Mode::Fixed,
            light_at: light,
            dark_at: dark,
            ..Config::default()
        }
    }

    fn dt(h: u32, m: u32) -> DateTime<Local> {
        let d = Local::now().date_naive();
        d.and_time(t(h, m)).and_local_timezone(Local).unwrap()
    }

    #[test]
    fn next_switch_disabled_is_empty() {
        let c = cfg_fixed(false, t(7, 0), t(19, 0));
        assert_eq!(next_switch_info(&c, dt(12, 0)), "");
    }

    #[test]
    fn next_switch_fixed() {
        let c = cfg_fixed(true, t(7, 0), t(19, 0));
        let s = next_switch_info(&c, dt(12, 0));
        assert!(s.contains("dark at 19:00"), "got {s}");
        assert!(s.contains("in 7 h"), "got {s}");
        let s = next_switch_info(&c, dt(20, 0));
        assert!(s.contains("light at 07:00"), "got {s}");
    }

    #[test]
    fn duration_fmt() {
        assert_eq!(i18n::duration_hm(Lang::En, 45), "45 min");
        assert_eq!(i18n::duration_hm(Lang::En, 60), "1 h");
        assert_eq!(i18n::duration_hm(Lang::En, 75), "1 h 15 min");
        assert_eq!(i18n::duration_hm(Lang::Ru, 75), "1 ч 15 мин");
    }

    #[test]
    fn next_switch_russian() {
        let mut c = cfg_fixed(true, t(7, 0), t(19, 0));
        c.language = "ru".to_string();
        let s = next_switch_info(&c, dt(12, 0));
        assert!(s.contains("Далее:"), "got {s}");
        assert!(s.contains("через"), "got {s}");
    }

    fn cfg_sun(auto: bool, lat: Option<f64>, lon: Option<f64>) -> Config {
        Config {
            auto_enabled: auto,
            mode: Mode::Sun,
            lat,
            lon,
            ..Config::default()
        }
    }

    #[test]
    fn sun_without_location_is_none() {
        let c = cfg_sun(true, None, None);
        let now = dt(12, 0);
        assert_eq!(desired_theme(&c, now), None);
        assert_eq!(next_switch_info(&c, now), "Enter coordinates…");
        assert!(sun_info(&c, now.date_naive()).contains("Enter coordinates"));
    }

    #[test]
    fn sun_zero_zero_is_valid() {
        // Gulf of Guinea — must compute, not be treated as unset.
        let c = cfg_sun(true, Some(0.0), Some(0.0));
        let now = dt(12, 0);
        assert!(desired_theme(&c, now).is_some());
        assert!(!sun_info(&c, now.date_naive()).contains("Enter coordinates"));
    }
}
