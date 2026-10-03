use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct Tray {
    icon: TrayIcon,
    open_item: MenuItem,
    toggle_item: MenuItem,
    quit_item: MenuItem,
    last_dark: std::cell::Cell<Option<bool>>,
    pub open_id: MenuId,
    pub toggle_id: MenuId,
    pub quit_id: MenuId,
}

impl Tray {
    pub fn update(&self, is_dark: bool, text: &TrayText<'_>) {
        let _ = self.icon.set_tooltip(Some(text.tooltip));
        self.open_item.set_text(text.open);
        self.toggle_item.set_text(if is_dark {
            text.switch_to_light
        } else {
            text.switch_to_dark
        });
        self.quit_item.set_text(text.quit);
        if self.last_dark.get() != Some(is_dark) {
            self.last_dark.set(Some(is_dark));
            let _ = self
                .icon
                .set_icon(Some(if is_dark { dark_icon() } else { light_icon() }));
        }
    }
}

pub struct TrayText<'a> {
    pub tooltip: &'a str,
    pub open: &'a str,
    pub switch_to_light: &'a str,
    pub switch_to_dark: &'a str,
    pub quit: &'a str,
}

pub fn create(menu: &TrayText<'_>, initially_dark: bool) -> Result<Tray> {
    let open = MenuItem::new(menu.open, true, None);
    let toggle = MenuItem::new(
        if initially_dark {
            menu.switch_to_light
        } else {
            menu.switch_to_dark
        },
        true,
        None,
    );
    let quit = MenuItem::new(menu.quit, true, None);

    let menu_builder = Menu::new();
    menu_builder.append(&open)?;
    menu_builder.append(&toggle)?;
    menu_builder.append(&PredefinedMenuItem::separator())?;
    menu_builder.append(&quit)?;

    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu_builder))
        .with_tooltip(menu.tooltip)
        .with_icon(if initially_dark {
            dark_icon()
        } else {
            light_icon()
        })
        .build()?;

    Ok(Tray {
        icon,
        last_dark: std::cell::Cell::new(Some(initially_dark)),
        open_id: open.id().clone(),
        toggle_id: toggle.id().clone(),
        quit_id: quit.id().clone(),
        open_item: open,
        toggle_item: toggle,
        quit_item: quit,
    })
}

fn light_icon() -> Icon {
    const RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-light-64.rgba"));
    Icon::from_rgba(RGBA.to_vec(), 64, 64).expect("valid icon")
}

fn dark_icon() -> Icon {
    const RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-dark-64.rgba"));
    Icon::from_rgba(RGBA.to_vec(), 64, 64).expect("valid icon")
}

#[cfg(test)]
mod tests {
    #[test]
    fn tray_icons_rasterize() {
        const LIGHT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-light-64.rgba"));
        const DARK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-dark-64.rgba"));
        for rgba in [LIGHT, DARK] {
            assert_eq!(rgba.len(), 64 * 64 * 4);
            let opaque = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] > 16).count();
            assert!(opaque > 64 * 64 / 2, "icon is mostly transparent");
        }
        assert_ne!(LIGHT, DARK);
    }
}
