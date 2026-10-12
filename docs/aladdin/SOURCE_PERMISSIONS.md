# Source permissions

Status: **ADOPTED founder decision (2026-10-12)**. This record changes which donor source may be copied. It grants no runtime authority: every import still needs its own exact-head qualified change, with its provenance record, notices, tests and review gates.

## Founder attestation

On 2026-10-12 the founder stated: "i have official permission to copy and use the source codes from all sources. apply this on the project."

The founder therefore attests that official permission is held to **copy, use, adapt and rename source code** from every donor repository recorded in `SOURCE_REUSE_MATRIX.md`. This covers the copyleft donors that were previously reference-only for want of a written grant:

| Donor | Public license | Previous rule | Rule from 2026-10-12 |
|---|---|---|---|
| [Firecrawl](https://github.com/firecrawl/firecrawl/tree/f84845c864b6e2af22791c18fcf89ac2a21f1434) `f84845c` | AGPL-3.0 (engine) | Behavioral reference only; native rebuild (founder decision 2026-10-11) | Code may be copied and adapted under this permission. |
| [Skyvern-AI/skyvern](https://github.com/Skyvern-AI/skyvern/tree/f5f3f28a9ce0021a1db24286928a7bff6582a1eb) `f5f3f28` | AGPL-3.0 | Behavioral reference only | Code may be copied and adapted under this permission. |

For permissive donors (MIT, Apache-2.0, BSD) nothing changes. Their licenses already allow copying and renaming, provided the copyright and permission notices travel with the copied code.

## What the attestation does not change

- **Notices and provenance.** Every copied or adapted file keeps an attribution header (donor, path, commit). Every import keeps a provenance record with blob SHAs and a modification summary, plus the donor's notice under `docs/aladdin/notices/`. For the AGPL donors the notice records the upstream license **and** this permission.
- **Architecture and authority.** The SG-000074..SG-000108 invariants still apply to copied code, whatever its license. In particular, no code path may evaluate caller-, model- or page-supplied script, and no second channel into the browser is allowed. Donor code that breaks an invariant is not copied, however permissive its terms.
- **Dependencies.** A donor's third-party dependencies (crates, npm packages) are separate works. Each is admitted on its own license, with integrity, SBOM and supply-chain audit, as for playwright-core.
- **Models.** This record covers source code only. The written-grant requirement for redistributing or rebranding models (`ALADDIN_AI_SERVER_ARCHITECTURE.md`, `MODEL_INTEGRATION_CONTRACTS.md`) is unchanged.

## The permission documents

The permission documents themselves are **not in the repository**. Aladdin is distributed under Apache-2.0. AGPL-3.0 code may be distributed under other terms only if the copyright holders' permission allows it, so the documents should state that scope, including relicensing and distribution under Aladdin's license. File a copy of each document, or its reference, in `docs/aladdin/grants/` (one file per donor: grantor, date, scope, contact), and have counsel confirm the scope (`ALADDIN_MASTER_PLAN.md`, the open item "Written rights grants"). Until then this attestation is the record, and every AGPL import names it in its provenance.
