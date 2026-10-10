# Infrastructure and cost measurement — pricing deferred

Status: independent planning review, 2026-10-09. PROPOSED, NOT ADOPTED. No implementation or authority change. Evidence baseline: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Economics correction

Original script arithmetic reproduces. At its illustrative assumptions: 1500 Hala steps × $0.000825 = $1.2375/month; $2.20/h ×720h = $1584/month; simple API-cost crossover = 1280 subscribers. Income break-even at a guessed subscription price is a different calculation. Its 0.2 amortized GPU-seconds/step and 30% utilization imply 3,888,000 steps/GPU-month, not demonstrated capacity or latency. Subscriber count alone is a poor scaling trigger.

Pricing, allowances, packs and checkout remain deferred. Replace billing milestones with cost per attempted and verified task, utilization, cold/warm/idle spend and caps. Core avoids mandatory founder inference cost; relay, support, signing and updates still cost money. Future training belongs to the founder's separate program and is not on Core's critical path.

## Cost-only helper

tools/unit_economics.py has no network/dependencies. Its hourly/GPU-second/utilization inputs are explicit illustrative estimates, not current provider quotes or capacity measurements. It reports warm spend, amortized work cost, capacity assumptions and a separately labeled historical API-equivalent crossover. It recommends no subscription price, allowance, billing rollout or third-party AI pilot.

Future measurements must include cold load, queue, active/idle worker seconds, storage, egress, voice, failures and utilization; cost per verified task includes failed attempts. Record exact region/GPU/runtime/revision and quote date. Do not provision or purchase infrastructure under this review. Current deployment alternatives and their tradeoffs are in ALADDIN_AI_SERVER_ARCHITECTURE.md.
