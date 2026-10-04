# Dependency and License Review

Status: SG-000072 v0.2 review of every third-party component in the Deskal Windows x64 release, recorded with the evidence used. This is an engineering review, not legal advice; it records declared licenses and shipped license texts and does not claim legal certainty. Every component version below was re-verified against the live `Cargo.lock` and `package-lock.json` on 2026-10-04; the shipped dependency closure is unchanged since the QDRAL-P13 review.

Scope: the crates linked into the shipped binaries (`qdral.exe`, `qdral-mcp-host.exe`, `qdrald.exe`) for `x86_64-pc-windows-msvc`, following normal (non-dev) dependency edges from `cargo metadata`, and the npm packages present in the packaged `app\qdral-mcp\node_modules`. Development-only dependencies (TypeScript, `@types/node`, test-only crates) are listed separately because they are not distributed.

Sources: `Cargo.lock` (exact versions and registry SHA-256 checksums), `package-lock.json` (exact versions and integrity hashes), each package's declared license, and the license files the package itself ships (collected into `THIRD_PARTY_NOTICES.txt` by `scripts/third-party-notices.mjs`).

## Deskal

| Component | License | Notes |
|---|---|---|
| Deskal (all workspace crates and `@qdral/mcp`) | Apache-2.0 | `LICENSE` at the repository root, shipped in the release payload. |

## Rust crates in the shipped binaries

All are from crates.io, pinned by `Cargo.lock` with registry checksums.

| Crate | Version | License | Purpose in Deskal |
|---|---|---|---|
| serde, serde_core, serde_derive | 1.0.229 | MIT OR Apache-2.0 | Typed JSON records, configuration, IPC envelopes |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | JSON parsing and serialization |
| itoa | 1.0.18 | MIT OR Apache-2.0 | Integer formatting (serde_json) |
| zmij | 1.0.23 | MIT | Floating-point formatting (serde_json) |
| memchr | 2.8.3 | Unlicense OR MIT | Byte search (serde_json) |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | SHA-256 for manifests, digests, approvals, audit chains |
| digest, block-buffer, crypto-common | 0.10.7 / 0.10.4 / 0.1.7 | MIT OR Apache-2.0 | RustCrypto traits used by sha2 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | CPU feature detection for sha2 |
| generic-array | 0.14.7 | MIT (declared by the crate; it ships an MIT `LICENSE`) | Fixed-size arrays for RustCrypto |
| typenum | 1.20.1 | MIT OR Apache-2.0 | Type-level numbers for generic-array |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | Conditional compilation helper |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | Raw Win32 bindings (ACLs, jobs, processes, registry, console, WinHTTP) |
| windows | 0.52.0, 0.58.0 | MIT OR Apache-2.0 | WinRT/Win32 bindings (Windows Hello presence, UI Automation, clipboard) |
| windows-core | 0.52.0, 0.58.0 | MIT OR Apache-2.0 | Core support for the windows bindings |
| windows-implement | 0.58.0 | MIT OR Apache-2.0 | Procedural macro for COM implementations (compile time) |
| windows-interface | 0.58.0 | MIT OR Apache-2.0 | Procedural macro for COM interfaces (compile time) |
| windows-result | 0.2.0 | MIT OR Apache-2.0 | Windows error and result types |
| windows-strings | 0.1.0 | MIT OR Apache-2.0 | Windows string types |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | Import-library selection for the Windows targets |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | Import libraries for x86_64-pc-windows-msvc |
| proc-macro2, quote, syn (2.x and 3.x), unicode-ident | 1.0.107 / 1.0.47 / 2.0.119, 3.0.6 / 1.0.26 | MIT OR Apache-2.0 (unicode-ident adds Unicode-3.0) | Compile-time procedural macros (serde_derive, windows-implement/interface); not linked into the binaries |

Observations:
- Two major versions of `windows` (0.52 and 0.58) are linked because different capability providers adopted different versions. This increases binary size but is not a security issue; consolidating them would touch closed provider grains and is recorded as a future, non-release improvement.
- `syn` 2.x and 3.x are both used at compile time only.
- No crate requires copyleft obligations; every license is permissive (MIT, Apache-2.0, Unlicense, Unicode-3.0).
- No unused runtime dependency was found in the shipped graph; each entry above is reached from code that is compiled into a shipped binary.

## npm packages in the release

Pinned by `package-lock.json` and copied into the release from the locked production dependency closure.

| Package | Version | License | Purpose |
|---|---|---|---|
| @modelcontextprotocol/server | 2.1.0 | MIT | MCP server (stdio transport, tool registration) |
| @modelcontextprotocol/core | 2.1.0 | MIT | Protocol types used by the server |
| zod | 4.2.0 | MIT | Tool input schemas |

## Development-only dependencies (not distributed)

| Package | Version | License | Purpose |
|---|---|---|---|
| typescript | 5.9.3 | Apache-2.0 | Compiles `apps/qdral-mcp` |
| @types/node, undici-types | 24.10.1 / 7.16.0 | MIT | Node.js type definitions |

## Security considerations

- Rust dependencies are resolved only from crates.io with lockfile checksums; npm dependencies are installed with `npm ci` from the lockfile with integrity hashes.
- No dependency performs network access on Deskal's behalf except the Windows OS transport (WinHTTP) used by the closed SG-000016/SG-000017/SG-000040 paths.
- No build script downloads code at build time (`npm` runs with `--ignore-scripts`).
- Known-vulnerability scanning: on 2026-10-04, `cargo audit` 0.22.2 against the RustSec advisory database (1,290 advisories, database commit `ef6173cb` dated 2026-10-03) reported 0 vulnerabilities for the 54 crates in `Cargo.lock` (exit 0, verified on native Windows against the SG-000072 tree), and `npm audit` reported 0 vulnerabilities for `package-lock.json` (production and development). Both audits run in CI on every change (the `Supply chain audit` job) and fail the build on any known vulnerability.

## Third-party notices

`THIRD_PARTY_NOTICES.txt` is generated from the actual release payload and shipped inside it, with each component's declared license and the full text of every license or notice file the component ships. The generator lists any component without a declared license or shipped license file; the current release has none.
