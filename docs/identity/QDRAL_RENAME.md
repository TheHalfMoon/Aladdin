# Qdral Identity Migration

Status: ACTIVE PRODUCT IDENTITY
Effective date: 2026-10-03

Qdral is the official product and project name formerly known as Quntal, and before that Cotra.

## Canonical naming

- Product: `Qdral`
- Root package: `qdral`
- MCP package: `@qdral/mcp`
- Relay package: `@qdral/relay`
- Rust crates: `qdral-*`
- Local daemon: `qdrald`
- CLI and install root: `qdral` / `%LOCALAPPDATA%\Qdral`
- Environment-variable prefix: `QDRAL_`
- OAuth scope prefix: `qdral.`
- Repository identity: `TheHalfMoon/Qdral`

## Historical evidence and compatibility

Closed SpecGrain records, canonical evidence, the canonical history ledger, merged PR/CI evidence, and the prior `QUNTAL_RENAME.md` record intentionally retain Quntal or Cotra where those names identify the project at the time the evidence was authored. Those historical names must not be rewritten as if old evidence had originally used Qdral.

Current project-owned code, packages, crates, executables, CLI surfaces, environment variables, OAuth/tool metadata, install paths, live documentation, and future planning use Qdral. A retained old-name string is permitted only when it is explicitly historical evidence or a deliberately documented compatibility boundary.

## Authority

This rename changes identity only. It does not grant new filesystem, process, network, browser, UI, secret, approval, trust, executable-admission, remote-session, or privileged authority. Existing fail-closed security boundaries remain authoritative.

## Repository migration

The intended canonical repository is `https://github.com/TheHalfMoon/Qdral`. Repository-name migration is an administrative identity operation and does not alter code authority. Historical GitHub URLs and exact evidence references may continue to resolve through GitHub redirects and remain valid as historical evidence.
