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

/// Apply a desktop wallpaper target: a single image file via
/// `SystemParametersInfoW`, or a folder of images via the system
/// slideshow (`IDesktopWallpaper::SetSlideshow` + interval/shuffle).
pub fn apply_desktop(path: &Path, interval_min: u32, shuffle: bool) -> Result<()> {
    if path.is_dir() {
        set_slideshow(path, interval_min, shuffle)
    } else {
        crate::theme::set_wallpaper(path)
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

    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }
    let hpath = windows::core::HSTRING::from(abs.to_string_lossy().as_ref());

    unsafe {
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        use windows::Win32::UI::Shell::{
            DesktopWallpaper, IDesktopWallpaper, IShellItem, IShellItemArray,
            SHCreateItemFromParsingName, SHCreateShellItemArrayFromShellItem,
            DESKTOP_SLIDESHOW_OPTIONS,
        };

        let item: IShellItem = SHCreateItemFromParsingName(&hpath, None)
            .with_context(|| format!("cannot open folder {}", abs.display()))?;
        let array: IShellItemArray =
            SHCreateShellItemArrayFromShellItem(&item).context("cannot build shell item array")?;
        let dw: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
            .context("cannot create DesktopWallpaper COM object")?;
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
}
