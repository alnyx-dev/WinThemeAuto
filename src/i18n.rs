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
    pub lock_check: &'static str,
    pub light_lock_ph: &'static str,
    pub dark_lock_ph: &'static str,
    pub lock_hint: &'static str,
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
    pub logs: &'static str,
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
            light_pm: "Light ±min",
            dark_pm: "Dark ±min",
            apply_to: "Apply to:",
            apps: "Apps",
            system: "System",
            light_label: "Light:",
            dark_label: "Dark:",
            light_wp_ph: "Light wallpaper (optional)…",
            dark_wp_ph: "Dark wallpaper (optional)…",
            wp_hint: "A custom path wins over the theme wallpaper. Empty = theme only.",
            lock_check: "Sync lock screen with theme",
            light_lock_ph: "Light lock screen (optional)…",
            dark_lock_ph: "Dark lock screen (optional)…",
            lock_hint: "Empty = same as desktop wallpaper (or theme).",
            accent_check: "Sync accent color with theme",
            accent_hint:
                "Click a swatch to fill the field, then Apply — taskbar needs “Show accent color” on.",
            rescan: "Rescan themes",
            autostart: "Start with Windows",
            check_updates: "Check for updates",
            checking: "Checking…",
            apply: "Apply",
            to_light: "To light",
            to_dark: "To dark",
            light_now: "Light now",
            dark_now: "Dark now",
            system_default: "(No theme file — flags only)",
            language: "Language:",
            close_title: "Minimize to tray?",
            close_body: "Closing the window keeps WinThemeAuto running in the tray (auto-switch stays on). To quit completely, use the tray menu → Exit.",
            close_minimize: "Minimize",
            close_quit: "Quit",
            close_never: "Don't ask again",
            tray_open: "Open",
            tray_exit: "Exit",
            releases: "Releases",
            onboard: "Turn on Auto switch, pick a mode, press Apply",
            cancel: "Cancel",
            logs: "Logs",
        },
        Lang::Ru => UiStrings {
            tab_auto: "Автопереключение",
            tab_appearance: "Оформление",
            tab_settings: "Настройки",
            auto_title: "Автопереключение",
            mode_time: "По времени",
            mode_sun: "Восход / закат",
            light_from: "Светлая с",
            dark_from: "Тёмная с",
            lat_ph: "Шир. 55,75",
            lon_ph: "Долг. 37,61",
            detect: "Найти",
            light_pm: "Свет ±мин",
            dark_pm: "Тёмн. ±мин",
            apply_to: "Применить к:",
            apps: "Приложения",
            system: "Система",
            light_label: "Светлая:",
            dark_label: "Тёмная:",
            light_wp_ph: "Светлые обои (необязательно)…",
            dark_wp_ph: "Тёмные обои (необязательно)…",
            wp_hint: "Свой путь важнее обоев темы. Пусто = только тема.",
            lock_check: "Экран блокировки в цвет темы",
            light_lock_ph: "Светлый локскрин (необязательно)…",
            dark_lock_ph: "Тёмный локскрин (необязательно)…",
            lock_hint: "Пусто = как рабочий стол (или тема).",
            accent_check: "Акцент в цвет темы",
            accent_hint:
                "Клик заполняет поле — нажмите Применить. Для панели включите «Цвет в Пуске и на панели».",
            rescan: "Обновить темы",
            autostart: "Запускать с Windows",
            check_updates: "Проверить обновления",
            checking: "Проверка…",
            apply: "Применить",
            to_light: "К светлой",
            to_dark: "К тёмной",
            light_now: "Сейчас светлая",
            dark_now: "Сейчас тёмная",
            system_default: "(Без файла темы — только флаги)",
            language: "Язык:",
            close_title: "Свернуть в трей?",
            close_body: "Закрытие окна оставляет WinThemeAuto в трее (автопереключение работает). Чтобы выйти полностью: меню трея → Выход.",
            close_minimize: "Свернуть",
            close_quit: "Выйти",
            close_never: "Больше не спрашивать",
            tray_open: "Открыть",
            tray_exit: "Выход",
            releases: "Релизы",
            onboard: "Включите авто, выберите режим, нажмите Применить",
            cancel: "Отмена",
            logs: "Логи",
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
            "bad_coords" => "Enter coordinates: latitude -90..90, longitude -180..180 — or click Detect, then Apply",
            "accent_hex" => "{p} accent must be hex RGB, e.g. 0078D4",
            "save_fail" => "Failed to save settings: {e}",
            "autostart_fail" => "Autostart: {e}",
            "nothing_toggle" => "Nothing to toggle: enable Apps or System",
            "switch_fail" => "Failed to switch theme: {e}",
            "error" => "Error: {e}",
            "wallpaper" => "Wallpaper: {e}",
            "lockscreen" => "Lock screen: {e}",
            "accent" => "Accent: {e}",
            "locating" => "Locating…",
            "geo_ok" => "Found via IP — {e}. Click Apply.",
            "geo_fail" => "Location failed: {e}. Enter coordinates manually.",
            "checking_updates" => "Checking for updates…",
            "latest" => "You have the latest version (v{e}).",
            "found" => "Found v{e} — downloading…",
            "installing" => "Installing v{e} — the app will restart…",
            "check_fail" => "Update check failed: {e}",
            "dl_fail" => "Download failed: {e}",
            "install_fail" => "Install failed: {e}",
            "theme_gone" => "Selected theme no longer installed — reset to no-theme (flags only)",
            "rescanning" => "Scanning installed themes…",
            "need_target" => "Auto-switch needs Apps or System — enable at least one",
            "cancelled" => "Update cancelled.",
            "pick_light" => "Choose light-mode wallpaper",
            "pick_dark" => "Choose dark-mode wallpaper",
            "pick_light_lock" => "Choose light-mode lock screen image",
            "pick_dark_lock" => "Choose dark-mode lock screen image",
            "held" => "Manual theme — auto resumes at the next switch ({e})",
            "held_plain" => "Manual theme — auto paused until Apply",
            "need_coords" => "Enter coordinates for sunrise/sunset",
            "need_coords_short" => "Enter coordinates…",
            _ => "{e}",
        }),
        Lang::Ru => ru(match key {
            "bad_time" => "Время в формате ЧЧ:ММ, напр. 07:30",
            "same_time" => "Время светлой и тёмной должны различаться",
            "bad_offset" => "Сдвиг — целое число от -180 до 180",
            "bad_coords" => "Введите координаты: широта -90..90, долгота -180..180 — или нажмите Найти, затем Применить",
            "accent_hex" => "Акцент {p} — hex RGB, напр. 0078D4",
            "save_fail" => "Не удалось сохранить: {e}",
            "autostart_fail" => "Автозапуск: {e}",
            "nothing_toggle" => "Нечего переключать: включите Приложения или Систему",
            "switch_fail" => "Не удалось переключить тему: {e}",
            "error" => "Ошибка: {e}",
            "wallpaper" => "Обои: {e}",
            "lockscreen" => "Блокировка: {e}",
            "accent" => "Акцент: {e}",
            "locating" => "Определение…",
            "geo_ok" => "Найдено по IP — {e}. Нажмите Применить.",
            "geo_fail" => "Не удалось определить: {e}. Введите координаты вручную.",
            "checking_updates" => "Проверка обновлений…",
            "latest" => "У вас последняя версия (v{e}).",
            "found" => "Найдена v{e} — загрузка…",
            "installing" => "Установка v{e} — перезапуск…",
            "check_fail" => "Ошибка проверки: {e}",
            "dl_fail" => "Ошибка загрузки: {e}",
            "install_fail" => "Ошибка установки: {e}",
            "theme_gone" => "Выбранная тема удалена — сброшено на «без темы» (только флаги)",
            "rescanning" => "Сканирование установленных тем…",
            "need_target" => "Для автопереключения нужны Приложения или Система — включите хоть одно",
            "cancelled" => "Обновление отменено.",
            "pick_light" => "Выберите обои светлого режима",
            "pick_dark" => "Выберите обои тёмного режима",
            "pick_light_lock" => "Выберите картинку блокировки светлого режима",
            "pick_dark_lock" => "Выберите картинку блокировки тёмного режима",
            "held" => "Ручная тема — авто возобновится на следующем переключении ({e})",
            "held_plain" => "Ручная тема — авто на паузе до «Применить»",
            "need_coords" => "Введите координаты для восхода/заката",
            "need_coords_short" => "Введите координаты…",
            _ => "{e}",
        }),
    }
}

