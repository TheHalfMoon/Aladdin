# Hala model selection and Brain/Eye decision

Status: RESEARCH RECOMMENDATION / PROPOSED AMENDMENT. Inspected 2026-10-13.
No weights downloaded, inference executed, training started or serving qualified.
See [evidence](AI_EVIDENCE_REGISTER.md) and [decision record](../adr/0001-native-hala-small-model-research.md).

## Decision for the first experiment

Test **Qwen/Qwen3.5-4B** first, unchanged, as one multimodal Hala model.
Its compact dense architecture, Apache-2.0 license file, multilingual coverage
and documented tool parser make it a practical general planning/vision starting
point. This is an engineering choice, not demonstrated task superiority.
Hala Flash is an internal routing label, not a second customer application.

Use **Qwen/Qwen3-VL-2B-Instruct** as the lower-memory challenger; use
**Hcompany/Holo4-35B-A3B** as the stronger, domain-specialized comparison
baseline on separately authorized temporary compute. Also compare
**Hcompany/Holo-3.1-4B** early: it could beat the general Qwen model on GUI tasks
without large-model residency. Its card tags English; Arabic competence and
general tool routing remain unproven. **Holo-3.1-0.8B** is a later ultra-small
grounding challenger, not a promised bilingual planner.

Initial runtime: **vLLM 0.31.0**, release dated 2026-10-05, tag resolving to
`db9527a46873454610df6dbedf79a36d6bf1a7f6`, on founder-controlled
Linux NVIDIA compute. Before any experiment, bake a digest-pinned image including CUDA, torch,
Transformers, tokenizer, processor and prompt revisions. Current model cards
and runtime recipes support the architecture family; they do not qualify this
exact model/runtime/GPU/parser combination. Transformers single-request
inference is the reference fallback for diagnosis, not another production service.
SGLang is a challenger only if paired results show a material cost/latency benefit.

Start BF16 with 8,192 total context positions, one image per call, at most 256
generated tokens and explicit reasoning limits. This deliberately differs from
the Qwen card's recommendation for much longer thinking contexts; evaluate the
quality effect rather than inheriting a 128K/262K serving commitment. Disable
thinking through the supported chat-template mechanism for the first bounded
JSON task, then ablate capped thinking. Do not depend on truncated hidden reasoning
as an authorization or verification mechanism. Default escalation is **off**.

## Exact checkpoint inventory

Pins below are live Hugging Face API `sha` values, corroborated with raw README
and configuration files at those revisions. API file inventories and license
metadata have different evidential strength from a separate LICENSE file.
Parameter totals include vision/embeddings when API metadata provides them.

| Checkpoint | Revision | Public license evidence | Total weights; BF16 floor, decimal GB | Use and material limitation |
|---|---|---|---|---|
| Qwen/Qwen2-VL-2B-Instruct | `895c3a49bc3fa70a340399125c650a463535e71c` | Apache-2.0 file + card | 2.209B; 4.42 | Older bilingual/OCR baseline; no evidence of contemporary long-task reliability |
| Qwen/Qwen3-VL-2B-Instruct | `89644892e4d85e24eaac8bacfd4f463576704203` | Apache-2.0 metadata/card; standalone LICENSE absent in inspected inventory | 2.128B; 4.26 | Low-cost challenger; new GUI/tool capabilities need native evaluation |
| Qwen/Qwen3.5-4B | `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` | Apache-2.0 file + card | 4.660B; 9.32 | First general multimodal experiment; dense model, not an A3B MoE |
| showlab/ShowUI-2B | `cabec4fcc48d15ffd3efe0b33ea9bc7d41509d60` | MIT card + Apache base; separate weight-license artifact review needed | Total not returned; nominal 2B | Grounding/navigation specialist; normalized coordinates and Python-like examples need strict JSON adaptation; Arabic unqualified |
| microsoft/Fara-7B | `4fca9cdeeff278777751ec558257753b89cf078b` | MIT card, Apache Qwen2.5-VL base | 8.292B; 16.58 | Useful web-agent comparison; card explicitly English-only and advises against commercial/high-stakes deployment despite permissive license |
| Hcompany/Holo4-35B-A3B | `508458727831368da85d53a8055c2b0f923ed4f3` | Apache-2.0 file/card; base Qwen3.6 Apache claim | 35.107B; 70.21 | Strong specialized baseline; active 3B does not reduce resident weights to 3B |
| Hcompany/Holo-3.1-4B | `8c88265a5a159bfd1492db9243733dd2e6e04a6e` | Apache-2.0 card; base/license chain needs release audit | 5.175B; 10.35 | Compact GUI-specialized challenger; English tag and no Aladdin tool/Arabic qualification |
| Hcompany/Holo-3.1-0.8B | `72da4c53b351eb60e10a7022279019633c479292` | Apache-2.0 card; base/license chain needs release audit | 1.107B; 2.21 | Very small visual challenger; advertised size is not total residency |
| Qwen/Qwen3-VL-4B-Instruct | `ebb281ec70b05090aa6165b016eac8ec08e71b17` | Apache-2.0 card/metadata | 4.438B; 8.88 | Compatibility fallback if Qwen3.5 serving fails; no presumed quality ordering |

