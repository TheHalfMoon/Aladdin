# Aladdin Planning Package (NOT ADOPTED)

Status: PROPOSED PLANNING ONLY
Date: 2026-10-09
Planning base: `main@61e664b3c39380a76aede29aa9c2d7fcbc449b08`
Parent planning issues: #279 (master plan), #280 (Opus + REA research), #281 (identity), security blocker #278, active program #260.

This directory is a research and planning proposal for the Aladdin product (formerly Deskal; earlier names are recorded in `docs/identity/`). It is a Diffcipline Section 14 planning-only change:

- it does not claim implementation;
- it does not advance any SpecGrain state or `docs/canonical/CURRENT.md`;
- it may describe future authority but grants none;
- it lists every unresolved architectural question in `IMPLEMENTATION_ROADMAP.md`.

Canonical truth remains `docs/canonical/*` and `.specgrain/*`. Where this package and canonical documents disagree, canonical documents win until a signed, governed adoption PR changes them.

## Evidence labels used throughout

| Label | Meaning |
|---|---|
| VERIFIED | Observed directly in source at an exact revision, or produced by a command actually executed during this research (command and output recorded). |
| DOCUMENTED | Stated by an owner's own documentation, not independently confirmed. |
| CLAIMED | Stated by a vendor or third party, possibly conflicting with other sources. |
| PLANNED | Future intent with no implementation. |
| EXPERIMENTAL | Implemented but not production-reachable or not qualified. |
| BLOCKED | Prevented by an open governance or security gate. |
| UNKNOWN | Not determinable with the access and tools available. |
| ESTIMATE | A modeled number with explicit assumptions; not a measurement. |

## Files

| File | Purpose |
|---|---|
| [ALADDIN_MASTER_PLAN.md](ALADDIN_MASTER_PLAN.md) | Executive plan, verified frontier, decision register, adversarial review, readiness conclusion |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Target architecture, current-code inventory, local host, MCP and client surfaces |
| [SOURCE_REUSE_MATRIX.md](SOURCE_REUSE_MATRIX.md) | Founder and external donor dispositions (COPY / ADAPT / WRAP / REFERENCE / REJECT) |
| [COMPETITOR_RESEARCH.md](COMPETITOR_RESEARCH.md) | Competitor capability matrix and findings, measured versus claimed |
| [REA_INVESTIGATION_REPORT.md](REA_INVESTIGATION_REPORT.md) | What REA was actually run on, results, failures, and coverage gaps |
| [MULTI_DEVICE_ARCHITECTURE.md](MULTI_DEVICE_ARCHITECTURE.md) | Aladdin Device Fabric: identity, connectivity, orchestration, security, reliability |
| [ALADDIN_AI_SERVER_ARCHITECTURE.md](ALADDIN_AI_SERVER_ARCHITECTURE.md) | Server-side Aladdin AI services, protocol, tenancy, and voice |
| [MODEL_INTEGRATION_CONTRACTS.md](MODEL_INTEGRATION_CONTRACTS.md) | Hala One and Reliance interface, data, evaluation, and rollout contracts (no training) |
| [UI_UX_AND_APP_SIZE_BUDGET.md](UI_UX_AND_APP_SIZE_BUDGET.md) | App design and byte-level size budget with measurement procedure |
| [SECURITY_THREAT_MODEL.md](SECURITY_THREAT_MODEL.md) | Whole-system threat model and the #278 repair proposal with reproduced regression |
| [BENCHMARK_AND_E2E_PLAN.md](BENCHMARK_AND_E2E_PLAN.md) | Benchmark methodology, metrics, and the conditions for any "faster/more accurate" claim |
| [INFRASTRUCTURE_AND_UNIT_ECONOMICS.md](INFRASTRUCTURE_AND_UNIT_ECONOMICS.md) | Cost model, scenarios, break-even analysis, pricing recommendation |
| [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md) | Work packets, critical path, acceptance criteria, open decisions and risks |
| [tools/unit_economics.py](tools/unit_economics.py) | Dependency-free, reproducible cost model used by the economics document |
