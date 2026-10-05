//! SG-000074 exact engine command line.
//!
//! The host launches the engine with exactly the frozen flag set and
//! nothing else. Any forbidden flag fails closed at construction, and the
//! same check runs in tests over every argv the builder emits.

use crate::error::HostError;
use std::path::Path;

/// Flags the host passes to the engine, in order. This set is frozen:
/// sandbox-enabling, extensionless, background-quiet, ephemeral.
const ALLOWED_FLAGS: &[&str] = &[
    "--headless",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--disable-background-networking",
    "--disable-sync",
    "--no-service-autorun",
];

/// Flags that must never appear. Each weakens the sandbox, opens a
/// control surface, loads foreign code, or escapes the profile.
const FORBIDDEN_FLAGS: &[&str] = &[
    "--no-sandbox",
    "--disable-setuid-sandbox",
    "--no-zygote",
    "--single-process",
    "--remote-debugging-port",
    "--remote-debugging-address",
    "--load-extension",
    "--load-component-extension",
    "--disable-web-security",
    "--enable-automation",
    "--disable-component-update",
];

const INITIAL_URL: &str = "about:blank";

/// Build the exact engine command line: frozen flags, the Deskal-owned
/// profile directory, and the blank initial page. No caller input beyond
/// the validated engine path and profile directory ever reaches argv.
pub fn build_argv(profile_dir: &Path) -> Result<Vec<String>, HostError> {
    let profile = profile_dir
        .to_str()
        .ok_or_else(|| HostError::Invalid("profile directory is not UTF-8".into()))?;
    if profile.is_empty() {
        return Err(HostError::Invalid("profile directory is empty".into()));
    }
    let mut argv: Vec<String> = ALLOWED_FLAGS.iter().map(|flag| flag.to_string()).collect();
    argv.push(format!("--user-data-dir={profile}"));
    argv.push(INITIAL_URL.into());
    assert_argv_clean(&argv)?;
    Ok(argv)
}

/// Fail closed on any forbidden flag or any user-data-dir outside the
/// validated profile construction path.
pub fn assert_argv_clean(argv: &[String]) -> Result<(), HostError> {
    for arg in argv {
        let flag = arg.split('=').next().unwrap_or(arg);
        if FORBIDDEN_FLAGS.contains(&flag) {
            return Err(HostError::Invalid(format!("forbidden engine flag: {flag}")));
        }
        if flag == "--user-data-dir" {
            continue;
        }
        if !ALLOWED_FLAGS.contains(&flag) && *arg != INITIAL_URL {
            return Err(HostError::Invalid(format!("unknown engine flag: {flag}")));
        }
    }
    if !argv.iter().any(|arg| arg.starts_with("--user-data-dir=")) {
        return Err(HostError::Invalid(
            "engine argv lacks the profile directory".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn argv_is_exact_and_clean() {
        let argv = build_argv(&PathBuf::from("/tmp/deskal-profile")).unwrap();
        assert_eq!(argv.first().unwrap(), "--headless");
        assert_eq!(argv.last().unwrap(), INITIAL_URL);
        assert!(argv
            .iter()
            .any(|arg| arg == "--user-data-dir=/tmp/deskal-profile"));
        assert_argv_clean(&argv).unwrap();
    }

    #[test]
    fn forbidden_flags_fail_closed() {
        for flag in FORBIDDEN_FLAGS {
            let argv = vec!["--headless".to_string(), flag.to_string()];
            assert!(assert_argv_clean(&argv).is_err(), "{flag}");
        }
        assert!(assert_argv_clean(&["--headless".to_string()]).is_err());
        assert!(assert_argv_clean(&["--headless".to_string(), "--evil".to_string()]).is_err());
    }
}
