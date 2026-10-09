# Aladdin AI — Server-Side Architecture (Proposed)

Status: PROPOSED PLANNING ONLY. Nothing here is deployed, provisioned, or billed. No model is trained, fine-tuned, merged, quantized, or deployed by this work.

## 1. Principle

All AI inference and AI orchestration for the Aladdin AI tier run on founder-operated servers. The installed app is a thin interface plus the local execution authority (`qdrald`). The cloud never holds OS authority on a customer device: it sends typed proposals over an authenticated channel; the device validates and executes only what local policy permits, with local human approval where required.

Aladdin Core never depends on these services.

## 2. Services

```text
App (chat / push-to-talk)         MCP clients (optional)
        |  TLS + device-bound session token
        v
+-----------------------------------------------------------+
| API gateway: authn, tenant routing, rate limits, budgets  |
+-----+---------------+----------------+--------------------+
      |               |                |
 Session         Speech gateway   Billing/metering
 coordinator     (WebRTC/WebSocket)  (usage events, hard caps)
      |               |
      v               v
 Task orchestrator <--- STT / TTS workers (GPU pool)
 (bounded agent loop: Kodac-derived)
      |  model router (cost/latency aware)
      +--> Hala One service (vision-language planner, GPU pool)
      +--> Reliance service (decision model, small GPU or CPU)
      +--> Optional BYO provider (user's own Claude/OpenAI key)
      |
      v  typed ComputerActionProposal / DeviceTask
 Device channel (existing relay, E2E to device)
      |
      v
 qdrald on the device: validate -> approve -> execute -> receipt
```

| Service | Responsibility | State | Scale-to-zero |
|---|---|---|---|
| API gateway | OAuth/session auth, tenant isolation, per-tenant rate limits, request budgets | Stateless | Yes (serverless functions) |
| Session coordinator | Conversation sessions, device selection, cancellation fan-out | Short-lived state in a managed KV | Partly |
| Task orchestrator | Bounded agent loop: plan, request observation, propose, await receipt, verify, recover | Durable task record | Yes |
| Model router | Chooses Hala One, Reliance, or BYO provider per step by policy, cost, and latency | Stateless | Yes |
| Hala One service | Screenshot plus context to typed action proposals | Stateless; KV cache per request | Only at low scale (cold start cost) |
| Reliance service | Typed decisions: risk flags, abstain, ranking, anomaly detection, routing | Stateless | Yes |
| Speech services | Streaming STT, TTS, VAD, barge-in | Per-stream state | Partly |
| Billing/metering | Usage events per tenant, hard ceilings, invoices via payment provider | Durable | Yes |
| Storage | Encrypted task records, opt-in memory, receipts copies | Durable | n/a |

## 3. Orchestration loop (per task)

1. User intent arrives (text or transcribed speech) with a target device id.
2. Orchestrator asks the device for a structured observation (window list, UIA tree, optional cropped capture). The device decides what may leave based on the egress grant; screenshots are cropped and redacted on device.
3. Router picks the cheapest adequate method: deterministic tool call if the intent maps to a structured tool (no model); Hala One for visual planning; Reliance for yes/no checks (is this target sensitive, should we abstain, did the postcondition hold).
4. The proposal is sent to the device as a typed `ComputerActionProposal` bound to the observation generation.
5. Device validates, requests local approval if needed, executes, verifies the postcondition locally, and returns a signed receipt.
6. Loop until done, budget exhausted, or user stop. Hard bounds: steps, wall time, tokens, cost, consecutive failures, repeated identical proposals.

Prediction versus observation: the orchestrator never treats a predicted UI state as observed. Every step after a mutation uses a fresh observation or a device-verified postcondition.

## 4. Protocol (cloud to device)

Reuse the existing relay frames (SG-000052) with two new frame payload kinds:

