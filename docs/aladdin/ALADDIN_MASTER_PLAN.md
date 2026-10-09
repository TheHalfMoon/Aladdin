# Aladdin Master Plan (Proposed, NOT ADOPTED)

Status: PROPOSED PLANNING ONLY. Research date 2026-10-09. Author: Claude (Opus 5.5) research session for the founder. Grants no authority, imports no code, closes no issue.

## 1. Executive summary

Aladdin already has the hardest part that competitors lack: a governed, typed, per-action authority kernel with native Windows UI Automation, exact-window capture, bounded real input under a local Full User lease, approvals with Windows Hello, device identity, OAuth, and a self-hostable relay. Its largest product gaps are not safety but capability and usability: no live browser engine (the browser layer is contract-only), no governed interactive shell yet, no desktop app, no multi-device orchestration, and an unsigned release that still depends on a separately installed Node.js.

The plan therefore keeps the existing architecture and fills gaps in this order: repair the reproduced approval-ledger defect (#278); finish shell sessions with a same-language Rust ConPTY engine from the founder's Winds project; make the browser contracts real with a small Rust CDP client against the user's installed Edge/Chrome; add a task/receipt model; ship a compact thin-client app (building on the WPF preview in draft PR #282, or Tauri if cross-platform is prioritized); then multi-device; then the server-side Aladdin AI tier, priced with two usage meters so heavy vision use cannot sink the margin.

Two founder assumptions should change: (1) "Aladdin" and "Reliance" carry serious trademark risk and need counsel review before any public rebrand; (2) "10,000 calls for $12" is only sustainable if "calls" are not Hala One vision inferences; define two meters instead.

## 2. Verified GitHub frontier (2026-10-09)

| Item | Value | Evidence |
|---|---|---|
| Repository | `TheHalfMoon/Aladdin` (renamed from Deskal), public, Apache-2.0, default branch `main` | `gh repo view` |
| `main` head | `61e664b3c39380a76aede29aa9c2d7fcbc449b08` (merge of PR #277, 2026-10-09 04:13 +0300) | fresh clone |
| CI on head | Run `37868730423` success | `gh run list` |
| Open PRs | None at research start; draft #282 (read-only WPF desktop preview) opened at 05:02Z during research | `gh pr list --state open` |
| Remote branches | 289 | `git branch -r` |
| Releases | One: "Cotra 0.1.0" (`v0.1.0`), zip 4,839,256 bytes | `gh release view` |
| Open issues in scope | #278 (security), #279, #280, #281 (planning, not adopted), #260 (SG-000096 active) | `gh issue view` |
| Active grain | SG-000096 shell and process sessions (T02 sub-slices merging; production shell unarmed) | `.specgrain/specs/SG-000096.json`, `docs/canonical/CURRENT.md` |
| #278 | Reproduced deterministically at ledger level (see `SECURITY_THREAT_MODEL.md` Section 5) | local test run |
| Branch protection API | 404 for the research account (READ permission); repository rules unknown to this session | `gh api .../protection` |

## 3. Research and REA execution summary

- Aladdin source read across all crates and apps; inventory in `ARCHITECTURE.md` Section 1.
- Founder repositories inspected: kernux, Ascout, Kodac, Winds, Sentrdel, Orcel, Morize, commandF, SpecGrain, Diffcipline (revisions in `SOURCE_REUSE_MATRIX.md`). Private-repository details are summarized at capability level; file-level evidence was delivered privately.
- External sources cloned and read at exact revisions: Desktop Commander, Open Computer Use, AgentQL, REA, browser-use, Stagehand, Playwright MCP, Chrome DevTools MCP, MeshAgent, UFO, cua, Windows-MCP, Agent-S, WebMCP.
- REA 6.1.0 installed from the exact registry tarball (integrity verified), diagnostics run, and used on Desktop Commander's installed package: one run failed (out of memory on 17,510 files), one scoped run succeeded and produced confirmed findings. Native decompilation is blocked (no Ghidra/Hopper/IDA). See `REA_INVESTIGATION_REPORT.md`.
- Claude and OpenAI computer-use internals were studied only through public documentation; their local apps were not reverse engineered because their terms prohibit it.
- Not accessible: the founder's Desktop Commander fork and any private TinyFish source (searched four authenticated GitHub accounts and the local disk).
- Measurements actually taken: release binary sizes for `qdrald` and the browser host, Node runtime size, REA timings, #278 reproduction. Not measured: Go host size, any task latency or success rate, any GPU throughput.

## 4. Final recommended architecture

See `ARCHITECTURE.md` Section 2. In one line: untrusted intent (app, MCP clients, Aladdin AI cloud) -> edge -> `qdrald` (sole authority) -> execution hosts (Computer Host, Browser Engine, Shell Host, providers, Device Fabric) -> verified receipts.

## 5. Decision register

Each decision lists rationale, source, reuse option, alternatives, trade-offs, dependencies, owner, acceptance test, rollback, and uncertainty.

| ID | Decision | Rationale and source | Reuse | Alternatives and trade-offs | Depends on | Owner | Acceptance test | Rollback | Uncertainty |
|---|---|---|---|---|---|---|---|---|---|
| D1 | Keep `qdrald` as the only authority; adopt Kernux vocabularies, not a Kernux daemon | Two authorities create "least restrictive wins" bugs; #279 already forbids a second policy owner | ADAPT Kernux method ceilings, consequence and egress classes | Run Kernux as peer daemon: more processes, duplicated policy | None | Founder + security reviewer | Policy unit tests map every Kernux class to one `qdral-policy` rule | Remove vocabulary mapping | Low |
| D2 | Fix #278 before any approval-holding feature | Reproduced replay and record loss after restart | Repair design in threat model | Defer: unacceptable for SG-000096 interactive sessions and remote approvals | None | Security | Section 5.5 matrix | Revert to current ledger (known-defective) | Low |
| D3 | Shell sessions via Rust ConPTY adapted from Winds; Desktop Commander as behavioral reference | Same language as `qdral-provider-process`; Winds has Windows-native lifecycle tests | ADAPT (private founder source) | Port DC TypeScript: adds Node runtime coupling and a language boundary | D2, SG-000096 T01 | Process owner | SG-000096 contract items 1 to 9 | Keep Safe-only execution | Medium (Winds engine maturity outside Winds) |
| D4 | Live browser engine: small Rust CDP client over a debugging pipe to the user's installed Edge/Chrome; keep existing SG-000021 to SG-000026 contracts | The current snapshot is a template; Playwright-based Node sidecar would add ~93 MB Node runtime | REFERENCE Playwright AX snapshot and refs, browser-use DOM serializer, Stagehand extension (for signed-in mode later) | Playwright sidecar: fastest to build, large; Stagehand extension: great for signed-in browsers, powerful `debugger` permission | D1 | Browser owner | Live fixtures pass with all existing origin/redirect/download/upload controls | Disable live engine (contract-only state) | Medium (CDP surface area) |
| D5 | Structured-first execution hierarchy L1 to L7, no silent escalation | Fewer screenshots, fewer model calls, fewer wrong targets | Existing UIA registry | Screenshot-first: simpler, slower, costlier | D4 | Architecture | ANWS: screenshots per task and wrong-target rate | n/a | Low |
| D6 | Device Fabric on existing identity/relay with E2E device channel, LAN direct, per-device grants, visible indicator | Reuses SG-000052 to SG-000057 contracts; avoids AGPL and SYSTEM-agent designs | REFERENCE MeshAgent indicators, UFO3 DAG | MeshCentral import: large, different security model | D2, WP-06 | Remote owner | `MULTI_DEVICE_ARCHITECTURE.md` Section 9 | Disable controller role | Medium (NAT traversal) |
| D7 | Evaluate removing Node from the device by porting the MCP edge to Rust | Node is 92.8 MB of a ~100 MB install; the rest is ~10 MB | Official Rust MCP SDK (verify license and maturity) | Bundle Node: simpler, larger | WP-09 prototype | Edge owner | 40-tool contract tests pass on Rust edge | Keep Node edge | Medium (edge has 13.8k TS lines including OAuth) |
| D8 | Thin app client over a frozen local app API; approvals rendered by `qdrald`. Toolkit: WPF on in-box .NET Framework (draft PR #282) for a Windows-only MVP, or Tauri 2 for a cross-platform path, chosen by measurement | Size is a hard constraint; WPF ships no runtime; Tauri shares Rust and reaches macOS/Linux | PR #282 preview; REFERENCE Winds desktop (Tauri) | Electron (large); WinUI (extra runtime) | WP-08 | UX owner | App cannot approve; size and memory budget; Arabic RTL review | Ship CLI-only | Medium (toolkit choice is reversible only if the app API is frozen first) |
| D9 | Aladdin AI: cloud proposes, device decides; Kodac-derived bounded orchestrator | Keeps OS authority local | ADAPT Kodac model layer and loop | Cloud-side execution: violates safety model | D1, WP-06 | AI owner | Forged/stale proposals fail closed | Disable AI tier | Low |
| D10 | Hala One base = Holo4-35B-A3B (Apache-2.0); reject Holo4-27B (CC BY-NC) | License | Model card | 27B: better claimed scores, non-commercial | License verification of base and processor | Founder | License audit record | Use BYO provider | Medium (claimed scores conflict with OSWorld 2.0 authors) |
| D11 | Reliance = d1 family, strictly advisory (can only make actions stricter) | Calibrated typed answers, low latency | Model card | Use Hala One for decisions: costlier | LFM revenue-cap acceptance | Founder | False-allow metrics; disabled-model test | Disable Reliance | Medium (calibration on Windows data unknown) |
| D12 | Two-meter AI allowance; per-token API at launch; self-host after ~300 subscribers | Economics model | `tools/unit_economics.py` | Single 10,000-call meter: loss-making for heavy vision users | WP-20 measurements | Founder | Measured cost per verified task under cap | Adjust allowance | High (GPU-seconds per step unmeasured) |
| D13 | Never inject vendor-controlled content into model-visible results; no remote feature flags affecting tools | Verified Desktop Commander pattern | n/a | n/a | None | Security | Code review checklist | n/a | Low |
| D14 | Name clearance before public rebrand | Registered ALADDIN marks in classes 9 and 42 | n/a | Rebrand anyway: legal risk | WP-02 | Founder + counsel | Written clearance | Keep Deskal or choose a new mark | High |

## 6. Strongest source-reuse strategy (summary)

Keep what Aladdin owns (authority, UIA, capture, input, approvals, identity, relay). Adapt from the founder's own code where it is same-language and tested (Winds ConPTY, Kodac model loop, Kernux vocabularies and secret handles, Sentrdel detectors). Adapt narrowly from MIT/Apache donors (Desktop Commander edit/search/PDF behaviors; UI-TARS action parser). Reference, never import, large or restrictively licensed systems (Playwright, Stagehand, UFO3, MeshAgent, RustDesk, cua Spaces). Details in `SOURCE_REUSE_MATRIX.md`.

## 7. Adversarial review of this plan

| # | Question | Answer |
|---|---|---|
| 1 | Duplicating existing Aladdin functionality? | The browser host and UIA registry appear to carry parallel coordinate/capture contracts; the plan assigns one owner before adding browser actuation. No other duplication proposed. |
| 2 | Unnecessary donor import? | Porting Desktop Commander's terminal manager was planned in SG-000096; the Winds Rust engine is a better fit. Firecrawl remains gated. Playwright is not imported. |
| 3 | Simpler proven alternative? | For the browser, a Playwright sidecar is simpler to build but costs ~93 MB and a second runtime; D4/D7 trade build effort for size. This is the plan's main bet and should be reconsidered if WP-09 shows the Rust edge is expensive. |
| 4 | Two authorization systems? | No. Kernux contributes vocabulary only; Reliance is advisory; the cloud only proposes. |
| 5 | Hosted AI too much authority? | No authority: proposals are validated on device against local policy and approvals. Residual risk: a persuasive model can still get a user to click "Approve"; mitigated by exact-action dialogs and consequence classes. |
| 6 | Lateral movement? | Per-device grants, no inheritance, controller compromise test (MD Section 9). Residual risk: a user who grants unattended Full User on many devices to one controller accepts that blast radius; default is observation-only. |
| 7 | Scales without uncontrolled cost? | Yes with two meters and hard caps; no with a single 10,000-vision-call promise. |
| 8 | 16 GB Windows thin client? | Yes: no local model, no bundled browser; budget idle under 150 MB. Needs measurement. |
| 9 | Core independent of the founder's AI? | Yes by design: BYO clients and providers; local execution. |
| 10 | Largest latency source? | Model round trips per step (seconds), not input injection (milliseconds). Structured routing and batching attack it. |
| 11 | Largest failure source? | Stale or wrong targets and unexpected UI states (dialogs, focus changes), plus web pages without stable structure. Generation binding and postconditions address the first; the browser engine and Hala One address the rest. |
| 12 | Highest security risk? | Approval integrity (#278) today; tomorrow, remote actuation plus prompt injection steering users to approve. |
| 13 | MCP limits per client? | ChatGPT: write actions plan-dependent (beta for Business/Enterprise/Edu per OpenAI help center; Plus/Pro disputed), no mobile, local servers need the tunnel. Others: untested in this session. |
| 14 | Simplest first product? | Core MVP: local Windows, files + Git + governed shell + live browser + UIA desktop, thin app client (D8), works with Claude/Codex/Cursor via stdio. No cloud. |
| 15 | Better than Desktop Commander and TinyFish how? | Enforced per-action authority instead of guardrails; native UIA semantics; local structured browser extraction without per-step fees; multi-device with device auth; receipts. |
| 16 | Measurable differentiators? | Verified success on ANWS, wrong-target rate, screenshots per task, cost per verified task, adversarial pass rate, kill-switch latency, install size. |
| 17 | Evidence that would disprove the architecture? | If ANWS shows structured-first routing does not reduce model calls or wrong targets versus a screenshot-only loop with the same model; if per-action approvals make tasks unusable (approval count per task too high); if the Rust CDP engine fails on common sites that Playwright handles. |
| 18 | Postpone? | Full Admin, Persistent Admin, macOS/Linux, unattended remote, TinyFish adapter, Firecrawl crawl, WebMCP execution (keep discovery). |
| 19 | Without rebuilding third-party products wholesale? | Yes; every row in the reuse matrix is a bounded file set or a pattern. |
| 20 | Every milestone has tests and dependencies? | Yes in `IMPLEMENTATION_ROADMAP.md` Sections 2 to 4; quantitative thresholds for success rates remain to be set after the first baseline (cannot be set honestly before measurement). |

## 8. Conclusion

NOT READY FOR ADOPTION AS CANONICAL; READY FOR FOUNDER REVIEW AS A PROPOSAL.

Reasons it is ready for review: the frontier is verified; the main code facts, the #278 defect, donor licenses, model licenses, and economics drivers are evidenced; decisions have owners, tests, and rollbacks.

Reasons it is not ready for adoption: trademark clearance is unresolved (D14); no product benchmark exists yet, so success and latency targets are hypotheses; GPU cost per step is unmeasured; the Go host size and client compatibility are unverified; governance review (Jev, Alibaba OCR, exact-head CI) has not been run on this planning PR; and #278 must remain open until an independently qualified repair merges.
