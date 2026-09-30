use cotra_lifecycle::install::{InstallOptions, Installer, UninstallOptions};
use cotra_lifecycle::layout::Layout;
use cotra_lifecycle::manifest::MANIFEST_FILE;
use cotra_lifecycle::{host_platform, ErrorKind, LifecycleError};
use serde_json::json;
use std::path::PathBuf;

const HELP: &str = "\
Cotra - Computer Orchestration & Trusted Runtime Access

Usage:
  cotra install [--source <release-dir>] [--node <node.exe>] [--no-path] [--json]
      Install or repair the per-user Cotra install under %LOCALAPPDATA%\\Cotra.
      The release directory defaults to the directory containing this cotra.exe.
      Every payload file is verified against manifest.json before anything is copied.
      Administrator rights are not required and elevated prompts are refused.

  cotra uninstall [--purge-data --yes] [--json]
      Remove Cotra binaries, the version pointer, and the PATH entry.
      Configuration, secrets, logs, and audit/approval/trust history are kept
      unless --purge-data --yes is given. Workspaces are never touched.

  cotra version [--json]
      Show this CLI version and the installed and previous versions.

  cotra help
      Show this help.
";

struct Args {
    command: String,
    rest: Vec<String>,
}

impl Args {
    fn flag(&mut self, name: &str) -> bool {
        match self.rest.iter().position(|arg| arg == name) {
            Some(index) => {
                self.rest.remove(index);
                true
            }
            None => false,
        }
    }

    fn value(&mut self, name: &str) -> Result<Option<String>, LifecycleError> {
        match self.rest.iter().position(|arg| arg == name) {
            Some(index) if index + 1 < self.rest.len() => {
                let value = self.rest.remove(index + 1);
                self.rest.remove(index);
                Ok(Some(value))
            }
            Some(_) => Err(LifecycleError::usage(format!("{name} requires a value"))),
            None => Ok(None),
        }
    }

    fn finish(&self) -> Result<(), LifecycleError> {
        match self.rest.first() {
            Some(extra) => Err(LifecycleError::usage(format!(
                "unexpected argument {extra:?}; run `cotra help`"
            ))),
            None => Ok(()),
        }
    }
}

fn main() {
    let mut raw = std::env::args().skip(1);
    let command = raw.next().unwrap_or_else(|| "help".into());
    let mut args = Args {
        command,
        rest: raw.collect(),
    };
    let json_output = args.flag("--json");
    match run(&mut args) {
        Ok(output) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output.json).unwrap_or_default()
                );
            } else {
                print!("{}", output.human);
            }
        }
        Err(error) => {
            if json_output {
                println!(
                    "{}",
                    json!({"ok": false, "error": {"kind": error.kind, "message": error.message}})
                );
            } else {
                eprintln!("cotra: {}", error.message);
            }
            std::process::exit(error.kind.exit_code());
        }
    }
}

struct Output {
    human: String,
    json: serde_json::Value,
}

fn run(args: &mut Args) -> Result<Output, LifecycleError> {
    match args.command.as_str() {
        "help" | "--help" | "-h" => {
            args.finish()?;
            Ok(Output {
                human: HELP.into(),
                json: json!({"ok": true, "help": HELP}),
            })
        }
        "install" => install(args),
        "uninstall" => uninstall(args),
        "version" | "--version" => version(args),
        other => Err(LifecycleError::usage(format!(
            "unknown command {other:?}; run `cotra help`"
        ))),
    }
}

