# Aladdin master plan — independent Sol revision

Status: independent planning review, 2026-10-09. PROPOSED, NOT ADOPTED. No implementation or authority change. Evidence baseline: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Judgment

Opus produced a useful research inventory and a defensible authority model. I would not adopt the package unchanged. Its delivery strategy optimizes hypothetical runtime size before proving working tasks, duplicates mature browser engineering, and contradicts the founder's server-only AI and deferred-pricing requirements. The security repair proposal is incomplete. Keep the kernel, simplify everything around it, and ship a Windows app that completes one bounded task before building fleet infrastructure.

These scores assess the proposed plan, not the quality or security of a shipped product. They are engineering judgments, not benchmark measurements.

| Category | /10 | Evidence and reason |
|---|---:|---|
| Research quality | 7 | Exact source pins and honest measurement labels are valuable; REA raw provenance is unavailable, donor conclusions sometimes outrun inspected code. |
| Architecture quality | 7 | One local authority and structured-first execution are strong; Rust CDP and an MCP rewrite add avoidable ownership. |
| Security | 6 | Correct local trust boundary; independently reproduced ledger defects, incomplete crash/concurrency/rollback repair, overconfident redaction. |
| Speed/performance strategy | 6 | Good focus on observations and postconditions; no task baseline, warm-worker or extra Reliance-call measurements. |
| Product UX strategy | 6 | Thin shell and visible approvals are sound; toolkit indecision and broad prerequisite graph postpone an actual workflow. |
| Reuse efficiency | 5 | Keeps existing kernel; rejects the most useful browser runtime on an incorrect incremental Node-size argument. |
| Multi-device design | 6 | Explicit target grants and unknown-outcome handling are strong; LAN, QUIC and DAGs precede the two-target proof. |
| Aladdin AI integration | 5 | Cloud proposes/device decides is right; API pilot contradicts founder hosting; d1 is a custom decision head. |
| Cost efficiency | 5 | Script arithmetic reproduces; 300-subscriber crossover is unsupported; price hypotheses become architectural dependencies. |
| Speed to a real MVP | 4 | 25 packets are not all on the MVP path, but shell/browser/task/parity/installer sequencing still delays narrow value. |
| Maintainability | 6 | Typed contracts help; new CDP maintenance, optional edge rewrite and overlapping plan registers increase drift. |
| Competitive potential | 7 | Local authority, native Windows and verified artifacts could differentiate; superiority has not been measured. |

## Verified frontier and evidence ledger

Fresh GitHub reads on 2026-10-09 confirmed public repository identity Aladdin, default branch main, and main head `61e664b3c39380a76aede29aa9c2d7fcbc449b08`. PR #283 remains open/draft at the baseline above, 15 changed files: 14 Markdown and the dependency-free economics script. PR #282 remains open/draft, four files. Issues #278/#279/#280/#281/#260 were open when inspected. The active canonical program remains SG-000096; this review does not activate a successor.

Both original PR heads had 11/11 completed successful checks. Actual #283 Jev log: 15 expected and 15 reviewed hunks, PASSED, no reported blocking findings. Actual Alibaba log: one reviewable file, 14 Markdown exclusions with unsupported_ext. Green delegation does not establish review of those documents. No submitted formal reviews or review threads were returned at initial inspection; an earlier independent issue comment existed. This report supplies manual architectural review, not founder adoption.

