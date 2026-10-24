// SPDX-License-Identifier: GPL-3.0-or-later
//
// The About page: creator, maintainer, repository and licence.
//
// libcosmic's own About page has fixed sections ("Developers", "Designers"…) and no way to say
// "Creator" or "Maintainer", so this one is built from the settings widgets.

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, icon};
use cosmic::{Element, theme};

use crate::fl;

/// Who made the application.
pub const CREATOR: &str = "Hugo Fortin";
/// Who looks after it.
pub const MAINTAINER: &str = "Hugo Fortin";
/// The `repository` from `Cargo.toml`.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// The text of the GPL 3 (`Cargo.toml` says `GPL-3.0-or-later`).
pub const LICENSE_URL: &str = "https://www.gnu.org/licenses/gpl-3.0.html";

/// The value shown on a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    /// Link text and the URL it opens.
    Link(String, &'static str),
}

/// The page's lines, translated: (label, value).
pub fn lines() -> Vec<(String, Value)> {
    vec![
        (fl!("about-creator"), Value::Text(CREATOR.to_owned())),
        (fl!("about-maintainer"), Value::Text(MAINTAINER.to_owned())),
        (
            fl!("about-link"),
            Value::Link(fl!("about-repository"), REPOSITORY),
        ),
        (
            fl!("about-license"),
            Value::Link(fl!("about-license-name"), LICENSE_URL),
        ),
    ]
}

/// The version line, e.g. "Version 0.1.0".
fn version() -> String {
    format!("{} {}", fl!("about-version"), env!("CARGO_PKG_VERSION"))
}

/// Icon, name and version on top, then the lines. `open` builds the message that opens a URL.
pub fn page<'a, Message: Clone + 'static>(
    name: String,
    icon: &icon::Handle,
    open: impl Fn(&'static str) -> Message,
) -> Element<'a, Message> {
    let space = theme::spacing();
    let header = widget::column::with_capacity(3)
        .push(widget::icon::icon(icon.clone()).size(128))
        .push(widget::text::title3(name))
        .push(widget::text::caption(version()))
        .spacing(space.space_xxs)
        .align_x(Alignment::Center);
    let mut section = widget::settings::section();
    for (label, value) in lines() {
        let answer: Element<'a, Message> = match value {
            Value::Text(text) => widget::text::body(text).into(),
            Value::Link(text, url) => widget::button::link(text)
                .trailing_icon(true)
                .on_press(open(url))
                .into(),
        };
        section = section.add(widget::settings::item(label, answer));
    }
    widget::column::with_capacity(2)
        .push(header)
        .push(section)
        .spacing(space.space_m)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// The page in the side drawer of the window.
