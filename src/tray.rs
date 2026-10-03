use anyhow::{Context, Result};
use tray_icon::{
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct Tray {
    _icon: TrayIcon,
    pub open_id: MenuId,
    pub toggle_id: MenuId,
    pub quit_id: MenuId,
}

pub fn create() -> Result<Tray> {
    let open = MenuItem::new("Open", true, None);
    let toggle = MenuItem::new("Toggle theme", true, None);
    let quit = MenuItem::new("Exit", true, None);

    let menu = Menu::new();
    menu.append(&open)?;
    menu.append(&toggle)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit)?;

    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("WinThemeAuto")
        .with_icon(make_icon())
        .build()?;

    Ok(Tray {
        _icon: icon,
        open_id: open.id().clone(),
        toggle_id: toggle.id().clone(),
        quit_id: quit.id().clone(),
    })
}

fn make_icon() -> Icon {
    // Rendered from icon.svg at startup; premultiplied RGBA is exactly
    // what the Windows tray (HICON) expects.
    let rgba = render_svg(64).expect("bundled icon.svg must rasterize");
    Icon::from_rgba(rgba, 64, 64).expect("valid icon")
}

/// Rasterize the bundled `icon.svg` to premultiplied RGBA at `size`px.
fn render_svg(size: u32) -> Result<Vec<u8>> {
    const SVG: &[u8] = include_bytes!("../icon.svg");
    let tree = resvg::usvg::Tree::from_data(SVG, &resvg::usvg::Options::default())
        .map_err(|e| anyhow::anyhow!("cannot parse icon.svg: {e}"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
        .context("icon pixmap too large")?;
    let scale = size as f32 / tree.size().width().max(1.0);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_rasterizes() {
        let rgba = render_svg(32).unwrap();
        assert_eq!(rgba.len(), 32 * 32 * 4);
        let opaque = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] > 16).count();
        assert!(opaque > 32 * 32 / 2, "icon is mostly transparent");
    }
}
