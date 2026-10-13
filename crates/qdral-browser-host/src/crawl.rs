//! SG-000108 slice 7: map and crawl. A bounded, host-side queue over the one
//! admitted destination of a browser lifetime: every page is opened by the
//! governed navigation (SG-000075, the redirect and rebinding checks
//! included), its links come from the host's observation, and its content,
//! if asked, from the slice 6 extraction.
//!
//! Aladdin-native: Firecrawl's documented crawl behaviour (a path-prefix
//! scope, a depth bound, internal links only, fragments dropped, non-page
//! files and non-web schemes skipped, include and exclude paths) is the
//! reference; no Firecrawl code is copied. Include and exclude are path
//! prefixes, not regular expressions.

use crate::error::HostError;
use crate::extraction::{ExtractOptions, PageExtraction};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Pages one crawl may open.
pub const MAX_CRAWL_PAGES: u32 = 50;
/// Link hops from the start page.
pub const MAX_CRAWL_DEPTH: u32 = 5;
/// Wall-clock bound of one crawl (checked between pages).
pub const MAX_CRAWL_DURATION: Duration = Duration::from_secs(600);
/// Distinct in-scope URLs one crawl remembers (visited and queued).
pub const MAX_CRAWL_URLS: usize = 1000;
/// Skipped links reported (later ones are only counted).
pub const MAX_SKIPPED_REPORTED: usize = 200;
/// Include and exclude path prefixes, each.
pub const MAX_PATH_PATTERNS: usize = 16;
pub const MAX_PATH_PATTERN_BYTES: usize = 256;

/// Path endings that are files, not pages: never opened by a crawl.
const NOT_PAGES: &[&str] = &[
    ".pdf", ".zip", ".gz", ".tgz", ".rar", ".7z", ".tar", ".png", ".jpg", ".jpeg", ".gif", ".webp",
    ".svg", ".ico", ".bmp", ".avif", ".mp3", ".mp4", ".m4a", ".wav", ".ogg", ".webm", ".mov",
    ".avi", ".exe", ".msi", ".dmg", ".pkg", ".deb", ".rpm", ".apk", ".iso", ".bin", ".doc",
    ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".csv", ".json", ".xml", ".woff", ".woff2", ".ttf",
    ".otf", ".css", ".js", ".mjs", ".wasm",
];

/// What to crawl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlSpec {
    /// An `https` URL on the browser lifetime's admitted destination. Its
    /// directory is the crawl's scope: only paths under it are opened.
    pub start_url: String,
    pub max_pages: u32,
    pub max_depth: u32,
    /// Path prefixes a page must start with (none: every in-scope path).
    pub include_paths: Vec<String>,
    /// Path prefixes a page must not start with.
    pub exclude_paths: Vec<String>,
    /// Extract every opened page (slice 6); `None` maps links only.
    pub extract: Option<ExtractOptions>,
    pub deadline: Duration,
}

/// Why a discovered link was not opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Not an `https` URL on the crawl's origin, or not resolvable
    /// (another scheme, a relative path, credentials, a backslash).
    OtherOrigin,
    /// Outside the start URL's directory.
    OutOfScope,
    /// An include path did not match, or an exclude path did.
    Filtered,
    /// A file, not a page (by its path ending).
    NotAPage,
    /// Dot segments or an encoded dot: not in the form the engine requests.
    Unnormalized,
    /// Past the depth bound.
    TooDeep,
    /// The crawl already remembers MAX_CRAWL_URLS URLs.
    TooMany,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedLink {
    pub url: String,
    pub reason: SkipReason,
}

/// Why the crawl stopped before its queue was empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlStop {
    PageLimit,
    Deadline,
    Cancelled,
    /// The worker failed in a way the lifetime does not survive.
    WorkerFailed,
}

