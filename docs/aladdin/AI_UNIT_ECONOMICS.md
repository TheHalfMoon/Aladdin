# Basic and Pro unit economics

Status: EXECUTABLE SENSITIVITY, not pricing adoption or a revenue forecast.
Primary public tariffs/fees were checked 2026-10-13. No workload is measured.
The business decision is the lowest useful subscription with bounded founder
cost, at 10/50/100/500/1,000 subscribers; paid conversion/retention are unknown.

## Reproduce and inspect

```text
python docs/aladdin/tools/ai_cost_model.py
python docs/aladdin/tools/ai_cost_model.py --json
python -m unittest discover -s docs/aladdin/tools -p test_ai_cost_model.py -v
```

No dependencies/network/accounts/GPU needed. [Inputs](tools/ai_cost_inputs.json)
are the authority for calculations; [all tables](research/AI_COST_SCENARIOS.md)
cover $5/$8/$10/$15/$20 and all subscriber counts. The old
`tools/unit_economics.py` remains preserved as a historical cost-only helper.
New tariff research supersedes neither its evidence nor adopted deferred billing.
The committed generated Markdown normalizes platform stdout line endings to LF;
its content is exactly the calculator's default output.

## Definitions and formulas

An attempted task is a submitted in-scope goal. A successful task has the
independent local postconditions satisfied, no unauthorized effects and an
honest completion report. Denied/cancelled/error tasks remain in total spend
and are separately reported; never discard failures to lower reported cost.

For tasks T, calls/task M, retry overhead R (additional attempts/base calls),
busy seconds G, cold probability P, billed startup L and per-call idle I:

`small calls = T*M*(1+R)`

`small billed seconds = calls*(G + P*L + I)`

`GPU dollars = small billed seconds*small hourly/3600 + extra advanced billed seconds*advanced hourly/3600`.

Escalation is additive: a small-model attempt already happened. Failures and
retries consume compute even if there are no verified tasks. Input/image sizes
describe benchmark workload and network reserve; **there is no fictional
self-hosted dollar/token tariff**. GPU service time remains an explicit measured
input once available. Network reserve counts uploaded and returned bytes
conservatively; it is not a verified provider egress charge.

`verified tasks = T*success probability`

`inference infrastructure cost/success = (GPU + control + model storage + network/state)/verified tasks`.

`operating cost/success` also includes support. Payment, sales tax and maintenance
are separately modeled, so a GPU-only figure cannot be mistaken for profit.
When all tasks fail, spend remains real and cost per verified task is undefined;
JSON returns `null` with `UNDEFINED_NO_VERIFIED_TASKS`, not zero. Capped expected
counts can be fractional or zero; they are scenario expectations, not real half-tasks.
Cash contribution deducts payment fees, refunds/dispute reserves, GPU,
infrastructure and support from tax-exclusive receipts. After-maintenance
contribution subtracts founder maintenance labor too; it is not net company
profit (acquisition, accounting, legal/signing and initial R&D are unresolved).

## Cohorts: illustrative workload, not observed customer behavior

| Cohort | Tasks/mo | Steps/task | Calls/task | Input/output tokens/call | Images/call; KiB/image | Retry overhead | Success/error assumption | Busy GPU seconds/call | Support min/mo |
|---|---:|---:|---:|---|---|---:|---|---:|---:|
| Light | 20 | 4 | 1 | 1,200 / 128 | 0.2; 200 | 5% | 95% / 5% | 3 | 1 |
| Regular | 100 | 8 | 3 | 2,000 / 192 | 0.5; 250 | 15% | 90% / 10% | 6 | 3 |
| Heavy | 400 | 15 | 8 | 3,500 / 256 | 0.8; 300 | 25% | 80% / 20% | 8 | 8 |
| Power | 1,500 | 30 | 20 | 6,000 / 384 | 1; 400 | 35% | 70% / 30% | 12 | 20 |

Fractional images are workload averages. Fixed mix is 40/45/13/2 percent.
Default escalation is 0%; stress is 10% of all small calls, with separate
15/20/30/45 busy seconds plus 50%-probability 90s large-model startup and idle.
Serverless idle allocation is explicit per cohort, not a hidden utilization
discount. Rare/long visual tasks could be much more expensive than these inputs.

Public fee comparator: Paddle 5% + $0.50/check-out transaction, with explicit
bespoke pricing for under-$10 products. This does **not** verify merchant
eligibility or a $3 signed offer. Stripe's US headline 2.9%+$0.30 is only a
comparison; region/currency redirects and billing/tax product fees differ.
The executable base uses the more conservative published Paddle headline,
2% refunds and 0.1% disputes with $15 reserve (estimates, not its quoted dispute
fee). Do not double-count MoR-covered billing support or claim it covers product
support. Tax-inclusive rate 0 means tax-exclusive scenario, not tax exemption.
Set 15% as an illustrative VAT-inclusive sensitivity only after determining
the seller/customer legal treatment; no jurisdiction is inferred from this PC.

Fixed cash baseline is $15 control/auth/license service + $1.40 model storage
+ $2 disk/backup per month; these service/storage amounts are estimates except
the cited storage unit tariff. Per-account state is $0.05; transfer $0.10/GB is
a reserve, not a supplier quote. Support is $20/h. Maintenance is 15h×$20=$300/mo.
Signing/legal/compliance have an unresolved provision, currently zero in the
illustration; include real quotes before calling the price commercially verified.
Shared Basic/Pro fixed costs must be allocated once across the product fleet.

