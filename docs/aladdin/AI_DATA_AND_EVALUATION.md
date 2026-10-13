# Hala data, adaptation and independent evaluation

Status: PROPOSED experiment protocol. No weights, datasets or customer
trajectories downloaded; no training or model evaluation performed.
Use [model selection](AI_MODEL_SELECTION.md) and [first experiment](AI_FIRST_PROTOTYPE.md).

## No training before a baseline

Compare unchanged checkpoints with the same frozen task harness first. A bad
coordinate transform, stale target, missing tool or expensive approval wait is
not fixed by LoRA. Improve observation formatting, bounded JSON parsing,
deterministic templates and independent verification before collecting new
training data. Arabic fluency and RTL geometry need separate assessment.
Default customer training is off and unrelated to diagnostic telemetry consent.

Candidate data sources:

| Source | Inspected pin / rights evidence | Use / excluded use |
|---|---|---|
| Founder-authored synthetic Windows/browser apps | Pin each app/fixture/template revision and authorship record | Best first source for rights-cleared tool choice, recovery, UIA and RTL; avoid third-party assets/secrets |
| Hcompany/DragOn | `3451c1fc3543c18fc3361a32bfe1f12cc69b64b5`; Apache-2.0 card; English/French | Drag grounding across highlighting, spreadsheet selection, slide manipulation and sliders. Training partition only after asset/provenance audit. No official/public/private test examples used for training |
| showlab/ShowUI-desktop-8K | `0d190dfee8ca2cb24f1a7157f8bdde52845d5348`; 7,496 train rows, OmniAct-derived; license metadata absent | Useful research reference, **training blocked** until underlying dataset/screenshot rights are verified; code/model MIT is not dataset clearance |
| Rights-cleared owned desktop trajectories | Written participant/app/asset authorization plus redaction and provenance hashes | Independent task trajectories with verified final states; use synthetic accounts and documents initially |
| Arabic/RTL owned fixtures | Native bilingual author review, app/layout family IDs | Arabic labels, dates/numerals, filename normalization, RTL widgets and mixed English text; translation/resizing are linked derivatives |

DragOn card task counts are not independent screenshot counts: several
instructions can refer to one image. Source licensing does not certify every
embedded screenshot/trademark/personal-data right. Keep provenance and record
any right not established. Never use benchmark test trajectories, evaluation
outcomes or model-generated solutions to improve the model being scored on
that frozen benchmark. No default scraping of arbitrary copyrighted apps.

## Dataset stages and stop/go criteria

All counts and hardware below are proposed experiment sizes, not acquired data.
The unit is an independent task trajectory or grounding scene with linked steps,
not each resized image, translated prompt or repeated run.

| Stage | Size/diversity and source | Labeling and separation | Training envelope / metric / gate |
|---|---|---|---|
| Pilot | 500 independent owned trajectories plus up to 2,000 diagnosed grounding/selection scenes; >=5 app families, >=30 layout families; 30% native Arabic/RTL | Two reviewers for target/postcondition/abstention labels; disagreement adjudicated; group by app/template/asset. 70/15/15 train/dev/holdout, with a separate frozen launch evaluation | One LoRA, short context, frozen vision tower first; 24–48GB GPU feasibility test, timeout <=4h, priced cap separately approved. Proceed only for >=5pp paired improvement on diagnosed weakness, no safety/Arabic regression and nonnegative overall task-cost effect |
| Intermediate | 5,000 independent trajectories and up to 20,000 scenes; >=15 app and 100 layout families; unseen app holdouts; explicit recovery/denial/ask-user examples | Verify artifacts and actions with deterministic checkers; rights manifest per source; deduplicate by image/asset/template/perceptual hash; keep all derivatives in one split | 48–80GB feasibility envelope with fixed epochs/early stopping after measured memory/time pilot; candidate quality gain must survive held-out apps and lower cost/success than larger-model escalation |
| Advanced | 20,000 independently authored/consented trajectories and up to 100,000 scenes; long-horizon failures, interruption and UI-change recovery; >=30 app families | Step labels link original intent, observation, exact action, approval class and verified outcome; fixed task/app family holdouts; benchmark test partitions excluded from collection | Multi-GPU only if smaller experiments show value. Full adaptation/RL needs a funded proposal, verifier reliability and reward-hacking tests. Stop if regression, contamination or no statistically supported gain after two bounded attempts |

Do not pick two LoRAs merely to match Brain/Eye names. Compare no tuning,
one LoRA and two adapters only after evidence identifies a specialist benefit.
Full-model tuning has more memory/forgetting/operational risk; reject it for the
pilot. Preference optimization needs trusted positive/negative outcomes.
Reinforcement optimization needs resettable, rights-cleared environments and
verifiable rewards that cannot be satisfied by forging a receipt or hiding effects.
Do not train against a model's own opinion of task success.

Free Colab/Kaggle capacity is opportunistic: quotas, GPU availability, sessions,
licenses/privacy and storage vary; a 4B vision LoRA may exceed one free GPU.
Prepare a small CPU data-validation run first, save checkpoints in suitable
founder-owned private model/data storage only after rights/access approval, and
use resumable bounded remote jobs. No weights on the founder's personal PC.
Free credits do not authorize chargeable overflow or training in this assignment.

