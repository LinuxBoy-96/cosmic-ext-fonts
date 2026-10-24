// SPDX-License-Identifier: GPL-3.0-or-later
//
// The app, written for the libcosmic revision pinned in Cargo.toml.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use cosmic::app::{ContextDrawer, Core, Task};
use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use cosmic::iced::font as iced_font;
use cosmic::iced::futures::{SinkExt, Stream, StreamExt};
use cosmic::iced::widget::container as iced_container;
use cosmic::iced::widget::scrollable::{self as iced_scrollable, RelativeOffset};
use cosmic::iced::widget::text::LineHeight;
use cosmic::iced::widget::{Column, Row, Space};
use cosmic::iced::{Alignment, Border, Color, Length, Subscription};
use cosmic::widget::{self, icon, menu};
use cosmic::{Application, ApplicationExt, Element};

use fonts_core::catalog::{self, CatalogError, CatalogFont, CatalogVariant};
use fonts_core::fontdb;
use fonts_core::installer::{self, FontFileInfo, InstallError, InstallOutcome};
use fonts_core::inventory::{
    self, FaceEntry, FaceKey, FamilyEntry, Inventory, Origin, WeightClass,
};
use fonts_core::watcher::FontDirWatcher;

use crate::about;
use crate::fl;
use crate::open_dialog;

/// The app icon, for the About page.
const APP_ICON: &[u8] =
    include_bytes!("../res/icons/hicolor/scalable/apps/io.github.linuxboy_96.cosmic-ext-fonts.svg");
const WATERFALL_SIZES: [f32; 6] = [12.0, 16.0, 20.0, 28.0, 40.0, 56.0];
/// Waterfall line height, in times the font size; the default (1.3) leaves big gaps between sizes.
const WATERFALL_LINE_HEIGHT: f32 = 1.15;
/// Where the app keeps its own settings (`~/.config/cosmic/<APP_ID>/v1/`).
const CONFIG_VERSION: u64 = 1;
const THEME_KEY: &str = "app_theme";
/// The catalog (~1500 rows) is shown a page at a time; rendering it all is slow.
const CATALOG_PAGE: usize = 200;
/// A burst of file writes (copying twenty fonts) counts as one change.
const WATCH_SETTLE: Duration = Duration::from_millis(300);

pub struct Flags {
    pub open_path: Option<PathBuf>,
}

/// Font files on their way to the renderer. `Debug` prints sizes, never bytes.
#[derive(Clone, Default)]
pub struct FontData(Vec<Vec<u8>>);

impl fmt::Debug for FontData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes: usize = self.0.iter().map(Vec::len).sum();
        write!(f, "FontData({} files, {bytes} bytes)", self.0.len())
    }
}

/// A catalog variant downloaded for preview: its faces and its bytes. Nothing stays on disk.
#[derive(Debug, Clone)]
pub struct CatalogPreviewData {
    faces: Vec<FontFileInfo>,
    fonts: FontData,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// The inventory finished loading. The number is the request; stale answers are dropped.
    InventoryLoaded(u64, Vec<FamilyEntry>),
    Search(String),
    SelectFamily(usize),
    /// A face row was clicked: show the waterfall in that face.
    SelectFace(usize),
    Back,
    PreviewText(String),
    /// A font file was opened: what each of its faces is.
    ProbedFile(PathBuf, Result<Vec<FontFileInfo>, String>),
    OpenFileDialog,
    /// The chooser's answer: a file, nothing (closed), or why it couldn't open.
    ChooserAnswered(Result<Option<PathBuf>, String>),
    /// The "couldn't open the chooser" dialog was closed.
    DismissChooserError,
    FileChosen(PathBuf),
    Install,
    Installed(Result<InstallOutcome, String>),
    /// First click: ask to confirm (nothing deleted yet). File and face index (collections hold
    /// several faces).
    UninstallFace(PathBuf, u32),
    ConfirmUninstall,
    /// First click on "Uninstall family": ask to confirm. Takes the lowercased family name.
    UninstallFamily(String),
    ConfirmUninstallFamily,
    CancelUninstall,
    Uninstalled(Result<(), String>),
    /// Font files read off-thread, for the renderer.
    FontsRead(FontData),
    FontLoaded(Result<(), iced_font::Error>),
    Refresh,
    /// The user font folder changed (another program installed or removed a font).
    FontsChanged,
    Tab(widget::segmented_button::Entity),
    /// View > About, or the drawer's close button: toggle the About page.
    ToggleAbout,
    SetTheme(ThemeChoice),
    OpenUrl(String),
    // Catalog (Google Fonts via gwfh) --------------------------------------
    /// Show the catalog list ("Retry" after a failure comes here too).
    OpenCatalog,
    CatalogLoaded(Result<Vec<CatalogFont>, String>),
    CatalogSearch(String),
    ShowMoreCatalog,
    SelectCatalogFont(usize),
    /// A variant of the open catalog family was clicked: preview it.
    SelectCatalogVariant(usize),
    /// A variant's download finished (or failed), keyed by its URL.
    CatalogPreviewLoaded(String, Result<CatalogPreviewData, String>),
    /// Keyed by catalog id.
    CatalogVariantsLoaded(String, Result<Vec<CatalogVariant>, String>),
    InstallVariant {
        url: String,
        family: String,
        variant_id: String,
    },
    CatalogInstalled(Result<InstallOutcome, String>),
}

enum View {
    /// List of installed families.
    Browse,
    /// Detail of one installed family (waterfall + faces + uninstall).
    /// Keyed by name: indices go stale every time the inventory reloads.
    Family {
        name: &'static str,
        status: Option<String>,
    },
    /// A font file not (necessarily) installed yet: preview + Install.
    File {
        path: PathBuf,
        info: Option<FontFileInfo>,
        /// Interned family name for the preview font, set once probed.
        family: Option<&'static str>,
        error: Option<String>,
        status: Option<String>,
    },
    /// Remote catalog: browsable list of installable families.
    Catalog,
    /// One catalog family: a preview, and its variants with Install / Uninstall.
    CatalogFamily {
        index: usize,
        variants: Option<Vec<CatalogVariant>>,
        /// The variant on show: an index into `variants`.
        selected: Option<usize>,
        preview: CatalogPreview,
        error: Option<String>,
        status: Option<String>,
    },
}

/// What the preview of the selected catalog variant is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CatalogPreview {
    /// Nothing to draw: no variant selected, or one that has no TTF to fetch.
    Nothing,
    Loading,
    /// The renderer has the font, under this family name.
    Ready(&'static str),
    Failed(String),
}

/// What's waiting for confirmation. Removal is permanent: fonts have no trash.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    /// One face: its file and index (a collection holds several).
    Face(PathBuf, u32),
    /// Every user-installed face of a family (lowercased name).
    Family(String),
}

/// The app's theme: follow the desktop, or force light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Follow COSMIC, including later changes.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// The value saved in the settings file.
    fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Unknown values (an empty or hand-edited file) mean the default.
    fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|choice| choice.key() == key)
            .unwrap_or_default()
    }

    fn label(self) -> String {
        match self {
            Self::System => fl!("theme-system"),
            Self::Light => fl!("theme-light"),
            Self::Dark => fl!("theme-dark"),
        }
    }

    /// Light and dark use the user's own COSMIC themes, kept even if the desktop switches mode.
    fn theme(self) -> cosmic::Theme {
        match self {
            Self::System => cosmic::theme::system_preference(),
            Self::Light => pinned_theme(false),
            Self::Dark => pinned_theme(true),
        }
    }
}

/// The desktop's theme for one mode (accent colour included), locked to that mode.
fn pinned_theme(dark: bool) -> cosmic::Theme {
    pin(
        &if dark {
            cosmic::theme::system_dark()
        } else {
            cosmic::theme::system_light()
        },
        dark,
    )
}

/// Turns the desktop's theme for a mode (`system`) into a custom theme: libcosmic keeps
/// re-applying the desktop's mode to system themes, which would undo the choice. Without a
/// theme for that mode (not COSMIC, or a fresh account) the desktop answers with the dark
/// default, so the built-in palette of the mode is used instead.
fn pin(system: &cosmic::Theme, dark: bool) -> cosmic::Theme {
    let palette = if system.cosmic().is_dark == dark {
        system.cosmic().clone()
    } else if dark {
        cosmic::cosmic_theme::Theme::dark_default()
    } else {
        cosmic::cosmic_theme::Theme::light_default()
    };
    cosmic::Theme::custom(Arc::new(palette))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    Theme(ThemeChoice),
    /// Open or close the About page.
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Message {
        match *self {
            MenuAction::Theme(choice) => Message::SetTheme(choice),
            MenuAction::About => Message::ToggleAbout,
        }
    }
}

/// State of the desktop's file chooser.
#[derive(Debug, Default, PartialEq, Eq)]
enum Chooser {
    #[default]
    Closed,
    /// Open; we only ask for one at a time.
    Open,
    /// It couldn't open, and why; a dialog shows it until closed.
    Failed(String),
}

/// The two lists of the top switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Installed,
    Catalog,
}

pub struct AppModel {
    core: Core,
    view: View,
    /// The "Installed | Catalog" switcher; only `show_tab` moves its selection.
    tabs: widget::segmented_button::SingleSelectModel,
    families: Vec<FamilyEntry>,
    filtered: Vec<usize>,
    search: String,
    preview_text: String,
    /// Index into the current family's faces; None = family default.
    selected_face: Option<usize>,
    loading: bool,
    /// Last inventory request sent; older answers are ignored.
    reload_seq: u64,
    /// How many faces each installed font file holds (more than one for a `.ttc`).
    file_faces: HashMap<PathBuf, usize>,
    /// Faces the renderer can draw: what was installed at startup (it scanned the same
    /// folders) plus whatever we registered since.
    renderer_faces: HashSet<FaceKey>,
    /// False until the first inventory has seeded `renderer_faces`.
    renderer_seeded: bool,
    /// Removal awaiting its confirmation (only one at a time).
    pending_uninstall: Option<Pending>,
    chooser: Chooser,
    /// `iced::font::Family::Name` needs a `&'static str`: leak each name once, not per frame.
    interned: HashMap<String, &'static str>,
    browse_scroll: widget::Id,
    catalog_scroll: widget::Id,
    // Catalog state ---------------------------------------------------------
    catalog: Vec<CatalogFont>,
    /// Parallel to `catalog`: is a family of that name installed?
    catalog_installed: Vec<bool>,
    catalog_filtered: Vec<usize>,
    catalog_search: String,
    /// Rows of the catalog list currently shown (grows by a page on "Show more").
    catalog_shown: usize,
    catalog_loading: bool,
    catalog_error: Option<String>,
    /// Catalog variants already downloaded and handed to the renderer: TTF URL → family name.
    previews: HashMap<String, &'static str>,
    // Settings and menus -----------------------------------------------------
    theme_choice: ThemeChoice,
    /// `None` when there is nowhere to keep settings (no home directory).
    config: Option<Config>,
    /// Menu shortcuts (none yet).
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// The icon of the About page, made once.
    about_icon: icon::Handle,
}

/// Run blocking work on the tokio pool and hand the result to `map`.
fn blocking<T, F, M>(work: F, map: M) -> Task<Message>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
    M: Fn(Result<T, String>) -> Message + Send + Sync + 'static,
{
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(work).await {
                Ok(inner) => inner,
                Err(join_err) => Err(join_err.to_string()),
            }
        },
        move |res| cosmic::Action::App(map(res)),
    )
}

