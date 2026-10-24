//! The Google Fonts catalog, through the google-webfonts-helper API (<https://gwfh.mranftl.com>,
//! no key needed). A download is just a `.ttf` handed to [`crate::installer::install`], like a
//! local file. The list is cached for a day, and an old copy is used when offline.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use thiserror::Error;
use ureq::typestate::AgentScope;

const API_BASE: &str = "https://gwfh.mranftl.com/api/fonts";
/// Hosts we download TTFs from: the catalog's CDN and the API host.
const DOWNLOAD_HOSTS: [&str; 2] = ["fonts.gstatic.com", "gwfh.mranftl.com"];
/// Safety cap on downloaded files (largest Google fonts are a few MB).
const MAX_DOWNLOAD_BYTES: u64 = 50 * 1024 * 1024;
/// The family list is ~1 MB; anything near this is not the catalog.
const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;
#[allow(clippy::duration_suboptimal_units)] // from_hours needs Rust 1.91
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response from the catalog: {0}")]
    Parse(String),
    #[error("this variant has no downloadable TTF")]
    NoTtf,
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("downloaded file is larger than 50 MiB")]
    TooLarge,
    #[error("the catalog returned an invalid font id")]
    InvalidId,
    #[error("refusing to download from {0}")]
    UntrustedUrl(String),
}

impl From<ureq::Error> for CatalogError {
    fn from(e: ureq::Error) -> Self {
        CatalogError::Network(e.to_string())
    }
}

/// One family in the catalog list.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogFont {
    pub id: String,
    pub family: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub variants: Vec<String>,
}

/// One downloadable variant of a family.
#[derive(Debug, Clone)]
pub struct CatalogVariant {
    pub id: String,
    pub weight: fontdb::Weight,
    pub style: fontdb::Style,
    pub ttf_url: Option<String>,
}

impl CatalogVariant {
    /// "Bold Italic"-style label, mirroring `inventory::variant_label`.
    #[must_use]
    pub fn label(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        let w = crate::inventory::weight_name(self.weight);
        if !w.is_empty() {
            parts.push(w);
        }
        if self.style == fontdb::Style::Italic {
            parts.push("Italic");
        }
        if parts.is_empty() {
            "Regular".to_owned()
        } else {
            parts.join(" ")
        }
    }
}

// Raw shapes of the gwfh API ------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawDetail {
    #[serde(default)]
    variants: Vec<RawVariant>,
}

#[derive(Debug, Deserialize)]
struct RawVariant {
    id: String,
    #[serde(default, rename = "fontWeight")]
    font_weight: String,
    #[serde(default, rename = "fontStyle")]
    font_style: String,
    #[serde(default)]
    ttf: Option<String>,
}

impl From<RawVariant> for CatalogVariant {
    fn from(raw: RawVariant) -> Self {
        let weight = raw
            .font_weight
            .trim()
            .parse::<u16>()
            .map_or(fontdb::Weight(400), fontdb::Weight);
        let style = if raw.font_style.eq_ignore_ascii_case("italic") {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        };
        CatalogVariant {
            id: raw.id,
            weight,
            style,
            ttf_url: raw.ttf,
        }
    }
}

// HTTP -----------------------------------------------------------------------

/// rustls with the pure-Rust `RustCrypto` provider: no C, no `ring`. ureq has no default
/// provider without its `rustls` feature, so we set one or the first request would panic.
fn tls_config(roots: ureq::tls::RootCerts) -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::Rustls)
        .unversioned_rustls_crypto_provider(Arc::new(rustls_rustcrypto::provider()))
        .root_certs(roots)
        .build()
}

/// Timeouts on every phase, so a stalled server can't leave the UI on "Loading…". HTTPS only,
/// redirects included.
#[allow(clippy::duration_suboptimal_units)] // from_mins needs Rust 1.91
fn agent_builder(roots: ureq::tls::RootCerts) -> ureq::config::ConfigBuilder<AgentScope> {
    ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(3)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(300)))
        .tls_config(tls_config(roots))
}

/// The agent every request goes through; trusts Mozilla's roots (`webpki-roots`).
fn agent() -> ureq::Agent {
    agent_builder(ureq::tls::RootCerts::WebPki).build().into()
}

