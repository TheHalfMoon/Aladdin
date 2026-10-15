# Aladdin Basic and native Pro: implementation-ready research plan

Status: RESEARCH RECOMMENDATION / PROPOSED ARCHITECTURE AMENDMENT.
Evidence baseline: `TheHalfMoon/Aladdin`, `main@9098addeee1ddb9f856dfdcdfcb45d9cb3288f63`,
verified 2026-10-13. No AI product, trained Hala or production deployment is
delivered by this plan. Product identity is Aladdin per the founder request;
existing Deskal/qdral source/compatibility identity migration remains issue #281.

## Executive recommendation

Keep one product with **Basic $3/month** and **Pro with native founder-hosted
Hala**. Basic must offer usable command controls and deterministic workflows
inside the existing Windows app; it cannot require an external AI account.
Pro adds its own text-first task conversation and completes approved tasks
through the same trusted local executor, with independently verified outcomes.
No customer GPU, OpenAI/Anthropic key or separate AI subscription is required.

Use **one unchanged Qwen3.5-4B VLM plus a tool-first harness** for the first
experiment. Compare Qwen3-VL-2B, compact Holo3.1-4B and Holo4-35B-A3B. No
initial Brain/Eye adapters, fine-tuning, mandatory Reliance/router model or
always-warm Holo4 GPU. Serve the compact model on founder-controlled scale-zero
24GB compute, contingent on real runtime/privacy/cost qualification. Start with
native filesystem tasks; later GUI/browser workflows require their actual
qualified integration, not simulated contract evidence.

Research **$10–$15 Pro**, with $8 as a later measured-cost challenger and finite
usage limits. $3 Basic can have positive cash contribution at small scale but
needs roughly 167 low-support accounts to cover the illustrated $300/month
maintenance allocation. No prices/billing are activated. Heavy/power users,
cold starts and support can reverse margins; neither GPU-hour tariffs nor
model grounding scores prove a sustainable completed-task product.

## Index, precedence and amendment

| Document | Substance / next-session use |
|---|---|
| This master | Product tiers, current code, competitor positioning, final architecture and 25 direct answers |
| [Model selection](AI_MODEL_SELECTION.md) | Exact checkpoints/licenses/memory, Brain/Eye alternatives, initial runtime, escalation/reversal |
| [Hala architecture](AI_HALA_ARCHITECTURE.md) | Tool-first task loop, proposed contracts, privacy/security and inference minimization |
| [Founder hosting](AI_FOUNDER_HOSTING.md) | Provider tariffs, A–D scenarios, unknown capacity/latency and measurement protocol |
| [Unit economics](AI_UNIT_ECONOMICS.md) | Cohorts, fees/taxes/support, all prices, break-even, uncertainty and caps |
| [Data and evaluation](AI_DATA_AND_EVALUATION.md) | Rights/provenance, no-tuning baseline, three dataset stages, independent task protocol |
| [First prototype / roadmap](AI_FIRST_PROTOTYPE.md) | Deterministic/mock control, future <=$5 real-model proposal, milestones and resume steps |
| [Decision record](../adr/0001-native-hala-small-model-research.md) | Explicit proposed amendment to Hala candidate, no silent supersession |
| [Evidence register](AI_EVIDENCE_REGISTER.md) | Source pins, actual validation, governance/review/publication limitations |
| [Generated cost scenarios](research/AI_COST_SCENARIOS.md) and [model pin manifest](research/AI_MODEL_PINS.json) | Reproducible arithmetic and text-only metadata provenance |

Canonical governance, closed-grain designs and live code take precedence over
this research or a stale summary. Preserve `ARCHITECTURE.md`, WPF and Playwright
decisions, `MODEL_INTEGRATION_CONTRACTS.md`, relay vocabulary and all security
authority. The 2026-10-09 adopted Holo4/Hala One candidate becomes a proposed
compact-model experiment amendment, not an accepted runtime change. The
founder's new tier requirements govern this study; Pro pricing adoption,
production hosting and capability changes still need their separate decisions.

