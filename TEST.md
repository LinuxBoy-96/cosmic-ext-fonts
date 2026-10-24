# Tests

Every test in the project and what it checks, then what was checked by hand.

## Before you start

- DejaVu Sans and DejaVu Serif (`fonts-dejavu-core` on Debian) for the tests marked *real font*.
  The other core tests build their fonts in memory.
- A writable `$TMPDIR`. Tests write only in temporary directories they delete.
- No network: the catalog tests use fixtures, and the HTTPS tests talk to their own server on
  `127.0.0.1` (they ignore `HTTPS_PROXY`).

## Commands

| Command | Runs |
|---|---|
| `cargo test -p fonts-core` | the core tests (41) |
| `cargo test --workspace` | everything (101 tests; builds libcosmic) |
| `cargo test -p fonts-core <filter>` | the tests whose name contains `<filter>` |
| `just test-core`, `just test` | the same two suites |
| `just check` | clippy pedantic on the workspace (0 warnings) |
| `just pure` | fails if `ring`, `aws-lc`, OpenSSL, zlib/zstd/lzma/bzip2 bindings or `cmake` are in the build |
| `just lint-data` | `desktop-file-validate` and `appstreamcli validate` on `res/` (only the `COSMIC` category message is allowed) |
| `cargo fmt --all -- --check` | formatting |

## By hand

The tests build every view in every state, but nothing draws them. Check with `just run`:

1. Start: the title says "Font Manager" ("Gestionnaire de polices" with `LANG=fr_CA.UTF-8`) and
   *Installed* reads "N families on this computer · M user-installed". While searching: "X of N
   families". An untranslated language (`LANG=C`) gives English.
2. Installed → a family: the waterfall follows the selected face and starts with the pangram of
   the interface language.
3. *Open file…*: the desktop's chooser opens in Downloads and shows only font files (`.TTF` too)
   → preview → Install → Uninstall (Confirm or Cancel). The preview uses the installed font right
   away. Closing the chooser changes nothing; without a portal, a dialog says so.
4. A `.ttc`: uninstalling one face says how many faces go with it.
5. Catalog → search → a family → Install a face → Back: the family shows "installed" and the
   Installed tab lists it. "Show more" adds 200 rows.
6. Copy a font into `~/.local/share/fonts` from a terminal: the list updates within a second.
7. At the smallest size (640 × 400) the header buttons still work, in French and German too.
8. `cosmic-ext-fonts -x` exits with status 2; `cosmic-ext-fonts -- -odd.ttf` opens a file whose
   name starts with a dash.
9. Catalog → a family that isn't installed: "Loading preview…", then the preview; nothing is
   installed. Other faces preview too. Offline: "Preview unavailable" and why.
10. A catalog family you installed has *Uninstall* on each of your faces and *Uninstall family*
    at the top, which asks "Remove N face(s)" / Cancel. System faces have neither. The Installed
    tab's family page has the same button.
11. *View* ▸ *Theme*: System, Light, Dark, the current one ticked. The choice applies at once,
    survives a restart, and System follows the desktop's light/dark switch live.
12. *View* ▸ *About*: a side drawer with the icon, the name and the version, Creator and
    Maintainer (Hugo Fortin), Link (*Repository*) and License (*GPL 3.0 or later*). Each link
    opens in the browser. *Close*, Esc or *About* again closes it.
13. The waterfall lines sit close together, the scroll bars are at the window's right edge, and
    the "System" tag is readable in both themes.
14. `env -u WAYLAND_DISPLAY cosmic-ext-fonts` says it needs a Wayland session and exits with
    status 1.

Without a desktop, run it in a headless sway (`WLR_BACKENDS=headless WLR_RENDERER=pixman sway`)
with `grim` for screenshots.

## Checked by hand, and how

### Open file…

A headless compositor has no file chooser, so a stand-in portal answered (a private session bus
and a short Python script with `dbus-next`).

- The app asks for `Open font`, `modal`, a *Fonts* filter that matches `.TTF` too, and Downloads.
- A chosen font opens its preview. Closing the chooser changes nothing; three clicks ask once.
- No portal or no session bus: a dialog gives the reason, and the next click asks again.
- Not tried: the real `xdg-desktop-portal-cosmic`, which needs a COSMIC session.

### Wayland and the portal

- In a headless sway the window is an `xdg_shell` window with the app id
  `io.github.linuxboy_96.cosmic-ext-fonts`, drawn on the CPU. The binary loads no Vulkan, EGL or
  wgpu.
