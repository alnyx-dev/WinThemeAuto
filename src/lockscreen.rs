use anyhow::{bail, Context, Result};
use std::path::Path;

/// Sets the Windows lock screen image (per-user, no admin needed)
/// via WinRT `UserProfilePersonalizationSettings::TrySetLockScreenImageAsync`.
///
/// Returns Ok on success, Err with a human-readable message otherwise.
/// Missing file / unsupported device are reported as errors so callers
/// can surface them in the UI status line / logs.
pub fn set_image(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("lock screen image not found: {}", path.display());
    }
    // Canonicalize so StorageFile gets an absolute path, then strip the
    // `\\?\` extended-path prefix: WinRT rejects it with
    // "cannot open image as StorageFile".
    let abs = path
        .canonicalize()
        .with_context(|| format!("cannot resolve {}", path.display()))?;
    let mut s = abs.to_string_lossy().into_owned();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        s = format!(r"\\{stripped}");
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    // WinRT needs COM initialized. The UI thread is STA (winit calls
    // OleInitialize), so init as STA too — MTA here breaks winit with
    // RPC_E_CHANGED_MODE. Ignore "already initialized" errors.
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }
    let hpath = windows::core::HSTRING::from(s.as_str());

    let file = windows::Storage::StorageFile::GetFileFromPathAsync(&hpath)
        .with_context(|| format!("GetFileFromPathAsync rejected {s}"))?
        .get()
        .with_context(|| format!("cannot open {s} as StorageFile"))?;

    if !windows::System::UserProfile::UserProfilePersonalizationSettings::IsSupported()? {
        bail!("lock screen API not supported on this device");
    }
    let settings = windows::System::UserProfile::UserProfilePersonalizationSettings::Current()?;
    let ok = settings
        .TrySetLockScreenImageAsync(&file)
        .context("TrySetLockScreenImageAsync failed")?
        .get()
        .context("lock screen request failed")?;
    if !ok {
        // TrySet reports silent `false` with no reason. The older LockScreen
        // API throws with an HRESULT instead — and on some builds it
        // succeeds where TrySet returns false (e.g. ThemeD jpg ~1 MB).
        match windows::System::UserProfile::LockScreen::SetImageFileAsync(&file) {
            Ok(op) => match op.get() {
                Ok(()) => {
                    crate::log::info(format!("lock screen: set via LockScreen API {s}"));
                    return Ok(());
                }
                Err(e) => {
                    let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    bail!("Windows rejected {s} [{ext}, {bytes} B] ({e:#}) — try a local JPG/PNG <2 MB",);
                }
            },
            Err(e) => {
                let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                bail!(
                    "Windows rejected {s} [{ext}, {bytes} B] ({e:#}) — try a local JPG/PNG <2 MB",
                );
            }
        }
    }
    Ok(())
}
