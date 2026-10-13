//! SG-000108 product caller, slice 1a: the host binary's worker mode.
//!
//! `qdral-browser-host --worker` never spawns the engine itself: it serves
//! a closed capability vocabulary over one `launch_worker` lifetime at a
//! time, for exactly one admitted destination. Deskal supervision is the
//! only peer (the pipe's other end). Test builds only in this slice: the
//! whole module exists under the `live-browser-test` feature, which admits
//! only the loopback fixture origin, and release qualification asserts the
//! feature is absent (docs/security/SG-000108_PRODUCT_CALLER.md).
//!
//! Approval binding for consequential frames (the `bind` frame and the
//! two-part digest) arrives in slice 1b; navigation here is reachable only
//! in test builds.

use crate::confinement::AdmittedDestination;
use crate::discovery::{discover_engine, SearchConfig};
use crate::error::HostError;
use crate::observation::{ObservationScope, ObservedNode};
use crate::proto::{decode_frame, write_frame};
use crate::worker_launch::{launch_worker, BrowserWorker, WorkerLaunch};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

/// Worker-mode protocol generation, distinct from SG-000074's generation 1
/// (hello/ping/shutdown), so a classic peer and a worker-mode peer refuse
/// each other at hello.
pub const WORKER_HOST_GENERATION: u32 = 2;

/// Closed worker-mode request vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerHostRequest {
    Hello {
        generation: u32,
    },
    /// Start a lifetime for one destination. Test builds admit only the
    /// loopback fixture, trusting exactly its certificate.
    Open {
        workspace: String,
        host: String,
        fixture_spki: String,
    },
    Navigate {
        url: String,
    },
    Observe {},
    /// End the current lifetime (profile removed); the host keeps serving.
    Close {},
    Shutdown {},
}

/// One observed node as it crosses to Deskal: identities and digests only.
/// Raw names and values never leave the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireNode {
    pub node_id: String,
    pub role: String,
    pub name_digest: String,
    pub value_digest: String,
    pub redacted: bool,
    pub children: Vec<WireNode>,
}

impl From<&ObservedNode> for WireNode {
    fn from(node: &ObservedNode) -> Self {
        Self {
            node_id: node.node_id.clone(),
            role: node.role.clone(),
            name_digest: node.name_digest.clone(),
            value_digest: node.value_digest.clone(),
            redacted: node.redacted,
            children: node.children.iter().map(WireNode::from).collect(),
        }
    }
}

/// Closed worker-mode reply vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerHostReply {
    Hello {
        generation: u32,
        host: String,
    },
    /// A lifetime started: its nonce (bound into every later approval
    /// binding) and the address the host pinned.
    Opened {
        lifetime: String,
        pinned: String,
    },
    Navigated {
        url: String,
        status: u16,
    },
    Observed {
        page_id: String,
        origin: String,
        page_generation: u64,
        document_generation: u64,
        nodes: Vec<WireNode>,
        node_count: usize,
        truncated: bool,
    },
    Closed {
        job_empty: bool,
        profile_removed: bool,
    },
    Bye {},
    /// A typed refusal: a bounded, closed code, never page text.
    Error {
        code: String,
    },
}

/// What Deskal supervision passes the host in worker mode. Never caller
/// input: these come from Deskal's own state and, in this slice, from the
/// test harness.
#[derive(Debug, Clone)]
pub struct WorkerHostArgs {
    pub host_id: String,
    pub node: PathBuf,
    pub worker_main: PathBuf,
    pub profile_root: PathBuf,
    pub temp_dir: PathBuf,
}

/// The closed error code for a host error. No message text crosses: a
/// message could carry a URL or page text.
pub fn error_code(error: &HostError) -> String {
    match error {
        HostError::Unavailable(reason) => format!("unavailable:{reason:?}").to_ascii_lowercase(),
        HostError::Invalid(_) => "invalid".into(),
        HostError::Download(_) => "download".into(),
        _ => "failed".into(),
    }
}

struct Lifetime {
    worker: BrowserWorker,
    workspace: String,
    profile_identity: String,
}

