//! Installs and removes user fonts. Unlike gnome-font-viewer, we check the copy, spot duplicates
//! by face rather than file name, and never overwrite another font with the same file name.
//! Files are parsed by `fontdb`, like the inventory, so both always agree on names and styles.

use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::inventory::{FaceKey, Inventory, display_family};

/// Largest font file we open. Real fonts top out around 150 MB (full CJK collections); this
/// just stops accidents like opening a disk image.
pub const MAX_FONT_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// More faces than this means a corrupt collection header (real ones have a few dozen).
const MAX_FACES_PER_FILE: u32 = 1024;

/// True inside a Flatpak sandbox.
#[must_use]
pub fn in_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// `$XDG_DATA_HOME/fonts`, else `~/.local/share/fonts`; `None` without a home directory.
///
/// A relative `$XDG_DATA_HOME` is ignored, as the XDG spec says. In a Flatpak it points to the
/// app's private folder, so we use the host's `~/.local/share/fonts` (granted by
/// `--filesystem=xdg-data/fonts`), the only place the rest of the system sees.
#[must_use]
pub fn user_font_dir() -> Option<PathBuf> {
    if !in_flatpak()
        && let Some(xdg) = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    {
        return Some(xdg.join("fonts"));
    }
    dirs::home_dir().map(|home| home.join(".local/share/fonts"))
}

/// Metadata extracted from one face of a font file.
#[derive(Debug, Clone)]
pub struct FontFileInfo {
    pub family: String,
    pub weight: fontdb::Weight,
    pub style: fontdb::Style,
    pub stretch: fontdb::Stretch,
    pub monospaced: bool,
    /// Number of faces in the file (>1 for `.ttc` collections).
    pub face_count: u32,
}

impl FontFileInfo {
    fn from_face(info: &fontdb::FaceInfo, face_count: u32) -> Self {
        Self {
            family: display_family(info),
            weight: info.weight,
            style: info.style,
            stretch: info.stretch,
            monospaced: info.monospaced,
            face_count,
        }
    }

    /// The identity used for duplicate detection.
    #[must_use]
    pub fn key(&self) -> FaceKey {
        FaceKey::new(&self.family, self.weight, self.style, self.stretch)
    }
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("failed to read font file: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a valid font file")]
    InvalidFont,
    #[error("font file is larger than 256 MiB")]
    TooLarge,
    #[error("installed file failed post-install verification")]
    VerificationFailed,
    #[error("refusing to remove a file outside the user font directory")]
    NotUserFont,
    #[error("cannot locate the user font directory (no home directory)")]
    NoUserFontDir,
}

/// What the first bytes of a file say about it.
struct Header {
    /// Faces the file declares (1 for a plain font).
    faces: u32,
    /// Extension that matches the file's format.
    extension: &'static str,
}

/// Reads the sfnt header of `path`, refusing anything but a regular file of sane size.
fn read_header(path: &Path) -> Result<Header, InstallError> {
    let file = fs::File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(InstallError::InvalidFont);
    }
    if meta.len() > MAX_FONT_FILE_BYTES {
        return Err(InstallError::TooLarge);
    }

    let mut head = Vec::with_capacity(12);
    file.take(12).read_to_end(&mut head)?;
    header_from_bytes(&head, meta.len())
}

fn header_from_bytes(head: &[u8], file_len: u64) -> Result<Header, InstallError> {
    match head.get(..4) {
        Some(b"ttcf") => {
            let count = head
                .get(8..12)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_be_bytes)
                .ok_or(InstallError::InvalidFont)?;
            // The offset table must fit in the file, or the header lies.
            let table_end = 12 + 4 * u64::from(count);
            if count == 0 || count > MAX_FACES_PER_FILE || table_end > file_len {
                return Err(InstallError::InvalidFont);
            }
            Ok(Header {
                faces: count,
                extension: "ttc",
            })
        }
        Some(b"OTTO") => Ok(Header {
            faces: 1,
            extension: "otf",
        }),
        Some(b"\0\x01\0\0" | b"true" | b"typ1") => Ok(Header {
            faces: 1,
            extension: "ttf",
        }),
        _ => Err(InstallError::InvalidFont),
    }
}