fn get_text(url: &str) -> Result<String, CatalogError> {
    get_text_with(&agent(), url)
}

fn get_text_with(agent: &ureq::Agent, url: &str) -> Result<String, CatalogError> {
    let mut response = agent.get(url).call()?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_JSON_BYTES)
        .read_to_string()?;
    Ok(body)
}

// Disk cache -----------------------------------------------------------------

/// `$XDG_CACHE_HOME/cosmic-ext-fonts/catalog.json`, or `~/.cache/...`.
fn cache_file() -> Option<PathBuf> {
    Some(
        dirs::cache_dir()?
            .join("cosmic-ext-fonts")
            .join("catalog.json"),
    )
}

fn cache_age(path: &Path) -> Option<Duration> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    SystemTime::now().duration_since(modified).ok()
}

/// Atomic write (tmp + rename) so a crash never leaves a half-written cache.
fn write_cache(path: &Path, body: &str) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, body).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}

// Public API -----------------------------------------------------------------

/// The whole catalog (~1500 families): from the cache if it's under a day old, else the
/// network, else an old cache. Blocking; call it from `spawn_blocking`.
///
/// # Errors
/// Network failure or a bad response, with no cache to fall back on.
pub fn fetch_catalog() -> Result<Vec<CatalogFont>, CatalogError> {
    load_catalog(cache_file().as_deref(), || get_text(API_BASE))
}

/// [`fetch_catalog`] with the cache location and the network injected.
fn load_catalog(
    cache: Option<&Path>,
    fetch: impl FnOnce() -> Result<String, CatalogError>,
) -> Result<Vec<CatalogFont>, CatalogError> {
    if let Some(path) = cache
        && cache_age(path).is_some_and(|age| age < CACHE_TTL)
        && let Some(fonts) = read_cache(path)
    {
        return Ok(fonts);
    }

    // Parse before caching: garbage is never persisted.
    match fetch().and_then(|body| parse_catalog(&body).map(|fonts| (body, fonts))) {
        Ok((body, fonts)) => {
            if let Some(path) = cache {
                write_cache(path, &body);
            }
            Ok(fonts)
        }
        // Offline, or a 200 that is not a catalog: a stale list beats no list.
        Err(err) => cache.and_then(read_cache).ok_or(err),
    }
}

fn read_cache(path: &Path) -> Option<Vec<CatalogFont>> {
    parse_catalog(&fs::read_to_string(path).ok()?).ok()
}

/// Catalog ids are lowercase words and dashes (`open-sans`, `m-plus-1p`). They go into the URL
/// path, so nothing else is allowed.
fn is_valid_font_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The downloadable variants of one family. Blocking.
///
/// # Errors
/// An invalid id, a network failure or an unparsable response.
pub fn fetch_variants(font_id: &str) -> Result<Vec<CatalogVariant>, CatalogError> {
    if !is_valid_font_id(font_id) {
        return Err(CatalogError::InvalidId);
    }
    let body = get_text(&format!("{API_BASE}/{font_id}"))?;
    parse_variants(&body)
}

/// The URL comes from a third-party API, so only HTTPS (HTTP gets upgraded) to a known host,
/// with no credentials or port.
fn trusted_download_url(url: &str) -> Result<String, CatalogError> {
    let untrusted = || CatalogError::UntrustedUrl(url.to_owned());

    let uri: ureq::http::Uri = url.parse().map_err(|_| untrusted())?;
    let host = uri.host().ok_or_else(untrusted)?;
    let plain_authority = uri
        .authority()
        .is_some_and(|a| a.as_str().eq_ignore_ascii_case(host));

    if matches!(uri.scheme_str(), Some("https" | "http"))
        && plain_authority
        && DOWNLOAD_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(host))
    {
        let path = uri.path_and_query().map_or("/", |p| p.as_str());
        Ok(format!("https://{}{path}", host.to_ascii_lowercase()))
    } else {
        Err(untrusted())
    }
}

/// A downloaded TTF in a private temp folder, deleted on drop.
#[derive(Debug)]
pub struct Download {
    _dir: tempfile::TempDir,
    pub path: PathBuf,
}