/// Serve the worker-mode vocabulary until shutdown, end of input or a
/// protocol violation. At most one lifetime exists at a time; ending the
/// host ends it (the job is kill-on-close either way).
pub fn serve_worker_mode(
    args: &WorkerHostArgs,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<(), String> {
    let mut lifetime: Option<Lifetime> = None;
    let mut said_hello = false;
    let result = loop {
        let request: WorkerHostRequest = match decode_frame(input) {
            Ok(request) => request,
            Err(_) => break Err("protocol violation".to_string()),
        };
        if !said_hello && !matches!(request, WorkerHostRequest::Hello { .. }) {
            break Err("frame before hello".to_string());
        }
        let reply = match request {
            WorkerHostRequest::Hello { generation } => {
                if generation != WORKER_HOST_GENERATION {
                    break Err("protocol generation mismatch".to_string());
                }
                said_hello = true;
                WorkerHostReply::Hello {
                    generation: WORKER_HOST_GENERATION,
                    host: args.host_id.clone(),
                }
            }
            WorkerHostRequest::Open {
                workspace,
                host,
                fixture_spki,
            } => {
                if lifetime.is_some() {
                    error("lifetime_open")
                } else {
                    match open(args, &workspace, &host, &fixture_spki) {
                        Ok((opened, reply)) => {
                            lifetime = Some(opened);
                            reply
                        }
                        Err(err) => WorkerHostReply::Error {
                            code: error_code(&err),
                        },
                    }
                }
            }
            WorkerHostRequest::Navigate { url } => match lifetime.as_mut() {
                None => error("no_lifetime"),
                Some(live) => match live.worker.navigate(&url) {
                    Ok(outcome) => WorkerHostReply::Navigated {
                        url: outcome.url,
                        status: outcome.status,
                    },
                    Err(err) => WorkerHostReply::Error {
                        code: error_code(&err),
                    },
                },
            },
            WorkerHostRequest::Observe {} => match lifetime.as_mut() {
                None => error("no_lifetime"),
                Some(live) => observe(live),
            },
            WorkerHostRequest::Close {} => match lifetime.take() {
                None => error("no_lifetime"),
                Some(live) => {
                    let report = live.worker.shutdown();
                    WorkerHostReply::Closed {
                        job_empty: report.job_empty,
                        profile_removed: report.profile_removed,
                    }
                }
            },
            WorkerHostRequest::Shutdown {} => {
                if let Some(live) = lifetime.take() {
                    let _ = live.worker.shutdown();
                }
                let _ = send(output, &WorkerHostReply::Bye {});
                break Ok(());
            }
        };
        if send(output, &reply).is_err() {
            break Err("reply failed".to_string());
        }
    };
    if let Some(live) = lifetime.take() {
        let _ = live.worker.kill();
    }
    result
}

fn error(code: &str) -> WorkerHostReply {
    WorkerHostReply::Error { code: code.into() }
}

fn send(output: &mut impl Write, value: &WorkerHostReply) -> Result<(), HostError> {
    write_frame(output, value)?;
    output.flush().map_err(HostError::from)
}

/// Start a lifetime: the host's own engine discovery (A10 re-verifies it at
/// launch), its own pid-named ephemeral profile (#298) carrying the A9
/// marker, and the fixture destination with its certificate.
fn open(
    args: &WorkerHostArgs,
    workspace: &str,
    host: &str,
    fixture_spki: &str,
) -> Result<(Lifetime, WorkerHostReply), HostError> {
    crate::observation::validate_identity_field(workspace)?;
    if fixture_spki.len() != 44
        || !fixture_spki
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
    {
        return Err(HostError::Invalid(
            "fixture certificate hash is malformed".into(),
        ));
    }
    // Everything that can be refused without a profile is refused first.
    let pin = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let destination = AdmittedDestination::fixture(host, pin)?;
    let engine = discover_engine(&SearchConfig::production())?;
    let profile_dir = crate::worker_launch::ephemeral_profile_dir(&args.profile_root)?;
    // From here until the worker owns the profile, any failure removes it:
    // the #298 sweep never collects this host's own profiles while it runs.
    let launched = launch_in(args, &profile_dir, engine, destination, fixture_spki);
    let (worker, profile_identity) = match launched {
        Ok(launched) => launched,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&profile_dir);
            return Err(err);
        }
    };
    let nonce = match crate::worker_launch::lifetime_key() {
        Ok(nonce) => nonce,
        Err(err) => {
            let _ = worker.kill();
            return Err(err);
        }
    };
    let lifetime = nonce[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok((
        Lifetime {
            worker,
            workspace: workspace.to_owned(),
            profile_identity,
        },
        WorkerHostReply::Opened {
            lifetime,
            pinned: pin.to_string(),
        },
    ))
}

/// Mark the fresh profile (A9: adopt-back proves the marker round-trips),
/// then launch the worker in it.
fn launch_in(
    args: &WorkerHostArgs,
    profile_dir: &std::path::Path,
    engine: crate::discovery::Engine,
    destination: AdmittedDestination,
    fixture_spki: &str,
) -> Result<(BrowserWorker, String), HostError> {
    let profile = crate::profile::mark_profile(profile_dir, &engine.fingerprint())?;
    let profile = crate::profile::adopt_profile(profile.dir(), &engine.fingerprint())?;
    let spec = WorkerLaunch {
        node: args.node.clone(),
        worker_main: args.worker_main.clone(),
        engine,
        profile_dir: profile.dir().to_path_buf(),
        ephemeral_profile: true,
        temp_dir: args.temp_dir.clone(),
        destinations: vec![destination],
        test_trusted_spki: Some(fixture_spki.to_owned()),
    };
    let worker = launch_worker(&spec, &crate::confinement::RegistryPolicySource)?;
    Ok((worker, profile.identity().to_owned()))
}

