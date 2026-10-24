// SPDX-License-Identifier: GPL-3.0-or-later

mod about;
mod app;
mod localize;
mod open_dialog;
#[cfg(test)]
mod packaging;
mod session;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use cosmic::iced::{Limits, Size};

const USAGE: &str = "usage: cosmic-ext-fonts [--] [FONT_FILE]";

/// Below this the search field in the header gets too narrow in French and German.
const MIN_WIDTH: f32 = 640.0;
const MIN_HEIGHT: f32 = 400.0;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    /// Start the GUI, on this font file if there is one.
    Open(Option<PathBuf>),
}

/// Parses the arguments, program name excluded.
///
/// File names stay `OsString`, so names that aren't valid UTF-8 still work.
fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut file: Option<PathBuf> = None;
    let mut only_files = false;

    for arg in args {
        let option = if only_files { None } else { arg.to_str() };
        match option {
            Some("-h" | "--help") => return Ok(Command::Help),
            Some("-V" | "--version") => return Ok(Command::Version),
            Some("--") => only_files = true,
            // A lone "-" too: we don't read fonts from stdin.
            Some(text) if text.starts_with('-') => {
                return Err(format!("unknown option '{text}'"));
            }
            _ => {
                if file.replace(PathBuf::from(arg)).is_some() {
                    return Err("only one font file can be opened at a time".to_owned());
                }
            }
        }
    }
    Ok(Command::Open(file))
}

fn main() -> ExitCode {
    // `cosmic-ext-fonts font.ttf` (e.g. from a file manager) opens that file directly.
    let open_path = match parse_args(std::env::args_os().skip(1)) {
        Ok(Command::Help) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Version) => {
            println!("cosmic-ext-fonts {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Command::Open(path)) => path,
        Err(message) => {
            eprintln!("cosmic-ext-fonts: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    if !session::has_wayland() {
        eprintln!("cosmic-ext-fonts: needs a Wayland session (WAYLAND_DISPLAY is not set)");
        return ExitCode::FAILURE;
    }

    localize::localize();

    let settings = cosmic::app::Settings::default()
        .size(Size::new(900.0, 650.0))
        .size_limits(Limits::NONE.min_width(MIN_WIDTH).min_height(MIN_HEIGHT));

    match cosmic::app::run::<app::AppModel>(settings, app::Flags { open_path }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cosmic-ext-fonts: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn no_argument_starts_the_browser() {
        assert_eq!(parse(&[]), Ok(Command::Open(None)));
    }

    #[test]
    fn one_argument_is_the_font_file() {
        assert_eq!(
            parse(&["a.ttf"]),
            Ok(Command::Open(Some(PathBuf::from("a.ttf"))))
        );
    }

    #[test]
    fn help_and_version_win_over_a_file() {
        assert_eq!(parse(&["a.ttf", "--help"]), Ok(Command::Help));
        assert_eq!(parse(&["-V", "a.ttf"]), Ok(Command::Version));
    }

    #[test]
    fn unknown_options_are_refused_instead_of_opened_as_files() {
        assert!(parse(&["--frobnicate"]).is_err());
        assert!(parse(&["-x"]).is_err());
    }

    #[test]
    fn double_dash_lets_a_file_name_start_with_a_dash() {
        assert_eq!(
            parse(&["--", "-odd.ttf"]),
            Ok(Command::Open(Some(PathBuf::from("-odd.ttf"))))
        );
        // After `--` even "--help" is just a (strange) file name.
        assert_eq!(
            parse(&["--", "--help"]),
            Ok(Command::Open(Some(PathBuf::from("--help"))))
        );
    }

    #[test]
    fn a_second_file_is_refused() {
        assert!(parse(&["a.ttf", "b.ttf"]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn file_names_that_are_not_utf8_survive() {
        use std::os::unix::ffi::OsStringExt;

        let name = OsString::from_vec(b"caf\xe9.ttf".to_vec());
        assert_eq!(
            parse_args([name.clone()]),
            Ok(Command::Open(Some(PathBuf::from(name))))
        );
    }
}
