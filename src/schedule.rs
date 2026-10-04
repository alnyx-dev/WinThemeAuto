use crate::{
    config::{Config, Mode},
    i18n::{self, Lang},
    sun::{self, Sun},
    theme::Theme,
};
use chrono::{DateTime, Duration, Local, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};

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

pub struct NextSwitch {
    pub dark: bool,
    pub at: DateTime<Local>,
}

pub fn hold_active(cfg: &Config, want: Theme, now: DateTime<Local>) -> bool {
    matches!(
        (cfg.manual_hold, cfg.manual_hold_until),
        (Some(held), until) if held != want && until.map(|u| now < u).unwrap_or(true)
    )
}

pub fn next_switch(cfg: &Config, now: DateTime<Local>) -> Option<NextSwitch> {
    if !cfg.auto_enabled {
        return None;
    }
    if !cfg.change_apps && !cfg.change_system {
        return None;
    }
    match cfg.mode {
        Mode::Fixed => {
            let (light_t, dark_t) = (cfg.light_at, cfg.dark_at);
            if light_t == dark_t {
                return None;
            }
            let current = theme_for(now.time(), light_t, dark_t);
            let (dark, next_t) = if current == Theme::Light {
                (true, dark_t)
            } else {
                (false, light_t)
            };
            Some(NextSwitch {
                dark,
                at: resolve_target(now, next_t),
            })
        }
        Mode::Sun => {
            let (lat, lon) = coords(cfg)?;
            let today = now.date_naive();
            let t = now.time();
            match day_times(cfg, today) {
                None => {
                    let polar_day = matches!(sun::sun_events(lat, lon, today), Sun::PolarDay);
                    let tomorrow = today.succ_opt().unwrap_or(today);
                    let (l2, d2) = day_times(cfg, tomorrow)?;
                    let (dark, next_t) = if polar_day { (true, d2) } else { (false, l2) };
                    Some(NextSwitch {
                        dark,
                        at: resolve_target(now, next_t),
                    })
                }
                Some((light_t, dark_t)) => {
                    if light_t == dark_t {
                        return None;
                    }
                    let current = theme_for(t, light_t, dark_t);
                    if current == Theme::Light || t < light_t {
                        let (dark, next_t) = if current == Theme::Light {
                            (true, dark_t)
                        } else {
                            (false, light_t)
                        };
                        Some(NextSwitch {
                            dark,
                            at: resolve_target(now, next_t),
                        })
                    } else {
                        let tomorrow = today.succ_opt().unwrap_or(today);
                        if let Some((l2, _)) = day_times(cfg, tomorrow) {
                            Some(NextSwitch {
                                dark: false,
                                at: resolve_target(now, l2),
                            })
                        } else {
                            let (dark, next_t) = if current == Theme::Light {
                                (true, dark_t)
                            } else {
                                (false, light_t)
                            };
                            Some(NextSwitch {
                                dark,
                                at: resolve_target(now, next_t),
                            })
                        }
                    }
                }
            }
        }
    }
}

pub fn next_switch_info(cfg: &Config, now: DateTime<Local>) -> String {
    if let Some(sw) = next_switch(cfg, now) {
        let lang = lang_of(cfg);
        let name = i18n::next_theme_word(lang, sw.dark);
        let mins = (sw.at - now).num_minutes().max(0);
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
            Lang::En => format!("Next: {} at {} ({})", name, hm(sw.at.time()), when),
            Lang::Ru => format!("Далее: {} в {} ({})", name, hm(sw.at.time()), when),
        };
    }
    if !cfg.auto_enabled {
        return String::new();
    }
    if !cfg.change_apps && !cfg.change_system {
        return String::new();
    }
    if cfg.mode == Mode::Fixed {
        return String::new();
    }
    let lang = lang_of(cfg);
    let Some((lat, lon)) = coords(cfg) else {
        return i18n::msg(lang, "need_coords_short", "");
    };
    match sun::sun_events(lat, lon, now.date_naive()) {
        Sun::PolarDay => match lang {
            Lang::En => "Polar day — stays light (sun never sets)".into(),
            Lang::Ru => "Полярный день — остаётся светлая (солнце не заходит)".into(),
        },
        Sun::PolarNight => match lang {
            Lang::En => "Polar night — stays dark (sun never rises)".into(),
            Lang::Ru => "Полярная ночь — остаётся тёмная (солнце не восходит)".into(),
        },
        Sun::Normal { .. } => String::new(),
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
    resolve_target_in(now, next_t)
}

