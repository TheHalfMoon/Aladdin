# Infrastructure and Unit Economics (Proposed)

Status: PROPOSED PLANNING ONLY. All numbers are ESTIMATES from `tools/unit_economics.py` (run with `python -I unit_economics.py`). No infrastructure was provisioned and nothing was purchased. Prices for GPUs and per-token APIs come from third-party listings that disagree with each other; confirm on official pages before any commitment.

## 1. Inputs

| Input | Value used | Source / status |
|---|---|---|
| Card processing | 2.9% + US$0.30 per charge | Typical card processor list price; merchant-of-record services cost more (about 5% + $0.50) |
| Hala One via per-token API (base Holo4-35B-A3B) | $0.30/M input, $0.03/M cached input, $2.00/M output | CLAIMED third-party listing dated 2026-10-05; confirm with H Company |
| Hala One step shape | ~1,500 uncached input tokens (screenshot ~1,300), ~2,500 cached prefix, ~150 output | ESTIMATE; measure |
| Resulting Hala step cost | low $0.00072, expected $0.00082, high $0.00200 | Model output |
| Reliance decision | $0.000005 to $0.0001 (expected $0.00002) | ESTIMATE from 8 ms per question on an RTX 4090-class GPU |
| Voice minute (STT + TTS) | $0.001 to $0.01 (expected $0.003) | ESTIMATE |
| Serverless H100 | about $3.95/hour (Modal, per-second billing); RunPod flex about $4.79/hour | CLAIMED third-party listings |
| Reserved H100-class | $2.20/hour | ESTIMATE |
| GPU-seconds per Hala step | 0.2 at good batching; 30% average utilization | ESTIMATE; the single most important number to measure |
| Control plane fixed cost | $60 (100 subs) to $9,000 (100k subs) per month | ESTIMATE |

## 2. Aladdin Core at US$5

Founder compute per Core user: zero by design. Inference comes from the user's own Claude/ChatGPT/Codex subscriptions or BYO keys; execution runs on the user's machines; updates ship via GitHub Releases; the relay is self-hostable and devices can connect directly on LAN.

Net revenue after card fees: $4.55 per monthly charge (fees take 9% at this price point; annual billing at $50/year reduces the fixed-fee share to about 3.5%).

Optional founder-run relay for convenience: bandwidth only (control frames are small; screenshots are not relayed for Core unless the user streams them to a remote MCP client). This is the only recurring Core cost and must have a hard per-account quota (already modeled by `apps/qdral-relay/src/quotas.ts`).

Verdict: Core at $5 is sustainable with zero mandatory founder compute.

## 3. Aladdin AI at US$12

Net revenue per charge: $11.35. Compute budget after ops: $11.20 per subscriber-month.

Variable AI cost per subscriber-month (model output):

| Profile | Hala steps | Reliance decisions | Voice minutes | Cost |
|---|---:|---:|---:|---:|
| low | 300 | 1,500 | 10 | $0.46 |
| expected | 1,500 | 6,000 | 45 | $1.64 |
| high | 6,000 | 20,000 | 240 | $16.55 (high prices) |

Scenarios using a per-token API (no idle GPU):

| Subscribers | Usage | Net revenue | Total cost | Monthly margin |
|---:|---|---:|---:|---:|
| 100 | low | $1,135 | $106 | $1,029 |
| 100 | expected | $1,135 | $224 | $911 |
| 100 | high | $1,135 | $1,715 | -$580 |
| 1,000 | low | $11,352 | $708 | $10,644 |
| 1,000 | expected | $11,352 | $1,892 | $9,460 |
| 1,000 | high | $11,352 | $16,800 | -$5,448 |
| 10,000 | low | $113,520 | $6,075 | $107,445 |
| 10,000 | expected | $113,520 | $17,925 | $95,595 |
| 10,000 | high | $113,520 | $167,000 | -$53,480 |
| 100,000 | low | $1,135,200 | $54,750 | $1,080,450 |
| 100,000 | expected | $1,135,200 | $173,250 | $961,950 |
| 100,000 | high | $1,135,200 | $1,664,000 | -$528,800 |

