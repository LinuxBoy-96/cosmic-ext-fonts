//! The installed fonts, from `fontdb`. `fontdb` lists faces (one per weight, style and width);
//! people browse families, so this groups them.

use std::collections::HashSet;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use fontdb::{Database, FaceInfo, Source};

use crate::installer::user_font_dir;

/// Where a face comes from. Users may only uninstall [`Origin::User`] faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Lives under `$XDG_DATA_HOME/fonts` (or `~/.local/share/fonts`).
    User,
    /// Anywhere else (system packages, `/usr/share/fonts`, ...).
    System,
}

/// What makes two faces the same font: family (ignoring ASCII case), weight, style and width.
/// Used for duplicates and by the renderer, never the file name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FaceKey {
    family: String,
    weight: fontdb::Weight,
    style: fontdb::Style,
    stretch: fontdb::Stretch,
}

impl FaceKey {
    #[must_use]
    pub fn new(
        family: &str,
        weight: fontdb::Weight,
        style: fontdb::Style,
        stretch: fontdb::Stretch,
    ) -> Self {
        Self {
            family: family.to_ascii_lowercase(),
            weight,
            style,
            stretch,
        }
    }
}

/// A single face (one variant of a family).
#[derive(Debug, Clone)]
pub struct FaceEntry {
    pub id: fontdb::ID,
    /// First family name from the font's name table (what UIs display).
    pub family: String,
    pub post_script_name: String,
    pub style: fontdb::Style,
    pub weight: fontdb::Weight,
    pub stretch: fontdb::Stretch,
    pub monospaced: bool,
    /// Path on disk, if the face was loaded from a file.
    pub path: Option<PathBuf>,
    /// Index within a collection file (`.ttc`); 0 for plain `.ttf`/`.otf`.
    pub index: u32,
    pub origin: Origin,
}

impl FaceEntry {
    /// Human-readable variant label, e.g. "Bold Italic Condensed".
    #[must_use]
    pub fn variant_label(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();

        let weight = weight_name(self.weight);
        if !weight.is_empty() {
            parts.push(weight);
        }
        match self.style {
            fontdb::Style::Italic => parts.push("Italic"),
            fontdb::Style::Oblique => parts.push("Oblique"),
            fontdb::Style::Normal => {}
        }
        let stretch = stretch_name(self.stretch);
        if !stretch.is_empty() {
            parts.push(stretch);
        }

        if parts.is_empty() {
            "Regular".to_owned()
        } else {
            parts.join(" ")
        }
    }

    /// The identity used for duplicate detection.
    #[must_use]
    pub fn key(&self) -> FaceKey {
        FaceKey::new(&self.family, self.weight, self.style, self.stretch)
    }
}

/// A family: what the user sees as "one font".
#[derive(Debug, Clone)]
pub struct FamilyEntry {
    pub name: String,
    pub faces: Vec<FaceEntry>,
}

impl FamilyEntry {
    /// True if *any* face of the family is user-installed.
    #[must_use]
    pub fn has_user_faces(&self) -> bool {
        self.faces.iter().any(|f| f.origin == Origin::User)
    }

    /// True if the family ships at least one monospaced face.
    #[must_use]
    pub fn monospaced(&self) -> bool {
        self.faces.iter().any(|f| f.monospaced)
    }
}

/// A face's display name: its first family name, which `fontdb` makes the US English one.
pub(crate) fn display_family(info: &FaceInfo) -> String {
    info.families
        .first()
        .map_or_else(|| info.post_script_name.clone(), |(name, _)| name.clone())
}

fn file_path(info: &FaceInfo) -> Option<&Path> {
    match &info.source {
        Source::File(p) | Source::SharedFile(p, _) => Some(p),
        Source::Binary(_) => None,
    }
}

/// Flatpak's mirror of the host's user fonts. We scan the real folder, so these are hidden.
const FLATPAK_USER_FONTS: &str = "/run/host/user-fonts";

fn is_mirror(info: &FaceInfo) -> bool {
    file_path(info).is_some_and(|p| p.starts_with(FLATPAK_USER_FONTS))
}

/// The inventory. Cheap to rebuild: fontdb loads ~1400 faces in ~25 ms.
#[derive(Debug)]
pub struct Inventory {
    db: Database,
    user_dir: Option<PathBuf>,
}

impl Inventory {
    /// Scans the system font folders (listed in the fontconfig files, read by fontdb) and the
    /// user's.
    #[must_use]
    pub fn load_system() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();

        let user_dir = user_font_dir();
        // The default fontconfig setup already includes `~/.local/share/fonts`; scan it
        // ourselves only if a trimmed config left it out.
        if let Some(dir) = user_dir.as_deref().filter(|d| d.is_dir())
            && !db
                .faces()
                .any(|info| file_path(info).is_some_and(|p| p.starts_with(dir)))
        {
            db.load_fonts_dir(dir);
        }

