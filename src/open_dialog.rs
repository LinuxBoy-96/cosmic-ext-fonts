// SPDX-License-Identifier: GPL-3.0-or-later
//
// "Open file…" uses the desktop's file chooser through the XDG portal. On COSMIC that's the
// COSMIC Files dialog, the same one COSMIC Edit uses. Links go through the portal too.

use std::path::{Path, PathBuf};

use cosmic::dialog::ashpd::desktop::ResponseError;
use cosmic::dialog::ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use cosmic::dialog::ashpd::desktop::open_uri::OpenFileRequest;
use cosmic::dialog::ashpd::{self, url::Url};

use crate::fl;

/// The font files we open, whatever the case of the extension.
const FONT_EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];

/// `ttf` → `*.[tT][tT][fF]`: filter globs are case-sensitive, so both cases are spelled out.
fn glob(extension: &str) -> String {
    let mut pattern = String::from("*.");
    for letter in extension.chars() {
        pattern.push('[');
        pattern.push(letter.to_ascii_lowercase());
        pattern.push(letter.to_ascii_uppercase());
        pattern.push(']');
    }
    pattern
}

/// Show font files only.
fn font_filter() -> FileFilter {
    FONT_EXTENSIONS
        .into_iter()
        .fold(FileFilter::new(&fl!("font-filter")), |filter, extension| {
            filter.glob(&glob(extension))
        })
}

/// Start in Downloads (where fonts usually land), else home. `None` lets the desktop pick.
pub fn start_folder() -> Option<PathBuf> {
    start_in(dirs::download_dir(), dirs::home_dir())
}

fn start_in(downloads: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    downloads
        .or_else(|| home.as_ref().map(|home| home.join("Downloads")))
        .filter(|dir| dir.is_dir())
        .or(home)
}

/// The chosen file: the first URI, which must be a local file.
fn file_of(uris: &[Url]) -> Result<PathBuf, String> {
    uris.first()
        .and_then(|uri| uri.to_file_path().ok())
        .ok_or_else(|| "the file chooser answered without a file of this computer".to_string())
}

/// Whether the user closed the dialog without choosing (the portal reports it as an error).
fn cancelled(error: &ashpd::Error) -> bool {
    matches!(error, ashpd::Error::Response(ResponseError::Cancelled))
}

/// The error and its causes on one line; the portal's own message is in the causes.
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

/// Shows the dialog, starting in `start`. `Ok(None)` means it was closed without a choice.
pub async fn ask(start: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    let request = SelectedFiles::open_file()
        .title(fl!("open-font-title").as_str())
        .modal(true)
        .filter(font_filter())
        .current_folder::<&Path>(start.as_deref())
        .map_err(|error| chain(&error))?;
    match request.send().await.and_then(|answer| answer.response()) {
        Ok(selected) => file_of(selected.uris()).map(Some),
        Err(error) if cancelled(&error) => Ok(None),
        Err(error) => Err(chain(&error)),
    }
}

/// Opens `url` with the default app. The portal skips its app chooser for web links.
pub async fn open_uri(url: &str) -> Result<(), String> {
    let uri = Url::parse(url).map_err(|error| error.to_string())?;
    OpenFileRequest::default()
        .send_uri(&uri)
        .await
        .and_then(|request| request.response())
        .map_err(|error| chain(&error))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty temp folder, deleted on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "cosmic-ext-fonts-test-{tag}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("a scratch folder");
            Self(dir)
        }

        fn folder(&self, name: &str) -> PathBuf {
            let dir = self.0.join(name);
            std::fs::create_dir_all(&dir).expect("a folder");
            dir
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_dialog_shows_font_files_whatever_the_case_of_their_extension() {
        let filter = font_filter();
        assert_eq!(filter.label(), "Fonts");
        assert_eq!(
            filter.pattern_filters(),
            [
                "*.[tT][tT][fF]",
                "*.[oO][tT][fF]",
                "*.[tT][tT][cC]",
                "*.[oO][tT][cC]",
            ]
        );
        assert!(filter.mimetype_filters().is_empty());
    }

    #[test]
    fn the_dialog_starts_in_downloads_else_at_home() {
        let scratch = Scratch::new("start");
        let home = scratch.folder("home");

        // No Downloads anywhere: home.
        assert_eq!(start_in(None, Some(home.clone())), Some(home.clone()));
        // The folder the desktop is set to, if it exists…
        let named = scratch.folder("Telechargements");
        assert_eq!(
            start_in(Some(named.clone()), Some(home.clone())),
            Some(named)
        );
        // …else ~/Downloads, if it exists…
        let plain = scratch.folder("home/Downloads");
        assert_eq!(start_in(None, Some(home.clone())), Some(plain));
        // …and a setting pointing to a missing folder falls back to home.
        let gone = scratch.0.join("deleted");
        assert_eq!(start_in(Some(gone), Some(home.clone())), Some(home));
        // Nothing known: the desktop decides.
        assert_eq!(start_in(None, None), None);
    }

    #[test]
    fn the_file_is_the_first_address_and_a_file_of_this_computer() {
        let uris = |texts: &[&str]| -> Vec<Url> {
            texts.iter().map(|text| Url::parse(text).unwrap()).collect()
        };
        assert_eq!(
            file_of(&uris(&["file:///usr/share/fonts/DejaVu%20Sans.ttf"])),
            Ok(PathBuf::from("/usr/share/fonts/DejaVu Sans.ttf"))
        );
        assert_eq!(
            file_of(&uris(&["file:///a.otf", "file:///b.otf"])),
            Ok(PathBuf::from("/a.otf"))
        );
        // Not a local file, or nothing at all.
        assert!(file_of(&uris(&["https://example.com/a.ttf"])).is_err());
        assert!(file_of(&[]).is_err());
    }

    #[test]
    fn closing_the_dialog_is_not_a_failure() {
        assert!(cancelled(&ashpd::Error::Response(ResponseError::Cancelled)));
        assert!(!cancelled(&ashpd::Error::Response(ResponseError::Other)));
        assert!(!cancelled(&ashpd::Error::NoResponse));
    }

    #[test]
    fn an_error_is_told_with_its_causes() {
        #[derive(Debug)]
        struct Outer(std::io::Error);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("open dialog failed")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let error = Outer(std::io::Error::other("no such portal"));
        assert_eq!(chain(&error), "open dialog failed: no such portal");
    }
}
