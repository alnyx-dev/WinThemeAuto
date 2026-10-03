use anyhow::Result;
use winreg::{enums::*, RegKey};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "WinThemeAuto";

pub fn is_enabled() -> bool {
    let Ok(value) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(NAME))
    else {
        return false;
    };
    match current_exe_lower() {
        Some(us) => extract_exe(&value) == us,
        None => false,
    }
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

fn current_exe_lower() -> Option<String> {
    std::env::current_exe().ok().map(|p| {
        let raw = p.to_string_lossy().to_lowercase();
        raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
    })
}

fn extract_exe(value: &str) -> String {
    let t = value.trim();
    let path = if let Some(rest) = t.strip_prefix('"') {
        rest.split('"').next().unwrap_or("").to_string()
    } else {
        t.split_whitespace().next().unwrap_or("").to_string()
    };
    let lower = path.to_lowercase();
    lower.strip_prefix(r"\\?\").unwrap_or(&lower).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_exe_forms() {
        assert_eq!(
            extract_exe(r#""C:\Apps\WinThemeAuto-x64.exe" --tray"#),
            r"c:\apps\winthemeauto-x64.exe"
        );
        assert_eq!(extract_exe(r"C:\Apps\wta.exe --tray"), r"c:\apps\wta.exe");
        assert_eq!(extract_exe(""), "");
        assert_ne!(
            extract_exe(r#""D:\Old\Place\app.exe" --tray"#),
            extract_exe(r#""C:\Apps\WinThemeAuto-x64.exe" --tray"#)
        );
    }
}
