use anyhow::{Context, Result};
use serde::Deserialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API_URL: &str = "https://api.github.com/repos/alnyx-dev/WinThemeAuto/releases/latest";

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

pub fn open_releases_page() {
    const URL: &str = "https://github.com/alnyx-dev/WinThemeAuto/releases";
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", URL])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
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
    #[serde(default)]
    digest: Option<String>,
}

pub struct UpdateInfo {
    pub latest: String,
    pub download_url: String,
    pub expected_sha256: String,
    pub is_newer: bool,
}

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

    let rel: Release = serde_json::from_str(&body).context("cannot parse release info")?;
    let want = asset_name();
    let asset = rel
        .assets
        .iter()
        .find(|a| a.name == want)
        .with_context(|| format!("release {} has no file {want}", rel.tag_name))?;

    let mut expected = asset
        .digest
        .as_deref()
        .and_then(parse_digest)
        .map(str::to_string);
    if expected.is_none() {
        let sha_name = format!("{want}.sha256");
        if let Some(sha_asset) = rel.assets.iter().find(|a| a.name == sha_name) {
            expected = Some(fetch_sha256(&agent, &sha_asset.browser_download_url)?);
        }
    }
    let expected = expected.with_context(|| {
        format!(
            "release {} has no checksum for {want} вЂ” download manually",
            rel.tag_name
        )
    })?;

    Ok(UpdateInfo {
        latest: rel.tag_name.trim_start_matches('v').to_string(),
        download_url: asset.browser_download_url.clone(),
        expected_sha256: expected,
        is_newer: is_newer(&rel.tag_name, current_version()),
    })
}

fn parse_digest(digest: &str) -> Option<&str> {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest).trim();
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(hex)
    } else {
        None
    }
}

fn fetch_sha256(agent: &ureq::Agent, url: &str) -> Result<String> {
    let body = agent
        .get(url)
        .set("User-Agent", &user_agent())
        .call()
        .map_err(|e| anyhow::anyhow!("checksum download failed: {e}"))?
        .into_string()?;
    let hex = body.split_whitespace().next().unwrap_or("").to_lowercase();
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(hex)
    } else {
        Err(anyhow::anyhow!("bad checksum file"))
    }
}

pub fn sha256_hex(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn download(
    info: &UpdateInfo,
    cancel: &std::sync::atomic::AtomicBool,
    on_progress: impl Fn(u64, Option<u64>),
) -> Result<PathBuf> {
    use std::sync::atomic::Ordering;
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(180))
        .build();
    let resp = agent
        .get(&info.download_url)
        .set("User-Agent", &user_agent())
        .call()
        .map_err(|e| anyhow::anyhow!("download failed: {e}"))?;

    let total: Option<u64> = resp.header("Content-Length").and_then(|v| v.parse().ok());

    let dest = std::env::temp_dir().join("WinThemeAuto-update.exe");
    let mut file = std::fs::File::create(&dest)?;
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 16384];
    let mut done: u64 = 0;
    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&dest);
            anyhow::bail!("cancelled by user");
        }
        let n = std::io::Read::read(&mut reader, &mut buf)?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])?;
        done += n as u64;
        on_progress(done, total);
    }
    drop(file);

    let actual = sha256_hex(&dest)?;
    if actual.to_lowercase() != info.expected_sha256.to_lowercase() {
        let _ = std::fs::remove_file(&dest);
        anyhow::bail!(
            "checksum mismatch: expected {}, got {}",
            info.expected_sha256,
            actual
        );
    }
    Ok(dest)
}

pub fn self_install(new_exe: &Path) -> Result<()> {
    let current = std::env::current_exe().context("cannot locate current exe")?;
    let pid = std::process::id();
    let restart_args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let script_body = build_updater_script(&current, new_exe, pid, &restart_args);

    let script = std::env::temp_dir().join("WinThemeAuto-do-update.bat");
    std::fs::write(&script, script_body)?;

    std::process::Command::new("cmd")
        .arg("/C")
        .arg(&script)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("cannot start updater")?;
    Ok(())
}