/// Observe the committed document; nodes are dropped from the end until the
/// reply fits one frame, and the reply then says it was truncated.
fn observe(live: &mut Lifetime) -> WorkerHostReply {
    let scope = ObservationScope {
        workspace: live.workspace.clone(),
        profile_identity: live.profile_identity.clone(),
    };
    let observed = match live.worker.observe(&scope) {
        Ok(observed) => observed,
        Err(err) => {
            return WorkerHostReply::Error {
                code: error_code(&err),
            }
        }
    };
    let mut nodes: Vec<WireNode> = observed
        .observation
        .nodes
        .iter()
        .map(WireNode::from)
        .collect();
    let mut truncated = observed.observation.truncated;
    loop {
        let reply = WorkerHostReply::Observed {
            page_id: observed.page.page_id.clone(),
            origin: observed.page.origin.clone(),
            page_generation: observed.page.page_generation,
            document_generation: observed.document.document_generation,
            nodes: nodes.clone(),
            node_count: observed.observation.node_count,
            truncated,
        };
        if crate::proto::encode_frame(&reply).is_ok() || nodes.is_empty() {
            return reply;
        }
        nodes.pop();
        truncated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::UnavailableReason;

    #[test]
    fn the_vocabulary_is_closed_and_round_trips() {
        for frame in [
            r#"{"frame":"navigate","url":"https://fixture.test/","extra":1}"#,
            r#"{"frame":"evaluate","script":"1"}"#,
            r#"{"frame":"shutdown","now":true}"#,
        ] {
            assert!(
                serde_json::from_str::<WorkerHostRequest>(frame).is_err(),
                "{frame}"
            );
        }
        let open = WorkerHostRequest::Open {
            workspace: "w".into(),
            host: "fixture.test".into(),
            fixture_spki: "A".repeat(43) + "=",
        };
        let text = serde_json::to_string(&open).unwrap();
        assert_eq!(
            serde_json::from_str::<WorkerHostRequest>(&text).unwrap(),
            open
        );
    }

    #[test]
    fn errors_cross_as_closed_codes_without_text() {
        assert_eq!(
            error_code(&HostError::Invalid(
                "https://secret.example/?q=page text".into()
            )),
            "invalid"
        );
        assert!(error_code(&HostError::Unavailable(
            UnavailableReason::UnsupportedEngine
        ))
        .starts_with("unavailable:"));
    }

    #[test]
    fn frames_before_hello_or_of_another_generation_end_the_host() {
        let args = WorkerHostArgs {
            host_id: "h".into(),
            node: PathBuf::from("node.exe"),
            worker_main: PathBuf::from("main.js"),
            profile_root: std::env::temp_dir(),
            temp_dir: std::env::temp_dir(),
        };
        let frames = |requests: &[WorkerHostRequest]| {
            let mut bytes = Vec::new();
            for request in requests {
                bytes.extend(crate::proto::encode_frame(request).unwrap());
            }
            bytes
        };
        let mut out = Vec::new();
        let early = frames(&[WorkerHostRequest::Observe {}]);
        assert!(serve_worker_mode(&args, &mut early.as_slice(), &mut out).is_err());
        let wrong = frames(&[WorkerHostRequest::Hello { generation: 1 }]);
        assert!(serve_worker_mode(&args, &mut wrong.as_slice(), &mut out).is_err());
        // Before a lifetime: typed refusals, then a clean shutdown.
        let mut out = Vec::new();
        let ok = frames(&[
            WorkerHostRequest::Hello {
                generation: WORKER_HOST_GENERATION,
            },
            WorkerHostRequest::Navigate {
                url: "https://fixture.test/".into(),
            },
            WorkerHostRequest::Shutdown {},
        ]);
        assert!(serve_worker_mode(&args, &mut ok.as_slice(), &mut out).is_ok());
        let mut replies = out.as_slice();
        assert!(matches!(
            decode_frame::<WorkerHostReply, _>(&mut replies).unwrap(),
            WorkerHostReply::Hello {
                generation: WORKER_HOST_GENERATION,
                ..
            }
        ));
        assert_eq!(
            decode_frame::<WorkerHostReply, _>(&mut replies).unwrap(),
            WorkerHostReply::Error {
                code: "no_lifetime".into()
            }
        );
        assert_eq!(
            decode_frame::<WorkerHostReply, _>(&mut replies).unwrap(),
            WorkerHostReply::Bye {}
        );
    }
}