/// Tells the UI when the user font folder changes behind its back. `notify` uses a blocking
/// channel, so it gets its own thread.
fn watch_user_fonts() -> impl Stream<Item = Message> {
    cosmic::iced::stream::channel(1, async |mut output| {
        let (tx, mut changes) = cosmic::iced::futures::channel::mpsc::unbounded::<()>();
        std::thread::spawn(move || {
            let Ok(watcher) = FontDirWatcher::start() else {
                return;
            };
            while watcher.wait() {
                // Let a burst of writes settle, then report it once.
                std::thread::sleep(WATCH_SETTLE);
                let _ = watcher.poll();
                if tx.unbounded_send(()).is_err() {
                    break;
                }
            }
        });
        while changes.next().await.is_some() {
            if output.send(Message::FontsChanged).await.is_err() {
                break;
            }
        }
    })
}

// ---------------------------------------------------------------- labels

fn weight_label(class: WeightClass) -> String {
    match class {
        WeightClass::Thin => fl!("weight-thin"),
        WeightClass::ExtraLight => fl!("weight-extra-light"),
        WeightClass::Light => fl!("weight-light"),
        WeightClass::Regular => fl!("weight-regular"),
        WeightClass::Medium => fl!("weight-medium"),
        WeightClass::SemiBold => fl!("weight-semi-bold"),
        WeightClass::Bold => fl!("weight-bold"),
        WeightClass::ExtraBold => fl!("weight-extra-bold"),
        WeightClass::Black => fl!("weight-black"),
    }
}

fn stretch_label(stretch: fontdb::Stretch) -> Option<String> {
    use fontdb::Stretch as S;
    Some(match stretch {
        S::UltraCondensed => fl!("stretch-ultra-condensed"),
        S::ExtraCondensed => fl!("stretch-extra-condensed"),
        S::Condensed => fl!("stretch-condensed"),
        S::SemiCondensed => fl!("stretch-semi-condensed"),
        S::Normal => return None,
        S::SemiExpanded => fl!("stretch-semi-expanded"),
        S::Expanded => fl!("stretch-expanded"),
        S::ExtraExpanded => fl!("stretch-extra-expanded"),
        S::UltraExpanded => fl!("stretch-ultra-expanded"),
    })
}

/// "Bold Italic Condensed" in the user's language; "Regular" when nothing sets it apart.
fn variant_label(weight: fontdb::Weight, style: fontdb::Style, stretch: fontdb::Stretch) -> String {
    let mut parts: Vec<String> = Vec::new();
    let class = inventory::weight_class(weight);
    if class != WeightClass::Regular {
        parts.push(weight_label(class));
    }
    match style {
        fontdb::Style::Italic => parts.push(fl!("style-italic")),
        fontdb::Style::Oblique => parts.push(fl!("style-oblique")),
        fontdb::Style::Normal => {}
    }
    parts.extend(stretch_label(stretch));
    if parts.is_empty() {
        weight_label(WeightClass::Regular)
    } else {
        parts.join(" ")
    }
}

/// Google Fonts categories, translated; unknown ones are shown as-is.
fn category_label(category: &str) -> String {
    match category {
        "sans-serif" => fl!("category-sans-serif"),
        "serif" => fl!("category-serif"),
        "display" => fl!("category-display"),
        "handwriting" => fl!("category-handwriting"),
        "monospace" => fl!("category-monospace"),
        other => other.to_owned(),
    }
}

fn install_error_text(error: &InstallError) -> String {
    match error {
        InstallError::Io(e) => fl!("error-io", error = e.to_string()),
        InstallError::InvalidFont => fl!("error-invalid-font"),
        InstallError::TooLarge => fl!("error-font-too-large"),
        InstallError::VerificationFailed => fl!("error-verification"),
        InstallError::NotUserFont => fl!("error-not-user-font"),
        InstallError::NoUserFontDir => fl!("error-no-user-dir"),
    }
}

fn catalog_error_text(error: &CatalogError) -> String {
    match error {
        CatalogError::Network(e) => fl!("error-network", error = e.clone()),
        CatalogError::Parse(e) => fl!("error-catalog-response", error = e.clone()),
        CatalogError::NoTtf => fl!("no-ttf"),
        CatalogError::Io(e) => fl!("error-io", error = e.to_string()),
        CatalogError::TooLarge => fl!("error-download-too-large"),
        CatalogError::InvalidId => fl!("error-catalog-id"),
        CatalogError::UntrustedUrl(url) => fl!("error-untrusted-url", url = url.clone()),
    }
}

/// The switcher's entries: "Installed" first, and selected.
fn tab_model() -> widget::segmented_button::SingleSelectModel {
    widget::segmented_button::Model::builder()
        .insert(|b| b.text(fl!("tab-installed")).data(Tab::Installed).activate())
        .insert(|b| b.text(fl!("tab-catalog")).data(Tab::Catalog))
        .build()
}

fn tab_entity(
    model: &widget::segmented_button::SingleSelectModel,
    tab: Tab,
) -> Option<widget::segmented_button::Entity> {
    model
        .iter()
        .find(|&entity| model.data::<Tab>(entity) == Some(&tab))
}

/// The line under the tabs for the installed list: the counts, or the matches while searching.
fn installed_line(total: usize, by_user: usize, found: Option<usize>) -> String {
    if let Some(found) = found {
        return fl!("results-count", shown = found, total = total);
    }
    let mut parts = vec![fl!("installed-count", total = total)];
    if by_user > 0 {
        parts.push(fl!("installed-by-user", count = by_user));
    }
    parts.join(" · ")
}

/// Same for the catalog: how many families, and how many are installed already.
fn catalog_line(total: usize, installed: usize, found: Option<usize>) -> String {
    if let Some(found) = found {
        return fl!("results-count", shown = found, total = total);
    }
    let mut parts = vec![fl!("catalog-count", total = total)];
    if installed > 0 {
        parts.push(fl!("catalog-installed", count = installed));
    }
    parts.join(" · ")
}

/// View > Theme: one entry per choice, the current one ticked.
fn theme_items(current: ThemeChoice) -> Vec<menu::Item<MenuAction, String>> {
    ThemeChoice::ALL
        .into_iter()
        .map(|choice| {
            menu::Item::CheckBox(
                choice.label(),
                None,
                choice == current,
                MenuAction::Theme(choice),
            )
        })
        .collect()
}

/// Opens a link in the default browser through the desktop's portal (also in a Flatpak). A
/// failure goes to the terminal.
fn open_url(url: String) -> Task<Message> {
    cosmic::iced::Task::future(async move {
        if let Err(error) = open_dialog::open_uri(&url).await {
            eprintln!("cosmic-ext-fonts: cannot open {url}: {error}");
        }
    })
    .discard()
}

