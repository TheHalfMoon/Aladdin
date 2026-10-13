//! SG-000108 product caller, slice 1a: the real `qdral-browser-host
//! --worker` binary under Deskal supervision, against the loopback HTTPS
//! fixture. Native Windows only, with the built worker and an installed
//! engine; run by `npm run browser-launcher:native`.
#![cfg(all(windows, feature = "live-browser-test"))]

use qdral_browser_host::worker_host::{WorkerHostReply, WorkerHostRequest};
use qdral_browser_host::{Supervisor, WorkerHostLaunch};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

fn host_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_qdral-browser-host"))
}

/// Node and the worker come from the test harness's environment: a source
/// that exists only in test builds (docs/security/SG-000108_PRODUCT_CALLER.md).
fn prerequisites() -> (PathBuf, PathBuf) {
    let node = std::env::var_os("QDRAL_SG108_NODE").expect("QDRAL_SG108_NODE");
    let main = std::env::var_os("QDRAL_SG108_WORKER_MAIN").expect("QDRAL_SG108_WORKER_MAIN");
    (PathBuf::from(node), PathBuf::from(main))
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "sg108-worker-host-{label}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp root");
    dir
}

struct Fixture {
    child: std::process::Child,
    spki: String,
}

impl Fixture {
    fn start() -> Self {
        let (node, main) = prerequisites();
        let app = main.parent().and_then(Path::parent).expect("worker app");
        let mut child = std::process::Command::new(node)
            .arg(app.join("scripts").join("https-fixture.mjs"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("start the HTTPS fixture");
        let stdout = child.stdout.take().expect("fixture stdout");
        let (ready, spki) = mpsc::channel();
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

fn launch(label: &str) -> (Supervisor, WorkerHostLaunch, PathBuf) {
    let (node, worker_main) = prerequisites();
    let root = temp_root(label);
    let launch = WorkerHostLaunch {
        node,
        worker_main,
        profile_root: root.join("profiles"),
        temp_dir: root.join("staging"),
        state_root: root.join("state"),
    };
    for dir in [&launch.profile_root, &launch.temp_dir, &launch.state_root] {
        std::fs::create_dir_all(dir).expect("dirs");
    }
    let mut supervisor = Supervisor::new();
    supervisor
        .launch_worker_host(&host_binary(), &launch)
        .expect("launch the worker-mode host");
    (supervisor, launch, root)
}

fn entries(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|read| {
            read.filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_lifetime_through_the_real_host_binary_navigates_observes_and_closes_clean() {
    let fixture = Fixture::start();
    let (mut supervisor, launch, root) = launch("lifetime");
    let host = supervisor.worker_host().expect("live worker host");
    let (lifetime, pinned) = match host
        .request(&WorkerHostRequest::Open {
            workspace: "ws-sg108".into(),
            host: "fixture.test".into(),
            fixture_spki: fixture.spki.clone(),
        })
        .expect("open")
    {
        WorkerHostReply::Opened { lifetime, pinned } => (lifetime, pinned),
        other => panic!("open refused: {other:?}"),
    };
    assert_eq!(lifetime.len(), 32, "a 128-bit lifetime nonce");
    assert!(lifetime.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(pinned, "127.0.0.1");
    // The host made its own pid-named profile under the root (#298).
    let profiles = entries(&launch.profile_root);
    assert_eq!(profiles.len(), 1, "{profiles:?}");
    assert!(
        profiles[0].starts_with(&format!("qdral-sg108-profile-{}-", host.pid())),
        "{profiles:?}"
    );
    // A second lifetime while one is open is refused, typed.
    assert_eq!(
        host.request(&WorkerHostRequest::Open {
            workspace: "ws-sg108".into(),
            host: "fixture.test".into(),
            fixture_spki: fixture.spki.clone(),
        })
        .expect("second open"),
        WorkerHostReply::Error {
            code: "lifetime_open".into()
        }
    );
    match host
        .request(&WorkerHostRequest::Navigate {
            url: "https://fixture.test/reports".into(),
        })
        .expect("navigate")
    {
        WorkerHostReply::Navigated { url, status } => {
            assert_eq!(url, "https://fixture.test/reports");
            assert_eq!(status, 200);
        }
        other => panic!("navigation refused: {other:?}"),
    }
    // Another origin is outside the lifetime's single destination.
    assert!(matches!(
        host.request(&WorkerHostRequest::Navigate {
            url: "https://other.test/".into(),
        })
        .expect("navigate elsewhere"),
        WorkerHostReply::Error { .. }
    ));
    let observed = host
        .request(&WorkerHostRequest::Observe {})
        .expect("observe");
    let text = serde_json::to_string(&observed).expect("reply text");
    match &observed {
        WorkerHostReply::Observed {
            origin,
            nodes,
            node_count,
            ..
        } => {
            assert_eq!(origin, "https://fixture.test:443");
            assert!(*node_count > 0 && !nodes.is_empty());
        }
        other => panic!("observation refused: {other:?}"),
    }
    // Identities and digests only: the page's text never crosses.
    assert!(
        !text.contains("Quarterly report"),
        "raw page text crossed: {text}"
    );
    println!(
        "SG108-EVIDENCE worker host observation: {} bytes, no raw page text",
        text.len()
    );
    match host.request(&WorkerHostRequest::Close {}).expect("close") {
        WorkerHostReply::Closed {
            job_empty,
            profile_removed,
        } => assert!(job_empty && profile_removed),
        other => panic!("close refused: {other:?}"),
    }
    assert!(
        entries(&launch.profile_root).is_empty(),
        "the profile stayed"
    );
    // After the lifetime: typed refusals, the host keeps serving.
    assert_eq!(
        host.request(&WorkerHostRequest::Observe {})
            .expect("observe after close"),
        WorkerHostReply::Error {
            code: "no_lifetime".into()
        }
    );
    assert!(
        supervisor.shutdown_worker_host().expect("shutdown"),
        "the host family stayed"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_refused_open_leaves_no_profile_behind() {
    let fixture = Fixture::start();
    let (mut supervisor, launch, root) = launch("refused-open");
    let host = supervisor.worker_host().expect("live worker host");
    for (host_name, spki) in [
        ("not a host", fixture.spki.clone()),
        ("fixture.test", "short".to_string()),
    ] {
        assert!(matches!(
            host.request(&WorkerHostRequest::Open {
                workspace: "ws-sg108".into(),
                host: host_name.into(),
                fixture_spki: spki,
            })
            .expect("open"),
            WorkerHostReply::Error { .. }
        ));
    }
    assert!(
        entries(&launch.profile_root).is_empty(),
        "a refused open left a profile"
    );
    assert!(supervisor.shutdown().expect("shutdown"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_launch_refused_after_the_profile_exists_removes_it() {
    // The worker entry is an existing file that is not `main.js`: the open
    // passes every check that needs no profile, makes the lifetime's
    // profile, and is then refused by the launch (typed). The profile it
    // made must be gone.
    let fixture = Fixture::start();
    let (node, _) = prerequisites();
    let root = temp_root("refused-launch");
    let launch = WorkerHostLaunch {
        worker_main: node.clone(),
        node,
        profile_root: root.join("profiles"),
        temp_dir: root.join("staging"),
        state_root: root.join("state"),
    };
    for dir in [&launch.profile_root, &launch.temp_dir, &launch.state_root] {
        std::fs::create_dir_all(dir).expect("dirs");
    }
    let mut supervisor = Supervisor::new();
    supervisor
        .launch_worker_host(&host_binary(), &launch)
        .expect("launch the worker-mode host");
    let host = supervisor.worker_host().expect("live worker host");
    let reply = host
        .request(&WorkerHostRequest::Open {
            workspace: "ws-sg108".into(),
            host: "fixture.test".into(),
            fixture_spki: fixture.spki.clone(),
        })
        .expect("open");
    assert_eq!(
        reply,
        WorkerHostReply::Error {
            code: "unavailable:unsupportedruntime".into()
        }
    );
    assert!(
        entries(&launch.profile_root).is_empty(),
        "the refused launch left its profile"
    );
    assert!(supervisor.shutdown().expect("shutdown"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn shutting_the_host_down_mid_lifetime_takes_the_worker_and_profile_with_it() {
    let fixture = Fixture::start();
    let (mut supervisor, launch, root) = launch("shutdown");
    let host = supervisor.worker_host().expect("live worker host");
    assert!(matches!(
        host.request(&WorkerHostRequest::Open {
            workspace: "ws-sg108".into(),
            host: "fixture.test".into(),
            fixture_spki: fixture.spki.clone(),
        })
        .expect("open"),
        WorkerHostReply::Opened { .. }
    ));
    // Only one host at a time, of either mode.
    assert!(supervisor
        .launch_worker_host(&host_binary(), &launch)
        .is_err());
    // The supervisor's own shutdown covers the worker-mode host.
    assert!(
        supervisor.shutdown().expect("shutdown"),
        "the host family stayed"
    );
    assert!(!supervisor.is_live());
    assert!(
        entries(&launch.profile_root).is_empty(),
        "the lifetime's profile stayed"
    );
    let _ = std::fs::remove_dir_all(&root);
}
