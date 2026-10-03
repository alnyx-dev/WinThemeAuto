#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code.trim().to_lowercase().as_str() {
            "ru" | "rus" | "russian" | "ru-ru" => Lang::Ru,
            _ => Lang::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
        }
    }

    pub fn index(self) -> i32 {
        match self {
            Lang::En => 0,
            Lang::Ru => 1,
        }
    }

    pub fn from_index(i: i32) -> Self {
        match i {
            1 => Lang::Ru,
            _ => Lang::En,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiStrings {
    pub tab_auto: &'static str,
    pub tab_appearance: &'static str,
    pub tab_settings: &'static str,
    pub auto_title: &'static str,
    pub mode_time: &'static str,
    pub mode_sun: &'static str,
    pub light_from: &'static str,
    pub dark_from: &'static str,
    pub lat_ph: &'static str,
    pub lon_ph: &'static str,
    pub detect: &'static str,
    pub light_pm: &'static str,
    pub dark_pm: &'static str,
    pub apply_to: &'static str,
    pub apps: &'static str,
    pub system: &'static str,
    pub light_label: &'static str,
    pub dark_label: &'static str,
    pub light_wp_ph: &'static str,
    pub dark_wp_ph: &'static str,
    pub wp_hint: &'static str,
    pub accent_check: &'static str,
    pub accent_hint: &'static str,
    pub rescan: &'static str,
    pub autostart: &'static str,
    pub check_updates: &'static str,
    pub checking: &'static str,
    pub apply: &'static str,
    pub to_light: &'static str,
    pub to_dark: &'static str,
    pub light_now: &'static str,
    pub dark_now: &'static str,
    pub system_default: &'static str,
    pub language: &'static str,
    pub close_title: &'static str,
    pub close_body: &'static str,
    pub close_minimize: &'static str,
    pub close_quit: &'static str,
    pub close_never: &'static str,
    pub tray_open: &'static str,
    pub tray_exit: &'static str,
    pub releases: &'static str,
    pub onboard: &'static str,
    pub cancel: &'static str,
}

pub fn ui(lang: Lang) -> UiStrings {
    match lang {
        Lang::En => UiStrings {
            tab_auto: "Auto switch",
            tab_appearance: "Appearance",
            tab_settings: "Settings",
            auto_title: "Auto switch",
            mode_time: "By time",
            mode_sun: "Sunrise / sunset",
            light_from: "Light from",
            dark_from: "Dark from",
            lat_ph: "Lat 55.75",
            lon_ph: "Lon 37.61",
            detect: "Detect",
            light_pm: "Light В±min",
            dark_pm: "Dark В±min",
            apply_to: "Apply to:",
            apps: "Apps",
            system: "System",
            light_label: "Light:",
            dark_label: "Dark:",
            light_wp_ph: "Light wallpaper (optional)вЂ¦",
            dark_wp_ph: "Dark wallpaper (optional)вЂ¦",
            wp_hint: "A custom path wins over the theme wallpaper. Empty = theme only.",
            accent_check: "Sync accent color with theme",
            accent_hint:
                "Click a swatch to fill the field, then Apply вЂ” taskbar needs вЂњShow accent colorвЂќ on.",
            rescan: "Rescan themes",
            autostart: "Start with Windows",
            check_updates: "Check for updates",
            checking: "CheckingвЂ¦",
            apply: "Apply",
            to_light: "To light",
            to_dark: "To dark",
            light_now: "Light now",
            dark_now: "Dark now",
            system_default: "(No theme file вЂ” flags only)",
            language: "Language:",
            close_title: "Minimize to tray?",
            close_body: "Closing the window keeps WinThemeAuto running in the tray (auto-switch stays on). To quit completely, use the tray menu в†’ Exit.",
            close_minimize: "Minimize",
            close_quit: "Quit",
            close_never: "Don't ask again",
            tray_open: "Open",
            tray_exit: "Exit",
            releases: "Releases",
            onboard: "Turn on Auto switch, pick a mode, press Apply",
            cancel: "Cancel",
        },
        Lang::Ru => UiStrings {
            tab_auto: "РђРІС‚РѕРїРµСЂРµРєР»СЋС‡РµРЅРёРµ",
            tab_appearance: "РћС„РѕСЂРјР»РµРЅРёРµ",
            tab_settings: "РќР°СЃС‚СЂРѕР№РєРё",
            auto_title: "РђРІС‚РѕРїРµСЂРµРєР»СЋС‡РµРЅРёРµ",
            mode_time: "РџРѕ РІСЂРµРјРµРЅРё",
            mode_sun: "Р’РѕСЃС…РѕРґ / Р·Р°РєР°С‚",
            light_from: "РЎРІРµС‚Р»Р°СЏ СЃ",
            dark_from: "РўС‘РјРЅР°СЏ СЃ",
            lat_ph: "РЁРёСЂ. 55,75",
            lon_ph: "Р”РѕР»Рі. 37,61",
            detect: "РќР°Р№С‚Рё",
            light_pm: "РЎРІРµС‚ В±РјРёРЅ",
            dark_pm: "РўС‘РјРЅ. В±РјРёРЅ",
            apply_to: "РџСЂРёРјРµРЅРёС‚СЊ Рє:",
            apps: "РџСЂРёР»РѕР¶РµРЅРёСЏ",
            system: "РЎРёСЃС‚РµРјР°",
            light_label: "РЎРІРµС‚Р»Р°СЏ:",
            dark_label: "РўС‘РјРЅР°СЏ:",
            light_wp_ph: "РЎРІРµС‚Р»С‹Рµ РѕР±РѕРё (РЅРµРѕР±СЏР·Р°С‚РµР»СЊРЅРѕ)вЂ¦",
            dark_wp_ph: "РўС‘РјРЅС‹Рµ РѕР±РѕРё (РЅРµРѕР±СЏР·Р°С‚РµР»СЊРЅРѕ)вЂ¦",
            wp_hint: "РЎРІРѕР№ РїСѓС‚СЊ РІР°Р¶РЅРµРµ РѕР±РѕРµРІ С‚РµРјС‹. РџСѓСЃС‚Рѕ = С‚РѕР»СЊРєРѕ С‚РµРјР°.",
            accent_check: "РђРєС†РµРЅС‚ РІ С†РІРµС‚ С‚РµРјС‹",
            accent_hint:
                "РљР»РёРє Р·Р°РїРѕР»РЅСЏРµС‚ РїРѕР»Рµ вЂ” РЅР°Р¶РјРёС‚Рµ РџСЂРёРјРµРЅРёС‚СЊ. Р”Р»СЏ РїР°РЅРµР»Рё РІРєР»СЋС‡РёС‚Рµ В«Р¦РІРµС‚ РІ РџСѓСЃРєРµ Рё РЅР° РїР°РЅРµР»РёВ».",
            rescan: "РћР±РЅРѕРІРёС‚СЊ С‚РµРјС‹",
            autostart: "Р—Р°РїСѓСЃРєР°С‚СЊ СЃ Windows",
            check_updates: "РџСЂРѕРІРµСЂРёС‚СЊ РѕР±РЅРѕРІР»РµРЅРёСЏ",
            checking: "РџСЂРѕРІРµСЂРєР°вЂ¦",
            apply: "РџСЂРёРјРµРЅРёС‚СЊ",
            to_light: "Рљ СЃРІРµС‚Р»РѕР№",
            to_dark: "Рљ С‚С‘РјРЅРѕР№",
            light_now: "РЎРµР№С‡Р°СЃ СЃРІРµС‚Р»Р°СЏ",
            dark_now: "РЎРµР№С‡Р°СЃ С‚С‘РјРЅР°СЏ",
            system_default: "(Р‘РµР· С„Р°Р№Р»Р° С‚РµРјС‹ вЂ” С‚РѕР»СЊРєРѕ С„Р»Р°РіРё)",
            language: "РЇР·С‹Рє:",
            close_title: "РЎРІРµСЂРЅСѓС‚СЊ РІ С‚СЂРµР№?",
            close_body: "Р—Р°РєСЂС‹С‚РёРµ РѕРєРЅР° РѕСЃС‚Р°РІР»СЏРµС‚ WinThemeAuto РІ С‚СЂРµРµ (Р°РІС‚РѕРїРµСЂРµРєР»СЋС‡РµРЅРёРµ СЂР°Р±РѕС‚Р°РµС‚). Р§С‚РѕР±С‹ РІС‹Р№С‚Рё РїРѕР»РЅРѕСЃС‚СЊСЋ: РјРµРЅСЋ С‚СЂРµСЏ в†’ Р’С‹С…РѕРґ.",
            close_minimize: "РЎРІРµСЂРЅСѓС‚СЊ",
            close_quit: "Р’С‹Р№С‚Рё",
            close_never: "Р‘РѕР»СЊС€Рµ РЅРµ СЃРїСЂР°С€РёРІР°С‚СЊ",
            tray_open: "РћС‚РєСЂС‹С‚СЊ",
            tray_exit: "Р’С‹С…РѕРґ",
            releases: "Р РµР»РёР·С‹",
            onboard: "Р’РєР»СЋС‡РёС‚Рµ Р°РІС‚Рѕ, РІС‹Р±РµСЂРёС‚Рµ СЂРµР¶РёРј, РЅР°Р¶РјРёС‚Рµ РџСЂРёРјРµРЅРёС‚СЊ",
            cancel: "РћС‚РјРµРЅР°",
        },
    }
}

pub fn msg(lang: Lang, key: &str, arg: &str) -> String {
    let en = |s: &str| s.replace("{e}", arg).replace("{p}", arg);
    let ru = |s: &str| s.replace("{e}", arg).replace("{p}", arg);
    match lang {
        Lang::En => en(match key {
            "bad_time" => "Time must be HH:MM, e.g. 07:30",
            "same_time" => "Light and dark times must differ",
            "bad_offset" => "Offset must be an integer from -180 to 180",
            "bad_coords" => "Enter coordinates: latitude -90..90, longitude -180..180 вЂ” or click Detect, then Apply",
            "accent_hex" => "{p} accent must be hex RGB, e.g. 0078D4",
            "save_fail" => "Failed to save settings: {e}",
            "autostart_fail" => "Autostart: {e}",
            "nothing_toggle" => "Nothing to toggle: enable Apps or System",
            "switch_fail" => "Failed to switch theme: {e}",
            "error" => "Error: {e}",
            "wallpaper" => "Wallpaper: {e}",
            "accent" => "Accent: {e}",
            "locating" => "LocatingвЂ¦",
            "geo_ok" => "Found via IP вЂ” {e}. Click Apply.",
            "geo_fail" => "Location failed: {e}. Enter coordinates manually.",
            "checking_updates" => "Checking for updatesвЂ¦",
            "latest" => "You have the latest version (v{e}).",
            "found" => "Found v{e} вЂ” downloadingвЂ¦",
            "installing" => "Installing v{e} вЂ” the app will restartвЂ¦",
            "check_fail" => "Update check failed: {e}",
            "dl_fail" => "Download failed: {e}",
            "install_fail" => "Install failed: {e}",
            "theme_gone" => "Selected theme no longer installed вЂ” reset to no-theme (flags only)",
            "need_target" => "Auto-switch needs Apps or System вЂ” enable at least one",
            "cancelled" => "Update cancelled.",
            "pick_light" => "Choose light-mode wallpaper",
            "pick_dark" => "Choose dark-mode wallpaper",
            "held" => "Manual theme вЂ” auto resumes at the next switch ({e})",
            "held_plain" => "Manual theme вЂ” auto paused until Apply",
            "need_coords" => "Enter coordinates for sunrise/sunset",
            "need_coords_short" => "Enter coordinatesвЂ¦",
            _ => "{e}",
        }),
        Lang::Ru => ru(match key {
            "bad_time" => "Р’СЂРµРјСЏ РІ С„РѕСЂРјР°С‚Рµ Р§Р§:РњРњ, РЅР°РїСЂ. 07:30",
            "same_time" => "Р’СЂРµРјСЏ СЃРІРµС‚Р»РѕР№ Рё С‚С‘РјРЅРѕР№ РґРѕР»Р¶РЅС‹ СЂР°Р·Р»РёС‡Р°С‚СЊСЃСЏ",
            "bad_offset" => "РЎРґРІРёРі вЂ” С†РµР»РѕРµ С‡РёСЃР»Рѕ РѕС‚ -180 РґРѕ 180",
            "bad_coords" => "Р’РІРµРґРёС‚Рµ РєРѕРѕСЂРґРёРЅР°С‚С‹: С€РёСЂРѕС‚Р° -90..90, РґРѕР»РіРѕС‚Р° -180..180 вЂ” РёР»Рё РЅР°Р¶РјРёС‚Рµ РќР°Р№С‚Рё, Р·Р°С‚РµРј РџСЂРёРјРµРЅРёС‚СЊ",
            "accent_hex" => "РђРєС†РµРЅС‚ {p} вЂ” hex RGB, РЅР°РїСЂ. 0078D4",
            "save_fail" => "РќРµ СѓРґР°Р»РѕСЃСЊ СЃРѕС…СЂР°РЅРёС‚СЊ: {e}",
            "autostart_fail" => "РђРІС‚РѕР·Р°РїСѓСЃРє: {e}",
            "nothing_toggle" => "РќРµС‡РµРіРѕ РїРµСЂРµРєР»СЋС‡Р°С‚СЊ: РІРєР»СЋС‡РёС‚Рµ РџСЂРёР»РѕР¶РµРЅРёСЏ РёР»Рё РЎРёСЃС‚РµРјСѓ",
            "switch_fail" => "РќРµ СѓРґР°Р»РѕСЃСЊ РїРµСЂРµРєР»СЋС‡РёС‚СЊ С‚РµРјСѓ: {e}",
            "error" => "РћС€РёР±РєР°: {e}",
            "wallpaper" => "РћР±РѕРё: {e}",
            "accent" => "РђРєС†РµРЅС‚: {e}",
            "locating" => "РћРїСЂРµРґРµР»РµРЅРёРµвЂ¦",
            "geo_ok" => "РќР°Р№РґРµРЅРѕ РїРѕ IP вЂ” {e}. РќР°Р¶РјРёС‚Рµ РџСЂРёРјРµРЅРёС‚СЊ.",
            "geo_fail" => "РќРµ СѓРґР°Р»РѕСЃСЊ РѕРїСЂРµРґРµР»РёС‚СЊ: {e}. Р’РІРµРґРёС‚Рµ РєРѕРѕСЂРґРёРЅР°С‚С‹ РІСЂСѓС‡РЅСѓСЋ.",
            "checking_updates" => "РџСЂРѕРІРµСЂРєР° РѕР±РЅРѕРІР»РµРЅРёР№вЂ¦",
            "latest" => "РЈ РІР°СЃ РїРѕСЃР»РµРґРЅСЏСЏ РІРµСЂСЃРёСЏ (v{e}).",
            "found" => "РќР°Р№РґРµРЅР° v{e} вЂ” Р·Р°РіСЂСѓР·РєР°вЂ¦",
            "installing" => "РЈСЃС‚Р°РЅРѕРІРєР° v{e} вЂ” РїРµСЂРµР·Р°РїСѓСЃРєвЂ¦",
            "check_fail" => "РћС€РёР±РєР° РїСЂРѕРІРµСЂРєРё: {e}",
            "dl_fail" => "РћС€РёР±РєР° Р·Р°РіСЂСѓР·РєРё: {e}",
            "install_fail" => "РћС€РёР±РєР° СѓСЃС‚Р°РЅРѕРІРєРё: {e}",
            "theme_gone" => "Р’С‹Р±СЂР°РЅРЅР°СЏ С‚РµРјР° СѓРґР°Р»РµРЅР° вЂ” СЃР±СЂРѕС€РµРЅРѕ РЅР° В«Р±РµР· С‚РµРјС‹В» (С‚РѕР»СЊРєРѕ С„Р»Р°РіРё)",
            "need_target" => "Р”Р»СЏ Р°РІС‚РѕРїРµСЂРµРєР»СЋС‡РµРЅРёСЏ РЅСѓР¶РЅС‹ РџСЂРёР»РѕР¶РµРЅРёСЏ РёР»Рё РЎРёСЃС‚РµРјР° вЂ” РІРєР»СЋС‡РёС‚Рµ С…РѕС‚СЊ РѕРґРЅРѕ",
            "cancelled" => "РћР±РЅРѕРІР»РµРЅРёРµ РѕС‚РјРµРЅРµРЅРѕ.",
            "pick_light" => "Р’С‹Р±РµСЂРёС‚Рµ РѕР±РѕРё СЃРІРµС‚Р»РѕРіРѕ СЂРµР¶РёРјР°",
            "pick_dark" => "Р’С‹Р±РµСЂРёС‚Рµ РѕР±РѕРё С‚С‘РјРЅРѕРіРѕ СЂРµР¶РёРјР°",
            "held" => "Р СѓС‡РЅР°СЏ С‚РµРјР° вЂ” Р°РІС‚Рѕ РІРѕР·РѕР±РЅРѕРІРёС‚СЃСЏ РЅР° СЃР»РµРґСѓСЋС‰РµРј РїРµСЂРµРєР»СЋС‡РµРЅРёРё ({e})",
            "held_plain" => "Р СѓС‡РЅР°СЏ С‚РµРјР° вЂ” Р°РІС‚Рѕ РЅР° РїР°СѓР·Рµ РґРѕ В«РџСЂРёРјРµРЅРёС‚СЊВ»",
            "need_coords" => "Р’РІРµРґРёС‚Рµ РєРѕРѕСЂРґРёРЅР°С‚С‹ РґР»СЏ РІРѕСЃС…РѕРґР°/Р·Р°РєР°С‚Р°",
            "need_coords_short" => "Р’РІРµРґРёС‚Рµ РєРѕРѕСЂРґРёРЅР°С‚С‹вЂ¦",
            _ => "{e}",
        }),
    }
}

pub fn mode_word(lang: Lang, light: bool) -> &'static str {
    match (lang, light) {
        (Lang::En, true) => "Light",
        (Lang::En, false) => "Dark",
        (Lang::Ru, true) => "РЎРІРµС‚Р»С‹Р№",
        (Lang::Ru, false) => "РўС‘РјРЅС‹Р№",
    }
}

pub fn wp_missing(lang: Lang, light: bool, path: &str) -> String {
    match lang {
        Lang::En => format!("{} wallpaper not found: {path}", mode_word(lang, light)),
        Lang::Ru => format!(
            "{} РѕР±РѕРё РЅРµ РЅР°Р№РґРµРЅС‹: {path}",
            if light {
                "РЎРІРµС‚Р»С‹Рµ"
            } else {
                "РўС‘РјРЅС‹Рµ"
            }
        ),
    }
}

pub fn dl_progress(lang: Lang, version: &str, done: u64, total: Option<u64>) -> String {
    let amounts = match total {
        Some(t) => format!(
            "{} / {}",
            crate::update::human_size(done),
            crate::update::human_size(t)
        ),
        None => crate::update::human_size(done),
    };
    let pct = match total {
        Some(t) if t > 0 => format!(" вЂ” {}%", (done * 100 / t).min(100)),
        _ => String::new(),
    };
    match lang {
        Lang::En => format!("Downloading v{version} вЂ” {amounts}{pct}вЂ¦"),
        Lang::Ru => format!("Р—Р°РіСЂСѓР·РєР° v{version} вЂ” {amounts}{pct}вЂ¦"),
    }
}
pub fn duration_hm(lang: Lang, mins: i64) -> String {
    match lang {
        Lang::En => {
            if mins < 60 {
                format!("{mins} min")
            } else if mins % 60 == 0 {
                format!("{} h", mins / 60)
            } else {
                format!("{} h {} min", mins / 60, mins % 60)
            }
        }
        Lang::Ru => {
            if mins < 60 {
                format!("{mins} РјРёРЅ")
            } else if mins % 60 == 0 {
                format!("{} С‡", mins / 60)
            } else {
                format!("{} С‡ {} РјРёРЅ", mins / 60, mins % 60)
            }
        }
    }
}

pub fn next_theme_word(lang: Lang, dark: bool) -> &'static str {
    match (lang, dark) {
        (Lang::En, true) => "dark",
        (Lang::En, false) => "light",
        (Lang::Ru, true) => "С‚С‘РјРЅР°СЏ",
        (Lang::Ru, false) => "СЃРІРµС‚Р»Р°СЏ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_roundtrip() {
        assert_eq!(Lang::from_code("ru"), Lang::Ru);
        assert_eq!(Lang::from_code("RU-RU"), Lang::Ru);
        assert_eq!(Lang::from_code("en"), Lang::En);
        assert_eq!(Lang::from_code(""), Lang::En);
        assert_eq!(Lang::from_code("de"), Lang::En);
        assert_eq!(Lang::Ru.code(), "ru");
        assert_eq!(Lang::from_index(1), Lang::Ru);
        assert_eq!(Lang::from_index(0), Lang::En);
        assert_eq!(Lang::from_index(99), Lang::En);
    }

    #[test]
    fn ui_strings_complete_and_distinct() {
        let en = ui(Lang::En);
        let ru = ui(Lang::Ru);
        for s in [
            en.tab_auto,
            en.tab_appearance,
            en.apply,
            en.to_dark,
            en.system_default,
            en.tray_open,
            en.tray_exit,
            en.releases,
            en.onboard,
            en.cancel,
            ru.tab_auto,
            ru.tab_appearance,
            ru.apply,
            ru.to_dark,
            ru.system_default,
            ru.tray_open,
            ru.tray_exit,
            ru.releases,
            ru.onboard,
            ru.cancel,
        ] {
            assert!(!s.is_empty());
        }
        assert_ne!(en.apply, ru.apply);
        assert_ne!(en.tab_settings, ru.tab_settings);
    }

    #[test]
    fn messages_fill_slots() {
        assert!(msg(Lang::En, "bad_time", "").contains("HH:MM"));
        assert!(msg(Lang::Ru, "bad_time", "").contains("Р§Р§:РњРњ"));
        assert!(msg(Lang::En, "save_fail", "x").contains('x'));
        assert!(msg(Lang::Ru, "accent", "boom").contains("boom"));
        assert_eq!(mode_word(Lang::Ru, true), "РЎРІРµС‚Р»С‹Р№");
        assert_eq!(duration_hm(Lang::Ru, 75), "1 С‡ 15 РјРёРЅ");
        assert_eq!(duration_hm(Lang::En, 75), "1 h 15 min");
    }
}
