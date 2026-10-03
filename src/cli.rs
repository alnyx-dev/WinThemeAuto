//! Headless command-line actions for scripts, hotkeys and automation.
//!
//! The GUI remains the default (no args, or `--tray` for autostart).
//! Any action flag takes precedence and runs without a window:
//! `--toggle`, `--light`, `--dark`, `--status`, `--help`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Launch the GUI. `start_hidden` = `--tray`.
    Ui {
        start_hidden: bool,
    },
    Toggle,
    Light,
    Dark,
    Status,
    Help,
    /// Unknown flag; the string names it for the error message.
    Invalid,
}

/// Parse `std::env::args()` (including argv[0]).
/// Unknown `--flags` map to [`Action::Invalid`]; positional args are ignored
/// so file associations / launchers never break the GUI.
pub fn parse(args: impl IntoIterator<Item = String>) -> Action {
    let mut start_hidden = false;
    let mut action: Option<Action> = None;
    for arg in args.into_iter().skip(1) {
        match arg.as_str() {
            "--tray" => start_hidden = true,
            "--toggle" => {
                action.get_or_insert(Action::Toggle);
            }
            "--light" => {
                action.get_or_insert(Action::Light);
            }
            "--dark" => {
                action.get_or_insert(Action::Dark);
            }
            "--status" => {
                action.get_or_insert(Action::Status);
            }
            "--help" | "-h" | "-?" | "/?" => {
                action.get_or_insert(Action::Help);
            }
            s if s.starts_with('-') || s.starts_with('/') => {
                action.get_or_insert(Action::Invalid);
            }
            _ => {}
        };
    }
    match action {
        Some(a) => a,
        None => Action::Ui { start_hidden },
    }
}

pub const HELP: &str = "\
WinThemeAuto — light/dark theme switcher

Usage:
  WinThemeAuto.exe [options]

Options:
  (no args)   Open the settings window
  --tray      Start hidden in the tray (used for autostart)
  --toggle    Flip light <-> dark now, no window
  --light     Switch to light now, no window
  --dark      Switch to dark now, no window
  --status    Print current theme and next switch, no window
  --help      Show this text
";

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Action {
        parse(s.iter().map(|x| x.to_string()))
    }

    #[test]
    fn defaults_to_ui() {
        assert_eq!(
            args(&["exe"]),
            Action::Ui {
                start_hidden: false
            }
        );
        assert_eq!(args(&["exe", "--tray"]), Action::Ui { start_hidden: true });
    }

    #[test]
    fn actions_win_over_tray() {
        assert_eq!(args(&["exe", "--tray", "--toggle"]), Action::Toggle);
        assert_eq!(args(&["exe", "--toggle", "--tray"]), Action::Toggle);
        assert_eq!(args(&["exe", "--light"]), Action::Light);
        assert_eq!(args(&["exe", "--dark"]), Action::Dark);
        assert_eq!(args(&["exe", "--status"]), Action::Status);
        assert_eq!(args(&["exe", "--help"]), Action::Help);
        assert_eq!(args(&["exe", "-h"]), Action::Help);
    }

    #[test]
    fn first_action_wins_and_unknown_is_invalid() {
        assert_eq!(args(&["exe", "--light", "--dark"]), Action::Light);
        assert_eq!(args(&["exe", "--nope"]), Action::Invalid);
        // Positional args are ignored.
        assert_eq!(
            args(&["exe", "somefile.txt"]),
            Action::Ui {
                start_hidden: false
            }
        );
    }
}
