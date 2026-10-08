<div align="center">

<img src="icon.svg" width="120" alt="WinThemeAuto logo">

# WinThemeAuto

**Automatically switch Windows between light and dark themes — by schedule or by sunrise and sunset.**

_A tiny, fast, native Windows app written in Rust. No Electron, no background service, no bloat: just a lightweight tray utility that keeps your theme in sync with your day._

[![Windows](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D4?style=flat-square&logo=windows&logoColor=white)](https://github.com/alnyx-dev/WinThemeAuto/releases)
[![Release](https://img.shields.io/github/v/release/alnyx-dev/WinThemeAuto?style=flat-square)](https://github.com/alnyx-dev/WinThemeAuto/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/alnyx-dev/WinThemeAuto/release.yml?style=flat-square&label=build)](https://github.com/alnyx-dev/WinThemeAuto/actions)
[![Downloads](https://img.shields.io/github/downloads/alnyx-dev/WinThemeAuto/total?style=flat-square)](https://github.com/alnyx-dev/WinThemeAuto/releases)
[![Stars](https://img.shields.io/github/stars/alnyx-dev/WinThemeAuto?style=flat-square)](https://github.com/alnyx-dev/WinThemeAuto/stargazers)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint-blue?style=flat-square)](https://slint.dev/)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Website](https://img.shields.io/badge/website-online-brightgreen?style=flat-square)](https://alnyx-dev.github.io/WinThemeAuto)

</div>

## 📑 Contents

- [✨ Features](#-features)
- [📥 Download](#-download)
- [🚀 Quick start](#-quick-start)
- [🖥️ Usage](#️-usage)
- [⚙️ Settings reference](#️-settings-reference)
- [📁 Configuration file](#-configuration-file)
- [🧠 How it works](#-how-it-works)
- [🆚 Comparison](#-comparison)
- [💡 Tips & tricks](#-tips--tricks)
- [🧩 Project structure](#-project-structure)
- [🔒 Privacy](#-privacy)
- [❓ FAQ](#-faq)
- [🛠️ Troubleshooting](#️-troubleshooting)
- [📜 Changelog](#-changelog)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

## ✨ Features

- **Two auto-switch modes**
  - 🕒 **By time** — e.g. light at `07:00`, dark at `19:00` (overnight ranges supported, e.g. light `20:00` → dark `06:00`)
  - 🌅 **By sun** — sunrise/sunset calculated offline for your coordinates, with polar day/night handling
- **Sun offsets** — shift switching relative to sunrise/sunset (e.g. `-30` = 30 min before), ±180 min
- **Separate Apps / System targets** — switch `AppsUseLightTheme`, `SystemUsesLightTheme`, or both
- **One-click toggle** — instantly flip the current theme from the window or tray
- **Next-switch preview** — shows what switches next, at what time, and in how long
- **IP geolocation** — one-click coordinate detection via `ipwho.is` (with `ipapi.co` fallback), or enter coordinates manually
- **System tray** — close minimizes to tray; double-click tray icon to reopen; `Open` / `Toggle theme` / `Exit` menu
- **Start with Windows** — registry `Run` key autostart (starts hidden with `--tray`)
- **Instant apply** — broadcasts `WM_SETTINGCHANGE` / `WM_THEMECHANGED` and refreshes taskbars so apps pick up the change immediately
- **Robust config** — tolerant JSON parsing, validation/clamping, atomic saves, corrupt-file backup
- 🖼️ **Full Windows themes** — pick a light and a dark `.theme` from installed ones; the wallpaper follows the switch, silently. Or point each mode at **any image file** — custom wallpapers win over theme ones
- 🎞️ **Wallpaper slideshows** — point a mode at a **folder** of images instead of a file and Windows rotates them natively (`IDesktopWallpaper`), with shared interval (1–1440 min) and shuffle
- 🔒 **Lock screen sync** — per-mode lock screen images via WinRT (no admin), with desktop/theme fallback
- 🎨 **Accent color sync** — different Windows accent per light/dark mode, applied on every switch
- 🔄 **Self-updates** — one click checks GitHub Releases, downloads the newest exe (x64/x86 auto-matched) and installs it with a restart
- **Single instance** — a second launch just focuses the running window instead of duplicating tray icons

## 📥 Download

| File | Architecture | Who is it for? |
|------|--------------|----------------|
| `WinThemeAuto-x64.exe` | 64-bit (x86-64) | Most modern PCs — **download this one** |
| `WinThemeAuto-x86.exe` | 32-bit (x86) | Older 32-bit Windows, or 64-bit via WOW64 compatibility |

Get them from the [latest release](https://github.com/alnyx-dev/WinThemeAuto/releases). No installer, no admin rights — each exe is a single portable file.

**System requirements:** Windows 10 (1809+) or Windows 11. ~11 MB download, ~0% CPU when idle (wakes exactly at each switch, plus a 60 s safety net).

## 🚀 Quick start

1. Download `WinThemeAuto-x64.exe` from [Releases](https://github.com/alnyx-dev/WinThemeAuto/releases) and run it.
2. Enable **Auto switch**, pick a mode, click **Apply**.
3. (Optional) enable **Start with Windows** — the app will launch hidden in the tray on boot.

> 💡 Don't want automation? Just use the **Switch** button / tray `Toggle theme` as a manual light-dark flipper.

### Build from source

Requirements: Windows 10/11, [Rust](https://rustup.rs/) stable.

```powershell
git clone https://github.com/alnyx-dev/WinThemeAuto.git
cd WinThemeAuto
cargo build --release
.\target\release\win-theme-auto.exe
```

Run tests:

```powershell
cargo test
```

## 🖥️ Usage

1. Open the app — the header shows the current theme (`Light now` / `Dark now`), the next switch preview, and a quick **Switch** button. Below it, tabs switch between **Auto switch**, **Appearance** and **Settings** (the status line + **Apply** footer stays put).
2. On the **Auto switch** tab, toggle **Enabled** (closing the window minimizes to tray while enabled).
3. Choose a mode:
   - **By time**: set `Light from` and `Dark from` in `HH:MM` (24-hour) format.
   - **Sunrise and sunset**: enter latitude/longitude, or click **Detect**, then set offsets in minutes.
4. Still on the tab, pick **Apply to**: **Apps** and/or **System**.
5. On the **Appearance** tab, pick a full `.theme` for light and dark — or set a **custom wallpaper path** per mode (`…` picks an image file, `Folder` picks a slideshow folder; wins over theme wallpaper). Set the slideshow **interval** and **Shuffle** for folders. Enable **Sync lock screen with theme** and set per-mode lock images (empty = same as desktop). Enable **Sync accent color** and click a swatch (or type hex) per mode.
6. On the **Settings** tab: **Start with Windows**, version + **Check for updates**, plus a **Logs** button (`%APPDATA%\WinThemeAuto\app.log`).
7. Click **Apply** (always visible at the bottom). Settings save and apply immediately.

Closing the window hides it to the tray — use the tray menu or double-click the icon to bring it back.

### Tray menu

| Action         | What it does                          |
|----------------|---------------------------------------|
| `Open`         | Show the main window                  |
| `Switch to…`   | Flip light ↔ dark immediately (label follows the current theme) |
| `Exit`         | Quit the app (auto-switch stops)      |

Double-clicking the tray icon also opens the window.

### CLI

```
WinThemeAuto-x64.exe [--tray] [--toggle | --light | --dark | --status | --help]
```

| Flag       | Description                                        |
|------------|----------------------------------------------------|
| `--tray`   | Start hidden in the tray (used for autostart)      |
| `--toggle` | Flip light ↔ dark immediately, no window           |
| `--light`  | Switch to light immediately, no window             |
| `--dark`   | Switch to dark immediately, no window              |
| `--status` | Print current theme + next switch, no window       |
| `--help`   | Show usage                                         |

Action flags win over `--tray` and work while the GUI instance is running
(handy for AutoHotkey / StreamDeck / scheduled tasks). `--status` prints
e.g. `Dark now • Next: light at 07:00 (in 7 h)`; `--toggle/--light/--dark`
print the resulting `light` / `dark` and hold it until the next scheduled
switch, just like the in-app Switch.

### Updates

The **Settings** tab shows the current version (e.g. v0.2.6) and a **Check for updates** button — the result replaces the version text and glows green when you're up to date. If a newer release exists, the app downloads the matching exe (x64/x86 auto-detected), installs it over itself and restarts — settings are kept. See [How does self-update work?](#-faq) for details.

## ⚙️ Settings reference

| Setting | Description |
|---------|-------------|
| `Enabled` | Master switch for automatic switching. Checked exactly at each switch (+60 s safety net). |
| `By time / Sunrise and sunset` | Switching mode. |
| `Light from` / `Dark from` | Fixed-mode boundaries (`HH:MM`). May cross midnight. Must differ. |
| `Lat.` / `Lon.` | Coordinates for sun mode. Valid ranges: lat `-90…90`, lon `-180…180`. |
| `Detect` | Fills in coordinates from your public IP (online, one-shot). You still need to press **Apply**. |
| `Light/Dark offset (min)` | Shift relative to sunrise/sunset. Integer `-180…180`. Negative = earlier. |
| `Apps` / `System` | Which registry values to manage. At least one should be on for auto-switch info to appear. |
| `Light/Dark theme` | Full installed `.theme` for each mode — switches flags **plus** wallpaper. `System default` = flags only. |
| `Light/Dark wallpaper` | Custom image file (`jpg/png/bmp`) or slideshow **folder** per mode — wins over the theme wallpaper. `…` opens a file picker, `Folder` a folder picker. Empty = theme only. |
| `Every (min)` / `Shuffle` | Slideshow rotation for wallpaper folders: interval in minutes (`1–1440`), optional random order. Handled natively by Windows. |
| `Sync lock screen` | Applies the per-mode image to the Windows lock screen on every switch (WinRT, no admin). |
| `Light/Dark lock screen` | Custom lock image (`jpg/png`, local file <2 MB works best) per mode. Empty = same as desktop wallpaper (or theme). |
| `Sync accent color` | Applies the per-mode accent on every switch — click a swatch to fill + apply instantly, or type hex. |
| `Light/Dark accent` | Windows accent color per mode, e.g. `0078D4`. Needs **Show accent color on Start and taskbar** enabled for the taskbar to follow. |
| `Start with Windows` | Writes `HKCU\...\Run\WinThemeAuto = "<exe>" --tray`. Uncheck to remove. |
| `Language` | `EN` / `RU` interface switch in Settings — applies instantly, no restart. |
| `Check for updates` | Compares the app version with the latest GitHub Release; if newer, downloads and self-installs it, then restarts. |

The header subtitle previews the next switch (`Dark now • Next: dark at 19:00 (in 7 h)`); errors appear in red in the footer status line (bad time format, missing coordinates, save failures).

## 📁 Configuration file

Settings are stored as JSON at:

```
%APPDATA%\WinThemeAuto\config.json
```

Example:

```json
{
  "auto_enabled": true,
  "mode": "Sun",
  "light_at": "07:00:00",
  "dark_at": "19:00:00",
  "lat": 55.7558,
  "lon": 37.6173,
  "light_offset_min": -30,
  "dark_offset_min": 15,
  "change_apps": true,
  "change_system": true,
  "light_theme": "C:/Windows/Resources/Themes/aero.theme",
  "dark_theme": "",
  "light_wallpaper": "C:/Wallpapers/day.jpg",
  "dark_wallpaper": "C:/Wallpapers/night",
  "slideshow_interval_min": 30,
  "slideshow_shuffle": false,
  "lockscreen_enabled": true,
  "light_lockscreen": "C:/Wallpapers/lock-day.jpg",
  "dark_lockscreen": "C:/Wallpapers/lock-night.jpg",
  "accent_enabled": true,
  "light_accent": "0078D4",
  "dark_accent": "4CC2FF",
  "language": "en",
  "manual_hold": null,
  "manual_hold_until": null
}
```

Notes:

- `light_wallpaper` / `dark_wallpaper` accept an image file or a folder (folder = native Windows slideshow of its `jpg/png/bmp` images).
- Unknown or malformed fields are ignored; invalid times fall back to defaults (`07:00` / `19:00`).
- Out-of-range values are clamped (`lat`, `lon`, offsets); an expired `manual_hold_until` clears the hold.
- `manual_hold` / `manual_hold_until` are managed by Switch/CLI — no need to hand-edit them.
- Saves are atomic (`config.json.tmp` → rename). A fully unparseable file is backed up to `config.json.corrupt.bak` and defaults are used.
- Diagnostics go to `%APPDATA%\WinThemeAuto\app.log` (rotated at 256 KB, one backup kept; **Logs** button in Settings opens it).

## 🧠 How it works

- **Theme control:** reads/writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme` and `SystemUsesLightTheme` (`1` = light, `0` = dark).
- **Full themes (optional):** installed `.theme` files are enumerated from `C:\Windows\Resources\Themes` and `%LOCALAPPDATA%\Microsoft\Windows\Themes` (UTF-8/UTF-16 aware, `SystemMode`/`AppMode` + wallpaper parsed). A **custom wallpaper path** per mode (file picker, folder picker or manual entry, validated on Apply) wins over the theme wallpaper. A single image is applied via `SystemParametersInfoW`; a folder becomes a native Windows slideshow via `IDesktopWallpaper::SetSlideshow` (+ interval/shuffle) — no shell flashes, unlike launching `.theme` files.
- **Lock screen (optional):** per-mode lock image via WinRT (`TrySetLockScreenImageAsync`, `LockScreen` fallback, STA-safe) — no admin, per-user. Custom lock path wins; empty falls back to the desktop wallpaper/theme image.
- **Accent sync (optional):** per-mode accent applied through the same `SetUserColorPreference` path Settings uses (proper `AccentPalette` included; direct registry writes as fallback), then broadcast like a theme switch. Honors your `ColorPrevalence` setting — it won't force accent onto the taskbar if you turned that off.
- **Live refresh:** after a change, broadcasts `WM_SETTINGCHANGE (ImmersiveColorSet)` + `WM_THEMECHANGED` (repeated once after 200 ms for slow apps) and invalidates `Shell_TrayWnd` / `Shell_SecondaryTrayWnd` so the taskbar and apps update without logoff.
- **Scheduler:** at each scheduled switch (plus a 60 s safety net for clock changes and sleep/resume) the app computes the *desired* theme for `now` and applies it only if the registry doesn't already match (avoids redundant writes).
- **Fixed mode:** a circular time-interval check — handles both same-day (`07:00→19:00`) and overnight (`20:00→06:00`) ranges.
- **Sun mode:** offline solar calculation (mean anomaly → ecliptic longitude → transit → hour angle, `-0.833°` zenith correction) per date + longitude/latitude. Returns `Normal { rise, set }`, `PolarDay`, or `PolarNight`.
- **UI:** [Slint](https://slint.dev/) (Fluent style), single compact window with a stable size — info lines occupy reserved space so the window never jumps. The app's own title bar is forced light via `DwmSetWindowAttribute` for consistent readability.
- **Self-update:** compares `CARGO_PKG_VERSION` with the latest GitHub Release tag, downloads the arch-matching asset and hands over to a hidden updater script that waits for exit, swaps the exe and relaunches with the same args.
- **Autostart:** standard `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry.
- **Single instance:** a named mutex guards the process; a second launch forwards focus to the running window and exits.

## 🆚 Comparison

|  | **WinThemeAuto** | Auto Dark Mode | Windows Night light |
|---|---|---|---|
| Light/dark **theme** switching | ✅ | ✅ | ❌ (only color temperature) |
| Full theme + wallpaper | ✅ (optional, silent) | ✅ (via shell) | ❌ |
| Sunrise/sunset mode | ✅ (offline calc) | ✅ | ✅ |
| Portable single exe | ✅ (~12 MB) | ❌ (installer + service) | built-in |
| Open source | ✅ MIT | ✅ GPL | ❌ |
| Memory footprint | ~10 MB, tray only | service + app | system |

## 💡 Tips & tricks

- **Night owl?** Use overnight ranges like light `09:00` → dark `02:00` — crossing midnight just works.
- **Softer transitions:** set `Dark offset` to `+30` so dark mode kicks in 30 min *after* sunset, when it's actually dark outside.
- **Taskbar stuck?** The app already refreshes `Shell_TrayWnd` on every switch — if your taskbar still lags, it's a Windows quirk, not the app.
- **Multiple PCs:** copy `%APPDATA%\WinThemeAuto\config.json` between machines to clone your setup.
- **Manual mode:** disable **Auto switch** and uncheck autostart — the app becomes a pure tray toggle for themes.
- **Wallpaper that follows:** pick light/dark themes whose wallpapers you like — e.g. a bright photo theme for day, a dark abstract one for night. Or skip themes entirely and set two custom image paths.
- **Second launch?** It just focuses the already-running window — you'll never get duplicate tray icons.

## 🧩 Project structure

```
WinThemeAuto/
├── src/
│   ├── main.rs      # Entry point, CLI actions, event-loop wiring
│   ├── actions.rs   # Scheduler tick, toggle, apply, self-update flow
│   ├── ui.rs        # Window setup, language, dialogs, background tasks
│   ├── state.rs     # Shared app state + theme/hold helpers
│   ├── log.rs       # Rotating file log (%APPDATA%\WinThemeAuto\app.log)
│   ├── accent.rs    # Per-mode accent color via SetUserColorPreference + registry fallback
│   ├── cli.rs       # Headless flags: --toggle/--light/--dark/--status/--help
│   ├── config.rs    # Load/save/validate %APPDATA%\WinThemeAuto\config.json
│   ├── i18n.rs      # EN/RU strings for UI, status line, tray and schedule
│   ├── schedule.rs  # Fixed + sun scheduling, next-switch datetimes
│   ├── sun.rs       # Offline sunrise/sunset math
│   ├── theme.rs     # Registry read/write + broadcast
│   ├── lockscreen.rs # WinRT lock screen image (TrySet + LockScreen fallback)
│   ├── geo.rs       # IP geolocation (ipwho.is, ipapi.co fallback)
│   ├── tray.rs      # Tray icon + menu
│   ├── themes.rs    # Installed .theme enumeration + parsing
│   ├── single_instance.rs # Named-mutex guard + focus running window
│   ├── update.rs    # Self-update: check, download, self-install
│   └── autostart.rs # HKCU Run key management
├── ui/
│   └── main.slint   # Slint UI definition
├── build.rs         # Slint compiler hook
└── Cargo.toml
```

Key dependencies: `slint`, `winreg`, `tray-icon`, `chrono` (+`chrono-tz` for dev-tests), `serde` / `serde_json`, `ureq`, `dirs`, `windows-sys`, `windows` (WinRT lock screen), `anyhow`, `rfd` (native file picker).

## 🔒 Privacy

- Coordinates and preferences stay local in your config file.
- Network is used only when **you** ask for it:
  - **Detect via IP** → one GET to `https://ipwho.is/` (falls back to `https://ipapi.co/json/`)
  - **Check for updates** → GitHub Releases API + exe download from `github.com`
- Sun-time computation itself is fully offline.

## ❓ FAQ

<details>
<summary><b>Does it need admin rights?</b></summary>

No. Everything lives in `HKEY_CURRENT_USER` — no elevation, no service, no drivers.

</details>

<details>
<summary><b>Does it work on Windows 10?</b></summary>

Yes, Windows 10 1809+ and Windows 11, both 64- and 32-bit builds.

</details>

<details>
<summary><b>Does it send anything to the internet?</b></summary>

Only on your explicit action: **Detect via IP** queries `ipwho.is` (fallback `ipapi.co`), **Check for updates** queries the GitHub Releases API. Everything else — including sunrise/sunset math — is offline.

</details>

<details>
<summary><b>Why two exe files?</b></summary>

Native code can't be universal: `x64` is for 64-bit Windows (most PCs), `x86` for 32-bit ones (it also runs on 64-bit via compatibility mode).

</details>

<details>
<summary><b>How does self-update work?</b></summary>

The **Check for updates** button compares your version with the latest GitHub Release, downloads the matching exe, then a hidden updater script waits for the app to exit, swaps the file and restarts it (keeping your args like `--tray`). Settings in `%APPDATA%` are untouched.

</details>

<details>
<summary><b>Running the exe does nothing / no second window?</b></summary>

That's the single-instance guard: if the app is already running (look for the tray icon), a second launch just brings its window forward instead of starting a copy.

</details>

<details>
<summary><b>Light and dark at the same time in different apps?</b></summary>

Uncheck one of **Apps** / **System** and toggle manually — e.g. dark apps with a light taskbar.

</details>

## 🛠️ Troubleshooting

| Symptom | Fix |
|---------|-----|
| `Time must be HH:MM` | Use 24-hour format, e.g. `07:30`. |
| `Light and dark times must differ` | Pick two different times. |
| `Offset must be an integer from -180 to 180` | Offsets are minutes, e.g. `-30` = 30 min early. |
| `Auto-switch needs Apps or System` | Enable at least one of **Apps** / **System** while auto is on. |
| `Failed to save settings: …` | `%APPDATA%\WinThemeAuto` not writable — check permissions/antivirus; a corrupt file is backed up to `config.json.corrupt*.bak`. |
| `Enter coordinates…` | Sun mode needs valid lat/lon — use **Detect via IP** or enter manually. |
| `Location failed…` | No internet or IP service blocked — enter coordinates manually. |
| `Nothing to toggle` | Enable at least one of **Apps** / **System**. |
| `No switches` / `Polar day/night — stays …` | Expected above the Arctic Circle in summer/winter — theme stays fixed. |
| `Light/Dark wallpaper not found` | The custom path must point to an existing file or folder — use the `…` / `Folder` picker or fix the path. |
| `… slideshow folder has no images` | The folder contains no `jpg/png/bmp` files — add images or pick another folder. |
| `Slideshow interval must be 1–1440 minutes` | Interval is minutes, e.g. `30`. |
| `Light/Dark lock screen not found` | Same for lock images — use the `…` picker; empty falls back to desktop. |
| `Lock screen: …` | Shows the WinRT reason + `[ext, size]` — use a local `jpg/png` (<2 MB); `TrySet` false falls back to `LockScreen` API. |
| `Light/Dark accent must be hex RGB` | Use 6 hex digits, e.g. `0078D4` (a leading `#` is fine too). |
| Theme doesn't stick | Another app may be overwriting the registry keys; check for conflicting theme tools. |
| Accent not visible on taskbar | Enable **Show accent color on Start and taskbar** in Windows Settings → Personalization → Colors. |

## 📜 Changelog

### v0.6.1
- 🪶 Bounded-RAM previews: wallpaper/lock thumbnails via the shell thumbnail API (320×180) instead of full-file decode — fixes 300+ MB usage with large (4K/8K) wallpapers

### v0.6.0
- 🎞️ Wallpaper slideshows: per-mode **folder** of images applied as a native Windows slideshow (`IDesktopWallpaper`), with shared interval (1–1440 min) and shuffle; file paths keep working as single images
- 🖼️ Appearance tab: `Folder` picker buttons, interval + shuffle controls, folder preview via first image, EN/RU strings, validation on Apply
- 🔄 Slideshow follows auto-switch, manual Switch and CLI; lock-screen fallback reuses the folder's first image

### v0.5.0
- 🔒 Per-theme lock screen: separate light/dark images via WinRT (no admin), STA-safe COM, `LockScreen` fallback when `TrySet` returns false
- 🖼️ Appearance tab: lock checkbox + pickers with thumbnails, EN/RU strings, validation on Apply
- 🔄 Lock follows auto-switch, manual Switch, CLI and theme-wallpaper fallback; full error chain in status + log
- 📦 New dep: `windows 0.61` (Foundation/Storage/System_UserProfile/Com)

### v0.4.0
- ⏰ Exact scheduler: wakes precisely at each switch (60 s safety net for clock/sleep drift) instead of polling every 5 s
- ✋ Persistent hold: manual theme (window, tray or CLI) survives restarts — stored in config until the next switch
- 📝 Rotating log file with a **Logs** button in Settings
- 🌍 Geolocation fallback (`ipapi.co`) when the primary provider fails
- ⚡ Theme rescan moved off the UI thread; `main.rs` split into `actions` / `ui` / `state` modules
- 📦 Dependency refresh: `ureq 3`, `tray-icon 0.26`, `rfd 0.17`, `windows-sys 0.61`

### v0.3.0
- ✋ Manual override that sticks: Switch holds the hand-picked theme until the next scheduled change (auto no longer snaps it back in 5 s)
- 🌞🌙 Theme-aware tray: sun icon by day, moon by night, fully translated menu
- ✅ Honest Apply: edits are a draft until Apply (dirty `•` marker), language and autostart apply instantly and never get blocked by a broken time/coordinates
- 🖼️ Wallpaper thumbnails + one-click clear; update downloads show live progress with Cancel; Releases button
- 🛠️ Core fixes: external theme drift reconciled every tick, fail-closed single instance, accent/wallpaper applied on Apply when auto is off, bounded updater script with stale-exe fallback, DST-exact countdowns, Cyrillic `.theme` names, strict coordinates, fixed window size with Apply pinned at the bottom

### v0.2.8
- 🖥️ Headless CLI: `--toggle` / `--light` / `--dark` / `--status` / `--help` for hotkeys and scripts (works alongside the running app)
- 🔔 Live tray: status tooltip and Switch to light/dark label follow the theme
- 🔒 Verified self-updates: SHA256 checksum via release digest or `.sha256` asset
- 🌐 EN/RU interface with instant switch, translated status and schedule lines
- 🛠️ Core fixes: `0,0` coordinates valid, stale accent/wallpaper cache reset on Apply, exe-verified single instance, shaded accent palette, theme rescan without restart

### v0.2.7
- 📦 Slimmer exe: 12.7 MB → 10.6 MB (tray icon pre-rendered at build time, `panic=abort`)
- 🌐 Landing refresh: Lucide glyphs, accent + geolocation cards, brand text selection

### v0.2.6
- 🎨 Accent color sync: per-mode Windows accent applied on every switch (Settings-grade path with registry fallback), with one-click swatches
- 🗂️ Tabbed settings window: Auto switch / Appearance / Settings with a persistent Apply footer
- 🖼️ Custom wallpaper paths: per-mode image files (file picker included) that win over `.theme` wallpapers

### v0.2.2
- 🖥️ Reworked settings window: compact cards, stable size (no more jumping on location detect or status updates)
- ☀️ Sun offsets always visible, next switch shown in the header

### v0.2.1
- 🎨 Brand icon everywhere: window, tray, exe file, readme and website
- 🌐 Landing page with always-fresh download links

### v0.2.0
- 🖼️ Full Windows themes: pick light/dark `.theme` from installed ones, wallpaper follows silently
- 🛡️ Single-instance guard: second launch focuses the running window
- 📢 Double theme-change broadcast for slow apps

### v0.1.1
- 🔄 Self-updates: **Check for updates** button with automatic download, install and restart
- 🪟 Window sizing fixed: opens at full content size and grows with it (no more title-bar-only strip); wider default width (440px)
- Shows current version in the window footer

### v0.1.0
- Initial release: fixed-time and sunrise/sunset switching, tray icon, autostart, IP geolocation, x64 + x86 portable builds

## 🤝 Contributing

Issues and PRs are welcome — have an idea? [Open an issue](https://github.com/alnyx-dev/WinThemeAuto/issues).

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Please keep PRs focused and add/adjust unit tests for `config`, `schedule`, `sun`, `geo` or `update` logic when relevant.

## ⭐ Star history

If you find this useful, a star costs you one click and keeps the project alive:

[![Star History](https://api.star-history.com/svg?repos=alnyx-dev/WinThemeAuto&type=Date)](https://www.star-history.com/#alnyx-dev/WinThemeAuto&Date)

## 📄 License

MIT — see [LICENSE](LICENSE).
