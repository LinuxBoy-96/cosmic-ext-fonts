//! The non-GUI part of cosmic-ext-fonts: listing fonts ([`inventory`]), installing and removing
//! them ([`installer`]), the Google Fonts catalog ([`catalog`]) and watching the font folder
//! ([`watcher`]). No daemon, D-Bus or Polkit.

pub mod catalog;
pub mod installer;
pub mod inventory;
pub mod watcher;

pub use fontdb;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use crate::installer::{self, InstallError, InstallOutcome};
    use crate::inventory::{self, Inventory, Origin};

    /// Where distros put `DejaVu` (Fedora splits Sans and Serif). The tests need both.
    const DEJAVU_CANDIDATES: &[&str] = &[
        "/usr/share/fonts/truetype/dejavu",    // Debian, Ubuntu, Pop!_OS
        "/usr/share/fonts/dejavu-sans-fonts",  // Fedora (Sans)
        "/usr/share/fonts/dejavu-serif-fonts", // Fedora (Serif)
        "/usr/share/fonts/dejavu",             // openSUSE, Void
        "/usr/share/fonts/TTF",                // Arch
    ];

    /// First candidate directory containing `file`.
    fn dejavu(file: &str) -> PathBuf {
        DEJAVU_CANDIDATES
            .iter()
            .map(|d| Path::new(d).join(file))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{file} not found; looked in {DEJAVU_CANDIDATES:?}"))
    }

    fn dejavu_regular() -> PathBuf {
        dejavu("DejaVuSans.ttf")
    }

    /// Directory holding the `DejaVu Sans` faces (for whole-dir scans).
    fn dejavu_sans_dir() -> PathBuf {
        dejavu_regular().parent().unwrap().to_path_buf()
    }

    // Tests below read or mutate XDG_DATA_HOME (process-global); serialize them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Also for tests that only read the environment (anything building an `Inventory`).
    fn env_lock() -> MutexGuard<'static, ()> {
        // A panicking test must not poison every test after it.
        ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn with_temp_user_dir<F: FnOnce(&Path)>(f: F) {
        let _guard = env_lock();
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: every test touching the environment holds ENV_LOCK.
        unsafe { std::env::set_var("XDG_DATA_HOME", tmp.path()) };
        f(&tmp.path().join("fonts"));
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
    }

    /// Hand-made fonts (only the tables `fontdb` reads: `name`, `OS/2`, `post`) for edge cases.
    mod synth {
        pub struct NameRecord {
            platform: u16,
            encoding: u16,
            language: u16,
            id: u16,
            text: Vec<u8>,
        }

        /// A Windows (platform 3) record, UTF-16BE; `language` is a Windows LCID.
        pub fn win(language: u16, id: u16, text: &str) -> NameRecord {
            NameRecord {
                platform: 3,
                encoding: 1,
                language,
                id,
                text: text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
            }
        }

        /// A Macintosh Roman record (platform 1), one byte per ASCII character.
        pub fn mac(id: u16, text: &str) -> NameRecord {
            NameRecord {
                platform: 1,
                encoding: 0,
                language: 0,
                id,
                text: text.as_bytes().to_vec(),
            }
        }

        pub const EN_US: u16 = 0x0409;
        pub const DE: u16 = 0x0407;
        pub const ZH_TW: u16 = 0x0404;

        /// `name` table with the records given.
        pub fn name_table(records: &[NameRecord]) -> Vec<u8> {
            let count = u16::try_from(records.len()).unwrap();
            let storage_offset = 6 + 12 * count;
            let mut table = Vec::new();
            for v in [0, count, storage_offset] {
                table.extend_from_slice(&u16::to_be_bytes(v));
            }
            let mut storage = Vec::new();
            for r in records {
                let length = u16::try_from(r.text.len()).unwrap();
                let offset = u16::try_from(storage.len()).unwrap();
                for v in [r.platform, r.encoding, r.language, r.id, length, offset] {
                    table.extend_from_slice(&u16::to_be_bytes(v));
                }
                storage.extend_from_slice(&r.text);
            }
            table.extend_from_slice(&storage);
            table
        }

        /// `OS/2` version 4, 96 bytes: only weight, width and `fsSelection` set.
        pub fn os2(weight: u16, width: u16, fs_selection: u16) -> Vec<u8> {
            let mut t = vec![0_u8; 96];
            t[0..2].copy_from_slice(&4_u16.to_be_bytes());
            t[4..6].copy_from_slice(&weight.to_be_bytes());
            t[6..8].copy_from_slice(&width.to_be_bytes());
            t[62..64].copy_from_slice(&fs_selection.to_be_bytes());
            t
        }

        /// `post` version 3: italic angle in degrees, and the fixed-pitch flag.
        pub fn post(italic_angle: i16, fixed_pitch: bool) -> Vec<u8> {
            let mut t = vec![0_u8; 32];
            t[0..4].copy_from_slice(&0x0003_0000_u32.to_be_bytes());
            t[4..8].copy_from_slice(&(i32::from(italic_angle) << 16).to_be_bytes());
            t[12..16].copy_from_slice(&u32::from(fixed_pitch).to_be_bytes());
            t
        }

        pub type Tables = Vec<([u8; 4], Vec<u8>)>;

        /// One sfnt that will sit `base` bytes into its file.
        fn sfnt(tables: &Tables, base: usize) -> Vec<u8> {
            let count = tables.len();
            let mut out = Vec::new();
            out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
            out.extend_from_slice(&u16::try_from(count).unwrap().to_be_bytes());
            out.extend_from_slice(&[0; 6]); // searchRange, entrySelector, rangeShift
            let mut offset = base + 12 + 16 * count;
            let mut data = Vec::new();
            for (tag, table) in tables {
                out.extend_from_slice(tag);
                out.extend_from_slice(&[0; 4]); // checksum: not verified
                out.extend_from_slice(&u32::try_from(offset).unwrap().to_be_bytes());
                out.extend_from_slice(&u32::try_from(table.len()).unwrap().to_be_bytes());
                data.extend_from_slice(table);
                while data.len() % 4 != 0 {
                    data.push(0);
                }
                offset = base + 12 + 16 * count + data.len();
            }
            out.extend_from_slice(&data);
            out
        }

        pub fn font(tables: &Tables) -> Vec<u8> {
            sfnt(tables, 0)
        }

        /// A `.ttc` holding these faces.
        pub fn collection(faces: &[Tables]) -> Vec<u8> {
            let header = 12 + 4 * faces.len();
            let mut offsets = Vec::new();
            let mut body = Vec::new();
            for tables in faces {
                offsets.push(header + body.len());
                body.extend_from_slice(&sfnt(tables, header + body.len()));
            }
            let mut out = b"ttcf".to_vec();
            out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
            out.extend_from_slice(&u32::try_from(faces.len()).unwrap().to_be_bytes());
            for offset in offsets {
                out.extend_from_slice(&u32::try_from(offset).unwrap().to_be_bytes());
            }
            out.extend_from_slice(&body);
            out
        }

        /// A plain regular face named `family` (English), nothing else.
        pub fn simple(family: &str) -> Tables {
            vec![
                (*b"OS/2", os2(400, 5, 0x40)),
                (
                    *b"name",
                    name_table(&[
                        win(EN_US, 1, family),
                        win(EN_US, 6, &format!("{}-Regular", family.replace(' ', ""))),
                    ]),
                ),
                (*b"post", post(0, false)),
            ]
        }
    }

    fn write_font(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    // ------------------------------------------------------------- real fonts

    #[test]
    fn probe_reads_internal_family_name_not_file_name() {
        // The file is `DejaVuSans.ttf` (no space); the family is "DejaVu Sans".
        let info = installer::probe(&dejavu_regular()).unwrap();
        assert_eq!(info.family, "DejaVu Sans");
        assert_eq!(info.style, fontdb::Style::Normal);
        assert!(!info.monospaced);
        assert_eq!(info.face_count, 1);
        assert_eq!(installer::probe_faces(&dejavu_regular()).unwrap().len(), 1);
    }

    #[test]
    fn inventory_groups_faces_by_family() {
        let _guard = env_lock();
        let dir = dejavu_sans_dir();
        let inv = Inventory::load_dirs(&[&dir]);
        assert!(
            inv.face_count() > 0,
            "no faces loaded from {}",
            dir.display()
        );

        let families = inv.families();
        let sans = families
            .iter()
            .find(|f| f.name == "DejaVu Sans")
            .expect("DejaVu Sans family present");
        // Regular, Bold, Oblique, BoldOblique, ExtraLight, Condensed variants…
        assert!(sans.faces.len() >= 4, "expected several DejaVu Sans faces");

        // Families are sorted case-insensitively.
        let names: Vec<_> = families.iter().map(|f| f.name.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn probe_and_inventory_describe_every_system_face_the_same_way() {
        let _guard = env_lock();
        let dir = dejavu_sans_dir();
        let inv = Inventory::load_dirs(&[&dir]);
        let in_inventory: HashSet<_> = inv.faces().iter().map(inventory::FaceEntry::key).collect();

        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "ttf") {
                for face in installer::probe_faces(&path).unwrap() {
                    assert!(in_inventory.contains(&face.key()), "{}", path.display());
                    checked += 1;
                }
            }
        }
        assert!(checked >= 4, "only {checked} faces compared");
    }

    #[test]
    fn duplicate_detection_is_by_face_identity_not_file_name() {
        let inv = {
            let _guard = env_lock();
            Inventory::load_dirs(&[dejavu_sans_dir()])
        };

        // Same family+face already in inventory => AlreadyInstalled, no copy.
        with_temp_user_dir(|user_fonts| {
            let outcome = installer::install(&dejavu_regular(), &inv).unwrap();
            assert_eq!(
                outcome,
                InstallOutcome::AlreadyInstalled {
                    family: "DejaVu Sans".into()
                }
            );
            assert!(
                !user_fonts.exists() || user_fonts.read_dir().unwrap().next().is_none(),
                "no file should have been written for a duplicate"
            );
        });
    }

    #[test]
    fn install_writes_verifies_and_shows_up_as_user_origin() {
        // Empty inventory => the font is "new" and must be written.
        let empty = {
            let _guard = env_lock();
            Inventory::load_dirs::<&str>(&[])
        };

        with_temp_user_dir(|_user_fonts| {
            let outcome = installer::install(&dejavu_regular(), &empty).unwrap();
            let dest = match outcome {
                InstallOutcome::Installed(p) => p,
                other @ InstallOutcome::AlreadyInstalled { .. } => {
                    panic!("expected Installed, got {other:?}")
                }
            };
            assert!(dest.exists(), "installed file must exist on disk");

            // Rescan: the new face must be visible, flagged as user-installed.
            let inv = Inventory::load_dirs(&[dest.parent().unwrap()]);
            assert!(inv.contains_family("DejaVu Sans"));
            let families = inv.families();
            let fam = families.iter().find(|f| f.name == "DejaVu Sans").unwrap();
            assert!(fam.faces.iter().all(|f| f.origin == Origin::User));
            assert!(fam.has_user_faces());

            // Uninstall must succeed inside the user dir…
            installer::uninstall(&dest).unwrap();
            assert!(!dest.exists());
        });

        // …and refuse outside of it (system font path, default user dir).
        let _guard = env_lock();
        let err = installer::uninstall(&dejavu_regular()).unwrap_err();
        assert!(matches!(err, installer::InstallError::NotUserFont));
    }

    #[test]
    fn name_collision_with_different_font_gets_fresh_name() {
        let empty = {
            let _guard = env_lock();
            Inventory::load_dirs::<&str>(&[])
        };

        with_temp_user_dir(|user_fonts| {
            std::fs::create_dir_all(user_fonts).unwrap();
            // Plant a *different* font under the same file name.
            std::fs::copy(dejavu("DejaVuSerif.ttf"), user_fonts.join("DejaVuSans.ttf")).unwrap();

            let outcome = installer::install(&dejavu_regular(), &empty).unwrap();
            let dest = match outcome {
                InstallOutcome::Installed(p) => p,
                other @ InstallOutcome::AlreadyInstalled { .. } => {
                    panic!("expected Installed, got {other:?}")
                }
            };
            assert_eq!(
                dest.file_name().unwrap().to_str().unwrap(),
                "DejaVuSans-1.ttf",
                "collision must produce a fresh name, not overwrite"
            );
            // The planted impostor is untouched.
            let impostor = installer::probe(&user_fonts.join("DejaVuSans.ttf")).unwrap();
            assert_eq!(impostor.family, "DejaVu Serif");
        });
    }

    #[test]
    fn watcher_sees_installs() {
        with_temp_user_dir(|user_fonts| {
            let watcher = crate::watcher::FontDirWatcher::start().unwrap();
            assert_eq!(watcher.dir, *user_fonts);
            assert!(!watcher.poll(), "no events yet");

            std::fs::create_dir_all(user_fonts).unwrap();
            std::fs::copy(dejavu_regular(), user_fonts.join("W.ttf")).unwrap();

            // inotify delivery is asynchronous; give it a moment.
            let mut seen = false;
            for _ in 0..50 {
                if watcher.poll() {
                    seen = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert!(seen, "watcher must report the new file");
        });
    }

    // -------------------------------------------------- family names and styles

    #[test]
    fn english_name_wins_even_when_it_is_not_the_first_record() {
        // Traditional Chinese (0x404) and German (0x407) sort before English (0x409).
        let tables = vec![(
            *b"name",
            synth::name_table(&[
                synth::win(synth::ZH_TW, 1, "測試字體"),
                synth::win(synth::DE, 1, "Testschrift"),
                synth::win(synth::EN_US, 1, "Test Family"),
                synth::win(synth::EN_US, 6, "TestFamily-Regular"),
            ]),
        )];
        let dir = tempfile::tempdir().unwrap();
        let path = write_font(dir.path(), "Test.ttf", &synth::font(&tables));

        let probed = installer::probe(&path).unwrap();
        assert_eq!(probed.family, "Test Family");

        // The inventory shows the same name, and finds the face by any of its names.
        let _guard = env_lock();
        let inv = Inventory::load_dirs(&[dir.path()]);
        assert_eq!(inv.families()[0].name, "Test Family");
        for name in ["Test Family", "Testschrift", "測試字體"] {
            assert!(inv.contains_family(name), "{name}");
        }
    }

    #[test]
    fn typographic_family_is_preferred_over_the_legacy_one() {
        let tables = vec![(
            *b"name",
            synth::name_table(&[
                synth::win(synth::EN_US, 1, "Foo Condensed"),
                synth::win(synth::EN_US, 6, "Foo-Condensed"),
                synth::win(synth::EN_US, 16, "Foo"),
            ]),
        )];
        let dir = tempfile::tempdir().unwrap();
        let path = write_font(dir.path(), "Foo.ttf", &synth::font(&tables));
        assert_eq!(installer::probe(&path).unwrap().family, "Foo");
    }

    #[test]
    fn macintosh_only_names_are_understood() {
        // Old fonts carry only a MacRoman name; the inventory lists them, so the probe must too.
        let tables = vec![(
            *b"name",
            synth::name_table(&[synth::mac(1, "Mac Only"), synth::mac(6, "MacOnly-Regular")]),
        )];
        let dir = tempfile::tempdir().unwrap();
        let path = write_font(dir.path(), "Mac.ttf", &synth::font(&tables));

        assert_eq!(installer::probe(&path).unwrap().family, "Mac Only");
        let _guard = env_lock();
        let inv = Inventory::load_dirs(&[dir.path()]);
        assert_eq!(inv.families()[0].name, "Mac Only");
    }

    #[test]
    fn oblique_fonts_are_not_reported_as_italic() {
        // OS/2 v4 "oblique" bit + a slanted `post` angle: fontdb says Oblique.
        let mut tables = synth::simple("Slanty");
        tables[0] = (*b"OS/2", synth::os2(400, 5, 0x0200));
        tables[2] = (*b"post", synth::post(-12, false));
        let dir = tempfile::tempdir().unwrap();
        let path = write_font(dir.path(), "Slanty.ttf", &synth::font(&tables));

        assert_eq!(
            installer::probe(&path).unwrap().style,
            fontdb::Style::Oblique
        );

        let inv = {
            let _guard = env_lock();
            Inventory::load_dirs(&[dir.path()])
        };
        assert_eq!(inv.faces()[0].style, fontdb::Style::Oblique);
        // So the font is recognised as installed, not offered (and written) again.
        with_temp_user_dir(|user_fonts| {
            assert_eq!(
                installer::install(&path, &inv).unwrap(),
                InstallOutcome::AlreadyInstalled {
                    family: "Slanty".into()
                }
            );
            assert!(!user_fonts.exists());
        });
    }

    #[test]
    fn slanted_post_angle_alone_means_italic_and_fixed_pitch_means_monospaced() {
        let mut tables = synth::simple("Leaning Mono");
        tables[2] = (*b"post", synth::post(-9, true));
        let dir = tempfile::tempdir().unwrap();
        let path = write_font(dir.path(), "Lean.ttf", &synth::font(&tables));

        let probed = installer::probe(&path).unwrap();
        assert_eq!(probed.style, fontdb::Style::Italic);
        assert!(probed.monospaced);
    }

    // ------------------------------------------------------------- collections

    fn two_face_collection() -> Vec<u8> {
        synth::collection(&[synth::simple("Duo One"), synth::simple("Duo Two")])
    }

    #[test]
    fn a_collection_is_probed_face_by_face() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_font(dir.path(), "Duo.ttc", &two_face_collection());

        let faces = installer::probe_faces(&path).unwrap();
        let names: Vec<_> = faces.iter().map(|f| f.family.as_str()).collect();
        assert_eq!(names, ["Duo One", "Duo Two"]);
        assert!(faces.iter().all(|f| f.face_count == 2));
        assert_eq!(installer::probe(&path).unwrap().family, "Duo One");

        let _guard = env_lock();
        let inv = Inventory::load_dirs(&[dir.path()]);
        assert_eq!(inv.face_count(), 2);
        assert_eq!(inv.faces_in_file(&path), 2);
        assert_eq!(inv.faces_in_file(&dir.path().join("other.ttc")), 0);
    }

    #[test]
    fn a_collection_is_installed_when_one_of_its_faces_is_missing() {
        let src = tempfile::tempdir().unwrap();
        let path = write_font(src.path(), "Duo.ttc", &two_face_collection());
        let holder = tempfile::tempdir().unwrap();
        write_font(
            holder.path(),
            "One.ttf",
            &synth::font(&synth::simple("Duo One")),
        );
        let only_one = {
            let _guard = env_lock();
            Inventory::load_dirs(&[holder.path()])
        };

        with_temp_user_dir(|_| {
            // Face one exists, face two does not: the collection is installed.
            let outcome = installer::install(&path, &only_one).unwrap();
            let InstallOutcome::Installed(dest) = outcome else {
                panic!("expected Installed, got {outcome:?}");
            };
            assert_eq!(dest.file_name().unwrap(), "Duo.ttc");

            // Removing a collection removes the whole file.
            installer::uninstall(&dest).unwrap();
            assert!(!dest.exists());
        });
    }

    #[test]
    fn broken_or_lying_collections_are_refused() {
        let dir = tempfile::tempdir().unwrap();

        // Second face is not a font: the collection is refused whole.
        let mut broken = two_face_collection();
        let second = u32::from_be_bytes(broken[16..20].try_into().unwrap()) as usize;
        broken[second..second + 4].copy_from_slice(b"junk");
        let path = write_font(dir.path(), "Broken.ttc", &broken);
        assert!(matches!(
            installer::probe_faces(&path),
            Err(InstallError::InvalidFont)
        ));

        // Header claims four billion faces: refused at once, not after four billion tries.
        let mut lying = two_face_collection();
        lying[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
        let path = write_font(dir.path(), "Lying.ttc", &lying);
        let started = std::time::Instant::now();
        assert!(matches!(
            installer::probe_faces(&path),
            Err(InstallError::InvalidFont)
        ));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    // ------------------------------------------------- files that are not fonts

    #[test]
    fn non_fonts_are_refused_before_anything_is_copied() {
        let dir = tempfile::tempdir().unwrap();
        let empty = {
            let _guard = env_lock();
            Inventory::load_dirs::<&str>(&[])
        };

        let cases: [(&str, Vec<u8>); 4] = [
            ("empty.ttf", Vec::new()),
            ("text.ttf", b"hello, I am not a font".to_vec()),
            ("woff.ttf", b"wOFF\0\x01\0\0 more bytes follow".to_vec()),
            ("cut.ttf", vec![0, 1, 0, 0, 0, 3]),
        ];
        with_temp_user_dir(|user_fonts| {
            for (name, bytes) in &cases {
                let path = write_font(dir.path(), name, bytes);
                assert!(
                    matches!(installer::probe(&path), Err(InstallError::InvalidFont)),
                    "{name}"
                );
                assert!(installer::install(&path, &empty).is_err(), "{name}");
            }
            // A directory, and a file that does not exist.
            assert!(matches!(
                installer::probe(dir.path()),
                Err(InstallError::InvalidFont)
            ));
            assert!(matches!(
                installer::probe(&dir.path().join("nope.ttf")),
                Err(InstallError::Io(_))
            ));
            assert!(
                !user_fonts.exists(),
                "nothing may be written for a non-font"
            );
        });
    }

    #[test]
    fn huge_files_are_refused_without_reading_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.ttf");
        // Sparse: costs no disk space.
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(installer::MAX_FONT_FILE_BYTES + 1).unwrap();
        drop(file);

        assert!(matches!(
            installer::probe(&path),
            Err(InstallError::TooLarge)
        ));
        assert!(matches!(
            installer::read_font_file(&path),
            Err(InstallError::TooLarge)
        ));
    }

    #[test]
    fn read_font_file_returns_the_bytes_of_a_font() {
        let bytes = installer::read_font_file(&dejavu_regular()).unwrap();
        assert_eq!(bytes, std::fs::read(dejavu_regular()).unwrap());
        assert!(installer::read_font_file(Path::new("/dev/null")).is_err());
    }

    #[test]
    fn installed_files_always_get_an_extension_the_scanner_lists() {
        let src = tempfile::tempdir().unwrap();
        let path = write_font(
            src.path(),
            "download.bin",
            &synth::font(&synth::simple("Binary Named")),
        );
        let empty = {
            let _guard = env_lock();
            Inventory::load_dirs::<&str>(&[])
        };

        with_temp_user_dir(|user_fonts| {
            let InstallOutcome::Installed(dest) = installer::install(&path, &empty).unwrap() else {
                panic!("expected Installed");
            };
            assert_eq!(dest, user_fonts.join("download.ttf"));
            let inv = Inventory::load_dirs(&[user_fonts]);
            assert!(
                inv.contains_family("Binary Named"),
                "the install must be visible"
            );
        });
    }

    // ---------------------------------------------------------------- inventory

    #[test]
    fn scanning_the_same_files_twice_lists_each_face_once() {
        let _guard = env_lock();
        let dir = dejavu_sans_dir();
        let once = Inventory::load_dirs(&[&dir]);
        let twice = Inventory::load_dirs(&[&dir, &dir]);

        assert_eq!(once.face_count(), twice.face_count());
        assert_eq!(once.faces().len(), twice.faces().len());
        let mut seen = HashSet::new();
        for face in twice.faces() {
            assert!(seen.insert((face.path, face.index)), "face listed twice");
        }

        // Overlapping directories (a parent and its child) are deduplicated too.
        let parent = dir.parent().unwrap();
        let overlapping = Inventory::load_dirs(&[parent, &dir]);
        let direct = Inventory::load_dirs(&[parent]);
        assert_eq!(overlapping.face_count(), direct.face_count());
    }

    #[test]
    fn the_user_font_directory_is_listed_once_by_the_system_scan() {
        with_temp_user_dir(|user_fonts| {
            write_font(
                user_fonts,
                "Mine.ttf",
                &synth::font(&synth::simple("Only Mine")),
            );

            let inv = Inventory::load_system();
            let mine: Vec<_> = inv
                .faces()
                .into_iter()
                .filter(|f| f.path.as_deref().is_some_and(|p| p.starts_with(user_fonts)))
                .collect();
            assert_eq!(mine.len(), 1, "{mine:?}");
            assert_eq!(mine[0].origin, Origin::User);
            assert_eq!(mine[0].family, "Only Mine");
        });
    }

    #[test]
    fn files_to_register_reports_each_new_file_once() {
        let _guard = env_lock();
        let dir = tempfile::tempdir().unwrap();
        let one = write_font(dir.path(), "One.ttf", &synth::font(&synth::simple("One")));
        let mut known = HashSet::new();

        // First inventory: seeds the set (the renderer scanned these itself).
        let first = Inventory::load_dirs(&[dir.path()]).families();
        assert_eq!(
            inventory::files_to_register(&first, &mut known),
            std::slice::from_ref(&one)
        );
        assert!(inventory::files_to_register(&first, &mut known).is_empty());

        // A collection and a new plain font appear: each file once.
        let duo = write_font(dir.path(), "Duo.ttc", &two_face_collection());
        let three = write_font(
            dir.path(),
            "Three.ttf",
            &synth::font(&synth::simple("Three")),
        );
        let second = Inventory::load_dirs(&[dir.path()]).families();
        let mut new = inventory::files_to_register(&second, &mut known);
        new.sort();
        let mut expected = vec![duo, three];
        expected.sort();
        assert_eq!(new, expected);

        // Removed fonts stay known (the renderer keeps them): nothing to send.
        std::fs::remove_file(&one).unwrap();
        let third = Inventory::load_dirs(&[dir.path()]).families();
        assert!(inventory::files_to_register(&third, &mut known).is_empty());
        // …and a removed font that comes back is not sent twice.
        write_font(dir.path(), "One.ttf", &synth::font(&synth::simple("One")));
        let fourth = Inventory::load_dirs(&[dir.path()]).families();
        assert!(inventory::files_to_register(&fourth, &mut known).is_empty());
    }

    #[test]
    fn face_keys_ignore_ascii_case_only() {
        let key = |family: &str, weight| {
            inventory::FaceKey::new(
                family,
                fontdb::Weight(weight),
                fontdb::Style::Normal,
                fontdb::Stretch::Normal,
            )
        };
        assert_eq!(key("DejaVu Sans", 400), key("dejavu sans", 400));
        assert_ne!(key("DejaVu Sans", 400), key("DejaVu Sans", 700));
        assert_ne!(key("DejaVu Sans", 400), key("DejaVu Serif", 400));
    }

    #[test]
    fn weight_classes_match_their_names() {
        use inventory::{WeightClass, weight_class, weight_name};
        let class = |w| weight_class(fontdb::Weight(w));
        assert_eq!(class(100), WeightClass::Thin);
        assert_eq!(class(400), WeightClass::Regular);
        assert_eq!(class(500), WeightClass::Medium);
        assert_eq!(class(700), WeightClass::Bold);
        assert_eq!(class(900), WeightClass::Black);
        assert_eq!(class(1000), WeightClass::Black);
        assert_eq!(weight_name(fontdb::Weight(400)), "");
        assert_eq!(weight_name(fontdb::Weight(600)), "Semi Bold");
    }

    // ------------------------------------------------------------ user directory

    #[test]
    fn user_font_dir_follows_xdg_but_ignores_relative_values() {
        let _guard = env_lock();
        let home_based = dirs::home_dir().map(|h| h.join(".local/share/fonts"));
        // SAFETY: every test touching the environment holds ENV_LOCK.
        unsafe { std::env::set_var("XDG_DATA_HOME", "/somewhere/absolute") };
        let absolute = installer::user_font_dir();
        unsafe { std::env::set_var("XDG_DATA_HOME", "relative/dir") };
        let relative = installer::user_font_dir();
        unsafe { std::env::set_var("XDG_DATA_HOME", "") };
        let empty = installer::user_font_dir();
        unsafe { std::env::remove_var("XDG_DATA_HOME") };

        if !installer::in_flatpak() {
            assert_eq!(absolute, Some(PathBuf::from("/somewhere/absolute/fonts")));
            assert_eq!(relative, home_based);
            assert_eq!(empty, home_based);
        }
    }
}
