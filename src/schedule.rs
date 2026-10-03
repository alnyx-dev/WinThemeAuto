use crate::{
    config::{Config, Mode},
    i18n::{self, Lang},
    sun::{self, Sun},
    theme::Theme,
};
use chrono::{DateTime, Duration, Local, LocalResult, NaiveDate, NaiveTime, Utc};

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
                        " вЂў light {}, dark {}",
                        hm(shifted(rise, cfg.light_offset_min)),
                        hm(shifted(set, cfg.dark_offset_min))
                    ),
                ),
                Lang::Ru => (
                    format!(
                        "РЎРµРіРѕРґРЅСЏ: РІРѕСЃС…РѕРґ {}, Р·Р°РєР°С‚ {}",
                        hm(shifted(rise, 0)),
                        hm(shifted(set, 0))
                    ),
                    format!(
                        " вЂў СЃРІРµС‚Р»Р°СЏ {}, С‚С‘РјРЅР°СЏ {}",
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
            Lang::Ru => "РџРѕР»СЏСЂРЅС‹Р№ РґРµРЅСЊ: СЃРѕР»РЅС†Рµ РЅРµ Р·Р°С…РѕРґРёС‚".into(),
        },
        Sun::PolarNight => match lang {
            Lang::En => "Polar night: sun never rises".into(),
            Lang::Ru => "РџРѕР»СЏСЂРЅР°СЏ РЅРѕС‡СЊ: СЃРѕР»РЅС†Рµ РЅРµ РІРѕСЃС…РѕРґРёС‚".into(),
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
    match cfg.mode {
        Mode::Fixed => {
            let (light_t, dark_t) = (cfg.light_at, cfg.dark_at);
            if light_t == dark_t {
                return String::new();
            }
            switch_string(lang_of(cfg), now, light_t, dark_t)
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
                    (dark, d2, mins_until(now, d2))
                } else {
                    (light, l2, mins_until(now, l2))
                };
                return match lang {
                    Lang::En => format!(
                        "Next: {} at {} (in {})",
                        name,
                        hm(next_t),
                        i18n::duration_hm(lang, mins)
                    ),
                    Lang::Ru => format!(
                        "Р”Р°Р»РµРµ: {} РІ {} (С‡РµСЂРµР· {})",
                        name,
                        hm(next_t),
                        i18n::duration_hm(lang, mins)
                    ),
                };
            }
            if polar_day {
                match lang {
                    Lang::En => "Polar day вЂ” stays light (sun never sets)".into(),
                    Lang::Ru => "РџРѕР»СЏСЂРЅС‹Р№ РґРµРЅСЊ вЂ” РѕСЃС‚Р°С‘С‚СЃСЏ СЃРІРµС‚Р»Р°СЏ (СЃРѕР»РЅС†Рµ РЅРµ Р·Р°С…РѕРґРёС‚)".into(),
                }
            } else {
                match lang {
                    Lang::En => "Polar night вЂ” stays dark (sun never rises)".into(),
                    Lang::Ru => "РџРѕР»СЏСЂРЅР°СЏ РЅРѕС‡СЊ вЂ” РѕСЃС‚Р°С‘С‚СЃСЏ С‚С‘РјРЅР°СЏ (СЃРѕР»РЅС†Рµ РЅРµ РІРѕСЃС…РѕРґРёС‚)".into(),
                }
            }
        }
        Some((light_t, dark_t)) => {
            if light_t == dark_t {
                return String::new();
            }
            let current = theme_for(t, light_t, dark_t);
            if current == Theme::Light || t < light_t {
                switch_string(lang, now, light_t, dark_t)
            } else {
                let tomorrow = today.succ_opt().unwrap_or(today);
                if let Some((l2, _)) = day_times(cfg, tomorrow) {
                    let mins = mins_until(now, l2);
                    let light = i18n::next_theme_word(lang, false);
                    let when = if mins == 0 {
                        match lang {
                            Lang::En => "now".to_string(),
                            Lang::Ru => "СЃРµР№С‡Р°СЃ".to_string(),
                        }
                    } else {
                        match lang {
                            Lang::En => format!("in {}", i18n::duration_hm(lang, mins)),
                            Lang::Ru => format!("С‡РµСЂРµР· {}", i18n::duration_hm(lang, mins)),
                        }
                    };
                    return match lang {
                        Lang::En => format!("Next: {light} at {} ({when})", hm(l2)),
                        Lang::Ru => format!("Р”Р°Р»РµРµ: {light} РІ {} ({when})", hm(l2)),
                    };
                }
                switch_string(lang, now, light_t, dark_t)
            }
        }
    }
}