/// Family names that differ only in case are the same family.
fn same_family(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// The variant a catalog family opens on: its regular one, else its first.
fn default_variant(variants: &[CatalogVariant]) -> Option<usize> {
    variants
        .iter()
        .position(|v| v.weight == fontdb::Weight::NORMAL && v.style == fontdb::Style::Normal)
        .or_else(|| (!variants.is_empty()).then_some(0))
}

/// Downloads a variant to a private temp file and keeps only what the renderer needs: a
/// preview installs nothing.
fn fetch_preview(url: &str, family: &str, variant_id: &str) -> Result<CatalogPreviewData, String> {
    let download =
        catalog::download_ttf(url, family, variant_id).map_err(|e| catalog_error_text(&e))?;
    let faces = installer::probe_faces(&download.path).map_err(|e| install_error_text(&e))?;
    let bytes = installer::read_font_file(&download.path).map_err(|e| install_error_text(&e))?;
    Ok(CatalogPreviewData {
        faces,
        fonts: FontData(vec![bytes]),
    })
}

impl AppModel {
    /// The starting model. `config` holds the settings (`None` without a home directory).
    fn new(core: Core, config: Option<Config>) -> Self {
        let theme_choice = config
            .as_ref()
            .and_then(|config| config.get::<String>(THEME_KEY).ok())
            .map_or_else(ThemeChoice::default, |key| ThemeChoice::from_key(&key));

        Self {
            core,
            view: View::Browse,
            tabs: tab_model(),
            families: Vec::new(),
            filtered: Vec::new(),
            search: String::new(),
            preview_text: fl!("pangram"),
            selected_face: None,
            loading: true,
            reload_seq: 0,
            file_faces: HashMap::new(),
            renderer_faces: HashSet::new(),
            renderer_seeded: false,
            pending_uninstall: None,
            chooser: Chooser::Closed,
            interned: HashMap::new(),
            browse_scroll: widget::Id::new("browse"),
            catalog_scroll: widget::Id::new("catalog"),
            catalog: Vec::new(),
            catalog_installed: Vec::new(),
            catalog_filtered: Vec::new(),
            catalog_search: String::new(),
            catalog_shown: CATALOG_PAGE,
            catalog_loading: false,
            catalog_error: None,
            previews: HashMap::new(),
            theme_choice,
            config,
            key_binds: HashMap::new(),
            about_icon: icon::from_svg_bytes(APP_ICON),
        }
    }

    /// The View menu: Theme, then About.
    fn view_menu(&self) -> Vec<menu::Item<MenuAction, String>> {
        vec![
            menu::Item::Folder(fl!("theme"), theme_items(self.theme_choice)),
            menu::Item::Divider,
            menu::Item::Button(fl!("about"), None, MenuAction::About),
        ]
    }

    fn intern(&mut self, name: &str) -> &'static str {
        if let Some(&s) = self.interned.get(name) {
            return s;
        }
        let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
        self.interned.insert(name.to_owned(), leaked);
        leaked
    }

    /// Show a status line in whichever view is current (if it has one).
    fn set_status(&mut self, message: String) {
        match &mut self.view {
            View::Family { status, .. }
            | View::File { status, .. }
            | View::CatalogFamily { status, .. } => *status = Some(message),
            View::Browse | View::Catalog => {}
        }
    }

    /// Uninstall, then Confirm or Cancel (there's no trash). The question is per face, and it
    /// says how many fonts the file holds. `None` for a face with no file.
    fn uninstall_button(&self, face: &FaceEntry) -> Option<Element<'static, Message>> {
        let path = face.path.as_ref()?;
        let target = Pending::Face(path.clone(), face.index);

        if self.pending_uninstall.as_ref() != Some(&target) {
            return Some(
                widget::button::destructive(fl!("uninstall"))
                    .on_press(Message::UninstallFace(path.clone(), face.index))
                    .into(),
            );
        }

        let mut row = Row::new()
            .spacing(cosmic::theme::active().cosmic().spacing.space_xxs)
            .align_y(Alignment::Center);
        let sharing = self.file_faces.get(path).copied().unwrap_or(1);
        if sharing > 1 {
            row = row.push(widget::text::caption(fl!(
                "uninstall-collection",
                count = sharing
            )));
        }
        Some(
            row.push(
                widget::button::destructive(fl!("confirm")).on_press(Message::ConfirmUninstall),
            )
            .push(widget::button::standard(fl!("cancel")).on_press(Message::CancelUninstall))
            .into(),
        )
    }

    /// "Uninstall family": removes every user-installed face after one confirmation. `None`
    /// when there's nothing of the user's to remove.
    fn uninstall_family_button(&self, family: &str) -> Option<Element<'static, Message>> {
        let files = self.user_files(family);
        if files.is_empty() {
            return None;
        }
        let key = family.to_lowercase();
        if self.pending_uninstall != Some(Pending::Family(key.clone())) {
            return Some(
                widget::button::destructive(fl!("uninstall-family"))
                    .on_press(Message::UninstallFamily(key))
                    .into(),
            );
        }

        // A collection goes whole: count the faces of each file, not just the family's.
        let faces: usize = files
            .iter()
            .map(|path| self.file_faces.get(path).copied().unwrap_or(1))
            .sum();
        Some(
            Row::new()
                .spacing(cosmic::theme::active().cosmic().spacing.space_xxs)
                .align_y(Alignment::Center)
                .push(
                    widget::button::destructive(fl!("uninstall-family-confirm", count = faces))
                        .on_press(Message::ConfirmUninstallFamily),
                )
                .push(widget::button::standard(fl!("cancel")).on_press(Message::CancelUninstall))
                .into(),
        )
    }

    /// The files of the family's user-installed faces, each once.
    fn user_files(&self, family: &str) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        let faces = self
            .installed_family(family)
            .into_iter()
            .flat_map(|fam| &fam.faces);
        for face in faces {
            if face.origin == Origin::User
                && let Some(path) = &face.path
                && !files.contains(path)
            {
                files.push(path.clone());
            }
        }
        files
    }

    /// Whether the thing awaiting confirmation is already gone.
    fn pending_is_moot(&self) -> bool {
        match &self.pending_uninstall {
            Some(Pending::Face(path, _)) => !self.file_faces.contains_key(path),
            Some(Pending::Family(name)) => self.user_files(name).is_empty(),
            None => false,
        }
    }

    fn refilter(&mut self) {
        let needle = self.search.to_lowercase();
        self.filtered = self
            .families
            .iter()
            .enumerate()
            .filter(|(_, fam)| needle.is_empty() || fam.name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
    }

    /// Switches list; the switcher and the view always change together. Opening the catalog
    /// the first time (or after a failure) fetches it.
    fn show_tab(&mut self, tab: Tab) -> Task<Message> {
        if let Some(entity) = tab_entity(&self.tabs, tab) {
            self.tabs.activate(entity);
        }
        self.selected_face = None;
        self.pending_uninstall = None;
        match tab {
            Tab::Installed => {
                self.view = View::Browse;
            }
            Tab::Catalog => {
                self.view = View::Catalog;
                self.catalog_error = None;
                if self.catalog.is_empty() && !self.catalog_loading {
                    self.catalog_loading = true;
                    return Self::fetch_catalog_task();
                }
            }
        }
        Task::none()
    }

    /// Which catalog families are installed (by name, ignoring case), for the badges and count.
    fn refresh_catalog_installed(&mut self) {
        let names: HashSet<String> = self
            .families
            .iter()
            .map(|fam| fam.name.to_lowercase())
            .collect();
        self.catalog_installed = self
            .catalog
            .iter()
            .map(|font| names.contains(&font.family.to_lowercase()))
            .collect();
    }

    fn refilter_catalog(&mut self) {
        let needle = self.catalog_search.to_lowercase();
        self.catalog_filtered = self
            .catalog
            .iter()
            .enumerate()
            .filter(|(_, f)| needle.is_empty() || f.family.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
    }

    fn reload_inventory(&mut self) -> Task<Message> {
        self.reload_seq += 1;
        let seq = self.reload_seq;
        // fontdb scans ~1400 faces in ~25 ms; still keep it off the UI thread.
        blocking(
            || Ok::<_, String>(Inventory::load_system().families()),
            move |res| Message::InventoryLoaded(seq, res.unwrap_or_default()),
        )
    }

    /// Hands font files to the renderer (it only knows the fonts present at startup) so
    /// previews can use them by name. Reads off-thread; [`Message::FontsRead`] loads them.
    fn register_fonts(paths: Vec<PathBuf>) -> Task<Message> {
        blocking(
            move || {
                Ok::<_, String>(FontData(
                    paths
                        .iter()
                        .filter_map(|path| installer::read_font_file(path).ok())
                        .collect(),
                ))
            },
            |res| Message::FontsRead(res.unwrap_or_default()),
        )
    }

    /// Probe a font file off-thread (it is memory-mapped, not read).
    fn probe_file(path: PathBuf) -> Task<Message> {
        let probe_path = path.clone();
        blocking(
            move || installer::probe_faces(&probe_path).map_err(|e| install_error_text(&e)),
            move |res| Message::ProbedFile(path.clone(), res),
        )
    }

    fn scroll_to_top(id: &widget::Id) -> Task<Message> {
        iced_scrollable::snap_to(id.clone(), RelativeOffset::START.into())
    }

    fn open_chooser() -> Task<Message> {
        Task::perform(open_dialog::ask(open_dialog::start_folder()), |answer| {
            cosmic::Action::App(Message::ChooserAnswered(answer))
        })
    }

    /// The installed family with this name, ignoring case.
    fn installed_family(&self, family: &str) -> Option<&FamilyEntry> {
        self.families
            .iter()
            .find(|fam| same_family(&fam.name, family))
    }

    /// The installed entry for this exact face, so callers can offer Uninstall or a System badge.
    fn installed_face(
        &self,
        family: &str,
        weight: fontdb::Weight,
        style: fontdb::Style,
        stretch: fontdb::Stretch,
    ) -> Option<&FaceEntry> {
        self.installed_family(family).and_then(|fam| {
            fam.faces
                .iter()
                .find(|f| f.weight == weight && f.style == style && f.stretch == stretch)
        })
    }

    /// Shows a variant of the catalog family, with a font the renderer has (installed or
    /// downloaded before) or after a download kept in memory only. Nothing is installed.
    fn select_variant(&mut self, position: usize) -> Task<Message> {
        let View::CatalogFamily {
            index,
            variants: Some(variants),
            ..
        } = &self.view
        else {
            return Task::none();
        };
        let (Some(font), Some(variant)) = (self.catalog.get(*index), variants.get(position)) else {
            return Task::none();
        };
        let family = font.family.clone();
        let variant = variant.clone();

        let installed = self
            .installed_face(
                &family,
                variant.weight,
                variant.style,
                fontdb::Stretch::Normal,
            )
            .and_then(|_| self.installed_family(&family))
            .map(|fam| fam.name.clone());

        let mut task = Task::none();
        let preview = if let Some(name) = installed {
            CatalogPreview::Ready(self.intern(&name))
        } else if let Some(url) = variant.ttf_url {
            if let Some(&name) = self.previews.get(&url) {
                CatalogPreview::Ready(name)
            } else {
                task = Self::preview_task(url, family, variant.id);
                CatalogPreview::Loading
            }
        } else {
            CatalogPreview::Nothing
        };

        if let View::CatalogFamily {
            selected,
            preview: shown,
            ..
        } = &mut self.view
        {
            *selected = Some(position);
            *shown = preview;
        }
        task
    }

    fn preview_task(url: String, family: String, variant_id: String) -> Task<Message> {
        let answers = url.clone();
        blocking(
            move || fetch_preview(&url, &family, &variant_id),
            move |res| Message::CatalogPreviewLoaded(answers.clone(), res),
        )
    }

    /// A downloaded variant: give the renderer its font (once), remember it by URL, and show
    /// it if the user is still on that variant.
    fn preview_loaded(
        &mut self,
        url: &str,
        result: Result<CatalogPreviewData, String>,
    ) -> Task<Message> {
        let mut task = Task::none();
        let outcome = match result {
            Ok(data) => {
                if let Some(first) = data.faces.first() {
                    let name = self.intern(&first.family);
                    // The renderer never unloads a font: hand it only what it lacks.
                    let mut unseen = false;
                    for face in &data.faces {
                        unseen |= self.renderer_faces.insert(face.key());
                    }
                    if unseen {
                        task = self.update(Message::FontsRead(data.fonts));
                    }
                    self.previews.insert(url.to_owned(), name);
                    CatalogPreview::Ready(name)
                } else {
                    CatalogPreview::Failed(fl!("error-invalid-font"))
                }
            }
            Err(error) => CatalogPreview::Failed(error),
        };

        // The answer may be for a variant the user has left since.
        if let View::CatalogFamily {
            variants: Some(variants),
            selected: Some(position),
            preview,
            ..
        } = &mut self.view
            && variants.get(*position).and_then(|v| v.ttf_url.as_deref()) == Some(url)
        {
            *preview = outcome;
        }
        task
    }

    /// Applies a theme and saves the choice.
    fn set_theme(&mut self, choice: ThemeChoice) -> Task<Message> {
        self.theme_choice = choice;
        if let Some(config) = &self.config {
            // Failing to save is no reason to refuse the change.
            let _ = config.set(THEME_KEY, choice.key());
        }
        cosmic::command::set_theme(choice.theme())
    }

    fn fetch_catalog_task() -> Task<Message> {
        blocking(
            || catalog::fetch_catalog().map_err(|e| catalog_error_text(&e)),
            Message::CatalogLoaded,
        )
    }

    fn fetch_variants_task(font_id: String) -> Task<Message> {
        let answers = font_id.clone();
        blocking(
            move || catalog::fetch_variants(&font_id).map_err(|e| catalog_error_text(&e)),
            move |res| Message::CatalogVariantsLoaded(answers.clone(), res),
        )
    }

    fn install_status(result: Result<InstallOutcome, String>) -> String {
        match result {
            Ok(InstallOutcome::Installed(dest)) => {
                fl!("installed-to", path = dest.display().to_string())
            }
            Ok(InstallOutcome::AlreadyInstalled { family }) => {
                fl!("already-installed", family = family)
            }
            Err(error) => fl!("install-failed", error = error),
        }
    }

    // ---------------------------------------------------------------- views

    fn back_button() -> Element<'static, Message> {
        widget::button::icon(icon::from_name("go-previous-symbolic"))
            .on_press(Message::Back)
            .into()
    }

    /// "Installed | Google Fonts catalog": both lists one click away, the visible one selected.
    fn tab_switcher(&self) -> Element<'_, Message> {
        widget::segmented_control::horizontal(&self.tabs)
            .on_activate(Message::Tab)
            .width(Length::Shrink)
            .into()
    }

    fn view_browse(&self) -> Element<'_, Message> {
        let space = cosmic::theme::active().cosmic().spacing;

        let search = widget::search_input(fl!("search-placeholder"), &self.search)
            .on_input(Message::Search)
            .on_clear(Message::Search(String::new()));

        let mut list = widget::list_column();
        for &idx in &self.filtered {
            let fam = &self.families[idx];
            let mut meta: Vec<String> = vec![fl!("face-count", count = fam.faces.len())];
            if fam.monospaced() {
                meta.push(fl!("monospace"));
            }
            if fam.has_user_faces() {
                meta.push(fl!("user-installed"));
            }

            list = list.add(
                widget::button::custom(
                    Column::new()
                        .push(widget::text::title4(&fam.name))
                        .push(widget::text::caption(meta.join(" · ")))
                        .spacing(space.space_xxxs),
                )
                .width(Length::Fill)
                .on_press(Message::SelectFamily(idx)),
            );
        }

        let header = Row::new()
            .push(search)
            .push(widget::button::standard(fl!("open-file")).on_press(Message::OpenFileDialog))
            .push(widget::button::standard(fl!("refresh")).on_press(Message::Refresh))
            .spacing(space.space_xs)
            .align_y(Alignment::Center);

        let count = widget::text::caption(if self.loading {
            fl!("scanning")
        } else {
            let by_user = self
                .families
                .iter()
                .filter(|fam| fam.has_user_faces())
                .count();
            let found = (!self.search.is_empty()).then_some(self.filtered.len());
            installed_line(self.families.len(), by_user, found)
        });

        Column::new()
            .push(self.tab_switcher())
            .push(header)
            .push(count)
            .push(
                widget::scrollable(list)
                    .id(self.browse_scroll.clone())
                    .height(Length::Fill),
            )
            .spacing(space.space_s)
            .padding(space.space_s)
            .into()
    }

    fn waterfall(
        &self,
        family: &'static str,
        attrs: Option<(fontdb::Weight, fontdb::Style, fontdb::Stretch)>,
    ) -> Element<'static, Message> {
        let space = cosmic::theme::active().cosmic().spacing;

        let (weight, style, stretch) = match attrs {
            Some((w, s, st)) => (to_iced_weight(w), to_iced_style(s), to_iced_stretch(st)),
            None => (
                iced_font::Weight::Normal,
                iced_font::Style::Normal,
                iced_font::Stretch::Normal,
            ),
        };

        let font = iced_font::Font {
            family: iced_font::Family::Name(family),
            weight,
            style,
            stretch,
        };

        let mut col = Column::new().spacing(space.space_xxs).width(Length::Fill);
        for size in WATERFALL_SIZES {
            col = col.push(
                widget::text(self.preview_text.clone())
                    .font(font)
                    .size(size)
                    .line_height(LineHeight::Relative(WATERFALL_LINE_HEIGHT)),
            );
        }
        col.into()
    }

    /// The waterfall in a full-width scrollable, so the scrollbar sits at the right edge rather
    /// than at the end of the longest line.
    fn waterfall_pane(
        &self,
        family: &'static str,
        attrs: Option<(fontdb::Weight, fontdb::Style, fontdb::Stretch)>,
        height: Length,
    ) -> Element<'static, Message> {
        widget::scrollable(self.waterfall(family, attrs))
            .width(Length::Fill)
            .height(height)
            .into()
    }

    /// The text every preview shows, editable.
    fn preview_input(&self) -> Element<'_, Message> {
        widget::text_input(fl!("preview-placeholder"), &self.preview_text)
            .on_input(Message::PreviewText)
            .into()
    }

    /// The "System" tag for package-manager fonts, which we can't remove. Full-strength text
    /// colour: the toolkit's caption text is too dim to read on every row.
    fn system_badge() -> Element<'static, Message> {
        widget::container(widget::text::body(fl!("system")))
            .padding([1, 10])
            .class(cosmic::theme::Container::custom(|theme| {
                let cosmic = theme.cosmic();
                let text = Color::from(cosmic.on_bg_color());
                iced_container::Style {
                    text_color: Some(text),
                    icon_color: Some(text),
                    background: None,
                    border: Border {
                        color: Color { a: 0.6, ..text },
                        width: 1.0,
                        radius: cosmic.corner_radii.radius_xl.into(),
                    },
                    ..Default::default()
                }
            }))
            .into()
    }

    fn view_family(&self, name: &'static str, status: Option<&String>) -> Element<'_, Message> {
        let space = cosmic::theme::active().cosmic().spacing;
        let Some(fam) = self.families.iter().find(|f| f.name == name) else {
            return self.view_browse();
        };

        // The name takes the space left and wraps, so a long one never pushes the buttons out.
        let mut header = Row::new()
            .push(Self::back_button())
            .push(widget::text::title2(&fam.name).width(Length::Fill))
            .spacing(space.space_xs)
            .align_y(Alignment::Center);
        if let Some(button) = self.uninstall_family_button(&fam.name) {
            header = header.push(button);
        }

        // The waterfall follows the selected face (family default otherwise).
        let selected = self
            .selected_face
            .and_then(|i| fam.faces.get(i).map(|f| (i, f)));
        let attrs = selected.map(|(_, f)| (f.weight, f.style, f.stretch));

        let mut faces = widget::list_column();
        for (i, face) in fam.faces.iter().enumerate() {
            let is_selected = selected.is_some_and(|(sel, _)| sel == i);
            let marker = if is_selected { "●" } else { "○" };
            let label = widget::text::body(format!(
                "{marker}  {}",
                variant_label(face.weight, face.style, face.stretch)
            ));

            let mut row = Row::new()
                .push(widget::button::custom(label).on_press(Message::SelectFace(i)))
                .push(Space::new().width(Length::Fill))
                .spacing(space.space_xs)
                .align_y(Alignment::Center);

            match face.origin {
                Origin::User => {
                    if let Some(button) = self.uninstall_button(face) {
                        row = row.push(button);
                    }
                }
                // System fonts belong to the package manager: no button.
                Origin::System => {
                    row = row.push(Self::system_badge());
                }
            }
            faces = faces.add(row);
        }

        let mut col = Column::new()
            .push(header)
            .push(self.preview_input())
            .push(self.waterfall_pane(name, attrs, Length::FillPortion(3)))
            .push(widget::text::title4(fl!("faces")))
            .push(
                widget::scrollable(faces)
                    .width(Length::Fill)
                    .height(Length::FillPortion(2)),
            )
            .spacing(space.space_s)
            .padding(space.space_s);
        if let Some(status) = status {
            col = col.push(widget::text::body(status.clone()));
        }
        col.into()
    }

    fn view_catalog(&self) -> Element<'_, Message> {
        let space = cosmic::theme::active().cosmic().spacing;

        let search = widget::search_input(fl!("catalog-search-placeholder"), &self.catalog_search)
            .on_input(Message::CatalogSearch)
            .on_clear(Message::CatalogSearch(String::new()));

        let mut col = Column::new()
            .push(self.tab_switcher())
            .push(search)
            .spacing(space.space_s)
            .padding(space.space_s);

        if self.catalog_loading {
            return col.push(widget::text::body(fl!("catalog-loading"))).into();
        }
        if let Some(err) = &self.catalog_error {
            return col
                .push(widget::text::title4(fl!("catalog-unavailable")))
                .push(widget::text::body(err.clone()))
                .push(widget::button::standard(fl!("retry")).on_press(Message::OpenCatalog))
                .into();
        }

        let shown = self.catalog_filtered.len().min(self.catalog_shown);
        let installed_count = self.catalog_installed.iter().filter(|&&yes| yes).count();
        let found = (!self.catalog_search.is_empty()).then_some(self.catalog_filtered.len());
        col = col.push(widget::text::caption(catalog_line(
            self.catalog.len(),
            installed_count,
            found,
        )));

        let mut list = widget::list_column();
        for &idx in self.catalog_filtered.iter().take(shown) {
            let font = &self.catalog[idx];
            let installed = self.catalog_installed.get(idx).copied().unwrap_or(false);

            let mut meta = vec![
                category_label(&font.category),
                fl!("variant-count", count = font.variants.len()),
            ];
            if installed {
                meta.push(fl!("installed-badge"));
            }

            list = list.add(
                widget::button::custom(
                    Column::new()
                        .push(widget::text::title4(&font.family))
                        .push(widget::text::caption(
                            meta.into_iter()
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                                .join(" · "),
                        ))
                        .spacing(space.space_xxxs),
                )
                .width(Length::Fill)
                .on_press(Message::SelectCatalogFont(idx)),
            );
        }
        if self.catalog_filtered.len() > shown {
            list = list.add(
                widget::container(
                    widget::button::standard(fl!(
                        "show-more",
                        shown = shown,
                        total = self.catalog_filtered.len()
                    ))
                    .on_press(Message::ShowMoreCatalog),
                )
                .center_x(Length::Fill),
            );
        }

        col.push(
            widget::scrollable(list)
                .id(self.catalog_scroll.clone())
                .height(Length::Fill),
        )
        .into()
    }

    /// One catalog family: a preview of the selected variant (downloaded, not installed), then
    /// its variants to pick, install or remove.
    fn view_catalog_family<'a>(
        &'a self,
        index: usize,
        variants: Option<&'a Vec<CatalogVariant>>,
        selected: Option<usize>,
        preview: &'a CatalogPreview,
        error: Option<&'a String>,
        status: Option<&'a String>,
    ) -> Element<'a, Message> {
        let space = cosmic::theme::active().cosmic().spacing;
        let Some(font) = self.catalog.get(index) else {
            return self.view_catalog();
        };

        let mut header = Row::new()
            .push(Self::back_button())
            .push(widget::text::title2(&font.family).width(Length::Fill))
            .push(widget::text::caption(category_label(&font.category)))
            .spacing(space.space_xs)
            .align_y(Alignment::Center);
        if let Some(button) = self.uninstall_family_button(&font.family) {
            header = header.push(button);
        }

        let mut col = Column::new()
            .push(header)
            .spacing(space.space_s)
            .padding(space.space_s);

        if let Some(status) = status {
            col = col.push(widget::text::body(status.clone()));
        }

        match (variants, error) {
            (Some(variants), _) => {
                let attrs = selected
                    .and_then(|i| variants.get(i))
                    .map(|v| (v.weight, v.style, fontdb::Stretch::Normal));
                col = col
                    .push(self.preview_input())
                    .push(self.catalog_preview(preview, attrs))
                    .push(widget::text::title4(fl!("faces")))
                    .push(
                        widget::scrollable(self.variant_list(font, variants, selected))
                            .width(Length::Fill)
                            .height(Length::FillPortion(2)),
                    );
            }
            (None, Some(err)) => {
                col = col
                    .push(widget::text::title4(fl!("variants-unavailable")))
                    .push(widget::text::body(err.clone()));
            }
            _ => {
                col = col.push(widget::text::body(fl!("variants-loading")));
            }
        }

        col.into()
    }

    /// The selected variant's waterfall, or a placeholder.
    fn catalog_preview<'a>(
        &self,
        preview: &'a CatalogPreview,
        attrs: Option<(fontdb::Weight, fontdb::Style, fontdb::Stretch)>,
    ) -> Element<'a, Message> {
        let space = cosmic::theme::active().cosmic().spacing;
        // Same height as the waterfall, so the list below doesn't jump.
        let room = |content: Element<'a, Message>| -> Element<'a, Message> {
            widget::container(content)
                .width(Length::Fill)
                .height(Length::FillPortion(3))
                .into()
        };
        match preview {
            CatalogPreview::Ready(name) => self.waterfall_pane(name, attrs, Length::FillPortion(3)),
            CatalogPreview::Loading => room(widget::text::body(fl!("preview-loading")).into()),
            CatalogPreview::Failed(error) => room(
                Column::new()
                    .push(widget::text::title4(fl!("preview-unavailable")))
                    .push(widget::text::body(error.clone()))
                    .spacing(space.space_xxs)
                    .into(),
            ),
            CatalogPreview::Nothing => room(widget::text::body(fl!("no-ttf")).into()),
        }
    }

    /// The variants of a catalog family: click one to preview it. Each has Install, Uninstall
    /// or a System tag.
    fn variant_list<'a>(
        &'a self,
        font: &'a CatalogFont,
        variants: &'a [CatalogVariant],
        selected: Option<usize>,
    ) -> Element<'a, Message> {
        let space = cosmic::theme::active().cosmic().spacing;
        let mut list = widget::list_column();
        for (i, variant) in variants.iter().enumerate() {
            let marker = if selected == Some(i) { "●" } else { "○" };
            let label = widget::text::body(format!(
                "{marker}  {}",
                variant_label(variant.weight, variant.style, fontdb::Stretch::Normal)
            ));
            let mut row = Row::new()
                .push(widget::button::custom(label).on_press(Message::SelectCatalogVariant(i)))
                .push(Space::new().width(Length::Fill))
                .spacing(space.space_xs)
                .align_y(Alignment::Center);

            // Same three-way logic as the file view; gwfh has no stretch axis.
            match self.installed_face(
                &font.family,
                variant.weight,
                variant.style,
                fontdb::Stretch::Normal,
            ) {
                None => match &variant.ttf_url {
                    Some(url) => {
                        row = row.push(widget::button::suggested(fl!("install")).on_press(
                            Message::InstallVariant {
                                url: url.clone(),
                                family: font.family.clone(),
                                variant_id: variant.id.clone(),
                            },
                        ));
                    }
                    None => {
                        row = row.push(widget::text::caption(fl!("no-ttf")));
                    }
                },
                Some(face) if face.origin == Origin::User => {
                    if let Some(button) = self.uninstall_button(face) {
                        row = row.push(button);
                    }
                }
                Some(_) => row = row.push(Self::system_badge()),
            }
            list = list.add(row);
        }
        list.into()
    }

    fn view_file<'a>(
        &'a self,
        path: &'a Path,
        info: Option<&'a FontFileInfo>,
        family: Option<&'static str>,
        error: Option<&'a String>,
        status: Option<&'a String>,
    ) -> Element<'a, Message> {
        let space = cosmic::theme::active().cosmic().spacing;

        let mut col = Column::new().spacing(space.space_s).padding(space.space_s);

        col = col.push(
            Row::new()
                .push(Self::back_button())
                .push(widget::text::caption(path.display().to_string()))
                .spacing(space.space_xs)
                .align_y(Alignment::Center),
        );

        match (info, error) {
            (Some(info), _) => {
                col = col.push(widget::text::title2(&info.family));
                let mut meta = vec![fl!("faces-in-file", count = info.face_count)];
                if info.monospaced {
                    meta.push(fl!("monospace"));
                }
                col = col.push(widget::text::caption(meta.join(" · ")));

                if let Some(family) = family {
                    col = col.push(self.waterfall_pane(
                        family,
                        Some((info.weight, info.style, info.stretch)),
                        Length::Fill,
                    ));
                }

                // Install if absent, Uninstall if a user font, a badge if a system font.
                let action: Element<'a, Message> = match self.installed_face(
                    &info.family,
                    info.weight,
                    info.style,
                    info.stretch,
                ) {
                    None => widget::button::suggested(fl!("install"))
                        .on_press(Message::Install)
                        .into(),
                    Some(face) if face.origin == Origin::User => self
                        .uninstall_button(face)
                        .unwrap_or_else(|| widget::text::body(fl!("installed")).into()),
                    Some(_) => widget::text::body(fl!("already-installed-system")).into(),
                };

                let mut actions = Row::new().spacing(space.space_xs).push(action);
                if let Some(status) = status {
                    actions = actions.push(widget::text::body(status));
                }
                col = col.push(actions.align_y(Alignment::Center));
            }
            (None, Some(err)) => {
                col = col
                    .push(widget::text::title3(fl!("preview-failed")))
                    .push(widget::text::body(err.clone()));
            }
            _ => {
                col = col.push(widget::text::body(fl!("reading-font")));
            }
        }

        col.into()
    }
}

