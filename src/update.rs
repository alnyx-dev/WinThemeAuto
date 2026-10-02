//! Self-updates via GitHub Releases.
//!
//! Flow: check latest release tag -> compare with our version ->
//! download the exe matching our architecture -> spawn a small updater
//! script that waits for this process to exit, swaps the exe and
//! restarts the app.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API_URL: &str = "https://api.github.com/repos/alnyx-dev/WinThemeAuto/releases/latest";

/// Hides the updater console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn user_agent() -> String {
    format!("WinThemeAuto/{}", current_version())
}

fn asset_name() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "WinThemeAuto-x64.exe"
    } else {
        "WinThemeAuto-x86.exe"
    }
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub struct UpdateInfo {
    /// Latest version without leading `v`, e.g. `"0.2.0"`.
    pub latest: String,
    pub download_url: String,
    pub is_newer: bool,
}

/// Parse `"v1.2.3"`, `"1.2"`, `"0.2.0-beta.1"` into `(major, minor, patch)`.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim();
    let s = s.strip_prefix('v').unwrap_or(s);
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let mut parts = core.split('.');
    let num = |p: Option<&str>| p.unwrap_or("0").trim().parse::<u64>().ok();
    Some((num(parts.next())?, num(parts.next())?, num(parts.next())?))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Query GitHub for the latest release and the asset matching our arch.
pub fn check() -> Result<UpdateInfo> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let body = agent
        .get(API_URL)
        .set("User-Agent", &user_agent())
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| anyhow::anyhow!("update check failed: {e}"))?
        .into_string()?;

    let rel: Release =
        serde_json::from_str(&body).context("cannot parse release info")?;
    let want = asset_name();
    let asset = rel
        .assets
        .iter()
        .find(|a| a.name == want)
        .with_context(|| format!("release {} has no file {want}", rel.tag_name))?;

    Ok(UpdateInfo {
        latest: rel.tag_name.trim_start_matches('v').to_string(),
        download_url: asset.browser_download_url.clone(),
        is_newer: is_newer(&rel.tag_name, current_version()),
    })
}

/// Download the new exe into the temp dir. Returns its path.
pub fn download(url: &str) -> Result<PathBuf> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(180))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", &user_agent())
        .call()
        .map_err(|e| anyhow::anyhow!("download failed: {e}"))?;

    let dest = std::env::temp_dir().join("WinThemeAuto-update.exe");
    let mut file = std::fs::File::create(&dest)?;
    std::io::copy(&mut resp.into_reader(), &mut file)?;
    Ok(dest)
}

/// Spawn a hidden updater script that waits for this process to exit,
/// replaces the current exe with `new_exe` and restarts the app.
/// The caller should exit right after this returns `Ok`.
pub fn self_install(new_exe: &Path) -> Result<()> {
    let current = std::env::current_exe().context("cannot locate current exe")?;
    let pid = std::process::id();
    let restart_args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();

    // %1=pid %2=new exe %3=current exe %4..=restart args
    let script_body = "@echo off\r\n\
         set PID=%1\r\n\
         set NEW=%2\r\n\
         set CUR=%3\r\n\
         shift & shift & shift\r\n\
         :wait\r\n\
         tasklist /FI \"PID eq %PID%\" 2>nul | find \"%PID%\" >nul\r\n\
         if not errorlevel 1 (ping -n 2 127.0.0.1 >nul & goto wait)\r\n\
         set TRIES=0\r\n\
         :replace\r\n\
         move /Y %NEW% %CUR% >nul\r\n\
         if errorlevel 1 (set /A TRIES+=1 & if %TRIES% LSS 30 (ping -n 2 127.0.0.1 >nul & goto replace))\r\n\
         start \"\" %CUR% %1 %2 %3 %4 %5 %6 %7 %8 %9\r\n\
         del \"%~f0\"\r\n";

    let script = std::env::temp_dir().join("WinThemeAuto-do-update.bat");
    std::fs::write(&script, script_body)?;

    let mut cmd = std::process::Command::new("cmd");
    cmd.arg("/C")
        .arg(&script)
        .arg(pid.to_string())
        .arg(new_exe)
        .arg(&current)
        .args(&restart_args)
        .creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().context("cannot start updater")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse() {
        assert_eq!(parse_version("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.10.3"), Some((1, 10, 3)));
        assert_eq!(parse_version("v2.0"), Some((2, 0, 0)));
        assert_eq!(parse_version("0.2.0-beta.1"), Some((0, 2, 0)));
        assert_eq!(parse_version("  v0.1.0  "), Some((0, 1, 0)));
        assert!(parse_version("oops").is_none());
        assert!(parse_version("").is_none());
    }

    #[test]
    fn version_compare() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.2.0"));
        assert!(!is_newer("oops", "0.1.0"));
    }

    #[test]
    fn release_json_parses() {
        let json = r#"{"tag_name":"v0.2.0","assets":[
            {"name":"WinThemeAuto-x64.exe","browser_download_url":"https://example.com/x64.exe"},
            {"name":"WinThemeAuto-x86.exe","browser_download_url":"https://example.com/x86.exe"}]}"#;
        let rel: Release = serde_json::from_str(json).unwrap();
        assert_eq!(rel.tag_name, "v0.2.0");
        let want = if cfg!(target_arch = "x86_64") {
            "WinThemeAuto-x64.exe"
        } else {
            "WinThemeAuto-x86.exe"
        };
        let asset = rel.assets.iter().find(|a| a.name == want).unwrap();
        assert!(asset.browser_download_url.starts_with("https://example.com/"));
    }
}
