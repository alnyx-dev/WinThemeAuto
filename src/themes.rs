use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub name: String,
    pub path: PathBuf,
    pub wallpaper: Option<PathBuf>,
    pub system_mode: Option<String>,
    pub app_mode: Option<String>,
    pub user: bool,
}

fn system_themes_dir() -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()))
        .join("Resources")
        .join("Themes")
}

fn user_themes_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("Microsoft").join("Windows").join("Themes"))
}

pub fn enumerate() -> Vec<ThemeEntry> {
    let mut sys = Vec::new();
    let mut usr = Vec::new();
    collect_dir(&system_themes_dir(), false, &mut sys);
    if let Some(dir) = user_themes_dir() {
        collect_dir(&dir, true, &mut usr);
    }
    sys.sort_by_key(|t: &ThemeEntry| t.name.to_lowercase());
    usr.sort_by_key(|t: &ThemeEntry| t.name.to_lowercase());
    sys.extend(usr);
    sys
}

fn collect_dir(dir: &Path, user: bool, out: &mut Vec<ThemeEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
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
        out.push(parse_entry(&path, user));
    }
}

pub fn display_name(t: &ThemeEntry) -> String {
    let mode = t.system_mode.as_deref().or(t.app_mode.as_deref());
    let mut tags: Vec<&str> = Vec::new();
    match mode {
        Some("light") => tags.push("Light"),
        Some("dark") => tags.push("Dark"),
        _ => {}
    }
    if t.user {
        tags.push("user");
    }
    if tags.is_empty() {
        t.name.clone()
    } else {
        format!("{} ({})", t.name, tags.join(", "))
    }
}

fn parse_entry(path: &Path, user: bool) -> ThemeEntry {
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
        user,
    }
}

fn read_text(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let words = |data: &[u8]| {
        data.as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect::<Vec<_>>()
    };
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        if !s.contains('\0') {
            return s;
        }
        let w = words(&bytes);
        if let Ok(le) = String::from_utf16(&w) {
            if looks_like_ini(&le) {
                return le;
            }
        }
        return s;
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        if let Ok(s) = String::from_utf16(&words(&bytes[2..])) {
            return s;
        }
    }
    if bytes.len() % 2 == 0 && bytes.len() >= 4 {
        let w = words(&bytes);
        if let Ok(s) = String::from_utf16(&w) {
            if looks_like_ini(&s) {
                return s;
            }
        }
    }
    if bytes.len() % 2 == 0 && bytes.len() >= 4 {
        let w = words(&bytes);
        if String::from_utf16(&w).is_ok()
            && w.iter().filter(|x| **x < 0x100).count() * 4 > w.len() * 3
        {
            return String::from_utf16(&w).unwrap_or_default();
        }
    }
    bytes.iter().map(|&b| b as char).collect()
}

fn looks_like_ini(s: &str) -> bool {
    s.contains('[') && s.contains('=')
}

fn parse_ini(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
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

fn expand_env(s: String) -> String {
    let mut out = s;
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
        assert_eq!(
            get("Theme", "DisplayName").as_deref(),
            Some("My Light Theme")
        );
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
        let entry = parse_entry(&path, false);
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
        std::fs::write(&path, "[Control Panel\\Desktop]\nWallpaper=C:\\a.jpg\n").unwrap();
        let entry = parse_entry(&path, false);
        assert_eq!(entry.name, "plain");
        assert_eq!(
            entry
                .wallpaper
                .map(|p| p.to_string_lossy().into_owned())
                .as_deref(),
            Some(r"C:\a.jpg")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn enumerate_finds_system_themes() {
        let all = enumerate();
        assert!(!all.is_empty(), "no .theme files found");
        assert!(all.iter().all(|t| !t.name.is_empty() && t.path.exists()));
    }

    #[test]
    fn user_tag_disambiguates_dupes() {
        let mk = |user: bool| ThemeEntry {
            name: "Windows".to_string(),
            path: PathBuf::from(if user {
                r"C:\U\a.theme"
            } else {
                r"C:\W\a.theme"
            }),
            wallpaper: None,
            system_mode: Some("dark".to_string()),
            app_mode: None,
            user,
        };
        assert_eq!(display_name(&mk(false)), "Windows (Dark)");
        assert_eq!(display_name(&mk(true)), "Windows (Dark, user)");
    }

    #[test]
    fn utf16_no_bom_cyrillic_reads() {
        let dir = std::env::temp_dir().join("winthemeauto-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cyr.theme");
        let sample =
            "[Theme]\r\nDisplayName=РњРѕСЏ РўС‘РјРЅР°СЏ РўРµРјР°\r\n[VisualStyles]\r\nSystemMode=Dark\r\n";
        let mut bytes = Vec::new();
        for w in sample.encode_utf16() {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();
        let entry = parse_entry(&path, true);
        assert_eq!(entry.name, "РњРѕСЏ РўС‘РјРЅР°СЏ РўРµРјР°");
        assert_eq!(entry.system_mode.as_deref(), Some("dark"));
        assert!(entry.user);
        let _ = std::fs::remove_file(&path);
    }
}