## Live frontier, preservation and governance

Default branch is `main`; live SHA equals the previously observed `9098adde...`,
but was independently refetched. Main CI run [38453056402](https://github.com/TheHalfMoon/Aladdin/actions/runs/38453056402)
was successful for that exact SHA. The local main checkout contains untracked
`apps/qdral-browser-worker/testerarmy/`; the engine-window worktree has modified
SG-000108 spec/discovery/launcher/decision files. They were not changed or
imported. Research starts in its own forward-only branch from exact main.

Final frontier refresh during this study: main advanced to
`9b3f838e7b5642ffc8b10cc72888b504c0561e5d` as PR #323 merged. The research
branch incorporates it with a normal signed+DCO merge, preserving upstream
history. Its post-merge CI [38458523264](https://github.com/TheHalfMoon/Aladdin/actions/runs/38458523264)
was in progress at refresh. New draft PR #327, head
`88db260e4517472711acf27f2ab2b4654668e9d3`, covers A10 engine-version enforcement.
The original worktree/PR observations below are initial snapshots, not claims
that those branches remained unfinished after this refresh.

SG-000096 and SG-000108 are both `GRAIN`, with parallel work explicitly permitted
for disjoint shell vs browser authority. Neither is canonically closed. The
draft open PR [#323](https://github.com/TheHalfMoon/Aladdin/pull/323), head
`5e697fab045d760cbfa9759ccd489fc7c41847a8`, covers TesterArmy browser qualification.
Issues #260/#264–267/#271 cover remaining shell/native identity work; #290 tracks
browser; #288 app memory; #292 a contained PowerShell test flake; #281 identity.
Issue counts/heads are snapshots, not permanent state.

`EXECUTION_FRONTIER.md` was last updated at 2026-10-10 and says browser
implementation not started. Live main instead contains merged confinement,
private worker, launcher, navigation/observation, actuation/downloads, declarative
WebMCP, extraction, map/crawl, robots/sitemaps and crash/profile cleanup slices.
Their individual evidence is in commits and the SG-000108 decision record.
This study documents the distinction without rewriting historical canonical
records or advancing grain state. Product wiring/engine-window/qualification
remain materially different from a completed helper.
The newly merged A8 record reports an independent TesterArmy fixture journey
and network probes. It explicitly does not drive the production worker and
does not exercise its local approvals/create-only commit; no product reachability
or general browser task claim is inferred from that successful test-only suite.

Ruleset [24456712](https://github.com/TheHalfMoon/Aladdin/rules/24456712) enforces
PR-only, no deletion/non-fast-forward and ten strict required contexts:
Governance, Supply chain audit, Rust/Ubuntu, Rust/Windows, Node/Ubuntu,
Node/Windows, Release qualification/Windows, Relay container/Ubuntu,
Alibaba OCR Delegation and Jev Semantic Review. Project Diffcipline also requires
signed+DCO commits, exact-head review, manual exclusions, resolved blockers,
normal merge and post-merge verification. Zero approving-review requirement
in the API is not permission to bypass semantic/security review.

## Source-grounded subsystem inventory

I = implemented in source; R = reachable in the selected runtime under its
policy/prerequisites; T = test-only/native qualification; C = contract or
planning only. R never means a shipped paid end-to-end UI feature. Effort is
incremental person-day **estimate**, not progress or an aggregate project quote.
Source paths below refer to the immutable baseline; [evidence](AI_EVIDENCE_REGISTER.md)
pins the GitHub tree and the actually selected daemon entrypoint.

| Subsystem / inspected source | I / R / T / C | Reusable interface and deterministic value | Blocker / security limit / AI dependency / effort |
|---|---|---|---|
| Authority kernel/policy: `crates/qdrald/Cargo.toml`, `src/sg000039_main.rs`, `qdral-policy` | I/R: selected binary dispatch, workspace/trust/policy/remote gate; T: negative tests; C: proposed task/cloud adapter | Existing request/envelope/failure vocabulary; deterministic admission replaces model risk decisions | No persistent native task API. Preserve sole authority and local-only ceilings; 3–6 days for minimal records/API slice |
| Approval broker: `qdral-approval/src/lib.rs`, `qdrald/src/fs_mutation.rs`, #286 note | I/R: SOFT/STRONG exact digest/nonce/expiry/consume; T: mock-presence regressions | Real local approval, not model consent | #278 repaired; valid tail rollback/same-user limitations remain. Task blanket approvals absent; 2–4 days UI presentation/denial/cancel integration |
| Windows UIA: `qdral-provider-uia`, `qdrald/src/uia.rs` | I/R: window/tree/ref semantics and invoke/value/select/toggle/scroll via host, local Full User checks as applicable; T: native fixtures | Typed element/generation identities avoid visual click inference | Protected/password/stale/foreign denied; local-only. Cloud-proposal adapter not qualified; 3–6 days scoped AI binding, not UIA rewrite |
| Observation/capture: UIA adapter and `apps/deskal-computer-host` | I/R conditional host: exact-window capture and bounded frames; T: frame/native tests | AX tree first, hash/generation/geometry checks, relevant capture | Capture approval and protected surfaces; egress consent/redaction/resize mapping missing for Pro; 2–5 days |
| Computer Host/input: `qdral-provider-uia/src/computer_host.rs`, `qdrald/src/uia.rs` | I/R conditional configured Go host: local lease-gated cursor/input/window actions; T: native SG094/95; C: remote Full Control | Reuse private protocol, interruption and exact targets | Host is not guaranteed in current published artifact; no model or hosted principal obtains Full User. Packaging/Pro integration 3–6 days |
| Browser worker/security: `qdral-browser-host/src/worker_launch.rs`, `worker_navigation.rs`, `apps/qdral-browser-worker` | I/T: substantial live engine helpers and native probes; R: no demonstrated product path through shipping daemon; C: old provider templates and new exposure integration | Typed navigation/DOM, WebMCP/extract/map/crawl/robots can remove many model steps | Main daemon lacks browser-host dependency; host main has hello/ping/shutdown path; SG108/A10/#323 still open. Personal profiles/remote/generic eval absent. Wiring/qualification 6–12 days estimate, ongoing work preserved |
| Filesystem: `qdral-provider-fs/src/lib.rs`, `mutation.rs`, `qdrald/src/fs_mutation.rs`, MCP `fs_mutation.ts` | I/R: bounded trusted-workspace reads/find/mkdir/move/edit/remove; T: approval+postcondition tests | Native file operations, hashes and checkers eliminate UI/model actions | Real Downloads must be trusted; metadata/date adapter absent, no recursive arbitrary delete or overwrites. Recipe/metadata/UI 3–5 days |
| Process/terminal: `qdral-provider-process`, SG096 and MCP `process.ts` | I/R: registered/hash-bound argv-only Safe process; T: native containment; C: unarmed Full User interactive shell | Qualified exact functions avoid model-generated shell | No free-form shell/stdin/session feature today; SG096 T02–T05 remain. Job Object is supervision, not arbitrary-user filesystem/network sandbox; 8–15 days remaining estimate, not required for file pilot |
| Git: `qdral-provider-git`, daemon git modules, MCP git modules | I/R: status/diff/log/local mutations under policy; fetch/push need configured destination policy; T: identity/drift tests | Exact Git functions avoid terminal and inference | Installed destination config can fail closed; no blanket authenticated push/force/delete. Not needed for first Pro; 1–3 days UX exposure |
| MCP: `apps/qdral-mcp/src/server.ts`, `tool_contract.ts`, `kernel.ts` | I/R: 40 tool entries, 26 remote core/14 local-only desktop profile, stdio/loopback/relay; T: provider parity/OAuth tests | Schema/scopes/discovery/annotations reused; no edge rewrite | Discovery is not provider readiness; hosted Pro cannot inherit local-only profile. Contract/proposal binding 2–4 days |
| Relay/communication: `apps/qdral-relay`, MCP uplink/remote leases | I/R: self-hostable OAuth/pairing/outbound routing/container; T: fixture E2E and chaos; C: founder production host and Pro proposal protocol | Existing bounded sessions/route/device identity instead of new mesh | Remote core only; TLS not relay-blind E2E; no queued offline mutations. Tenant/usage/cloud contract 4–8 days |
| Windows app: `apps/aladdin-desktop-preview/src/Program.cs` | I/R preview: status/version/doctor/devices/files/help and truthful unavailable AI; T: self-test/UIA E2E; C: execution/chat/events/API | WPF shell, async bounded I/O, UIA ids, command router | AI, pairing and mutation controls not connected; #288 memory open. Compact task/chat view/API 5–10 days |
| Tasks/receipts: audit/envelopes/provider results and proposed adopted task record | I/R: operation/audit evidence; T: provider checks; C: persistent cross-step task timeline/reconciliation | Hash/state-based verifier instead of model "done" | No general native orchestrator. Unknown outcome/idempotency/cancel/admission must be built; 4–7 days |
| Installer/updates: `qdral-lifecycle`, packaging/release workflows | I/R: per-user lifecycle/doctor/rollback; T: extracted archive/reproducibility/SBOM CI; C: unified WPF/Go/worker/Node release | Preserve one governed update path; do not ship weights locally | Current published historical zip differs from main; Node is an external prerequisite, no signing certificate claim. Unified bundle/qualification 4–8 days plus signing decision |
| User/device/session identity: MCP `device_identity.ts`, `oauth_authorization.ts`, daemon `remote_lease.rs`, policy stores | I/R: device keys/epochs/pairing/PKCE/revocation/remote leases; T: isolation/replay tests; C: commercial entitlements and native Pro sessions | Reuse identity, short-lived sessions and proof, not new auth framework | Subscription grants no local trust/privilege. Entitlement/offline grace/provider secrets/legal not implemented; 3–6 days after decisions |

A production-reachable FS operation was tested through the selected daemon's
existing test dispatch, not a customer installer or real Windows Hello flow.
Do not add all effort rows: shared API/UI/task integration overlaps. For the
critical path and native acceptance see [roadmap](AI_FIRST_PROTOTYPE.md).

## Basic product specification: $3, local and independently useful

Basic includes no native model inference. Its paid value must be reliable
Windows controls, packaged updates, transparent approvals/history and useful
repeatable tasks; a bare MCP server alone competes with free tools.

| Feature | Today at baseline | Basic launch requirement |
|---|---|---|
| File management | Kernel/MCP reads and approved bounded mutations; preview lists files | Native forms for trusted workspace list/find/read/create/edit/move/remove, exact previews and receipt; organizer/report recipe only after qualification |
| Terminal/process | Safe registered argv execution; arbitrary shell unarmed | Registered process form first; Full User terminal/session only after SG096 closeout, never marketed now |
| Desktop | Native observations/approved actions conditionally available through local MCP/host | Inspect window/element and explicitly select qualified action; local lease and broker preserved; no AI needed to choose a named action |
| Browser | Helpers implemented, product path/closeout missing | Isolated browser journey only once SG108/exposure/packaging pass; no personal signed-in profile promise |
| MCP/workspaces | Current interfaces and trust/identity contracts | Optional external-agent setup, command/task controls usable without it; explicit workspace/root selection |
| Security/cancellation/history | Existing operation audit, approval/revoke and worker primitives; app timeline not connected | Real status, local authority-reducing stop, task history with verified/partial/unknown, independent checker and safe resume |
| Devices/sessions | Identity/pairing/relay exist; app lists local state | One local device launch; supported pairing/revocation UI later, optional relay. No mandatory hosted relay for local operation |

Basic's preview works for six read-only command families today without AI.
The complete paid product is **not built**: mutation/task controls and unified
installer still need work. No natural-language planner is promised for Basic.
Use deterministic forms/templates rather than simulated chat pretending to
understand arbitrary instructions. Offline local execution remains available
during AI outage; entitlement caching/grace must be explicitly designed so
license-server unavailability does not unexpectedly disable local controls.
Auth/licensing, support, signing/updates/downloads, refunds/tax, optional relay
and maintenance remain costs even with zero GPU.

## Pro product specification and text-first UI

Pro includes Basic plus Aladdin-owned bilingual text conversation. Customer
installs one Aladdin product, signs in to Aladdin and receives native Hala
service when entitled; future purchase/account implementation is separate.
The user says "Organize my Downloads folder by file type and date"; Aladdin
resolves scope/date ambiguity, proposes a bounded plan, obtains exact local
approval, executes native FS operations, verifies hashes/locations and reports
actual changes. A response consisting only of instructions fails the product goal.

Use the existing WPF conversation/command surface and add a compact task view,
target device/workspace selector, plan, approval status, sequenced actual event
timeline, local stop, verified receipts and error/unknown reconciliation. Reuse
kernel-owned approval presentation; do not mint UI approval tokens. Events
recover from gaps by a bounded status snapshot. Show queued/cold/working/
awaiting approval/verified/partial/unknown/offline distinctly. Allow cancel in
every phase; a cloud outage cannot hide the local stop or turn unknown into failure.

Launch support is restricted to verified file organization, bounded extraction/
report templates and qualified observation/action combinations. Reading an
arbitrary PDF or writing a formatted spreadsheet requires an actual parser or
qualified UI workflow, not a claimed capability inferred from FS bytes. Existing
web extraction helps only when wired/qualified. Task memory starts with local
bounded task state and explicit user preferences, no hidden server training or
unbounded cross-task screenshots. Voice, broader spreadsheet control, general
desktop navigation, long-horizon recovery and multi-device orchestration are
later independently evaluated scopes, not release prerequisites.

UX acceptance: keyboard/Narrator, 100/150/200% DPI, high contrast, long English/
Arabic text and RTL, responsiveness during stalled process/huge folders,
honest unavailable capability controls, stop while cloud disconnects, and
native final-state proof. No large UI rewrite, Electron bundle, new auth platform,
new relay mesh or external chatbot is needed by this design.

## Architecture options and selection

All task quality/latency comparisons are hypotheses until the paired protocol.
Every option uses the same local authority; a bigger model cannot buy more privilege.

| Option | Expected task quality / Arabic | Latency / residency / founder cost | Engineering, maintenance, UX, scale | Decision |
|---|---|---|---|---|
| A compact Hala + tool-first | Promising narrow templates; arbitrary GUI/Arabic unproved | 4B short-context residency estimate 12–18GiB; scale-zero cold delay, low idle floor | Smallest model/runtime ownership, structured verification, shared bounded workers | **Best launch experiment** |
| B large Hala always-on | Stronger candidate, not demonstrated across Windows/Arabic | Holo4 ~70GB BF16 weights; large warm floor and startup | Fewer model variants but expensive idle/overprovision; no safety improvement by itself | Reject launch commitment |
| C compact + selective large | Potential hard-task recovery benefit; needs bilingual paired proof | Both attempts, separate storage/cold/queue; caps needed | Router/error/debug/tenant budgeting more complex; no customer-visible second app | **Future upgrade**, off initially |
| D Brain/Eye specialists | Potential specialization, no current task evidence | Two residency states/adapters and often extra invocation | Training/switching/cache/coordination cost and more failures | Reject initial complexity |
| E confirmed deterministic recipes + small eye on demand | High known-recipe reliability, restricted free-text scope | Zero model work for recipes; small visual requests as needed | Cheapest fallback; scope must be visible, cannot claim general native intelligence | **Cheapest credible restricted fallback** |

Winner objective: minimum **cost per independently verified successful task**
subject to strict authority, privacy and useful latency/coverage gates. Reverse
A if the small model fails bilingual task quality despite a qualified harness,
or if its retries/startup make a stronger model cheaper per success. Activate C
only on measured incremental benefit; see [specific reversal tests](AI_MODEL_SELECTION.md).

## Competitor/product research

Only public claims are compared; internal competitor GPU economics and equal
workload reliability are unknown. Current sources are in the evidence register.

| Product | Deployment / AI dependence / price | Integration, privacy and UX comparison |
|---|---|---|
| Desktop Commander | Local MCP free/no account/no monthly limit; hosted connector Free 10,000 calls, Pro $20/month unlimited tool calls; connects external AI clients | Strong file/process integration and client ecosystem. A tool-call plan is not bundled native inference. Aladdin Basic must justify $3 through standalone UI/safety/packaging, Pro through integrated Hala; no unsupported claim it lacks safety |
| Claude computer use | Hosted proprietary inference; customer application runs client tools in its controlled environment; API costs separate from Claude Pro ($20 US/month) | Screenshot/input and current browser toolsets, prompt-injection protections and sandbox guidance. Pro subscription is not a license for unlimited API reselling. Aladdin's architecture owns open weights and local approvals; relative task quality not tested |
| OpenAI ChatGPT agent / computer use | ChatGPT Plus $20/month and other eligible plans with limits; hosted agent and connected/local work vary by product; API computer use separately priced | Mature integrated task experience; local execution can still send task data to model services. Do not claim all OpenAI agents are browser-only. Aladdin targets standalone Windows/native authority and price transparency; superiority unknown |
| Browser Use | Local open-source agent/harness or hosted browser/agent; current cloud browser headline $0.02/browser-hour plus model usage, pay-as-you-go | DOM/AX, browser sessions, providers and MCP useful reference. Browser tariff excludes inference/control cost and does not prove Windows desktop support. Do not import personal-cookie/eval authority |
| Microsoft Power Automate | Desktop/RPA flows, cloud connectors; Premium $15/user/month paid yearly US marketing price, other plans and region variation | Deterministic workflows are real competitors to Basic and demonstrate model-free value; recorder/configuration effort trades off against general free text. Enterprise integration and actual privacy depend on deployment |
| AutoIt / local GUI scripts | Free Windows scripting/automation, no native hosted AI required | Powerful deterministic desktop workflows; script authoring and validation are user work. Aladdin competes on reviewed task UI, trustworthy execution and convenience, not raw availability of clicks |

Credible differentiation hypothesis: one native Windows product, optional MCP
compatibility, explicit local consent/identity/freshness, bilingual native AI,
structured operations and honest independent receipts. The commercial risk is
real: free local MCP and existing RPA may already solve Basic's tasks. Validate
with 10–20 target-user interviews and an unpaid controlled usability pilot after
product scope approval; this study sends no messages or sells anything.

## Final founder questions: explicit answers

1. **Basic at $3:** qualified local files/registered processes/desktop controls,
   deterministic recipes, approvals/history/cancel, optional MCP/relay; browser only
   after live wiring/qualification. No bundled AI.
2. **Works today:** FS/kernel/MCP/identity/lifecycle and conditional local desktop
   primitives; WPF six read-only commands. Native task UI/general shell/live product
   browser/integrated Pro are not complete.
3. **Independent Basic:** yes by design; read-only preview already needs no AI.
   Useful complete mutation workflows still need standalone app integration.
4. **Basic costs:** payment/refunds/tax, support, auth/license service, downloads/
   signing/update security, optional relay, maintenance; zero native GPU cost.
5. **Native Pro experience:** goal→bounded plan→local approval→real tools→independent
   state verification→honest report/recovery in Aladdin's own text UI.
6. **First checkpoint:** Qwen3.5-4B at the immutable pin in model selection.
7. **2B/4B sufficient:** unproven; plausible for narrow tool-first tasks. Test 4B
   then 2B on identical bilingual native tasks, not just grounding.
8. **Holo4 worth cost:** unproven; baseline only, selective use after marginal
   cost/success evidence; never mandatory always-on by assumption.
9. **Brain/Eye:** one multimodal model first, separate roles in prompts/contracts.
10. **Fine-tuning before experiments:** no; unchanged models and harness ablations first.
11. **Minimize calls:** exact recipes, direct tools/UIA/DOM, bounded context/output,
    event waits and deterministic verifiers; cache only safe unchanged advisory state.
12. **Existing replacements:** FS/read/find/mkdir/move/edit, Git, registered process,
    UIA semantics, bounded observation; future qualified WebMCP/extract/map/crawl.
13. **Cheapest practical hosting:** compact scale-zero 24GB founder model pool,
    tested against Modal/Vast/pods; actual cheapest cost/success remains unmeasured.
14. **Cost/success:** not measured. Capped illustrative operating $0.072–$0.037
    across 10–1,000 accounts, including support but excluding fees/tax/maintenance.
15. **10/100/1,000 AI users:** capped illustrative GPU $7.82/$78.24/$782.45 and
    operating $37.39/$208.34/$1,917.79 monthly; add maintenance/payment/legal.
    Uncapped GPU $64.22/$642.22/$6,422.24; workload assumptions govern all values.
16. **Sustainable Pro range:** test $10–$15 bounded beta; $8 challenger after
    measured value/margin; no fully costed promise at ten users or infinite usage.
17. **Heavy-user protection:** admission reservations, predictable credits, per-task/
    account/global budgets, call/image/context/retry/concurrency/worker caps.
18. **No outside customer AI accounts:** Aladdin login/entitlement reaches
    founder model service; customer holds only local device/session authority.
19. **Privacy:** minimized consented observations, protected-screen denial,
    default no content persistence/training, TLS/encrypted storage/tenant isolation,
    DPA/retention/region/deletion qualification before launch.
20. **Sole local authority:** Hala proposes; local kernel validates identity/
    freshness/scope/approval/lease, dispatches, interrupts and verifies. Hosted
    traffic cannot masquerade as local-only control without a reviewed contract.
21. **Smallest meaningful prototype:** synthetic file organizer with mock/deterministic
    control, real kernel approvals/postconditions, then one bounded model intent.
22. **Validation:** owned native Windows/browser/RTL tasks, permission/injection/
    cancel/outage/unknown outcomes, frozen paired variants and artifact checkers;
    public benchmarks are supplements, never training data.
23. **Implement first:** authenticated native task API/FS controls and deterministic
    control loop, then one cheap bilingual hosted model feasibility experiment.
24. **Separate approvals:** runtime amendment/new grains, actual spending/model
    grants/data/training, privacy/legal/retention/signing, entitlement/billing/price,
    production release, any privilege or remote profile expansion.
25. **Shortest credible route:** preserve the current kernel/UI/MCP/browser work,
    qualify one verified native task, test unchanged compact Hala, measure cost/
    latency/success, integrate bounded text Pro, then broaden only passing scopes.

## Spending, acceptance and handoff

No paid compute, billing, training, model weights or privileged implementation
has been started. The first real-model <=$5 proposal is explicit in the prototype
document. Required CI's Jev job can consume paid credits on PR creation; the
no-spending directive requires a narrow exception before opening that PR.
Local validation and review can proceed without bypassing or altering the gate.
Planning commits do not close SG096/108 or claim canonical AI completion.

Follow the evidence register for actual tests/review versus prospective gates.
Before merge recheck exact head/base, required contexts and manual exclusions;
normal expected-head merge only when legitimately authorized and qualified.
Verify real main and post-merge CI. If spending/review approval is unavailable,
preserve signed commits and the concrete PR body; report publication/merge
pending instead of fabricating a green or merged result. Resume instructions
are in [first prototype](AI_FIRST_PROTOTYPE.md).
