<div align="center">
  <br>
  <img src="res/icons/hicolor/scalable/apps/io.github.linuxboy_96.cosmic-ext-fonts.svg" width="150" />
  <h1>Fonts</h1>

  <p>Font manager for the COSMIC™ desktop</p>

  <img src="res/screenshots/window-light.png#gh-light-mode-only" />
  <img src="res/screenshots/window-dark.png#gh-dark-mode-only" />
</div>

Browse, preview, install and remove the fonts on your computer, grouped by family. It replaces
`gnome-font-viewer`, the last GTK app a COSMIC install needs for this.

## Features

- **Installed** tab: every family on the computer, yours and the system's, with a search box.
- **Google Fonts catalog** tab: preview any family before installing it. A preview installs
  nothing.
- *Open file…* (or a font file given on the command line) opens a `.ttf`, `.otf`, `.ttc` or
  `.otc` with an Install button. The chooser is the desktop's and starts in Downloads.
- Remove the fonts you installed, one face or a whole family, after a confirmation. A `.ttc`
  goes as a whole, and the question says how many faces it holds. System fonts are read-only.
- The list refreshes by itself when another program adds or removes a font.
- The View menu holds the theme (System, Light or Dark) and the About page.

Compared with gnome-font-viewer:

| gnome-font-viewer | Fonts |
|---|---|
| Says "Installed" without checking | Checks that the file was written and still parses |
| Spots duplicates by file name | Spots them by family, weight, style and width, face by face |
| Overwrites or fails on a name clash | Picks a free name (`Foo-1.ttf`) |
| Shows file names (`DejaVuSans.ttf`) | Shows the family name ("DejaVu Sans"), picked as COSMIC's font database picks it |

## How it works

```
cosmic-ext-fonts      GUI (libcosmic)
└── fonts-core        logic, no GUI, tested on a real font directory
    ├── inventory     fontdb scan, families, user or system
    ├── installer     install, uninstall, duplicate check
    ├── catalog       Google Fonts list and downloads (gwfh)
    └── watcher       live refresh of the user font directory
```

- Previews use COSMIC's own text stack (fontdb, cosmic-text), so what you see is what COSMIC
  draws. A font installed or previewed while the app runs is shown right away, no restart.
- Fonts are installed in `$XDG_DATA_HOME/fonts`, by default `~/.local/share/fonts`. No daemon,
  no Polkit.
- A font file can be up to 256 MiB and a collection up to 1024 faces. Anything bigger, or a
  header that lies about its size, is refused before it is read.
- The theme is saved in `~/.config/cosmic/io.github.linuxboy_96.cosmic-ext-fonts/v1/app_theme`.

### Google Fonts catalog

> [!WARNING]
> The catalog's HTTPS has not been audited. It uses rustls with the pure-Rust RustCrypto
> provider ([`rustls-rustcrypto`](https://github.com/RustCrypto/rustls-rustcrypto)), whose authors
> say not to use it in production, and `cargo audit` flags `rsa` (RUSTSEC-2023-0071, a timing
> leak of private keys). The app holds no private key, sends no credential and only downloads
> public fonts, which it checks like any local file. An app with secrets should use `ring` or
> `aws-lc-rs` instead.

The list comes from <https://gwfh.mranftl.com>, a community mirror of the Google Fonts index (no
API key). It is cached for a day in `$XDG_CACHE_HOME/cosmic-ext-fonts/`, and the old copy is used
when the network is down. A reply that doesn't parse never replaces a good cache.

Downloads are HTTPS only, from `fonts.gstatic.com` or `gwfh.mranftl.com`, and go through the same
checks as a local file. No C, no `ring`, no OpenSSL. The root certificates are Mozilla's, built in
(`webpki-roots`). The tests cover every TLS suite and key exchange, forged signatures and wrong
names (see `TEST.md`).

## Languages

English, French, German, Spanish, Italian, Portuguese and Dutch, in
`i18n/<lang>/cosmic_ext_fonts.ftl` (Fluent). The language follows the desktop. It sets the title
("Font Manager", "Gestionnaire de polices"…), the window name ("Fonts", "Polices"…) and the
preview text: a pangram plus the accented letters of the language, so a missing letter shows at
once.

To add a language, copy `i18n/en/` and translate it. The tests then ask for its alphabet row and
for its lines in `res/app.desktop` (`Name`, `Comment`, `Keywords`) and `res/metainfo.xml`.

## Desktop integration

Docks find the app by its id, `io.github.linuxboy_96.cosmic-ext-fonts`. The `.deb`, the Flatpak
and `just install` all install the desktop entry, the metainfo and the icon under that id, and the
`packaging` tests keep them in sync.

## Build

```sh
just test-core      # core logic only (needs the DejaVu fonts)
just test           # all tests (builds libcosmic)
just check          # clippy pedantic
just pure           # fails if anything in the build compiles C
just build-release  # the app
just run
sudo just install   # binary, desktop entry, metainfo, icon
just vendor && just build-vendored  # offline build
```

The justfile follows the cosmic-utils apps (calculator, examine). `just flatpak` installs into
`/app`, and `just rootdir=<dir> install` stages the files in a directory.

- Rust 1.93 or newer (edition 2024). libcosmic is pinned to a git revision and `Cargo.lock` is
  committed, so a fresh clone builds what was tested.
- The binary links glibc dynamically. `.cargo/config.toml` adds `cargo build-x86_64`,
  `build-arm64` and `build-riscv64`, with the Debian cross compilers.
- Everything is Rust except two system libraries: `libwayland-client` (loaded with `dlopen`) and
  `libxkbcommon`. No fontconfig, no GPU stack. `Cargo.lock` lists `ring` because `rustls-webpki`
  can use it, but it is never built (`just pure`).
- It draws on the CPU (tiny-skia) and hands the pixels to the compositor in shared memory, so it
  needs no GPU driver, Vulkan or EGL.
- *Open file…* and the About page's links go through `xdg-desktop-portal` over D-Bus (`zbus`), so
  no `xdg-open`. Without the portal, *Open file…* says so in a dialog.
- Command line: `cosmic-ext-fonts [--] [FONT_FILE]`, plus `-h` and `-V`.

**Flatpak:** `io.github.linuxboy_96.cosmic-ext-fonts.json` builds on the COSMIC BaseApp with the
`rust-stable` extension (`just flatpak-build`). Run `just flatpak-sources` to regenerate
`cargo-sources.json` whenever `Cargo.lock` changes. Fonts go to the host's `~/.local/share/fonts`,
so every app sees them, and the file chosen in *Open file…* comes in through the portal.

**Debian:** `cargo vendor debian/vendor`, then `dpkg-buildpackage -us -uc -b` (see
`debian/README.source`).

The tests are listed in `TEST.md`.

## License

GPL-3.0-or-later (GNU GPL version 3 or any later version, see `LICENSE`).
COSMIC™ is a trademark of System76. This is a community project, not affiliated with System76.