/// One opened page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawledPage {
    pub url: String,
    /// Where the governed navigation committed (after redirects).
    pub final_url: String,
    pub depth: u32,
    pub status: u16,
    pub extraction: Option<PageExtraction>,
    /// Why the page was not read: a refused or failed navigation, a non-2xx
    /// status, a redirect out of scope or onto a page already read, or a
    /// failed observation or extraction.
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlReport {
    pub pages: Vec<CrawledPage>,
    /// Every in-scope URL the crawl found, opened or not, in discovery
    /// order (the map).
    pub discovered: Vec<String>,
    pub skipped: Vec<SkippedLink>,
    /// Skipped links past MAX_SKIPPED_REPORTED.
    pub skipped_unreported: usize,
    pub stopped: Option<CrawlStop>,
}

/// One page as the driver saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub final_url: String,
    pub status: u16,
}

/// The browser side of a crawl.
pub trait CrawlDriver {
    /// Open `url` through the governed navigation.
    fn open(&mut self, url: &str) -> Result<Visit, HostError>;
    /// The links of the committed page, as observed (absolute or
    /// root-relative URLs as written).
    fn links(&mut self) -> Result<Vec<String>, HostError>;
    fn extract(&mut self, options: ExtractOptions) -> Result<PageExtraction, HostError>;
}

/// The crawl's origin and scope, from its start URL.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Scope {
    /// `https://host` (port 443 implied).
    base: String,
    /// The start URL's directory, ending in `/`.
    prefix: String,
    include: Vec<String>,
    exclude: Vec<String>,
}

fn invalid(message: &str) -> HostError {
    HostError::Invalid(message.into())
}

/// Path and query of an `https` URL on `base`, without its fragment; `None`
/// if it is on another origin or not resolvable.
fn split_on(base: &str, url: &str) -> Option<String> {
    let resolved = if url.starts_with('/') && !url.starts_with("//") {
        format!("{base}{url}")
    } else {
        url.to_owned()
    };
    let target = crate::navigation::parse_navigation_url(&resolved).ok()?;
    let rest = resolved.get("https://".len()..)?;
    let path = rest.find('/').map_or("/", |at| &rest[at..]);
    if format!("https://{}", target.host) != base {
        return None;
    }
    let path = path.split('#').next().unwrap_or("/");
    Some(if path.is_empty() {
        "/".into()
    } else {
        path.to_owned()
    })
}

fn unnormalized(path: &str) -> bool {
    let only_path = path.split('?').next().unwrap_or(path);
    let lower = only_path.to_ascii_lowercase();
    lower.contains("%2e")
        || only_path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
}

fn not_a_page(path: &str) -> bool {
    let only_path = path.split('?').next().unwrap_or(path).to_ascii_lowercase();
    NOT_PAGES.iter().any(|ending| only_path.ends_with(ending))
}

fn checked_patterns(patterns: &[String]) -> Result<Vec<String>, HostError> {
    if patterns.len() > MAX_PATH_PATTERNS {
        return Err(invalid("too many crawl path patterns"));
    }
    for pattern in patterns {
        if !pattern.starts_with('/')
            || pattern.len() > MAX_PATH_PATTERN_BYTES
            || pattern.chars().any(|ch| ch.is_control() || ch == '#')
        {
            return Err(invalid(
                "a crawl path pattern must be a bounded absolute path",
            ));
        }
    }
    Ok(patterns.to_vec())
}

impl Scope {
    fn of(spec: &CrawlSpec) -> Result<(Self, String), HostError> {
        let target = crate::navigation::parse_navigation_url(&spec.start_url)?;
        let base = format!("https://{}", target.host);
        let path = split_on(&base, &spec.start_url)
            .ok_or_else(|| invalid("the crawl start URL cannot be resolved"))?;
        if unnormalized(&path) {
            return Err(invalid("the crawl start URL is not normalized"));
        }
        let only_path = path.split('?').next().unwrap_or("/");
        let prefix = match only_path.rfind('/') {
            Some(at) => only_path[..=at].to_owned(),
            None => "/".to_owned(),
        };
        let scope = Self {
            base: base.clone(),
            prefix,
            include: checked_patterns(&spec.include_paths)?,
            exclude: checked_patterns(&spec.exclude_paths)?,
        };
        Ok((scope, format!("{base}{path}")))
    }