Weights-only floors exclude KV/recurrent state, visual activations, CUDA graphs,
allocator, scales and batching. Approximate 4B **resident** budgeting is 12–18 GiB
BF16 and 5–9 GiB with qualified 4-bit weights at short context/batch 1–4; these
are estimates, not measurements. Holo4 BF16 needs about 65.4 GiB for weights
alone; an 80GB GPU is a fitting hypothesis, not a maximum-context guarantee.
FP8 is roughly 35.1 GB before overhead, 4-bit roughly 17.6 GB before overhead;
released quantizations, hardware support and quality must be checked separately.
RTX 4090 supports neither every FP8 kernel nor Blackwell NVFP4 by implication.

## Evidence comparison without a false leaderboard

The Qwen3.5 card reports broad multilingual and agent evaluations, including
201-language coverage; this does not demonstrate Arabic Windows control.
Fara's publisher reports WebVoyager 73.5 and Online-M2W 34.1 averaged over
three runs. Holo4's card reports OSWorld 2.0 30.9% and $0.61/task for the
35B-A3B variant. The 85.2% OSWorld claim elsewhere in that card belongs to
**Holo4-27B**, not 35B-A3B, and uses a different benchmark. Those vendor task
costs are not founder-hosted Aladdin costs. ShowUI grounding results and
Holo3.1 aggregate charts do not supply directly comparable end-to-end data.

No cross-vendor score above is used in the executable cost model. No model
is called benchmark-leading here. Freeze identical tools, tasks, action budgets,
language strata and verifier rules in [evaluation](AI_DATA_AND_EVALUATION.md).
Use publisher results only to select candidates and identify failure hypotheses.

## Brain and Eye: compare four implementations

For one dense 4B model, let W be resident model memory, A adapter memory and
K per-request state. Inference counts refer to tasks that need intelligence;
deterministic task templates can use zero model calls under every design.

| Design | Memory and invocations | Quality/data implications | Operations and cost decision |
|---|---|---|---|
| A: one multitask VLM | W + K; one call can interpret goal and ground an element | Unchanged baseline; bilingual/planning/vision compete within one model | Selected. One image processor, queue, model identity and calibration surface |
| B: same base + Brain/Eye LoRAs | Shared W + 2A + K only if runtime genuinely shares base; separate workers can duplicate W. Sequential Brain→Eye often adds a call | Requires diagnosed specialist deficits, independent training data and adapter-interference tests | Deferred. Adapter switching is not free: load/scheduling, cache fragmentation and changed prompts require measurement |
| C: two specialized models | Wbrain + Weye + separate K; often two network/queue/model turns | Specialists may improve a hard domain; Arabic/generalization can regress | Reject at launch. Additional residency/idle cost and failure combinations need a proven task benefit |
| D: small eye + deterministic planning | Weye + K; zero calls for templates, a visual call only when structured targets fail | Good for known recipes; cannot understand arbitrary natural-language goals alone | Cheapest credible restricted fallback; label template-only task scope honestly |

Conceptual roles do not establish a training need. With existing native FS/UIA/DOM
tools, a single request-to-template decision can replace dozens of visual steps.
Train **nothing initially**. If evidence warrants adaptation, prioritize one
tool-selection/abstention/recovery LoRA on a known weak model before two adapters.
Use task-level verified completion and total billed GPU seconds, not only grounding
accuracy or decoder tokens, to evaluate the choice.

## Reversal and escalation gates

Replace Qwen3.5-4B with Qwen3-VL-2B only if the one-sided 95% paired
non-inferiority confidence bound on its success difference is no worse than
-3 percentage points in each launch language/domain, preserves all
safety denials and reduces cost per verified task by at least 20% with acceptable
p95 latency. Prefer Holo3.1-4B if its paired confidence interval shows a material
GUI/recovery benefit and Arabic/tool calling do not regress. These are proposed
decision thresholds; the founder has not approved a release on them.
An interval too wide to establish non-inferiority means insufficient evidence;
expand independent task/app samples rather than accepting a noisy point estimate.

Holo4-27B has stronger publisher scores but its current weight card/license is
**CC BY-NC 4.0** at `4de03cfeee1d930b616bfd4817570293f29ad8b9` (27.357B
API total; about 54.71GB BF16 weights). Its Apache base does not override that
noncommercial weight restriction. This makes 35B-A3B the practical permissive
strong baseline for this study. A verified founder grant covering commercial
27B use could reopen the baseline choice; no such scope is verified here.

Holo4 earns selective use only if the absolute verified-task gain on failed or
ambiguous eligible cases pays for its additional compute and does not require
an always-warm large-model floor. Compare `C_small / S_small` with
`(C_small + p_escalate * C_large) / S_routed`, including both attempts, startup,
storage and idle. Never let repeated escalations remove budget/authority ceilings.
One failed parse gets one bounded repair attempt; a second fails to ask-user.
Unknown dispatched mutations are reconciled locally, not retried by a bigger model.