## Basic stays $3: sustainable only with a low-touch product

After modeled fees/refunds/disputes, $3 yields $2.272 in receipts. One minute of
support ($0.333) and $0.05 variable infrastructure leave **$1.889/account**
before fixed costs. The Basic-only $15 control floor breaks even at **8 accounts
cash**, or **167 accounts** including $300 maintenance. At 100 accounts,
cash contribution is $173.87 but after-maintenance contribution is **-$126.13**.
GPU cost is zero; hosted relay is optional and its actual cost must be added.

Three product-support minutes instead of one cost another $0.667/account and
raise full-cost break-even to about **258 accounts**. Five minutes leave about
$0.555 before fixed cost and require roughly **568 accounts**. Verify these by
editing the support input; no support data has been collected. At very small
scale the founder subsidizes maintenance. Annual collection at $36/year can
reduce fixed transaction fees while preserving the $3 monthly equivalent, but
refund/entitlement/tax rules and customer preference remain decisions. No
annual plan or billing implementation is created here.

## Pro: all prices and uncapped risk

At 100 subscribers, the unbounded mix uses $642.22 GPU/month and $777.05
operating/month. After $300 maintenance, monthly contribution is:

| Pro research price | $5 | $8 | $10 | $15 | $20 |
|---|---:|---:|---:|---:|---:|
| Uncapped mix, 100 accounts | -$664.05 | -$385.35 | -$199.55 | $264.95 | $729.45 |

At $10, a regular user contributes $3.31 after their equal fixed/labor allocation
at 100 accounts; a heavy user loses $12.95 and a power user loses $193.07.
Even $20 loses money on these unbounded heavy/power assumptions. The 2% power
cohort contributes about 59% of modeled GPU expense. An average margin is not
protection from a few unusually active customers.
At ten subscribers, the 2% mix represents 0.2 expected power users. The executable
discrete sensitivity instead uses four light/four regular/one heavy/one power
customers; one power user alone costs about $190 GPU/month before other costs.
It is not a probability forecast; it exposes concentration hidden by fractional
cohort expectations. The full discrete price table is included in generated results.

Full-cost flex break-even for this uncapped mix is about 268 accounts at $10,
55 at $15 or 31 at $20; $5/$8 have negative variable contribution and no scale
fixes them under the assumed mix. Calculate `N >= fixed/(net receipts-variable
cost)` only when the denominator is positive; actual traffic/support change it.

## Recommended research price and allowances

**Test $10–$15/month for a bounded text-first private beta.** $8 is a lower-price
challenger once caps, retention/value and real costs support it; $5 is a tightly
limited/light-use experiment, not the recommended general Pro price. No price
is adopted. At 10 customers, all examined prices need a founder labor subsidy.

With an illustrative 7,200 billed-small-GPU-second monthly ceiling per account,
GPU spend is <=$1.38/account at the quoted small flex tariff. The base mix uses
less ($0.782 average). At $10, capped after-maintenance contribution is -$249.64
(10), $25.38 (50), $369.16 (100), $3,119.41 (500), $6,557.21 (1,000). Arithmetic
break-even is about 47 accounts. At $8 it is about 64; $5 about 143. These include
unchanged support estimates, and omit still-unquoted legal/signing/acquisition.

The cap supplies roughly 100 regular tasks, **or** 35 heavy-style tasks, **or**
11 power-style tasks under the base call assumptions; it does not supply all
three. The capped mix delivers 51.9 expected verified tasks/account versus
110.7 uncapped. Cost control that removes most wanted tasks is not automatically
a useful product. Measure willingness to pay and intervention/success at those
limits before adopting them. More efficient zero/one-call templates improve
value; hiding failures or outsourcing manual steps does not.

Display predictable task credits by measured complexity with a preview before
starting. Keep an internal dollar/time reservation including startup/idle and
failures. Do not directly penalize users for founder pool cold-start accidents;
reserve those costs in the fleet budget and validate a fair mapping. Denied
actions must not consume execution credits unfairly; compute still costs the
founder. Protect both individual and global spend: finite calls/task, image and
context bounds, concurrency, queued requests, retry count and worker ceiling.
No automatic paid top-up or infinite "fair use" promise. Exhaustion pauses AI
with an honest message while Basic commands and local stop remain available.

## Uncertainty and decisions before monetization

The stress workload has 3× busy time, 120s cold loads on every small call and
10% Holo4 escalation: at 100 users its GPU is $5,413.59/month and $10 contribution
after maintenance is -$4,977.03 including extra model storage. Unmeasured speed/startup assumptions can reverse
the cheap architecture's apparent margin. Conversely, pooling/caching/batching
could reduce cost, but receive no assumed discount before measurements.

Advanced residency adds 80GB storage in the stress model and possibly a separate
warm floor beyond its GPU calculation; the stress is not a complete Holo4 service
quote. Currency conversion, VAT/sales tax, dispute fees, actual support,
signing/compliance, egress, CI maintenance, sales acquisition, churn and initial
development require updated inputs. A "$10 sustainable" claim is conditional
on a useful capped workload, roughly >=50 paying users, the tested success and
latency gates, and those unresolved costs. Current cost per successful task is
**not measured**; modeled capped operating values range $0.0720 to $0.0369
across 10–1,000 users. They are engineering scenarios, not marketing promises.