fn cmd_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn build_updater_script(
    current: &Path,
    new_exe: &Path,
    pid: u32,
    restart_args: &[String],
) -> String {
    let cur_q = cmd_quote(&current.to_string_lossy());
    let new_q = cmd_quote(&new_exe.to_string_lossy());
    let mut start_cur = format!("start \"\" {cur_q}");
    let mut start_new = format!("start \"\" {new_q}");
    for a in restart_args {
        let q = cmd_quote(a);
        start_cur.push(' ');
        start_cur.push_str(&q);
        start_new.push(' ');
        start_new.push_str(&q);
    }
    format!(
        "@echo off\r\n\
         set \"PID={pid}\"\r\n\
         set /A N=0\r\n\
         :wait\r\n\
         tasklist /FI \"PID eq %PID%\" 2>nul | find \"%PID%\" >nul\r\n\
         if errorlevel 1 goto exited\r\n\
         set /A N+=1\r\n\
         if %N% GEQ 90 goto exited\r\n\
         ping -n 2 127.0.0.1 >nul\r\n\
         goto wait\r\n\
         :exited\r\n\
         set /A TRIES=0\r\n\
         :replace\r\n\
         move /Y {new_q} {cur_q} >nul\r\n\
         if not errorlevel 1 goto replaced\r\n\
         set /A TRIES+=1\r\n\
         if %TRIES% GEQ 30 goto replace_failed\r\n\
         ping -n 2 127.0.0.1 >nul\r\n\
         goto replace\r\n\
         :replaced\r\n\
         {start_cur}\r\n\
         del \"%~f0\"\r\n\
         goto :eof\r\n\
         :replace_failed\r\n\
         rem Replace failed (locked file?) вЂ” run the new build from its temp\r\n\
         rem path instead of silently booting the stale exe; the next check\r\n\
         rem retries the replace.\r\n\
         {start_new}\r\n\
         del \"%~f0\"\r\n"
    )
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
            {"name":"WinThemeAuto-x64.exe","browser_download_url":"https://example.com/x64.exe","digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"},
            {"name":"WinThemeAuto-x86.exe","browser_download_url":"https://example.com/x86.exe"}]}"#;
        let rel: Release = serde_json::from_str(json).unwrap();
        assert_eq!(rel.tag_name, "v0.2.0");
        let want = if cfg!(target_arch = "x86_64") {
            "WinThemeAuto-x64.exe"
        } else {
            "WinThemeAuto-x86.exe"
        };
        let asset = rel.assets.iter().find(|a| a.name == want).unwrap();
        assert!(asset
            .browser_download_url
            .starts_with("https://example.com/"));
        let old_json = r#"{"tag_name":"v0.1.0","assets":[
            {"name":"WinThemeAuto-x64.exe","browser_download_url":"https://example.com/x64.exe"}]}"#;
        let old: Release = serde_json::from_str(old_json).unwrap();
        assert!(old.assets[0].digest.is_none());
    }

    #[test]
    fn digest_parses() {
        let hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(parse_digest(&format!("sha256:{hex}")), Some(hex));
        assert_eq!(parse_digest(hex), Some(hex));
        assert!(parse_digest("sha256:xyz").is_none());
        assert!(parse_digest("").is_none());
    }

    #[test]
    fn human_size_formats() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(12_582_912), "12.0 MB");
    }

    #[test]
    fn sha256_of_empty_file() {
        let path = std::env::temp_dir().join("winthemeauto-sha-test.bin");
        std::fs::write(&path, []).unwrap();
        let hex = sha256_hex(&path).unwrap();
        assert_eq!(
            hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cmd_quote_wraps_and_escapes() {
        assert_eq!(cmd_quote("--tray"), "\"--tray\"");
        assert_eq!(
            cmd_quote(r"C:\My Apps\tool.exe"),
            "\"C:\\My Apps\\tool.exe\""
        );
    }

    #[test]
    fn updater_script_embeds_everything() {
        let cur = Path::new(r"C:\Apps\WinThemeAuto-x64.exe");
        let new = Path::new(r"C:\Temp\WinThemeAuto-update.exe");
        let args = [
            "--tray".to_string(),
            "--extra".to_string(),
            "a b".to_string(),
        ];
        let body = build_updater_script(cur, new, 1234, &args);
        assert!(!body.contains("%1"), "must not use %n params");
        assert!(body.contains(
            r#"move /Y "C:\Temp\WinThemeAuto-update.exe" "C:\Apps\WinThemeAuto-x64.exe""#
        ));
        for a in [&"--tray", &"--extra", "\"a b\""] {
            assert!(body.contains(a), "missing {a}");
        }
        let starts: Vec<&str> = body
            .lines()
            .filter(|l| l.starts_with("start \"\""))
            .collect();
        assert_eq!(starts.len(), 2);
        assert!(starts[0].contains(r#""C:\Apps\WinThemeAuto-x64.exe""#));
        assert!(starts[1].contains(r#""C:\Temp\WinThemeAuto-update.exe""#));
        assert!(body.contains("GEQ 90"));
        assert!(body.contains("set \"PID=1234\""));
    }
}