/// Keep letters and digits, turn everything else into `-`.
fn file_safe(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

/// Downloads a variant's TTF, ready for `installer::install`.
///
/// It goes in a fresh 0700 folder (in `$XDG_RUNTIME_DIR` if there is one), not at a predictable
/// `/tmp` path where another user could plant a symlink. The name keeps family and variant
/// (`Roboto-700italic.ttf`) so the installed file reads well.
///
/// # Errors
/// An untrusted URL, a network or I/O failure, or a body over 50 MiB.
pub fn download_ttf(url: &str, family: &str, variant_id: &str) -> Result<Download, CatalogError> {
    let url = trusted_download_url(url)?;
    let mut response = agent().get(&url).call()?;

    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let dir = tempfile::Builder::new()
        .prefix("cosmic-ext-fonts-")
        .tempdir_in(base)?;
    let path = dir.path().join(format!(
        "{}-{}.ttf",
        file_safe(family),
        file_safe(variant_id)
    ));

    // Read one byte past the cap so oversize is an error, not a silent truncation.
    let mut reader = response.body_mut().as_reader().take(MAX_DOWNLOAD_BYTES + 1);
    let mut file = fs::File::create(&path)?;
    let written = std::io::copy(&mut reader, &mut file)?;
    if written > MAX_DOWNLOAD_BYTES {
        return Err(CatalogError::TooLarge);
    }
    file.sync_all()?;

    Ok(Download { _dir: dir, path })
}

// Parsing (pure, unit-testable) ----------------------------------------------

fn parse_catalog(body: &str) -> Result<Vec<CatalogFont>, CatalogError> {
    let fonts: Vec<CatalogFont> =
        serde_json::from_str(body).map_err(|e| CatalogError::Parse(e.to_string()))?;
    if fonts.is_empty() {
        return Err(CatalogError::Parse("the family list is empty".into()));
    }
    Ok(fonts)
}

fn parse_variants(body: &str) -> Result<Vec<CatalogVariant>, CatalogError> {
    let raw: RawDetail =
        serde_json::from_str(body).map_err(|e| CatalogError::Parse(e.to_string()))?;
    Ok(raw.variants.into_iter().map(CatalogVariant::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_catalog_list_fixture() {
        let fixture = r#"[
            {"id":"roboto","family":"Roboto","variants":["100","regular","700italic"],
             "subsets":["latin"],"category":"sans-serif","version":"v47",
             "lastModified":"2025-01-01","popularity":1,"defSubset":"latin","defVariant":"regular"},
            {"id":"lora","family":"Lora","variants":["regular"],"category":"serif"}
        ]"#;
        let fonts = parse_catalog(fixture).unwrap();
        assert_eq!(fonts.len(), 2);
        assert_eq!(fonts[0].family, "Roboto");
        assert_eq!(fonts[0].variants.len(), 3);
        assert_eq!(fonts[1].category, "serif");
    }

    #[test]
    fn parses_variants_and_maps_weight_style() {
        let fixture = r#"{
            "id":"roboto","family":"Roboto",
            "variants":[
                {"id":"regular","fontFamily":"'Roboto'","fontStyle":"normal",
                 "fontWeight":"400","ttf":"https://fonts.gstatic.com/r.ttf","woff":"w"},
                {"id":"700italic","fontStyle":"italic","fontWeight":"700",
                 "ttf":"https://fonts.gstatic.com/ri.ttf"},
                {"id":"100","fontStyle":"normal","fontWeight":"100","ttf":null}
            ]
        }"#;
        let variants = parse_variants(fixture).unwrap();
        assert_eq!(variants.len(), 3);

        assert_eq!(variants[0].weight, fontdb::Weight(400));
        assert_eq!(variants[0].style, fontdb::Style::Normal);
        assert_eq!(variants[0].label(), "Regular");

        assert_eq!(variants[1].weight, fontdb::Weight(700));
        assert_eq!(variants[1].style, fontdb::Style::Italic);
        assert_eq!(variants[1].label(), "Bold Italic");
        assert!(variants[1].ttf_url.is_some());

        assert_eq!(variants[2].label(), "Thin");
        assert!(variants[2].ttf_url.is_none());
    }

    const ONE_FAMILY: &str =
        r#"[{"id":"lora","family":"Lora","variants":["regular"],"category":"serif"}]"#;

    /// Make a cache file look `age` old.
    fn age_file(path: &Path, age: Duration) {
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    fn no_network() -> Result<String, CatalogError> {
        Err(CatalogError::Network("offline".into()))
    }

    fn forbidden() -> Result<String, CatalogError> {
        panic!("the network must not be used");
    }

    #[test]
    fn fresh_cache_is_served_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("catalog.json");
        write_cache(&cache, ONE_FAMILY);

        let fonts = load_catalog(Some(&cache), forbidden).unwrap();
        assert_eq!(fonts[0].family, "Lora");
    }

    #[test]
    fn network_result_is_cached_only_when_it_parses() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("catalog.json");

        let fonts = load_catalog(Some(&cache), || Ok(ONE_FAMILY.to_owned())).unwrap();
        assert_eq!(fonts.len(), 1);
        assert_eq!(fs::read_to_string(&cache).unwrap(), ONE_FAMILY);

        // Garbage, an HTML error page and an empty list are all refused and never written.
        for bad in ["<html>502</html>", "{}", "[]", ""] {
            let fresh = tempfile::tempdir().unwrap();
            let path = fresh.path().join("catalog.json");
            let err = load_catalog(Some(&path), || Ok(bad.to_owned())).unwrap_err();
            assert!(matches!(err, CatalogError::Parse(_)), "{bad:?}: {err}");
            assert!(!path.exists(), "{bad:?} must not be cached");
        }
    }

    #[test]
    fn stale_cache_beats_no_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("catalog.json");
        write_cache(&cache, ONE_FAMILY);
        age_file(&cache, CACHE_TTL * 3);
        assert!(cache_age(&cache).unwrap() > CACHE_TTL);

        // Offline.
        let fonts = load_catalog(Some(&cache), no_network).unwrap();
        assert_eq!(fonts[0].family, "Lora");

        // Online, but a 200 that isn't a catalog.
        let fonts = load_catalog(Some(&cache), || Ok("<html>maintenance</html>".into())).unwrap();
        assert_eq!(fonts[0].family, "Lora");
        assert_eq!(
            fs::read_to_string(&cache).unwrap(),
            ONE_FAMILY,
            "the good cache survives a bad answer"
        );

        // Nothing cached at all: the error is reported.
        assert!(load_catalog(None, no_network).is_err());
        let empty = tempfile::tempdir().unwrap();
        let missing = empty.path().join("catalog.json");
        assert!(matches!(
            load_catalog(Some(&missing), no_network),
            Err(CatalogError::Network(_))
        ));
    }

    #[test]
    fn font_ids_are_plain_words() {
        for ok in [
            "roboto",
            "open-sans",
            "m-plus-1p",
            "libre-barcode-39-extended",
        ] {
            assert!(is_valid_font_id(ok), "{ok}");
        }
        for bad in [
            "", "..", "../x", "a/b", "a?b", "a#b", "Roboto", "a b", "é", "a\nb",
        ] {
            assert!(!is_valid_font_id(bad), "{bad:?}");
        }
        assert!(!is_valid_font_id(&"a".repeat(101)));
        // Rejected before any request is made.
        assert!(matches!(
            fetch_variants("../etc"),
            Err(CatalogError::InvalidId)
        ));
    }

    #[test]
    fn downloads_are_https_on_known_hosts_only() {
        let ok = |url: &str| trusted_download_url(url).unwrap();
        assert_eq!(
            ok("https://fonts.gstatic.com/s/roboto/v30/a.ttf"),
            "https://fonts.gstatic.com/s/roboto/v30/a.ttf"
        );
        // Plain HTTP on a known host is upgraded, query kept.
        assert_eq!(
            ok("http://fonts.gstatic.com/s/x/y.ttf?v=1"),
            "https://fonts.gstatic.com/s/x/y.ttf?v=1"
        );
        assert_eq!(
            ok("https://FONTS.GSTATIC.COM/a.ttf"),
            "https://fonts.gstatic.com/a.ttf"
        );
        assert_eq!(
            ok("https://gwfh.mranftl.com/a.ttf"),
            "https://gwfh.mranftl.com/a.ttf"
        );

        for bad in [
            "https://evil.example/a.ttf",
            "https://fonts.gstatic.com.evil.example/a.ttf",
            "https://evilfonts.gstatic.com/a.ttf",
            "https://fonts.gstatic.com@evil.example/a.ttf",
            "https://user:pw@fonts.gstatic.com/a.ttf",
            "https://fonts.gstatic.com:8443/a.ttf",
            "https://127.0.0.1/a.ttf",
            "ftp://fonts.gstatic.com/a.ttf",
            "file:///etc/passwd",
            "//fonts.gstatic.com/a.ttf",
            "/a.ttf",
            "",
        ] {
            assert!(
                matches!(
                    trusted_download_url(bad),
                    Err(CatalogError::UntrustedUrl(_))
                ),
                "{bad:?} must be refused"
            );
        }
        // The check runs before any connection or file is made.
        assert!(matches!(
            download_ttf("http://127.0.0.1:9/a.ttf", "Fam", "regular"),
            Err(CatalogError::UntrustedUrl(_))
        ));
    }

    #[test]
    fn agent_refuses_plain_http() {
        // Fails before any connection is attempted (and so would a redirect to http).
        let err = agent().get("http://127.0.0.1:9/").call().unwrap_err();
        assert!(matches!(err, ureq::Error::RequireHttpsOnly(_)), "{err}");
    }

    #[test]
    fn temp_file_names_cannot_escape_the_directory() {
        assert_eq!(file_safe("Open Sans"), "Open-Sans");
        assert_eq!(file_safe("../../etc/passwd"), "------etc-passwd");
        assert_eq!(file_safe("700italic"), "700italic");
    }

    #[test]
    fn cache_write_is_atomic_and_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("catalog.json");
        assert!(cache_age(&path).is_none(), "missing cache has no age");

        write_cache(&path, "[]");
        assert_eq!(fs::read_to_string(&path).unwrap(), "[]");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "tmp file renamed away"
        );
        assert!(cache_age(&path).unwrap() < CACHE_TTL);
    }

    /// HTTPS end to end, against a local server with the same pure-Rust provider and the test
    /// PKI in `testdata/tls`.
    mod tls {
        use std::io::{self, Read, Write};
        use std::net::TcpListener;
        use std::thread::{self, JoinHandle};
        use std::time::Instant;

        use rustls::crypto::CryptoProvider;
        use rustls::pki_types::pem::PemObject;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use rustls::{CipherSuite, NamedGroup, ProtocolVersion, SupportedProtocolVersion};
        use ureq::tls::{Certificate, RootCerts};

        use super::*;

        /// A root and a leaf for `localhost` issued by it, valid 2020-2120.
        struct Pki {
            name: &'static str,
            root: &'static str,
            leaf: &'static str,
            key: &'static str,
        }

        macro_rules! pki {
            ($name:literal, $root:literal, $leaf:literal) => {
                Pki {
                    name: $name,
                    root: include_str!(concat!("../testdata/tls/", $root, "-root.pem")),
                    leaf: include_str!(concat!("../testdata/tls/", $leaf, ".pem")),
                    key: include_str!(concat!("../testdata/tls/", $leaf, ".key")),
                }
            };
        }

        const PKI: [Pki; 4] = [
            pki!("P-256 root, P-256 leaf", "ec256", "leaf-ec-by-ec256"),
            pki!("P-384 root, P-256 leaf", "ec384", "leaf-ec-by-ec384"),
            pki!("RSA-4096 root, P-256 leaf", "rsa4096", "leaf-ec-by-rsa4096"),
            pki!(
                "RSA-4096 root, RSA-2048 leaf",
                "rsa4096",
                "leaf-rsa2048-by-rsa4096"
            ),
        ];

        /// What the server negotiated.
        #[derive(Debug, PartialEq)]
        struct Negotiated {
            version: Option<ProtocolVersion>,
            suite: Option<CipherSuite>,
            group: Option<NamedGroup>,
        }

        fn certs(pem: &str) -> Vec<CertificateDer<'static>> {
            CertificateDer::pem_slice_iter(pem.as_bytes())
                .collect::<Result<_, _>>()
                .unwrap()
        }

        fn server_config(
            pki: &Pki,
            chain: Vec<CertificateDer<'static>>,
            provider: CryptoProvider,
            versions: &[&'static SupportedProtocolVersion],
        ) -> rustls::ServerConfig {
            let key = PrivateKeyDer::from_pem_slice(pki.key.as_bytes()).unwrap();
            rustls::ServerConfig::builder_with_provider(Arc::new(provider))
                .with_protocol_versions(versions)
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(chain, key)
                .unwrap()
        }

        /// Answer one HTTPS request on a free port of localhost with `ONE_FAMILY`.
        fn serve_once(
            config: rustls::ServerConfig,
        ) -> (u16, JoinHandle<Result<Negotiated, String>>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let config = Arc::new(config);
            let handle = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(30);
                let tcp = loop {
                    match listener.accept() {
                        Ok((tcp, _)) => break tcp,
                        Err(e)
                            if e.kind() == io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(e) => return Err(format!("accept: {e}")),
                    }
                };
                let err = |e: io::Error| e.to_string();
                tcp.set_nonblocking(false).map_err(err)?;
                tcp.set_read_timeout(Some(Duration::from_secs(30)))
                    .map_err(err)?;
                let conn = rustls::ServerConnection::new(config).map_err(|e| e.to_string())?;
                let mut tls = rustls::StreamOwned::new(conn, tcp);

                let (mut head, mut buf) = (Vec::new(), [0_u8; 512]);
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = tls.read(&mut buf).map_err(err)?;
                    if n == 0 {
                        break;
                    }
                    head.extend_from_slice(&buf[..n]);
                }
                write!(
                    tls,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{ONE_FAMILY}",
                    ONE_FAMILY.len()
                )
                .map_err(err)?;
                tls.flush().map_err(err)?;
                tls.conn.send_close_notify();
                let _ = tls.conn.complete_io(&mut tls.sock);
                Ok(Negotiated {
                    version: tls.conn.protocol_version(),
                    suite: tls.conn.negotiated_cipher_suite().map(|s| s.suite()),
                    group: tls
                        .conn
                        .negotiated_key_exchange_group()
                        .map(rustls::crypto::SupportedKxGroup::name),
                })
            });
            (port, handle)
        }

        /// The real agent, trusting `roots` and ignoring proxies (CI sandboxes often set
        /// `HTTPS_PROXY`).
        fn agent_trusting(roots: RootCerts) -> ureq::Agent {
            agent_builder(roots).proxy(None).build().into()
        }

        fn trusting(pki: &Pki) -> ureq::Agent {
            let root = Certificate::from_pem(pki.root.as_bytes()).unwrap();
            agent_trusting(RootCerts::Specific(Arc::new(vec![root])))
        }

        fn both_versions() -> [&'static SupportedProtocolVersion; 2] {
            [&rustls::version::TLS12, &rustls::version::TLS13]
        }

        #[test]
        fn https_works_with_every_suite_group_and_certificate_type() {
            use rustls_rustcrypto as rc;

            struct Case {
                pki: usize,
                version: &'static SupportedProtocolVersion,
                suite: rustls::SupportedCipherSuite,
                group: NamedGroup,
            }
            let case = |pki, version, suite, group| Case {
                pki,
                version,
                suite,
                group,
            };
            let (tls12, tls13) = (&rustls::version::TLS12, &rustls::version::TLS13);
            // The client offers X25519 first; other groups go through a HelloRetryRequest.
            let cases = [
                case(0, tls13, rc::TLS13_AES_128_GCM_SHA256, NamedGroup::X25519),
                case(
                    1,
                    tls13,
                    rc::TLS13_AES_256_GCM_SHA384,
                    NamedGroup::secp256r1,
                ),
                case(
                    2,
                    tls13,
                    rc::TLS13_CHACHA20_POLY1305_SHA256,
                    NamedGroup::secp384r1,
                ),
                case(3, tls13, rc::TLS13_AES_128_GCM_SHA256, NamedGroup::X25519), // RSA-PSS
                case(
                    0,
                    tls12,
                    rc::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                    NamedGroup::secp256r1,
                ),
                case(
                    1,
                    tls12,
                    rc::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                    NamedGroup::X25519,
                ),
                case(
                    2,
                    tls12,
                    rc::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                    NamedGroup::secp384r1,
                ),
                case(
                    3,
                    tls12,
                    rc::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
                    NamedGroup::X25519,
                ),
                case(
                    3,
                    tls12,
                    rc::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
                    NamedGroup::secp256r1,
                ),
                case(
                    3,
                    tls12,
                    rc::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
                    NamedGroup::secp384r1,
                ),
            ];

            for c in cases {
                let pki = &PKI[c.pki];
                let label = format!("{} / {:?} / {:?}", pki.name, c.suite.suite(), c.group);

                let mut provider = rc::provider();
                provider.cipher_suites = vec![c.suite];
                provider.kx_groups.retain(|g| g.name() == c.group);
                let config = server_config(pki, certs(pki.leaf), provider, &[c.version]);
                let (port, server) = serve_once(config);

                let body = get_text_with(
                    &trusting(pki),
                    &format!("https://localhost:{port}/api/fonts"),
                )
                .unwrap_or_else(|e| panic!("{label}: {e}"));
                assert_eq!(body, ONE_FAMILY, "{label}");
                assert_eq!(parse_catalog(&body).unwrap().len(), 1, "{label}");

                let seen = server
                    .join()
                    .unwrap()
                    .unwrap_or_else(|e| panic!("{label}: server: {e}"));
                let version = if std::ptr::eq(c.version, tls13) {
                    ProtocolVersion::TLSv1_3
                } else {
                    ProtocolVersion::TLSv1_2
                };
                let expected = Negotiated {
                    version: Some(version),
                    suite: Some(c.suite.suite()),
                    group: Some(c.group),
                };
                assert_eq!(seen, expected, "{label}");
            }
        }

        #[test]
        fn a_forged_signature_in_the_chain_is_refused() {
            // One per signature algorithm: ECDSA P-256, ECDSA P-384 and RSA-4096. Flipping the
            // leaf's last bit breaks the issuer's signature and nothing else.
            for pki in &PKI[..3] {
                let mut chain = certs(pki.leaf);
                let mut der = chain[0].as_ref().to_vec();
                *der.last_mut().unwrap() ^= 1;
                chain[0] = CertificateDer::from(der);

                let config =
                    server_config(pki, chain, rustls_rustcrypto::provider(), &both_versions());
                let (port, server) = serve_once(config);
                let err = get_text_with(&trusting(pki), &format!("https://localhost:{port}/"))
                    .unwrap_err()
                    .to_string();
                assert!(err.contains("BadSignature"), "{}: {err}", pki.name);
                let _ = server.join();
            }
        }

        #[test]
        fn an_unknown_issuer_and_a_wrong_name_are_refused() {
            let pki = &PKI[0];
            let config = || {
                server_config(
                    pki,
                    certs(pki.leaf),
                    rustls_rustcrypto::provider(),
                    &both_versions(),
                )
            };

            // The production agent trusts Mozilla's roots, not the test root.
            let (port, server) = serve_once(config());
            let err = get_text_with(
                &agent_trusting(RootCerts::WebPki),
                &format!("https://localhost:{port}/"),
            )
            .unwrap_err()
            .to_string();
            assert!(err.contains("UnknownIssuer"), "{err}");
            let _ = server.join();

            // Trusted root, but the certificate is for `localhost`, not this address.
            let (port, server) = serve_once(config());
            let err = get_text_with(&trusting(pki), &format!("https://127.0.0.1:{port}/"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("not valid for name"), "{err}");
            let _ = server.join();
        }

        #[test]
        fn the_tls_stack_stays_pure_rust() {
            // ureq's own `rustls` feature turns on `ring` (C and assembly) and
            // `native-tls` links OpenSSL; only the provider-less rustls is allowed.
            let manifest = include_str!("../Cargo.toml");
            let ureq = manifest
                .lines()
                .find(|l| l.trim_start().starts_with("ureq"))
                .expect("a ureq dependency");
            assert!(ureq.contains("default-features = false"), "{ureq}");
            assert!(ureq.contains("\"rustls-no-provider\""), "{ureq}");
            for banned in [
                "\"rustls\"",
                "native-tls",
                "platform-verifier",
                "aws-lc",
                "\"ring\"",
            ] {
                assert!(!ureq.contains(banned), "{banned} in {ureq}");
            }
        }
    }
}
