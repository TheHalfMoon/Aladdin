# Evidence register and reproducible handoff

Status: RESEARCH EVIDENCE, inspected 2026-10-13. No production AI capability
or SpecGrain completion is claimed. Read [master](AI_PRODUCT_MASTER_PLAN.md)
and [prototype/roadmap](AI_FIRST_PROTOTYPE.md) before implementing anything.

## Evidence labels

- **VERIFIED SOURCE FACT:** live code/config/API inspected at a pin; not task performance.
- **MEASURED LOCAL RESULT:** an actual run reported with command/environment/scope.
- **PUBLISHER CLAIM:** official card/docs result, not independently reproduced.
- **ENGINEERING ESTIMATE:** effort, workload, GPU time/memory and future capacity assumptions.
- **PROPOSED DECISION:** research recommendation, not adopted privilege/price/deployment.
- **UNPROVEN / UNKNOWN:** absent runtime, rights, source or test evidence stays explicit.

No statistical model comparison, legal permission, hosted service uptime,
customer usage, support burden, revenue or GPU performance was measured.

## Repository and governance facts

| ID | Source / exact snapshot | Observation / limitation |
|---|---|---|
| R1 | [Aladdin tree](https://github.com/TheHalfMoon/Aladdin/tree/9098addeee1ddb9f856dfdcdfcb45d9cb3288f63), `git fetch origin`, GitHub commits/main | Public repo `TheHalfMoon/Aladdin`, default main, exact inspected SHA. Not assumed from the pasted historical SHA |
| R2 | `.specgrain/specs/SG-000096.json`, `SG-000108.json`, `.specgrain/canonical-evidence.json`, `docs/canonical/PLANNING_INDEX.md`, `CURRENT.md` | Two GRAIN packets, parallel disjoint authority explicitly permitted in SG108. Historical records are preserved, not blanket present implementation claims |
| R3 | [Ruleset](https://github.com/TheHalfMoon/Aladdin/rules/24456712), `docs/governance/DIFFCIPLINE.md`, `.github/workflows/ci.yml`, `review-gates.yml` | Ten strict contexts, no bypass, exact-head/signature/DCO/normal-merge discipline. Review Gates invokes TypeSafe API even for draft PR opening; OCR is delegation, not managed-model review |
| R4 | [main CI 38453056402](https://github.com/TheHalfMoon/Aladdin/actions/runs/38453056402) | SUCCESS on exact baseline main. Does not qualify new research commits |
| R5 | [PR323](https://github.com/TheHalfMoon/Aladdin/pull/323), `5e697fab045d760cbfa9759ccd489fc7c41847a8`; GitHub open issues | Sole open PR at initial inspection, draft TesterArmy E2E. Issues 260/264/265/266/267/271/281/288/290/292 open. Refresh before merge |
| R6 | `git worktree list --porcelain` + individual status | Main has untracked testerarmy directory; engine-window branch has modified spec/discovery/launcher/decision. Other observed worktrees clean; all preserved. New branch `research/aladdin-native-hala-2026-10-13` isolated from them |
| R7 | `crates/qdrald/Cargo.toml`, `src/sg000039_main.rs`/`browser.rs`; browser host Cargo/main; MCP `tool_contract.ts` | Selected daemon is sg000039_main, not legacy main.rs. 40 contract entries/26 remote core/14 local-only. Browser host dependency/product wiring absent; live worker helper source does not establish MCP reachability |
| R8 | `docs/aladdin/EXECUTION_FRONTIER.md` vs SG108 record and Git log through #326 | Summary at 2026-10-10 is stale about browser progress. New live code contains implemented/tested browser slices, while final wiring/version-window/E2E/closeout remain separate |
| R9 | `apps/aladdin-desktop-preview/src/Program.cs`, `docs/aladdin/UI_UX_AND_APP_SIZE_BUDGET.md`, packaging and legal files | Read-only WPF commands, AI/actions not connected; Node external prerequisite. Historical published artifact differs from main; self-host-only privacy terms need founder-Pro amendment before launch |

No source/approval/policy/CI/identity runtime file changed in this package.
The only integration into prior planning is the docs/aladdin README index link.
No repository settings or other user project changed.

## Model and data source register

The [machine pin manifest](research/AI_MODEL_PINS.json) records live revision,
architecture, API parameter total when present, license evidence type and
README SHA-256. Text/config/license only were retrieved, never weights.
All following model URLs have an immutable revision in [model selection](AI_MODEL_SELECTION.md).

| ID | Primary source | Supports / does not prove |
|---|---|---|
| M1 | [Qwen3.5-4B pinned card](https://huggingface.co/Qwen/Qwen3.5-4B/blob/851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a/README.md) and LICENSE/config at same pin | Dense 4B LM + vision, Apache-2.0, multilingual/tool/parser instructions. No Aladdin Arabic/task/latency result |
| M2 | [Qwen3-VL-2B pinned card](https://huggingface.co/Qwen/Qwen3-VL-2B-Instruct/blob/89644892e4d85e24eaac8bacfd4f463576704203/README.md) | Small VLM, commercial-license metadata and serving examples; no standalone LICENSE in inspected inventory |
| M3 | [Qwen2-VL-2B card](https://huggingface.co/Qwen/Qwen2-VL-2B-Instruct/blob/895c3a49bc3fa70a340399125c650a463535e71c/README.md) | Older baseline, Arabic image text support claim, license file. No reliable general GUI product proof |
| M4 | [ShowUI card](https://huggingface.co/showlab/ShowUI-2B/blob/cabec4fcc48d15ffd3efe0b33ea9bc7d41509d60/README.md), [paper](https://arxiv.org/abs/2411.17465) | GUI visual action/grounding model, MIT card and Qwen base; output examples need strict adaptation |
| M5 | [Fara card](https://huggingface.co/microsoft/Fara-7B/blob/4fca9cdeeff278777751ec558257753b89cf078b/README.md) | MIT attribution, English-only limitation, web-agent evaluation, deployment guidance; not Arabic/native Windows qualification |
| M6 | [Holo4 card/license](https://huggingface.co/Hcompany/Holo4-35B-A3B/tree/508458727831368da85d53a8055c2b0f923ed4f3) | 35B total MoE, public Apache/base claim, domain benchmarks; costs/scores attributed to publisher, not founder measurements |
| M7 | [Holo3.1-4B](https://huggingface.co/Hcompany/Holo-3.1-4B/tree/8c88265a5a159bfd1492db9243733dd2e6e04a6e), [0.8B](https://huggingface.co/Hcompany/Holo-3.1-0.8B/tree/72da4c53b351eb60e10a7022279019633c479292) | Compact domain challengers, Apache card and English tag; Arabic/tool/serving compatibility unqualified |
| M8 | [Qwen3.5 base](https://huggingface.co/Qwen/Qwen3.5-4B-Base/tree/1001bb4d826a52d1f399e183466143f4da7b741b), [Qwen3.6 base](https://huggingface.co/Qwen/Qwen3.6-35B-A3B/tree/995ad96eacd98c81ed38be0c5b274b04031597b0) | Base cards Apache-2.0; selected release audit still needs every artifact/license/notice/custom-code chain |
| M9 | [vLLM 0.31.0](https://github.com/vllm-project/vllm/releases/tag/v0.31.0), tag `db9527a46873454610df6dbedf79a36d6bf1a7f6`; [Qwen runtime recipe](https://docs.vllm.ai/projects/recipes/en/latest/Qwen/Qwen3.5.html) | Current runtime candidate and family recipe, not tested candidate 4B GPU/image/tool-parser image; larger-model recipe is not proof of 24GB capacity |
| D1 | [DragOn](https://huggingface.co/datasets/Hcompany/DragOn/tree/3451c1fc3543c18fc3361a32bfe1f12cc69b64b5) | Apache card, independent-image vs repeated-task distinction, train/eval partitions; underlying asset audit not completed |
| D2 | [ShowUI desktop data](https://huggingface.co/datasets/showlab/ShowUI-desktop-8K/tree/0d190dfee8ca2cb24f1a7157f8bdde52845d5348) | OmniAct-derived 7,496 rows; dataset license absent in metadata; **not cleared for training** |

Public Apache/MIT observations are research evidence, not verified founder
direct grants or legal counsel. Preserve the adopted model-grant release gate
until its scope is resolved for exact chosen checkpoints. Model permission
does not settle dataset/screenshot rights or product trademark clearance.

## Provider/payment/competitor register

| ID | Primary source, retrieved 2026-10-13 | Selected observation / limitation |
|---|---|---|
| H1 | [RunPod pricing](https://www.runpod.io/pricing), page updated 2026-10-08 | Small flex $0.69/h, L4 pod $0.59/h, A5000 pod $0.27/h, A100 flex $2.72/h. Region/capacity/GPU-class equality unverified |
| H2 | [RunPod billing](https://docs.runpod.io/serverless/pricing), updated 2026-10-03 | Start/execution/idle charged, active-worker discount by sales; storage rates. No historical fixed discount assumed |
| H3 | [Vast prices](https://vast.ai/pricing), [serverless billing](https://docs.vast.ai/guides/serverless/pricing) | Dynamic offer pricing; ready/loading compute, inactive storage/bandwidth. Numeric marketplace offer unavailable in page extraction, no guessed quote |
| H4 | [HF Endpoints pricing](https://huggingface.co/docs/inference-endpoints/pricing), [Jobs](https://huggingface.co/docs/hub/jobs-overview) | CPU/GPU tariffs and dedicated-endpoint initialization billing; Jobs are temporary compute, not free production hosting |
| H5 | [Modal pricing](https://modal.com/pricing) | GPU + separate CPU/memory, startup/keepalive, credits/region surcharges/egress. Credits excluded from sustained economics |
| P1 | [Paddle pricing](https://www.paddle.com/pricing) | Headline 5%+$0.50; under-$10 bespoke quote advised. Merchant/country acceptance and binding price not verified |
| P2 | [Stripe pricing](https://stripe.com/us/pricing) | US comparator is not founder quote; observed locale redirects show why regional fees cannot be assumed. No seller-country inference |
| C1 | [Desktop Commander pricing](https://desktopcommander.app/pricing/) | Local MCP free; hosted 10,000 free calls / $20 unlimited tool-call Pro. External AI clients still part of advertised architecture |
| C2 | [Claude tool docs](https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use-tool), [Pro pricing](https://support.claude.com/en/articles/8325606-what-is-the-pro-plan) | Hosted inference/client-controlled computer tools and safety guidance; $20 US Pro is separate from API usage |
| C3 | [ChatGPT agent](https://help.openai.com/en/articles/11752874-chatgpt-agent), [Plus](https://help.openai.com/en/articles/6950777-what-is-chatgpt-plus), [local work security](https://help.openai.com/en/articles/20001548-agent-security-and-local-work-sync-in-chatgpt) | Integrated paid agent and data-flow cautions; no general claim it only controls a cloud browser; no confidential economics inferred |
| C4 | [Browser Use pricing](https://browser-use.com/pricing), [local harness](https://browser-use.com/browser-harness) | Pay-as-you-go browser/model separation and local open-source option; browser price excludes AI cost |
| C5 | [Power Automate](https://www.microsoft.com/en-us/power-platform/products/power-automate/pricing), [desktop flows](https://learn.microsoft.com/en-us/power-automate/desktop-flows/introduction), [AutoIt](https://www.autoitscript.com/site/autoit/) | Deterministic workflow alternatives; regional product prices, no comparable task-success measurements |

## Actual validation and review receipt

Environment: native Windows host, bundled Python 3.12.14; repository baseline
above. Shell invocations use `rtk` per the supplied RTK instructions.

| Check | Actual result / scope |
|---|---|
| Offline Python economics tests | 10 tests PASS initially; final count recorded in review handoff after any repairs. Independent one-hour quote arithmetic, retry/cold/idle/escalation, failure denominator, tax, fixed allocation, invalid input, warm floor, cap delivery and Basic break-even |
| Existing native FS control | `cargo test --locked --offline -p qdrald fs_mutation::tests --target-dir <isolated work/cargo-ai> -- --test-threads=1`: **3 passed**, 78 filtered, 0.13s test time. Build 1m57s. Existing test brokers, not Windows Hello/user-installed task or Hala inference |
| Graft structural pass | v0.21.1, non-deep $0 build, scoped daemon/FS/MCP/browser host; 104 files, 2,204 nodes, 3,738 edges, no source import/AI call. BrowserWorker caller query undercounts duplicate names; raw imports/Cargo/dispatch inspected independently |
| Alibaba OpenCodeReview | Installed local v1.13.0; pinned CI version v1.12.9. Local exact-range delegation/rules and manual exclusions recorded in final handoff; no managed-model review claim. Future CI pinned gate still required |
| PStack | Resume/ADR/review used under explicit autonomous research scope. Fresh-context adversarial review receipt will be recorded in final handoff; no missing provider silently called green |
| Jev | Local `jev --version`: 2026.919.0; **not invoked for paid judgments**. Required TypeSafe CI gate unrun for new head until narrow spending exception. Never substitute an inline review for this required check |
| REA | Official repo inspected/pinned `4c838469477f0b6ea844c235f78534a55118ddca`; prior Aladdin REA reports read as historical evidence. No new reverse-engineering run: own source resolves reachability and avoids inferring capability from binaries. No bypass/paid provider or runtime result fabricated |
| New-head required CI / merge | Pending publication. Baseline green does not qualify this change. PR creation automatically runs paid Jev; no workflow disabled or bypassed |

Research exceeds Diffcipline's default 12-file/600-line **implementation** packet
target because the founder requested one integrated 22-section study, executable
economics and permanent evidence. The exception is explicit and coherent:
only research docs, one proposed ADR, pin/config/generated evidence and the
offline calculator/tests; no dependencies, runtime, privileges or canonical
grain-state delta. Split into focused signed+DCO commits, one reviewable PR.

## Publication and resume state

Research branch: `research/aladdin-native-hala-2026-10-13`, based on R1.
Check the latest signed head and final validation receipt when resuming; this
document cannot claim the SHA of its eventual own commit or merge.
Publish/qualify only after the explicit no-spending exception for existing
Jev CI (and any chargeable CI runner time) is received. No budget authorizes
GPU experiments/billing/training unless separately granted.
If publishing is blocked, preserve the committed plan and prepared PR body,
state the concrete blocker and verify canonical main unchanged. After any
authorized merge, verify actual main SHA/tree and post-merge CI; do not mark
the AI product built or a GRAIN closed merely because these planning files merge.
