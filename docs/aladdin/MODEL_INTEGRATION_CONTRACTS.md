# Hala One and Reliance — Integration Contracts (Proposed)

Status: PROPOSED PLANNING ONLY. Neither model is deployed by this work. The founder owns fine-tuning, datasets, experiments, and checkpoints. This document specifies only interfaces, data specifications for later founder use, evaluation, serving targets, and rollout rules.

## 1. Common rules

1. Model output is advice. It never mints a target id, approval, lease, or receipt, and never lowers a policy-defined risk level.
2. Every request and response is schema-validated; unknown fields or actions fail closed.
3. Every response records `model_id`, `model_revision` (weights digest), `prompt_template_revision`, `serving_config_revision`, and latency.
4. A disabled or unavailable model degrades to structured-only operation; it never triggers a silent fallback to a stronger input method.

## 2. Hala One (vision-language planner)

### 2.1 Base model facts (DOCUMENTED on the model card, 2026-10-09)

| Item | Value |
|---|---|
| Candidate base | `Hcompany/Holo4-35B-A3B` |
| License | Apache-2.0 (weights). Verify the base `Qwen3.6-35B-A3B` license, processor/tokenizer files, and any custom code at the exact revision before commercial use. |
| Architecture | Qwen3.6 MoE; about 35B total parameters; "A3B" implies about 3B active (inferred from naming, not stated) |
| Context | 262,144 tokens (config) |
| Inputs | Image + text |
| Serving | UNKNOWN until exact runtime/vision processor/function-calling qualification; current pinned README does not list vLLM/SGLang support. |
| Other sibling | Holo4-27B is not selected under the founder's preferred35B candidate; restrictive public terms and any direct-grant coverage require separate qualification, not categorical rejection. |

### 2.2 Request schema (device-minimized observation)

```json
{
  "schema": "aladdin.hala.request.v1",
  "request_id": "uuid",
  "tenant_id": "opaque",
  "task": {"goal": "string", "step_index": 7, "max_steps": 60},
  "device": {"device_id": "dev-...", "os": "windows-11", "locale": "ar-SA"},
  "observation": {
    "generation": "obs-...",
    "window": {"window_ref": "win-...", "title": "redacted-or-text", "process": "notepad.exe"},
    "accessibility": [{"element_ref": "el-...", "role": "Button", "name": "Save", "bounds": [10, 20, 100, 30]}],
    "image": {"format": "png", "width": 1280, "height": 800, "coordinate_space": "window", "scale": 1.0, "redactions": [[10, 20, 100, 30]]},
    "truncation": {"elements_omitted": 0}
  },
  "history": [{"step": 6, "action": "click", "target": "el-...", "result": "completed", "postcondition": "passed"}],
  "allowed_actions": ["semantic_invoke", "set_value", "click", "type", "key", "scroll", "wait", "done", "ask_user"],
  "method_ceiling": "WINDOW_TARGETED_INPUT"
}
```

### 2.3 Response schema (`ComputerActionProposal`)

```json
{
  "schema": "aladdin.hala.proposal.v1",
  "request_id": "uuid",
  "observation_generation": "obs-...",
  "actions": [
    {"type": "semantic_invoke", "element_ref": "el-..."},
    {"type": "click", "point": [412, 230], "coordinate_space": "window", "element_ref_hint": "el-..."},
    {"type": "type", "text": "...", "secret": false}
  ],
  "expected_postcondition": {"kind": "element_exists", "role": "Dialog", "name": "Save As"},
  "status": "continue",
  "confidence": 0.0,
  "rationale_short": "<= 200 chars, never shown as authority"
}
```

Device-side normalization: maps to existing typed operations (`desktop_element_invoke`, `desktop_input_execute`, and so on), rejects coordinates outside the captured frame, rejects stale generations, and evaluates each action separately for consequence class and approval.

### 2.4 Proposed hypotheses, not adopted performance gates

| Metric | Target |
|---|---|
| p50 step latency (server, warm) | <= 1.2 s |
| p95 step latency | <= 3.0 s |
| Cold start (serverless) | Measure startup/loading/queue and warm-worker spend before choosing policy |
| Throughput | Measure GPU-seconds per step at batch sizes 1, 8, 32 |
| Cost per step | <= US$0.001 at expected utilization (see economics) |

Engineering levers: prefix caching for stable system and task context, image downscale to the smallest size that preserves accuracy (measure 1024, 1280, 1568 long edge), FP8 weights for inference only (validate accuracy delta), speculative decoding if supported, short structured outputs.

