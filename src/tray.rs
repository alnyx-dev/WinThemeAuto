use anyhow::Result;
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
    const S: u32 = 32;
    let mut rgba = Vec::with_capacity((S * S * 4) as usize);
    let c = (S as f32 - 1.0) / 2.0;
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let d = (dx * dx + dy * dy).sqrt();
            let px = if d > 15.0 {
                [0, 0, 0, 0]
            } else if d > 13.5 {
                [128, 128, 128, 255]
            } else if (x as f32) < c {
                [30, 30, 30, 255]
            } else {
                [245, 245, 245, 255]
            };
            rgba.extend_from_slice(&px);
        }
    }
    Icon::from_rgba(rgba, S, S).expect("valid icon")
}
