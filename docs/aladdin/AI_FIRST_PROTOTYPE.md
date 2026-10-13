# First meaningful prototype and measurable execution roadmap

Status: SPECIFIED, not a built AI product. No infrastructure or training spend
authorized. Existing deterministic filesystem regressions may be run offline;
the future hosted model experiment remains behind separate gates.

## Experiment 0: deterministic and mock controls

Create a synthetic Downloads workspace of 12 small owned files with known hashes,
extension classes and fixture dates: six text/PDF/images plus uppercase extensions,
extensionless/hidden files, Unicode names, an existing destination collision,
a stale-content case and a reparse/symlink denial case. Total <=1MiB. The fixture
manifest supplies date metadata only for this test; the current MCP file list
does not establish a general mtime observation API. Never point the experiment
at the user's real Downloads or another repository.

Control D0 selects a fixed recipe with explicit parameters. Control D1 uses a
mock planner returning the proposed narrow `organize_files` intent, exercising
the same validation/execution/checker loop that future Hala will use. Mock
approval belongs only in an isolated test harness and is visibly labeled;
it cannot produce real broker tokens or be enabled in production configuration.
Existing `qdrald` `fs_mutation::tests` exercise fake test brokers, SOFT moves/
mkdir/edit and STRONG-removal denial/availability. Their passing result would
prove that slice, not a full task scheduler or cloud adapter.

The recipe defaults to extension then `modified_utc` year-month, excludes
directories/reparse/protected state, preserves filenames/bytes, and stops on
collision or drift. The user's example "by file type and date" is ambiguous
about date source and timezone; the real UI must explain its default and let
the user choose before mutation. No overwrite, delete, extraction or shell.

Execution sequence and proof:

1. Local user selects the trusted synthetic root; bound inventory/metadata,
   device and policy revision are observed. No screenshot is needed.
2. Recipe/mock planner returns a bounded intent; the validator rejects any
   external root, unknown field, shell command or approval claim.
3. Executor expands <=12 moves and <=12 create-directory actions into existing
   workspace-relative `fs_mkdir`/`fs_move` calls. Show the plan, then preserve
   the broker's fresh per-action SOFT approvals. No task blanket authorization.
4. Bind source identity/content and destination to each operation. On stale
   state/collision, stop and report; do not ask the model to invent a new path.
5. An independent checker enumerates the entire fixture and sibling sentinel
   directory, confirms file hashes, expected destination structure, no original
   leftovers for moved files and no unexpected writes. Use a separately
   constructed expected map; do not verify by echoing the executor's receipt.
6. Report exact verified moves, skipped files and partial/denied/unknown states.
   Record calls/actions/latency. Inject approval denial, cancellation before
   dispatch and after one move, source mutation, broker failure and response
   loss after dispatch. Unknown outcome requires new inventory reconciliation,
   no repeated move and no automatic rollback mutation.

Acceptance for controls: 30 fixture resets, identical safe final states for
D0/D1, zero out-of-root effects, truthful partial/cancel states, no effects after
denial or cancel-before-dispatch, and checker detection of a deliberately wrong
move. A concurrent filesystem race needs native held-identity checks, not a
Python `resolve()` simulation. The control implementation is future test-only
work, not a new privileged executor introduced by this study.

## Experiment 1: real Hala with the same filesystem loop

After rights and spending approval, replace the mock with the pinned
Qwen3.5-4B. The user types a native Aladdin text request; founder-side inference
returns the narrow intent. Existing local code/policy/broker executes and
verifies, and the WPF timeline presents the actual result. Customer uses an
Aladdin test identity; no OpenAI/Anthropic account/key or third-party chat UI.
This is a complete workflow, not a text-generation demo. An initial CLI/test
adapter can prove the loop before native UI wiring; label it as a harness,
not the customer-ready desktop experience.

Proposed hardware: one founder-controlled Linux NVIDIA **24GB L4-class** worker,
>=32GB host RAM, 20GB persistent model space, vLLM 0.31.0 pinned commit, BF16,
8K context and <=256 output tokens, one in-flight call, no escalation. Exact
available small flex GPU/runtime compatibility must be confirmed before spending.
Download weights only to remote temporary compute from immutable pins.

Proposed **maximum future authorization: $5 total**, including taxes/transfer/
storage/control resources, <=1 hour of billed worker lifetime, <=100 requests,
<=60 minutes wall time, and deletion of temporary resources after evidence
export. At $0.69/h the GPU-time ceiling is $0.69, but storage minimums,
provider funding minimums and startup/egress may require more than $5. If an
account requires a higher deposit or cannot enforce the total cap, stop and
request a revised budget. This is an approval proposal, not permission to spend.
Free compute is acceptable only after verifying capacity, privacy and zero
chargeable overflow; no free-tier credit is silently activated.

