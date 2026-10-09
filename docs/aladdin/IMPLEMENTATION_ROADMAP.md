# Implementation Roadmap, Acceptance Criteria, Open Decisions and Risks (Proposed)

Status: PROPOSED PLANNING ONLY. Work packet ids (`WP-xx`) are proposal labels, not SpecGrain ids. Each packet that is adopted becomes one or more SpecGrains under existing governance: one bounded claim per PR (Diffcipline: about 12 files and 600 net lines by default), signed+DCO commits, exact-head 9/9 CI, genuine TypeSafe Jev, Alibaba Open Code Review with manual review of exclusions, zero unresolved threads, normal merge commits, post-merge CI, native Windows evidence for Windows claims. Nothing here changes SG-000096's authorization order.

## 1. Critical path to a usable Windows product (Aladdin Core MVP)

```text
WP-00 plan adoption
  -> WP-01 #278 ledger repair  ----------------------------+
        -> WP-04 SG-000096 shell sessions (T02..T05)        |
  -> WP-05 live browser engine (parallel only if governance |
           allows two grains; otherwise after WP-04)        |
  -> WP-06 task/receipt model + task tools  <---------------+
  -> WP-07 filesystem parity (SG-000097 subset)
  -> WP-08 app shell (client only; builds on #282; can start early)
  -> WP-10 installer bundle + size gate
  -> WP-11 native E2E + adversarial suites
  => Core MVP
```

Parallel tracks that do not touch host authority: WP-02 (trademark), WP-03 (identity docs after WP-02), WP-08 (UI shell against mocked `qdrald` events), WP-11 harness scaffolding, WP-18 (AI server skeleton without device actions), WP-22 voice evaluation offline.

## 2. Work packets