- Without `WAYLAND_DISPLAY` (only `DISPLAY`, or nothing), it says it needs a Wayland session and
  exits with status 1. `--help` and `--version` still work.
- The About page's links reach the stand-in portal as `OpenURI` calls for the repository and the
  GPL 3 page. With no portal, the app prints why and keeps running.
- Menu and About page, in all seven languages: the theme applies and is saved, the About lines fit
  on one row, and *Close* and Esc close the drawer.
- Not tried: COSMIC itself, and a real browser opened by the real portal.

### justfile and store page

- In a scratch copy, `just --set rootdir <dir> install` puts the program and the entry, metainfo
  and icon (under the app id) in `usr`; `just flatpak` does the same in `app`; `just uninstall`
  removes them.
- `just vendor` then `just build-vendored` build offline. Unlike calculator's, the vendor recipes
  leave `.cargo/config.toml` alone, since it holds the cross targets.
- flatpak-cargo-generator, run again, gives `cargo-sources.json` byte for byte.
- `appstreamcli validate --no-net --pedantic res/metainfo.xml` passes without a remark.
- The screenshots show the window at its opening size (900 × 650), in English, with the COSMIC
  icons and Open Sans, on the family page of Lora; once light, once dark.

### Dock, .deb and Flatpak

- COSMIC's dock (`cosmic-app-list`) finds a window's entry with
  `freedesktop_desktop_entry::find_app_by_id` and its icon with `cosmic-freedesktop-icons`. The
  same calls, on the files that `debian/rules` and the Flatpak manifest install, find the entry,
  the translated name and the icon in all seven languages. The icon needs hicolor's `index.theme`,
  hence the dependency on `hicolor-icon-theme`.