Use 60 frozen free-text prompts (30 English/30 native Arabic), balanced across
supported grouping/date/collision semantics and ambiguity, plus safety/error
fixtures. No model is allowed to change verification or approval criteria.
Proposed go: >=54/60 correct verified intents/tasks (separate ambiguous ask-user
labels), all safety denials honored, no unauthorized effect, <=1 AI call for
ordinary grouping tasks, and evidence complete enough to replace GPU-second
assumptions. Observe warm/cold latency rather than declaring it in advance.
Stop on first unauthorized effect, protected-data upload, budget/control failure,
repeated OOM/invalid parser, >2 invalid proposals for one task or any training
contamination. Failure means diagnose the harness/model and revise, not deploy.

This initial experiment says little about GUI competence: no screenshot is
required. A second paid comparison would add owned Arabic/English UI grounding
fixtures and the small/optimized vs Holo4 baseline; it needs its own bounded
hardware, runtime and spend proposal. Do not provision an 80GB worker now.

## Shortest credible roadmap

Estimates below are focused person-days for an experienced engineer with review
support, not dates/commitments. Integration/security failures can increase them;
existing SG-000096/108 work proceeds under its own governance and is not re-counted
as completed AI work.

| Milestone | Reuse / prerequisites | Estimated effort | Objective exit / measurable blocker |
|---|---|---:|---|
| R0 current research | This package, source pins, offline cost tests, adversarial review | Research work only | Signed+DCO commits, qualified exact-head review/CI if spending permits; no grain completion change |
| R1 Basic task loop | WPF read-only shell, existing FS kernel/broker; private authenticated API proposed in adopted plan | 8–15 days | Real create-only report and organizer control in isolated workspace; progress/cancel/unknown; metadata contract explicit; 30 resets + injected failures |
| R2 model/server feasibility | R1 control harness, public rights plus adopted model-grant resolution, approved <=$5 test | 4–8 days | Pinned compact model/parser, 60 bilingual cases, actual billing/VRAM/startup measurements; no production endpoint |
| R3 native Pro integration | R1/R2; reviewed device-bound cloud proposal adapter and local-only-profile boundary | 8–15 days | Native chat→approval→execution→independent checker→report; tenant isolation, out-of-order/stale/cancel/outage tests; no external customer keys |
| R4 browser/desktop extension | SG-000108 launcher/exposure/engine-window/E2E closeout and approved Pro contract; existing SG-000095 local lease | 6–12 incremental days, excluding current browser work | Controlled PDF journey, UIA/RTL forms and artifacts, no personal profile/remote Full User widening; domain success gates |
| R5 qualified bounded beta | R3 and whichever R4 scopes actually pass; installer/signing/legal/entitlement/privacy decisions | 6–10 days | Frozen native task evaluation, measured cost/latency/support pilot, failure honest, approved quota/price; release decision separate |
| R6 optional scale/voice/tuning | Real beta traces and diagnosed bottleneck | Estimate only after evidence | Larger model/adapters/warm pool/STT-TTS each needs benefit, runtime/rights/spend qualification; no default prerequisite |

A narrow filesystem Pro feasibility path is about **20–38 person-days after
this plan**, not a reliable calendar release promise. Broader beta adds R4/R5
and external signing/privacy/runtime qualification; do not advertise general
computer control merely from the organizer demonstration.

## Dependencies, risks and founder decisions

Preserve the active grains and unfinished A10 engine-version work; do not import
another worktree's dirty changes. PR #323 remains draft browser qualification,
not evidence merged on main. A prototype can progress using qualified FS while
browser finishes; it cannot expose new desktop cloud authority on that basis.

Separate decisions: architecture amendment accepting a compact Hala launch
candidate; actual Pro pricing/allowance; model/licensing/grant records;
entitlement/offline grace policy; private proposal/API contracts and new grains;
any transaction approval, metadata/capability exposure or privilege expansion;
production deployment/regions/retention/DPA; paid inference/training/hardware;
signing/distribution/release; voice; dataset acquisition and training consent.
This assignment authorizes planning these decisions, not activating them.

Largest risks: model fails Arabic/long tasks; approval prompts make Basic/Pro
unusable; browser helpers remain unwired; queue/cold latency defeats UX; unknown
mutations duplicate effects; quotas buy too little useful work; support and
signing/legal consume the $3 margin; license/data rights insufficient. Resolve
them with the smallest controlled experiment, not a broad rewrite.

## Resume without this conversation

Read [master plan](AI_PRODUCT_MASTER_PLAN.md), [evidence](AI_EVIDENCE_REGISTER.md)
and [decision](../adr/0001-native-hala-small-model-research.md). Fetch live main,
inspect open PRs/worktrees and `.specgrain/`, then reread adopted governance.
Run offline cost tests and compare regenerated tables. Locate the research
branch/PR by its exact published head from the evidence handoff; do not assume
this package's baseline SHA remains current. Obtain the documented narrow
review-CI spending exception before opening a PR if its Jev job is still paid.
After qualified merge, record exact merge/main SHA and post-merge CI externally
or in a subsequent factual handoff; never self-reference a future merge SHA.
