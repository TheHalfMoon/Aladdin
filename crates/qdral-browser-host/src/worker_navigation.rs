//! SG-000108 slice 3a: the host's per-request navigation decisions for the
//! live worker.
//!
//! The worker forwards every request the engine makes (top-level documents,
//! redirects, frames, popups, fetch/XHR and other subresources, service
//! workers) as a `route_request`; this module decides each one with the
//! SG-000075 policy and the worker only applies the answer. Destinations are
//! fixed for an engine lifetime: they were admitted with
//! `validate_navigation` (resolution and public-address pinning) and the
//! engine's resolver is pinned to them, so a request may only name one of
//! those hosts, and the mediation rules (exact-origin frames, popups and
//! subresources, bounded same-origin redirect chains) then apply unchanged.

use crate::confinement::AdmittedDestination;
use crate::error::HostError;
use crate::navigation::{
    mediate_frame, mediate_popup, mediate_service_worker, mediate_subresource,
    parse_navigation_url, NavigationTarget, RedirectChain, SubresourceKind,
};

/// What kind of request the worker is asking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    /// A top-level document of the primary page.
    Document,
    /// A redirect hop of a document, frame or popup navigation.
    Redirect,
    /// A child frame's document.
    Frame,
    /// A popup or new window's document.
    Popup,
    /// `fetch` or XHR.
    Fetch,
    /// Any other subresource (script, style, image, font, media, ...).
    Subresource,
    /// A service-worker script or a request made by a service worker.
    ServiceWorker,
}

impl RouteKind {
    /// The closed wire vocabulary.
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "document" => Self::Document,
            "redirect" => Self::Redirect,
            "frame" => Self::Frame,
            "popup" => Self::Popup,
            "fetch" => Self::Fetch,
            "subresource" => Self::Subresource,
            "service_worker" => Self::ServiceWorker,
            _ => return None,
        })
    }
}

/// Per-lifetime navigation state owned by the host.
#[derive(Debug)]
pub struct NavigationPolicy {
    admitted: Vec<String>,
    expected: Option<String>,
    begun_origin: Option<String>,
    chain: Option<RedirectChain>,
    committed_origin: Option<String>,
}

impl std::fmt::Debug for RedirectChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedirectChain")
            .field("hops", &self.len())
            .finish()
    }
}

impl NavigationPolicy {
    pub fn new(destinations: &[AdmittedDestination]) -> Self {
        Self {
            admitted: destinations.iter().map(|d| d.host().to_owned()).collect(),
            expected: None,
            begun_origin: None,
            chain: None,
            committed_origin: None,
        }
    }

    /// Parse with the SG-000075 parser and require an admitted host.
    fn admitted_target(&self, url: &str) -> Result<NavigationTarget, HostError> {
        let target = parse_navigation_url(url)?;
        if !self.admitted.iter().any(|host| host == &target.host) {
            return Err(HostError::Invalid(format!(
                "{} is not admitted for this browser lifetime",
                target.host
            )));
        }
        Ok(target)
    }

    /// Start a host-requested top-level navigation.
    pub fn begin(&mut self, url: &str) -> Result<NavigationTarget, HostError> {
        let target = self.admitted_target(url)?;
        self.chain = Some(RedirectChain::start(url)?);
        self.expected = Some(url.to_owned());
        self.begun_origin = Some(target.origin.clone());
        Ok(target)
    }

    /// Record the committed top-level document after a navigation. The
    /// committed origin must be the one the navigation began with, so even a
    /// redirect hop that escaped interception cannot widen the page.
    pub fn commit(&mut self, url: &str) -> Result<(), HostError> {
        let target = self.admitted_target(url)?;
        let begun = self.begun_origin.take();
        if begun.as_deref() != Some(target.origin.as_str()) {
            self.expected = None;
            self.chain = None;
            return Err(HostError::Invalid(
                "the committed document left the navigation's origin".into(),
            ));
        }
        self.committed_origin = Some(target.origin);
        self.expected = None;
        self.chain = None;
        Ok(())
    }

    pub fn committed_origin(&self) -> Option<&str> {
        self.committed_origin.as_deref()
    }

