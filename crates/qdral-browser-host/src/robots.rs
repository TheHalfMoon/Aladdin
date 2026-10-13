//! #317: robots.txt (RFC 9309) and sitemap reading for the crawl. Pure,
//! bounded parsers over text the worker fetched through a governed
//! navigation; nothing here performs I/O.
//!
//! Aladdin-native: the behaviour follows RFC 9309 and the sitemaps.org
//! protocol, as Firecrawl's crawl does; no Firecrawl code is copied.

/// The product token Deskal's crawl matches `user-agent` lines against.
pub const ROBOTS_PRODUCT_TOKEN: &str = "deskal";
/// Rules kept from one robots.txt (later ones are ignored).
pub const MAX_ROBOTS_RULES: usize = 512;
/// Bytes of one rule's path pattern (longer rules are ignored).
pub const MAX_ROBOTS_PATTERN_BYTES: usize = 512;
/// Sitemap URLs kept from one robots.txt.
pub const MAX_ROBOTS_SITEMAPS: usize = 8;
/// `<loc>` entries kept from one sitemap.
pub const MAX_SITEMAP_LOCS: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rule {
    allow: bool,
    pattern: String,
}

/// The rules that apply to Deskal, and the sitemaps a robots.txt names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Robots {
    rules: Vec<Rule>,
    /// Nothing may be crawled (robots.txt unreachable, RFC 9309 2.3.1.4).
    disallow_all: bool,
    pub sitemaps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    /// Lines before any `user-agent`.
    None,
    Ours,
    Star,
    Other,
}

impl Robots {
    /// No robots.txt (a 4xx answer): everything is allowed.
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// An unreachable robots.txt (a 5xx answer or no answer): nothing is.
    pub fn disallow_all() -> Self {
        Self {
            disallow_all: true,
            ..Self::default()
        }
    }

    /// Parse a robots.txt. The groups naming Deskal's product token apply;
    /// if none does, the `*` groups do; a group is one or more
    /// `user-agent` lines and the rules after them. Unknown lines and
    /// over-long rules are ignored.
    pub fn parse(text: &str) -> Self {
        let mut ours: Vec<Rule> = Vec::new();
        let mut star: Vec<Rule> = Vec::new();
        let mut matched_ours = false;
        let mut sitemaps = Vec::new();
        // The agents of the group being read, and whether a rule has been
        // seen since its last `user-agent` line.
        let mut current = (false, false);
        let mut group = Group::None;
        let mut in_rules = true;
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "user-agent" => {
                    if in_rules {
                        current = (false, false);
                        in_rules = false;
                    }
                    let token = value.to_ascii_lowercase();
                    if token == "*" {
                        current.1 = true;
                    } else if token == ROBOTS_PRODUCT_TOKEN {
                        current.0 = true;
                    }
                    group = if current.0 {
                        matched_ours = true;
                        Group::Ours
                    } else if current.1 {
                        Group::Star
                    } else {
                        Group::Other
                    };
                }
                "allow" | "disallow" => {
                    in_rules = true;
                    // An empty disallow is no rule; patterns must be paths.
                    if value.is_empty()
                        || value.len() > MAX_ROBOTS_PATTERN_BYTES
                        || !(value.starts_with('/') || value.starts_with('*'))
                    {
                        continue;
                    }
                    let rule = Rule {
                        allow: key == "allow",
                        pattern: value.to_owned(),
                    };
                    let target = match group {
                        Group::Ours => &mut ours,
                        Group::Star => &mut star,
                        Group::None | Group::Other => continue,
                    };
                    if target.len() < MAX_ROBOTS_RULES {
                        target.push(rule);
                    }
                }
                "sitemap"
                    if sitemaps.len() < MAX_ROBOTS_SITEMAPS && value.starts_with("https://") =>
                {
                    sitemaps.push(value.to_owned());
                }
                _ => {}
            }
        }
        Self {
            rules: if matched_ours { ours } else { star },
            disallow_all: false,
            sitemaps,
        }
    }

    /// Whether `path` (path and query, as requested) may be crawled: the
    /// longest matching pattern decides, an allow wins a tie, and no match
    /// allows. `/robots.txt` itself is always allowed.
    pub fn allowed(&self, path: &str) -> bool {
        if self.disallow_all {
            return false;
        }
        if path == "/robots.txt" {
            return true;
        }
        let mut best: Option<(usize, bool)> = None;
        for rule in &self.rules {
            if !matches(&rule.pattern, path) {
                continue;
            }
            let length = rule.pattern.len();
            best = match best {
                Some((len, allow)) if len > length || (len == length && allow) => {
                    Some((len, allow))
                }
                _ => Some((length, rule.allow)),
            };
        }
        best.is_none_or(|(_, allow)| allow)
    }
}