pub fn drawer<'a, Message: Clone + 'static>(
    name: String,
    icon: &icon::Handle,
    open: impl Fn(&'static str) -> Message,
    close: Message,
) -> ContextDrawer<'a, Message> {
    context_drawer::context_drawer(page(name, icon, open), close).title(fl!("about"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SPDX licence from `Cargo.toml`.
    const LICENSE: &str = env!("CARGO_PKG_LICENSE");

    // The page must agree with the package, the store, the manifests and the source headers.
    const CARGO: &str = include_str!("../Cargo.toml");
    const CORE_CARGO: &str = include_str!("../fonts-core/Cargo.toml");
    const METAINFO: &str = include_str!("../res/metainfo.xml");
    const CONTROL: &str = include_str!("../debian/control");
    const CHANGELOG: &str = include_str!("../debian/changelog");
    const COPYRIGHT: &str = include_str!("../debian/copyright");
    const LICENSE_TEXT: &str = include_str!("../LICENSE");
    const README: &str = include_str!("../README.md");

    /// The first `key = "value"` of a manifest (the package's own).
    fn manifest_value(manifest: &'static str, key: &str) -> &'static str {
        manifest
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)?
                    .strip_prefix(" = \"")?
                    .strip_suffix('"')
            })
            .unwrap_or_else(|| panic!("no {key} in the manifest"))
    }

    #[test]
    fn the_page_names_the_creator_the_maintainer_the_repository_and_the_licence() {
        // No translation is loaded in tests: English.
        assert_eq!(
            lines(),
            [
                ("Creator".to_owned(), Value::Text("Hugo Fortin".into())),
                ("Maintainer".to_owned(), Value::Text("Hugo Fortin".into())),
                (
                    "Link".to_owned(),
                    Value::Link(
                        "Repository".into(),
                        "https://github.com/LinuxBoy-96/cosmic-ext-fonts"
                    )
                ),
                (
                    "License".to_owned(),
                    Value::Link(
                        "GPL 3.0 or later".into(),
                        "https://www.gnu.org/licenses/gpl-3.0.html"
                    )
                ),
            ]
        );
    }

    #[test]
    fn the_licence_of_the_page_is_the_one_of_the_package() {
        // "GPL 3.0 or later", linking to the GPL 3 text.
        assert_eq!(LICENSE, "GPL-3.0-or-later");
        assert!(LICENSE_URL.ends_with("/gpl-3.0.html"), "{LICENSE_URL}");
    }

    #[test]
    fn the_page_and_its_drawer_are_built_and_the_links_open_their_address() {
        let handle = icon::from_svg_bytes(include_bytes!(
            "../res/icons/hicolor/scalable/apps/io.github.linuxboy_96.cosmic-ext-fonts.svg"
        ));
        // Each link opens its own URL, and there are no other links.
        let opened = std::cell::RefCell::new(Vec::new());
        let _page: Element<'_, ()> = page("Fonts".to_owned(), &handle, |url| {
            opened.borrow_mut().push(url);
        });
        assert_eq!(*opened.borrow(), [REPOSITORY, LICENSE_URL]);
        let drawer: ContextDrawer<'_, ()> = drawer("Fonts".to_owned(), &handle, |_| (), ());
        assert_eq!(drawer.title.as_deref(), Some("About"));
    }

    #[test]
    fn every_file_that_names_the_licence_names_the_one_of_the_page() {
        // Both crates and the metainfo use the SPDX name.
        assert_eq!(manifest_value(CARGO, "license"), LICENSE);
        assert_eq!(manifest_value(CORE_CARGO, "license"), LICENSE);
        assert!(
            METAINFO.contains(&format!("<project_license>{LICENSE}</project_license>")),
            "metainfo"
        );
        // Debian calls it `GPL-3+`. Everything uses it except the metainfo, which is CC0 (stanzas
        // first, then the licence paragraphs), and the "any later version" wording is there.
        let licences: Vec<_> = COPYRIGHT
            .lines()
            .filter_map(|line| line.strip_prefix("License: "))
            .collect();
        assert_eq!(
            licences,
            ["GPL-3+", "CC0-1.0", "GPL-3+", "GPL-3+", "CC0-1.0"],
            "debian/copyright"
        );
        assert!(COPYRIGHT.contains("(at your option) any later version"));
        assert!(!COPYRIGHT.contains("GPL-3.0-only"), "debian/copyright");
        // `LICENSE` is the GPL 3 text, and the README names the licence.
        let title: Vec<_> = LICENSE_TEXT.split_whitespace().take(8).collect();
        assert_eq!(
            title,
            [
                "GNU", "GENERAL", "PUBLIC", "LICENSE", "Version", "3,", "29", "June"
            ],
            "LICENSE"
        );
        assert!(README.contains(&format!("\n{LICENSE} (")), "README");
    }

    #[test]
    fn every_source_file_carries_the_licence_of_the_page() {
        // Every file in `src/`, and the icon, has the SPDX line at the top.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files: Vec<_> = std::fs::read_dir(root.join("src"))
            .expect("the sources are there")
            .map(|entry| entry.expect("a source file").path())
            .collect();
        files.push(
            root.join("res/icons/hicolor/scalable/apps/io.github.linuxboy_96.cosmic-ext-fonts.svg"),
        );
        assert!(
            files.iter().any(|file| file.ends_with("src/main.rs")),
            "{files:?}"
        );
        for file in files {
            let text = std::fs::read_to_string(&file).expect("a text file");
            assert!(
                text.lines()
                    .take(2)
                    .any(|line| line.contains(&format!("SPDX-License-Identifier: {LICENSE}"))),
                "{}",
                file.display()
            );
        }
    }

    #[test]
    fn the_creator_and_the_maintainer_are_named_in_full_wherever_they_are_named() {
        // In `debian/control`, the changelog, the copyright file and the metainfo.
        assert!(
            CONTROL.contains(&format!("\nMaintainer: {MAINTAINER} <")),
            "control"
        );
        assert!(
            CHANGELOG.contains(&format!("\n -- {MAINTAINER} <")),
            "changelog"
        );
        // Every copyright stanza, not just the first.
        let holders: Vec<_> = COPYRIGHT
            .lines()
            .filter_map(|line| line.strip_prefix("Copyright: "))
            .collect();
        assert!(!holders.is_empty(), "copyright");
        for holder in holders {
            assert_eq!(holder, format!("2026 {CREATOR}"), "copyright");
        }
        assert!(
            COPYRIGHT.contains(&format!("\nUpstream-Contact: {CREATOR} <")),
            "copyright"
        );
        assert!(
            METAINFO.contains(&format!(
                "<developer id=\"io.github.linuxboy_96\">\n    <name>{CREATOR}</name>"
            )),
            "metainfo"
        );
    }

    #[test]
    fn the_link_of_the_page_is_the_repository_the_package_names() {
        assert_eq!(manifest_value(CARGO, "repository"), REPOSITORY);
        assert!(
            METAINFO.contains(&format!("<url type=\"homepage\">{REPOSITORY}</url>")),
            "metainfo"
        );
        assert!(
            CONTROL.contains(&format!("\nHomepage: {REPOSITORY}\n")),
            "control"
        );
        assert!(
            COPYRIGHT.contains(&format!("\nSource: {REPOSITORY}\n")),
            "copyright"
        );
    }
}