| Payload | Direction | Key fields |
|---|---|---|
| `observe.request` / `observe.result` | cloud -> device -> cloud | `observation_scope`, `max_bytes`, `egress_class`; result carries `generation`, redaction metadata |
| `proposal.submit` / `proposal.result` | cloud -> device -> cloud | `ComputerActionProposal` (see `MODEL_INTEGRATION_CONTRACTS.md`), `idempotency_key`; result is a P20 state plus receipt |

The device rejects proposals whose `observation_generation` is stale, whose method exceeds the granted ceiling, or whose target is unknown.

## 5. Tenancy, data, and privacy

- Tenant isolation: per-tenant encryption keys (envelope encryption), tenant id in every row and cache key, no cross-tenant prompt or KV caching, separate queues per tenant for fairness.
- Retention defaults: screenshots are not persisted (processed in memory); task transcripts 30 days; receipts 90 days; memory only when the user enables it, with export and delete.
- Training on user data: off by default; explicit opt-in per tenant; opted-in data is stored separately with provenance for the founder's fine-tuning program.
- Redaction: on device first (secure fields, Sentrdel-derived detectors); server-side second pass before any storage.
- Abuse defenses: rate limits per tenant and device, budget ceilings, anomaly detection (Reliance-assisted), refusal policies for disallowed task categories.

## 6. Voice (server-side)

| Stage | Design | Candidates (license must be verified at exact revision) |
|---|---|---|
| Capture | Push-to-talk in app; Opus over WebRTC or WebSocket; 16 kHz mono | n/a |
| VAD and endpointing | Server-side VAD with short hangover; barge-in cancels TTS immediately | Silero VAD (MIT) |
| STT (Arabic + English, code-switching) | Streaming partial transcripts | NVIDIA Nemotron streaming ASR family (reported Arabic support, cache-aware RNNT); Whisper large-v3-turbo (MIT) for accuracy fallback; Qwen3-ASR (Arabic listed) |
| Intent routing | d1-omni-600M accepts audio (up to 30 s) and can route voice commands directly; English-only audio training per model card, so Arabic routing must use transcripts until evaluated | LiquidAI d1-omni-600M (`lfm1.0`) |
| Response | Streaming text from orchestrator; short spoken confirmations | n/a |
| TTS (Arabic + English) | Streaming synthesis | SILMA TTS (bilingual Arabic/English, model card states commercial use permitted); Qwen3-TTS and CosyVoice2 (Apache-2.0; Arabic not confirmed) |

Rules: voice never authorizes an action. Consequential actions requested by voice still require the on-device approval dialog. Recording policy: audio is streamed and discarded after transcription unless the user opts in; speaker identity is not used for authorization. Each voice session meters audio minutes for cost accounting.

Latency target (to validate): first partial transcript under 300 ms after speech onset; first audio of response under 800 ms after end of speech for short confirmations.

## 7. External providers and independence

- Aladdin AI must work without Claude, ChatGPT, or any external model provider: Hala One plus Reliance are the default planners.
- BYO provider mode: the user may attach their own provider key for planning; cost is the user's; the same proposal contract applies.
- Optional hosted web agents (TinyFish) only as BYO-account adapters with explicit data-egress consent.

## 8. Deployment stages (no provisioning in this work)

| Stage | Hala One serving | Reason |
|---|---|---|
| Pilot (< ~300 subscribers) | Per-token API for the base Holo4-35B-A3B, or a LoRA-capable per-token provider if one supports the fine-tuned adapter | Avoids idle GPU cost; the self-hosted break-even is about 150 to 200 subscribers (see economics) |
| Growth | One reserved GPU (H100-class) running vLLM or SGLang with continuous batching, prefix caching, FP8 weights; overflow to serverless | Per-step cost falls below API cost at moderate utilization |
| Scale | Multiple GPUs, regional placement near users, autoscaling on queue depth | |

Exact GPU throughput per step for Holo4-35B-A3B is UNKNOWN and must be measured before committing (see `BENCHMARK_AND_E2E_PLAN.md`).
