use cotra_approval::{ApprovalBroker, ApprovalDecision, ApprovalPrompt};
use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use cotra_provider_git::{GitMutationState, GitProvider, GitProviderError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn dispatch(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Option<Value>, ProviderError> {
    let operation = match (request.capability.as_str(), request.operation.as_str()) {
        ("git.branch.create", "create") => Mutation::BranchCreate {
            branch: required_string(request, "branch")?.to_owned(),
        },
        ("git.stage", "stage") => Mutation::Stage {
            paths: required_paths(request)?,
        },
        ("git.unstage", "unstage") => Mutation::Unstage {
            paths: required_paths(request)?,
        },
        ("git.commit", "commit") => Mutation::Commit {
            message: required_string(request, "message")?.to_owned(),
        },
        _ => return Ok(None),
    };

    let repository = request.target.as_deref().ok_or_else(|| {
        ProviderError::new(
            FailureCode::InvalidRequest,
            "Git mutation repository target is required",
        )
    })?;
    let expected_head = required_string(request, "expected_head")?;
    let provider = GitProvider::new(&workspace.root).map_err(map_git_error)?;
    let approved_state = operation.state(&provider, repository)?;
    require_expected_head(&approved_state, expected_head)?;

    let prompt = ApprovalPrompt {
        workspace_id: workspace.id.clone(),
        action: operation.action().to_owned(),
        target: repository.to_owned(),
        summary: operation.summary(&approved_state),
        digest: approval_digest(workspace, request, &approved_state, &operation),
    };
    require_approval(approval, &prompt)?;

    let current_state = operation.state(&provider, repository)?;
    if current_state != approved_state {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "Git repository state changed after local approval",
        ));
    }
    require_expected_head(&current_state, expected_head)?;

    let result = operation.apply(&provider, repository, &approved_state)?;
    Ok(Some(result))
}

#[derive(Debug)]
enum Mutation {
    BranchCreate { branch: String },
    Stage { paths: Vec<String> },
    Unstage { paths: Vec<String> },
    Commit { message: String },
}

impl Mutation {
    fn state(
        &self,
        provider: &GitProvider,
        repository: &str,
    ) -> Result<GitMutationState, ProviderError> {
        match self {
            Self::BranchCreate { .. } => provider.branch_state(repository),
            Self::Stage { paths } => provider.path_state(repository, paths),
            Self::Unstage { paths } => provider.unstage_state(repository, paths),
            Self::Commit { .. } => provider.staged_state(repository),
        }
        .map_err(map_git_error)
    }

    fn action(&self) -> &'static str {
        match self {
            Self::BranchCreate { .. } => "create local Git branch",
            Self::Stage { .. } => "stage local Git paths",
            Self::Unstage { .. } => "unstage local Git paths",
            Self::Commit { .. } => "create local Git commit",
        }
    }

    fn summary(&self, state: &GitMutationState) -> String {
        match self {
            Self::BranchCreate { branch } => format!(
                "branch={branch} expected_head={} current_branch={}",
                state.head,
                state.branch.as_deref().unwrap_or("DETACHED")
            ),
            Self::Stage { paths } => format!(
                "paths={} expected_head={} current_branch={}",
                paths.len(),
                state.head,
                state.branch.as_deref().unwrap_or("DETACHED")
            ),
            Self::Unstage { paths } => format!(
                "paths={} expected_head={} current_branch={}",
                paths.len(),
                state.head,
                state.branch.as_deref().unwrap_or("DETACHED")
            ),
            Self::Commit { message } => format!(
                "staged_state_bound=true message_bytes={} expected_head={} current_branch={}",
                message.len(),
                state.head,
                state.branch.as_deref().unwrap_or("DETACHED")
            ),
        }
    }

    fn apply(
        &self,
        provider: &GitProvider,
        repository: &str,
        approved: &GitMutationState,
    ) -> Result<Value, ProviderError> {
        match self {
            Self::BranchCreate { branch } => {
                provider.create_branch(repository, branch, approved)
            }
            Self::Stage { paths } => provider.stage(repository, paths, approved),
            Self::Unstage { paths } => provider.unstage(repository, paths, approved),
            Self::Commit { message } => provider.commit(repository, message, approved),
        }
        .map_err(map_git_error)
    }

    fn digest_material(&self, hasher: &mut Sha256) {
        match self {
            Self::BranchCreate { branch } => {
                digest_field(hasher, b"BRANCH_CREATE");
                digest_field(hasher, branch.as_bytes());
            }
            Self::Stage { paths } => {
                digest_field(hasher, b"STAGE");
                for path in paths {
                    digest_field(hasher, path.as_bytes());
                }
            }
            Self::Unstage { paths } => {
                digest_field(hasher, b"UNSTAGE");
                for path in paths {
                    digest_field(hasher, path.as_bytes());
                }
            }
            Self::Commit { message } => {
                digest_field(hasher, b"COMMIT");
                let message_digest = Sha256::digest(message.as_bytes());
                digest_field(hasher, &message_digest);
            }
        }
    }
}