impl Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;

    // Flathub derives the repository from this id: change both together.
    const APP_ID: &'static str = "io.github.linuxboy_96.cosmic-ext-fonts";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Message>) {
        let mut model = Self::new(core, Config::new(Self::APP_ID, CONFIG_VERSION).ok());

        let mut tasks = vec![model.reload_inventory()];
        // The toolkit starts with the desktop's theme.
        if model.theme_choice != ThemeChoice::System {
            tasks.push(cosmic::command::set_theme(model.theme_choice.theme()));
        }
        // libcosmic has two titles: the window's (task bar, alt-tab) is the app name, as in the
        // launcher; the header bar's says what the app is.
        model.set_header_title(fl!("app-title"));
        if let Some(id) = model.core.main_window_id() {
            tasks.push(model.set_window_title(fl!("app-name"), id));
        }

        if let Some(path) = flags.open_path {
            model.view = View::File {
                path: path.clone(),
                info: None,
                family: None,
                error: None,
                status: None,
            };
            tasks.push(Self::probe_file(path));
        }

        (model, Task::batch(tasks))
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(watch_user_fonts)
    }

    /// The menu bar, as in Scan: theme and About.
    fn header_start(&self) -> Vec<Element<'_, Message>> {
        let bar = menu::bar(vec![menu::Tree::with_children(
            Element::from(menu::root(fl!("menu-view"))),
            menu::items(&self.key_binds, self.view_menu()),
        )])
        .item_height(menu::ItemHeight::Dynamic(40))
        .item_width(menu::ItemWidth::Uniform(260))
        .spacing(4.0);
        vec![bar.into()]
    }

    /// The only dialog: why the file chooser couldn't open.
    fn dialog(&self) -> Option<Element<'_, Message>> {
        let Chooser::Failed(error) = &self.chooser else {
            return None;
        };
        Some(
            widget::dialog()
                .title(fl!("chooser-failed"))
                .body(fl!("chooser-failed-body", error = error.as_str()))
                .primary_action(
                    widget::button::standard(fl!("close")).on_press(Message::DismissChooserError),
                )
                .into(),
        )
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        self.core.window.show_context.then(|| {
            about::drawer(
                fl!("app-name"),
                &self.about_icon,
                |url| Message::OpenUrl(url.to_owned()),
                Message::ToggleAbout,
            )
        })
    }

    /// Esc closes the About page, as in Scan.
    fn on_escape(&mut self) -> Task<Message> {
        if self.core.window.show_context {
            self.set_show_context(false);
        }
        Task::none()
    }

    // One flat `match` per message is the iced idiom; splitting it hurts more.
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InventoryLoaded(seq, families) => {
                if seq != self.reload_seq {
                    return Task::none();
                }
                self.file_faces.clear();
                for path in families
                    .iter()
                    .flat_map(|fam| &fam.faces)
                    .filter_map(|face| face.path.as_ref())
                {
                    *self.file_faces.entry(path.clone()).or_default() += 1;
                }
                let unseen = inventory::files_to_register(&families, &mut self.renderer_faces);
                self.families = families;
                self.loading = false;
                self.refilter();
                self.refresh_catalog_installed();
                // The open family may have vanished (its last face uninstalled).
                let gone = matches!(
                    &self.view,
                    View::Family { name, .. } if !self.families.iter().any(|f| f.name == *name)
                );
                if gone {
                    self.view = View::Browse;
                    self.selected_face = None;
                }
                if self.pending_is_moot() {
                    self.pending_uninstall = None;
                }
                // The first inventory only seeds `renderer_faces` (the renderer scanned those
                // fonts itself); later ones bring new fonts to register.
                if std::mem::replace(&mut self.renderer_seeded, true) && !unseen.is_empty() {
                    return Self::register_fonts(unseen);
                }
            }
            Message::Search(input) => {
                self.search = input;
                self.refilter();
                return Self::scroll_to_top(&self.browse_scroll);
            }
            Message::SelectFamily(idx) => {
                if let Some(fam) = self.families.get(idx) {
                    let name = fam.name.clone();
                    let name = self.intern(&name);
                    self.view = View::Family { name, status: None };
                    self.selected_face = None;
                    self.pending_uninstall = None;
                }
            }
            Message::SelectFace(i) => {
                // Toggle: re-clicking the selected face returns to the default.
                self.selected_face = if self.selected_face == Some(i) {
                    None
                } else {
                    Some(i)
                };
            }
            Message::Back => {
                // A catalog family goes back to the catalog, anything else to the installed list.
                let list = if matches!(self.view, View::CatalogFamily { .. }) {
                    Tab::Catalog
                } else {
                    Tab::Installed
                };
                return self.show_tab(list);
            }
            Message::PreviewText(text) => {
                self.preview_text = text;
            }
            Message::OpenFileDialog => {
                // The chooser is the desktop's window and the app keeps running, so a second
                // click mustn't open a second one.
                if self.chooser != Chooser::Open {
                    self.chooser = Chooser::Open;
                    return Self::open_chooser();
                }
            }
            Message::ChooserAnswered(answer) => match answer {
                Ok(answer) => {
                    self.chooser = Chooser::Closed;
                    if let Some(path) = answer {
                        return self.update(Message::FileChosen(path));
                    }
                }
                Err(error) => self.chooser = Chooser::Failed(error),
            },
            Message::DismissChooserError => {
                if matches!(self.chooser, Chooser::Failed(_)) {
                    self.chooser = Chooser::Closed;
                }
            }
            Message::FileChosen(path) => {
                self.view = View::File {
                    path: path.clone(),
                    info: None,
                    family: None,
                    error: None,
                    status: None,
                };
                self.pending_uninstall = None;
                return Self::probe_file(path);
            }
            Message::ProbedFile(path, result) => {
                // An answer for a file the user has already left is dropped.
                if !matches!(&self.view, View::File { path: shown, .. } if *shown == path) {
                    return Task::none();
                }
                match result {
                    Ok(faces) => {
                        let Some(first) = faces.first().cloned() else {
                            return Task::none();
                        };
                        let name = self.intern(&first.family);
                        // The waterfall needs the renderer to have the font. Skip faces it
                        // already has (installed or previewed): fonts are never unloaded.
                        let mut unseen = false;
                        for face in &faces {
                            unseen |= self.renderer_faces.insert(face.key());
                        }
                        if let View::File {
                            info,
                            family,
                            error,
                            ..
                        } = &mut self.view
                        {
                            *info = Some(first);
                            *family = Some(name);
                            *error = None;
                        }
                        if unseen {
                            return Self::register_fonts(vec![path]);
                        }
                    }
                    Err(e) => {
                        if let View::File { error, .. } = &mut self.view {
                            *error = Some(e);
                        }
                    }
                }
            }
            Message::FontsRead(data) => {
                return Task::batch(data.0.into_iter().map(|bytes| {
                    iced_font::load(bytes).map(|res| cosmic::Action::App(Message::FontLoaded(res)))
                }));
            }
            // A font registered: nothing to change.
            Message::FontLoaded(_) => {}
            Message::Install => {
                if let View::File { path, .. } = &self.view {
                    let path = path.clone();
                    return blocking(
                        move || {
                            let inventory = Inventory::load_system();
                            installer::install(&path, &inventory)
                                .map_err(|e| install_error_text(&e))
                        },
                        Message::Installed,
                    );
                }
            }
            Message::Installed(result) | Message::CatalogInstalled(result) => {
                self.set_status(Self::install_status(result));
                return self.reload_inventory();
            }
            Message::UninstallFace(path, index) => {
                self.pending_uninstall = Some(Pending::Face(path, index));
            }
            Message::UninstallFamily(name) => {
                self.pending_uninstall = Some(Pending::Family(name));
            }
            Message::CancelUninstall => {
                self.pending_uninstall = None;
            }
            Message::ConfirmUninstall => {
                let Some(Pending::Face(path, _)) = self.pending_uninstall.take() else {
                    return Task::none();
                };
                return blocking(
                    move || installer::uninstall(&path).map_err(|e| install_error_text(&e)),
                    Message::Uninstalled,
                );
            }
            Message::ConfirmUninstallFamily => {
                let Some(Pending::Family(name)) = self.pending_uninstall.take() else {
                    return Task::none();
                };
                let files = self.user_files(&name);
                return blocking(
                    move || {
                        // Stop at the first failure and report it; the reloaded list shows
                        // what's left.
                        for path in &files {
                            installer::uninstall(path).map_err(|e| install_error_text(&e))?;
                        }
                        Ok(())
                    },
                    Message::Uninstalled,
                );
            }
            Message::Uninstalled(result) => {
                self.set_status(match result {
                    Ok(()) => fl!("uninstalled"),
                    Err(error) => fl!("uninstall-failed", error = error),
                });
                return self.reload_inventory();
            }
            Message::Refresh => {
                self.loading = true;
                return self.reload_inventory();
            }
            Message::FontsChanged => {
                return self.reload_inventory();
            }
            Message::Tab(entity) => {
                if let Some(&tab) = self.tabs.data::<Tab>(entity) {
                    return self.show_tab(tab);
                }
            }
            Message::OpenCatalog => return self.show_tab(Tab::Catalog),
            Message::ToggleAbout => {
                let show = !self.core.window.show_context;
                self.set_show_context(show);
            }
            Message::SetTheme(choice) => return self.set_theme(choice),
            Message::OpenUrl(url) => return open_url(url),
            Message::CatalogLoaded(result) => {
                self.catalog_loading = false;
                match result {
                    Ok(fonts) => {
                        self.catalog = fonts;
                        self.refilter_catalog();
                        self.refresh_catalog_installed();
                    }
                    Err(e) => self.catalog_error = Some(e),
                }
            }
            Message::CatalogSearch(input) => {
                self.catalog_search = input;
                self.catalog_shown = CATALOG_PAGE;
                self.refilter_catalog();
                return Self::scroll_to_top(&self.catalog_scroll);
            }
            Message::ShowMoreCatalog => {
                self.catalog_shown += CATALOG_PAGE;
            }
            Message::SelectCatalogFont(index) => {
                if let Some(font) = self.catalog.get(index) {
                    let id = font.id.clone();
                    self.view = View::CatalogFamily {
                        index,
                        variants: None,
                        selected: None,
                        preview: CatalogPreview::Nothing,
                        error: None,
                        status: None,
                    };
                    self.pending_uninstall = None;
                    return Self::fetch_variants_task(id);
                }
            }
            Message::CatalogVariantsLoaded(id, result) => {
                let mut first = None;
                // Only the family on screen takes the answer; a late one for a family the
                // user has left is dropped.
                if let View::CatalogFamily {
                    index,
                    variants,
                    error,
                    ..
                } = &mut self.view
                    && self.catalog.get(*index).is_some_and(|font| font.id == id)
                {
                    match result {
                        Ok(list) => {
                            first = default_variant(&list);
                            *variants = Some(list);
                        }
                        Err(e) => *error = Some(e),
                    }
                }
                // Open on a variant so a preview shows right away.
                if let Some(position) = first {
                    return self.select_variant(position);
                }
            }
            Message::SelectCatalogVariant(position) => return self.select_variant(position),
            Message::CatalogPreviewLoaded(url, result) => {
                return self.preview_loaded(&url, result);
            }
            Message::InstallVariant {
                url,
                family,
                variant_id,
            } => {
                self.set_status(fl!(
                    "downloading",
                    family = family.clone(),
                    variant = variant_id.clone()
                ));
                return blocking(
                    move || {
                        // Private temp dir, then the same verified pipeline as a local file.
                        let download = catalog::download_ttf(&url, &family, &variant_id)
                            .map_err(|e| catalog_error_text(&e))?;
                        let inventory = Inventory::load_system();
                        installer::install(&download.path, &inventory)
                            .map_err(|e| install_error_text(&e))
                    },
                    Message::CatalogInstalled,
                );
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match &self.view {
            View::Browse => self.view_browse(),
            View::Family { name, status } => self.view_family(name, status.as_ref()),
            View::File {
                path,
                info,
                family,
                error,
                status,
            } => self.view_file(
                path,
                info.as_ref(),
                *family,
                error.as_ref(),
                status.as_ref(),
            ),
            View::Catalog => self.view_catalog(),
            View::CatalogFamily {
                index,
                variants,
                selected,
                preview,
                error,
                status,
            } => self.view_catalog_family(
                *index,
                variants.as_ref(),
                *selected,
                preview,
                error.as_ref(),
                status.as_ref(),
            ),
        }
    }

    // TODO(v0.2): drag-and-drop of font files.
}