pub fn mode_word(lang: Lang, light: bool) -> &'static str {
    match (lang, light) {
        (Lang::En, true) => "Light",
        (Lang::En, false) => "Dark",
        (Lang::Ru, true) => "Светлый",
        (Lang::Ru, false) => "Тёмный",
    }
}

pub fn wp_missing(lang: Lang, light: bool, path: &str) -> String {
    match lang {
        Lang::En => format!("{} wallpaper not found: {path}", mode_word(lang, light)),
        Lang::Ru => format!(
            "{} обои не найдены: {path}",
            if light {
                "Светлые"
            } else {
                "Тёмные"
            }
        ),
    }
}

pub fn lock_missing(lang: Lang, light: bool, path: &str) -> String {
    match lang {
        Lang::En => format!("{} lock screen not found: {path}", mode_word(lang, light)),
        Lang::Ru => format!(
            "{} локскрин не найден: {path}",
            if light {
                "Светлый"
            } else {
                "Тёмный"
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
        Some(t) if t > 0 => format!(" — {}%", (done * 100 / t).min(100)),
        _ => String::new(),
    };
    match lang {
        Lang::En => format!("Downloading v{version} — {amounts}{pct}…"),
        Lang::Ru => format!("Загрузка v{version} — {amounts}{pct}…"),
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
                format!("{mins} мин")
            } else if mins % 60 == 0 {
                format!("{} ч", mins / 60)
            } else {
                format!("{} ч {} мин", mins / 60, mins % 60)
            }
        }
    }
}

pub fn next_theme_word(lang: Lang, dark: bool) -> &'static str {
    match (lang, dark) {
        (Lang::En, true) => "dark",
        (Lang::En, false) => "light",
        (Lang::Ru, true) => "тёмная",
        (Lang::Ru, false) => "светлая",
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
            en.logs,
            en.lock_check,
            en.light_lock_ph,
            en.dark_lock_ph,
            en.lock_hint,
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
            ru.logs,
            ru.lock_check,
            ru.light_lock_ph,
            ru.dark_lock_ph,
            ru.lock_hint,
        ] {
            assert!(!s.is_empty());
        }
        assert_ne!(en.apply, ru.apply);
        assert_ne!(en.tab_settings, ru.tab_settings);
        assert_ne!(en.lock_check, ru.lock_check);
    }

    #[test]
    fn messages_fill_slots() {
        assert!(msg(Lang::En, "bad_time", "").contains("HH:MM"));
        assert!(msg(Lang::Ru, "bad_time", "").contains("ЧЧ:ММ"));
        assert!(msg(Lang::En, "save_fail", "x").contains('x'));
        assert!(msg(Lang::Ru, "accent", "boom").contains("boom"));
        assert_eq!(mode_word(Lang::Ru, true), "Светлый");
        assert_eq!(duration_hm(Lang::Ru, 75), "1 ч 15 мин");
        assert_eq!(duration_hm(Lang::En, 75), "1 h 15 min");
    }
}