/// Reads a font file into memory for the renderer, with the same limits as [`probe_faces`].
///
/// # Errors
/// I/O failure, not a font, or over the size cap.
pub fn read_font_file(path: &Path) -> Result<Vec<u8>, InstallError> {
    read_header(path)?;
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(MAX_FONT_FILE_BYTES + 1)
        .read_to_end(&mut data)?;
    if data.len() as u64 > MAX_FONT_FILE_BYTES {
        return Err(InstallError::TooLarge);
    }
    Ok(data)
}

/// The first face of a font file, which is what the UI shows. See [`probe_faces`] for
/// collections.
///
/// # Errors
/// I/O failure, an unparsable font, or over the size cap.
pub fn probe(path: &Path) -> Result<FontFileInfo, InstallError> {
    probe_faces(path)?
        .into_iter()
        .next()
        .ok_or(InstallError::InvalidFont)
}

/// Every face of a font file (several for a `.ttc`). A collection only counts as installed
/// when all its faces are. The file is memory-mapped, so a big collection costs a few pages.
///
/// # Errors
/// I/O failure, over the size cap, or a face that can't be parsed or has no family name.
pub fn probe_faces(path: &Path) -> Result<Vec<FontFileInfo>, InstallError> {
    inspect(path).map(|inspected| inspected.faces)
}

struct Inspected {
    faces: Vec<FontFileInfo>,
    /// Extension that matches the file's format (`ttf`, `otf` or `ttc`).
    extension: &'static str,
}

fn inspect(path: &Path) -> Result<Inspected, InstallError> {
    let header = read_header(path)?;

    let mut db = fontdb::Database::new();
    db.load_font_file(path)?;

    let mut infos: Vec<&fontdb::FaceInfo> = db.faces().collect();
    infos.sort_by_key(|info| info.index);
    // fontdb skips faces it can't parse; refuse the whole collection rather than half of it.
    if infos.len() != header.faces as usize {
        return Err(InstallError::InvalidFont);
    }

    Ok(Inspected {
        faces: infos
            .into_iter()
            .map(|info| FontFileInfo::from_face(info, header.faces))
            .collect(),
        extension: header.extension,
    })
}

/// Outcome of an installation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Copied to this path and verified on disk.
    Installed(PathBuf),
    /// Every face is already installed; nothing was written.
    AlreadyInstalled { family: String },
}

/// Installs a font file into the user font folder.
///
/// # Errors
/// I/O failure, an unparsable font, or a copy that fails the check.
pub fn install(path: &Path, inventory: &Inventory) -> Result<InstallOutcome, InstallError> {
    let inspected = inspect(path)?;
    let Some(first) = inspected.faces.first() else {
        return Err(InstallError::InvalidFont);
    };

    // Already installed? Compare faces, not file names.
    if inspected
        .faces
        .iter()
        .all(|f| inventory.contains_face(&f.family, f.weight, f.style, f.stretch))
    {
        return Ok(InstallOutcome::AlreadyInstalled {
            family: first.family.clone(),
        });
    }

    let dir = user_font_dir().ok_or(InstallError::NoUserFontDir)?;
    fs::create_dir_all(&dir)?;

    // Never overwrite a different font with the same file name.
    let dest = free_destination(&dir, path, inspected.extension);
    fs::copy(path, &dest)?;
    // A crash right after the copy must not leave a truncated font behind.
    fs::File::open(&dest)?.sync_all()?;

    // Check the copy: it must parse and have the same faces as the original.
    let verified = probe_faces(&dest).is_ok_and(|copy| {
        copy.len() == inspected.faces.len()
            && copy
                .iter()
                .zip(&inspected.faces)
                .all(|(a, b)| a.key() == b.key())
    });
    if !verified {
        let _ = fs::remove_file(&dest);
        return Err(InstallError::VerificationFailed);
    }

    Ok(InstallOutcome::Installed(dest))
}

/// Removes a user font file (every face, for a `.ttc`). Refuses anything outside the user font
/// folder: system fonts belong to the package manager.
///
/// # Errors
/// [`InstallError::NotUserFont`] outside the user folder, or I/O failure.
pub fn uninstall(path: &Path) -> Result<(), InstallError> {
    let canonical = path.canonicalize()?;
    // If the user dir doesn't even exist, nothing can be inside it.
    let dir_canonical = user_font_dir()
        .and_then(|dir| dir.canonicalize().ok())
        .ok_or(InstallError::NotUserFont)?;

    if !canonical.starts_with(&dir_canonical) {
        return Err(InstallError::NotUserFont);
    }

    fs::remove_file(&canonical)?;
    Ok(())
}

