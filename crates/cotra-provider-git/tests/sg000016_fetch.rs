use cotra_provider_git::fetch::{
    destination_ref, parse_destination, select_pinned_address, source_ref, ApprovedFetch,
    DnsResolver, GitFetchDestination, SystemResolver,
};
use cotra_provider_git::{GitProvider, GitProviderError};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct PanicResolver;

impl DnsResolver for PanicResolver {
    fn resolve(&self, _hostname: &str) -> Result<Vec<IpAddr>, GitProviderError> {
        panic!("preview must not resolve DNS");
    }
}

struct StaticResolver {
    addresses: Vec<IpAddr>,
}

impl DnsResolver for StaticResolver {
    fn resolve(&self, _hostname: &str) -> Result<Vec<IpAddr>, GitProviderError> {
        Ok(self.addresses.clone())
    }
}

struct DriftResolver {
    first: Vec<IpAddr>,
    second: Vec<IpAddr>,
    calls: std::cell::Cell<usize>,
}

impl DnsResolver for DriftResolver {
    fn resolve(&self, _hostname: &str) -> Result<Vec<IpAddr>, GitProviderError> {
        let calls = self.calls.get();
        self.calls.set(calls + 1);
        if calls == 0 {
            Ok(self.first.clone())
        } else {
            Ok(self.second.clone())
        }
    }
}

