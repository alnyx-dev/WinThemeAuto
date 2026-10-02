//! Installed Windows themes (`.theme` files).
//!
//! Used to offer full light/dark theme selection: applying a theme means
//! switching the light/dark flags (see [`crate::theme`]) plus swapping the
//! wallpaper the theme points at — silently, without launching anything.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    /// Display name (from `[Theme] DisplayName`, or file stem as fallback).
    pub name: String,
    pub path: PathBuf,
    pub wallpaper: Option<PathBuf>,
    /// `Some("light")` / `Some("dark")` from `[VisualStyles] SystemMode`.
    pub system_mode: Option<String>,
    /// Same, from `[VisualStyles] AppMode`.
    pub app_mode: Option<String>,
}

fn system_themes_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()),
    )
    .join("Resources")
    .join("Themes")
}

fn user_themes_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("Microsoft").join("Windows").join("Themes"))
}

/// All installed themes, system first, then user ones.
pub fn enumerate() -> Vec<ThemeEntry> {
    let mut out = Vec::new();
    for dir in [Some(system_themes_dir()), user_themes_dir()]
        .into_iter()
        .flatten()
    {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("theme"))
            {
                continue;
            }
            out.push(parse_entry(&path));
        }
    }
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

/// Display name with the theme's own mode as a hint, e.g. `"Windows (Dark)"`.
pub fn display_name(t: &ThemeEntry) -> String {
    let mode = t.system_mode.as_deref().or(t.app_mode.as_deref());
    match mode {
        Some("light") => format!("{} (Light)", t.name),
        Some("dark") => format!("{} (Dark)", t.name),
        _ => t.name.clone(),
    }
}

fn parse_entry(path: &Path) -> ThemeEntry {
    let text = read_text(path);
    let sections = parse_ini(&text);
    let get = |section: &str, key: &str| {
        sections
            .iter()
            .find(|(s, _)| s.eq_ignore_ascii_case(section))
            .and_then(|(_, kv)| kv.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)))
            .map(|(_, v)| v.clone())
    };

    let name = get("Theme", "DisplayName")
        .filter(|n| !n.is_empty() && !n.starts_with('@'))
        .or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().replace('_', " "))
        })
        .unwrap_or_else(|| "Unnamed theme".to_string());

    let wallpaper = get("Control Panel\\Desktop", "Wallpaper")
        .map(expand_env)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);

    let norm_mode = |v: Option<String>| {
        v.map(|m| m.to_lowercase())
            .filter(|m| m == "light" || m == "dark")
    };

    ThemeEntry {
        name,
        path: path.to_path_buf(),
        wallpaper,
        system_mode: norm_mode(get("VisualStyles", "SystemMode")),
        app_mode: norm_mode(get("VisualStyles", "AppMode")),
    }
}

/// Read a `.theme` file regardless of encoding (UTF-8, UTF-16 LE, legacy).
fn read_text(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        return s;
    }
    let words = |data: &[u8]| {
        data.as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect::<Vec<_>>()
    };
    // With BOM.
    if bytes.starts_with(&[0xFF, 0xFE]) {
        if let Ok(s) = String::from_utf16(&words(&bytes[2..])) {
            return s;
        }
    }
    // Without BOM: ASCII-range UTF-16LE has a zero high byte in most words.
    if bytes.len() % 2 == 0 && bytes.len() >= 4 {
        let w = words(&bytes);
        if String::from_utf16(&w).is_ok() && w.iter().filter(|x| **x < 0x100).count() * 4 > w.len() * 3 {
            return String::from_utf16(&w).unwrap_or_default();
        }
    }
    // Last resort: byte-to-char mapping keeps ASCII readable.
    bytes.iter().map(|&b| b as char).collect()
}

/// Minimal case-preserving INI parser: `[(section, [(key, value)])]`.
fn parse_ini(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    // Strip BOM if present.
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    for line in text.lines() {
        let line = line.trim().trim_end_matches('\0');
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                sections.push((line[1..end].trim().to_string(), Vec::new()));
            }
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if let Some((_, kv)) = sections.last_mut() {
                kv.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }
    sections
}

/// Expand `%VAR%` placeholders (theme files use `%SystemRoot%`, etc.).
fn expand_env(s: String) -> String {
    let mut out = s;
    // Iterate: values may themselves contain variables.
    for _ in 0..5 {
        let Some(start) = out.find('%') else {
            break;
        };
        let rest = &out[start + 1..];
        let Some(end) = rest.find('%') else {
            break;
        };
        let var = &rest[..end];
        let value = std::env::var(var).unwrap_or_default();
        out.replace_range(start..start + 1 + end + 1, &value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; comment\r\n\
        [Theme]\r\n\
        DisplayName=My Light Theme\r\n\
        [Control Panel\\Desktop]\r\n\
        Wallpaper=%SystemRoot%\\web\\wallpaper\\img0.jpg\r\n\
        [VisualStyles]\r\n\
        SystemMode=Light\r\n\
        AppMode=Dark\r\n";

    #[test]
    fn parses_sample() {
        let sections = parse_ini(SAMPLE);
        let get = |s: &str, k: &str| {
            sections
                .iter()
                .find(|(sec, _)| sec.eq_ignore_ascii_case(s))
                .and_then(|(_, kv)| kv.iter().find(|(key, _)| key.eq_ignore_ascii_case(k)))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(get("Theme", "DisplayName").as_deref(), Some("My Light Theme"));
        assert_eq!(
            get("Control Panel\\Desktop", "Wallpaper").as_deref(),
            Some("%SystemRoot%\\web\\wallpaper\\img0.jpg")
        );
        assert_eq!(get("VisualStyles", "SystemMode").as_deref(), Some("Light"));
    }

    #[test]
    fn utf16_file_reads() {
        let dir = std::env::temp_dir().join("winthemeauto-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("sample.theme");
        let wide: Vec<u16> = SAMPLE.encode_utf16().collect();
        let mut bytes = vec![0xFF, 0xFE];
        for w in wide {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();
        let text = read_text(&path);
        assert!(text.contains("DisplayName=My Light Theme"), "got {text:?}");
        let entry = parse_entry(&path);
        assert_eq!(entry.name, "My Light Theme");
        assert_eq!(entry.system_mode.as_deref(), Some("light"));
        assert_eq!(entry.app_mode.as_deref(), Some("dark"));
        assert!(entry.wallpaper.is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn expands_env() {
        let expanded = expand_env(r"%SystemRoot%\web\wallpaper\img0.jpg".to_string());
        assert!(!expanded.contains('%'), "got {expanded}");
        assert!(expanded.to_lowercase().contains("web"), "got {expanded}");
    }

    #[test]
    fn utf8_file_reads_and_falls_back() {
        let dir = std::env::temp_dir().join("winthemeauto-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("plain.theme");
        // No DisplayName -> falls back to file stem (underscores become spaces).
        std::fs::write(&path, "[Control Panel\\Desktop]\nWallpaper=C:\\a.jpg\n").unwrap();
        let entry = parse_entry(&path);
        assert_eq!(entry.name, "plain");
        assert_eq!(entry.wallpaper.map(|p| p.to_string_lossy().into_owned()).as_deref(), Some(r"C:\a.jpg"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn enumerate_finds_system_themes() {
        // Any real Windows install ships themes in C:\Windows\Resources\Themes.
        let all = enumerate();
        assert!(!all.is_empty(), "no .theme files found");
        assert!(all.iter().all(|t| !t.name.is_empty() && t.path.exists()));
    }
}