// --------------------------------------------------------- fontdb → iced

fn to_iced_weight(w: fontdb::Weight) -> iced_font::Weight {
    use iced_font::Weight as W;
    match inventory::weight_class(w) {
        WeightClass::Thin => W::Thin,
        WeightClass::ExtraLight => W::ExtraLight,
        WeightClass::Light => W::Light,
        WeightClass::Regular => W::Normal,
        WeightClass::Medium => W::Medium,
        WeightClass::SemiBold => W::Semibold,
        WeightClass::Bold => W::Bold,
        WeightClass::ExtraBold => W::ExtraBold,
        WeightClass::Black => W::Black,
    }
}

fn to_iced_style(s: fontdb::Style) -> iced_font::Style {
    match s {
        fontdb::Style::Normal => iced_font::Style::Normal,
        fontdb::Style::Italic => iced_font::Style::Italic,
        fontdb::Style::Oblique => iced_font::Style::Oblique,
    }
}

fn to_iced_stretch(s: fontdb::Stretch) -> iced_font::Stretch {
    use fontdb::Stretch as F;
    use iced_font::Stretch as I;
    match s {
        F::UltraCondensed => I::UltraCondensed,
        F::ExtraCondensed => I::ExtraCondensed,
        F::Condensed => I::Condensed,
        F::SemiCondensed => I::SemiCondensed,
        F::Normal => I::Normal,
        F::SemiExpanded => I::SemiExpanded,
        F::Expanded => I::Expanded,
        F::ExtraExpanded => I::ExtraExpanded,
        F::UltraExpanded => I::UltraExpanded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontdb::{Stretch, Style, Weight};

    // Nothing here calls `localize()`, so labels come out in English.

    #[test]
    fn a_face_is_named_by_what_sets_it_apart() {
        let name = variant_label;
        assert_eq!(
            name(Weight::NORMAL, Style::Normal, Stretch::Normal),
            "Regular"
        );
        assert_eq!(
            name(Weight::BOLD, Style::Italic, Stretch::Normal),
            "Bold Italic"
        );
        assert_eq!(
            name(Weight::NORMAL, Style::Oblique, Stretch::Normal),
            "Oblique"
        );
        assert_eq!(
            name(Weight::LIGHT, Style::Normal, Stretch::Condensed),
            "Light Condensed"
        );
        assert_eq!(
            name(Weight(800), Style::Italic, Stretch::SemiExpanded),
            "Extra Bold Italic Semi Expanded"
        );
    }

    #[test]
    fn english_labels_match_the_core_names() {
        for value in (50..=950).step_by(50) {
            let weight = Weight(value);
            let class = inventory::weight_class(weight);
            if class != WeightClass::Regular {
                assert_eq!(
                    weight_label(class),
                    inventory::weight_name(weight),
                    "weight {value}"
                );
            }
        }
        for stretch in [
            Stretch::UltraCondensed,
            Stretch::ExtraCondensed,
            Stretch::Condensed,
            Stretch::SemiCondensed,
            Stretch::Normal,
            Stretch::SemiExpanded,
            Stretch::Expanded,
            Stretch::ExtraExpanded,
            Stretch::UltraExpanded,
        ] {
            assert_eq!(
                stretch_label(stretch).unwrap_or_default(),
                inventory::stretch_name(stretch),
                "{stretch:?}"
            );
        }
    }

    #[test]
    fn the_renderer_gets_the_weight_the_label_names() {
        use iced_font::Weight as W;

        let expected = [
            (100, W::Thin),
            (200, W::ExtraLight),
            (300, W::Light),
            (400, W::Normal),
            (500, W::Medium),
            (600, W::Semibold),
            (700, W::Bold),
            (800, W::ExtraBold),
            (900, W::Black),
        ];
        for (value, want) in expected {
            assert_eq!(to_iced_weight(Weight(value)), want, "weight {value}");
        }
        // Between two steps the nearer name wins, as in the labels.
        assert_eq!(to_iced_weight(Weight(449)), W::Normal);
        assert_eq!(to_iced_weight(Weight(450)), W::Medium);
    }

    /// Strips Fluent's invisible isolation marks so text can be compared.
    fn plain(text: &str) -> String {
        text.replace(['\u{2068}', '\u{2069}'], "")
    }

    #[test]
    fn the_installed_line_says_what_its_number_counts() {
        assert_eq!(
            plain(&installed_line(269, 12, None)),
            "269 families on this computer · 12 user-installed"
        );
        // Nothing user-installed: no empty "0 user-installed".
        assert_eq!(
            plain(&installed_line(1, 0, None)),
            "1 family on this computer"
        );
        // While searching: the matches out of the total.
        assert_eq!(
            plain(&installed_line(269, 12, Some(7))),
            "7 of 269 families"
        );
        assert_eq!(plain(&installed_line(1, 1, Some(1))), "1 of 1 family");
        assert_eq!(
            plain(&installed_line(269, 12, Some(0))),
            "0 of 269 families"
        );
    }

    #[test]
    fn the_catalog_line_says_where_its_families_come_from() {
        assert_eq!(
            plain(&catalog_line(1959, 37, None)),
            "1959 families on Google Fonts · 37 already installed"
        );
        assert_eq!(
            plain(&catalog_line(1959, 0, None)),
            "1959 families on Google Fonts"
        );
        assert_eq!(
            plain(&catalog_line(1959, 37, Some(12))),
            "12 of 1959 families"
        );
    }

    #[test]
    fn show_more_says_how_far_the_list_goes() {
        assert_eq!(
            plain(&fl!("show-more", shown = 200, total = 1959)),
            "Show more (200 of 1959)"
        );
    }

    #[test]
    fn the_switcher_has_installed_then_catalog_and_starts_on_installed() {
        let mut model = tab_model();
        let installed = tab_entity(&model, Tab::Installed).expect("an Installed tab");
        let catalog = tab_entity(&model, Tab::Catalog).expect("a Catalog tab");

        assert_eq!(model.text(installed), Some("Installed"));
        assert_eq!(model.text(catalog), Some("Google Fonts catalog"));
        assert_eq!(model.position(installed), Some(0));
        assert!(model.is_active(installed) && !model.is_active(catalog));

        // `show_tab` relies on this: selecting one entry deselects the other.
        model.activate(catalog);
        assert!(model.is_active(catalog) && !model.is_active(installed));
    }

    #[test]
    fn catalog_categories_are_translated_and_unknown_ones_kept() {
        assert_eq!(category_label("sans-serif"), "Sans serif");
        assert_eq!(category_label("monospace"), "Monospace");
        assert_eq!(category_label("klingon"), "klingon");
    }

    // Fixtures -----------------------------------------------------------------
    //
    // No window, renderer or network: tasks are only counted (`units`), never run. A `Core`
    // still reads the desktop's theme settings, as in the app.

    const REGULAR_URL: &str = "https://fonts.gstatic.com/s/roboto/v1/regular.ttf";
    const BOLD_URL: &str = "https://fonts.gstatic.com/s/roboto/v1/bold.ttf";

    fn model() -> AppModel {
        AppModel::new(Core::default(), None)
    }

    fn variant(id: &str, weight: u16, style: Style, url: Option<&str>) -> CatalogVariant {
        CatalogVariant {
            id: id.into(),
            weight: Weight(weight),
            style,
            ttf_url: url.map(str::to_owned),
        }
    }

    fn catalog_font(id: &str, family: &str) -> CatalogFont {
        CatalogFont {
            id: id.into(),
            family: family.into(),
            category: "sans-serif".into(),
            variants: Vec::new(),
        }
    }

    /// A model showing the catalog family "Roboto" with these variants.
    fn on_roboto(variants: Vec<CatalogVariant>) -> AppModel {
        let mut model = model();
        model.catalog = vec![catalog_font("roboto", "Roboto")];
        model.view = View::CatalogFamily {
            index: 0,
            variants: Some(variants),
            selected: None,
            preview: CatalogPreview::Nothing,
            error: None,
            status: None,
        };
        model
    }

    fn preview_of(model: &AppModel) -> &CatalogPreview {
        match &model.view {
            View::CatalogFamily { preview, .. } => preview,
            _ => panic!("not on a catalog family"),
        }
    }

    /// What arrives when a variant has been downloaded.
    fn downloaded(family: &str, weight: u16) -> CatalogPreviewData {
        CatalogPreviewData {
            faces: vec![FontFileInfo {
                family: family.into(),
                weight: Weight(weight),
                style: Style::Normal,
                stretch: Stretch::Normal,
                monospaced: false,
                face_count: 1,
            }],
            // Nothing draws in these tests: the bytes only have to exist.
            fonts: FontData(vec![vec![0]]),
        }
    }

    fn face(family: &str, weight: u16, origin: Origin, file: &str, index: u32) -> FaceEntry {
        FaceEntry {
            id: fontdb::ID::dummy(),
            family: family.into(),
            post_script_name: String::new(),
            style: Style::Normal,
            weight: Weight(weight),
            stretch: Stretch::Normal,
            monospaced: false,
            path: Some(PathBuf::from(file)),
            index,
            origin,
        }
    }

    const USER_DIR: &str = "/home/u/.local/share/fonts";

    /// One family with a collection (two faces, one file), a single user file and a system
    /// file; and a second family that's all system.
    fn installed_families() -> Vec<FamilyEntry> {
        let user = |name: &str| format!("{USER_DIR}/{name}");
        vec![
            FamilyEntry {
                name: "Mixed".into(),
                faces: vec![
                    face("Mixed", 400, Origin::User, &user("Mixed.ttc"), 0),
                    face("Mixed", 700, Origin::User, &user("Mixed.ttc"), 1),
                    face("Mixed", 300, Origin::User, &user("Mixed-Light.ttf"), 0),
                    face(
                        "Mixed",
                        900,
                        Origin::System,
                        "/usr/share/fonts/Mixed-Black.ttf",
                        0,
                    ),
                ],
            },
            FamilyEntry {
                name: "Plain".into(),
                faces: vec![face(
                    "Plain",
                    400,
                    Origin::System,
                    "/usr/share/fonts/Plain.ttf",
                    0,
                )],
            },
        ]
    }

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
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // Catalog previews ---------------------------------------------------------

    #[test]
    fn a_catalog_family_opens_on_its_regular_variant() {
        let v = |weight, style| variant("x", weight, style, None);

        assert_eq!(default_variant(&[]), None);
        // Wherever the regular one is in the list...
        let list = [
            v(700, Style::Normal),
            v(400, Style::Italic),
            v(400, Style::Normal),
        ];
        assert_eq!(default_variant(&list), Some(2));
        // ...and the first one when there is none.
        assert_eq!(
            default_variant(&[v(700, Style::Normal), v(300, Style::Normal)]),
            Some(0)
        );
    }

    #[test]
    fn selecting_a_variant_previews_it_without_installing_it() {
        let mut model = on_roboto(vec![
            variant("regular", 400, Style::Normal, Some(REGULAR_URL)),
            variant("100", 100, Style::Normal, None),
        ]);

        // Not installed, not downloaded yet: the download starts.
        let task = model.select_variant(0);
        assert_eq!(*preview_of(&model), CatalogPreview::Loading);
        assert_eq!(task.units(), 1);

        // The file arrives: the renderer gets it (once), and it is on show.
        let task = model.preview_loaded(REGULAR_URL, Ok(downloaded("Roboto", 400)));
        assert_eq!(*preview_of(&model), CatalogPreview::Ready("Roboto"));
        assert_eq!(task.units(), 1);
        assert_eq!(model.previews.get(REGULAR_URL), Some(&"Roboto"));
        assert!(model.families.is_empty(), "a preview installs nothing");

        // The same file arriving twice (two requests in flight) isn't given to the renderer again.
        let task = model.preview_loaded(REGULAR_URL, Ok(downloaded("Roboto", 400)));
        assert_eq!(task.units(), 0);

        // A variant without a TTF has nothing to show, and nothing to fetch.
        let task = model.select_variant(1);
        assert_eq!(*preview_of(&model), CatalogPreview::Nothing);
        assert_eq!(task.units(), 0);

        // Back to the first: it is in memory already, no second download.
        let task = model.select_variant(0);
        assert_eq!(*preview_of(&model), CatalogPreview::Ready("Roboto"));
        assert_eq!(task.units(), 0);
    }

    #[test]
    fn an_installed_variant_previews_from_the_installed_font() {
        let mut model = on_roboto(vec![
            variant("regular", 400, Style::Normal, Some(REGULAR_URL)),
            variant("700", 700, Style::Normal, Some(BOLD_URL)),
        ]);
        model.families = vec![FamilyEntry {
            name: "roboto".into(),
            faces: vec![face(
                "roboto",
                400,
                Origin::System,
                "/usr/share/fonts/R.ttf",
                0,
            )],
        }];

        // The installed face needs no download: the renderer has it already.
        let task = model.select_variant(0);
        assert_eq!(*preview_of(&model), CatalogPreview::Ready("roboto"));
        assert_eq!(task.units(), 0);

        // Another face of the same family is not installed: that one is fetched.
        let task = model.select_variant(1);
        assert_eq!(*preview_of(&model), CatalogPreview::Loading);
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn a_late_answer_for_a_variant_the_user_left_is_kept_but_not_shown() {
        let mut model = on_roboto(vec![
            variant("regular", 400, Style::Normal, Some(REGULAR_URL)),
            variant("700", 700, Style::Normal, Some(BOLD_URL)),
        ]);
        let _ = model.select_variant(0);
        let _ = model.select_variant(1);

        // The first variant's file arrives while the second is on show.
        let _ = model.preview_loaded(REGULAR_URL, Ok(downloaded("Roboto", 400)));
        assert_eq!(*preview_of(&model), CatalogPreview::Loading);
        assert!(model.previews.contains_key(REGULAR_URL));

        // Going back to it costs nothing.
        let task = model.select_variant(0);
        assert_eq!(*preview_of(&model), CatalogPreview::Ready("Roboto"));
        assert_eq!(task.units(), 0);
    }

    #[test]
    fn a_failed_preview_says_so_and_the_next_try_downloads_again() {
        let mut model = on_roboto(vec![variant(
            "regular",
            400,
            Style::Normal,
            Some(REGULAR_URL),
        )]);
        let _ = model.select_variant(0);

        let _ = model.preview_loaded(REGULAR_URL, Err("no network".into()));
        assert_eq!(
            *preview_of(&model),
            CatalogPreview::Failed("no network".into())
        );
        assert!(!model.previews.contains_key(REGULAR_URL));

        // A file that holds no font is a failure too.
        let empty = CatalogPreviewData {
            faces: Vec::new(),
            fonts: FontData::default(),
        };
        let _ = model.preview_loaded(REGULAR_URL, Ok(empty));
        assert!(matches!(preview_of(&model), CatalogPreview::Failed(_)));

        let task = model.select_variant(0);
        assert_eq!(*preview_of(&model), CatalogPreview::Loading);
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn opening_a_catalog_family_previews_its_regular_variant_at_once() {
        let mut model = model();
        model.catalog = vec![catalog_font("roboto", "Roboto")];

        // The list of variants is asked for; nothing to show meanwhile.
        let task = model.update(Message::SelectCatalogFont(0));
        assert_eq!(task.units(), 1);
        assert_eq!(*preview_of(&model), CatalogPreview::Nothing);

        // It arrives, bold first: the regular one is chosen, and its file fetched.
        let variants = vec![
            variant("700", 700, Style::Normal, Some(BOLD_URL)),
            variant("regular", 400, Style::Normal, Some(REGULAR_URL)),
        ];
        let task = model.update(Message::CatalogVariantsLoaded(
            "roboto".into(),
            Ok(variants),
        ));
        assert!(matches!(
            &model.view,
            View::CatalogFamily {
                selected: Some(1),
                ..
            }
        ));
        assert_eq!(*preview_of(&model), CatalogPreview::Loading);
        assert_eq!(task.units(), 1);

        // An answer for a family no longer on screen is ignored.
        let variants = vec![variant("regular", 400, Style::Normal, Some(REGULAR_URL))];
        let task = model.update(Message::CatalogVariantsLoaded("lato".into(), Ok(variants)));
        assert_eq!(task.units(), 0);
    }

    // Removing a family --------------------------------------------------------

    #[test]
    fn family_names_differing_by_case_are_one_family() {
        assert!(same_family("DejaVu Sans", "dejavu sans"));
        assert!(same_family("Ünder Sans", "ünder sans"));
        assert!(!same_family("DejaVu Sans", "DejaVu Sans Mono"));
    }

    #[test]
    fn removing_a_family_concerns_only_the_files_the_user_installed() {
        let mut model = model();
        model.families = installed_families();

        // Each file once (the collection holds two faces), system files left out, any case.
        assert_eq!(
            model.user_files("MIXED"),
            [
                PathBuf::from(format!("{USER_DIR}/Mixed.ttc")),
                PathBuf::from(format!("{USER_DIR}/Mixed-Light.ttf")),
            ]
        );
        assert!(model.user_files("Plain").is_empty());
        assert!(model.user_files("Nobody").is_empty());

        // The button only shows when there's something to remove.
        assert!(model.uninstall_family_button("Mixed").is_some());
        assert!(model.uninstall_family_button("Plain").is_none());
        assert!(model.uninstall_family_button("Nobody").is_none());
    }

    #[test]
    fn removing_a_family_waits_for_its_confirmation() {
        let mut model = model();
        model.families = installed_families();
        model
            .file_faces
            .insert(PathBuf::from(format!("{USER_DIR}/Mixed.ttc")), 2);

        // The first click asks; nothing is removed.
        let task = model.update(Message::UninstallFamily("mixed".into()));
        assert_eq!(task.units(), 0);
        assert_eq!(
            model.pending_uninstall,
            Some(Pending::Family("mixed".into()))
        );
        assert!(model.uninstall_family_button("Mixed").is_some());

        // Cancel puts the plain button back, and a stray confirmation does nothing.
        let _ = model.update(Message::CancelUninstall);
        assert_eq!(model.pending_uninstall, None);
        assert_eq!(model.update(Message::ConfirmUninstallFamily).units(), 0);

        // One question at a time: the latest one stands.
        let _ = model.update(Message::UninstallFace(PathBuf::from("/x.ttf"), 0));
        let _ = model.update(Message::UninstallFamily("mixed".into()));
        assert_eq!(
            model.pending_uninstall,
            Some(Pending::Family("mixed".into()))
        );

        // Confirmed: the removal goes to the pool and the question is gone.
        assert_eq!(model.update(Message::ConfirmUninstallFamily).units(), 1);
        assert_eq!(model.pending_uninstall, None);
    }

    #[test]
    fn a_question_about_something_already_gone_is_dropped() {
        let mut model = model();
        model.families = installed_families();
        model.pending_uninstall = Some(Pending::Family("mixed".into()));
        assert!(!model.pending_is_moot());

        // The inventory reloads without the user's files (removed elsewhere).
        let system_only = installed_families().split_off(1);
        let _ = model.update(Message::InventoryLoaded(model.reload_seq, system_only));
        assert_eq!(model.pending_uninstall, None);
    }

    // Theme --------------------------------------------------------------------

    #[test]
    fn the_theme_choice_is_a_word_and_anything_else_is_the_default() {
        for choice in ThemeChoice::ALL {
            assert_eq!(ThemeChoice::from_key(choice.key()), choice);
        }
        // An empty or hand-edited file must not keep the app from starting.
        for junk in ["", "sepia", "Dark", " light"] {
            assert_eq!(ThemeChoice::from_key(junk), ThemeChoice::System, "{junk:?}");
        }
        assert_eq!(ThemeChoice::default(), ThemeChoice::System);
    }

    #[test]
    fn a_pinned_theme_keeps_its_mode_whatever_the_desktop_answers() {
        use cosmic::cosmic_theme::{ThemeBuilder, palette::Srgb};
        use cosmic::theme::ThemeType;

        // With no theme for that mode the desktop answers with the dark default: use the
        // built-in palette.
        let light = pin(&cosmic::Theme::dark(), false);
        let dark = pin(&cosmic::Theme::light(), true);
        assert!(!light.cosmic().is_dark);
        assert!(dark.cosmic().is_dark);

        // The desktop's own theme of the mode (here with a red accent) is kept.
        let own = ThemeBuilder::light()
            .accent(Srgb::new(0.8, 0.1, 0.1))
            .build();
        assert!(!own.is_dark);
        let kept = pin(&cosmic::Theme::system(Arc::new(own.clone())), false);
        assert_eq!(kept.cosmic().accent_color(), own.accent_color());
        assert_ne!(
            kept.cosmic().accent_color(),
            cosmic::cosmic_theme::Theme::light_default().accent_color()
        );

        // Not a system theme: libcosmic re-applies those to follow the desktop.
        for pinned in [&light, &dark, &kept] {
            assert!(matches!(pinned.theme_type, ThemeType::Custom(_)));
        }
    }

    #[test]
    fn the_settings_file_keeps_the_theme_choice() {
        let scratch = Scratch::new("settings");
        let config =
            || Config::with_custom_path("test.Fonts", CONFIG_VERSION, scratch.0.clone()).unwrap();

        let mut model = AppModel::new(Core::default(), Some(config()));
        assert_eq!(
            model.theme_choice,
            ThemeChoice::System,
            "nothing is kept yet"
        );

        let task = model.update(Message::SetTheme(ThemeChoice::Dark));
        assert!(
            task.units() > 0,
            "the theme is applied now, not at the next start"
        );
        assert_eq!(model.theme_choice, ThemeChoice::Dark);
        assert_eq!(ticked(&model), [ThemeChoice::Dark]);

        // The next run starts with it, and its menu ticks it.
        let next = AppModel::new(Core::default(), Some(config()));
        assert_eq!(next.theme_choice, ThemeChoice::Dark);
        assert_eq!(ticked(&next), [ThemeChoice::Dark]);

        // An unknown value in the file: the default.
        config().set(THEME_KEY, "sepia").unwrap();
        let odd = AppModel::new(Core::default(), Some(config()));
        assert_eq!(odd.theme_choice, ThemeChoice::System);

        // Nowhere to save: the choice still applies, it just isn't saved.
        let mut homeless = AppModel::new(Core::default(), None);
        assert!(
            homeless
                .update(Message::SetTheme(ThemeChoice::Light))
                .units()
                > 0
        );
        assert_eq!(homeless.theme_choice, ThemeChoice::Light);
        assert_eq!(ticked(&homeless), [ThemeChoice::Light]);
    }

    /// The ticked choices in View > Theme.
    fn ticked(model: &AppModel) -> Vec<ThemeChoice> {
        let items = model.view_menu();
        let menu::Item::Folder(_, choices) = &items[0] else {
            panic!("the first entry of the View menu is the Theme submenu");
        };
        choices
            .iter()
            .filter_map(|item| match item {
                menu::Item::CheckBox(_, _, true, MenuAction::Theme(choice)) => Some(*choice),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_menu_bar_holds_the_theme_and_the_about_page() {
        let model = model();
        // As in Scan: the menus sit at the start of the header bar, and there's no gear.
        assert_eq!(model.header_start().len(), 1);
        assert!(model.header_end().is_empty());

        let items = model.view_menu();
        assert_eq!(items.len(), 3);
        let menu::Item::Folder(name, choices) = &items[0] else {
            panic!("the first entry is the Theme submenu");
        };
        assert_eq!(name, "Theme");
        // One entry per choice, in order, each sending its own choice.
        let sent: Vec<_> = choices
            .iter()
            .map(|item| match item {
                menu::Item::CheckBox(label, _, _, MenuAction::Theme(choice)) => {
                    assert_eq!(*label, choice.label());
                    *choice
                }
                _ => panic!("the submenu holds nothing but the choices"),
            })
            .collect();
        assert_eq!(sent, ThemeChoice::ALL);
        assert!(matches!(items[1], menu::Item::Divider));
        let menu::Item::Button(name, _, action) = &items[2] else {
            panic!("the last entry is the About page");
        };
        assert_eq!(name, "About");
        assert_eq!(*action, MenuAction::About);
    }

    #[test]
    fn the_menu_ticks_the_theme_in_force_and_only_that_one() {
        let mut model = model();
        assert_eq!(ticked(&model), [ThemeChoice::System]);
        for choice in ThemeChoice::ALL {
            let _ = model.update(Message::SetTheme(choice));
            assert_eq!(ticked(&model), [choice]);
        }
    }

    #[test]
    fn the_entries_of_the_menu_send_the_messages_they_stand_for() {
        use menu::action::MenuAction as _;
        assert!(matches!(MenuAction::About.message(), Message::ToggleAbout));
        for choice in ThemeChoice::ALL {
            assert!(matches!(
                MenuAction::Theme(choice).message(),
                Message::SetTheme(sent) if sent == choice
            ));
        }
    }

    #[test]
    fn the_about_entry_opens_and_closes_the_about_drawer() {
        let mut model = model();
        assert!(model.context_drawer().is_none());

        let _ = model.update(Message::ToggleAbout);
        let drawer = model.context_drawer().expect("About opens the drawer");
        assert_eq!(drawer.title.as_deref(), Some("About"));
        // The drawer's close button sends the same message as the menu entry.
        assert!(matches!(drawer.on_close, Message::ToggleAbout));
        drop(drawer);

        let _ = model.update(Message::ToggleAbout);
        assert!(model.context_drawer().is_none());

        // Esc closes it too, as in Scan, and does nothing when it is closed.
        let _ = model.update(Message::ToggleAbout);
        let _ = model.on_escape();
        assert!(model.context_drawer().is_none());
        let _ = model.on_escape();
        assert!(model.context_drawer().is_none());
    }

    // The file chooser --------------------------------------------------------

    #[test]
    fn the_file_chooser_is_asked_once_and_its_failure_is_told() {
        let mut model = model();

        // The first click asks the desktop; a second one while the chooser is open asks nothing.
        assert_eq!(model.update(Message::OpenFileDialog).units(), 1);
        assert_eq!(model.update(Message::OpenFileDialog).units(), 0);

        // Closed without a choice: nothing changes, and the button asks again.
        assert_eq!(model.update(Message::ChooserAnswered(Ok(None))).units(), 0);
        assert!(model.dialog().is_none());
        assert!(matches!(model.view, View::Browse));
        assert_eq!(model.update(Message::OpenFileDialog).units(), 1);

        // No chooser at all: a dialog says why, until it is closed.
        let _ = model.update(Message::ChooserAnswered(Err("no portal".into())));
        assert!(model.dialog().is_some());
        let _ = model.update(Message::DismissChooserError);
        assert!(model.dialog().is_none());

        // Closing a dialog does not make the app forget that a chooser is open…
        assert_eq!(model.update(Message::OpenFileDialog).units(), 1);
        let _ = model.update(Message::DismissChooserError);
        assert_eq!(model.update(Message::OpenFileDialog).units(), 0);
        // …and the button asks again while the error is still on screen.
        let _ = model.update(Message::ChooserAnswered(Err("no portal".into())));
        assert_eq!(model.update(Message::OpenFileDialog).units(), 1);
        assert!(model.dialog().is_none());

        // A font file chosen: its preview opens and the file is read.
        let path = PathBuf::from("/fonts/Some.ttf");
        let task = model.update(Message::ChooserAnswered(Ok(Some(path.clone()))));
        assert!(matches!(&model.view, View::File { path: shown, .. } if *shown == path));
        assert_eq!(task.units(), 1);
    }

    // Screens ------------------------------------------------------------------

    #[test]
    fn every_screen_can_be_built_in_every_state() {
        let mut model = model();
        model.families = installed_families();
        model.refilter();
        model.catalog = vec![
            catalog_font("roboto", "Roboto"),
            catalog_font("mixed", "Mixed"),
        ];
        model.refilter_catalog();
        model.refresh_catalog_installed();
        let mixed = model.intern("Mixed");

        let variants = || {
            vec![
                variant("regular", 400, Style::Normal, Some(REGULAR_URL)),
                variant("700", 700, Style::Normal, None),
            ]
        };
        let catalog_family = |index, preview| View::CatalogFamily {
            index,
            variants: Some(variants()),
            selected: Some(0),
            preview,
            error: None,
            status: Some("done".into()),
        };
        let screens = [
            View::Browse,
            View::Catalog,
            View::Family {
                name: mixed,
                status: None,
            },
            catalog_family(0, CatalogPreview::Nothing),
            catalog_family(0, CatalogPreview::Loading),
            catalog_family(0, CatalogPreview::Failed("no network".into())),
            // An installed family: its faces carry Uninstall, and its preview is the font.
            catalog_family(1, CatalogPreview::Ready(mixed)),
            View::CatalogFamily {
                index: 0,
                variants: None,
                selected: None,
                preview: CatalogPreview::Nothing,
                error: Some("offline".into()),
                status: None,
            },
        ];
        for screen in screens {
            model.view = screen;
            for pending in [None, Some(Pending::Family("mixed".into()))] {
                model.pending_uninstall = pending;
                let _ = model.view();
            }
        }

        model.set_show_context(true);
        assert!(model.context_drawer().is_some());
    }
}