fn approval_digest(
    workspace: &Workspace,
    request: &RequestEnvelope,
    state: &GitMutationState,
    mutation: &Mutation,
) -> String {
    let mut hasher = Sha256::new();
    digest_field(&mut hasher, b"COTRA_GIT_MUTATION_APPROVAL_V1");
    digest_field(&mut hasher, workspace.id.as_bytes());
    digest_field(&mut hasher, POLICY_REVISION.as_bytes());
    digest_field(&mut hasher, request.capability.as_bytes());
    digest_field(&mut hasher, request.operation.as_bytes());
    digest_field(
        &mut hasher,
        request.target.as_deref().unwrap_or_default().as_bytes(),
    );
    digest_field(&mut hasher, state.repository_root.as_bytes());
    digest_field(&mut hasher, state.head.as_bytes());
    digest_field(
        &mut hasher,
        state.branch.as_deref().unwrap_or("DETACHED").as_bytes(),
    );
    let material_digest = Sha256::digest(state.material.as_bytes());
    digest_field(&mut hasher, &material_digest);
    mutation.digest_material(&mut hasher);
    hex_lower(&hasher.finalize())
}

fn require_expected_head(state: &GitMutationState, expected: &str) -> Result<(), ProviderError> {
    if state.head != expected {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "Git HEAD does not match arguments.expected_head",
        ));
    }
    Ok(())
}

fn require_approval(
    approval: &impl ApprovalBroker,
    prompt: &ApprovalPrompt,
) -> Result<(), ProviderError> {
    match approval.request(prompt) {
        Ok(ApprovalDecision::Approved) => Ok(()),
        Ok(ApprovalDecision::Denied) => Err(ProviderError::new(
            FailureCode::ApprovalDenied,
            "local user denied Git mutation",
        )),
        Err(error) => Err(ProviderError::new(error.code, error.message)),
    }
}

fn required_string<'a>(
    request: &'a RequestEnvelope,
    name: &str,
) -> Result<&'a str, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("Git mutation requires arguments.{name}"),
            )
        })
}

fn required_paths(request: &RequestEnvelope) -> Result<Vec<String>, ProviderError> {
    request
        .arguments
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                "Git mutation requires arguments.paths",
            )
        })?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                ProviderError::new(
                    FailureCode::InvalidRequest,
                    "Git mutation path entries must be strings",
                )
            })
        })
        .collect()
}

fn map_git_error(error: GitProviderError) -> ProviderError {
    ProviderError::new(error.code, error.message)
}

fn digest_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use cotra_approval::test_support::FixedApprovalBroker;
    use cotra_contracts::INTERNAL_PROTOCOL_VERSION;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cotra-git-dispatch-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        root
    }

    fn git(cwd: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn repository() -> PathBuf {
        let root = temp_root();
        git(&root, &["init"]);
        git(&root, &["config", "user.name", "Cotra Test"]);
        git(&root, &["config", "user.email", "cotra@example.invalid"]);
        fs::write(root.join("a.txt"), "one\n").expect("a");
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-m", "initial"]);
        root
    }

    fn workspace(root: &Path) -> Workspace {
        Workspace {
            id: "default".into(),
            root: fs::canonicalize(root).expect("canonical"),
        }
    }

    fn request(root: &Path, capability: &str, operation: &str, arguments: Value) -> RequestEnvelope {
        let head = git(root, &["rev-parse", "HEAD"]).trim().to_owned();
        let mut arguments = arguments;
        arguments["expected_head"] = json!(head);
        RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "git-mutation-test".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: capability.into(),
            operation: operation.into(),
            target: Some(".".into()),
            arguments,
        }
    }

    #[test]
    fn denied_approval_preserves_git_state() {
        struct Deny;
        impl ApprovalBroker for Deny {
            fn request(
                &self,
                _prompt: &ApprovalPrompt,
            ) -> Result<ApprovalDecision, cotra_approval::ApprovalError> {
                Ok(ApprovalDecision::Denied)
            }
        }

        let root = repository();
        fs::write(root.join("a.txt"), "two\n").expect("modify");
        let request = request(
            &root,
            "git.stage",
            "stage",
            json!({"paths": ["a.txt"]}),
        );
        let before = git(&root, &["diff", "--cached", "--name-only"]);
        let error = dispatch(&workspace(&root), &Deny, &request)
            .expect_err("denial")
            .expect("handled mutation");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        assert_eq!(git(&root, &["diff", "--cached", "--name-only"]), before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn approved_stage_mutates_only_after_bound_approval() {
        let root = repository();
        fs::write(root.join("a.txt"), "two\n").expect("modify");
        let request = request(
            &root,
            "git.stage",
            "stage",
            json!({"paths": ["a.txt"]}),
        );
        let result = dispatch(
            &workspace(&root),
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request,
        )
        .expect("dispatch")
        .expect("handled");
        assert_eq!(result["paths"][0], "a.txt");
        assert_eq!(
            git(&root, &["diff", "--cached", "--name-only"]).trim(),
            "a.txt"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    fn null_device() -> &'static str {
        "NUL"
    }

    #[cfg(not(windows))]
    fn null_device() -> &'static str {
        "/dev/null"
    }
}