    /// The normalized URL to open, or why not.
    fn admit(&self, link: &str) -> Result<String, SkipReason> {
        let path = split_on(&self.base, link).ok_or(SkipReason::OtherOrigin)?;
        if unnormalized(&path) {
            return Err(SkipReason::Unnormalized);
        }
        if !path.starts_with(&self.prefix) {
            return Err(SkipReason::OutOfScope);
        }
        if not_a_page(&path) {
            return Err(SkipReason::NotAPage);
        }
        let included =
            self.include.is_empty() || self.include.iter().any(|p| path.starts_with(p.as_str()));
        if !included || self.exclude.iter().any(|p| path.starts_with(p.as_str())) {
            return Err(SkipReason::Filtered);
        }
        Ok(format!("{}{path}", self.base))
    }
}

fn check_spec(spec: &CrawlSpec) -> Result<(), HostError> {
    if spec.max_pages == 0 || spec.max_pages > MAX_CRAWL_PAGES {
        return Err(invalid("max_pages must be 1 to MAX_CRAWL_PAGES"));
    }
    if spec.max_depth > MAX_CRAWL_DEPTH {
        return Err(invalid("max_depth is above MAX_CRAWL_DEPTH"));
    }
    if spec.deadline.is_zero() || spec.deadline > MAX_CRAWL_DURATION {
        return Err(invalid(
            "the crawl deadline must be within MAX_CRAWL_DURATION",
        ));
    }
    Ok(())
}

/// Whether a driver error ends the browser lifetime (the crawl stops) or
/// only this page.
fn fatal(error: &HostError) -> bool {
    matches!(error, HostError::Unavailable(_))
}

