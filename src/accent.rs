//! Accent-color sync: a custom Windows accent per light/dark mode.
//!
//! Primary path is the same one Settings uses: uxtheme's
//! `SetUserColorPreference` (ordinal 122), which writes `AccentColor`,
//! `AccentColorMenu`/`StartColorMenu` and a proper `AccentPalette`,
//! then commits the change. If that entry point is missing (older or
//! stripped builds), fall back to direct registry writes with a flat
//! palette, plus a settings broadcast.

use anyhow::{bail, Context, Result};
use winreg::{enums::*, RegKey, RegValue};

const EXPLORER_ACCENT: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent";
const DWM_KEY: &str = r"Software\Microsoft\Windows\DWM";

/// Mirrors uxtheme's `IMMERSIVE_COLOR_PREFERENCE`; `color2` is the accent
/// as a COLORREF (`0x00BBGGRR`).
#[repr(C)]
struct ImmersiveColorPreference {
    color_set: u32,
    color1: u32,
    color2: u32,
}

type GetColorPreference = unsafe extern "system" fn(*mut ImmersiveColorPreference, i32) -> i32;
type SetColorPreference = unsafe extern "system" fn(*const ImmersiveColorPreference, i32) -> i32;

/// Parse `#RRGGBB` / `RRGGBB` into a COLORREF (`0x00BBGGRR`).
pub fn parse_hex(s: &str) -> Option<u32> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    let (r, g, b) = ((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF);
    Some((b << 16) | (g << 8) | r)
}

/// COLORREF (`0x00BBGGRR`) → registry DWORD (`0xFFRRGGBB`, opaque).
pub fn to_dword(color: u32) -> u32 {
    let (r, g, b) = (color & 0xFF, (color >> 8) & 0xFF, (color >> 16) & 0xFF);
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Shaded 8-entry fallback palette: `[R,G,B,0x00]` × 8, dark → base → light.
///
/// Entry 4 is the base color; 0-3 mix toward black, 5-7 toward white.
/// Matches the shape Windows Settings writes via `SetUserColorPreference` —
/// a flat palette (all entries identical) renders as a dirty solid on
/// taskbar/start in the fallback path.
pub fn shaded_palette(color: u32) -> [u8; 32] {
    let (r, g, b) = (
        (color & 0xFF) as f32,
        ((color >> 8) & 0xFF) as f32,
        ((color >> 16) & 0xFF) as f32,
    );
    fn mix(base: f32, target: f32, t: f32) -> u8 {
        ((base * (1.0 - t) + target * t).round().clamp(0.0, 255.0)) as u8
    }
    let mut out = [0u8; 32];
    let (chunks, _) = out.as_chunks_mut::<4>();
    for (i, entry) in chunks.iter_mut().enumerate() {
        let (rr, gg, bb) = if i < 4 {
            let t = (4 - i) as f32 * 0.20;
            (mix(r, 0.0, t), mix(g, 0.0, t), mix(b, 0.0, t))
        } else if i == 4 {
            (r as u8, g as u8, b as u8)
        } else {
            let t = (i - 4) as f32 * 0.18;
            (mix(r, 255.0, t), mix(g, 255.0, t), mix(b, 255.0, t))
        };
        *entry = [rr, gg, bb, 0x00];
    }
    out
}

/// Apply an accent COLORREF for the current theme mode.
pub fn apply(color: u32) -> Result<()> {
    if system_api(color)? {
        return Ok(());
    }
    fallback_registry(color)
}

/// The Settings path: read the current preference block, swap the accent,
/// commit. Returns `Ok(false)` when the entry point is unavailable.
fn system_api(color: u32) -> Result<bool> {
    use windows_sys::Win32::Foundation::{FreeLibrary, S_OK};
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let name: Vec<u16> = "uxtheme.dll\0".encode_utf16().collect();
    // SAFETY: plain system DLL load by name.
    let lib = unsafe { LoadLibraryW(name.as_ptr()) };
    if lib.is_null() {
        return Ok(false);
    }
    // SAFETY: `lib` is valid until freed below; ordinals/names are correct.
    let result = unsafe {
        let get: Option<GetColorPreference> = std::mem::transmute(GetProcAddress(
            lib,
            c"GetUserColorPreference".as_ptr() as *const u8,
        ));
        let set: Option<SetColorPreference> =
            std::mem::transmute(GetProcAddress(lib, 122 as *const u8));
        let (Some(get), Some(set)) = (get, set) else {
            FreeLibrary(lib);
            return Ok(false);
        };

        let mut pref = ImmersiveColorPreference {
            color_set: 0,
            color1: 0,
            color2: 0,
        };
        if get(&mut pref, 0) != S_OK {
            FreeLibrary(lib);
            bail!("GetUserColorPreference failed");
        }
        pref.color2 = color & 0x00FF_FFFF;
        let ok = set(&pref, 1) == S_OK;
        FreeLibrary(lib);
        ok
    };
    if !result {
        bail!("SetUserColorPreference failed");
    }
    Ok(true)
}

/// Direct registry writes for when the system API is unavailable.
fn fallback_registry(color: u32) -> Result<()> {
    let dword = to_dword(color);
    let (dwm, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(DWM_KEY)
        .context("cannot open DWM key")?;
    dwm.set_value("AccentColor", &dword)?;
    let (accent, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(EXPLORER_ACCENT)
        .context("cannot open Explorer Accent key")?;
    accent.set_value("AccentColorMenu", &dword)?;
    accent.set_value("StartColorMenu", &dword)?;
    accent.set_raw_value(
        "AccentPalette",
        &RegValue {
            bytes: shaded_palette(color).to_vec(),
            vtype: REG_BINARY,
        },
    )?;
    crate::theme::notify_updated();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses() {
        assert_eq!(parse_hex("0078D4"), Some(0x00D47800));
        assert_eq!(parse_hex("#ff0000"), Some(0x000000FF));
        assert_eq!(parse_hex("  4CC2FF "), Some(0x00FFC24C));
        assert_eq!(parse_hex("FFF"), None);
        assert_eq!(parse_hex("GGGGGG"), None);
        assert_eq!(parse_hex(""), None);
    }

    #[test]
    fn dword_matches_known_registry_values() {
        // Red #FF0000 -> 0xFFFF0000; default blue #0078D4 -> 0xFF0078D4.
        assert_eq!(to_dword(0x000000FF), 0xFFFF0000);
        assert_eq!(to_dword(0x00D47800), 0xFF0078D4);
    }

    #[test]
    fn palette_shape() {
        let p = shaded_palette(0x00D47800);
        assert_eq!(p.len(), 32);
        let chunks = p.as_chunks::<4>().0;
        // Entry 4 is the base color 0xD47800 -> [0x00, 0x78, 0xD4, 0x00].
        assert_eq!(&chunks[4], &[0x00, 0x78, 0xD4, 0x00]);
        // Darker before, lighter after — not flat.
        let lum = |e: &[u8; 4]| e[0] as u32 + e[1] as u32 + e[2] as u32;
        assert!(lum(&chunks[0]) < lum(&chunks[4]), "first should be darker");
        assert!(lum(&chunks[7]) > lum(&chunks[4]), "last should be lighter");
        assert!(chunks.iter().any(|e| e != &[0x00, 0x78, 0xD4, 0x00]));
        // Alpha byte stays 0x00.
        assert!(chunks.iter().all(|e| e[3] == 0x00));
    }
}