        remove_duplicate_faces(&mut db);
        Self { db, user_dir }
    }

    /// An inventory of just these folders (the tests use it).
    #[must_use]
    pub fn load_dirs<P: AsRef<Path>>(dirs: &[P]) -> Self {
        let mut db = Database::new();
        for dir in dirs {
            db.load_fonts_dir(dir);
        }
        remove_duplicate_faces(&mut db);
        Self {
            db,
            user_dir: user_font_dir(),
        }
    }

    /// Rescan everything. Call after install/uninstall or a watcher event.
    pub fn reload(&mut self) {
        *self = Self::load_system();
    }

    /// Number of distinct faces (a `.ttc` with 5 faces counts 5).
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.db.faces().filter(|info| !is_mirror(info)).count()
    }

    fn face_entry(&self, info: &FaceInfo) -> FaceEntry {
        let path = file_path(info).map(Path::to_path_buf);

        let origin = match (&path, &self.user_dir) {
            (Some(p), Some(dir)) if p.starts_with(dir) => Origin::User,
            _ => Origin::System,
        };

        FaceEntry {
            id: info.id,
            family: display_family(info),
            post_script_name: info.post_script_name.clone(),
            style: info.style,
            weight: info.weight,
            stretch: info.stretch,
            monospaced: info.monospaced,
            path,
            index: info.index,
            origin,
        }
    }

    /// All faces, ungrouped.
    #[must_use]
    pub fn faces(&self) -> Vec<FaceEntry> {
        self.db
            .faces()
            .filter(|info| !is_mirror(info))
            .map(|info| self.face_entry(info))
            .collect()
    }

    /// How many faces live in this file (more than one for a `.ttc`).
    #[must_use]
    pub fn faces_in_file(&self, path: &Path) -> usize {
        self.db
            .faces()
            .filter(|info| file_path(info) == Some(path))
            .count()
    }

    /// Faces grouped by family, ignoring case like the duplicate check, sorted by name.
    #[must_use]
    pub fn families(&self) -> Vec<FamilyEntry> {
        let mut families: Vec<FamilyEntry> = Vec::new();

        let mut faces = self.faces();
        faces.sort_by(|a, b| {
            a.family
                .to_lowercase()
                .cmp(&b.family.to_lowercase())
                .then(a.weight.0.cmp(&b.weight.0))
        });

        for face in faces {
            match families.last_mut() {
                Some(fam) if fam.name.eq_ignore_ascii_case(&face.family) => fam.faces.push(face),
                _ => families.push(FamilyEntry {
                    name: face.family.clone(),
                    faces: vec![face],
                }),
            }
        }

        families
    }

    /// Whether this exact face is installed. gnome-font-viewer compared file names instead,
    /// which breaks as soon as two `webfont.ttf` collide.
    #[must_use]
    pub fn contains_face(
        &self,
        family: &str,
        weight: fontdb::Weight,
        style: fontdb::Style,
        stretch: fontdb::Stretch,
    ) -> bool {
        self.db.faces().any(|f| {
            f.weight == weight
                && f.style == style
                && f.stretch == stretch
                && f.families
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case(family))
        })
    }

    /// Is any face of this family installed?
    #[must_use]
    pub fn contains_family(&self, family: &str) -> bool {
        self.db.faces().any(|f| {
            f.families
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case(family))
        })
    }

    /// Raw access for advanced queries.
    #[must_use]
    pub fn db(&self) -> &Database {
        &self.db
    }
}

/// Overlapping folders can list a file twice; keep one entry per (file, face index).
fn remove_duplicate_faces(db: &mut Database) {
    let mut seen: HashSet<(PathBuf, u32)> = HashSet::new();
    let duplicates: Vec<fontdb::ID> = db
        .faces()
        .filter_map(|info| {
            let path = file_path(info)?;
            (!seen.insert((path.to_path_buf(), info.index))).then_some(info.id)
        })
        .collect();
    for id in duplicates {
        db.remove_face(id);
    }
}

/// Font files the renderer hasn't been given yet.
///
/// Adds every face to `known` (what the renderer can draw) and returns the files of the new
/// ones, each once. Call it on the first inventory to seed `known` (the renderer scans the same
/// folders at startup), then after each reload.
pub fn files_to_register<S: BuildHasher>(
    families: &[FamilyEntry],
    known: &mut HashSet<FaceKey, S>,
) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for face in families.iter().flat_map(|family| &family.faces) {
        if known.insert(face.key())
            && let Some(path) = &face.path
            && !files.contains(path)
        {
            files.push(path.clone());
        }
    }
    files
}

/// Weight buckets shared by every label and by the renderer mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightClass {
    Thin,
    ExtraLight,
    Light,
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

/// CSS-ish bucket of a numeric weight.
#[must_use]
pub fn weight_class(weight: fontdb::Weight) -> WeightClass {
    match weight.0 {
        0..=149 => WeightClass::Thin,
        150..=249 => WeightClass::ExtraLight,
        250..=349 => WeightClass::Light,
        350..=449 => WeightClass::Regular,
        450..=549 => WeightClass::Medium,
        550..=649 => WeightClass::SemiBold,
        650..=749 => WeightClass::Bold,
        750..=849 => WeightClass::ExtraBold,
        _ => WeightClass::Black,
    }
}

/// English weight names; regular is empty (no qualifier).
#[must_use]
pub fn weight_name(weight: fontdb::Weight) -> &'static str {
    match weight_class(weight) {
        WeightClass::Thin => "Thin",
        WeightClass::ExtraLight => "Extra Light",
        WeightClass::Light => "Light",
        WeightClass::Regular => "",
        WeightClass::Medium => "Medium",
        WeightClass::SemiBold => "Semi Bold",
        WeightClass::Bold => "Bold",
        WeightClass::ExtraBold => "Extra Bold",
        WeightClass::Black => "Black",
    }
}

#[must_use]
pub fn stretch_name(stretch: fontdb::Stretch) -> &'static str {
    match stretch {
        fontdb::Stretch::UltraCondensed => "Ultra Condensed",
        fontdb::Stretch::ExtraCondensed => "Extra Condensed",
        fontdb::Stretch::Condensed => "Condensed",
        fontdb::Stretch::SemiCondensed => "Semi Condensed",
        fontdb::Stretch::Normal => "",
        fontdb::Stretch::SemiExpanded => "Semi Expanded",
        fontdb::Stretch::Expanded => "Expanded",
        fontdb::Stretch::ExtraExpanded => "Extra Expanded",
        fontdb::Stretch::UltraExpanded => "Ultra Expanded",
    }
}
