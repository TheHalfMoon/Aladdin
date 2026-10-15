# Founder hosting: minimum cash spend and capacity qualification

Status: RESEARCH SCENARIOS, not a deployment. Public prices checked 2026-10-13;
availability, merchant account, region, taxes, DPA and exact GPU are unqualified.
See [model pins](AI_MODEL_SELECTION.md), [economics](AI_UNIT_ECONOMICS.md),
[sources](AI_EVIDENCE_REGISTER.md) and [generated scenarios](research/AI_COST_SCENARIOS.md).

## Current offers and suitability

| Provider / mode | Public price observation | Hardware suitability and total-cost caveat |
|---|---|---|
| RunPod flex serverless | 16GB class $0.58/h; L4/A5000/3090/MIG 24GB class $0.69/h; 4090 24GB $1.10/h; A100 80GB $2.72/h | Candidate scale-to-zero 4B BF16 on 24GB. Small class mixes different GPUs; benchmark a pinned eligible type, not the price-class label. Load/execution/idle timeout are billed |
| RunPod dedicated pods | L4 24GB $0.59/h; A5000 24GB $0.27/h; 4090 24GB $0.89/h; A100 80GB $1.79/h | A5000 is a cheap public warm offer to test, but may be slower; L4 is the conservative warm comparator. $0.27/h ×730 = $197.10 before storage, no demonstrated task capacity |
| Vast.ai instances / serverless | Prices are marketplace/offer-specific; dynamic cards supplied no numeric offer in the public page read | Serverless bills ready/loading compute plus storage/bandwidth; inactive workers still incur storage/bandwidth. Obtain dated verified-host offers including `dph_total`, disk, bandwidth, region and reliability before comparison. No invented "$0.20 4090" quote |
| Hugging Face Endpoints | AWS T4 14GB $0.50/h; AWS L4 24GB $0.80/h; GCP L4 $0.70/h; AWS A100 80GB $2.50/h | Supports custom serving and autoscaling; initialization/running billed per minute. Scale-to-zero cold behavior and exact Qwen image/parser compatibility need tests; a T4's nominal fit is not a latency guarantee |
| Hugging Face Jobs / Spaces | Jobs are temporary batch compute; Spaces/ZeroGPU are separately metered services | Useful controlled evaluation/training after authorization; not a promise of free private production GPU service. Recheck eligibility, quotas, data terms and exact hardware rates for the chosen job |
| Modal own model Functions | L4 $0.000222/s = $0.7992/h; T4 $0.000164/s; CPU $0.0000131/core/s, memory $0.00000222/GiB/s; egress $0.04/GiB | Founder model serving possible, scale-to-zero. Requested 2 cores +16GiB adds about $0.2222/h while billed. Default 60s keepalive is billed; tune it. Starter $30 credits are conditional, not a sustainable price reduction |
| Founder-owned consumer GPU | No public hosting tariff | Capex/depreciation, electricity, cooling, redundancy, residential network and staff remain costs. No customer GPU required. Quantify `capex/life_months + kWh*local_tariff + support + network + spare capacity`; do not assume free hardware |
| Founder CPU serving | Instance price can be lower; exact multimodal latency unknown | Reference/very-low-volume asynchronous experiments only until measured; no claim that a cheap CPU achieves useful interactive 4B vision throughput |

RunPod's 2026-10-03 billing page now directs active-worker discounts to sales;
do not reuse an older fixed discount table. Model-cache image/download phases
may occur before billed worker running time, while handler model loading is
billed. Measure billing fields, not only client time. Shared model storage is
$0.07/GB-month below 1TB; 20GB is $1.40/month, and container disk is separately
about $0.10/GB-month. The cost model reserves additional disk/backup expense.

Choose **RunPod 24GB flex** for an initial deployment experiment, contingent on
privacy/runtime/cost qualification. It offers a public low tariff and explicit
scale-zero billing. Modal is the credible alternative with transparent egress
and CPU/memory pricing. Vast may be cheaper but cannot win on an unobserved
quote. A ready A5000 pod could beat flex for steady use; measure equal workload
cost per success, including lower throughput, before making it always-on.

## Scenarios A–D

These compare the illustrative capped mix: 40% light, 45% regular, 13% heavy,
2% power, with at most 7,200 modeled billed small-GPU seconds/account/month.
The cap reduces delivered tasks, particularly heavy/power workloads. It is a
backend budgeting experiment, not an adopted customer allowance.

