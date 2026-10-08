use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const SUPPORTED_EXTS: &[&str] = &["jpg", "jpeg", "png", "bmp"];
pub const MIN_INTERVAL_MIN: u32 = 1;
pub const MAX_INTERVAL_MIN: u32 = 24 * 60;
pub const DEFAULT_INTERVAL_MIN: u32 = 30;

pub fn clamp_interval(v: u32) -> u32 {
    v.clamp(MIN_INTERVAL_MIN, MAX_INTERVAL_MIN)
}

pub fn parse_interval_minutes(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() {
        return Some(DEFAULT_INTERVAL_MIN);
    }
    match t.parse::<u32>() {
        Ok(v) if (MIN_INTERVAL_MIN..=MAX_INTERVAL_MIN).contains(&v) => Some(v),
        _ => None,
    }
}

pub fn is_image_file(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            SUPPORTED_EXTS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

pub fn collect_images(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image_file(p))
        .collect();
    out.sort();
    out
}

pub fn first_image(dir: &Path) -> Option<PathBuf> {
    collect_images(dir).into_iter().next()
}

/// Absolute path without the `\\?\` extended prefix: shell parsing APIs
/// (`SHCreateItemFromParsingName`) reject extended paths with
/// E_INVALIDARG, while `canonicalize()` produces exactly those.
pub fn shell_path(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().into_owned();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{stripped}")
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        s
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppliedWallpaper {
    Single(PathBuf),
    Slideshow {
        dir: PathBuf,
        interval_min: u32,
        shuffle: bool,
    },
}

pub fn applied_key(path: &Path, interval_min: u32, shuffle: bool) -> AppliedWallpaper {
    if path.is_dir() {
        AppliedWallpaper::Slideshow {
            dir: path.to_path_buf(),
            interval_min: clamp_interval(interval_min),
            shuffle,
        }
    } else {
        AppliedWallpaper::Single(path.to_path_buf())
    }
}

fn desktop_wallpaper() -> Result<windows::Win32::UI::Shell::IDesktopWallpaper> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }
    unsafe {
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        use windows::Win32::UI::Shell::DesktopWallpaper;
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
            .context("cannot create DesktopWallpaper COM object")
    }
}

pub fn set_slideshow(dir: &Path, interval_min: u32, shuffle: bool) -> Result<()> {
    let images = collect_images(dir);
    if images.is_empty() {
        bail!(
            "slideshow folder has no images (jpg/png/bmp): {}",
            dir.display()
        );
    }
    let abs = dir
        .canonicalize()
        .with_context(|| format!("cannot resolve {}", dir.display()))?;
    let interval_min = clamp_interval(interval_min);
    let interval_ms = interval_min.saturating_mul(60_000);

    let hpath = windows::core::HSTRING::from(shell_path(dir).as_str());

    unsafe {
        use windows::Win32::UI::Shell::{
            IShellItem, IShellItemArray, SHCreateItemFromParsingName,
            SHCreateShellItemArrayFromShellItem, DESKTOP_SLIDESHOW_OPTIONS,
        };

        let item: IShellItem = SHCreateItemFromParsingName(&hpath, None)
            .with_context(|| format!("cannot open folder {}", abs.display()))?;
        let array: IShellItemArray =
            SHCreateShellItemArrayFromShellItem(&item).context("cannot build shell item array")?;
        let dw = desktop_wallpaper()?;
        dw.SetSlideshow(&array)
            .with_context(|| format!("SetSlideshow failed for {}", abs.display()))?;
        let options = DESKTOP_SLIDESHOW_OPTIONS(if shuffle { 1 } else { 0 });
        dw.SetSlideshowOptions(options, interval_ms)
            .context("SetSlideshowOptions failed")?;
    }
    crate::log::info(format!(
        "slideshow: applied {} ({} images, every {} min, shuffle {})",
        abs.display(),
        images.len(),
        interval_min,
        if shuffle { "on" } else { "off" }
    ));
    Ok(())
}

/// Move an already-active slideshow to the next image (wraps around).
///
/// Verified on-device: a fresh `SetSlideshow` starts at the folder's first
/// image and re-setting the same folder keeps the position, so advancing
/// is only meaningful when that folder's slideshow is already active —
/// see [`plan_apply`].
pub fn advance() -> Result<()> {
    unsafe {
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::DSD_FORWARD;
        let dw = desktop_wallpaper()?;
        dw.AdvanceSlideshow(PCWSTR::null(), DSD_FORWARD)
            .context("AdvanceSlideshow failed")?;
    }
    crate::log::info("slideshow: advanced to next image");
    Ok(())
}

/// Source folder of the currently active slideshow, if any. A
/// single-folder slideshow reports the folder itself; a multi-file one
/// reports the parent of its first image.
pub fn active_source_dir() -> Option<PathBuf> {
    unsafe {
        use windows::Win32::UI::Shell::SIGDN_FILESYSPATH;
        let dw = desktop_wallpaper().ok()?;
        let items = dw.GetSlideshow().ok()?;
        if items.GetCount().ok()? == 0 {
            return None;
        }
        let first = items.GetItemAt(0).ok()?;
        let name = first.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = PathBuf::from(String::from_utf16_lossy(name.as_wide()));
        if path.is_dir() {
            Some(path)
        } else {
            path.parent().map(|d| d.to_path_buf())
        }
    }
}