/// RFC 9309 pattern matching: the pattern matches from the start of the
/// path; `*` matches any sequence and a final `$` anchors the end. Greedy
/// leftmost matching of the literal segments is exact for `*`-only globs
/// and needs no backtracking.
fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(rest) => (rest, true),
        None => (pattern, false),
    };
    let segments: Vec<&str> = pattern.split('*').collect();
    let Some((first, rest)) = segments.split_first() else {
        return true;
    };
    let Some(mut at) = path.starts_with(first).then_some(first.len()) else {
        return false;
    };
    let Some((last, middle)) = rest.split_last() else {
        // No `*`: a prefix match, or an exact one when anchored.
        return !anchored || at == path.len();
    };
    for segment in middle {
        match path[at..].find(segment) {
            Some(found) => at += found + segment.len(),
            None => return false,
        }
    }
    if anchored {
        // The last segment must end the path, after everything matched.
        path.len() >= at + last.len() && path.ends_with(last)
    } else {
        path[at..].contains(last)
    }
}

/// The `<loc>` URLs of a sitemap (sitemaps.org), with XML's five entities
/// decoded, and whether it is a sitemap index (its locations are sitemaps,
/// not pages). Linear; at most MAX_SITEMAP_LOCS locations.
pub fn sitemap_locations(text: &str) -> (Vec<String>, bool) {
    let lower = text.to_ascii_lowercase();
    let index = lower.contains("<sitemapindex");
    let mut locations = Vec::new();
    let mut from = 0;
    while locations.len() < MAX_SITEMAP_LOCS {
        let Some(open) = lower[from..].find("<loc>") else {
            break;
        };
        let start = from + open + "<loc>".len();
        let Some(close) = lower[start..].find("</loc>") else {
            break;
        };
        let end = start + close;
        let raw = text[start..end].trim();
        let raw = raw
            .strip_prefix("<![CDATA[")
            .and_then(|r| r.strip_suffix("]]>"))
            .unwrap_or(raw)
            .trim();
        let decoded = raw
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&");
        if !decoded.is_empty() {
            locations.push(decoded);
        }
        from = end + "</loc>".len();
    }
    (locations, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_group_wins_over_star_and_the_longest_match_decides() {
        let robots = Robots::parse(
            "# comment\nUser-agent: *\nDisallow: /\n\nUser-agent: OtherBot\nUser-agent: Deskal\nDisallow: /private\nAllow: /private/open\nDisallow: /*.php$\nDisallow: /tmp*/cache\nSitemap: https://site.test/sitemap.xml\nSitemap: http://site.test/insecure.xml\n",
        );
        assert!(robots.allowed("/"));
        assert!(robots.allowed("/docs/a"));
        assert!(!robots.allowed("/private"));
        assert!(!robots.allowed("/private/x"));
        assert!(robots.allowed("/private/open/x"));
        assert!(!robots.allowed("/index.php"));
        assert!(robots.allowed("/index.php?x=1"), "$ anchors the end");
        assert!(!robots.allowed("/tmp/a/cache"));
        assert!(!robots.allowed("/tmpx/cache/y"));
        assert!(robots.allowed("/tmp/a"));
        assert!(robots.allowed("/robots.txt"));
        assert_eq!(robots.sitemaps, ["https://site.test/sitemap.xml"]);
    }

    #[test]
    fn without_our_group_the_star_group_applies_and_ties_allow() {
        let robots = Robots::parse(
            "User-agent: googlebot\nDisallow: /\nUser-agent: *\nDisallow: /a\nAllow: /a\nDisallow:\n",
        );
        assert!(robots.allowed("/a/b"), "a tie is an allow");
        assert!(robots.allowed("/b"));
        // A group must name the product token itself, not a prefix of it.
        assert!(Robots::parse("User-agent: desk\nDisallow: /\n").allowed("/x"));
        assert!(!Robots::parse("User-agent: DESKAL\nDisallow: /\n").allowed("/x"));
        let none = Robots::parse("User-agent: googlebot\nDisallow: /\n");
        assert!(none.allowed("/anything"));
        // Rules before any user-agent line belong to no group.
        assert!(Robots::parse("Disallow: /\n").allowed("/x"));
        assert!(Robots::allow_all().allowed("/x"));
        assert!(!Robots::disallow_all().allowed("/x"));
        assert!(!Robots::disallow_all().allowed("/robots.txt"));
    }

    #[test]
    fn patterns_match_from_the_start_with_stars_and_an_end_anchor() {
        assert!(matches("/a", "/abc"));
        assert!(!matches("/a", "/b/a"));
        assert!(matches("/a$", "/a"));
        assert!(!matches("/a$", "/ab"));
        assert!(matches("*", "/x"));
        assert!(matches("/*.pdf$", "/docs/x.pdf"));
        assert!(!matches("/*.pdf$", "/docs/x.pdf.html"));
        assert!(matches("/*/b*/c", "/x/by/z/c"));
        assert!(!matches("/*/b*/c", "/x/y/z"));
        assert!(matches("/a*a$", "/aa"));
        assert!(
            !matches("/ab*b$", "/ab"),
            "the end segment may not overlap the start"
        );
        // Many stars on a long path stay fast (no backtracking).
        let pattern = format!("/{}x$", "*a".repeat(200));
        let path = format!("/{}", "a".repeat(2000));
        assert!(!matches(&pattern, &path));
    }

    #[test]
    fn rule_and_sitemap_counts_are_bounded() {
        let mut text = String::from("User-agent: *\n");
        for i in 0..MAX_ROBOTS_RULES + 10 {
            text.push_str(&format!("Disallow: /p{i}\n"));
        }
        text.push_str(&format!(
            "Disallow: /{}\n",
            "x".repeat(MAX_ROBOTS_PATTERN_BYTES)
        ));
        for i in 0..MAX_ROBOTS_SITEMAPS + 3 {
            text.push_str(&format!("Sitemap: https://site.test/s{i}.xml\n"));
        }
        let robots = Robots::parse(&text);
        assert_eq!(robots.rules.len(), MAX_ROBOTS_RULES);
        assert_eq!(robots.sitemaps.len(), MAX_ROBOTS_SITEMAPS);
        assert!(robots.allowed(&format!("/{}", "x".repeat(MAX_ROBOTS_PATTERN_BYTES))));
    }

    #[test]
    fn sitemaps_yield_their_locations_and_say_whether_they_are_an_index() {
        let (urls, index) = sitemap_locations(
            "<?xml version=\"1.0\"?><urlset><url><LOC> https://site.test/a?x=1&amp;y=2 </LOC></url><url><loc><![CDATA[https://site.test/b]]></loc></url><url><loc></loc></url><url><loc>https://site.test/unclosed",
        );
        assert_eq!(urls, ["https://site.test/a?x=1&y=2", "https://site.test/b"]);
        assert!(!index);
        let (urls, index) = sitemap_locations(
            "<sitemapindex><sitemap><loc>https://site.test/s1.xml</loc></sitemap></sitemapindex>",
        );
        assert_eq!(urls, ["https://site.test/s1.xml"]);
        assert!(index);
        let many = "<loc>https://site.test/x</loc>".repeat(MAX_SITEMAP_LOCS + 5);
        assert_eq!(sitemap_locations(&many).0.len(), MAX_SITEMAP_LOCS);
    }
}
