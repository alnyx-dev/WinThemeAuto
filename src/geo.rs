use anyhow::{bail, Result};
use serde::Deserialize;
use std::time::Duration;

const URL: &str = "https://ipwho.is/";

#[derive(Debug, Clone)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub place: String,
}

#[derive(Deserialize)]
struct Resp {
    success: Option<bool>,
    message: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    city: Option<String>,
    country: Option<String>,
}

pub fn detect_by_ip() -> Result<Location> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .build();
    let body = agent.get(URL).call()?.into_string()?;
    parse(&body)
}

fn parse(body: &str) -> Result<Location> {
    let r: Resp = serde_json::from_str(body)?;
    if r.success == Some(false) {
        bail!(r.message.unwrap_or_else(|| "service returned an error".into()));
    }
    let (Some(lat), Some(lon)) = (r.latitude, r.longitude) else {
        bail!("response has no coordinates");
    };
    let place = [r.city, r.country]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    Ok(Location {
        lat: round4(lat),
        lon: round4(lon),
        place,
    })
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ok() {
        let json = r#"{"ip":"1.2.3.4","success":true,"country":"Netherlands",
                       "city":"Amsterdam","latitude":52.36761,"longitude":4.90412}"#;
        let l = parse(json).unwrap();
        assert_eq!(l.place, "Amsterdam, Netherlands");
        assert_eq!(l.lat, 52.3676);
        assert_eq!(l.lon, 4.9041);
    }

    #[test]
    fn error_response() {
        let e = parse(r#"{"success":false,"message":"Invalid IP address"}"#).unwrap_err();
        assert!(e.to_string().contains("Invalid IP"));
    }
}
