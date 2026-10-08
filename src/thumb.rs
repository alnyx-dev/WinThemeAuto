//! Small shell thumbnails for UI previews.
//!
//! `slint::Image::load_from_path` decodes the whole file: a single 4K photo
//! costs ~33 MB of RGBA RAM (8K ~133 MB), times four previews (light/dark
//! wallpaper + light/dark lock screen), times the renderer's texture copy —
//! easily 300+ MB on machines with large wallpapers. Thumbnails requested
//! from the shell stay under a fixed size no matter how big the source is.

use std::path::Path;

const PREVIEW_W: i32 = 320;
const PREVIEW_H: i32 = 180;
/// Sanity cap in case a provider ignores the requested size.
const MAX_PIXELS: u64 = 640 * 360;

/// Load a small preview of an image file via the shell thumbnail provider.
/// Returns `None` when the provider has nothing (then the caller falls back).
pub fn load_thumbnail(path: &Path) -> Option<slint::Image> {
    let (w, h, px) = thumbnail_rgba(path)?;
    if w == 0 || h == 0 || px.len() != w as usize * h as usize * 4 {
        return None;
    }
    // Providers may ignore the requested size (seen on CI runners), so fit
    // the pixels ourselves — the output is bounded no matter what.
    let (w, h, px) = downscale_to_fit(w, h, &px, PREVIEW_W as u32, PREVIEW_H as u32);
    let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&px, w, h);
    Some(slint::Image::from_rgba8(buf))
}

/// Box-average downscale so the result fits within `max_w` x `max_h`.
/// Returns the input unchanged when it already fits.
fn downscale_to_fit(w: u32, h: u32, px: &[u8], max_w: u32, max_h: u32) -> (u32, u32, Vec<u8>) {
    if w <= max_w && h <= max_h {
        return (w, h, px.to_vec());
    }
    let scale = (max_w as f64 / w as f64)
        .min(max_h as f64 / h as f64)
        .min(1.0);
    let nw = ((w as f64 * scale).round() as u32).max(1).min(max_w);
    let nh = ((h as f64 * scale).round() as u32).max(1).min(max_h);
    let mut out = vec![0u8; nw as usize * nh as usize * 4];
    for y in 0..nh {
        let sy0 = (y as f64 / scale) as u32;
        let sy1 = (((y + 1) as f64 / scale).ceil() as u32).clamp(sy0 + 1, h);
        for x in 0..nw {
            let sx0 = (x as f64 / scale) as u32;
            let sx1 = (((x + 1) as f64 / scale).ceil() as u32).clamp(sx0 + 1, w);
            let mut acc = [0u64; 4];
            let mut n = 0u64;
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let o = (sy as usize * w as usize + sx as usize) * 4;
                    for c in 0..4 {
                        acc[c] += px[o + c] as u64;
                    }
                    n += 1;
                }
            }
            let o = (y as usize * nw as usize + x as usize) * 4;
            for c in 0..4 {
                out[o + c] = (acc[c] / n.max(1)) as u8;
            }
        }
    }
    (nw, nh, out)
}
fn thumbnail_rgba(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_RESIZETOFIT,
    };

    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }
    let hpath = HSTRING::from(path.to_string_lossy().as_ref());

    unsafe {
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&hpath, None).ok()?;
        let hbmp = factory
            .GetImage(
                windows::Win32::Foundation::SIZE {
                    cx: PREVIEW_W,
                    cy: PREVIEW_H,
                },
                SIIGBF_RESIZETOFIT,
            )
            .ok()?;
        if hbmp.0.is_null() {
            return None;
        }
        let out = bitmap_to_rgba(hbmp.0);
        windows_sys::Win32::Graphics::Gdi::DeleteObject(hbmp.0);
        out
    }
}