Self-hosted fine-tuned Hala One (a fine-tuned checkpoint generally cannot run on H Company's own per-token API; reserved $2.20/hour, 30% utilization, at least one warm GPU):

| Subscribers | GPUs | Hala GPU cost/month | Per subscriber | Margin after all costs |
|---:|---:|---:|---:|---:|
| 100 | 1 | $1,584 | $15.84 | -$549 |
| 1,000 | 1 | $1,584 | $1.58 | $9,113 |
| 10,000 | 4 | $6,336 | $0.63 | $101,634 |
| 100,000 | 39 | $61,776 | $0.62 | $1,023,924 |

Interpretation: self-hosting loses money below roughly 150 to 200 subscribers (one warm GPU must be paid for regardless of use) and wins clearly above about 1,000.

## 4. The "10,000 calls" allowance

| Interpretation of one "call" | Cost per call | Cost of 10,000 | Break-even calls per month at $12 |
|---|---:|---:|---:|
| 1. Local tool invocation | $0 | $0 | unbounded |
| 2. Remote device action via relay (~20 KB) | ~$0.0000004 | ~$0.004 | ~28 million |
| 3. Reliance decision | $0.00002 | $0.20 | ~560,000 |
| 4. Hala One visual inference | $0.000825 | $8.25 | ~13,600 |
| 5. Full AI workflow step | $0.000938 | $9.38 | ~11,900 |

Finding: 10,000 calls is sustainable at $12 for interpretations 1 to 3 with large headroom. For interpretations 4 and 5 it consumes 74% to 84% of the compute budget at expected prices, leaves almost no margin, and becomes loss-making at the high price assumption ($0.002 per step gives $20 per 10,000 steps).

## 5. Recommendation

1. Define the AI allowance in two meters, not one: "10,000 actions per month" (any tool, remote, or Reliance call) and "2,000 vision steps per month" (Hala One inferences). Expected variable cost stays under about $2.50 per subscriber; heavy users buy vision-step packs (for example 2,000 extra steps for $3) priced at a 50%+ gross margin.
2. Hard per-tenant ceilings: when the vision allowance ends, the assistant continues with structured-only methods or BYO provider, never silent overage.
3. Launch on a per-token API for the base model; move to self-hosted fine-tuned Hala One only after at least about 300 paying AI subscribers or a measured per-step GPU cost that beats the API at forecast utilization.
4. Measure before pricing publicly: GPU-seconds per step, cache hit rate, and average steps per verified task on the native suite. The model's biggest uncertainty is 0.2 GPU-seconds per step.
5. Consider $15 to $20 for an "AI Plus" tier with a larger vision allowance, if measured heavy-user share exceeds 10%.
6. Offer annual billing to cut fixed card fees.

## 6. Zero-founder-cost options and their limits

| Option | Effect | Limit |
|---|---|---|
| Scale-to-zero serverless GPUs | No idle cost | Cold starts of tens of seconds for a ~35B checkpoint; poor UX at low traffic |
| Per-token third-party API | No idle cost, no ops | Base model only unless a provider serves the fine-tuned adapter |
| LoRA adapter on a provider's shared base | Fine-tune without dedicated GPUs | Availability for Holo4-35B-A3B is UNKNOWN |
| Customer-owned infrastructure (BYO GPU or BYO provider key) | Zero founder cost | Only for advanced users; not the default AI experience |
| Quantization (FP8/INT4) for inference | Fewer GPUs per throughput | Accuracy must be re-validated per revision |

Aladdin AI cannot have zero founder compute: hosted Hala One, Reliance, and voice consume GPU time by definition. The plan keeps that cost metered, capped, and proportional to revenue.

## 7. Operational costs not in the model

Support staff, code-signing certificate (if chosen), legal (trademark, privacy), incident response, and taxes (VAT/GST collection, possibly via a merchant of record). These should be budgeted before launch.
