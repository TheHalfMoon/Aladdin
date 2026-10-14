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
use std::path::{Path, PathBuf};

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

/// What became of a consumed approval (slice 2: the "consumed, not
/// dispatched" record of decision 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// The host carried the action out.
    Executed,
    /// The host refused the action after the approval was spent (closed code).
    Refused(String),
    /// The reply never came (timeout, a dead or broken host): the approval
    /// was spent and the outcome is unknown until reconciled.
    Lost,
}

/// One consumed approval and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalOutcome {
    pub record_id: String,
    pub outcome: DispatchOutcome,
}

/// A download sent to the host whose reply has not arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadIntent {
    pub record_id: String,
    pub folder: PathBuf,
    pub relative: String,
}

/// What a lost download left at its destination. qdrald cannot tell the
/// host's committed file from one a same-user process placed at that name,
/// so a present file is reported, never adopted as a committed download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciled {
    PresentUnverified { sha256: String, bytes: u64 },
    Absent,
}

/// What is at `relative` inside `folder` now. `relative` must pass the
/// host's own destination rules (no root, traversal, streams, trailing
/// dots or spaces, reserved names); no segment below `folder` may be a
/// reparse point, and anything but a regular file at the name is refused
/// rather than reported. The file is hashed through the handle that was
/// checked, and never beyond the transfer limit.
pub fn reconcile_destination(folder: &Path, relative: &str) -> Result<Reconciled, ProviderError> {
    let relative = qdral_browser_host::validate_relative_destination(relative).map_err(|_| {
        ProviderError::new(
            FailureCode::InvalidRequest,
            "the destination name is not a plain relative path",
        )
    })?;
    let segments: Vec<&str> = relative.split(['/', '\\']).collect();
    let mut path = folder.to_path_buf();
    for (index, segment) in segments.iter().enumerate() {
        path.push(segment);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Reconciled::Absent)
            }
            Err(_) => return Err(uninspectable()),
        };
        let last = index + 1 == segments.len();
        if is_reparse_point(&metadata)
            || (last && !metadata.is_file())
            || (!last && !metadata.is_dir())
        {
            return Err(not_a_regular_file());
        }
    }
    let file = open_no_reparse(&path).map_err(|_| uninspectable())?;
    // The handle's own metadata: what was opened is still a plain file.
    let metadata = file.metadata().map_err(|_| uninspectable())?;
    if is_reparse_point(&metadata) || !metadata.is_file() {
        return Err(not_a_regular_file());
    }
    let limit = qdral_browser_host::MAX_TRANSFER_BYTES;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut reader = std::io::Read::take(file, limit + 1);
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut reader, &mut buffer).map_err(|_| {
            ProviderError::new(
                FailureCode::ProviderUnavailable,
                "the destination could not be read",
            )
        })?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        hasher.update(&buffer[..read]);
    }
    if bytes > limit {
        return Err(ProviderError::new(
            FailureCode::PostconditionFailed,
            "the destination is larger than any transfer",
        ));
    }
    Ok(Reconciled::PresentUnverified {
        sha256: hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        bytes,
    })
}

fn uninspectable() -> ProviderError {
    ProviderError::new(
        FailureCode::ProviderUnavailable,
        "the destination could not be inspected",
    )
}

fn not_a_regular_file() -> ProviderError {
    ProviderError::new(
        FailureCode::PostconditionFailed,
        "the destination is not a regular file",
    )
}

fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// Open for reading without following a reparse point at the final name.
fn open_no_reparse(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

/// One supervised worker-mode host and at most one lifetime.
pub struct LiveBrowser {
    supervisor: Supervisor,
    lifetime: Option<String>,
    outcomes: Vec<ApprovalOutcome>,
    pending_download: Option<DownloadIntent>,
    /// None while the host is (or may be) running; once ended, whether the
    /// whole family was confirmed gone. Set once, never revised: a later
    /// shutdown finds no host and would report a vacuous success.
    host_ended: Option<bool>,
    /// Test builds only: a shorter reply budget for dispatched frames.
    #[cfg(test)]
    reply_timeout: Option<std::time::Duration>,
    /// Test builds only: runs once, between a consume and its dispatch,
    /// with the host's pid (to make the host die there).
    #[cfg(test)]
    after_consume: Option<Box<dyn FnMut(u32)>>,
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
    // A failed pipe or platform call is the provider being unavailable (a
    // dead or broken host), not an internal fault.
    let failure = match error {
        qdral_browser_host::HostError::Io(_) | qdral_browser_host::HostError::Platform(_) => {
            FailureCode::ProviderUnavailable
        }
        _ => failure_for(&code),
    };
    ProviderError::new(failure, format!("browser host: {code}"))
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
            outcomes: Vec::new(),
            pending_download: None,
            host_ended: None,
            #[cfg(test)]
            reply_timeout: None,
            #[cfg(test)]
            after_consume: None,
        })
    }

    fn request(&mut self, request: &WorkerHostRequest) -> Result<WorkerHostReply, ProviderError> {
        let host = self.supervisor.worker_host().ok_or_else(|| {
            ProviderError::new(FailureCode::ProviderUnavailable, "no live browser host")
        })?;
        match host.request(request) {
            Ok(WorkerHostReply::Error { code }) => Err(refused(&code)),
            Ok(reply) => Ok(reply),
            Err(error) => {
                // A lost or broken channel: the lifetime is over for qdrald
                // (decision 3: a dead lifetime makes its page stale).
                self.end_host();
                Err(host_error(error))
            }
        }
    }

    /// End the lifetime and the whole host family, once. A host whose
    /// reply is lost may still be acting (suspended, or slow): only a
    /// confirmed-gone family makes the destination safe to reconcile.
    fn end_host(&mut self) {
        self.lifetime = None;
        if self.host_ended.is_none() {
            self.host_ended = Some(self.supervisor.shutdown_worker_host().unwrap_or(false));
        }
    }

    /// Send a consequential frame whose approval was just consumed, and
    /// record what became of that approval.
    fn dispatch(
        &mut self,
        record_id: &str,
        request: &WorkerHostRequest,
    ) -> Result<WorkerHostReply, ProviderError> {
        #[cfg(test)]
        if let Some(mut hook) = self.after_consume.take() {
            if let Some(pid) = self.supervisor.worker_host().map(|host| host.pid()) {
                hook(pid);
            }
        }
        #[cfg(test)]
        let reply_timeout = self.reply_timeout;
        let reply = match self.supervisor.worker_host() {
            None => Err(ProviderError::new(
                FailureCode::ProviderUnavailable,
                "no live browser host",
            )),
            #[cfg(test)]
            Some(host) if reply_timeout.is_some() => host
                .request_within(request, reply_timeout.unwrap_or_default())
                .map_err(host_error),
            Some(host) => host.request(request).map_err(host_error),
        };
        let (outcome, result) = match reply {
            Ok(WorkerHostReply::Error { code }) => {
                let error = refused(&code);
                (DispatchOutcome::Refused(code), Err(error))
            }
            Ok(reply) => (DispatchOutcome::Executed, Ok(reply)),
            // Every other path leaves the consumed approval's outcome
            // unknown: record it lost, and end the host before anything
            // reads the destination.
            Err(error) => {
                self.end_host();
                (DispatchOutcome::Lost, Err(error))
            }
        };
        self.outcomes.push(ApprovalOutcome {
            record_id: record_id.to_owned(),
            outcome,
        });
        result
    }

    /// Every consumed approval of this browser and its outcome, in order.
    pub fn outcomes(&self) -> &[ApprovalOutcome] {
        &self.outcomes
    }

    /// Whether a lifetime is live (false after open, before open, or once a
    /// lost reply ended it).
    pub fn has_lifetime(&self) -> bool {
        self.lifetime.is_some()
    }

    /// Whether the host family was ended and confirmed gone (None while
    /// it may still be running).
    pub fn host_ended(&self) -> Option<bool> {
        self.host_ended
    }

    /// Reconcile the download whose reply was lost, if any: report what is
    /// at its destination now. Never re-downloads. Refused, keeping the
    /// intent, unless the host family is confirmed gone: a host still
    /// running could commit after the report.
    pub fn reconcile(&mut self) -> Option<Result<(DownloadIntent, Reconciled), ProviderError>> {
        self.pending_download.as_ref()?;
        if self.host_ended != Some(true) {
            return Some(Err(ProviderError::new(
                FailureCode::ProviderUnavailable,
                "the browser host is not confirmed gone; the destination cannot be reconciled",
            )));
        }
        let intent = self.pending_download.take()?;
        Some(reconcile_destination(&intent.folder, &intent.relative).map(|found| (intent, found)))
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
        match self.dispatch(
            &record_id,
            &WorkerHostRequest::Navigate {
                url: url.into(),
                record_id: record_id.clone(),
                binding,
            },
        )? {
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
        // Recorded before the frame goes out, so a lost reply can be
        // reconciled against the destination.
        self.pending_download = Some(DownloadIntent {
            record_id: record_id.clone(),
            folder: PathBuf::from(&folder),
            relative: relative.into(),
        });
        let reply = self.dispatch(
            &record_id,
            &WorkerHostRequest::Download {
                node_id: node_id.into(),
                role: "link".into(),
                folder,
                relative: relative.into(),
                record_id: record_id.clone(),
                binding,
            },
        );
        // A reply, refusal or success, settles it; only a lost one stays.
        if !matches!(self.outcomes.last(), Some(o) if o.outcome == DispatchOutcome::Lost) {
            self.pending_download = None;
        }
        match reply? {
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
    fn reconciliation_reports_what_is_at_the_destination_and_nothing_else() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let folder =
            std::env::temp_dir().join(format!("qdral-reconcile-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(folder.join("sub")).unwrap();
        assert_eq!(
            reconcile_destination(&folder, "a.pdf").unwrap(),
            Reconciled::Absent
        );
        std::fs::write(folder.join("a.pdf"), b"%PDF-1.4").unwrap();
        assert_eq!(
            reconcile_destination(&folder, "a.pdf").unwrap(),
            Reconciled::PresentUnverified {
                sha256: sha256_hex("%PDF-1.4"),
                bytes: 8
            }
        );
        for bad in [
            "",
            "../a.pdf",
            "sub/../../a.pdf",
            "C:\\a.pdf",
            "/a.pdf",
            "a.pdf:stream",
            "a.pdf.",
            "a.pdf ",
            "NUL",
            "nul.pdf",
            "sub//a.pdf",
        ] {
            assert_eq!(
                reconcile_destination(&folder, bad).unwrap_err().code,
                FailureCode::InvalidRequest,
                "{bad}"
            );
        }
        // A directory at the name is refused, not reported.
        assert_eq!(
            reconcile_destination(&folder, "sub").unwrap_err().code,
            FailureCode::PostconditionFailed
        );
        // A junction on the way is not followed, even to a real file.
        std::fs::write(folder.join("sub").join("b.pdf"), b"%PDF-1.4").unwrap();
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(folder.join("via"))
            .arg(folder.join("sub"))
            .output()
            .expect("mklink");
        assert!(made.status.success(), "{made:?}");
        assert!(folder.join("via").join("b.pdf").is_file());
        assert_eq!(
            reconcile_destination(&folder, "via/b.pdf")
                .unwrap_err()
                .code,
            FailureCode::PostconditionFailed
        );
        assert_eq!(
            reconcile_destination(&folder, "via").unwrap_err().code,
            FailureCode::PostconditionFailed
        );
        // Nothing larger than a transfer is hashed or reported.
        let limit = qdral_browser_host::MAX_TRANSFER_BYTES;
        std::fs::write(folder.join("big.pdf"), vec![b' '; limit as usize + 1]).unwrap();
        assert_eq!(
            reconcile_destination(&folder, "big.pdf").unwrap_err().code,
            FailureCode::PostconditionFailed
        );
        std::fs::write(folder.join("big.pdf"), vec![b' '; limit as usize]).unwrap();
        assert!(matches!(
            reconcile_destination(&folder, "big.pdf").unwrap(),
            Reconciled::PresentUnverified { bytes, .. } if bytes == limit
        ));
        std::fs::remove_dir(folder.join("via")).unwrap();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    fn hostless(pending: bool, host_ended: Option<bool>, folder: &Path) -> LiveBrowser {
        LiveBrowser {
            supervisor: Supervisor::new(),
            lifetime: None,
            outcomes: Vec::new(),
            pending_download: pending.then(|| DownloadIntent {
                record_id: "r1".into(),
                folder: folder.to_path_buf(),
                relative: "a.pdf".into(),
            }),
            host_ended,
            reply_timeout: None,
            after_consume: None,
        }
    }

    #[test]
    fn reconciliation_waits_for_a_confirmed_end_of_the_host() {
        let folder =
            std::env::temp_dir().join(format!("qdral-reconcile-gate-{}", std::process::id()));
        assert!(hostless(false, Some(true), &folder).reconcile().is_none());
        // Running, or ended without confirmation: refused, and the intent
        // is kept (never silently dropped).
        for ended in [None, Some(false)] {
            let mut browser = hostless(true, ended, &folder);
            let refused = browser.reconcile().expect("pending").unwrap_err();
            assert_eq!(refused.code, FailureCode::ProviderUnavailable);
            assert!(browser.pending_download.is_some(), "{ended:?}");
        }
        let mut browser = hostless(true, Some(true), &folder);
        let (intent, found) = browser.reconcile().expect("pending").expect("reconciled");
        assert_eq!(intent.record_id, "r1");
        assert_eq!(found, Reconciled::Absent);
        assert!(browser.reconcile().is_none());
    }

    #[test]
    fn a_dispatch_without_a_host_is_recorded_lost() {
        let folder = std::env::temp_dir();
        let mut browser = hostless(false, None, &folder);
        let error = browser
            .dispatch("r9", &WorkerHostRequest::Observe {})
            .unwrap_err();
        assert_eq!(error.code, FailureCode::ProviderUnavailable);
        assert_eq!(
            browser.outcomes(),
            &[ApprovalOutcome {
                record_id: "r9".into(),
                outcome: DispatchOutcome::Lost
            }]
        );
        // No host existed, so its end is (trivially) confirmed.
        assert_eq!(browser.host_ended(), Some(true));
    }

    #[test]
    fn broken_pipes_and_platform_faults_are_the_provider_unavailable() {
        use qdral_browser_host::HostError;
        let io = HostError::Io(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        assert_eq!(host_error(io).code, FailureCode::ProviderUnavailable);
        let platform = HostError::Platform("job".into());
        assert_eq!(host_error(platform).code, FailureCode::ProviderUnavailable);
        let invalid = HostError::Invalid("x".into());
        assert_eq!(host_error(invalid).code, FailureCode::InvalidRequest);
    }

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
    use qdral_approval::{
        ApprovalDecision, ApprovalError, ApprovedToken, LocalApprovalBroker, NoPresenceVerifier,
        RecordedDecision,
    };
    use std::io::BufRead;
    use std::path::PathBuf;
    use std::sync::Arc;
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
        // The real broker over its own ledger; only the SOFT consent (the
        // system dialog) is scripted, as approval.
        let ledger = scene.root.join("approvals.json");
        let approve = LocalApprovalBroker::with_path_and_soft_decider(
            &ledger,
            Arc::new(NoPresenceVerifier),
            Arc::new(|_| ApprovalDecision::Approved),
        );
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
        // The ledger recorded exactly these two approvals, each consumed,
        // and both were carried out.
        let records = approve.history(10);
        assert_eq!(records.len(), 2, "{records:?}");
        assert!(records
            .iter()
            .all(|record| record.decision == RecordedDecision::Approved && record.consumed));
        let ids: Vec<&str> = browser
            .outcomes()
            .iter()
            .map(|o| o.record_id.as_str())
            .collect();
        assert!(records
            .iter()
            .all(|record| ids.contains(&record.id.as_str())));
        assert!(browser
            .outcomes()
            .iter()
            .all(|o| o.outcome == DispatchOutcome::Executed));
        // Reconciling the committed name reports exactly the committed file.
        assert_eq!(
            reconcile_destination(&scene.approved, "latest.pdf").expect("reconcile"),
            Reconciled::PresentUnverified {
                sha256: committed.sha256.clone(),
                bytes: committed.bytes
            }
        );
        assert!(
            browser.reconcile().is_none(),
            "no download was left pending"
        );
        println!(
            "SG108-EVIDENCE qdrald live journey: real-ledger approved navigation and download, sha256 {}",
            committed.sha256
        );
        assert!(browser.shutdown(), "the host family stayed");
    }

    #[test]
    #[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
    fn a_host_lost_after_the_consume_is_recorded_reconciled_and_ends_the_lifetime() {
        let fixture = Fixture::start();
        let scene = Scene::new("lost");
        let mut browser = scene.browser();
        if open(&mut browser, &scene, &fixture).is_none() {
            assert!(browser.shutdown());
            return;
        }
        let ledger = scene.root.join("approvals.json");
        let approve = LocalApprovalBroker::with_path_and_soft_decider(
            &ledger,
            Arc::new(NoPresenceVerifier),
            Arc::new(|_| ApprovalDecision::Approved),
        );
        browser
            .navigate(&scene.workspace, &approve, "https://fixture.test/reports")
            .expect("approved navigation");
        browser.observe().expect("observe");
        let latest = browser.select_latest_dated_link().expect("latest report");
        // The host dies after the broker consumed the download's approval
        // and before the frame reaches it.
        browser.after_consume = Some(Box::new(|pid: u32| {
            use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
            use windows_sys::Win32::System::Threading::{
                OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
            };
            // SAFETY: a plain Win32 open of this test's host pid; the handle
            // pins the process object so the wait below observes this host.
            let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
            assert_ne!(process, 0, "open the host process");
            // The tree kill may race children that exit on their own (its
            // exit status then reports them); what matters is the host died.
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output()
                .expect("taskkill");
            // SAFETY: the handle opened above, still open.
            let waited = unsafe { WaitForSingleObject(process, 10_000) };
            // SAFETY: closing the handle opened above, once.
            unsafe { CloseHandle(process) };
            assert_eq!(waited, WAIT_OBJECT_0, "the host did not die");
        }));
        let lost = browser
            .download(
                &scene.workspace,
                &approve,
                &latest,
                &scene.approved,
                "lost.pdf",
            )
            .expect_err("the host was gone");
        assert!(
            matches!(
                lost.code,
                FailureCode::ProviderUnavailable | FailureCode::ProcessTimeout
            ),
            "{lost:?}"
        );
        // The approval was spent, and the outcome says it was lost.
        let last = browser.outcomes().last().expect("an outcome").clone();
        assert_eq!(last.outcome, DispatchOutcome::Lost);
        assert!(approve
            .history(10)
            .iter()
            .any(|record| record.id == last.record_id && record.consumed));
        // The lifetime is over: the next action is stale, before any prompt.
        assert!(!browser.has_lifetime());
        let before = approve.history(10).len();
        let stale = browser
            .navigate(&scene.workspace, &approve, "https://fixture.test/reports")
            .expect_err("stale lifetime");
        assert_eq!(stale.code, FailureCode::TargetStale);
        assert_eq!(approve.history(10).len(), before, "a stale action prompted");
        // The family was ended and confirmed gone before anything reads
        // the destination.
        assert_eq!(browser.host_ended(), Some(true));
        // Reconciliation: the host died before acting, so nothing is there.
        let (intent, found) = browser
            .reconcile()
            .expect("a pending download")
            .expect("reconciled");
        assert_eq!(intent.record_id, last.record_id);
        assert_eq!(found, Reconciled::Absent);
        assert_eq!(files(&scene.approved), 0);
        println!("SG108-EVIDENCE qdrald live lost: consumed approval recorded lost, lifetime stale, destination reconciled absent");
        let _ = browser.shutdown();
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtSuspendProcess(process: windows_sys::Win32::Foundation::HANDLE) -> i32;
    }

    #[test]
    #[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
    fn a_suspended_host_is_ended_before_its_lost_download_is_reconciled() {
        use std::sync::atomic::{AtomicIsize, Ordering};
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_SUSPEND_RESUME, PROCESS_SYNCHRONIZE,
        };
        let fixture = Fixture::start();
        let scene = Scene::new("suspended");
        let mut browser = scene.browser();
        if open(&mut browser, &scene, &fixture).is_none() {
            assert!(browser.shutdown());
            return;
        }
        let approve = LocalApprovalBroker::with_path_and_soft_decider(
            scene.root.join("approvals.json"),
            Arc::new(NoPresenceVerifier),
            Arc::new(|_| ApprovalDecision::Approved),
        );
        browser
            .navigate(&scene.workspace, &approve, "https://fixture.test/reports")
            .expect("approved navigation");
        browser.observe().expect("observe");
        let latest = browser.select_latest_dated_link().expect("latest report");
        // The host is alive but silent (suspended, not dead) when the
        // consumed download goes out: the reply is lost, yet the host could
        // still act once it resumed.
        let handle = Arc::new(AtomicIsize::new(0));
        let held = Arc::clone(&handle);
        browser.reply_timeout = Some(Duration::from_secs(5));
        browser.after_consume = Some(Box::new(move |pid: u32| {
            // SAFETY: plain Win32 calls on a pid this test's host owns; the
            // handle is kept (so the pid cannot be reused under it) and
            // closed below.
            let process =
                unsafe { OpenProcess(PROCESS_SUSPEND_RESUME | PROCESS_SYNCHRONIZE, 0, pid) };
            assert_ne!(process, 0, "open the host process");
            // SAFETY: a valid process handle with PROCESS_SUSPEND_RESUME.
            assert_eq!(unsafe { NtSuspendProcess(process) }, 0, "suspend the host");
            held.store(process, Ordering::SeqCst);
        }));
        let lost = browser
            .download(
                &scene.workspace,
                &approve,
                &latest,
                &scene.approved,
                "lost.pdf",
            )
            .expect_err("the reply never came");
        assert_eq!(lost.code, FailureCode::ProcessTimeout, "{lost:?}");
        let last = browser.outcomes().last().expect("an outcome").clone();
        assert_eq!(last.outcome, DispatchOutcome::Lost);
        // The host was ended and confirmed gone before reconciliation: the
        // suspended process itself has exited.
        assert_eq!(browser.host_ended(), Some(true));
        let process = handle.load(Ordering::SeqCst);
        // SAFETY: the handle opened above, still open.
        assert_eq!(unsafe { WaitForSingleObject(process, 0) }, WAIT_OBJECT_0);
        // SAFETY: closing the handle opened above, once.
        unsafe { CloseHandle(process) };
        let (intent, found) = browser
            .reconcile()
            .expect("a pending download")
            .expect("reconciled");
        assert_eq!(intent.record_id, last.record_id);
        assert_eq!(found, Reconciled::Absent);
        // And it stays absent: nothing is left that could commit later.
        std::thread::sleep(Duration::from_secs(3));
        assert_eq!(files(&scene.approved), 0);
        println!("SG108-EVIDENCE qdrald live suspended: silent host ended and confirmed gone, lost download reconciled absent and stayed absent");
        let _ = browser.shutdown();
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
