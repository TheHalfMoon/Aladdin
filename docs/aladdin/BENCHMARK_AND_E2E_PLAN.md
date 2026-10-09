# Benchmark and End-to-End Plan (Proposed)

Status: PROPOSED PLANNING ONLY. No benchmark has been run. All results sections are empty by design until measured.

## 1. Principles

1. Measure products, not models: the same planner model must drive each competitor where possible, so differences come from the execution layer.
2. Independent verification: task success is judged by a checker that inspects final machine state, not by any agent's self-report.
3. Same conditions: identical VM image, screen resolution, DPI, locale, network, permissions, and time budget.
4. Pin everything: benchmark revision, task list hash, model revisions, competitor versions, and harness commit.
5. Publish failures and unknowns alongside successes.

## 2. Suites

| Suite | Purpose | Notes |
|---|---|---|
| Aladdin Native Windows Suite (ANWS) | Primary product benchmark: Explorer, Notepad, Settings (non-protected pages), Office (where licensed), VS Code, Windows Terminal, Edge, Chrome, file and Git workflows | Built from SG-000095 disposable-window fixtures; locked test split; Arabic-UI subset |
| OSWorld-Verified (Windows subset where available) | External comparability | Near saturation for frontier models (trackers report about 85 to 86% versus about 72% human); useful for regressions, not differentiation |
| OSWorld 2.0 (XLANG Lab) | Long-horizon stress | 108 tasks, checkpoint-graded; authors report under 21% binary completion for all systems at 500 steps; pin the release and budget |
| Browser suite | Structured web extraction and forms | Owned test sites plus WebMCP-enabled fixtures; no live third-party accounts |
| Multi-device suite | Two to four Windows VMs | Placement, handoff, file transfer, disconnects, revocation |
| Adversarial suite | Prompt injection, fake approval overlays, protected surfaces, stale targets, PID reuse, DPI changes | Pass means zero unauthorized side effects |
| Voice suite | Arabic, English, code-switched commands | WER, intent accuracy, end-to-end task success, latency |

cua-bench (MIT) may be wrapped as a runner if its Windows support suffices; otherwise build a thin harness over the existing fixtures.

## 3. Metrics

| Metric | Definition |
|---|---|
| Verified success rate | Fraction of tasks whose checker passes; reported overall, by application type, and by task length bucket |
| First useful action latency | Time from task submission to first executed state-changing or information-returning action |
| Action latency p50/p95 | Proposal received on device to execution completed |
| End-to-end time p50/p95 | Submission to verified completion |
| Screenshots per task | Count of captures leaving the device |
| Hala One requests per task; Reliance decisions per task | Model call counts |
| GPU cost per verified task | Sum of model costs / verified successes |
| Wrong-target actions | Actions whose executed target differs from the intended target |
| Unnecessary actions | Actions not needed by the minimal reference trajectory |
| Recovery rate | Tasks that hit a failure and still complete |
| Human intervention frequency | Approvals plus takeovers per task (approvals reported separately; they are a feature, not a failure) |
| Remote connection time | Pairing-to-first-command and reconnect times |
| Multi-device overhead | Extra time versus the same tasks on one device |
| Install size and memory | Per `UI_UX_AND_APP_SIZE_BUDGET.md` |

## 4. Competitor configurations

| Product | Configuration |
|---|---|
| Aladdin | Safe and Full User profiles separately; planner = Hala One, and separately a BYO frontier model |
| Desktop Commander | Same BYO frontier model via MCP |
| Claude computer use | Reference loop with `computer_toolset_20260801` on the same VM |
| OpenAI computer use | Reference loop with the `computer` tool on the same VM |
| UFO2 | Default Windows configuration with the same frontier model where supported |
| TinyFish | Browser suite only, BYO account, same task inputs |

Vendor terms must permit benchmarking; check each provider's terms before publishing results.

## 5. What it takes to claim "faster" or "more accurate"

A claim "Aladdin is more accurate than X on Y" is allowed only if all hold:

1. Same suite revision, same task set, same environment image, same planner model (or both products' best documented configuration, disclosed).
2. At least 3 independent runs; report mean and 95% confidence interval; the difference is statistically significant (paired test across tasks).
3. Independent checker and raw logs published.
4. No task excluded after seeing results.

"Faster" requires the same conditions on end-to-end p50 and p95 time, with success held equal or better.

## 6. Native Windows E2E gates per release

- Full ANWS on Windows 11 (non-admin user), 100% and 150% DPI, two monitors with negative origin.
- Adversarial suite: zero unauthorized side effects.
- Kill switch: all leases revoked and held keys released within 1 second.
- No orphan processes after cancel, crash, or revoke.
- Size and memory budgets met.
