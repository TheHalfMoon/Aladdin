# Minimum evidence for a useful, fast Aladdin

Status: ADOPTED planning direction (founder decision, 2026-10-09; Sol revision governs where it differs from the Opus baseline). Grants no authority: every capability still needs its own grain, review and qualification. Current execution state: `EXECUTION_FRONTIER.md`. Evidence baseline of this revision: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Priority and current limits

No complete Aladdin user journey, head-to-head competitor task, toolkit startup/RAM, full installer or GPU throughput benchmark was run here. Actual measurements are the approval tests, WPF build/self-tests and offline arithmetic. The first benchmark priority is a small native Windows acceptance harness with independent final-state checkers for the five journeys in the `IMPLEMENTATION_ROADMAP.md`. Broad leaderboard programs follow working functionality.

Keep security outcomes beside task time: faster unauthorized completion is failure. Report attempted tasks, verified success, unknowns, blocked capabilities and interventions. Do not exclude timeout/failure tasks after seeing results; successful-only latency can reward unreliable systems.

## Minimum qualification ladder

| Stage | Required evidence | Gate |
|---|---|---|
| Security repair | Existing 27 + three regression tests, all-event persistence/restart/concurrency/migration fault matrix | Zero unauthorized dispatch in tested cases; fail closed on ambiguous state |
| Local file/report | Known input/output fixtures, denied and approved save, content/path identity drift | Independent bytes/value/hash checker; no out-of-scope writes |
| Browser | Navigation/search/PDF plus iframe/shadow/stale ref/crash/egress denial fixtures | Correct artifact and bounded network/file behavior |
| Windows app/client | Actual visible native session, one real supported MCP client, stop/takeover/unknown handling | Non-admin, keyboard/Narrator, no fabricated state or self-approval |
| Three-machine remote | A controls B/C with distinct grants, encrypted permitted transfer and drop/revoke tests | Hash-verified copy, no cross-target authority |
| AI/voice | Five routing ablations, cold/warm/concurrency/runtime-memory and bilingual speech corpus | Server-only AI, tenant isolation, local consent and artifact checker |
| Release | Clean install, signature/update/recovery, complete size/process-tree measurements | Installed app/dependencies <400 MB, documented cache and prerequisites |

Native Windows coverage: pinned Windows11 build, non-admin account, installed Edge/Chrome versions, exact app/adapter revisions, locale, 100/150/200% DPI, second monitor with negative origin, font/text scaling, physical input interruption and protected-surface negatives. Initial compact tasks need fewer applications than a full ANWS suite; add coverage as capabilities are activated.

## Metrics and sampling

Instrument monotonic timestamps from intent through observation/proposal/approval/dispatch/postcondition. Report complete-task wall time p50/p95 including timeout/unknown rates, and separately system execution time versus human consent dwell. Also first useful result, wrong target, model calls, screenshot bytes/count, DOM/AX bytes/truncation, retries, recovery, cancel/lease/input-release time, process cleanup, task cost and aggregate idle/peak working set/private bytes.

Cost per verified task = total spend on all attempted tasks including failures divided by verified successes; separately report warm idle, load/start, storage and egress. No successes means undefined/infinite, not zero. GPU-seconds must state whether amortized batch cost or request elapsed time. Voice: onset-to-partial, end-of-speech-to-first-response/audio and time-to-verified-task, WER/intent error/code-switch accuracy and barge-in cancellation. Startup: cold/warm launch-to-interactive and daemon/worker readiness separately.

Use paired repeated runs across tasks, randomize execution order, pin model/template/tool environment and log censored timeouts. Pre-register task list and checker; report confidence intervals and enough samples for a meaningful p95. Three runs alone are not a significance guarantee. Zero observed unauthorized effects in N trials is evidence for those fixtures, not universal proof. Predefine failure handling and statistical comparison before leaderboard claims.

## What makes tasks faster

| Lever | Qualification experiment and safety condition |
|---|---|
| Tool selection/direct APIs | Deterministic router vs Hala-only; measure saved calls and routing failures; policy always rechecked |
| Browser locators/AX | Same tasks structured vs screenshots, bounded subtrees and explicit omitted counts; stale refs denied |
| UIA/native actions | Same target via semantic vs authorized input; compare wrong-target/postcondition failures |
| Model calls | Five AI ablations; selective Reliance and multi-question batches, no safety dependency |
| Screenshots/crops | Resolution/crop sweep with locked accuracy checker; exclude protected/secret content before egress |
| DOM observations | Incremental revision-bound diff vs full snapshot; prove missing-information recovery |
| Process sessions | Reuse owned authorized session, bounded paginated output; no stale lease/session inheritance |
| Waiting | Event/state-driven waits vs sleep/poll; bounded deadlines and trusted postconditions |
| Safe batching | Batch reads; mutation transactions only when explicitly authorized, checked and interruptible |
| Transfers | Bounded chunking/hash-resume, local commit reconciliation; do not retry unknown writes |
| Recovery/verification | Independent checker and targeted re-observation; model prediction never substituted for observation |
| Voice | Push-to-talk/streaming vs full clips; no voice consent and cancellation independent of cloud |
| Startup/cold workers | Lazy optional modules and warm/cold worker tests; measured spend vs readiness |


Competitor assessment and primary links are in COMPETITOR_RESEARCH.md. No superiority claim is qualified.
