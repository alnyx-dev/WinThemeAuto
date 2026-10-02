use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone, Copy)]
pub enum Sun {
    Normal { rise: DateTime<Utc>, set: DateTime<Utc> },
    PolarDay,
    PolarNight,
}

pub fn sun_events(lat: f64, lon: f64, date: NaiveDate) -> Sun {
    let epoch = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let n = (date - epoch).num_days() as f64;
    let lat = lat.clamp(-89.9, 89.9);

    let j_star = n - lon / 360.0;

    let m = (357.5291 + 0.98560028 * j_star).rem_euclid(360.0).to_radians();
    let c = 1.9148 * m.sin() + 0.0200 * (2.0 * m).sin() + 0.0003 * (3.0 * m).sin();

    let lambda = (m.to_degrees() + c + 180.0 + 102.9372)
        .rem_euclid(360.0)
        .to_radians();

    let transit = 2_451_545.0 + j_star + 0.0053 * m.sin() - 0.0069 * (2.0 * lambda).sin();

    let dec = (lambda.sin() * 23.4397_f64.to_radians().sin()).asin();

    let phi = lat.to_radians();
    let cos_h = ((-0.833_f64).to_radians().sin() - phi.sin() * dec.sin())
        / (phi.cos() * dec.cos());

    if cos_h > 1.0 {
        return Sun::PolarNight;
    }
    if cos_h < -1.0 {
        return Sun::PolarDay;
    }

    let h = cos_h.acos().to_degrees();
    Sun::Normal {
        rise: jd_to_utc(transit - h / 360.0),
        set: jd_to_utc(transit + h / 360.0),
    }
}

fn jd_to_utc(jd: f64) -> DateTime<Utc> {
    let secs = ((jd - 2_440_587.5) * 86_400.0).round() as i64;
    DateTime::from_timestamp(secs, 0).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn equator_equinox_is_about_12h() {
        let Sun::Normal { rise, set } = sun_events(0.0, 0.0, d(2024, 3, 20)) else {
            panic!("expected normal day");
        };
        let mins = (set - rise).num_minutes();
        assert!((715..=740).contains(&mins), "got {mins}");
    }

    #[test]
    fn london_midsummer() {
        let Sun::Normal { rise, set } = sun_events(51.5, -0.12, d(2024, 6, 21)) else {
            panic!("expected normal day");
        };
        let mins = (set - rise).num_minutes();
        assert!((985..=1010).contains(&mins), "got {mins}");
        assert!((3..=4).contains(&rise.hour()));
        assert!((20..=21).contains(&set.hour()));
    }

    #[test]
    fn polar_cases() {
        assert!(matches!(sun_events(80.0, 0.0, d(2024, 6, 21)), Sun::PolarDay));
        assert!(matches!(sun_events(80.0, 0.0, d(2024, 12, 21)), Sun::PolarNight));
    }
}