## Freeze the independent task set

Propose 300 independent task instances before any candidate evaluation:
60 filesystem, 60 synthetic browser, 60 native desktop, 30 document/spreadsheet,
30 recovery/UI-change, 30 permission/injection and 30 outage/cancel/unknown.
Browser and spreadsheet groups activate only when their actual adapters are
qualified; record unavailable tasks, do not silently omit them to improve scores.
Use 150 Arabic and 150 English instructions across the same domain strata,
including at least 60 RTL layouts; language is a cross-cutting label, not extra
independent cases. Require >=4 fully held-out application families, disjoint
template/asset roots and unique fixture seeds across development and test.

Write and hash the task manifest, fixture code/data, verifier rules, model/prompt
pins, context/image ceilings, runtime/GPU/driver, approval script and stop budget.
Keep the manifest immutable during a comparison. Fixing a verifier error creates
a new manifest and reruns all affected variants. Record task exclusions and why.
Use public ScreenSpot/OSWorld/Windows Agent Arena/WebVoyager only as external
evaluation supplements under their terms, not training data or proof of local
security. Desktop capability claims require native Windows 11 evidence.

## Controlled variants and AI-call experiment

| Variant | What is held fixed / changed | Calls permitted | Question |
|---|---|---|---|
| D0 deterministic | Selected exact recipe/parameters, same kernel and checker | 0 | What value can Basic complete without model work? Natural-language coverage not claimed |
| D1 request-to-template | Same recipe executor, model interprets one free-text goal | 1 (+1 bounded invalid-output repair recorded separately) | Is intelligence necessary only at the entry point? |
| D3 bounded recovery | Same executor; model on goal and up to two genuinely ambiguous state changes | <=3 | Do targeted recovery calls pay for themselves? |
| DR repeated vision | Same authority, tool ceilings and task/time budget, model after each observation/action | <=20, with extra complexity strata separately declared | What does repeated assistance cost and improve? |
| Small/minimal tools | Same small model and approved kernel, narrow visual tool menu | Same ceiling | Do structured tools cause the efficiency gain? |
| Small/optimized harness | FS/UIA/DOM/extraction/templates/verification as qualified | Same ceiling | Preferred launch candidate |
| Larger baseline | Same task/kernel/observation/checker, Holo4 temporarily served | Same ceiling | Is additional task completion worth serving cost? |
| Conditional routing | Frozen route classifier/thresholds, large escalation budget | Same total ceiling | Does incremental success outweigh both models' work? |
| Adapted small model | Same frozen harness after baseline and independent train split | Same ceiling | Does tuning add a real gain rather than benchmark leakage? |

Never weaken safety controls for the minimal-tool or repeated-call baselines.
Instrument request selection before execution, billable startup/idle/execution,
all retries/escalations and final state. Compare zero/one/three/repeated-call
variants on both identical known recipes and free-text coverage; a template's
perfect success on a tiny domain is not general natural-language competence.

## Metrics, uncertainty and quality gates

Primary: verified tasks/attempted tasks and cost/verified task, with all denied,
failed, timed-out and cancelled cases explained. Also record: postcondition
pass independently of reported completion, unauthorized effects, wrong-target
effects, intervention rate, actions/task, model calls/task, actual billed GPU
seconds, end-to-end p50/p95/p99, recovery success, image bytes/tokens, schema
validity, abstention/false-success and reliability on unseen layouts.

Run each case three times with fresh fixture reset, randomized paired candidate
order and fixed sampling/seed policy. Use paired task-level differences and
cluster bootstrap by task/application family; don't treat repeated steps or
three runs as independent new tasks. Report Wilson intervals for success rates,
paired bootstrap intervals for cost/latency and separate Arabic/domain results.
Zero unauthorized events in n cases is not proof of zero risk; the approximate
95% upper event-rate bound is 3/n. Report observed zero and sample limitations.

Proposed narrow-beta gates: >=90% verified success in the declared supported
filesystem templates, >=85% in each enabled browser/desktop domain, no
unauthorized effects observed, zero observed false **verified** completion
reports for supported deterministic filesystem tasks, <=10% user intervention
apart from required approval/clarification, and no meaningful Arabic regression.
Truthful partial/unknown outcomes are not false completion. Model success
mispredictions caught by the independent checker are counted separately and
must never be displayed as verified. Report finite-sample uncertainty for zero
false-verified events rather than promising absolute perfection.
Require >=99% schema-valid proposals with invalid ones safely rejected; visual
wrong-target rate must meet a separately approved risk budget. These thresholds
are research proposals requiring founder adoption, not a release authorization.
Safety fixtures must demonstrate refusal for forged approvals, stale/foreign refs,
protected content, prompt injection, cross-tenant requests, expired sessions,
budget exhaustion and unavailable model. Any unauthorized effect stops the
experiment immediately and invalidates release qualification.

Adversarial reviewers must challenge task/rights sampling, approval fatigue,
small-model reasoning truncation, observability leaks and the economics' workload
distribution. Publisher scores, mock tests and the cost-model unit tests do not
replace this real open-weight evaluation. No Hala benchmark result is claimed
by this research package.
