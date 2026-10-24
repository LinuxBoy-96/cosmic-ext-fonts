name := 'cosmic-ext-fonts'
appid := 'io.github.linuxboy_96.cosmic-ext-fonts'

rootdir := ''
prefix := '/usr'
flatpak-prefix := '/app'

base-dir := absolute_path(clean(rootdir / prefix))
flatpak-base-dir := absolute_path(clean(rootdir / flatpak-prefix))

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name
flatpak-bin-dst := flatpak-base-dir / 'bin' / name

desktop := appid + '.desktop'
desktop-src := 'res' / 'app.desktop'
desktop-dst := clean(rootdir / prefix) / 'share' / 'applications' / desktop
flatpak-desktop-dst := clean(rootdir / flatpak-prefix) / 'share' / 'applications' / desktop

metainfo := appid + '.metainfo.xml'
metainfo-src := 'res' / 'metainfo.xml'
metainfo-dst := clean(rootdir / prefix) / 'share' / 'metainfo' / metainfo
flatpak-metainfo-dst := clean(rootdir / flatpak-prefix) / 'share' / 'metainfo' / metainfo

icon := appid + '.svg'
icon-src := 'res' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon
icon-dst := clean(rootdir / prefix) / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon
flatpak-icon-dst := clean(rootdir / flatpak-prefix) / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean
    rm -rf .flatpak-builder

# Removes vendored dependencies
clean-vendor:
    rm -rf vendor vendor.tar

# `cargo clean` and removes vendored dependencies
clean-dist: clean clean-vendor

# Compiles with debug profile
build-debug *args:
    cargo build {{args}}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline --config vendor/config.toml' args)

# Runs a clippy check (the core is kept free of pedantic warnings)
check *args:
    cargo clippy --workspace --all-targets {{args}} -- -W clippy::pedantic

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

# Runs the core tests only: fast, no libcosmic (needs the DejaVu fonts)
test-core:
    cargo test -p fonts-core

# Runs all tests, core and app (builds libcosmic)
test *args:
    cargo test --workspace {{args}}

# Cargo.lock lists `ring` (an optional dependency of rustls-webpki), but it's never built:
# that's what this checks.

# Fails if a crate that compiles C is in the build graph
pure:
    #!/usr/bin/env sh
    set -eu
    bad="$(cargo tree --workspace -e normal,build --prefix none | grep -E '^(ring|aws-lc-rs|aws-lc-sys|openssl|openssl-sys|native-tls|cmake|libz-sys|zstd-sys|lzma-sys|bzip2-sys) ' || true)"
    if [ -n "$bad" ]; then printf 'C code in the build graph:\n%s\n' "$bad" >&2; exit 1; fi
    echo "no C-compiling crate in the build graph"

# Run the application for testing purposes
run *args:
    env RUST_BACKTRACE=full cargo run --release {{args}}

# Installs files
install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{metainfo-src}} {{metainfo-dst}}
    install -Dm0644 {{icon-src}} {{icon-dst}}

# Installs files (Flatpak)
flatpak:
    install -Dm0755 {{bin-src}} {{flatpak-bin-dst}}
    install -Dm0644 {{desktop-src}} {{flatpak-desktop-dst}}
    install -Dm0644 {{metainfo-src}} {{flatpak-metainfo-dst}}
    install -Dm0644 {{icon-src}} {{flatpak-icon-dst}}

# Uninstalls installed files
uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{metainfo-dst}} {{icon-dst}}

# vendor.tar holds the crates plus vendor/config.toml pointing cargo at them, so
# .cargo/config.toml (the cross-build settings) is left alone.

# Vendor dependencies locally
vendor:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf vendor
    cargo vendor --locked vendor > vendor-config.toml
    mv vendor-config.toml vendor/config.toml
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# The generator is fetched, not vendored; it needs python3 with aiohttp and tomlkit.

# Regenerates cargo-sources.json for Flatpak; run it after changing Cargo.lock
flatpak-sources:
    #!/usr/bin/env sh
    set -eu
    tmp="$(mktemp -d)"
    curl -sfL https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py -o "$tmp/fcg.py"
    python3 -I "$tmp/fcg.py" Cargo.lock -o cargo-sources.json
    rm -rf "$tmp"

# Builds the Flatpak and installs it for the current user
flatpak-build:
    flatpak-builder --user --install --force-clean .flatpak-builder/build {{appid}}.json

# Needs desktop-file-utils and appstream. The validator doesn't know the `COSMIC` category
# (the COSMIC launcher groups apps by it), so that one message is ignored.

# Validates the desktop entry and the metainfo
lint-data:
    #!/usr/bin/env sh
    set -eu
    report="$(desktop-file-validate {{desktop-src}} 2>&1 || true)"
    other="$(printf '%s\n' "$report" | grep -v 'contains an unregistered value "COSMIC"' | grep . || true)"
    if [ -n "$other" ]; then printf '%s\n' "$other" >&2; exit 1; fi
    appstreamcli validate --no-net {{metainfo-src}}