unsafe fn bitmap_to_rgba(hbmp: *mut core::ffi::c_void) -> Option<(u32, u32, Vec<u8>)> {
    use windows_sys::Win32::Graphics::Gdi::*;

    let mut bmp: BITMAP = std::mem::zeroed();
    if GetObjectW(
        hbmp,
        std::mem::size_of::<BITMAP>() as i32,
        &mut bmp as *mut _ as *mut _,
    ) == 0
    {
        return None;
    }
    let (w, h) = (bmp.bmWidth, bmp.bmHeight.abs());
    if w <= 0 || h <= 0 {
        return None;
    }
    if (w as u64) * (h as u64) > MAX_PIXELS {
        return None;
    }

    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = w;
    info.bmiHeader.biHeight = -h; // top-down DIB: rows in display order
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;

    let mut px = vec![0u8; w as usize * h as usize * 4];
    let hdc = GetDC(std::ptr::null_mut());
    let lines = GetDIBits(
        hdc,
        hbmp,
        0,
        h as u32,
        px.as_mut_ptr() as *mut _,
        &mut info,
        DIB_RGB_COLORS,
    );
    if !hdc.is_null() {
        ReleaseDC(std::ptr::null_mut(), hdc);
    }
    if lines == 0 {
        return None;
    }
    // DIB is BGRA; Slint wants RGBA.
    for p in px.as_chunks_mut::<4>().0 {
        p.swap(0, 2);
    }
    Some((w as u32, h as u32, px))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bmp24(w: u32, h: u32) -> Vec<u8> {
        let stride = ((w * 3 + 3) / 4) * 4;
        let pixels = stride * h;
        let mut out = Vec::with_capacity((54 + pixels) as usize);
        out.extend_from_slice(b"BM");
        out.extend(&(54 + pixels).to_le_bytes());
        out.extend(&[0u8; 4]);
        out.extend(&54u32.to_le_bytes());
        out.extend(&40u32.to_le_bytes());
        out.extend(&w.to_le_bytes());
        out.extend(&h.to_le_bytes());
        out.extend(&1u16.to_le_bytes());
        out.extend(&24u16.to_le_bytes());
        out.extend(&[0u8; 24]);
        for _ in 0..h {
            for x in 0..w {
                out.push((x * 17) as u8);
                out.push(0x80);
                out.push(0x40);
            }
            for _ in w * 3..stride {
                out.push(0);
            }
        }
        out
    }

    #[test]
    fn shell_thumbnail_stays_small() {
        let dir = std::env::temp_dir().join("wta-thumb-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.bmp");
        std::fs::write(&path, bmp24(64, 40)).unwrap();
        let img = load_thumbnail(&path);
        let _ = std::fs::remove_file(&path);
        let img = img.expect("shell should thumbnail a plain bmp");
        let size = img.size();
        assert!(size.width > 0 && size.height > 0);
        assert!(size.width <= 320 && size.height <= 180);
    }

    #[test]
    fn downscale_fits_and_averages() {
        // 4x2 gradient in R, constant G/B/A.
        let mut px = Vec::new();
        for v in [0u8, 64, 128, 255, 10, 20, 30, 40] {
            px.extend_from_slice(&[v, 100, 150, 255]);
        }
        // 4x2 at scale 0.5 -> 2x1; each dest pixel averages a 2x2 block.
        let (w, h, out) = downscale_to_fit(4, 2, &px, 2, 2);
        assert_eq!((w, h), (2, 1));
        assert_eq!(out[0], (0u8 + 64 + 10 + 20) / 4); // 23
        assert_eq!(&out[1..4], &[100, 150, 255]);
        assert_eq!(out[4], ((128u32 + 255 + 30 + 40) / 4) as u8); // 113
        assert_eq!(&out[5..8], &[100, 150, 255]);
        // Already-fitting input is returned unchanged.
        let (w, h, out) = downscale_to_fit(2, 2, &px[..16], 320, 180);
        assert_eq!((w, h), (2, 2));
        assert_eq!(out.as_slice(), &px[..16]);
    }

    #[test]
    fn missing_file_returns_none() {
        let missing = std::env::temp_dir().join("wta-thumb-test-no-such-file.bmp");
        assert!(load_thumbnail(&missing).is_none());
    }
}