fn day_times(cfg: &Config, date: NaiveDate) -> Option<(NaiveTime, NaiveTime)> {
    let (lat, lon) = coords(cfg)?;
    match sun::sun_events(lat, lon, date) {
        Sun::Normal { rise, set } => {
            let shifted_pair = (
                shifted(rise, cfg.light_offset_min),
                shifted(set, cfg.dark_offset_min),
            );
            if shifted_pair.0 == shifted_pair.1 {
                Some((shifted(rise, 0), shifted(set, 0)))
            } else {
                Some(shifted_pair)
            }
        }
        Sun::PolarDay | Sun::PolarNight => None,
    }
}

fn resolve_target(now: DateTime<Local>, next_t: NaiveTime) -> DateTime<Local> {
    let date = now.date_naive();
    if let LocalResult::Single(dt) = date.and_time(next_t).and_local_timezone(Local) {
        if dt > now {
            return dt;
        }
    }
    let tomorrow = date.succ_opt().unwrap_or(date);
    match tomorrow.and_time(next_t).and_local_timezone(Local) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => now,
    }
}

fn mins_until(now: DateTime<Local>, next_t: NaiveTime) -> i64 {
    (resolve_target(now, next_t) - now).num_minutes().max(0)
}

fn switch_string(
    lang: Lang,
    now: DateTime<Local>,
    light_t: NaiveTime,
    dark_t: NaiveTime,
) -> String {
    let t = now.time();
    let current = theme_for(t, light_t, dark_t);
    let (next_dark, next_t) = if current == Theme::Light {
        (true, dark_t)
    } else {
        (false, light_t)
    };
    let name = i18n::next_theme_word(lang, next_dark);
    let mins = mins_until(now, next_t);
    let when = if mins == 0 {
        match lang {
            Lang::En => "now".to_string(),
            Lang::Ru => "СЃРµР№С‡Р°СЃ".to_string(),
        }
    } else {
        match lang {
            Lang::En => format!("in {}", i18n::duration_hm(lang, mins)),
            Lang::Ru => format!("С‡РµСЂРµР· {}", i18n::duration_hm(lang, mins)),
        }
    };
    match lang {
        Lang::En => format!("Next: {} at {} ({})", name, hm(next_t), when),
        Lang::Ru => format!("Р”Р°Р»РµРµ: {} РІ {} ({})", name, hm(next_t), when),
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
        assert_eq!(i18n::duration_hm(Lang::Ru, 75), "1 С‡ 15 РјРёРЅ");
    }

    #[test]
    fn next_switch_russian() {
        let mut c = cfg_fixed(true, t(7, 0), t(19, 0));
        c.language = "ru".to_string();
        let s = next_switch_info(&c, dt(12, 0));
        assert!(s.contains("Р”Р°Р»РµРµ:"), "got {s}");
        assert!(s.contains("С‡РµСЂРµР·"), "got {s}");
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
        assert_eq!(next_switch_info(&c, now), "Enter coordinatesвЂ¦");
        assert!(sun_info(&c, now.date_naive()).contains("Enter coordinates"));
    }

    #[test]
    fn sun_zero_zero_is_valid() {
        let c = cfg_sun(true, Some(0.0), Some(0.0));
        let now = dt(12, 0);
        assert!(desired_theme(&c, now).is_some());
        assert!(!sun_info(&c, now.date_naive()).contains("Enter coordinates"));
    }
}