| Scenario | GPU / residency | GPU/month estimate | Other monthly operating cost | Operating total / verified-task cost estimate | Availability/concurrency decision |
|---|---|---:|---:|---|---|
| A: minimum cash | 24GB flex, compact 4B BF16, 12–18GiB estimated resident | Workload-dependent; zero active GPU idle after scale-zero, still storage/control costs | $18.40 fixed cash + variable state/transfer/support assumptions | At 10 users below: $37.39, $0.0720 including support | Async queue; no warm SLA; one active request/worker until measured |
| B: 10 | Same; model cache persistent | $7.82 | $29.57 | $37.39; $0.0720 | Founder labor subsidy still required; maximum-worker/spend limits separate from subscriber count |
| B: 50 | Same; bounded shared pool | $39.12 | $74.25 | $113.37; $0.0437 | Measure arrivals/queue before warming a worker |
| B: 100 | Same | $78.24 | $130.09 | $208.34; $0.0401 | Price/cap quality test; do not automatically start a warm GPU at 100 subscribers |
| C: 500 | Same pool, separately authorized worker ceiling | $391.22 | $576.87 | $968.09; $0.0373 | Microbatch independent calls if p95 permits; add warm floor only on observed traffic/SLO need |
| C: 1,000 | Same | $782.45 | $1,135.34 | $1,917.79; $0.0369 | Shared queues, tenant fairness and reserved budget; not 1,000 simultaneous tasks |
| D: burst/high concurrency | 24GB workers; batch 1/2/4 first | Unknown until workload/VRAM measurements | More control/observability/redundancy may exceed $18.40 floor | Unknown | Admission + queue age limit + cancellation; add workers on measured service-time demand, never unbounded autoscaling |

Operating totals include support at $20/hour and exclude payment deductions,
tax and $300/month maintenance. The uncapped 1,000-user mix instead needs
$6,422/month of flex GPU alone; this is why "unlimited" is rejected.
At 10 users, a single warm L4 pod already costs $430.70/month GPU alone.
Warm capacity is justified by latency requirements and utilization, not by a
model name or nominal subscriber threshold.

## Required latency/capacity fields, explicitly unmeasured

For **all** A–D scenarios: startup time, warm request latency, whole-task
p95/p99, throughput, peak resident VRAM and sustainable concurrent requests
are **UNKNOWN / NOT MEASURED**. The financial base uses 30s billed load with
25% cold probability, 3/6/8/12 busy seconds per small-model call by cohort,
5s billed idle per call and a stress test with 120s loads plus 3× busy time.
Those are sensitivity assumptions, not provider or model performance claims.

For a later experiment, propose warm planning p95 <=5s, initial cold start
<=60s with visible queuing, and p99 <=2×p95 for the same workload. These are
candidate product targets, not adopted guarantees. Record model load, processor,
prefill, decode, queue, network and local action/verification separately. Do not
include human approval waiting in GPU inference time; do include it in user
journey time and in any GPU keepalive actually incurred.

Measure at least 30 cold starts across restarts/regions, 500 warm requests and
load at 1/2/4/8/16 simultaneous tasks, fixed image/context distributions and
batch sizes 1/2/4. For p99, use >=1,000 samples and uncertainty intervals; a
small pilot's p99 is descriptive only. Record OOM, eviction, requeued/failed and
cancelled requests in denominators. After safety/runtime qualification, replay
fixed arrival traces at mean load, 8× bursts and an outage, rather than deriving
p99 from a monthly GPU-hour total. Capped traffic must be simulated separately
from the uncapped heavy-user mix.

The executable warm calculation is an intentionally conservative sizing
hypothesis: busy GPU work × peak/mean 8 ÷ utilization 50%, rounded up to
whole always-warm workers. It excludes measured batching and trace-based
burst pooling. Its large counts for uncapped power users are **not** a fleet
recommendation. A hybrid warm floor plus flex overflow needs arrival-trace
simulation; don't pretend it is cheaper before measuring coverage and latency.

## Operations that precede any service launch

Queue bounded per tenant and globally; one task admitted per tenant initially.
Reserve a worst-case task dollar allowance before dispatch and atomically settle
usage afterward; stale reservations expire safely after reconciliation, not
double dispatch. Cap task calls, context/image bytes, wall time, retries,
worker count, stored bytes and total monthly spend. Provider $80/hour default
limits are far too high for a tiny paid beta and do not prove a monthly cap.

Cancellation removes queued requests and stops planning immediately. Active
inference may continue billing until worker shutdown; account for that cost.
Never cancel a dispatched local mutation by dropping the cloud connection and
claiming it failed. Cloud retries need request ids and deadlines; device actions
need independent unknown-outcome reconciliation. Model weights live in founder
compute/storage, never on the founder's personal Windows computer in this study.
Holo4 remains temporary comparison capacity; no large warm worker at launch.

No GPU provider, payment account, endpoint or subscription was created. Remote
training/evaluation, free credits and paid trials all need explicit permission
and rights/access verification before use; "free credit" is not spending authority.
