use anyhow::{bail, Result};
use serde::Deserialize;
use std::time::Duration;

const PRIMARY_URL: &str = "https://ipwho.is/";
const FALLBACK_URL: &str = "https://ipapi.co/json/";

#[derive(Debug, Clone)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub place: String,
}

#[derive(Deserialize)]
struct IpwhoResp {
    success: Option<bool>,
    message: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    city: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize)]
struct IpapiResp {
    latitude: Option<f64>,
    longitude: Option<f64>,
    city: Option<String>,
    country_name: Option<String>,
    error: Option<bool>,
    reason: Option<String>,
}

pub fn detect_by_ip() -> Result<Location> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let primary: Result<String> = agent
        .get(PRIMARY_URL)
        .call()
        .map_err(|e| anyhow::anyhow!("{PRIMARY_URL}: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("{PRIMARY_URL}: {e}"));
    match primary.and_then(|body| parse_ipwho(&body)) {
        Ok(loc) => Ok(loc),
        Err(first) => {
            let second: Result<String> = agent
                .get(FALLBACK_URL)
                .call()
                .map_err(|e| anyhow::anyhow!("{FALLBACK_URL}: {e}"))?
                .body_mut()
                .read_to_string()
                .map_err(|e| anyhow::anyhow!("{FALLBACK_URL}: {e}"));
            match second.and_then(|body| parse_ipapi(&body)) {
                Ok(loc) => {
                    crate::log::info(format!(
                        "geolocation: primary failed ({first:#}), fallback ok"
                    ));
                    Ok(loc)
                }
                Err(e) => {
                    bail!("both geolocation providers failed: [{first:#}] [{e:#}]")
                }
            }
        }
    }
}

fn parse_ipwho(body: &str) -> Result<Location> {
    let r: IpwhoResp = serde_json::from_str(body)?;
    if r.success == Some(false) {
        bail!(r
            .message
            .unwrap_or_else(|| "service returned an error".into()));
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

fn parse_ipapi(body: &str) -> Result<Location> {
    let r: IpapiResp = serde_json::from_str(body)?;
    if r.error == Some(true) {
        bail!(r
            .reason
            .unwrap_or_else(|| "service returned an error".into()));
    }
    let (Some(lat), Some(lon)) = (r.latitude, r.longitude) else {
        bail!("response has no coordinates");
    };
    let place = [r.city, r.country_name]
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
        let l = parse_ipwho(json).unwrap();
        assert_eq!(l.place, "Amsterdam, Netherlands");
        assert_eq!(l.lat, 52.3676);
        assert_eq!(l.lon, 4.9041);
    }

    #[test]
    fn error_response() {
        let e = parse_ipwho(r#"{"success":false,"message":"Invalid IP address"}"#).unwrap_err();
        assert!(e.to_string().contains("Invalid IP"));
    }

    #[test]
    fn parses_ipapi_ok() {
        let json = r#"{"ip":"1.2.3.4","city":"Amsterdam","country_name":"Netherlands",
                       "latitude":52.36761,"longitude":4.90412}"#;
        let l = parse_ipapi(json).unwrap();
        assert_eq!(l.place, "Amsterdam, Netherlands");
        assert_eq!(l.lat, 52.3676);
        assert_eq!(l.lon, 4.9041);
    }

    #[test]
    fn ipapi_error_response() {
        let e = parse_ipapi(r#"{"error":true,"reason":"RateLimited"}"#).unwrap_err();
        assert!(e.to_string().contains("RateLimited"));
        let e = parse_ipapi(r#"{"city":"Nowhere"}"#).unwrap_err();
        assert!(e.to_string().contains("no coordinates"));
    }
}