/// Extensions `fontdb` picks up, spelled exactly as it expects.
fn is_scanned_extension(ext: &str) -> bool {
    matches!(
        ext,
        "ttf" | "otf" | "ttc" | "otc" | "TTF" | "OTF" | "TTC" | "OTC"
    )
}

/// A free file name in `dir` that the scanner will list: the source's name if possible,
/// `name-1.ext`… on a collision, and the format's extension if the source's isn't one the
/// scanner knows (`font.bin` would install but never show up).
fn free_destination(dir: &Path, src: &Path, format_ext: &str) -> PathBuf {
    let stem: OsString = src
        .file_stem()
        .filter(|s| !s.is_empty())
        .map_or_else(|| "font".into(), std::ffi::OsStr::to_os_string);
    let ext: OsString = match src.extension() {
        Some(e) if e.to_str().is_some_and(is_scanned_extension) => e.to_os_string(),
        _ => format_ext.into(),
    };

    let named = |suffix: &str| {
        let mut name = stem.clone();
        name.push(suffix);
        name.push(".");
        name.push(&ext);
        dir.join(name)
    };

    let candidate = named("");
    if !candidate.exists() {
        return candidate;
    }
    for n in 1..=999u32 {
        let candidate = named(&format!("-{n}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    named("-dup")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_are_classified_by_magic_number() {
        let kind =
            |head: &[u8], len: u64| header_from_bytes(head, len).map(|h| (h.faces, h.extension));

        assert!(matches!(kind(b"\0\x01\0\0abcd", 1000), Ok((1, "ttf"))));
        assert!(matches!(kind(b"true", 1000), Ok((1, "ttf"))));
        assert!(matches!(kind(b"OTTOabcd", 1000), Ok((1, "otf"))));

        let ttc = |count: u32| {
            let mut head = b"ttcf\0\x01\0\0".to_vec();
            head.extend_from_slice(&count.to_be_bytes());
            head
        };
        assert!(matches!(kind(&ttc(2), 1000), Ok((2, "ttc"))));

        // Lies about the count: zero, absurd, or an offset table longer than the file.
        assert!(kind(&ttc(0), 1000).is_err());
        assert!(kind(&ttc(u32::MAX), 1_000_000_000).is_err());
        assert!(kind(&ttc(MAX_FACES_PER_FILE + 1), 1_000_000).is_err());
        assert!(kind(&ttc(100), 100).is_err());
        // Truncated collection header, other formats, empty input.
        assert!(kind(b"ttcf\0\x01", 1000).is_err());
        assert!(kind(b"wOFF", 1000).is_err());
        assert!(kind(b"", 0).is_err());
    }

    #[test]
    fn destination_keeps_scanned_extensions_and_fixes_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let name = |p: PathBuf| p.file_name().unwrap().to_str().unwrap().to_owned();

        assert_eq!(
            name(free_destination(dir.path(), Path::new("/x/Foo.ttf"), "ttf")),
            "Foo.ttf"
        );
        assert_eq!(
            name(free_destination(dir.path(), Path::new("/x/Foo.OTF"), "ttf")),
            "Foo.OTF"
        );
        // The scanner doesn't take `Ttf`: use the format's extension.
        assert_eq!(
            name(free_destination(dir.path(), Path::new("/x/Foo.Ttf"), "ttf")),
            "Foo.ttf"
        );
        assert_eq!(
            name(free_destination(
                dir.path(),
                Path::new("/x/font.bin"),
                "otf"
            )),
            "font.otf"
        );
        assert_eq!(
            name(free_destination(dir.path(), Path::new("/x/noext"), "ttc")),
            "noext.ttc"
        );

        fs::write(dir.path().join("Foo.ttf"), b"x").unwrap();
        fs::write(dir.path().join("Foo-1.ttf"), b"x").unwrap();
        assert_eq!(
            name(free_destination(dir.path(), Path::new("/x/Foo.ttf"), "ttf")),
            "Foo-2.ttf"
        );
    }
}