The live active [main ruleset](https://github.com/TheHalfMoon/Aladdin/rules/24456712) has ten required check contexts, no bypass actors, and strict status checks. It permits merge/squash/rebase and does not require thread resolution at API level; project governance is stricter and requires normal merge and resolution. Do not infer absent protection from the legacy branch-protection field or Opus's 404.

| ID / evidence class | Observation and primary evidence |
|---|---|
| E1 VERIFIED FACT | [PR283](https://github.com/TheHalfMoon/Aladdin/pull/283), [PR282](https://github.com/TheHalfMoon/Aladdin/pull/282), [main](https://github.com/TheHalfMoon/Aladdin/tree/61e664b3c39380a76aede29aa9c2d7fcbc449b08). GitHub reports PR283 signature valid; DCO footer present. |
| E2 VERIFIED FACT | [Browser provider](https://github.com/TheHalfMoon/Aladdin/blob/87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985/crates/qdral-provider-browser/src/lib.rs#L1730) returns a fixed template; host supervision is not live browser actuation. |
| E3 VERIFIED FACT | [MCP tool contract](https://github.com/TheHalfMoon/Aladdin/blob/87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985/apps/qdral-mcp/src/tool_contract.ts) has 40 tools, 26 core plus 14 local-only; OAuth, pairing and relay already exist. |
| E4 EXPERIMENTAL FINDING | Original approval library plus three added tests in an isolated copy: 27 existing passed, three regressions failed, exit 101. Replay after restart, approval C lost, two live brokers accept one token. See security report and retained test log. |
| E5 MEASURED RESULT | PR282 verification compiled a 22,016-byte WPF executable and passed router/layout self-tests. No visible GUI automation, installer, RAM, startup or user-journey qualification was performed. |
| E6 VERIFIED FACT | Twelve public donor source checkouts, licenses and exact HEADs independently read; inventory in reuse matrix. Main snapshots of Playwright are prerelease, not selected production dependency pins. |
| E7 SOURCE-DOCUMENTED CLAIM | Holo4/d1 model cards, Liquid license, Chrome debugging-profile guidance, RunPod billing docs; serving and vendor scores are not Aladdin measurements. |
| E8 MEASURED RESULT | Original unit_economics.py executes offline and reproduces tables. At its assumptions, reserved GPU cost is $1,584/month and API-equivalent Hala cost $1.2375/subscriber; crossover is 1,280 subscribers, not 300. This comparison is illustrative and not the permitted AI pilot. |
| E9 BLOCKED CAPABILITY | REA doctor invocation exits 1: compiled runtime missing. Opus's timings, OOM and analysis artifact remain secondary execution evidence; installed Desktop Commander 0.2.52 and source patterns were independently inspected. |
| E10 VERIFIED FACT / limitation | Authorized private donor code and tests were inspected. Private revision/file evidence is kept in the local reuse-matrix appendix and excluded from public publication. No private code imported. |

Use MEASURED RESULT only for an actual run; SOURCE-DOCUMENTED CLAIM for vendor statements; ENGINEERING INFERENCE for architecture choices; ESTIMATE for effort/budgets; UNPROVEN ASSUMPTION for throughput/size; OPEN DECISION for adoption/legal clearance. A source contract is not an implemented journey. Competitor task performance is NOT MEASURED.

## Direct answers A–O

**A — Is the plan good?** Yes as research and a proposal inventory; insufficient as the adopted execution plan.

**B — Adopt unchanged?** No. Correct ledger recovery, browser reuse, hosting, pricing and milestone sequence first.

**C — Five largest weaknesses:** unnecessary CDP ownership; incomplete approval persistence/recovery; contradictory AI-hosting and subscriber threshold; infrastructure-heavy two-device path; no working-task evidence behind performance/product conclusions.

**D — Five strongest decisions:** qdrald remains sole authority; structured methods before pixels; no cloud-granted privilege; explicit per-device grants; unknown-outcome reconciliation rather than automatic mutation retries.

**E — Remove immediately:** MVP MCP rewrite prototype; LAN/QUIC prerequisites; compulsory Reliance calls; subscription allowances/billing gates; rollback to a known-defective ledger. Remove categorical claims that competitors lack authority kernels or that registered-secret filtering finds every screenshot secret.

**F — Redesign:** browser as a constrained Playwright worker; task receipts as a minimal per-step record; two-target orchestration over the existing relay; founder-hosted AI as one control service plus workers; compact WPF shell with real broker events.

**G — Reuse:** keep authority/UIA/capture/input/identity/OAuth/MCP; wrap Playwright-core; adapt bounded file/search and lifecycle behavior; copy small tested parser parts only after provenance review; use fleet and agent frameworks as references. Do not replace the authorized SG096 donor merely on a language preference.

**H — Better browser approach?** Yes: Playwright-core behind a private typed host interface, using installed Edge/Chrome and a dedicated profile. Qualify stable release and policy enforcement. Rust CDP remains a later, measurement-triggered optimization.

**I — WPF or Tauri?** Keep WPF for Windows v1. Reverse only with a funded near-term cross-platform requirement and measured parity on the same functional screens, accessibility/RTL, IPC security, startup, memory, packaging and update tests.

**J — Rewrite MCP now?** No. Preserve the 40-tool contract, OAuth and pairing. Package one pinned Node runtime for edge and worker processes. Measure total installed bytes before reconsidering.

**K — Fastest safe app?** Start the read-only shell/API and harness immediately; repair #278 independently; deliver approved create-only report generation and real browser download; qualify one actual MCP client; finish terminal activation under SG096 rather than making every shell feature a browser dependency.

**L — Faster/more reliable than rivals?** Possibly through generation-bound structured observations, direct APIs, event-driven waits, minimal model calls and independent artifact verification. Prove paired task-time distributions, success and unauthorized effects with the same planner and workload. Present advantage is a hypothesis.

**M — Scale without bloat?** Yes in design: same thin app, paginated directory, lazy device subscriptions, bounded queues and per-target grants. Fleet load, revocation propagation and staffing remain unmeasured.

**N — Overengineered?** Governance is necessary and must be preserved. The execution dependency graph and proposed networking/serving decomposition are overbuilt for the initial product. Evidence collection can be automated; review cannot be bypassed.

**O — My company first?** A Windows app that finds a real document and creates a verified approved report, with truthful progress and stop. Refuse a browser-engine rewrite, edge rewrite, enterprise mesh, payment system, mandatory router model or broad unattended privileges before that proof.

## Adoption and next work

Adopt the revised direction as planning only after founder and normal governance review. Keep #283 draft and #278 open. No production source, permissions, CI, canonical state, models or infrastructure were changed by this review.

Next three executable packets: (1) #278 isolated security repair with durable one-writer append and full fault tests; (2) WPF read-only app API/timeline and truthful daemon status, correcting UI-blocking I/O; (3) narrow live-browser adapter behind existing contracts with real download and denial fixtures. Packet 2 can develop read-only functionality independently; packet 3's mutations wait for qualified ledger repair and authorized grain sequencing. See fast-track report for estimates and exact exit tests.

## Revision provenance

This proposed revision corrects the Opus planning package at the baseline above. Original research/decisions remain available at that immutable Git revision; Opus execution claims are attributed and are not relabeled as Sol measurements. OPUS_TO_SOL_CHANGELOG.md records decision changes. No canonical adoption or implementation is claimed.
