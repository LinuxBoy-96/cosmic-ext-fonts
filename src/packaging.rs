// SPDX-License-Identifier: GPL-3.0-or-later
//
// Docks find a window's desktop entry by its Wayland app id and take the icon and name from it.
// So every install path (Debian, justfile, Flatpak) must use the same names, or the dock shows a
// generic icon and the tail of the id as the name.

const APP_ID: &str = <crate::app::AppModel as cosmic::Application>::APP_ID;
const PROGRAM: &str = env!("CARGO_PKG_NAME");

const DESKTOP: &str = include_str!("../res/app.desktop");
const METAINFO: &str = include_str!("../res/metainfo.xml");
const DEBIAN_RULES: &str = include_str!("../debian/rules");
const JUSTFILE: &str = include_str!("../justfile");
const FLATPAK: &str = include_str!("../io.github.linuxboy_96.cosmic-ext-fonts.json");
const DEBIAN_CONTROL: &str = include_str!("../debian/control");
const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// A key's value in the desktop entry (the untranslated one).
fn desktop_value(key: &str) -> &'static str {
    DESKTOP
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("the desktop entry has no {key}"))
}

/// Where the entry, the metainfo and the `hicolor` icon go under the install prefix, each named
/// `<app id><extension>`.
const INSTALLED: [(&str, &str); 3] = [
    ("share/applications", ".desktop"),
    ("share/metainfo", ".metainfo.xml"),
    ("share/icons/hicolor/scalable/apps", ".svg"),
];

/// The justfile lines that install those three files from `res/` (`just install`, and
/// `just flatpak` into `/app`).
const JUST_FILES: [&str; 12] = [
    "desktop := appid + '.desktop'",
    "desktop-src := 'res' / 'app.desktop'",
    "desktop-dst := clean(rootdir / prefix) / 'share' / 'applications' / desktop",
    "flatpak-desktop-dst := clean(rootdir / flatpak-prefix) / 'share' / 'applications' / desktop",
    "metainfo := appid + '.metainfo.xml'",
    "metainfo-src := 'res' / 'metainfo.xml'",
    "metainfo-dst := clean(rootdir / prefix) / 'share' / 'metainfo' / metainfo",
    "flatpak-metainfo-dst := clean(rootdir / flatpak-prefix) / 'share' / 'metainfo' / metainfo",
    "icon := appid + '.svg'",
    "icon-src := 'res' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon",
    "icon-dst := clean(rootdir / prefix) / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon",
    "flatpak-icon-dst := clean(rootdir / flatpak-prefix) / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon",
];

fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|candidate| candidate == line)
}

/// Whether `text` installs `<app id><extension>` into `root/directory`, by full path or with
/// `install -t` (which keeps the file name).
fn puts(text: &str, root: &str, directory: &str, extension: &str) -> bool {
    let file = format!("{APP_ID}{extension}");
    text.contains(&format!("{root}/{directory}/{file}"))
        || text.contains(&format!("{file} -t {root}/{directory}/"))
}

#[test]
fn a_wayland_window_finds_its_entry_by_the_application_id() {
    // libcosmic gives the compositor `APP_ID`; every install method must use it.
    for (way, text, root) in [
        ("debian/rules", DEBIAN_RULES, "usr"),
        ("Flatpak manifest", FLATPAK, "/app"),
    ] {
        for (directory, extension) in INSTALLED {
            assert!(
                puts(text, root, directory, extension),
                "{way}: {APP_ID}{extension} in {root}/{directory}"
            );
        }
    }
    // Same places and names for `just install` and `just flatpak`.
    assert!(
        has_line(JUSTFILE, &format!("appid := '{APP_ID}'")),
        "justfile"
    );
    for line in JUST_FILES {
        assert!(has_line(JUSTFILE, line), "justfile: {line}");
    }
    for file in ["desktop", "metainfo", "icon"] {
        for destination in [format!("{file}-dst"), format!("flatpak-{file}-dst")] {
            let line = format!("    install -Dm0644 {{{{{file}-src}}}} {{{{{destination}}}}}");
            assert!(has_line(JUSTFILE, &line), "justfile: {line}");
        }
    }
    assert!(
        FLATPAK.contains(&format!("\"id\": \"{APP_ID}\"")),
        "Flatpak id"
    );
    // The entry's icon is `<id>.svg`, and that file exists.
    assert_eq!(desktop_value("Icon"), APP_ID);
    let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("res/icons/hicolor/scalable/apps")
        .join(format!("{APP_ID}.svg"));
    assert!(
        icon.is_file(),
        "{} is the icon of the entry",
        icon.display()
    );
    // The store uses the same id.
    assert!(METAINFO.contains(&format!("<id>{APP_ID}</id>")));
    assert!(METAINFO.contains(&format!(
        "<launchable type=\"desktop-id\">{APP_ID}.desktop</launchable>"
    )));
}

