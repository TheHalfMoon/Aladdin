# 0001. Compact founder-hosted Hala for a bounded native Pro experiment

Status: proposed (2026-10-13). Research recommendation; not accepted runtime
architecture or spending authority. The founder's Basic $3/native Pro product
direction is the requirement for this study.

## Context

The adopted 2026-10-09 plan preserves `qdrald`, WPF, Node MCP and founder-only
AI inference but names Holo4 as Hala One, with pricing deferred. Current source
has reusable deterministic tools; it lacks the integrated native task loop and
complete live-browser product wiring. Large always-warm models create an idle
floor before end-to-end quality or useful paid demand is established.

## Proposed decision

Use one unchanged Qwen3.5-4B checkpoint as the first experimental Hala candidate
with a tool-first harness and scale-zero 24GB founder compute. Compare
Qwen3-VL-2B, Holo3.1-4B and Holo4-35B-A3B under the same authority/verifier.
Keep Brain/Eye conceptual roles inside one model, no initial LoRA or mandatory
Reliance/router model, and no launch-time large-model escalation. Basic remains
local without native inference; Pro adds Aladdin-owned text chat and founder
planning, no customer AI subscriptions/keys. Study $10–$15 bounded Pro pricing,
with $8 as a measured-cost challenger; adopt no billing/price now.

This proposes amending the Hala candidate choice in
[existing AI server architecture](../aladdin/ALADDIN_AI_SERVER_ARCHITECTURE.md)
and [model contracts](../aladdin/MODEL_INTEGRATION_CONTRACTS.md). It extends the
deferred-pricing research without implementing pricing. No earlier ADR/design
is superseded or rewritten; adoption requires the proper founder/governance
decision and subsequent capability-specific qualified grains.

## Consequences and reversal

One small model reduces residency, cold-loading and operations, and preserves
every local approval/target/session boundary. Narrow recipes can use zero/one
call but do not cover arbitrary GUI tasks; Arabic, structured outputs and
8K-context quality remain unproven. Caps limit heavy-user expense and also
limit useful work. Current economics are sensitivities, not measurements.

Reverse the model choice on paired verified-task/cost/language results as
specified in [model selection](../aladdin/AI_MODEL_SELECTION.md). Add selective
large-model escalation only if incremental successes justify all additional
work/storage/idle under a fixed budget. Train only after a diagnosed deficit.
Preserve WPF/Playwright/MCP decisions; do not widen remote/local authority to
make the proposed product appear integrated.
