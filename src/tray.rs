use crate::i18n::{self, Lang};
use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

#[derive(Clone, Copy)]
pub enum PauseAction {
    Hour,
    NextSwitch,
    Resume,
}

pub struct Tray {
    icon: TrayIcon,
    open_item: MenuItem,
    toggle_item: MenuItem,
    quit_item: MenuItem,
    pause_hour: MenuItem,
    pause_next: MenuItem,
    resume: MenuItem,
    last_dark: std::cell::Cell<Option<bool>>,
    pub open_id: MenuId,
    pub toggle_id: MenuId,
    pub quit_id: MenuId,
}

impl Tray {
    pub fn pause_action(&self, id: &MenuId) -> Option<PauseAction> {
        if id == self.pause_hour.id() {
            Some(PauseAction::Hour)
        } else if id == self.pause_next.id() {
            Some(PauseAction::NextSwitch)
        } else if id == self.resume.id() {
            Some(PauseAction::Resume)
        } else {
            None
        }
    }

    pub fn update_pause(&self, lang: Lang, can_pause: bool, has_next: bool, paused: bool) {
        let text = i18n::ui(lang);
        self.pause_hour.set_text(text.pause_hour);
        self.pause_next.set_text(text.pause_next);
        self.resume.set_text(text.resume_schedule);
        self.pause_hour.set_enabled(can_pause);
        self.pause_next.set_enabled(can_pause && has_next);
        self.resume.set_enabled(paused);
    }

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
    let pause_hour = MenuItem::new("", false, None);
    let pause_next = MenuItem::new("", false, None);
    let resume = MenuItem::new("", false, None);
    let quit = MenuItem::new(menu.quit, true, None);

    let menu_builder = Menu::new();
    menu_builder.append(&open)?;
    menu_builder.append(&toggle)?;
    menu_builder.append(&PredefinedMenuItem::separator())?;
    menu_builder.append(&pause_hour)?;
    menu_builder.append(&pause_next)?;
    menu_builder.append(&resume)?;
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
        pause_hour,
        pause_next,
        resume,
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
