# Aladdin AI: founder hosting, optional decision model

Status: ADOPTED planning direction (founder decision, 2026-10-09; Sol revision governs where it differs from the Opus baseline). Grants no authority: every capability still needs its own grain, review and qualification. Current execution state: `EXECUTION_FRONTIER.md`. Evidence baseline of this revision: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Decision

All Aladdin AI model inference, conversations, planning, orchestration, routing and speech processing run on founder-controlled servers. The local app captures audio, displays responses and validates/executes proposals. Core remains usable through external supported MCP clients without founder inference. Remove the per-token third-party Holo4 pilot and BYO planner from the Aladdin AI default architecture; they belong only in separately described Core integrations. No training, weight download, quantization, inference run, deployment or infrastructure purchase occurred here.

Start with deterministic routing plus Hala One. Reliance is optional until ablations show improved verified task quality or cost without unacceptable latency. Do not require one inference by each model on every step.

## Primary model evidence

Repository HEADs read without weights; pinned README/license pages independently inspected on 2026-10-09:

| Candidate | Exact inspected revision | Evidence / serving implication |
|---|---|---|
| [Hcompany/Holo4-35B-A3B](https://huggingface.co/Hcompany/Holo4-35B-A3B/tree/508458727831368da85d53a8055c2b0f923ed4f3) | 508458727831368da85d53a8055c2b0f923ed4f3 | Card: Qwen3.6 MoE, BF16 safetensors, config context 262,144, Apache-2.0 weights/base. Metadata about 35.1B total. Current README does not list vLLM/SGLang support; qualify exact runtime, vision processor and function-call format. |
| [LiquidAI/d1-3B](https://huggingface.co/LiquidAI/d1-3B/tree/051bcc464b01b9f92942b364d9586b0ef5912432) | 051bcc464b01b9f92942b364d9586b0ef5912432 | Card: 3.12B, SigLIP2 vision, 32,768 context, custom code, transformers >=5.14, model.system_one returns typed decisions with zero generated tokens. Not a chat-completion model. |
| [LiquidAI/d1-omni-600M](https://huggingface.co/LiquidAI/d1-omni-600M/tree/02b55d7076f15129e59ab3f94783f32c4b088674) | 02b55d7076f15129e59ab3f94783f32c4b088674 | Card: 587M, 16,384 combined positions, images reduce state/question text to 896 tokens, <=30 s audio, English-speaker audio training, transformers >=5.15, fp16 recommended. |

These are SOURCE-DOCUMENTED CLAIMS. d1's advertised warm 8 ms on RTX4090 is not end-to-end routing latency; include prefill, image preprocessing, queue, network and cold loading. No omni latency was reported. Precision changes can alter decisions; do not apply BF16/FP8 indiscriminately.

A3B active parameters do not mean 3B resident weights. Rough BF16 weight floor for 35B is 70 GB decimal (about 65 GiB), FP8 about 35 GB, before KV cache, vision activations, allocator, batching and runtime overhead. These are ESTIMATES. An 80 GB GPU may fit constrained BF16 workloads; maximum context/concurrency is unproven. Separate released quantized checkpoints need exact licensing, configuration, runtime and quality qualification. Do not assume a 262K config makes that deployment feasible.

The founder reports direct permission to use and rebrand the models. No model is redistributed, rebranded or publicly deployed until the written grant is on file and verified. That report is not independently verified and is not rejected because the public license has conditions. Retain written grant covering legal entities, exact base/checkpoint/derivatives, hosted use, commercial redistribution, rebranding, notices, duration and any revenue-condition override. Public Liquid LFM v1 defines a $10m threshold and conditional commercial permission; its equality wording warrants review before approaching the threshold. Do not replace it with an unconditional Apache or precise boundary claim. Preserve attribution to H Company/Qwen and Liquid AI regardless of product branding.

## Minimal serving and proposal contract

Use one control service for authenticated tenant sessions, bounded orchestration, device selection, cancellation and usage telemetry, plus isolated model/speech workers. Scale components independently only when a measured need exists. The browser/device remains local. No eight-service control-plane rollout is needed for an initial private beta.

Pin weights/code/tokenizer/processor/prompt/serving configuration. Audit d1 custom code before enabling trust_remote_code; pin and bake it offline into a server image, not runtime main downloads. Provide a custom typed decision adapter around system_one/system_one_batch; an OpenAI-compatible text-generation endpoint is not sufficient. Define enum/noul/score schemas, calibration metadata, truncation, invalid-result/timeout -> abstain and bounded questions per state.

Proposals reference device-minted targets and observation generations, principal/session/device/epoch, method ceiling, expiry and operation id. Unknown actions/fields/refs are rejected locally. Multi-action outputs are evaluated separately; a model 'done' is not a receipt. Use the existing relay vocabulary or an explicitly reviewed versioned amendment, not the two undocumented new frame kinds in Opus's server document.

## Ablation qualification (future, not executed)

Run the same frozen Windows/browser task set, same Hala revision and budgets, paired seeds/order and independent final-state checkers. Compare:

| Variant | Question tested |
|---|---|
| Hala only | Baseline task quality and turn/cost/latency |
| Deterministic router + Hala | Does direct API/structured method selection avoid unnecessary calls? Preferred initial path. |
| Reliance router + Hala | Does route accuracy save more turns than its added request/queue cost? |
| Reliance only for uncertainty/abstention | Does selective use improve safety/recovery with fewer false blocks? |
| Cached structured decisions | Can exact unchanged state/question/model/config revision avoid inference? Never reuse approvals; validate generations and TTL. |

Record success with confidence intervals, complete-task p50/p95, added routing latency, model calls, GPU-seconds, false-block/false-allow, postcondition disagreement and failures under model outage. Batch multiple related decision questions in one forward pass where supported. Safety remains deterministic with Reliance disabled. Selective/advisory model verdicts cannot lower policy or suppress an approval.

## Hosting choice

[RunPod overview](https://docs.runpod.io/serverless/overview) and [pricing](https://docs.runpod.io/serverless/pricing) describe cold starts, queued/flex and load-balanced endpoints, warm/active workers and billable worker time. Container/model caching may reduce initialization; model loading in handlers and idle timeout can still incur billed running time. Use exact endpoint/GPU/region quotes at future qualification; Opus's hourly assumptions are not a verified offer.

| Deployment | Hala One | Reliance | Speech / user experience |
|---|---|---|---|
| Flex serverless, scale zero | Lowest idle spend, potentially painful large-model cold start/queue | Smaller load may be tolerable, still measure | Long-lived voice streams and barge-in need a supported endpoint, not assumed queue polling |
| One active warm serverless worker | Predictable interactive readiness, bills while idle | Separate worker only if ablation warrants | Warm CPU/GPU stream service can avoid voice cold start |
| Rented always-on GPU/pod/VM | More operational ownership, stable memory/cache and latency | Co-location may contend; benchmark isolation | Straightforward persistent connections; single-node outage must be handled |
| Reserved/cloud GPU or own hardware | Capacity commitment and utilization risk | CPU viability depends on latency tests | Regional availability, maintenance and security burden |
| Third-party per-token model API | Cost comparison only | Service availability/semantics vary | Does not meet the chosen founder-hosted Aladdin AI boundary |

For low-concurrency launch, keep a warm Hala worker only if its measured queue/cold-start tradeoff meets the experience and budget; otherwise explicit asynchronous waiting, truthful offline states and admission limits. At scale, bound queues, request deadlines, max workers, aggregate reserved spend, context/image sizes and per-tenant budgets; tune dynamic batching against p95 rather than only throughput. Region-to-user latency, storage throughput, model load, peak memory, batch 1/8/32, concurrent 1/4/16, cold/warm and cancellation must all be measured before selection. No unconditional H100/FP8 recommendation.

Failure: bounded retry only for clearly not-started inference; local dispatched mutations remain unknown until reconciled. Budget reservation precedes paid worker dispatch, uses a conservative ceiling and idempotent usage reconciliation, and must account for idle/storage/egress as well as active requests. Provider hard caps alone do not ensure per-tenant control.

## Voice and privacy

Push-to-talk first. Founder-side streaming STT, endpointing and TTS; Arabic/English/code-switching corpus, accents, noise, clipping and names. Voice expresses intent, never authorization. Barge-in stops playback and planning immediately; side-effect cancellation still uses local execution rules. d1-omni's Arabic text tags do not validate Arabic audio; use server-transcribed text until a direct-audio ablation passes.

Minimize device observations before egress, crop exact windows and suppress protected/password content. Registered-secret filters do not guarantee discovery of arbitrary screenshot secrets. Use per-tenant/session cache keys and no cross-tenant reuse; retained prompt/KV buffers must expire. Choose retention settings through explicit product/privacy decisions, not assume proposed 30/90-day retention is adopted. Default raw screen/audio persistence off, diagnostic content opt-in, training separate/off by default. Validate provider logging, encryption, deletion, regions and contractual processing terms.

## Economics correction

Original script arithmetic reproduces. At its illustrative assumptions: 1500 Hala steps × $0.000825 = $1.2375/month; $2.20/h ×720h = $1584/month; simple API-cost crossover = 1280 subscribers. Income break-even at a guessed subscription price is a different calculation. Its 0.2 amortized GPU-seconds/step and 30% utilization imply 3,888,000 steps/GPU-month, not demonstrated capacity or latency. Subscriber count alone is a poor scaling trigger.

Pricing, allowances, packs and checkout remain deferred. Replace billing milestones with cost per attempted and verified task, utilization, cold/warm/idle spend and caps. Core avoids mandatory founder inference cost; relay, support, signing and updates still cost money. Future training belongs to the founder's separate program and is not on Core's critical path.