fn resolve_target_in<Tz: TimeZone>(now: DateTime<Tz>, next_t: NaiveTime) -> DateTime<Tz> {
    let date = now.date_naive();
    let tz = now.timezone();
    match date.and_time(next_t).and_local_timezone(tz.clone()) {
        LocalResult::Single(dt) if dt > now => return dt,
        // Fall-back day: the wall time happens twice — take whichever
        // occurrence is still upcoming.
        LocalResult::Ambiguous(first, second) => {
            if first > now {
                return first;
            }
            if second > now {
                return second;
            }
        }
        _ => {}
    }
    let tomorrow = date.succ_opt().unwrap_or(date);
    match tomorrow.and_time(next_t).and_local_timezone(tz.clone()) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => now,
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

    fn minute(m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(m / 60, m % 60, 0).unwrap()
    }

    #[test]
    fn switches_only_at_boundaries() {
        // Walk every minute over 48 h: the theme may only flip exactly at
        // light_at / dark_at, twice per day.
        for (lh, lm, dh, dm) in [
            (7, 0, 19, 0),
            (20, 0, 6, 0),
            (0, 0, 0, 1),
            (23, 30, 23, 31),
            (0, 0, 12, 0),
            (12, 0, 0, 0),
        ] {
            let (l, d) = (t(lh, lm), t(dh, dm));
            if l == d {
                for m in 0..1440 {
                    assert_eq!(theme_for(minute(m), l, d), Theme::Dark);
                }
                continue;
            }
            assert_eq!(theme_for(l, l, d), Theme::Light);
            assert_eq!(theme_for(d, l, d), Theme::Dark);
            let (lm, dm) = (lh * 60 + lm, dh * 60 + dm);
            let mut expected = vec![lm, dm, lm + 1440, dm + 1440];
            expected.retain(|&m| m != 0);
            expected.sort_unstable();
            let mut changes = vec![];
            let mut prev = theme_for(minute(0), l, d);
            for m in 1..2880 {
                let cur = theme_for(minute(m % 1440), l, d);
                if cur != prev {
                    changes.push(m);
                    prev = cur;
                }
            }
            assert_eq!(changes, expected, "bounds {l:?} {d:?}");
        }
    }

    mod dst {
        use super::resolve_target_in;
        use chrono::{LocalResult, NaiveDate, NaiveTime};
        use chrono_tz::America::New_York;

        fn at(y: i32, mo: u32, d: u32, h: u32, m: u32) -> chrono::DateTime<chrono_tz::Tz> {
            let date = NaiveDate::from_ymd_opt(y, mo, d).unwrap();
            let time = NaiveTime::from_hms_opt(h, m, 0).unwrap();
            match date.and_time(time).and_local_timezone(New_York) {
                LocalResult::Single(dt) => dt,
                LocalResult::Ambiguous(first, _) => first,
                LocalResult::None => panic!("test time does not exist"),
            }
        }

        #[test]
        fn spring_forward_skips_missing_time() {
            // 2024-03-10: 02:00–03:00 does not exist in New York.
            let now = at(2024, 3, 10, 0, 30);
            let next = resolve_target_in(now, NaiveTime::from_hms_opt(2, 30, 0).unwrap());
            assert_eq!(
                next.date_naive(),
                NaiveDate::from_ymd_opt(2024, 3, 11).unwrap()
            );
            assert_eq!(next.format("%H:%M%:z").to_string(), "02:30-04:00");
        }

        #[test]
        fn fall_back_takes_first_occurrence() {
            // 2024-11-03: 01:30 happens twice; the EDT one comes first.
            let now = at(2024, 11, 3, 0, 30);
            let next = resolve_target_in(now, NaiveTime::from_hms_opt(1, 30, 0).unwrap());
            assert_eq!(
                next.date_naive(),
                NaiveDate::from_ymd_opt(2024, 11, 3).unwrap()
            );
            assert_eq!(next.format("%H:%M%:z").to_string(), "01:30-04:00");
            // Between the two occurrences the second (EST) one is next.
            let between = next + chrono::Duration::minutes(30);
            let second = resolve_target_in(between, NaiveTime::from_hms_opt(1, 30, 0).unwrap());
            assert_eq!(
                second.date_naive(),
                NaiveDate::from_ymd_opt(2024, 11, 3).unwrap()
            );
            assert_eq!(second.format("%H:%M%:z").to_string(), "01:30-05:00");
            // Once both have passed, it rolls over to the next day.
            let past = second + chrono::Duration::minutes(1);
            let rolled = resolve_target_in(past, NaiveTime::from_hms_opt(1, 30, 0).unwrap());
            assert_eq!(
                rolled.date_naive(),
                NaiveDate::from_ymd_opt(2024, 11, 4).unwrap()
            );
        }
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
        let c = cfg_sun(true, Some(0.0), Some(0.0));
        let now = dt(12, 0);
        assert!(desired_theme(&c, now).is_some());
        assert!(!sun_info(&c, now.date_naive()).contains("Enter coordinates"));
    }

    #[test]
    fn hold_honored_until_expiry_or_agreement() {
        let now = dt(12, 0);
        let later = now + Duration::hours(2);
        let past = now - Duration::hours(2);
        let mut c = cfg_fixed(true, t(7, 0), t(19, 0));
        // At noon the schedule wants light; a dark hold is honored.
        c.manual_hold = Some(Theme::Dark);
        c.manual_hold_until = Some(later);
        assert!(hold_active(&c, Theme::Light, now));
        // Hold for the scheduled theme itself is not a hold.
        c.manual_hold = Some(Theme::Light);
        assert!(!hold_active(&c, Theme::Light, now));
        // Expired hold is not honored.
        c.manual_hold = Some(Theme::Dark);
        c.manual_hold_until = Some(past);
        assert!(!hold_active(&c, Theme::Light, now));
        // Missing expiry means indefinite hold.
        c.manual_hold_until = None;
        assert!(hold_active(&c, Theme::Light, now));
        // No hold at all.
        c.manual_hold = None;
        assert!(!hold_active(&c, Theme::Light, now));
    }
}
