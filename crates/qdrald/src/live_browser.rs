//! SG-000108 product caller, slice 1b: qdrald drives the live browser host
//! under the approval broker (test builds only: the `live-browser-test`
//! feature, which release builds never enable).
//!
//! Decision 3 of docs/security/SG-000108_PRODUCT_CALLER.md, the two-part
//! binding:
//! 1. the host computes its binding for the intended action (`bind`);
//! 2. qdrald prompts for, and the broker mints and consumes, an approval
//!    whose digest covers qdrald's own material (workspace, policy
//!    revision, the lifetime, the action) and the host binding;
//! 3. qdrald sends the action with the broker's record id and the host
//!    binding, and the host re-derives its binding before acting.
//!
//! The broker is the single source of approvals: a denied, unavailable or
//! mismatched approval never reaches the host.
//!
//! Slice 1b has no envelope dispatch into this module yet: the native tests
//! drive it directly. Exposure through the `browser_structured` profile and
//! the live-backed page store arrive together in slice 3.
#![cfg_attr(not(test), allow(dead_code))]

use qdral_approval::{now_ms, ApprovalBroker, ApprovalPrompt, ConsumeExpectation};
use qdral_browser_host::worker_host::{BindAction, WireNode, WorkerHostReply, WorkerHostRequest};
use qdral_browser_host::{Supervisor, WorkerHostLaunch};
use qdral_contracts::FailureCode;
use qdral_policy::{Workspace, POLICY_REVISION};
use qdral_provider_fs::ProviderError;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Domain separation for the combined approval digest.
const LIVE_APPROVAL_DOMAIN: &str = "deskal-sg108-live-approval/1";

/// The host lifetime's numeric policy revision. qdrald's own revision (a
/// string) is bound into qdrald's half of every digest.
const LIFETIME_POLICY_REVISION: u64 = 1;

/// A committed download, as the host reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDownload {
    pub relative: String,
    pub sha256: String,
    pub bytes: u64,
    pub source_url: String,
}

/// One supervised worker-mode host and at most one lifetime.
pub struct LiveBrowser {
    supervisor: Supervisor,
    lifetime: Option<String>,
}

/// The failure code for a host refusal (design open question 4), from the
/// closed code only: no host text crosses.
pub fn failure_for(code: &str) -> FailureCode {
    match code {
        "binding_mismatch" | "no_lifetime" | "no_observation" => FailureCode::TargetStale,
        "approval_replay" => FailureCode::ApprovalDenied,
        "invalid" => FailureCode::InvalidRequest,
        "download" => FailureCode::PostconditionFailed,
        "unavailable:timedout" => FailureCode::ProcessTimeout,
        code if code.starts_with("unavailable:") => FailureCode::ProviderUnavailable,
        _ => FailureCode::InternalError,
    }
}

fn host_error(error: qdral_browser_host::HostError) -> ProviderError {
    let code = qdral_browser_host::worker_host::error_code(&error);
    ProviderError::new(failure_for(&code), format!("browser host: {code}"))
}

