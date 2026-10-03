use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct Tray {
    icon: TrayIcon,
    toggle_item: MenuItem,
    pub open_id: MenuId,
    pub toggle_id: MenuId,
    pub quit_id: MenuId,
}

impl Tray {
    /// Refresh tooltip + toggle label for the current theme. Called every
    /// few seconds from the scheduler tick; the caller caches so the OS
    /// only hears about actual changes.
    pub fn update(&self, is_dark: bool, text: &TrayText<'_>) {
        let _ = self.icon.set_tooltip(Some(text.tooltip));
        self.toggle_item.set_text(if is_dark {
            text.switch_to_light
        } else {
            text.switch_to_dark
        });
    }
}

/// Translated strings for [`Tray::update`].
pub struct TrayText<'a> {
    pub tooltip: &'a str,
    pub switch_to_light: &'a str,
    pub switch_to_dark: &'a str,
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
        icon,
        open_id: open.id().clone(),
        toggle_id: toggle.id().clone(),
        quit_id: quit.id().clone(),
        toggle_item: toggle,
    })
}

fn make_icon() -> Icon {
    // Pre-rendered by build.rs into `icon-64.rgba`, so the exe carries
    // 16 KiB of pixels instead of a whole SVG stack. Premultiplied RGBA
    // is exactly what the Windows tray (HICON) expects.
    const RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon-64.rgba"));
    Icon::from_rgba(RGBA.to_vec(), 64, 64).expect("valid icon")
}

#[cfg(test)]
mod tests {
    #[test]
    fn icon_rasterizes() {
        const RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon-64.rgba"));
        assert_eq!(RGBA.len(), 64 * 64 * 4);
        let opaque = RGBA.as_chunks::<4>().0.iter().filter(|p| p[3] > 16).count();
        assert!(opaque > 64 * 64 / 2, "icon is mostly transparent");
    }
}