### 2.5 Optional data specification for the founder's separate program (not a Core dependency)

- Unit: `(observation, history, goal) -> proposal` plus outcome labels (`postcondition_passed`, `human_override`, `outcome_unknown`).
- Sources: opt-in user trajectories (redacted on device), synthetic Windows fixtures (control galleries, Office, Explorer, Settings, browsers), and public datasets with licenses that permit commercial training.
- Splits: by application and by task template, not by step, to prevent leakage; a locked test set of native Windows tasks never used for training; a separate Arabic-UI split (right-to-left layouts, Arabic labels).
- Negative data: sensitive targets that must yield `abstain` or `ask_user` (password fields, payment confirms, UAC, security settings).
- Training/serving skew risks: image resolution, coordinate spaces, accessibility tree formatting, history length; freeze these in a versioned `prompt_template_revision`.

### 2.6 Acceptance criteria for a Hala One release

1. Native Windows E2E suite success rate at least the incumbent's minus 0 points (no regression), measured on the locked set.
2. Zero proposals that target protected surfaces in the adversarial set (they must abstain).
3. Wrong-target rate below the incumbent.
4. p95 latency and cost per step within targets.
5. Schema-valid output in at least 99.9% of steps; invalid output is treated as abstain.

Rollout after independent qualification: shadow mode (minimized proposals logged under consent, not executed) on opted-in tenants, then canary 5%, then 50%, then 100%; instant rollback by router configuration to the previous revision.

## 3. Reliance (decision model)

### 3.1 Base model facts (DOCUMENTED)

| Item | d1-3B | d1-omni-600M |
|---|---|---|
| License | `lfm1.0` | `lfm1.0` |
| Size | 3.12B | 587M |
| Inputs | Text, JSON, images | Text, images, audio (<= 30 s, 16 kHz mono) |
| Context | 32,768 | 16,384 |
| Output | Typed answers (`noul` yes/no, `choice`, `score`) in one forward pass, zero output tokens | Same |
| Latency (card) | 8 ms warm on RTX 4090 per question | Not reported |
| Precision note | bf16 recommended | fp16 recommended; bf16 changed answers on 0.8% text and 1.7% audio rows |

The founder reports direct permission to use/rebrand the models; retain the written scope and distinguish it from public licensing. Public LFM v1 has conditional commercial rights and a US$10M threshold; review its exact entity/boundary wording and any direct-grant override before distribution. See ALADDIN_AI_SERVER_ARCHITECTURE.md for pinned source revisions and serving assessment.

A d1 service needs a pinned audited custom adapter around model.system_one/system_one_batch, not a generic chat-completion endpoint. Vendor warm latency excludes end-to-end network/queue/prefill qualification. Reliance is optional pending ablations; all inference is server-side.

### 3.2 Decision catalog (optional, after qualification)

| Decision id | Type | Inputs | Use |
|---|---|---|---|
| `target.sensitive` | noul | element role/name, window title, cropped image | Flag likely sensitive targets for stronger confirmation (cannot lower policy) |
| `step.abstain` | noul | proposal + observation summary | Recommend abstain or ask_user |
| `postcondition.passed` | noul | expected postcondition + new observation | Cheap verification assist; deterministic verifier remains authoritative |
| `method.rank` | choice | candidate methods for an intent | Prefer structured tools |
| `ui.anomaly` | score | observation diff | Detect unexpected dialogs or injection-like content |
| `route.intent` | choice | transcript or audio | Map voice or text to tool families |
| `cost.route` | choice | task features | Pick model tier for the next step |

Contract: `{"schema":"aladdin.reliance.request.v1","decision":"target.sensitive","state":{...},"images":[...]} -> {"answer":"yes","probability":0.93,"model_revision":"..."}`.

Hard rule: Reliance may only make an action stricter (add a confirmation, recommend abstain, raise risk). Any output that would make an action less strict is ignored by construction.

### 3.3 Evaluation

- Calibration: expected calibration error on a held-out native set; thresholds chosen for low false-allow (false "not sensitive").
- Measure false-allow and false-block per decision; publish both.
- Disabled-model test: system remains safe and functional with Reliance off.

## 4. Version migration and rollback

- Model revisions are immutable digests; the router maps logical names (`hala-one`, `reliance`) to revisions per tenant cohort.
- Prompt templates and serving configs are versioned with the model revision; mismatches fail closed at startup.
- Rollback is a router change with no client update.
