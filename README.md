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
- [🗺️ Roadmap](#️-roadmap)
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
- **IP geolocation** — one-click coordinate detection via `ipwho.is` (or enter coordinates manually)
- **System tray** — close minimizes to tray; double-click tray icon to reopen; `Open` / `Toggle theme` / `Exit` menu
- **Start with Windows** — registry `Run` key autostart (starts hidden with `--tray`)
- **Instant apply** — broadcasts `WM_SETTINGCHANGE` / `WM_THEMECHANGED` and refreshes taskbars so apps pick up the change immediately
- **Robust config** — tolerant JSON parsing, validation/clamping, atomic saves, corrupt-file backup
- 🖼️ **Full Windows themes** — pick a light and a dark `.theme` from installed ones; the wallpaper follows the switch, silently. Or point each mode at **any image file** — custom wallpapers win over theme ones
- 🔄 **Self-updates** — one click checks GitHub Releases, downloads the newest exe (x64/x86 auto-matched) and installs it with a restart
- **Single instance** — a second launch just focuses the running window instead of duplicating tray icons

## 📥 Download

| File | Architecture | Who is it for? |
|------|--------------|----------------|
| `WinThemeAuto-x64.exe` | 64-bit (x86-64) | Most modern PCs — **download this one** |
| `WinThemeAuto-x86.exe` | 32-bit (x86) | Older 32-bit Windows, or 64-bit via WOW64 compatibility |

Get them from the [latest release](https://github.com/alnyx-dev/WinThemeAuto/releases). No installer, no admin rights — each exe is a single portable file.

**System requirements:** Windows 10 (1809+) or Windows 11. ~12 MB download, ~0% CPU when idle (wakes up once every 5 s).

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

1. Open the app — the header shows the current effective theme (`Light now` / `Dark now`) with a quick **Switch** button.
2. Under **Auto switch**, toggle **Enabled** (closing the window minimizes to tray while enabled).
3. Choose a mode:
   - **By time**: set `Light from` and `Dark from` in `HH:MM` (24-hour) format.
   - **Sunrise and sunset**: enter latitude/longitude, or click **Detect via IP**, then optionally set offsets.
4. Under **Apply theme to**, check **Apps** and/or **System**.
5. (Optional) under **Windows themes**, pick a full `.theme` for light and dark — the wallpaper will follow each switch. Or set a **custom wallpaper path** per mode (via the `…` picker) — it wins over the theme wallpaper.
6. Click **Apply**. Settings save automatically and take effect within ~5 seconds.

Closing the window hides it to the tray — use the tray menu or double-click the icon to bring it back.

### Tray menu

| Action         | What it does                          |
|----------------|---------------------------------------|
| `Open`         | Show the main window                  |
| `Toggle theme` | Flip light ↔ dark immediately         |
| `Exit`         | Quit the app (auto-switch stops)      |

Double-clicking the tray icon also opens the window.

### CLI

```
WinThemeAuto-x64.exe [--tray]
```

| Flag    | Description                              |
|---------|------------------------------------------|
| `--tray`| Start hidden in the tray (used for autostart) |

### Updates

The footer shows the current version (e.g. v0.2.0) and a **Check for updates** button. If a newer release exists, the app downloads the matching exe (x64/x86 auto-detected), installs it over itself and restarts — settings are kept. See [How does self-update work?](#-faq) for details.

## ⚙️ Settings reference

| Setting | Description |
|---------|-------------|
| `Enabled` | Master switch for automatic switching. Checked every 5 s. |
| `By time / Sunrise and sunset` | Switching mode. |
| `Light from` / `Dark from` | Fixed-mode boundaries (`HH:MM`). May cross midnight. Must differ. |
| `Lat.` / `Lon.` | Coordinates for sun mode. Valid ranges: lat `-90…90`, lon `-180…180`. |
| `Detect via IP` | Fills in coordinates from your public IP (online, one-shot). You still need to press **Apply**. |
| `Light/Dark offset (min)` | Shift relative to sunrise/sunset. Integer `-180…180`. Negative = earlier. |
| `Apps` / `System` | Which registry values to manage. At least one should be on for auto-switch info to appear. |
| `Light/Dark theme` | Full installed `.theme` for each mode — switches flags **plus** wallpaper. `System default` = flags only. |
| `Light/Dark wallpaper` | Custom image path (`jpg/png/bmp`) per mode — wins over the theme wallpaper. `…` opens a file picker. Empty = theme only. |
| `Start with Windows` | Writes `HKCU\...\Run\WinThemeAuto = "<exe>" --tray`. Uncheck to remove. |
| `Check for updates` | Compares the app version with the latest GitHub Release; if newer, downloads and self-installs it, then restarts. |

The status line shows helpful hints (`Next: dark at 19:00 (in 7 h)`), and errors appear in red (bad time format, missing coordinates, save failures).

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
  "light_wallpaper": "C:/Wallpapers/day.jpg",
  "dark_wallpaper": "C:/Wallpapers/night.jpg"
}
```

Notes:

- Unknown or malformed fields are ignored; invalid times fall back to defaults (`07:00` / `19:00`).
- Out-of-range values are clamped (`lat`, `lon`, offsets).
- Saves are atomic (`config.json.tmp` → rename). A fully unparseable file is backed up to `config.json.corrupt.bak` and defaults are used.

## 🧠 How it works

- **Theme control:** reads/writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme` and `SystemUsesLightTheme` (`1` = light, `0` = dark).
- **Full themes (optional):** installed `.theme` files are enumerated from `C:\Windows\Resources\Themes` and `%LOCALAPPDATA%\Microsoft\Windows\Themes` (UTF-8/UTF-16 aware, `SystemMode`/`AppMode` + wallpaper parsed). A **custom wallpaper path** per mode (file picker or manual entry, validated on Apply) wins over the theme wallpaper. On a switch the wallpaper is applied via `SystemParametersInfoW` — no shell flashes, unlike launching `.theme` files.
- **Live refresh:** after a change, broadcasts `WM_SETTINGCHANGE (ImmersiveColorSet)` + `WM_THEMECHANGED` (repeated once after 200 ms for slow apps) and invalidates `Shell_TrayWnd` / `Shell_SecondaryTrayWnd` so the taskbar and apps update without logoff.
- **Scheduler:** every 5 s the app computes the *desired* theme for `now` and applies it only if the registry doesn't already match (avoids redundant writes).
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

## 🗺️ Roadmap

- [x] Fixed-time and sunrise/sunset switching
- [x] Tray icon + autostart
- [x] x64 and x86 release builds via GitHub Actions
- [x] Self-updates via GitHub Releases
- [x] Wallpaper follows the theme (via `.theme` wallpapers)
- [x] Custom wallpaper paths (independent of themes)
- [ ] Accent-color sync option
- [ ] Hotkey for instant toggle
- [ ] Screen brightness follow (laptops)

Have an idea? [Open an issue](https://github.com/alnyx-dev/WinThemeAuto/issues) — feature requests are welcome.

## 🧩 Project structure

```
WinThemeAuto/
├── src/
│   ├── main.rs      # UI wiring, timers, tray loop, settings apply
│   ├── config.rs    # Load/save/validate %APPDATA%\WinThemeAuto\config.json
│   ├── schedule.rs  # Fixed + sun scheduling, "next switch" text
│   ├── sun.rs       # Offline sunrise/sunset math
│   ├── theme.rs     # Registry read/write + broadcast
│   ├── geo.rs       # IP geolocation (ipwho.is)
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

Key dependencies: `slint`, `winreg`, `tray-icon`, `chrono`, `serde` / `serde_json`, `ureq`, `dirs`, `windows-sys`, `anyhow`, `rfd` (native file picker).

## 🔒 Privacy

- Coordinates and preferences stay local in your config file.
- Network is used only when **you** ask for it:
  - **Detect via IP** → one GET to `https://ipwho.is/`
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

Only on your explicit action: **Detect via IP** queries `ipwho.is`, **Check for updates** queries the GitHub Releases API. Everything else — including sunrise/sunset math — is offline.

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
| `Enter coordinates…` | Sun mode needs valid lat/lon — use **Detect via IP** or enter manually. |
| `Location failed…` | No internet or IP service blocked — enter coordinates manually. |
| `Nothing to toggle` | Enable at least one of **Apps** / **System**. |
| `No switches: polar day/night` | Expected above the Arctic Circle in summer/winter — theme stays fixed. |
| `Light/Dark wallpaper not found` | The custom path must point to an existing file — use the `…` picker or fix the path. |
| Theme doesn't stick | Another app may be overwriting the registry keys; check for conflicting theme tools. |

## 📜 Changelog

### v0.2.5
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

Issues and PRs are welcome:

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Please keep PRs focused and add/adjust unit tests for `config`, `schedule`, `sun`, `geo` or `update` logic when relevant.

## ⭐ Star history

If you find this useful, a star costs you one click and keeps the project alive:

[![Star History](https://starchart.cc/alnyx-dev/WinThemeAuto.svg?variant=adaptive)](https://star-history.com/#alnyx-dev/WinThemeAuto&Timeline)

## 📄 License

MIT — see [LICENSE](LICENSE).
