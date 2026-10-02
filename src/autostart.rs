use anyhow::Result;
use winreg::{enums::*, RegKey};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "WinThemeAuto";

pub fn is_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(NAME))
        .is_ok()
}

pub fn set(enabled: bool) -> Result<()> {
    let run = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
    if enabled {
        let exe = std::env::current_exe()?;
        let raw = exe.display().to_string();
        let clean = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
        run.set_value(NAME, &format!("\"{clean}\" --tray"))?;
    } else {
        let _ = run.delete_value(NAME);
    }
    Ok(())
}
