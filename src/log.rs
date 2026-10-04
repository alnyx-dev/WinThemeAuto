use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 256 * 1024;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("WinThemeAuto"))
}

pub fn path() -> Option<PathBuf> {
    dir().map(|d| d.join("app.log"))
}

fn rotate_in(path: &Path) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if meta.len() < MAX_BYTES {
        return;
    }
    let bak = path.with_extension("log.1");
    let _ = std::fs::remove_file(&bak);
    let _ = std::fs::rename(path, &bak);
}

fn write_to(dir: &Path, level: &str, msg: &str) {
    let path = dir.join("app.log");
    let _ = std::fs::create_dir_all(dir);
    rotate_in(&path);
    let flat: String = msg
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{ts}] [{level}] {flat}");
    }
}

fn write(level: &str, msg: &str) {
    if let Some(dir) = dir() {
        write_to(&dir, level, msg);
    }
}

pub fn info(msg: impl AsRef<str>) {
    write("INFO", msg.as_ref());
}

pub fn warn(msg: impl AsRef<str>) {
    write("WARN", msg.as_ref());
}

pub fn error(msg: impl AsRef<str>) {
    write("ERROR", msg.as_ref());
}

pub fn open_logs() {
    let Some(path) = path() else { return };
    if !path.exists() {
        info("WinThemeAuto log started");
    }
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &path.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wta-log-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn appends_lines() {
        let dir = tmp("append");
        write_to(&dir, "INFO", "hello");
        write_to(&dir, "ERROR", "boom");
        let text = std::fs::read_to_string(dir.join("app.log")).unwrap();
        assert!(text.contains("[INFO] hello"), "got {text}");
        assert!(text.contains("[ERROR] boom"), "got {text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotates_and_keeps_one_backup() {
        let dir = tmp("rotate");
        let _ = std::fs::create_dir_all(&dir);
        let big = "x".repeat((MAX_BYTES + 100) as usize);
        std::fs::write(dir.join("app.log"), &big).unwrap();
        write_to(&dir, "INFO", "after");
        let bak = std::fs::read_to_string(dir.join("app.log.1")).unwrap();
        assert_eq!(bak.len(), big.len());
        let cur = std::fs::read_to_string(dir.join("app.log")).unwrap();
        assert!(cur.contains("after"), "got {cur}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