/// Run a crawl. A refused spec is an error; everything after the start is
/// in the report. `cancelled` and the deadline are checked between pages.
pub fn crawl(
    driver: &mut dyn CrawlDriver,
    spec: &CrawlSpec,
    cancelled: &AtomicBool,
) -> Result<CrawlReport, HostError> {
    check_spec(spec)?;
    let (scope, start) = Scope::of(spec)?;
    let scope_start = scope
        .admit(&start)
        .map_err(|_| invalid("the crawl start URL is outside its own filters"))?;
    let started = Instant::now();
    let mut report = CrawlReport {
        pages: Vec::new(),
        discovered: vec![scope_start.clone()],
        skipped: Vec::new(),
        skipped_unreported: 0,
        stopped: None,
    };
    let mut seen: HashSet<String> = HashSet::from([scope_start.clone()]);
    // Final URLs already read, so a redirect onto one is not read twice.
    let mut read: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::from([(scope_start, 0)]);
    let skip = |report: &mut CrawlReport, url: String, reason: SkipReason| {
        if report.skipped.len() < MAX_SKIPPED_REPORTED {
            report.skipped.push(SkippedLink { url, reason });
        } else {
            report.skipped_unreported += 1;
        }
    };
    while let Some((url, depth)) = queue.pop_front() {
        if cancelled.load(Ordering::SeqCst) {
            report.stopped = Some(CrawlStop::Cancelled);
            break;
        }
        if started.elapsed() >= spec.deadline {
            report.stopped = Some(CrawlStop::Deadline);
            break;
        }
        if report.pages.len() as u32 >= spec.max_pages {
            report.stopped = Some(CrawlStop::PageLimit);
            break;
        }
        let mut page = CrawledPage {
            url: url.clone(),
            final_url: String::new(),
            depth,
            status: 0,
            extraction: None,
            failure: None,
        };
        let visit = match driver.open(&url) {
            Ok(visit) => visit,
            Err(error) => {
                let stop = fatal(&error);
                page.failure = Some(error.to_string());
                report.pages.push(page);
                if stop {
                    report.stopped = Some(CrawlStop::WorkerFailed);
                    break;
                }
                continue;
            }
        };
        page.final_url = visit.final_url.clone();
        page.status = visit.status;
        let in_scope = scope.admit(&visit.final_url);
        let failure = if !(200..300).contains(&visit.status) {
            Some(format!("status {}", visit.status))
        } else if in_scope.is_err() {
            Some("redirected out of the crawl scope".to_owned())
        } else if !read.insert(visit.final_url.clone()) {
            Some("redirected onto a page already read".to_owned())
        } else {
            None
        };
        if failure.is_some() {
            page.failure = failure;
            report.pages.push(page);
            continue;
        }
        if let Some(options) = spec.extract {
            match driver.extract(options) {
                Ok(extraction) => page.extraction = Some(extraction),
                Err(error) if fatal(&error) => {
                    page.failure = Some(error.to_string());
                    report.pages.push(page);
                    report.stopped = Some(CrawlStop::WorkerFailed);
                    break;
                }
                Err(error) => page.failure = Some(error.to_string()),
            }
        }
        let links = match driver.links() {
            Ok(links) => links,
            Err(error) => {
                let stop = fatal(&error);
                page.failure.get_or_insert(error.to_string());
                report.pages.push(page);
                if stop {
                    report.stopped = Some(CrawlStop::WorkerFailed);
                    break;
                }
                continue;
            }
        };
        report.pages.push(page);
        for link in links {
            let admitted = match scope.admit(&link) {
                Ok(admitted) => admitted,
                Err(reason) => {
                    skip(&mut report, link, reason);
                    continue;
                }
            };
            if seen.contains(&admitted) {
                continue;
            }
            if depth + 1 > spec.max_depth {
                skip(&mut report, admitted, SkipReason::TooDeep);
                continue;
            }
            if seen.len() >= MAX_CRAWL_URLS {
                skip(&mut report, admitted, SkipReason::TooMany);
                continue;
            }
            seen.insert(admitted.clone());
            report.discovered.push(admitted.clone());
            queue.push_back((admitted, depth + 1));
        }
    }
    if report.stopped.is_none() && !queue.is_empty() {
        report.stopped = Some(CrawlStop::PageLimit);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::UnavailableReason;
    use std::collections::HashMap;

    /// A site as a map from URL to (final URL, status, links).
    struct Site {
        pages: HashMap<String, (String, u16, Vec<String>)>,
        opened: Vec<String>,
        current: Option<String>,
        fail_extract: bool,
    }

    impl Site {
        fn new(pages: &[(&str, &str, u16, &[&str])]) -> Self {
            Self {
                pages: pages
                    .iter()
                    .map(|(url, end, status, links)| {
                        (
                            (*url).to_owned(),
                            (
                                (*end).to_owned(),
                                *status,
                                links.iter().map(|l| (*l).to_owned()).collect(),
                            ),
                        )
                    })
                    .collect(),
                opened: Vec::new(),
                current: None,
                fail_extract: false,
            }
        }
    }

    impl CrawlDriver for Site {
        fn open(&mut self, url: &str) -> Result<Visit, HostError> {
            self.opened.push(url.to_owned());
            let (end, status, _) = self
                .pages
                .get(url)
                .cloned()
                .ok_or_else(|| HostError::Invalid("navigation denied".into()))?;
            if status == 0 {
                return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation));
            }
            // The committed page is the final one, after redirects.
            self.current = Some(end.clone());
            Ok(Visit {
                final_url: end,
                status,
            })
        }
        fn links(&mut self) -> Result<Vec<String>, HostError> {
            let current = self.current.clone().unwrap_or_default();
            Ok(self
                .pages
                .get(&current)
                .map(|(_, _, links)| links.clone())
                .unwrap_or_default())
        }
        fn extract(&mut self, _options: ExtractOptions) -> Result<PageExtraction, HostError> {
            if self.fail_extract {
                return Err(HostError::Invalid("stale page".into()));
            }
            Ok(PageExtraction {
                url: self.current.clone().unwrap_or_default(),
                document_generation: self.opened.len() as u64,
                truncated: false,
                chunks: Vec::new(),
            })
        }
    }

    const S: &str = "https://site.test";

    fn spec(start: &str) -> CrawlSpec {
        CrawlSpec {
            start_url: start.into(),
            max_pages: 20,
            max_depth: 3,
            include_paths: Vec::new(),
            exclude_paths: Vec::new(),
            extract: Some(ExtractOptions::default()),
            deadline: Duration::from_secs(60),
        }
    }

    fn site() -> Site {
        Site::new(&[
            (
                "https://site.test/docs/",
                "https://site.test/docs/",
                200,
                &[
                    "/docs/a",
                    "https://site.test/docs/b#part",
                    "https://other.test/docs/x",
                    "/elsewhere",
                    "/docs/manual.pdf",
                    "/docs/../admin",
                    "/docs/%2e%2e/admin",
                    "javascript:alert(1)",
                    "mailto:a@site.test",
                    "relative/page",
                    "//site.test/docs/protocol-relative",
                    "https://user:pw@site.test/docs/creds",
                    "/docs\\..\\admin",
                    "https://site.test:8443/docs/port",
                    "https://site.test.evil.test/docs/x",
                    "https://127.0.0.1/docs/x",
                    "/docs/a",
                    "/docs/private/x",
                ],
            ),
            (
                "https://site.test/docs/a",
                "https://site.test/docs/a",
                200,
                &["/docs/c", "/docs/"],
            ),
            (
                "https://site.test/docs/b",
                "https://site.test/docs/c",
                200,
                &[],
            ),
            (
                "https://site.test/docs/c",
                "https://site.test/docs/c",
                200,
                &["/docs/d"],
            ),
            (
                "https://site.test/docs/d",
                "https://site.test/docs/d",
                404,
                &["/docs/e"],
            ),
            (
                "https://site.test/docs/private/x",
                "https://site.test/docs/private/x",
                200,
                &[],
            ),
        ])
    }

    fn reasons(report: &CrawlReport) -> Vec<(&str, SkipReason)> {
        report
            .skipped
            .iter()
            .map(|s| (s.url.as_str(), s.reason))
            .collect()
    }

    #[test]
    fn a_crawl_stays_on_its_origin_and_directory_and_reads_each_page_once() {
        let mut driver = site();
        let mut wanted = spec("https://site.test/docs/");
        wanted.exclude_paths = vec!["/docs/private".into()];
        let report = crawl(&mut driver, &wanted, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            driver.opened,
            [
                "https://site.test/docs/",
                "https://site.test/docs/a",
                "https://site.test/docs/b",
                "https://site.test/docs/c",
                "https://site.test/docs/d",
            ]
        );
        let read: Vec<(&str, bool)> = report
            .pages
            .iter()
            .map(|p| (p.url.as_str(), p.extraction.is_some()))
            .collect();
        assert_eq!(
            read,
            [
                ("https://site.test/docs/", true),
                ("https://site.test/docs/a", true),
                // b redirects onto c before c is opened: read as b's final URL.
                ("https://site.test/docs/b", true),
                // c itself is then a page already read.
                ("https://site.test/docs/c", false),
                ("https://site.test/docs/d", false),
            ]
        );
        assert_eq!(
            report.pages[3].failure.as_deref(),
            Some("redirected onto a page already read")
        );
        assert_eq!(report.pages[4].failure.as_deref(), Some("status 404"));
        assert_eq!(
            reasons(&report),
            [
                ("https://other.test/docs/x", SkipReason::OtherOrigin),
                ("/elsewhere", SkipReason::OutOfScope),
                ("/docs/manual.pdf", SkipReason::NotAPage),
                ("/docs/../admin", SkipReason::Unnormalized),
                ("/docs/%2e%2e/admin", SkipReason::Unnormalized),
                ("javascript:alert(1)", SkipReason::OtherOrigin),
                ("mailto:a@site.test", SkipReason::OtherOrigin),
                ("relative/page", SkipReason::OtherOrigin),
                (
                    "//site.test/docs/protocol-relative",
                    SkipReason::OtherOrigin
                ),
                (
                    "https://user:pw@site.test/docs/creds",
                    SkipReason::OtherOrigin
                ),
                ("/docs\\..\\admin", SkipReason::OtherOrigin),
                ("https://site.test:8443/docs/port", SkipReason::OtherOrigin),
                (
                    "https://site.test.evil.test/docs/x",
                    SkipReason::OtherOrigin
                ),
                ("https://127.0.0.1/docs/x", SkipReason::OtherOrigin),
                ("/docs/private/x", SkipReason::Filtered),
            ]
        );
        assert_eq!(report.stopped, None);
        // The map: every in-scope URL found, fragments dropped, once.
        assert_eq!(
            report.discovered,
            [
                "https://site.test/docs/",
                "https://site.test/docs/a",
                "https://site.test/docs/b",
                "https://site.test/docs/c",
                "https://site.test/docs/d",
            ]
        );
    }

    #[test]
    fn depth_page_count_cancellation_and_a_lost_worker_stop_the_crawl() {
        let mut shallow = spec("https://site.test/docs/");
        shallow.max_depth = 1;
        let mut driver = site();
        let report = crawl(&mut driver, &shallow, &AtomicBool::new(false)).unwrap();
        assert!(!driver
            .opened
            .contains(&"https://site.test/docs/c".to_owned()));
        assert!(reasons(&report).contains(&("https://site.test/docs/c", SkipReason::TooDeep)));

        let mut few = spec("https://site.test/docs/");
        few.max_pages = 2;
        let mut driver = site();
        let report = crawl(&mut driver, &few, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.pages.len(), 2);
        assert_eq!(report.stopped, Some(CrawlStop::PageLimit));

        let mut driver = site();
        let report = crawl(
            &mut driver,
            &spec("https://site.test/docs/"),
            &AtomicBool::new(true),
        )
        .unwrap();
        assert!(driver.opened.is_empty());
        assert_eq!(report.stopped, Some(CrawlStop::Cancelled));

        let mut driver = site();
        driver.pages.insert(
            "https://site.test/docs/a".into(),
            (String::new(), 0, Vec::new()),
        );
        let report = crawl(
            &mut driver,
            &spec("https://site.test/docs/"),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.stopped, Some(CrawlStop::WorkerFailed));
        assert_eq!(driver.opened.len(), 2);
    }

    #[test]
    fn the_deadline_is_checked_between_pages() {
        struct Slow(Site);
        impl CrawlDriver for Slow {
            fn open(&mut self, url: &str) -> Result<Visit, HostError> {
                std::thread::sleep(Duration::from_millis(30));
                self.0.open(url)
            }
            fn links(&mut self) -> Result<Vec<String>, HostError> {
                self.0.links()
            }
            fn extract(&mut self, options: ExtractOptions) -> Result<PageExtraction, HostError> {
                self.0.extract(options)
            }
        }
        let mut quick = spec("https://site.test/docs/");
        quick.deadline = Duration::from_millis(10);
        let mut driver = Slow(site());
        let report = crawl(&mut driver, &quick, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.pages.len(), 1);
        assert_eq!(report.stopped, Some(CrawlStop::Deadline));
    }

    #[test]
    fn a_map_opens_pages_without_extracting_and_a_failed_extraction_is_recorded() {
        let mut map = spec("https://site.test/docs/");
        map.extract = None;
        let mut driver = site();
        let report = crawl(&mut driver, &map, &AtomicBool::new(false)).unwrap();
        assert!(report.pages.iter().all(|p| p.extraction.is_none()));
        // No exclude here: /docs/private/x is in scope too.
        assert_eq!(report.discovered.len(), 6);

        let mut driver = site();
        driver.fail_extract = true;
        let report = crawl(
            &mut driver,
            &spec("https://site.test/docs/"),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(report.pages[0].failure.is_some());
        // Its links are still followed.
        assert!(report.pages.len() > 1);
    }

    #[test]
    fn includes_scope_and_bad_specs() {
        let mut only_a = spec("https://site.test/docs/");
        only_a.include_paths = vec!["/docs/a".into()];
        // The start page must pass its own filters.
        assert!(crawl(&mut site(), &only_a, &AtomicBool::new(false)).is_err());
        only_a.include_paths = vec!["/docs/a".into(), "/docs/".into()];
        assert!(crawl(&mut site(), &only_a, &AtomicBool::new(false)).is_ok());
        // A start page names its directory as the scope.
        let (scope, start) = Scope::of(&spec("https://site.test/docs/intro?x=1#top")).unwrap();
        assert_eq!(scope.prefix, "/docs/");
        assert_eq!(start, "https://site.test/docs/intro?x=1");
        for bad in [
            "http://site.test/",
            "https://site.test:8443/",
            "https://user@site.test/",
            "https://site.test/a/../b",
            "javascript:alert(1)",
        ] {
            assert!(
                crawl(&mut site(), &spec(bad), &AtomicBool::new(false)).is_err(),
                "{bad}"
            );
        }
        let mut wide = spec(S);
        wide.max_pages = MAX_CRAWL_PAGES + 1;
        assert!(crawl(&mut site(), &wide, &AtomicBool::new(false)).is_err());
        let mut deep = spec(S);
        deep.max_depth = MAX_CRAWL_DEPTH + 1;
        assert!(crawl(&mut site(), &deep, &AtomicBool::new(false)).is_err());
        let mut long = spec(S);
        long.deadline = MAX_CRAWL_DURATION + Duration::from_secs(1);
        assert!(crawl(&mut site(), &long, &AtomicBool::new(false)).is_err());
        let mut patterns = spec(S);
        patterns.exclude_paths = vec!["no-slash".into()];
        assert!(crawl(&mut site(), &patterns, &AtomicBool::new(false)).is_err());
        patterns.exclude_paths = vec!["/x".into(); MAX_PATH_PATTERNS + 1];
        assert!(crawl(&mut site(), &patterns, &AtomicBool::new(false)).is_err());
    }

    #[test]
    fn the_url_memory_and_the_skip_report_are_bounded() {
        let many: Vec<String> = (0..MAX_CRAWL_URLS + 50)
            .map(|i| format!("/docs/p{i}"))
            .collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        let mut driver = Site::new(&[(
            "https://site.test/docs/",
            "https://site.test/docs/",
            200,
            &refs,
        )]);
        let mut wanted = spec("https://site.test/docs/");
        wanted.max_pages = 1;
        let report = crawl(&mut driver, &wanted, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.discovered.len(), MAX_CRAWL_URLS);
        // The start URL is one of the remembered ones.
        assert_eq!(report.skipped.len(), 51);
        assert!(report
            .skipped
            .iter()
            .all(|s| s.reason == SkipReason::TooMany));
        let junk: Vec<String> = (0..MAX_SKIPPED_REPORTED + 7)
            .map(|i| format!("https://other.test/{i}"))
            .collect();
        let refs: Vec<&str> = junk.iter().map(String::as_str).collect();
        let mut driver = Site::new(&[(
            "https://site.test/docs/",
            "https://site.test/docs/",
            200,
            &refs,
        )]);
        let report = crawl(
            &mut driver,
            &spec("https://site.test/docs/"),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.skipped.len(), MAX_SKIPPED_REPORTED);
        assert_eq!(report.skipped_unreported, 7);
    }
}