fn install(args: &mut Args) -> Result<Output, LifecycleError> {
    let source = match args.value("--source")? {
        Some(source) => PathBuf::from(source),
        None => default_release_dir()?,
    };
    let node = args.value("--node")?.map(PathBuf::from);
    let add_to_path = !args.flag("--no-path");
    args.finish()?;
    let platform = host_platform();
    let installer = Installer::new(Layout::for_current_user()?, platform.as_ref());
    let report = installer.install(&source, &InstallOptions { node, add_to_path })?;
    let mut human = format!(
        "Cotra {} {} at {}\n  lifecycle CLI: {}\n  Node.js: {}\n",
        report.version,
        if report.repaired_existing {
            "verified and repaired"
        } else {
            "installed and verified"
        },
        report.version_dir.display(),
        report.cli.display(),
        report.node_path.display(),
    );
    if report.path_entry_added {
        human.push_str(
            "  PATH: the Cotra bin directory is on your user PATH (open a new terminal)\n",
        );
    } else {
        human.push_str("  PATH: not modified\n");
    }
    human.push_str("  User data kept across updates and uninstall:\n");
    for path in &report.retained_data {
        human.push_str(&format!("    {}\n", path.display()));
    }
    Ok(Output {
        human,
        json: json!({"ok": true, "install": report}),
    })
}

fn default_release_dir() -> Result<PathBuf, LifecycleError> {
    let exe =
        std::env::current_exe().map_err(|error| LifecycleError::io("locate cotra.exe", error))?;
    let dir = exe
        .parent()
        .ok_or_else(|| LifecycleError::usage("pass --source <release-dir>"))?
        .to_path_buf();
    if dir.join(MANIFEST_FILE).is_file() {
        Ok(dir)
    } else {
        Err(LifecycleError::usage(
            "no manifest.json next to cotra.exe; pass --source <release-dir>",
        ))
    }
}

fn uninstall(args: &mut Args) -> Result<Output, LifecycleError> {
    let purge_data = args.flag("--purge-data");
    let confirmed = args.flag("--yes");
    args.finish()?;
    if purge_data && !confirmed {
        return Err(LifecycleError::usage(
            "--purge-data permanently deletes configuration, secrets, logs, and history; add --yes to confirm",
        ));
    }
    let platform = host_platform();
    let installer = Installer::new(Layout::for_current_user()?, platform.as_ref());
    let report = installer.uninstall(&UninstallOptions { purge_data })?;
    let mut human = String::from("Cotra uninstalled.\n");
    for (label, paths) in [
        ("Removed", &report.removed),
        ("Kept (user data)", &report.retained),
        ("Purged", &report.purged),
        ("Could not remove", &report.residual),
    ] {
        if !paths.is_empty() {
            human.push_str(&format!("  {label}:\n"));
            for path in paths {
                human.push_str(&format!("    {}\n", path.display()));
            }
        }
    }
    if report.path_entry_removed {
        human.push_str("  PATH: the Cotra bin directory was removed from your user PATH\n");
    }
    if let Some(relocated) = &report.relocated_cli {
        human.push_str(&format!(
            "  The running cotra.exe was moved to {} for OS temp cleanup\n",
            relocated.display()
        ));
    }
    if !report.residual.is_empty() {
        return Err(LifecycleError::new(
            ErrorKind::Io,
            format!("{human}Some items could not be removed; close programs using them and rerun"),
        ));
    }
    Ok(Output {
        human,
        json: json!({"ok": true, "uninstall": report}),
    })
}

fn version(args: &mut Args) -> Result<Output, LifecycleError> {
    args.finish()?;
    let cli = env!("CARGO_PKG_VERSION");
    let layout = Layout::for_current_user()?;
    let current: Option<cotra_lifecycle::layout::CurrentRecord> =
        cotra_lifecycle::layout::read_json(&layout.current_file())?;
    let record: Option<cotra_lifecycle::layout::InstallRecord> =
        cotra_lifecycle::layout::read_json(&layout.install_file())?;
    let installed = current.as_ref().map(|current| current.version.clone());
    let previous = record.and_then(|record| record.previous);
    let human = format!(
        "cotra CLI {cli}\ninstalled: {}\nprevious: {}\n",
        installed.as_deref().unwrap_or("not installed"),
        previous.as_deref().unwrap_or("none"),
    );
    Ok(Output {
        human,
        json: json!({"ok": true, "cli": cli, "installed": installed, "previous": previous}),
    })
}