pub fn same_dir(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// What applying a wallpaper target should do. Files are plain sets;
/// folders configure a fresh slideshow on change, or advance the
/// already-active one on a theme switch (a fresh configure starts at
/// the first image, so advancing there would just skip it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyPlan {
    Skip,
    SetSingle,
    ConfigureShow,
    AdvanceShow,
}

pub fn plan_apply(is_dir: bool, key_changed: bool, switched: bool) -> ApplyPlan {
    match (is_dir, key_changed, switched) {
        (false, true, _) => ApplyPlan::SetSingle,
        (false, false, _) => ApplyPlan::Skip,
        (true, true, _) => ApplyPlan::ConfigureShow,
        (true, false, true) => ApplyPlan::AdvanceShow,
        (true, false, false) => ApplyPlan::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_clamps_and_parses() {
        assert_eq!(clamp_interval(0), MIN_INTERVAL_MIN);
        assert_eq!(clamp_interval(30), 30);
        assert_eq!(clamp_interval(99999), MAX_INTERVAL_MIN);
        assert_eq!(parse_interval_minutes(""), Some(DEFAULT_INTERVAL_MIN));
        assert_eq!(parse_interval_minutes("15"), Some(15));
        assert_eq!(parse_interval_minutes("0"), None);
        assert_eq!(parse_interval_minutes("99999"), None);
        assert_eq!(parse_interval_minutes("abc"), None);
    }

    #[test]
    fn image_filter() {
        assert!(is_image_file(Path::new("a.jpg")));
        assert!(is_image_file(Path::new("a.JPEG")));
        assert!(is_image_file(Path::new("a.png")));
        assert!(!is_image_file(Path::new("a.gif")));
        assert!(!is_image_file(Path::new("a")));
        assert!(!is_image_file(Path::new("dir.jpg/file")));
    }

    #[test]
    fn collect_sorts_and_filters() {
        let dir = std::env::temp_dir().join("winthemeauto-slideshow-test");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("b.jpg"), [1u8]);
        let _ = std::fs::write(dir.join("a.png"), [1u8]);
        let _ = std::fs::write(dir.join("note.txt"), [1u8]);
        let got = collect_images(&dir);
        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with("a.png"));
        assert!(got[1].ends_with("b.jpg"));
        assert_eq!(first_image(&dir), Some(dir.join("a.png")));
        let _ = std::fs::remove_file(dir.join("b.jpg"));
        let _ = std::fs::remove_file(dir.join("a.png"));
        let _ = std::fs::remove_file(dir.join("note.txt"));
        assert!(collect_images(&dir).is_empty());
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn applied_key_distinguishes_modes() {
        let file = PathBuf::from("C:\\a.jpg");
        let dir = std::env::temp_dir();
        let k1 = applied_key(&file, 30, false);
        assert_eq!(k1, AppliedWallpaper::Single(file.clone()));
        // temp_dir exists and is a dir on all platforms
        let k2 = applied_key(&dir, 30, true);
        assert_eq!(
            k2,
            AppliedWallpaper::Slideshow {
                dir: dir.clone(),
                interval_min: 30,
                shuffle: true
            }
        );
        // interval is clamped inside the key so tick dedup works
        let k3 = applied_key(&dir, 0, true);
        assert_eq!(
            k3,
            AppliedWallpaper::Slideshow {
                dir,
                interval_min: MIN_INTERVAL_MIN,
                shuffle: true
            }
        );
    }

    #[test]
    fn plan_table() {
        use ApplyPlan::*;
        // Files: plain set-or-skip, switches don't matter.
        assert_eq!(plan_apply(false, true, false), SetSingle);
        assert_eq!(plan_apply(false, true, true), SetSingle);
        assert_eq!(plan_apply(false, false, false), Skip);
        assert_eq!(plan_apply(false, false, true), Skip);
        // Fresh folder: configure (starts at first image, no advance).
        assert_eq!(plan_apply(true, true, false), ConfigureShow);
        assert_eq!(plan_apply(true, true, true), ConfigureShow);
        // Already-active folder: advance only on a real switch,
        // otherwise safety-net ticks would spin the slideshow.
        assert_eq!(plan_apply(true, false, true), AdvanceShow);
        assert_eq!(plan_apply(true, false, false), Skip);
    }

    #[test]
    fn shell_path_strips_extended_prefix() {
        // canonicalize() yields \\?\ paths on Windows; the shell chokes on
        // those (E_INVALIDARG, see friend log 2026-10-08).
        let dir = std::env::temp_dir();
        let s = shell_path(&dir);
        assert!(
            !s.starts_with(r"\\?\"),
            "extended prefix leaked into shell path: {s}"
        );
        assert!(s.len() >= 3, "unexpectedly short shell path: {s}");
        // Missing paths pass through untouched (no prefix to strip).
        let missing = Path::new(r"C:\wta-definitely-missing-dir-xyz\sub");
        assert_eq!(shell_path(missing), missing.to_string_lossy());
    }

    #[test]
    fn same_dir_compares() {
        let dir = std::env::temp_dir().join("wta-samedir-test");
        let _ = std::fs::create_dir_all(&dir);
        assert!(same_dir(&dir, &dir));
        assert!(same_dir(&dir, &dir.join(".")));
        assert!(!same_dir(&dir, &dir.join("elsewhere")));
        assert!(!same_dir(
            &dir,
            &std::env::temp_dir().join("wta-samedir-test-nope")
        ));
        let _ = std::fs::remove_dir(&dir);
    }
}
