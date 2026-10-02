use crate::{
    config::{Config, Mode},
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
    if is_light { Theme::Light } else { Theme::Dark }
}

pub fn desired_theme(cfg: &Config, now: DateTime<Local>) -> Theme {
    let t = now.time();
    match cfg.mode {
        Mode::Fixed => theme_for(t, cfg.light_at, cfg.dark_at),
        Mode::Sun => match sun::sun_events(cfg.lat, cfg.lon, now.date_naive()) {
            Sun::Normal { rise, set } => theme_for(
                t,
                shifted(rise, cfg.light_offset_min),
                shifted(set, cfg.dark_offset_min),
            ),
            Sun::PolarDay => Theme::Light,
            Sun::PolarNight => Theme::Dark,
        },
    }
}

pub fn sun_info(cfg: &Config, date: NaiveDate) -> String {
    match sun::sun_events(cfg.lat, cfg.lon, date) {
        Sun::Normal { rise, set } => {
            let mut s = format!(
                "Today: sunrise {}, sunset {}",
                hm(shifted(rise, 0)),
                hm(shifted(set, 0))
            );
            if cfg.light_offset_min != 0 || cfg.dark_offset_min != 0 {
                s += &format!(
                    "\nLight from {}, dark from {}",
                    hm(shifted(rise, cfg.light_offset_min)),
                    hm(shifted(set, cfg.dark_offset_min))
                );
            }
            s
        }
        Sun::PolarDay => "Polar day: sun never sets".into(),
        Sun::PolarNight => "Polar night: sun never rises".into(),
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
            switch_string(t, light_t, dark_t)
        }
        Mode::Sun => sun_next_switch(cfg, now),
    }
}

fn sun_next_switch(cfg: &Config, now: DateTime<Local>) -> String {
    let today = now.date_naive();
    let t = now.time();
    let today_times = day_times(cfg, today);

    match today_times {
        None => {
            let polar_day = matches!(
                sun::sun_events(cfg.lat, cfg.lon, today),
                Sun::PolarDay
            );
            let tomorrow = today.succ_opt().unwrap_or(today);
            if let Some((l2, d2)) = day_times(cfg, tomorrow) {
                let (name, next_t, mins) = if polar_day {
                    ("dark", d2, mins_until_tomorrow(t, d2))
                } else {
                    ("light", l2, mins_until_tomorrow(t, l2))
                };
                return format!(
                    "Next: {} at {} (in {})",
                    name,
                    hm(next_t),
                    duration_hm(mins)
                );
            }
            if polar_day {
                "No switches: polar day".into()
            } else {
                "No switches: polar night".into()
            }
        }
        Some((light_t, dark_t)) => {
            if light_t == dark_t {
                return String::new();
            }
            let current = theme_for(t, light_t, dark_t);
            if current == Theme::Light || t < light_t {
                switch_string(t, light_t, dark_t)
            } else {
                let tomorrow = today.succ_opt().unwrap_or(today);
                if let Some((l2, _)) = day_times(cfg, tomorrow) {
                    let mins = mins_until_tomorrow(t, l2);
                    let when = if mins == 0 {
                        "now".to_string()
                    } else {
                        format!("in {}", duration_hm(mins))
                    };
                    return format!("Next: light at {} ({})", hm(l2), when);
                }
                switch_string(t, light_t, dark_t)
            }
        }
    }
}

fn day_times(cfg: &Config, date: NaiveDate) -> Option<(NaiveTime, NaiveTime)> {
    match sun::sun_events(cfg.lat, cfg.lon, date) {
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

fn switch_string(t: NaiveTime, light_t: NaiveTime, dark_t: NaiveTime) -> String {
    let current = theme_for(t, light_t, dark_t);
    let (next_theme, next_t) = if current == Theme::Light {
        ("dark", dark_t)
    } else {
        ("light", light_t)
    };
    let mut mins = next_t.signed_duration_since(t).num_minutes();
    if mins < 0 {
        mins += 24 * 60;
    }
    let when = if mins == 0 {
        "now".to_string()
    } else {
        format!("in {}", duration_hm(mins))
    };
    format!("Next: {} at {} ({})", next_theme, hm(next_t), when)
}

fn duration_hm(mins: i64) -> String {
    if mins < 60 {
        format!("{mins} min")
    } else if mins % 60 == 0 {
        format!("{} h", mins / 60)
    } else {
        format!("{} h {} min", mins / 60, mins % 60)
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
        assert_eq!(duration_hm(45), "45 min");
        assert_eq!(duration_hm(60), "1 h");
        assert_eq!(duration_hm(75), "1 h 15 min");
    }
}