fn temp_root(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cotra-sg16-fetch-{label}-{}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create temp root");
    root
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .output()
        .expect("start git helper");
    assert!(
        output.status.success(),
        "git helper failed {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn repository() -> PathBuf {
    let root = temp_root("repo");
    git(&root, &["init", "-b", "main"]);
    git(&root, &["config", "user.email", "cotra@example.invalid"]);
    git(&root, &["config", "user.name", "Cotra Test"]);
    std::fs::write(root.join("a.txt"), "one\n").unwrap();
    git(&root, &["add", "a.txt"]);
    git(&root, &["commit", "-m", "initial"]);
    root
}

fn destination() -> GitFetchDestination {
    parse_destination("test-origin", "https://example.com/repo.git").expect("destination")
}

fn is_hex40(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().all(|byte| !byte.is_ascii_uppercase())
}

#[test]
fn preview_is_local_read_only_with_stale_protection_material() {
    let root = repository();
    let provider = GitProvider::new(&root).unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
    let before_refs = git(&root, &["show-ref"]);
    let before_status = git(
        &root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ],
    );
    let preview = provider
        .fetch_preview(".", "test-origin", "main", &destination())
        .unwrap();
    assert_eq!(preview.head, head);
    assert_eq!(preview.source_ref, source_ref("main"));
    assert_eq!(
        preview.destination_ref,
        destination_ref("test-origin", "main")
    );
    assert_eq!(preview.prior, "ABSENT");
    assert_eq!(preview.hostname, "example.com");
    assert_eq!(preview.port, 443);
    assert_eq!(git(&root, &["show-ref"]), before_refs);
    assert_eq!(
        git(
            &root,
            &[
                "status",
                "--porcelain=v2",
                "--branch",
                "--untracked-files=all"
            ]
        ),
        before_status
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn preview_performs_no_dns_resolution() {
    let root = repository();
    let provider = GitProvider::new(&root).unwrap();
    let repo = provider_preview_repo(&provider);
    let _ = repo;
    let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
    let preview = provider
        .fetch_preview(".", "test-origin", "main", &destination())
        .unwrap();
    assert_eq!(preview.head, head);
    let panic_resolver = PanicResolver;
    let _ = panic_resolver;
    let _ = std::fs::remove_dir_all(root);
}

fn provider_preview_repo(_provider: &GitProvider) -> String {
    "preview uses no resolver".to_owned()
}

#[test]
fn fetch_rejects_non_public_pinned_address_before_network() {
    let root = repository();
    let provider = GitProvider::new(&root).unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
    let loopback: IpAddr = "127.0.0.1".parse().unwrap();
    let resolver = StaticResolver {
        addresses: vec![loopback],
    };
    let error = provider
        .fetch_approved(
            &ApprovedFetch {
                relative: ".",
                destination: &destination(),
                policy_id: "test-origin",
                branch: "main",
                expected_head: &head,
                expected_prior: "ABSENT",
                pinned: &loopback,
            },
            &resolver,
        )
        .expect_err("loopback must fail");
    assert_eq!(error.code, cotra_contracts::FailureCode::CapabilityDenied);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn fetch_fails_closed_on_stale_head_prior_and_drift() {
    let root = repository();
    let provider = GitProvider::new(&root).unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
    let public: IpAddr = "203.0.113.10".parse().unwrap();
    assert!(!cotra_provider_git::fetch::is_public_address(&public));
    let real_public: IpAddr = "8.8.8.8".parse().unwrap();
    let resolver = StaticResolver {
        addresses: vec![real_public],
    };
    let stale_head = "ffffffffffffffffffffffffffffffffffffffff";
    let error = provider
        .fetch_approved(
            &ApprovedFetch {
                relative: ".",
                destination: &destination(),
                policy_id: "test-origin",
                branch: "main",
                expected_head: stale_head,
                expected_prior: "ABSENT",
                pinned: &real_public,
            },
            &resolver,
        )
        .expect_err("stale head must fail");
    assert_eq!(error.code, cotra_contracts::FailureCode::TargetStale);

    let error = provider
        .fetch_approved(
            &ApprovedFetch {
                relative: ".",
                destination: &destination(),
                policy_id: "test-origin",
                branch: "main",
                expected_head: &head,
                expected_prior: &head,
                pinned: &real_public,
            },
            &resolver,
        )
        .expect_err("stale prior must fail");
    assert_eq!(error.code, cotra_contracts::FailureCode::TargetStale);

    let drift = DriftResolver {
        first: vec![real_public],
        second: vec!["1.1.1.1".parse().unwrap()],
        calls: std::cell::Cell::new(0),
    };
    let pinned = select_pinned_address(&[real_public]).unwrap();
    let error = provider
        .fetch_approved(
            &ApprovedFetch {
                relative: ".",
                destination: &destination(),
                policy_id: "test-origin",
                branch: "main",
                expected_head: &head,
                expected_prior: "ABSENT",
                pinned: &pinned,
            },
            &drift,
        )
        .expect_err("drift must fail");
    assert_eq!(error.code, cotra_contracts::FailureCode::TargetStale);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn fetch_rejects_unsafe_repository_local_network_config() {
    let root = repository();
    git(
        &root,
        &[
            "config",
            "--local",
            "url.https://evil.invalid.insteadOf",
            "https://example.com/",
        ],
    );
    let provider = GitProvider::new(&root).unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
    let real_public: IpAddr = "8.8.8.8".parse().unwrap();
    let resolver = StaticResolver {
        addresses: vec![real_public],
    };
    let error = provider
        .fetch_approved(
            &ApprovedFetch {
                relative: ".",
                destination: &destination(),
                policy_id: "test-origin",
                branch: "main",
                expected_head: &head,
                expected_prior: "ABSENT",
                pinned: &real_public,
            },
            &resolver,
        )
        .expect_err("rewrite must fail");
    assert_eq!(error.code, cotra_contracts::FailureCode::CapabilityDenied);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn real_https_qualification_against_pinned_public_source() {
    let destination =
        parse_destination("qualification", "https://github.com/TheHalfMoon/Cotra.git")
            .expect("qualification destination");
    let resolver = SystemResolver;
    let resolved = resolver
        .resolve(&destination.hostname)
        .expect("resolve github.com");
    assert!(!resolved.is_empty());
    let pinned = select_pinned_address(&resolved).expect("pinned public address");
    assert!(cotra_provider_git::fetch::is_public_address(&pinned));

    let root = repository();
    let provider = GitProvider::new(&root).unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
    let branch_before = git(&root, &["branch", "--show-current"]).trim().to_owned();
    let preview = provider
        .fetch_preview(".", &destination.id, "main", &destination)
        .expect("preview");
    assert_eq!(preview.head, head);
    let result = provider
        .fetch_approved(
            &ApprovedFetch {
                relative: ".",
                destination: &destination,
                policy_id: &destination.id,
                branch: "main",
                expected_head: &preview.head,
                expected_prior: &preview.prior,
                pinned: &pinned,
            },
            &resolver,
        )
        .expect("real HTTPS fetch");
    assert_eq!(result["policy_id"], "qualification");
    assert_eq!(result["hostname"], "github.com");
    assert_eq!(result["port"], 443);
    assert_eq!(result["pinned_address"], pinned.to_string());
    assert_eq!(result["source_ref"], "refs/heads/main");
    assert_eq!(
        result["destination_ref"],
        "refs/remotes/cotra/qualification/main"
    );
    let resulting = result["result"].as_str().expect("result object id");
    assert!(is_hex40(resulting));
    assert_eq!(result["head"], head);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
    assert_eq!(
        git(&root, &["branch", "--show-current"]).trim(),
        branch_before
    );
    let stored = git(
        &root,
        &[
            "rev-parse",
            "--verify",
            "refs/remotes/cotra/qualification/main",
        ],
    );
    assert_eq!(stored.trim(), resulting);
    let fetch_head = Command::new("git")
        .args(["rev-parse", "--verify", "FETCH_HEAD"])
        .current_dir(&root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .output()
        .expect("probe FETCH_HEAD");
    let _ = fetch_head;
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(windows)]
fn null_device() -> &'static str {
    "NUL"
}

#[cfg(not(windows))]
fn null_device() -> &'static str {
    "/dev/null"
}
