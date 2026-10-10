# Opus to Sol change log

Status: ADOPTED planning direction (founder decision, 2026-10-09; Sol revision governs where it differs from the Opus baseline). Grants no authority: every capability still needs its own grain, review and qualification. Current execution state: `EXECUTION_FRONTIER.md`. Evidence baseline of this revision: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Decision changes

| Opus decision | Sol assessment | Recommended change | Evidence | Delivery effect |
|---|---|---|---|---|
| D1 sole qdrald authority | Strong, retain | Reuse vocabularies only; no peer donor policy daemon | Root SECURITY.md, current policy/kernel source | Preserves integration boundary |
| D2 fix ledger, rollback current | Repair incomplete; rollback unsafe | All-event durable append, one writer, strict bounded load, explicit recovery, valid-tail rollback/session policy | Independent27pass/3regressionfail; source ordering/loader | Blocks new privilege only, lets read-only UI proceed |
| D3 Winds replaces authorized donor | Extraction merits inspection, superiority unproved | Keep SG096 activation/donor commitment unless a scoped approved amendment wins on tests/effort | Existing program and inspected private lifecycle coupling | Avoids scope reset |
| D4 Rust CDP | Premature reimplementation | Wrap stable Playwright-core over private constrained host | Real source, installed channels/AX/frames, existing Node | Faster expected browser path; runtime cost measured later |
| D5 structured-first hierarchy | Retain, qualify | Direct APIs/UIA/browser, fresh generation/postconditions, no silent escalation | Existing typed targets; future paired ablations | Fewer turns is testable, not asserted |
| D6 Device Fabric LAN/QUIC/DAG | Too much before two targets | Existing outbound relay, B/C grants and bounded transfer first | Frozen relay contract, identity source, fleet patterns | Removes networking prerequisites |
| D7 Rust MCP evaluation | Not MVP-critical | Keep40-tool Node edge/OAuth; one runtime, separate workers | tool_contract.ts;13.8kTS and Node-inclusive budget | Avoids compatibility rewrite |
| D8 WPF/Tauri undecided | Choose Windows v1 | Keep WPF PR282, authenticated app API, measured reversal criteria | Independent22,016byte build/self-tests, marketing-only web | Starts real shell now |
| D9 cloud proposes | Retain, simplify | One bounded founder control service; local authority and tenant isolation | Kernel boundary; private bounded-loop inspection | Reduces deployment units |
| D10 Holo4 model/license | Candidate valid, serving unqualified | Pin card/processor/runtime; record direct permission separately; no third-party pilot | Pinned Holo4 card lacks claimed serving list | Server-only AI matches founder vision |
| D11 Reliance default | Benefit unproven | Custom system_one adapter only after optional ablations | Pinned d1 code-use contract and advertised warm latency | Avoids extra mandatory round trip |
| D12 meters/API/~300 | Contradicts scope, incorrect trigger | Defer prices/billing; cost telemetry/caps; founder-hosted qualification | Original arithmetic crossover1280 under estimates, not300 | Business model does not block engineering |
| D13 vendor instructions | Good principle, exploit language too broad | Keep tool results factual; distinguish remote flag activation from arbitrary text/RCE | DC source+installeddist; fixed worker program | Safe narrow behavior reuse |
| D14 naming clearance | Retain as product/legal decision, not full engineering blocker | Preserve compatibility identifiers; parallel legal review | Live repo already renamed, metadata drift | Avoids artificial serialization |
| Sentrdel arbitrary secret detectors | Source supports narrower feature | Known-secret filtering plus minimization/crops/explicit egress and broader tests | Actual private sink-guard/tests | Avoids false privacy assurance |
| Firecrawl source excluded | Founder supplies permission | Narrow extraction reuse; optional separate crawler, retain grant provenance | Founder message; freshsource897354b; rootAGPL/APIISC metadata | Adds useful donor without shipping fleet of services |
| Broad roadmap and late MCP tests | Working journeys arrive too late | F0–F4 local MVP, client/harness/size evidence early; all25 dispositions | Actual missing browser/app/task integrations | Proof of value earlier |
| Benchmark headline comparisons | Metrics/configurations not reconciled | Minimum journey set, paired samples, unknown/timeout accounting | Vendor docs/source, no head-to-head measurements | Honest evidence and smaller harness |

## Changes performed and publication model

Twelve requested local reports form the independent review/export package. Original topic documents are corrected in place on a separate planning branch, preserving Opus authorship and original history. The public proposal reuses those topic locations rather than adding twelve parallel architecture documents. Public independent judgment, critique and this change log provide review context. Private source provenance remains local and is excluded from public content.

Planning amendments grant no authority and do not modify production crates/apps, canonical state, security policy, permissions, dependencies/lockfiles, workflows, models or infrastructure. The dependency-free economics helper is planning-only and is revised to expose cost assumptions rather than subscription recommendations. At review time #278 remained open, #283 stayed draft, and no merge was authorized by this review; #278 has since been repaired by PR #286 and closed, and the founder adopted this direction.

Validation actually performed: fresh exact-head GitHub/CI/review-log reads; all original files read; public/private donor code/license inspection; pinned primary model/hosting/client docs; original economics run; native WPF build/router/layout self-tests; isolated approval regressions. REA analysis, visible GUI E2E, full installer/latency/RAM, model serving and multi-device journeys remain unqualified. Old-head CI never qualifies the amendment head.

Review gates retained: signed+DCO authorized identity, genuine Jev, genuine Alibaba OCR with manual exclusions, Graft when relevant, independent Windows E2E for implementation claims, exact-head CI, zero unresolved review threads before approved normal merge. This report is planning review, not automatic adoption.