fn refused(code: &str) -> ProviderError {
    ProviderError::new(failure_for(code), format!("browser host refused: {code}"))
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The combined approval digest: qdrald's material and the host binding.
pub(crate) fn approval_digest(
    workspace_id: &str,
    lifetime: &str,
    action: &BindAction,
    host_binding: &str,
) -> String {
    let action = serde_json::to_string(action).expect("a bind action always serializes");
    sha256_hex(&format!(
        "{LIVE_APPROVAL_DOMAIN}\n{workspace_id}\n{POLICY_REVISION}\n{lifetime}\n{action}\n{host_binding}"
    ))
}

impl LiveBrowser {
    /// Supervise one worker-mode host.
    pub fn start(host_binary: &Path, launch: &WorkerHostLaunch) -> Result<Self, ProviderError> {
        let mut supervisor = Supervisor::new();
        supervisor
            .launch_worker_host(host_binary, launch)
            .map_err(host_error)?;
        Ok(Self {
            supervisor,
            lifetime: None,
        })
    }

    fn request(&mut self, request: &WorkerHostRequest) -> Result<WorkerHostReply, ProviderError> {
        let host = self.supervisor.worker_host().ok_or_else(|| {
            ProviderError::new(FailureCode::ProviderUnavailable, "no live browser host")
        })?;
        match host.request(request).map_err(host_error)? {
            WorkerHostReply::Error { code } => Err(refused(&code)),
            reply => Ok(reply),
        }
    }

    /// Open a lifetime for the fixture destination (test builds only).
    pub fn open_fixture(
        &mut self,
        workspace: &Workspace,
        host: &str,
        fixture_spki: &str,
    ) -> Result<String, ProviderError> {
        match self.request(&WorkerHostRequest::Open {
            workspace: workspace.id.clone(),
            policy_revision: LIFETIME_POLICY_REVISION,
            host: host.into(),
            fixture_spki: fixture_spki.into(),
        })? {
            WorkerHostReply::Opened { lifetime, .. } => {
                self.lifetime = Some(lifetime.clone());
                Ok(lifetime)
            }
            _ => Err(ProviderError::new(
                FailureCode::InternalError,
                "unexpected browser host reply",
            )),
        }
    }

    /// The two-part approval for `action`: the host's binding, then the
    /// broker's approval over qdrald's material and that binding. Returns
    /// the consumed record id and the host binding to present.
    fn approve(
        &mut self,
        workspace: &Workspace,
        approval: &impl ApprovalBroker,
        action: BindAction,
        title: &str,
    ) -> Result<(String, String, BindAction), ProviderError> {
        // What the prompt shows: where the action goes, and for a download
        // also where the file lands (all of it bound into the digest).
        let lifetime = self.lifetime.clone().ok_or_else(|| {
            ProviderError::new(FailureCode::TargetStale, "no live browser lifetime")
        })?;
        let (binding, target) = match self.request(&WorkerHostRequest::Bind {
            action: action.clone(),
        })? {
            WorkerHostReply::Bound { binding, target } => (binding, target),
            _ => {
                return Err(ProviderError::new(
                    FailureCode::InternalError,
                    "unexpected browser host reply",
                ))
            }
        };
        let digest = approval_digest(&workspace.id, &lifetime, &action, &binding);
        let summary = match &action {
            BindAction::Download {
                folder, relative, ..
            } => {
                format!("lifetime={lifetime} source={target} destination={folder} file={relative}")
            }
            _ => format!("lifetime={lifetime} target={target}"),
        };
        let prompt = ApprovalPrompt::new(
            workspace.id.clone(),
            POLICY_REVISION,
            title,
            target.clone(),
            summary,
            digest.clone(),
        );
        let token = approval
            .request_token(&prompt)
            .map_err(|error| ProviderError::new(error.code, error.message))?;
        approval
            .consume(
                &token,
                &ConsumeExpectation::new(digest, workspace.id.clone(), POLICY_REVISION),
                now_ms(),
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?;
        Ok((token.record_id, binding, action))
    }

    /// An approved navigation of the lifetime's page.
    pub fn navigate(
        &mut self,
        workspace: &Workspace,
        approval: &impl ApprovalBroker,
        url: &str,
    ) -> Result<(String, u16), ProviderError> {
        let (record_id, binding, _) = self.approve(
            workspace,
            approval,
            BindAction::Navigate { url: url.into() },
            "navigate isolated browser page",
        )?;
        match self.request(&WorkerHostRequest::Navigate {
            url: url.into(),
            record_id,
            binding,
        })? {
            WorkerHostReply::Navigated { url, status } => Ok((url, status)),
            _ => Err(ProviderError::new(
                FailureCode::InternalError,
                "unexpected browser host reply",
            )),
        }
    }

    /// A bounded observation: identities and digests only.
    pub fn observe(&mut self) -> Result<Vec<WireNode>, ProviderError> {
        match self.request(&WorkerHostRequest::Observe {})? {
            WorkerHostReply::Observed { nodes, .. } => Ok(nodes),
            _ => Err(ProviderError::new(
                FailureCode::InternalError,
                "unexpected browser host reply",
            )),
        }
    }

    /// The host's typed selection of the latest dated link.
    pub fn select_latest_dated_link(&mut self) -> Result<String, ProviderError> {
        match self.request(&WorkerHostRequest::SelectLatestDatedLink {})? {
            WorkerHostReply::Selected { node_id } => Ok(node_id),
            _ => Err(ProviderError::new(
                FailureCode::InternalError,
                "unexpected browser host reply",
            )),
        }
    }

    /// An approved, create-only download of an observed link into
    /// `folder` (the approved workspace destination) as `relative`.
    pub fn download(
        &mut self,
        workspace: &Workspace,
        approval: &impl ApprovalBroker,
        node_id: &str,
        folder: &Path,
        relative: &str,
    ) -> Result<LiveDownload, ProviderError> {
        let folder = folder.to_string_lossy().into_owned();
        let (record_id, binding, _) = self.approve(
            workspace,
            approval,
            BindAction::Download {
                node_id: node_id.into(),
                role: "link".into(),
                folder: folder.clone(),
                relative: relative.into(),
            },
            "download from isolated browser page",
        )?;
        match self.request(&WorkerHostRequest::Download {
            node_id: node_id.into(),
            role: "link".into(),
            folder,
            relative: relative.into(),
            record_id,
            binding,
        })? {
            WorkerHostReply::Downloaded {
                relative,
                sha256,
                bytes,
                source_url,
            } => Ok(LiveDownload {
                relative,
                sha256,
                bytes,
                source_url,
            }),
            _ => Err(ProviderError::new(
                FailureCode::InternalError,
                "unexpected browser host reply",
            )),
        }
    }

    /// Whether the host still answers in step.
    pub fn is_usable(&mut self) -> bool {
        self.supervisor
            .worker_host()
            .is_some_and(|host| host.is_usable())
    }

    /// Shut the host family down; true when confirmed empty.
    pub fn shutdown(mut self) -> bool {
        self.supervisor.shutdown().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_refusals_map_onto_closed_failure_codes() {
        for (code, expected) in [
            ("binding_mismatch", FailureCode::TargetStale),
            ("no_lifetime", FailureCode::TargetStale),
            ("approval_replay", FailureCode::ApprovalDenied),
            ("invalid", FailureCode::InvalidRequest),
            ("download", FailureCode::PostconditionFailed),
            ("unavailable:timedout", FailureCode::ProcessTimeout),
            (
                "unavailable:managedpolicy",
                FailureCode::ProviderUnavailable,
            ),
            ("anything else", FailureCode::InternalError),
        ] {
            assert_eq!(failure_for(code), expected, "{code}");
        }
    }

    #[test]
    fn the_approval_digest_covers_both_halves() {
        let action = BindAction::Navigate {
            url: "https://fixture.test/".into(),
        };
        let base = approval_digest("w", "l1", &action, "h1");
        for other in [
            approval_digest("v", "l1", &action, "h1"),
            approval_digest("w", "l2", &action, "h1"),
            approval_digest("w", "l1", &action, "h2"),
            approval_digest(
                "w",
                "l1",
                &BindAction::Navigate {
                    url: "https://fixture.test/x".into(),
                },
                "h1",
            ),
        ] {
            assert_ne!(base, other);
        }
        assert_eq!(base.len(), 64);
        // Fixed vector, computed independently (Python sha256) over the
        // documented framing, with the live policy revision sg-000040-v1.
        assert_eq!(
            base,
            "8a8dd6cfaae484615bac5f162609033ebae154c15484ad9eda9b38a67a0ba72a"
        );
    }
}

/// Native tests: the real `qdral-browser-host --worker` binary (built with
/// the feature; its path from the test harness), the loopback fixture, and
/// the test approval broker, which mints and consumes like the ledger
/// (exact digest, scope and expiry). Run by `npm run browser-launcher:native`.
#[cfg(test)]
mod native {
    use super::*;
    use qdral_approval::test_support::FixedApprovalBroker;
    use qdral_approval::{ApprovalDecision, ApprovalError, ApprovedToken};
    use std::io::BufRead;
    use std::path::PathBuf;
    use std::time::Duration;

    fn env_path(name: &str) -> PathBuf {
        let value = std::env::var_os(name);
        assert!(value.is_some(), "{name} is not set");
        PathBuf::from(value.unwrap_or_default())
    }

    struct Fixture {
        child: std::process::Child,
        spki: String,
    }

    impl Fixture {
        fn start() -> Self {
            let node = env_path("QDRAL_SG108_NODE");
            let main = env_path("QDRAL_SG108_WORKER_MAIN");
            let app = main.parent().and_then(Path::parent).expect("worker app");
            let mut child = std::process::Command::new(node)
                .arg(app.join("scripts").join("https-fixture.mjs"))
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .expect("start the HTTPS fixture");
            let stdout = child.stdout.take().expect("fixture stdout");
            let (ready, spki) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(stdout)
                    .lines()
                    .map_while(Result::ok)
                {
                    if let Some(value) = line.strip_prefix("READY ") {
                        let _ = ready.send(value.to_owned());
                    }
                }
            });
            let spki = spki
                .recv_timeout(Duration::from_secs(30))
                .expect("fixture ready");
            Self { child, spki }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    struct Scene {
        root: PathBuf,
        approved: PathBuf,
        workspace: Workspace,
    }

    impl Scene {
        fn new(label: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let root = std::env::temp_dir().join(format!(
                "sg108-qdrald-live-{label}-{}-{nanos}",
                std::process::id()
            ));
            let approved = root.join("approved");
            for dir in ["profiles", "staging", "state", "approved"] {
                std::fs::create_dir_all(root.join(dir)).expect("dirs");
            }
            Self {
                workspace: Workspace {
                    id: "ws-sg108-live".into(),
                    root: approved.clone(),
                },
                approved,
                root,
            }
        }

        fn browser(&self) -> LiveBrowser {
            LiveBrowser::start(
                &env_path("QDRAL_SG108_HOST_BINARY"),
                &WorkerHostLaunch {
                    node: env_path("QDRAL_SG108_NODE"),
                    worker_main: env_path("QDRAL_SG108_WORKER_MAIN"),
                    profile_root: self.root.join("profiles"),
                    temp_dir: self.root.join("staging"),
                    state_root: self.root.join("state"),
                },
            )
            .expect("supervise the worker-mode host")
        }
    }

    impl Drop for Scene {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// Open the fixture lifetime; None when the host's real managed-policy
    /// precondition refused it (CI's runner has such a policy), which is
    /// then required to be exactly that refusal.
    fn open(browser: &mut LiveBrowser, scene: &Scene, fixture: &Fixture) -> Option<String> {
        let opened = browser.open_fixture(&scene.workspace, "fixture.test", &fixture.spki);
        if let Err(error) = &opened {
            if error.message.contains("unavailable:managedpolicy") {
                assert_eq!(error.code, FailureCode::ProviderUnavailable);
                println!("SG108-EVIDENCE qdrald live: managed policy present; open refused typed");
                return None;
            }
        }
        Some(opened.expect("open refused"))
    }

    fn report(path: &str) -> Vec<u8> {
        format!(
            "%PDF-1.4\n% {path}\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF\n"
        )
        .into_bytes()
    }

    fn files(dir: &Path) -> usize {
        std::fs::read_dir(dir).map(|read| read.count()).unwrap_or(0)
    }

    #[test]
    #[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
    fn the_journey_runs_under_the_broker_and_commits_the_verified_report() {
        let fixture = Fixture::start();
        let scene = Scene::new("journey");
        let mut browser = scene.browser();
        if open(&mut browser, &scene, &fixture).is_none() {
            assert!(browser.shutdown());
            return;
        }
        let approve = FixedApprovalBroker(ApprovalDecision::Approved);
        let (url, status) = browser
            .navigate(&scene.workspace, &approve, "https://fixture.test/reports")
            .expect("approved navigation");
        assert_eq!(
            (url.as_str(), status),
            ("https://fixture.test/reports", 200)
        );
        assert!(!browser.observe().expect("observe").is_empty());
        let latest = browser.select_latest_dated_link().expect("latest report");
        let committed = browser
            .download(
                &scene.workspace,
                &approve,
                &latest,
                &scene.approved,
                "latest.pdf",
            )
            .expect("approved download");
        // The independent checker.
        let bytes = std::fs::read(scene.approved.join("latest.pdf")).expect("committed file");
        let expected = report("/reports/2026-09-30.pdf");
        assert_eq!(bytes, expected);
        assert_eq!(committed.sha256, sha256_bytes(&expected));
        assert_eq!(committed.bytes, expected.len() as u64);
        assert_eq!(
            committed.source_url,
            "https://fixture.test/reports/2026-09-30.pdf"
        );
        println!(
            "SG108-EVIDENCE qdrald live journey: broker-approved navigation and download, sha256 {}",
            committed.sha256
        );
        assert!(browser.shutdown(), "the host family stayed");
    }

    /// Approves, but presents a token whose digest is not the prompt's: the
    /// broker refuses the consume, so nothing may reach the host.
    struct TamperingBroker(FixedApprovalBroker);

    impl ApprovalBroker for TamperingBroker {
        fn request_token(&self, prompt: &ApprovalPrompt) -> Result<ApprovedToken, ApprovalError> {
            let mut token = self.0.request_token(prompt)?;
            token.digest = "0".repeat(64);
            Ok(token)
        }

        fn consume(
            &self,
            token: &ApprovedToken,
            expected: &ConsumeExpectation,
            now_ms: u64,
        ) -> Result<(), ApprovalError> {
            self.0.consume(token, expected, now_ms)
        }
    }

    #[test]
    #[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
    fn a_denied_or_mismatched_approval_never_reaches_the_host() {
        let fixture = Fixture::start();
        let scene = Scene::new("denied");
        let mut browser = scene.browser();
        if open(&mut browser, &scene, &fixture).is_none() {
            assert!(browser.shutdown());
            return;
        }
        let approve = FixedApprovalBroker(ApprovalDecision::Approved);
        let deny = FixedApprovalBroker(ApprovalDecision::Denied);
        let tamper = TamperingBroker(FixedApprovalBroker(ApprovalDecision::Approved));
        // A denied navigation: refused by the broker, the page stays blank
        // (selection has no observation to work from).
        let denied = browser
            .navigate(&scene.workspace, &deny, "https://fixture.test/reports")
            .expect_err("denied navigation");
        assert_eq!(denied.code, FailureCode::ApprovalDenied);
        browser
            .navigate(&scene.workspace, &approve, "https://fixture.test/reports")
            .expect("approved navigation");
        browser.observe().expect("observe");
        let latest = browser.select_latest_dated_link().expect("latest report");
        // A denied download, and one whose consume fails: nothing written.
        let denied = browser
            .download(&scene.workspace, &deny, &latest, &scene.approved, "a.pdf")
            .expect_err("denied download");
        assert_eq!(denied.code, FailureCode::ApprovalDenied);
        let tampered = browser
            .download(&scene.workspace, &tamper, &latest, &scene.approved, "b.pdf")
            .expect_err("mismatched approval");
        // The broker refused the consume (not the host): its own code and
        // message.
        assert_eq!(tampered.code, FailureCode::TargetStale, "{tampered:?}");
        assert!(
            tampered.message.contains("digest changed after approval"),
            "{tampered:?}"
        );
        assert_eq!(files(&scene.approved), 0, "a refused approval wrote a file");
        // The lifetime is still live and in step: an approved download runs.
        assert!(browser.is_usable());
        browser
            .download(
                &scene.workspace,
                &approve,
                &latest,
                &scene.approved,
                "c.pdf",
            )
            .expect("approved download after refusals");
        assert_eq!(files(&scene.approved), 1);
        println!("SG108-EVIDENCE qdrald live denial: denied and mismatched approvals never reached the host");
        assert!(browser.shutdown(), "the host family stayed");
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