#[test]
fn the_entry_runs_the_program_every_way_installs() {
    // The program name must match everywhere it's installed or run.
    assert_eq!(
        desktop_value("Exec").split_whitespace().next(),
        Some(PROGRAM)
    );
    assert_eq!(desktop_value("Type"), "Application");
    assert_eq!(desktop_value("Terminal"), "false");
    assert!(
        has_line(JUSTFILE, &format!("name := '{PROGRAM}'")),
        "justfile"
    );
    for line in [
        "    install -Dm0755 {{bin-src}} {{bin-dst}}",
        "    install -Dm0755 {{bin-src}} {{flatpak-bin-dst}}",
    ] {
        assert!(has_line(JUSTFILE, line), "justfile: {line}");
    }
    assert!(
        DEBIAN_RULES.contains(&format!("/usr/bin/{PROGRAM}")),
        "debian/rules"
    );
    assert!(
        FLATPAK.contains(&format!("\"command\": \"{PROGRAM}\"")),
        "Flatpak command"
    );
}

/// The store fetches screenshots from the repository: each must be a real file in
/// `res/screenshots`, window-sized, and the first one the default.
#[test]
fn the_screenshots_of_the_store_are_files_of_the_repository() {
    let repository = env!("CARGO_PKG_REPOSITORY");
    let raw = format!(
        "{}/main/",
        repository.replace("https://github.com/", "https://raw.githubusercontent.com/")
    );
    let images: Vec<&str> = METAINFO
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("<image>")?
                .strip_suffix("</image>")
        })
        .collect();
    assert_eq!(images.len(), 2, "{images:?}");
    for image in images {
        let file = image
            .strip_prefix(raw.as_str())
            .unwrap_or_else(|| panic!("{image} is not a file of {repository}"));
        assert!(file.starts_with("res/screenshots/"), "{file}");
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "{file} is a PNG");
        // Size from the PNG header: a window, not a thumbnail.
        let side = |at: usize| {
            u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        assert!(
            side(16) >= 640 && side(20) >= 400,
            "{file}: {}x{}",
            side(16),
            side(20)
        );
    }
    let first = METAINFO
        .lines()
        .map(str::trim)
        .skip_while(|line| *line != "<screenshots>")
        .nth(1);
    assert_eq!(first, Some("<screenshot type=\"default\">"));
}

/// Flathub finds the repository from the id: `io.github.<owner>.<repository>`.
#[test]
fn the_id_names_the_repository() {
    let (owner, name) = env!("CARGO_PKG_REPOSITORY")
        .strip_prefix("https://github.com/")
        .and_then(|path| path.split_once('/'))
        .expect("a GitHub repository");
    let owner = owner.to_lowercase().replace('-', "_");
    assert_eq!(APP_ID, format!("io.github.{owner}.{name}"));
}

/// Wayland only, drawn on the CPU, links through the portal: nothing asks for X11, a GPU or
/// `xdg-utils`.
#[test]
fn nothing_asks_for_x11_a_gpu_or_xdg_utils() {
    for feature in ["\"x11\"", "\"wgpu\""] {
        assert!(!CARGO_TOML.contains(feature), "Cargo.toml: {feature}");
    }
    for arg in ["x11", "--share=ipc", "--device=dri"] {
        assert!(!FLATPAK.contains(arg), "Flatpak manifest: {arg}");
    }
    for package in [
        "libx11",
        "libxcb",
        "libxi",
        "xkbcommon-x11",
        "vulkan",
        "egl",
        "mesa",
        "xdg-utils",
    ] {
        assert!(
            !DEBIAN_CONTROL.contains(package),
            "debian/control: {package}"
        );
    }
    assert!(
        DESKTOP
            .lines()
            .all(|line| !line.starts_with("StartupWMClass"))
    );
}
