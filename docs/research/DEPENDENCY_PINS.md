# Qdral Dependency Pins

Snapshot: 2026-09-23

Historical record of the dependencies introduced by SG-000001. The current, release-wide dependency and license review is `docs/research/DEPENDENCY_LICENSE_REVIEW.md`; exact versions are pinned by `Cargo.lock` and `package-lock.json`.

This file records direct implementation dependencies introduced by SG-000001.

## Model Context Protocol TypeScript server

Package: @modelcontextprotocol/server@2.1.0

Upstream repository: modelcontextprotocol/typescript-sdk

Upstream snapshot reviewed:
7f7a94c22017e121a960e071bb50ec75e34450bd

License: MIT for the server package.

Use:
Qdral depends on the official server package rather than copying the MCP protocol implementation.

## Zod

Package: zod@4.2.0

Use:
MCP input schema validation at the unprivileged TypeScript edge.

## TypeScript

Package: typescript@5.9.3

Use:
Build/typecheck only.

## Node type definitions

Package: @types/node@24.10.1

Use:
Build/typecheck only.

## Rust dependencies

SG-000001 direct Rust dependencies are limited to serde and serde_json.

No Windows API crate is required in SG-000001. The small Windows final-path calls are declared directly against documented kernel32 APIs to keep the trusted dependency surface narrow.

## Lockfile status

SG-000001 begins before lockfiles exist because this repository was bootstrapped empty. Exact-head CI is required before merge. Release hardening requires committed lockfiles and dependency audit evidence; absence of lockfiles must not be represented as release-ready reproducibility.
