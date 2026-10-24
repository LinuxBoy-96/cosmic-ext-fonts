// SPDX-License-Identifier: GPL-3.0-or-later
//
// Fluent localization: `i18n/<lang>/*.ftl` embedded, English checked at compile time.

use std::sync::LazyLock;

use i18n_embed::{
    DefaultLocalizer, DesktopLanguageRequester, LanguageLoader, Localizer,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("failed to load the fallback language");
    loader
});

/// `fl!("id")` or `fl!("id", arg = value)`. Ids are checked against the English file at build time.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

/// Picks the desktop's languages, falling back to English.
pub fn localize() {
    let localizer = DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations);
    let requested = DesktopLanguageRequester::requested_languages();
    if let Err(error) = localizer.select(&requested) {
        eprintln!("failed to select language: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::Localizations;

    /// Message id → the `$variables` its text uses (selector branches included).
    fn messages(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current: Option<String> = None;
        for line in source.lines() {
            if line.starts_with(' ') || line.starts_with('}') {
                // Continuation of the message above.
            } else if line.starts_with('#') || line.trim().is_empty() {
                // A comment or blank line ends the message.
                current = None;
            } else if let Some((id, _)) = line.split_once(" = ") {
                current = Some(id.to_owned());
                found.entry(id.to_owned()).or_default();
            } else {
                current = None;
            }
            let Some(id) = &current else { continue };
            let mut rest = line;
            while let Some((_, after)) = rest.split_once('$') {
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                    .collect();
                found.entry(id.clone()).or_default().insert(name);
                rest = after;
            }
        }
        found
    }

    fn read(lang: &str) -> String {
        let file = Localizations::get(&format!("{lang}/cosmic_ext_fonts.ftl"))
            .unwrap_or_else(|| panic!("no messages for {lang}"));
        String::from_utf8(file.data.into_owned()).expect("messages are UTF-8")
    }

    #[test]
    fn every_language_has_the_messages_and_placeholders_of_english() {
        let languages: BTreeSet<String> = Localizations::iter()
            .filter_map(|path| path.split_once('/').map(|(lang, _)| lang.to_owned()))
            .collect();
        assert!(
            languages.contains("en") && languages.len() > 1,
            "{languages:?}"
        );

        let english = messages(&read("en"));
        assert!(
            english.len() > 50,
            "english messages not read: {}",
            english.len()
        );

        for lang in languages.iter().filter(|lang| *lang != "en") {
            let theirs = messages(&read(lang));
            let missing: Vec<_> = english
                .keys()
                .filter(|id| !theirs.contains_key(*id))
                .collect();
            let stale: Vec<_> = theirs
                .keys()
                .filter(|id| !english.contains_key(*id))
                .collect();
            assert!(
                missing.is_empty() && stale.is_empty(),
                "{lang}: missing {missing:?}, not in English {stale:?}"
            );
            for (id, variables) in &english {
                assert_eq!(&theirs[id], variables, "{lang}: placeholders of `{id}`");
            }
        }
    }

    /// The text of a one-line message (`pangram`, `app-title`).
    fn text_of(source: &str, id: &str) -> String {
        source
            .lines()
            .find_map(|line| line.strip_prefix(id)?.strip_prefix(" = "))
            .unwrap_or_else(|| panic!("no message `{id}`"))
            .to_owned()
    }

    fn languages() -> BTreeSet<String> {
        Localizations::iter()
            .filter_map(|path| path.split_once('/').map(|(lang, _)| lang.to_owned()))
            .collect()
    }

    /// The preview is a pangram plus the language's accented letters, so a missing glyph shows
    /// right away.
    #[test]
    fn every_language_previews_its_whole_alphabet_and_its_accents() {
        const LATIN: &str = "abcdefghijklmnopqrstuvwxyz";
        // (language, letters the pangram must have, accented letters)
        let expected = [
            ("en", LATIN, ""),
            ("fr", LATIN, "àâçéèêëîïôœùûüÿ"),
            ("de", LATIN, "äöüß"),
            ("es", LATIN, "áéíóúüñ"),
            // 21 letters: j k w x y only appear in loanwords.
            ("it", "abcdefghilmnopqrstuvz", "àèéìòù"),
            // 23 letters: k w y only appear in loanwords (the traditional pangram).
            ("pt", "abcdefghijlmnopqrstuvxz", "àáâãçéêíóôõú"),
            ("nl", LATIN, "éèëïóöü"),
        ];
        let listed: BTreeSet<String> = expected
            .iter()
            .map(|(lang, ..)| (*lang).to_owned())
            .collect();
        assert_eq!(listed, languages(), "a new language needs its row here");

        for (lang, letters, accents) in expected {
            let sample = text_of(&read(lang), "pangram").to_lowercase();
            for letter in letters.chars().chain(accents.chars()) {
                assert!(
                    sample.contains(letter),
                    "{lang}: no `{letter}` in `{sample}`"
                );
            }
        }
    }

    /// The About labels in every language (French: Créateur, Mainteneur, Lien, Licence).
    #[test]
    fn the_about_page_has_its_words_in_every_language() {
        const IDS: [&str; 6] = [
            "about-creator",
            "about-maintainer",
            "about-link",
            "about-repository",
            "about-license",
            "about-license-name",
        ];
        let expected = [
            (
                "en",
                "Creator|Maintainer|Link|Repository|License|GPL 3.0 or later",
            ),
            (
                "fr",
                "Créateur|Mainteneur|Lien|Dépôt|Licence|GPL 3.0 ou ultérieure",
            ),
            (
                "de",
                "Urheber|Betreuer|Link|Quellcode|Lizenz|GPL 3.0 oder später",
            ),
            (
                "es",
                "Creador|Mantenedor|Enlace|Repositorio|Licencia|GPL 3.0 o posterior",
            ),
            (
                "it",
                "Creatore|Manutentore|Collegamento|Repository|Licenza|GPL 3.0 o successiva",
            ),
            (
                "pt",
                "Criador|Mantenedor|Ligação|Repositório|Licença|GPL 3.0 ou posterior",
            ),
            (
                "nl",
                "Maker|Onderhouder|Link|Broncode|Licentie|GPL 3.0 of later",
            ),
        ];
        let listed: BTreeSet<String> = expected
            .iter()
            .map(|(lang, _)| (*lang).to_owned())
            .collect();
        assert_eq!(listed, languages(), "a new language needs its row here");
        for (lang, words) in expected {
            let source = read(lang);
            let words: Vec<_> = words.split('|').collect();
            assert_eq!(words.len(), IDS.len(), "{lang}");
            for (id, word) in IDS.into_iter().zip(&words) {
                assert_eq!(text_of(&source, id), *word, "{lang}: {id}");
            }
            // Always the GPL 3.0 or later.
            assert!(words[5].starts_with("GPL 3.0 "), "{lang}");
            // "Version" is translated too.
            assert!(!text_of(&source, "about-version").is_empty(), "{lang}");
        }
    }

    /// Menu words in every language, the same as Scan's so both apps read alike.
    #[test]
    fn the_menu_has_its_words_in_every_language() {
        let expected = [
            ("en", "View|About|Theme"),
            ("fr", "Affichage|À propos|Thème"),
            ("de", "Ansicht|Info|Design"),
            ("es", "Ver|Acerca de|Tema"),
            ("it", "Visualizza|Informazioni|Tema"),
            ("pt", "Ver|Acerca|Tema"),
            ("nl", "Beeld|Over|Thema"),
        ];
        let listed: BTreeSet<String> = expected
            .iter()
            .map(|(lang, _)| (*lang).to_owned())
            .collect();
        assert_eq!(listed, languages(), "a new language needs its row here");
        for (lang, words) in expected {
            let source = read(lang);
            let words: Vec<_> = words.split('|').collect();
            for (id, word) in ["menu-view", "about", "theme"].into_iter().zip(&words) {
                assert_eq!(text_of(&source, id), *word, "{lang}: {id}");
            }
        }
    }

    const DESKTOP: &str = include_str!("../res/app.desktop");
    const METAINFO: &str = include_str!("../res/metainfo.xml");

    /// `Key=` in English, `Key[lang]=` in the other languages.
    fn desktop_value(key: &str, lang: &str) -> &'static str {
        let prefix = if lang == "en" {
            format!("{key}=")
        } else {
            format!("{key}[{lang}]=")
        };
        DESKTOP
            .lines()
            .find_map(|line| line.strip_prefix(prefix.as_str()))
            .unwrap_or_else(|| panic!("the desktop entry has no {prefix}"))
    }

    /// `<tag>` in English, `<tag xml:lang="…">` otherwise.
    fn open_tag(tag: &str, lang: &str) -> String {
        if lang == "en" {
            format!("<{tag}>")
        } else {
            format!("<{tag} xml:lang=\"{lang}\">")
        }
    }

    /// The one-line `<tag>…</tag>` values of a language, in file order.
    fn metainfo_values(tag: &str, lang: &str) -> Vec<&'static str> {
        let open = open_tag(tag, lang);
        let close = format!("</{tag}>");
        METAINFO
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix(open.as_str())?
                    .strip_suffix(close.as_str())
            })
            .collect()
    }

    /// The first one (the app's `<name>` comes before the developer's).
    fn metainfo_value(tag: &str, lang: &str) -> &'static str {
        metainfo_values(tag, lang)
            .first()
            .copied()
            .unwrap_or_else(|| panic!("the metainfo has no {}", open_tag(tag, lang)))
    }

    /// Paragraphs and list items in a language (description and release notes).
    fn paragraphs(lang: &str) -> usize {
        ["p", "li"]
            .into_iter()
            .map(|tag| METAINFO.matches(open_tag(tag, lang).as_str()).count())
            .sum()
    }

    /// The header bar says what the launcher shows under the name (English capitalizes the title).
    #[test]
    fn the_title_is_the_comment_of_the_desktop_entry() {
        for lang in languages() {
            let title = text_of(&read(&lang), "app-title").to_lowercase();
            let comment = desktop_value("Comment", &lang).to_lowercase();
            assert_eq!(title, comment, "{lang}");
        }
    }

    /// The window title is the app's name, as in the launcher and dock; window switchers show it.
    #[test]
    fn the_window_is_named_by_the_name_of_the_desktop_entry() {
        for lang in languages() {
            let name = desktop_value("Name", &lang);
            assert_eq!(text_of(&read(&lang), "app-name"), name, "{lang}");
        }
    }

    /// The launcher and the store say the same things in every language: name, summary, keywords,
    /// and a fully translated description.
    #[test]
    fn the_desktop_entry_and_the_metainfo_are_complete_and_agree_in_every_language() {
        let english = paragraphs("en");
        assert!(english >= 2, "the metainfo describes the app: {english}");
        for lang in languages() {
            for (key, tag) in [("Name", "name"), ("Comment", "summary")] {
                let shown = desktop_value(key, &lang);
                assert!(!shown.is_empty(), "{lang}: {key}");
                assert_eq!(
                    metainfo_value(tag, &lang),
                    shown,
                    "{lang}: <{tag}> and {key}"
                );
            }
            assert_eq!(paragraphs(&lang), english, "{lang}: description");
            // The caption of each screenshot too.
            assert_eq!(
                metainfo_values("caption", &lang).len(),
                metainfo_values("caption", "en").len(),
                "{lang}: captions"
            );
            // Same search words as the launcher.
            let words: Vec<_> = desktop_value("Keywords", &lang)
                .split_terminator(';')
                .collect();
            assert!(!words.is_empty(), "{lang}: Keywords");
            assert_eq!(
                metainfo_values("keyword", &lang),
                words,
                "{lang}: <keyword> and Keywords"
            );
        }
    }

    #[test]
    fn the_message_reader_sees_variables_in_selector_branches() {
        let sample = "a = x { $one }\nb = { $n ->\n    [one] { $n } y { $m }\n   *[other] z\n}\n\n# c = { $no }\nd = plain\n";
        let found = messages(sample);
        assert_eq!(found["a"], BTreeSet::from(["one".to_owned()]));
        assert_eq!(found["b"], BTreeSet::from(["n".to_owned(), "m".to_owned()]));
        assert!(found["d"].is_empty());
        assert_eq!(found.len(), 3, "comments are not messages");
    }
}