| WP | Title | Depends on | Authority delta | Deliverable | Acceptance test | Effort |
|---|---|---|---|---|---|---|
| WP-00 | Adopt this planning package (or a revised version) | Founder review | None | Signed+DCO planning PR merged; #279/#280 updated | Governance gates pass; canonical docs unchanged except an index link if requested | S |
| WP-01 | Approval ledger repair (#278) | WP-00 not required (security fix may proceed on its own grain) | None (repairs existing approvals) | Section 5.4 design of `SECURITY_THREAT_MODEL.md` | Section 5.5 matrix; the two reproduction tests become passing regression tests | M |
| WP-02 | Trademark and naming clearance | None | None | Counsel opinion for Aladdin, Hala One, Reliance in target markets | Written decision recorded in an issue | Legal |
| WP-03 | Identity-only rename (#281) | WP-02 | None | README, site, distribution metadata; compatibility identifiers preserved | `scripts/check-current-identity.mjs` updated and passing | S |
| WP-04 | Full User shell and interactive sessions | WP-01, SG-000096 T01 | Adds Full User shell (already planned in SG-000096) | Rust ConPTY sessions adapted from the founder's Winds engine; Job Object supervision; bounded paginated output | SG-000096 activation contract items 1 to 9 | L |
| WP-05 | Live browser engine | WP-00 | Makes existing browser contracts real (SG-000021 to SG-000026 shapes) | Rust CDP client over `--remote-debugging-pipe` (no TCP port) to installed Edge/Chrome; AX-tree snapshot with refs; replace the template snapshot | Existing browser tests plus live-page fixtures; origin, redirect, download/upload controls still enforced | L |
| WP-06 | Task and receipt model | WP-01 | None (observation of owned work) | `task_status`, `task_cancel`, `task_receipt`; signed receipts; P20 result states | Cancel during each provider; receipts verify | M |
| WP-07 | Filesystem parity subset | WP-06 | File authority within workspace unchanged; adds fuzzy edit, streaming search, rich-file reads | ADAPT Desktop Commander edit/search/PDF behaviors | Parity tests against DC behaviors; path identity rules unchanged | M |
| WP-08 | Aladdin app shell | WP-00; builds on draft PR #282 (WPF read-only preview) | None (client of `qdrald`) | Freeze a local app API in `qdrald`; then compact chat, timeline, stop, indicators, device list in WPF (#282) or Tauri 2 per D8 measurement | UI tests; app cannot approve; kill switch works | M |
| WP-09 | MCP edge runtime decision (D7) | WP-00 | None | Prototype Rust MCP edge versus bundled Node; measure size and compatibility | Same 40-tool contract tests pass on both; size measured | M |
| WP-10 | Unified installer and size gate | WP-04, WP-05, WP-08 | Packaging only | Per-component signed (or attested) binaries; delta updates; size CI | Budget in `UI_UX_AND_APP_SIZE_BUDGET.md` | M |
| WP-11 | Native E2E and adversarial suites | WP-05, WP-06 | None | ANWS v1, adversarial corpus, CI on Windows runners | Section 6 of `BENCHMARK_AND_E2E_PLAN.md` | L |
| WP-12 | Client compatibility qualification | WP-10 | None | Verified matrix for Claude Desktop/Code, Codex, Cursor, ChatGPT (plan-specific) | Recorded transcripts per client and version | S |
| WP-13 | Device directory and controller role | WP-06 | Adds observation-only controller grants | Account/device groups; capability manifests | Section 9 tests 1 and 2 of `MULTI_DEVICE_ARCHITECTURE.md` | M |
| WP-14 | E2E encrypted device channel; LAN direct | WP-13 | Network listener for enrolled peers only, off by default | Noise/TLS raw-key channel; relay as fallback | Malicious relay test; no unauthenticated listener | L |
| WP-15 | DeviceTask queue, DAG, file transfer | WP-14 | Remote task admission under device grants | Per-device queue; signed receipts; create-only transfers | Two-device DAG test; disconnect tests | L |
| WP-16 | Remote indicator and kill switch | WP-08, WP-13 | None (adds safety) | Border and tray indicator; global hotkey | Indicator before first action; revoke < 1 s | S |
| WP-17 | Remote Full Control mapping (SG-000105) | WP-01, WP-15, WP-16 | Adds remote actuation under locally issued lease | P20 Section 4.5 | P20 SG-000105 criteria | L |
| WP-18 | AI server skeleton | WP-00 | None on devices | Gateway, auth, tenancy, metering, hard caps | Load and isolation tests | M |
| WP-19 | Orchestrator with BYO provider | WP-18, WP-06 | Cloud proposals only | Kodac-derived bounded loop; proposal protocol | Forged/stale proposal tests fail closed | M |
| WP-20 | Hala One integration (base model, shadow) | WP-19 | None beyond WP-19 | Router to base Holo4-35B-A3B via per-token API; shadow logging | Schema validity; latency; cost per step measured | M |
| WP-21 | Reliance service | WP-19 | None (advisory) | d1-3B decision catalog | Calibration and false-allow metrics | M |
| WP-22 | Voice pipeline | WP-18 | None | Streaming STT/TTS with Arabic/English evaluation | Voice suite WER and latency | M |
| WP-23 | Billing and allowances | WP-18 | None | Two meters (actions, vision steps), caps, packs | Budget overrun impossible in tests | M |
| WP-24 | macOS/Linux hosts | Core v1 | New platform authority | Later program | Platform conformance suite | XL |

## 3. Milestones and exit criteria

| Milestone | Includes | Exit criteria |
|---|---|---|
| M0 Plan adopted | WP-00 | Founder review recorded; #278 still open |
| M1 Secure foundation | WP-01, WP-06 | #278 closed through governance; receipts live |
| M2 Core MVP (local) | WP-04, WP-05, WP-07, WP-08, WP-10, WP-11 | ANWS v1 verified success baseline published; adversarial suite zero unauthorized side effects; size budget met |
| M3 Core commercial v1 | WP-02, WP-03, WP-09, WP-12, signing decision | Trademark decision; verified client matrix; signed or attested installer |
| M4 Multi-device production | WP-13 to WP-17 | Multi-device suite passes; malicious relay test passes; kill switch < 1 s |
| M5 Aladdin AI beta | WP-18 to WP-23 | Cost per verified task measured; caps enforced; voice suite baseline |
| M6 Cross-platform | WP-24 | Later |

## 4. Acceptance criteria (cross-cutting)

1. No PR expands authority without an explicit authority-delta section and security review.
2. Every Windows claim has native Windows evidence on a non-admin user.
3. Every donor import has the P20 ledger checklist completed at the exact diff.
4. No model output is ever executed without device-side validation.
5. Every mutation reports one of the five P20 result states; no automatic retry after dispatch.
6. Size and memory budgets enforced in CI from WP-10 onward.
7. Performance claims follow `BENCHMARK_AND_E2E_PLAN.md` Section 5.

## 5. What not to build (explicit)

- A second authority (Kernux daemon, donor policy engines).
- A bundled browser, bundled Node (if D7 succeeds), or bundled model weights.
- A hosted backend with device authority.
- Remote feature flags that alter model-visible content.
- A screenshot-only agent loop.
- Whole-product imports (MeshCentral, RustDesk, Firecrawl SaaS, cua Spaces).
- Global input as a baseline method.

## 6. Open decisions (founder)

| # | Decision | Options | Recommendation |
|---|---|---|---|
| OD-1 | Product name | Keep "Aladdin" after clearance, or choose a new mark | Obtain counsel review first: "ALADDIN" is registered by BlackRock in classes 9 and 42 (Australia confirmed; US registrations exist, including one in class 42 for chatbot software whose owner was not confirmed). High conflict risk for software/AI. |
| OD-2 | "Reliance" model name | Keep or rename | "Reliance" is a famous mark (Reliance Industries; "Reliance Intelligence" is its AI unit). Use an internal codename publicly unless cleared. "Hala One": no exact conflict found in a basic search; still clear formally. |
| OD-3 | Aladdin AI allowance | Single "10,000 calls" or two meters | Two meters (Section 5 of economics) |
| OD-4 | Hala One serving at launch | Self-host fine-tune versus per-token base model | Per-token base model until about 300 AI subscribers |
| OD-5 | Liquid LFM license revenue cap | Accept with review trigger or negotiate now | Accept for MVP; set a review trigger well before US$10M revenue |
| OD-6 | Code signing | Paid certificate, MSIX/Store, or unsigned with attestation | Budget a certificate or MSIX before commercial launch |
| OD-7 | Governance parallelism | Strict one-active-grain versus proven-independent parallel grains | Allow parallel grains only for non-overlapping authority surfaces with a written independence proof |
| OD-8 | Node removal (D7) | Port MCP edge to Rust or bundle Node | Prototype first (WP-09) |
| OD-9 | TinyFish | Not used, or BYO-account adapter | BYO-account adapter after the local browser engine exists |
| OD-10 | Firecrawl import | Requires durable written permission | Do not import until evidence is attached |
| OD-11 | User-data training | Opt-in only | Opt-in only, separate storage, deletion support |

## 7. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Trademark conflict forces rename after launch | High | High | OD-1 before any public rebrand |
| Governance throughput (one grain at a time) delays MVP | High | Medium | OD-7; automate qualification evidence |
| Browser engine scope creep | Medium | High | Reuse existing contracts; CDP subset only |
| Hala One per-step cost or latency worse than estimated | Medium | High | Measure in WP-20 before pricing; two-meter allowance |
| OSWorld metric confusion leads to misleading claims | Medium | High | Benchmark rules in Section 5 of the benchmark plan |
| Unsigned binaries reduce adoption | High | Medium | OD-6 |
| Same-user malware drives the UI | Medium | High | Documented residual risk; STRONG for trust changes; HMAC store |
| Open Computer Use upstream is inactive (low maintenance signal) | Medium | Low | Aladdin owns the imported code |
| LFM license revenue cap | Low now | Medium later | OD-5 |
| ChatGPT write-action availability varies by plan | High | Medium | Document per plan; do not promise |