    /// Decide one forwarded request. `origin` is the requesting context's
    /// origin (parent frame, opener, or document), empty for a top-level
    /// document request.
    pub fn decide(&mut self, kind: RouteKind, url: &str, origin: &str) -> Result<(), HostError> {
        let target = self.admitted_target(url)?;
        match kind {
            RouteKind::Document => {
                if self.expected.as_deref() == Some(url) {
                    return Ok(());
                }
                // A page-initiated top-level navigation stays within the
                // committed origin and starts its own redirect chain.
                match self.committed_origin.as_deref() {
                    Some(committed) if committed == target.origin => {
                        self.chain = Some(RedirectChain::start(url)?);
                        Ok(())
                    }
                    _ => Err(HostError::Invalid(
                        "top-level navigation widening denied".into(),
                    )),
                }
            }
            RouteKind::Redirect => match self.chain.as_mut() {
                Some(chain) => chain.push(url),
                None => Err(HostError::Invalid(
                    "redirect outside a navigation denied".into(),
                )),
            },
            RouteKind::Frame => mediate_frame(origin, url).map(|_| ()),
            RouteKind::Popup => mediate_popup(origin, url).map(|_| ()),
            RouteKind::Fetch => {
                mediate_subresource(origin, url, SubresourceKind::Fetch).map(|_| ())
            }
            RouteKind::Subresource => {
                mediate_subresource(origin, url, SubresourceKind::Fetch).map(|_| ())
            }
            RouteKind::ServiceWorker => {
                // Service workers are refused outright (SG-000108 #301); the
                // mediation result is irrelevant.
                let _ = mediate_service_worker(origin, url);
                Err(HostError::Invalid("service worker denied".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn policy() -> NavigationPolicy {
        let a =
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST)).unwrap();
        let b =
            AdmittedDestination::fixture("other.test", IpAddr::V4(Ipv4Addr::LOCALHOST)).unwrap();
        NavigationPolicy::new(&[a, b])
    }

    const HOME: &str = "https://fixture.test";

    #[test]
    fn only_admitted_https_destinations_can_begin() {
        let mut p = policy();
        assert!(p.begin("https://fixture.test/reports").is_ok());
        for url in [
            "https://example.com/",
            "http://fixture.test/",
            "https://fixture.test:8443/",
            "https://user@fixture.test/",
            "https://127.0.0.1/",
            "file:///C:/x",
            "javascript:alert(1)",
        ] {
            assert!(p.begin(url).is_err(), "{url}");
        }
    }

    #[test]
    fn documents_redirects_and_commits_follow_the_chain() {
        let mut p = policy();
        p.begin("https://fixture.test/a").unwrap();
        assert!(p
            .decide(RouteKind::Document, "https://fixture.test/a", "")
            .is_ok());
        assert!(p
            .decide(RouteKind::Redirect, "https://fixture.test/b", HOME)
            .is_ok());
        // Widening, loops, and other admitted origins are refused.
        assert!(p
            .decide(RouteKind::Redirect, "https://other.test/c", HOME)
            .is_err());
        assert!(p
            .decide(RouteKind::Redirect, "https://fixture.test/b", HOME)
            .is_err());
        // A commit on another (even admitted) origin is refused.
        let mut widened = policy();
        widened.begin("https://fixture.test/a").unwrap();
        assert!(widened.commit("https://other.test/a").is_err());
        assert!(
            widened.commit("https://fixture.test/a").is_err(),
            "no navigation in progress"
        );
        p.commit("https://fixture.test/b").unwrap();
        // The SG-000075 parser normalizes origins with an explicit port.
        assert_eq!(p.committed_origin(), Some("https://fixture.test:443"));
        // Page-initiated same-origin navigation is allowed, cross-origin not.
        assert!(p
            .decide(RouteKind::Document, "https://fixture.test/next", "")
            .is_ok());
        assert!(p
            .decide(RouteKind::Document, "https://other.test/", "")
            .is_err());
        // A redirect needs an active chain.
        let mut fresh = policy();
        assert!(fresh
            .decide(RouteKind::Redirect, "https://fixture.test/x", HOME)
            .is_err());
        // An unrequested document before any commit is refused.
        assert!(fresh
            .decide(RouteKind::Document, "https://fixture.test/x", "")
            .is_err());
    }

    #[test]
    fn redirect_chains_are_bounded() {
        let mut p = policy();
        p.begin("https://fixture.test/0").unwrap();
        let mut refused = false;
        for hop in 1..=10 {
            if p.decide(
                RouteKind::Redirect,
                &format!("https://fixture.test/{hop}"),
                HOME,
            )
            .is_err()
            {
                refused = true;
                break;
            }
        }
        assert!(refused, "the redirect chain was never limited");
    }

    #[test]
    fn frames_popups_and_subresources_are_exact_origin() {
        let mut p = policy();
        for kind in [
            RouteKind::Frame,
            RouteKind::Popup,
            RouteKind::Fetch,
            RouteKind::Subresource,
        ] {
            assert!(
                p.decide(kind, "https://fixture.test/x", HOME).is_ok(),
                "{kind:?}"
            );
            assert!(
                p.decide(kind, "https://other.test/x", HOME).is_err(),
                "{kind:?}"
            );
            assert!(
                p.decide(kind, "https://example.com/x", HOME).is_err(),
                "{kind:?}"
            );
            assert!(
                p.decide(kind, "http://fixture.test/x", HOME).is_err(),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn service_workers_and_unknown_kinds_are_refused() {
        let mut p = policy();
        assert!(p
            .decide(RouteKind::ServiceWorker, "https://fixture.test/sw.js", HOME)
            .is_err());
        assert_eq!(RouteKind::parse("websocket"), None);
        assert_eq!(RouteKind::parse("Document"), None);
        assert_eq!(RouteKind::parse("document"), Some(RouteKind::Document));
    }
}