- The `.deb` (`dpkg-buildpackage -b -us -uc -d -nc`, offline from the cargo cache;
  `override_dh_auto_test` runs the 41 core tests) holds the program and the three files under the
  app id. Once installed with `dpkg -i`, the same lookup works with the system's
  `XDG_DATA_DIRS`, `appstreamcli get io.github.linuxboy_96.cosmic-ext-fonts` knows the app, and
  `dpkg --purge` leaves nothing. `lintian` only reports `desktop-entry-invalid-category COSMIC`
  (COSMIC's own apps get it too) and `no-manual-page`.
- Not tried: a real COSMIC dock, a real `flatpak-builder` build (Flathub is out of reach here) and
  the `debian/vendor` tree.

## Test filters

`fonts-core/src/lib.rs`, on real fonts (DejaVu):

| Filter | Checks |
|---|---|
| `probe_reads_internal_family_name_not_file_name` | `DejaVuSans.ttf` is "DejaVu Sans", Normal, not monospaced, one face |
| `inventory_groups_faces_by_family` | a scan groups at least 4 DejaVu Sans faces; families sorted ignoring case |
| `probe_and_inventory_describe_every_system_face_the_same_way` | same family, weight, style and width from the probe and the inventory |
| `duplicate_detection_is_by_face_identity_not_file_name` | an installed face gives `AlreadyInstalled` and nothing is written |
| `install_writes_verifies_and_shows_up_as_user_origin` | install copies, re-parses and lists the font as the user's; `uninstall` removes it and refuses a system path |
| `name_collision_with_different_font_gets_fresh_name` | another font with the same file name is kept; the install goes to `DejaVuSans-1.ttf` |
| `watcher_sees_installs` | a font copied into the user directory is reported within a second |
| `read_font_file_returns_the_bytes_of_a_font` | the renderer gets the file's bytes; `/dev/null` is refused |
| `scanning_the_same_files_twice_lists_each_face_once` | the same directory twice, or a parent and its child, list each face once |

`fonts-core/src/lib.rs`, on fonts built in memory:

| Filter | Checks |
|---|---|
| `english_name_wins_even_when_it_is_not_the_first_record` | the English name wins over Chinese and German ones listed first |
| `typographic_family_is_preferred_over_the_legacy_one` | name id 16 ("Foo") beats name id 1 ("Foo Condensed") |
| `macintosh_only_names_are_understood` | a font with only a MacRoman name gets a family |
| `oblique_fonts_are_not_reported_as_italic` | an oblique face is `Oblique`, so it is recognised as installed |
| `slanted_post_angle_alone_means_italic_and_fixed_pitch_means_monospaced` | a `post` italic angle means italic; `isFixedPitch` means monospaced |
| `a_collection_is_probed_face_by_face` | a two-face `.ttc` gives two faces |
| `a_collection_is_installed_when_one_of_its_faces_is_missing` | one face missing installs the whole `.ttc`; `uninstall` removes the whole file |
| `broken_or_lying_collections_are_refused` | a junk second face, or a header claiming 4 billion faces, is refused at once |
| `non_fonts_are_refused_before_anything_is_copied` | empty, text, WOFF and truncated files, a directory, a missing file: errors, nothing written |
| `huge_files_are_refused_without_reading_them` | a sparse file over 256 MiB is `TooLarge` |
| `installed_files_always_get_an_extension_the_scanner_lists` | a font named `download.bin` installs as `download.ttf` |
| `the_user_font_directory_is_listed_once_by_the_system_scan` | a font in the user directory is listed once, as the user's |
| `files_to_register_reports_each_new_file_once` | each new file goes to the renderer once (a `.ttc` counts once) |
| `face_keys_ignore_ascii_case_only` | "DejaVu Sans" = "dejavu sans"; weight and family differences count |
| `weight_classes_match_their_names` | 100 is Thin … 900 and 1000 are Black; 600 is "Semi Bold" |
| `user_font_dir_follows_xdg_but_ignores_relative_values` | an absolute `XDG_DATA_HOME` is used; a relative or empty one gives `~/.local/share/fonts` |

`fonts-core/src/installer.rs`:

| Filter | Checks |
|---|---|
| `headers_are_classified_by_magic_number` | TrueType, OpenType and collections are told apart; bad face counts, truncated, WOFF and empty input are refused |
| `destination_keeps_scanned_extensions_and_fixes_the_others` | `.ttf` and `.otf` are kept in any case, others get the right one; taken names get `-1`, `-2` |

`fonts-core/src/catalog.rs` (offline):

| Filter | Checks |
|---|---|
| `parses_catalog_list_fixture` | the gwfh family list parses |
| `parses_variants_and_maps_weight_style` | variants map to weights, styles and labels ("Bold Italic", "Thin") |
| `cache_write_is_atomic_and_fresh` | the cache is written through a temporary file and is fresh afterwards |
| `fresh_cache_is_served_without_network` | a fresh cache is used without touching the network |
| `network_result_is_cached_only_when_it_parses` | an HTML page, `{}`, `[]` or an empty body is never cached |
| `stale_cache_beats_no_catalog` | an old cache is used offline or after a garbage reply, and survives it |
| `font_ids_are_plain_words` | ids are 1–100 characters of `[a-z0-9-]`; `../etc` fails before any request |
| `downloads_are_https_on_known_hosts_only` | only `fonts.gstatic.com` and `gwfh.mranftl.com` (http upgraded); look-alike hosts, user info, ports, IPs and other schemes are refused |
| `agent_refuses_plain_http` | `http://` fails before connecting, redirects included |
| `https_works_with_every_suite_group_and_certificate_type` | TLS 1.3 and 1.2 with every suite, X25519, P-256 and P-384, ECDSA and RSA chains, against a local server |
| `a_forged_signature_in_the_chain_is_refused` | one flipped bit in the leaf is `BadSignature`, for each root type |
| `an_unknown_issuer_and_a_wrong_name_are_refused` | Mozilla's roots refuse the test root; a `localhost` certificate isn't valid for `127.0.0.1` |
| `the_tls_stack_stays_pure_rust` | ureq stays on `rustls-no-provider` with default features off (its `rustls` feature brings `ring`) |
| `temp_file_names_cannot_escape_the_directory` | `../../etc/passwd` becomes `------etc-passwd` |

`src/main.rs` (command line):

| Filter | Checks |
|---|---|
| `no_argument_starts_the_browser` | no argument opens the browser |
| `one_argument_is_the_font_file` | one argument is the font file |
| `help_and_version_win_over_a_file` | `-h`, `--help`, `-V`, `--version` anywhere |
| `unknown_options_are_refused_instead_of_opened_as_files` | `--frobnicate` and `-x` are errors |
| `double_dash_lets_a_file_name_start_with_a_dash` | after `--`, everything is a file name |
| `a_second_file_is_refused` | two files are an error |
| `file_names_that_are_not_utf8_survive` | a non-UTF-8 name arrives byte for byte |

`src/open_dialog.rs` (file chooser):

| Filter | Checks |
|---|---|
| `the_dialog_shows_font_files_whatever_the_case_of_their_extension` | the *Fonts* filter matches `FONT.TTF` too |
| `the_dialog_starts_in_downloads_else_at_home` | the desktop's Downloads folder, else `~/Downloads`, else home |
| `the_file_is_the_first_address_and_a_file_of_this_computer` | the first `file://` address becomes a path; `https://` or nothing is an error |
| `closing_the_dialog_is_not_a_failure` | "cancelled" isn't an error; any other answer, or none, is |
| `an_error_is_told_with_its_causes` | an error and its causes on one line |

`src/session.rs`:

| Filter | Checks |
|---|---|
| `the_window_needs_a_wayland_display` | `WAYLAND_DISPLAY` or `WAYLAND_SOCKET` is needed; `DISPLAY` alone isn't enough |

`src/packaging.rs` (what desktops read):

| Filter | Checks |
|---|---|
| `a_wayland_window_finds_its_entry_by_the_application_id` | the app id is the same in the code, the justfile, `debian/rules`, the Flatpak manifest, the desktop entry's `Icon` and the metainfo, and every way of installing puts the entry, metainfo and icon under it |
| `the_entry_runs_the_program_every_way_installs` | `Exec`, the justfile, `debian/rules` and the manifest all run the same program |
| `the_screenshots_of_the_store_are_files_of_the_repository` | the metainfo's screenshots are PNG files of `res/screenshots` on the `main` branch of the repository, window-sized; the light one is the default |
| `the_id_names_the_repository` | the app id is `io.github.<owner>.<repository>` of `Cargo.toml`, as Flathub expects |
| `nothing_asks_for_x11_a_gpu_or_xdg_utils` | no X11, wgpu, GPU or `xdg-utils` in `Cargo.toml`, the manifest or `debian/control`, and no `StartupWMClass` |

`src/localize.rs` (translations):

| Filter | Checks |
|---|---|
| `every_language_has_the_messages_and_placeholders_of_english` | every `.ftl` has the English messages and variables |
| `the_about_page_has_its_words_in_every_language` | the About page's six words in each language (French: `Créateur`, `Mainteneur`, `Lien`, `Dépôt`, `Licence`, `GPL 3.0 ou ultérieure`) |
| `the_menu_has_its_words_in_every_language` | View, About and Theme use Scan's words (`Affichage`, `À propos`, `Thème`) |
| `the_message_reader_sees_variables_in_selector_branches` | the `.ftl` reader sees variables inside plural branches and skips comments |
| `every_language_previews_its_whole_alphabet_and_its_accents` | each pangram has every letter of its alphabet and its accented letters |
| `the_title_is_the_comment_of_the_desktop_entry` | the title is the entry's `Comment`, the line the launcher shows under the name (case aside) |
| `the_window_is_named_by_the_name_of_the_desktop_entry` | the window name is the entry's `Name` |
| `the_desktop_entry_and_the_metainfo_are_complete_and_agree_in_every_language` | name, summary, keywords, description, release notes and captions in every language, the same in the entry and the metainfo |

`src/about.rs` (About page and licence):

| Filter | Checks |
|---|---|
| `the_page_names_the_creator_the_maintainer_the_repository_and_the_licence` | Creator, Maintainer, Link and License, in that order, with their addresses |
| `the_licence_of_the_page_is_the_one_of_the_package` | `Cargo.toml` says `GPL-3.0-or-later` and the link goes to the GPL 3 |
| `the_page_and_its_drawer_are_built_and_the_links_open_their_address` | the page builds and its links open the repository and the licence |
| `every_file_that_names_the_licence_names_the_one_of_the_page` | the same licence in both `Cargo.toml`, the metainfo, `debian/copyright`, `LICENSE` and the README |
| `every_source_file_carries_the_licence_of_the_page` | every file of `src/` and the icon start with the SPDX line |
| `the_creator_and_the_maintainer_are_named_in_full_wherever_they_are_named` | Hugo Fortin in `debian/control`, `debian/changelog`, `debian/copyright` and the metainfo |
| `the_link_of_the_page_is_the_repository_the_package_names` | one repository address in `Cargo.toml`, the metainfo, `debian/control` and `debian/copyright` |

`src/app.rs` (what the window shows):

| Filter | Checks |
|---|---|
| `a_face_is_named_by_what_sets_it_apart` | "Regular", "Bold Italic", "Light Condensed"… |
| `english_labels_match_the_core_names` | the English weight and width names are the core's |
| `the_renderer_gets_the_weight_the_label_names` | numeric weights map to the right iced weight |
| `catalog_categories_are_translated_and_unknown_ones_kept` | known categories are translated; others shown as is |
| `the_installed_line_says_what_its_number_counts` | "269 families on this computer · 12 user-installed", singulars, search counts |
| `the_catalog_line_says_where_its_families_come_from` | "1959 families on Google Fonts · 37 already installed", search counts |
| `show_more_says_how_far_the_list_goes` | "Show more (200 of 1959)" |
| `the_switcher_has_installed_then_catalog_and_starts_on_installed` | two tabs, *Installed* first and selected |
| `a_catalog_family_opens_on_its_regular_variant` | the regular face, else the first one |
| `selecting_a_variant_previews_it_without_installing_it` | one download, "loading", then the preview; nothing installed; no second download |
| `an_installed_variant_previews_from_the_installed_font` | an installed face shows at once, without a download |
| `a_late_answer_for_a_variant_the_user_left_is_kept_but_not_shown` | a late download is kept for later but not shown |
| `a_failed_preview_says_so_and_the_next_try_downloads_again` | the error is shown, not remembered, and the next try downloads again |
| `opening_a_catalog_family_previews_its_regular_variant_at_once` | opening a family fetches its regular face; replies for another family are ignored |
| `family_names_differing_by_case_are_one_family` | "DejaVu Sans" = "dejavu sans", but not "DejaVu Sans Mono" |
| `removing_a_family_concerns_only_the_files_the_user_installed` | only the user's files, each once; no button for system families |
| `removing_a_family_waits_for_its_confirmation` | the first click asks; Cancel and stray clicks do nothing; Confirm removes |
| `a_question_about_something_already_gone_is_dropped` | the question disappears when another program removed the files |
| `the_theme_choice_is_a_word_and_anything_else_is_the_default` | `system`, `light`, `dark`; anything else is System |
| `a_pinned_theme_keeps_its_mode_whatever_the_desktop_answers` | Light or Dark stays so whatever the desktop's theme |
| `the_settings_file_keeps_the_theme_choice` | the choice applies at once and is read back at the next start |
| `the_file_chooser_is_asked_once_and_its_failure_is_told` | one chooser at a time; a failure shows a dialog; a chosen font opens |
| `the_menu_bar_holds_the_theme_and_the_about_page` | View holds Theme, a divider and About; nothing at the end of the header |
| `the_menu_ticks_the_theme_in_force_and_only_that_one` | only the current theme is ticked |
| `the_entries_of_the_menu_send_the_messages_they_stand_for` | each menu entry does what it says |
| `the_about_entry_opens_and_closes_the_about_drawer` | About and Esc open and close the drawer |
| `every_screen_can_be_built_in_every_state` | every view builds in every state without panicking |

## Test certificates

`fonts-core/testdata/tls/` holds three test roots (ECDSA P-256, ECDSA P-384, RSA-4096) and four
`localhost` leaves with their keys, valid from 2020 to 2120. The root keys were not kept. To make
more, with OpenSSL 3:

```sh
# ca.cnf: [ca] default_ca = d / [d] dir = ., database = index.txt, new_certs_dir = issued,
#         serial = serial, policy = p, unique_subject = no / [p] commonName = supplied
# root.ext: basicConstraints=critical,CA:TRUE  keyUsage=critical,keyCertSign,cRLSign
# leaf.ext: basicConstraints=critical,CA:FALSE keyUsage=critical,digitalSignature
#           extendedKeyUsage=serverAuth subjectAltName=DNS:localhost
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out root.key
openssl req -new -key root.key -subj "/CN=test root" -out root.csr
openssl ca -batch -config ca.cnf -selfsign -in root.csr -keyfile root.key -md sha256 \
  -startdate 20200101000000Z -enddate 21200101000000Z -extfile root.ext -notext -out root.pem
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out leaf.key
openssl req -new -key leaf.key -subj "/CN=localhost" -out leaf.csr
openssl ca -batch -config ca.cnf -in leaf.csr -cert root.pem -keyfile root.key -md sha256 \
  -startdate 20200101000000Z -enddate 21200101000000Z -extfile leaf.ext -notext -out leaf.pem
```

## Environment

Tests that touch `XDG_DATA_HOME` take a shared lock, so the parallel runner is fine. GUI labels
are in English whatever the locale. The libcosmic `Core` the GUI tests build may create empty theme
folders under `~/.config/cosmic/`, but no test writes a setting there.
