//! SG-000072 repository-wide release-blocker sweep. Fails on any TODO, FIXME,
//! HACK, or XXX marker in code, any todo!/unimplemented! macro, any panic! or
//! unreachable! in non-test code (except the single documented guarded match),
//! focused-test modifiers, console/debugger output, mocks in non-test code,
//! secret-shaped literals outside the exact allowlisted redaction/test
//! vectors, unexpected ignored tests, or unexpected workflows. Any new hit
//! fails the suite until it is fixed at its root or explicitly allowlisted
//! here with a reason.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn is_test_file(relative: &str) -> bool {
    relative.ends_with(".test.ts")
        || relative.ends_with(".test.js")
        || relative.ends_with(".test.mjs")
        || relative.ends_with("tests.rs")
        || relative.contains("tests/")
        || relative.contains("sg000")
        || relative.starts_with("crates/")
            && (relative.contains("_tests.rs") || relative.contains("/tests/"))
        || relative.contains("examples/")
}

fn text_files(root: &Path) -> Vec<(String, String)> {
    // The gate never scans itself: its allowlists name the exact vectors, and
    // the gate definition is reviewed as ordinary PR diff instead.
    const SELF: &str = "crates/qdral-lifecycle/tests/release_blocker_sweep.rs";
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if matches!(name.as_str(), ".git" | "target" | "node_modules" | "dist") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative.ends_with(".lock") || relative == "package-lock.json" || relative == SELF {
                continue;
            }
            let bytes = fs::read(&path).unwrap_or_default();
            if bytes.len() > 1_000_000 || bytes.iter().take(8192).any(|b| *b == 0) {
                continue;
            }
            if let Ok(text) = String::from_utf8(bytes) {
                out.push((relative, text));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn code_file(relative: &str) -> bool {
    matches!(
        Path::new(relative).extension().and_then(|e| e.to_str()),
        Some(
            "rs" | "ts"
                | "mts"
                | "cts"
                | "js"
                | "mjs"
                | "cjs"
                | "ps1"
                | "py"
                | "sh"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "jsonc"
        )
    )
}

/// (path, allowed line substring) pairs. Every other hit fails the gate.
fn marker_allowlist() -> BTreeMap<&'static str, Vec<&'static str>> {
    let mut map: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    map.insert(
        "distribution/openai/review/test-cases.json",
        vec!["Find every line that mentions TODO"],
    );
    map.insert(
        "distribution/openai/plugin.config.json",
        vec!["Find every TODO in the src folder."],
    );
    map.insert(
        "docs/security/RELEASE_SECURITY_REVIEW.md",
        vec!["no `TODO`, `FIXME`, `XXX`, `HACK`, `todo!()`, or `unimplemented!()` in code"],
    );
    map
}

fn secret_allowlist() -> BTreeMap<&'static str, Vec<&'static str>> {
    let mut map: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    map.insert(
        "apps/qdral-mcp/src/client-examples.test.ts",
        vec![r#"!text.includes("BEGIN PRIVATE")"#],
    );
    map.insert(
        "apps/qdral-mcp/src/openai-plugin.test.ts",
        vec!["-----BEGIN PRIVATE KEY-----"],
    );
    map.insert(
        "apps/qdral-mcp/src/git_push.test.ts",
        vec!["ghp_rawtoken123"],
    );
    map.insert("crates/qdral-tunnel/src/lib.rs", vec!["sk-proj-test"]);
    map.insert(
        "crates/qdral-provider-clipboard/src/lib.rs",
        vec!["\"sk-live-\""],
    );
    map.insert(
        "crates/qdral-provider-clipboard/src/tests.rs",
        vec!["concat!(\"ghp_\""],
    );
    map.insert(
        "crates/qdral-lifecycle/src/logs.rs",
        vec![
            "&[(\"github_pat_\", 16), (\"sess-\", 8), (\"ghp_\", 16), (\"sk-\", 8)]",
            "connect api_key=sk-proj-abcdef123456",
            "&[\"sk-proj-abcdef123456\", \"abc\", \"xyz.123\", \"sk-1234567890ab\"]",
            "key=sk-proj-abcdef123456",
            "\"OPENAI_API_KEY=sk-proj-abcdef123456\",",
            "&[\"sk-proj-abcdef123456\"],",
        ],
    );
    map.insert(
        "crates/qdral-policy/src/sg000017.rs",
        vec!["ghp_rawtoken123"],
    );
    map.insert(
        "crates/qdral-provider-git/src/push.rs",
        vec!["\"ghp_abcdef123456\""],
    );
    map.insert(
        "crates/qdral-lifecycle/tests/release_qualification.rs",
        vec!["sk-release-qualification-key-0123456789"],
    );
    map
}

fn allowed(allowlist: &BTreeMap<&str, Vec<&str>>, path: &str, line: &str) -> bool {
    allowlist
        .get(path)
        .map(|subs| subs.iter().any(|s| line.contains(s)))
        .unwrap_or(false)
}

fn contains_marker(line: &str) -> bool {
    for token in ["TODO", "FIXME", "HACK", "XXX"] {
        let mut rest = line;
        while let Some(pos) = rest.find(token) {
            let before = rest[..pos].chars().next_back();
            let after = rest[pos + token.len()..].chars().next();
            let before_ok = before
                .map(|c| c.is_ascii_alphanumeric() || c == '_')
                .unwrap_or(false);
            let after_ok = after
                .map(|c| c.is_ascii_alphanumeric() || c == '_')
                .unwrap_or(false);
            if !before_ok && !after_ok {
                return true;
            }
            rest = &rest[pos + token.len()..];
        }
    }
    false
}

fn contains_secret_shape(line: &str) -> bool {
    if line.contains("BEGIN PRIVATE") {
        return true;
    }
    for prefix in ["sk-proj-", "sk-live-", "ghp_", "gho_", "github_pat_"] {
        let mut rest = line;
        while let Some(pos) = rest.find(prefix) {
            let tail = &rest[pos + prefix.len()..];
            let run = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                .count();
            if run >= 6 {
                return true;
            }
            rest = &rest[pos + prefix.len()..];
        }
    }
    false
}

#[test]
fn no_release_blocker_markers_in_code() {
    let root = repo_root();
    let allowlist = marker_allowlist();
    let mut hits = Vec::new();
    for (relative, text) in text_files(&root) {
        // Governance specifications describe the sweep requirement itself in
        // prose; they are not shipped code markers.
        if !code_file(&relative) || relative.starts_with(".specgrain/") {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            if (contains_marker(line) || line.contains("todo!") || line.contains("unimplemented!"))
                && !allowed(&allowlist, &relative, line)
            {
                hits.push(format!("{}:{}: {}", relative, index + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "release-blocker markers:\n{}",
        hits.join("\n")
    );
}

#[test]
fn no_panic_unreachable_or_mock_in_non_test_code() {
    let root = repo_root();
    let mut hits = Vec::new();
    for (relative, text) in text_files(&root) {
        // Governance specifications describe sweep requirements in prose;
        // they are not shipped code.
        if !code_file(&relative) || is_test_file(&relative) || relative.starts_with(".specgrain/") {
            continue;
        }
        let is_rust = relative.ends_with(".rs");
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if is_rust && line.contains("panic!") {
                hits.push(format!("{relative}:{}: {trimmed}", index + 1));
            }
            if is_rust
                && line.contains("unreachable!")
                && !(relative == "crates/qdral-provider-process/src/lib.rs"
                    && line
                        .contains("PrivateExecutionMode::Public | PrivateExecutionMode::Success"))
            {
                hits.push(format!("{relative}:{}: {trimmed}", index + 1));
            }
            if line.to_lowercase().contains("mock") {
                hits.push(format!("{relative}:{}: {trimmed}", index + 1));
            }
            // Build/generator scripts report progress on stdout as ordinary CLI
            // output; this rule targets the shipped runtime under apps/.
            if relative.starts_with("apps/")
                && (relative.ends_with(".ts")
                    || relative.ends_with(".js")
                    || relative.ends_with(".mjs"))
                && (line.contains("console.log")
                    || line.contains("console.debug")
                    || line.contains("debugger"))
            {
                hits.push(format!("{relative}:{}: {trimmed}", index + 1));
            }
            if line.contains(".only(") {
                hits.push(format!("{relative}:{}: {trimmed}", index + 1));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "non-test code blockers:\n{}",
        hits.join("\n")
    );
}

#[test]
fn no_secret_shaped_literals_outside_allowlisted_vectors() {
    let root = repo_root();
    let allowlist = secret_allowlist();
    let mut hits = Vec::new();
    for (relative, text) in text_files(&root) {
        for (index, line) in text.lines().enumerate() {
            if contains_secret_shape(line) && !allowed(&allowlist, &relative, line) {
                hits.push(format!("{}:{}: {}", relative, index + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "secret-shaped literals:\n{}",
        hits.join("\n")
    );
}

#[test]
fn only_the_documented_ignored_tests_exist() {
    let root = repo_root();
    let mut hits = Vec::new();
    for (relative, text) in text_files(&root) {
        if !relative.ends_with(".rs") {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            if line.contains("#[ignore") {
                let documented = (relative
                    == "crates/qdral-lifecycle/tests/release_qualification.rs"
                    && line.contains(
                        "requires QDRAL_RELEASE_DIR; run by the release-qualification CI job",
                    ))
                    || (relative == "crates/qdral-provider-network/tests/sg000064_live.rs"
                        && line.contains(
                            "performs one real HTTPS request; run with --ignored for live evidence",
                        ));
                if !documented {
                    hits.push(format!("{}:{}: {}", relative, index + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "unexpected ignored tests:\n{}",
        hits.join("\n")
    );
}

#[test]
fn workflow_set_is_exactly_the_governed_quartet() {
    let root = repo_root();
    let dir = root.join(".github/workflows");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["ci.yml", "pages.yml", "release.yml", "review-gates.yml"]
    );
}
