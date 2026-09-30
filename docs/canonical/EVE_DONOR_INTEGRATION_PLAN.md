# Cotra Eve Donor Integration Plan

Status: IMPLEMENTATION-READY PLANNING
Planning date: 2026-09-30
Cotra planning base: ffaad8ad0d1ee189c7a41dc058ef6f7165e41e28
Eve donor repository: https://github.com/vercel/eve
Eve donor pin reviewed: 9c36b7c280fda89ae678cabfd8d906f4bde2216f
Eve license at reviewed pin: Apache-2.0
Cotra license: Apache-2.0
Founder-funded recurring infrastructure cost target: NONE

## 1. Decision

Cotra will use Eve as an authorized design and source donor, but Cotra will **not** embed Eve as its privileged runtime and will **not** turn Cotra into a general-purpose agent framework.

The architectural decision is:

```text
Agent / ChatGPT / future Kaf-style orchestrator
    -> MCP or another explicitly supported client adapter
    -> cotra-mcp
    -> cotrad policy and authority kernel
    -> approval/trust boundary
    -> typed providers
    -> evidence/audit
```

Eve-derived concepts may improve orchestration contracts, durability, capability packaging, evaluation, recovery, and test architecture, but they must not bypass the Cotra policy kernel.

The model, an agent harness, a skill, a subagent, an evaluator, a connection, a workflow engine, or a sandbox never receives authority merely because it can describe or request a tool.

Cotra remains the authority boundary.

## 2. Why Eve is useful

The reviewed Eve architecture provides several mature patterns that are directly useful to Cotra:

1. durable session/turn/step checkpointing and explicit replay semantics;
2. context minimization through separate instructions, skills, tools, workspace files, and subagents;
3. declared specialist isolation with separate tool, skill, sandbox, state, and connection surfaces;
4. dynamic capability discovery while still requiring execution-time checks;
5. explicit human approval and parked-work semantics;
6. durable callbacks and restart-safe reconstruction of tool execution state;
7. typed evaluation and judge infrastructure, including current support for TypeSafe Jev;
8. separation of trusted runtime secrets from sandbox/model-controlled processes;
9. explicit connection credential handling without serializing credentials into durable workflow state;
10. deterministic mock-based E2E infrastructure alongside real integration qualification.

These are valuable patterns. They are not automatically safe to transplant into Cotra unchanged.

## 3. Critical non-adoption decision

Eve's trusted app runtime has full Node.js access and unrestricted network access by design. That is appropriate for Eve's framework role, but it conflicts with Cotra's security objective.

Cotra therefore MUST NOT adopt the following as privileged-runtime defaults:

- unrestricted Node.js or JavaScript execution inside the authority kernel;
- a generic `bash`, shell, or command tool as the primary model-facing primitive;
- unrestricted app-runtime networking;
- `allow-all` sandbox egress;
- generic MCP/OpenAPI connection execution that bypasses Cotra destination policy;
- ambient credential injection into arbitrary model-selected requests;
- dynamic capability visibility as an authorization decision;
- subagent inheritance as an authority boundary;
- remote-agent routing as an implicit trust relationship;
- automatic replay of non-idempotent side effects after an ambiguous crash;
- cloud-required durability, sandboxing, telemetry, or workflow state;
- Vercel AI Gateway, Vercel Workflow, Vercel Sandbox, or Vercel-hosted infrastructure as a Cotra runtime dependency.

This plan borrows useful mechanisms while preserving Cotra's narrower trust model.

## 4. Product-boundary rule

Cotra and an Eve-derived agent runtime solve different problems.

Cotra owns:

- authority;
- policy;
- trust state;
- approvals;
- provider ceilings;
- Windows identities;
- filesystem/process/browser/UI/network safety;
- execution evidence;
- local audit;
- lifecycle security.

An agent runtime may own:

- prompts/instructions;
- skills;
- conversation history;
- task planning;
- model selection;
- subagents;
- scheduling;
- long-running reasoning;
- context compaction;
- application-level memory.

No agent-runtime feature may silently migrate into Cotra's authority boundary.

## 5. Eve donor provenance policy

Every direct code reuse from Eve MUST record:

- donor repository;
- donor commit SHA;
- donor file path;
- donor license;
- whether the code was copied, translated, or conceptually reimplemented;
- Cotra destination path;
- modifications made;
- security review result.

For the reviewed baseline:

```text
DONOR_REPOSITORY = vercel/eve
DONOR_SHA = 9c36b7c280fda89ae678cabfd8d906f4bde2216f
DONOR_LICENSE = Apache-2.0
```

Eve includes a NOTICE file. Any direct source reuse must preserve all legally required notices and attributions. Cotra MUST add or update a repository-level third-party notice/provenance artifact before release if any Eve source code, rather than only ideas, enters Cotra.

No Vercel trademark, branding, product identity, or implication of endorsement is inherited.

## 6. Adopt / adapt / reject matrix

| Eve capability or pattern | Cotra decision | Cotra use |
| --- | --- | --- |
| Durable sessions / steps | ADAPT | operation recovery and lifecycle state, not model conversation storage |
| Checkpoint/replay semantics | ADAPT | explicit idempotency and ambiguous-outcome handling |
| Context control | ADAPT | capability manifest minimization and client guidance |
| Skills | CLIENT-SIDE ONLY | optional orchestrator procedures; never authority |
| Subagents | CLIENT-SIDE ONLY | supported as independent callers; no implicit authority inheritance |
| Dynamic capabilities | ADAPT | visibility/manifest generation only; execution always re-authorized |
| Tool approvals | ADAPT | UX/event ideas only; Cotra nonce/digest/expiry/one-shot rules remain authoritative |
| Parked work | ADAPT | approval/lifecycle waiting without replaying effects |
| Sandbox isolation | CONCEPT ONLY | reinforce process separation; do not add generic shell sandbox in v1 |
| Connection credentials | CONCEPT ONLY | future typed credential brokering; no generic connection bypass |
| MCP/OpenAPI discovery | REJECT FOR PRIVILEGED EXECUTION | any external integration must map to Cotra policy/provider authority |
| Typed evals | ADAPT | P13 security/evidence/eval harness |
| Jev evaluation | ADAPT, ADVISORY ONLY | semantic evaluation/review; never authorization or approval issuance |
| Workflow callbacks | ADAPT | restart-safe callbacks/state reconstruction where required |
| Schedules | OUT OF COTRA V1 | orchestrator concern |
| Channels | OUT OF COTRA V1 | client/orchestrator concern |
| Remote agents | OUT OF COTRA V1 | no new trust boundary in release scope |
| Full Node app runtime | REJECT | expands TCB too far |
| Unrestricted runtime network | REJECT | conflicts with destination-scoped P11 policy |
| Default telemetry/export | REJECT | Cotra audit remains local by default |

## 7. New invariant: visibility is not authority

Eve demonstrates useful runtime capability discovery. Cotra will preserve a stricter rule:

```text
MODEL_VISIBLE_CAPABILITY != AUTHORIZED_CAPABILITY
```

A tool may be visible to a client and still be denied by cotrad.

A dynamically selected tool, skill, subagent, connection, or workflow may request an operation, but authority exists only after:

1. request normalization;
2. workspace validation;
3. exact capability/operation matching;
4. current policy revision validation;
5. current trust validation;
6. target/state revalidation;
7. required approval;
8. exact provider ceiling validation;
9. postcondition/evidence rules.

This invariant must appear in architecture and release security tests.

## 8. New invariant: evaluator is not authority

Cotra already requires genuine TypeSafe Jev for governed review. Eve's eval architecture confirms that evaluation models are useful for typed judgement.

Cotra MUST preserve:

```text
EVALUATOR -> SCORE / CLASSIFY / RECOMMEND
EVALUATOR -X-> APPROVE / ISSUE TOKEN / GRANT AUTHORITY
```

Jev may:

- review a diff;
- classify risk;
- assess evidence quality;
- evaluate tool descriptions;
- identify likely unsafe widening;
- recommend that stronger approval is required.

Jev MUST NOT:

- mint approval tokens;
- satisfy STRONG presence;
- change workspace trust;
- override hard denials;
- widen provider ceilings;
- convert an unqualified result into canonical evidence by itself.

Deterministic tests and concrete platform evidence remain canonical where the requirement is deterministic or platform-specific.

## 9. New invariant: delegation is not authorization

A client may use subagents, including a future Eve/Kaf-style runtime.

Cotra treats each resulting privileged request as an independent request that must pass the same kernel authorization path.

A parent agent cannot transfer authority simply by telling a child that it is authorized.

A child does not inherit:

- approval tokens;
- input leases;
- trust-change rights;
- secret references;
- network destination authority;
- filesystem capability outside the current workspace;
- browser/profile authority;
- Windows element/frame/coordinate identities.

Any future lineage metadata is audit/context only unless separately authenticated and explicitly included in policy. No lineage field is required for Cotra v1.

## 10. Durable operation model for Cotra

Cotra will borrow Eve's explicit checkpoint/replay discipline, but apply it to privileged operations rather than model turns.

A future P12 grain should establish a local durable operation journal with a minimal record similar to:

```text
operation_id
request_id
client_session_id
workspace_id
capability
operation
normalized_target_digest
policy_revision
workspace_revision
approval_reference_or_digest
provider
idempotency_class
state
attempt
started_at
finished_at
evidence_reference
result_digest
```

No raw secret value belongs in this journal.

### 10.1 Required operation states

At minimum:

```text
PREPARED
APPROVAL_PENDING
AUTHORIZED
EXECUTING
COMMITTED
FAILED
ABORTED
OUTCOME_UNKNOWN
```

State transitions must be monotonic or explicitly versioned. Recovery cannot pretend an `EXECUTING` operation succeeded after a crash without evidence.

### 10.2 Idempotency classes

Each privileged operation must have an explicit replay class:

```text
READ_REPLAYABLE
IDEMPOTENT_MUTATION
NON_IDEMPOTENT_MUTATION
EXTERNAL_SIDE_EFFECT
```

Examples:

- bounded read: usually `READ_REPLAYABLE` after fresh revalidation;
- setting a known state with postcondition: may be `IDEMPOTENT_MUTATION`;
- one-time external action: `NON_IDEMPOTENT_MUTATION` or `EXTERNAL_SIDE_EFFECT`.

### 10.3 Crash recovery law

After restart:

- `COMMITTED` is never executed again;
- `FAILED` is not silently retried;
- `APPROVAL_PENDING` is invalidated unless the exact canonical approval design explicitly supports safe restoration;
- consumed approvals remain consumed;
- expired approvals remain expired;
- input leases are invalidated;
- stale Windows identities are invalidated;
- non-idempotent `EXECUTING` operations with no postcondition proof become `OUTCOME_UNKNOWN` and require reconciliation rather than blind replay;
- replayable reads require fresh target/policy validation before retry;
- idempotent mutation retry requires exact operation identity and a postcondition showing replay cannot duplicate an effect.

This closes a major lifecycle gap without importing Eve's whole workflow runtime.

## 11. Parked approval and waiting semantics

Eve's parked-work model is useful, but Cotra must remain fail-closed.

For Cotra:

- waiting for approval must hold no privileged execution lease;
- waiting must not hold an input lease;
- target-sensitive state must be revalidated after the wait;
- approval digest remains bound to exact request/policy/workspace state;
- a process restart may invalidate the pending approval rather than recover it if safe continuity cannot be proven;
- no tool effect may occur before approval is successfully consumed;
- no approval may authorize a second execution after an ambiguous retry.

P12 recovery tests must cover these rules.

## 12. Capability manifest and context minimization

Borrow Eve's context-control principle: expose only the narrowest information necessary.

P13 should add a generated or verified Cotra capability manifest derived from actual registered MCP/kernel capability contracts.

Each capability descriptor should include only non-secret metadata such as:

```text
capability
operation
effect_level
approval_class
provider
provider_ceiling
network_class
replay_class
schema_digest
policy_revision_or_manifest_revision
```

The manifest is descriptive, not authoritative.

Release tests must prove:

1. every model-visible MCP tool maps to a known Cotra capability;
2. every state-changing capability maps to a kernel policy path;
3. no MCP tool can bypass cotrad;
4. no orphan model-visible action exists;
5. no privileged provider operation exists through a generic model-facing shell;
6. tool descriptions do not claim broader authority than the kernel grants;
7. disabled capabilities may remain absent from the client surface without changing kernel hard denials.

## 13. Skills and instructions boundary

Skills and instructions are useful for clients but do not belong in Cotra's privileged kernel.

Cotra may ship client-facing operator procedures or example skills that explain how to use typed tools safely. Such artifacts:

- carry no approval authority;
- cannot modify kernel policy;
- cannot create new providers;
- cannot bypass workspace rules;
- cannot expand network egress;
- cannot self-approve;
- are treated as untrusted procedural text from the kernel's perspective.

A future Kaf/Eve-style orchestrator may package these procedures using Eve-like skill conventions while still calling Cotra through the normal bounded interfaces.

## 14. Connections and credentials boundary

Eve's connection design is valuable as a reference for keeping credentials outside model-visible state, but Cotra P11 remains intentionally narrower.

For Cotra v1:

- `network/fetch` remains unauthenticated, destination-scoped, bounded HTTPS GET unless later canonical grains explicitly add authority;
- no generic OAuth/MCP/OpenAPI connection broker is added to P11;
- no caller-provided `Authorization`, `Cookie`, `Proxy-Authorization`, or arbitrary credential header is accepted by the bounded fetch grain;
- no agent runtime may inject a credential into Cotra's generic network provider.

If a future authenticated integration is authorized, it must use a typed provider or typed credential broker with:

- opaque secret reference IDs;
- destination binding;
- account/principal binding;
- minimum scopes;
- no raw secret return to model/sandbox;
- no durable serialization of token bytes;
- separate approval/policy if the action changes external state;
- independent revocation.

This is explicitly outside SG-000040 and must not delay or widen that grain.

## 15. Sandbox decision

Cotra does not add Eve's general shell sandbox to the v1 authority path.

Reason:

- Cotra already has typed process/file/browser/UI providers;
- adding a general model-facing command sandbox would expand attack surface and duplicate the client/orchestrator role;
- Windows local-machine authority is not equivalent to a disposable remote sandbox.

However, Eve's trust split reinforces Cotra's existing process-separation direction:

- secrets remain in trusted Cotra processes;
- model-controlled child processes do not inherit protected environment variables;
- child processes do not receive approval IPC credentials;
- child processes cannot call cotrad privileged IPC merely because they were spawned by Cotra;
- filesystem/process isolation must be explicit and must not be described as stronger than the underlying Windows primitives provide.

## 16. P11 integration rule

COTRA-P11 is already active. Eve adoption MUST NOT modify the active SG-000040 scope.

SG-000040 remains:

- one destination-scoped bounded HTTPS GET;
- no credentials;
- no generic sockets;
- no POST/write methods;
- no standing sessions;
- no automatic authority widening.

After SG-000040 closes, rebuild the live P11 exit matrix. Eve-derived work enters only if it is already required by P11's canonical exit criteria. Otherwise defer it to P12/P13 as specified below.

## 17. P12 revised implementation plan — installer, lifecycle, durability

P12 keeps its existing installer/lifecycle mission and adds explicit Eve-derived durability/recovery requirements.

### P12-A — lifecycle state and operation journal

Derive a narrow grain after P11 exits to implement:

- local durable operation journal;
- replay/idempotency classification;
- monotonic lifecycle states;
- crash recovery semantics;
- stale approval/input lease invalidation;
- evidence linkage;
- secret-free persistence.

Acceptance:

- deterministic restart/recovery tests;
- no blind replay of unknown non-idempotent effects;
- no consumed approval reuse;
- no stale input lease reuse;
- corrupted journal fails closed or enters explicit recovery mode;
- bounded retention/cleanup policy documented.

### P12-B — per-user installation and bootstrap

Implement/qualify:

- reproducible release/install plan;
- per-user installation first;
- cotrad installation;
- cotra-mcp installation;
- cotra-session/cotra-approve components when present in live architecture;
- OpenAI Secure MCP Tunnel setup assistant/supervision where canonical;
- first-run initialization;
- local state directory creation with correct ACLs;
- `start`, `stop`, `status`, and `doctor` paths;
- no accidental Administrator requirement.

Acceptance:

- real clean-install Windows qualification;
- exact installed-file manifest;
- first start and restart;
- no secret in logs/config dumps;
- no unauthorized listener;
- no paid hosted infrastructure requirement.

### P12-C — update, failure, recovery, rollback

Implement/qualify:

- version transition state machine;
- artifact integrity/provenance checks;
- safe stop/start boundaries;
- controlled failed-update injection;
- rollback or documented last-known-good recovery where required;
- operation-journal compatibility across supported updates;
- no replay of pre-update side effects;
- pending approvals invalidated or safely rebound according to canonical policy.

Acceptance:

- real update test from a supported prior version;
- failed update leaves no authority-bearing partial install;
- recovery returns to a known version/state;
- policy/schema incompatibility fails before privileged execution resumes.

### P12-D — uninstall and reinstall

Implement/qualify:

- uninstall;
- startup/task/service cleanup if any exist;
- tunnel/client configuration cleanup where owned by Cotra;
- explicit retained-data choices;
- credential removal or explicit retention policy;
- operation-journal handling;
- reinstall from supported post-uninstall state.

Acceptance:

- real Windows uninstall;
- no orphan privileged process or startup entry;
- no undocumented retained secret;
- reinstall works or fails with a documented recoverable reason.

### P12 exit matrix

P12 may exit only when real evidence exists for:

```text
CLEAN_INSTALL
FIRST_RUN
START
STOP
STATUS
DOCTOR
RESTART
UPDATE
FAILED_UPDATE
RECOVERY
ROLLBACK where required
UNINSTALL
REINSTALL where required
DURABLE_OPERATION_RECOVERY
APPROVAL_INVALIDATION_ON_RESTART
NO_BLIND_NON_IDEMPOTENT_REPLAY
```

## 18. P13 revised implementation plan — release hardening and Eve-derived assurance

P13 keeps its current release-hardening mission and adds the following donor-derived assurance work.

### P13-A — capability surface manifest

Implement/qualify:

- actual MCP-to-kernel capability mapping;
- schema digests;
- effect level and approval class metadata;
- provider ceiling metadata;
- replay class metadata;
- generated/verified release manifest.

Acceptance:

- every model-visible tool has one bounded kernel mapping;
- no model-facing tool has broader wording than implemented authority;
- no privileged operation is reachable only through undocumented/generic execution;
- manifest mismatch fails release qualification.

### P13-B — deterministic scenario/eval harness

Borrow Eve's eval structure, not its authority model.

Build a reusable scenario harness with:

- typed scenario IDs;
- input fixture;
- expected decision/result class;
- expected evidence fields;
- forbidden evidence fields;
- deterministic assertions;
- optional Jev semantic evaluation;
- reporter output suitable for release evidence.

Required scenario families include:

- authority widening;
- stale identity/state;
- approval replay;
- protected Cotra surfaces;
- filesystem escape;
- process/shell ceiling;
- browser origin/SSRF;
- UIA stale target;
- screenshot/frame staleness;
- coordinate/input lease;
- human interruption;
- clipboard secret denial;
- network SSRF/rebinding/redirects;
- lifecycle crash/recovery;
- installer privilege;
- update rollback;
- secret redaction.

Jev may add semantic findings but cannot replace deterministic assertions.

### P13-C — orchestrator compatibility tests

Cotra must prove that a sophisticated client can use it without obtaining implicit authority.

Test at least:

1. normal ChatGPT/MCP path;
2. multi-step client session with repeated bounded operations;
3. simulated parent/child agent callers using distinct client-session identities;
4. stale/forged client-session metadata does not bypass kernel checks;
5. client tool visibility changes do not change kernel policy;
6. a client-generated skill/instruction cannot bypass approval;
7. an evaluator recommendation cannot satisfy approval;
8. a client cannot use generic network fetch to smuggle credentials.

A real Kaf/Eve-derived client integration may be added as an optional interoperability test when available, but Cotra release MUST NOT depend on a paid or hosted Eve/Vercel service.

### P13-D — donor provenance, license, and SBOM

Before final release:

- inventory all copied/adapted Eve source;
- pin donor SHAs;
- preserve Apache-2.0 notices;
- include Eve NOTICE content as legally required if source was copied;
- record modifications;
- include donor-derived dependencies in SBOM;
- verify no accidental Vercel-only runtime dependency;
- verify no unwanted telemetry/export dependency;
- verify zero founder-funded recurring infrastructure requirement.

### P13-E — final Windows and E2E release qualification

Run the final canonical path:

```text
ChatGPT
-> OpenAI Secure MCP Tunnel
-> cotra-mcp
-> cotrad
-> typed providers
-> approval/trust
-> evidence/audit
```

Where a local orchestrator compatibility harness is available, also run:

```text
Orchestrator / Kaf-style client
-> cotra-mcp
-> cotrad
-> same authority kernel
-> same approvals
-> same typed providers
-> same evidence
```

The second path is proof of client independence, not a separate authority path.

## 19. Replay-safety test matrix

P12/P13 must include explicit replay tests.

| Scenario | Required result |
| --- | --- |
| crash before approval | no effect; fresh request required |
| crash after approval but before consume | approval only reusable if canonical token state proves it is still valid and unused; otherwise deny |
| crash after consume before provider call | do not mint a new approval automatically |
| crash during replayable read | retry only after fresh state/policy validation |
| crash during idempotent mutation | retry only with exact operation identity and safe postcondition |
| crash during non-idempotent mutation | `OUTCOME_UNKNOWN` unless external evidence proves result |
| restart with active input lease | lease invalid |
| restart with Windows element/frame identity | identity stale until re-resolved |
| update with pending approval | invalidate or explicitly migrate under a proven compatible policy; default invalidate |
| corrupted operation journal | fail closed / recovery mode, never silently discard and continue privileged work |

## 20. Secret-handling requirements

No Eve-derived durability feature may serialize secrets into durable state.

Prohibited durable data includes raw:

- tunnel keys;
- OAuth tokens;
- API keys;
- cookies;
- Windows credentials;
- approval secrets;
- private keys;
- bearer tokens.

Persist only opaque references and non-secret digests where required.

Diagnostic/support artifacts must remain redacted.

## 21. Telemetry decision

Cotra does not import Eve's default hosted telemetry behavior.

For Cotra v1:

- local audit/evidence is canonical;
- telemetry is disabled unless explicitly configured in a future authorized design;
- no release gate depends on Vercel Agent Runs, AI Gateway, hosted eval reporting, or another paid SaaS;
- local CI artifacts may contain bounded non-secret qualification evidence.

## 22. Source-copy policy

Prefer, in order:

1. reuse the security idea and implement it natively in Cotra;
2. translate a small algorithm/test pattern while recording provenance;
3. copy a narrowly bounded utility when its source is materially useful and its dependency graph is acceptable;
4. vendor a larger Eve component only if a future canonical decision proves that reimplementation would be worse.

Do not vendor the entire Eve runtime into Cotra.

Security-critical copied code receives the same review as first-party security-critical code.

## 23. No new authority from donor code

Any donor-derived change must state an authority delta.

If the intended authority delta is `NONE`, tests must prove no new externally reachable capability was introduced.

If donor-derived work does add authority in a future grain, that grain must independently define:

- new capability;
- effect level;
- approval class;
- workspace/trust dependency;
- policy revision;
- provider ceiling;
- replay class;
- secret exposure rules;
- negative tests;
- Windows/platform qualification.

No donor integration may hide an authority increase as refactoring.

## 24. Implementation order

The implementation order is fixed at the program level:

```text
FINISH CURRENT SG-000040 WITHOUT EVE SCOPE EXPANSION
-> SG-000040 CLOSEOUT
-> REBUILD P11 GAP MATRIX
-> CLOSE ANY REAL P11 GAPS
-> P11 EXIT
-> P12 lifecycle/durability grains
-> P12 EXIT
-> P13 manifest/eval/provenance/security/release grains
-> P13 EXIT
-> FINAL P01-P13 COMPLETION MATRIX
-> PROJECT_COMPLETE = YES
```

Exact future SpecGrain IDs MUST be derived from live canonical truth. This planning document intentionally does not pre-allocate SG numbers.

## 25. P12 proposed grain families

Do not treat these labels as canonical IDs. Derive actual grains live.

1. **Lifecycle journal and replay law**
   - durable operation state;
   - replay classes;
   - crash semantics;
   - approval/input invalidation.

2. **Per-user install/bootstrap**
   - install manifest;
   - startup;
   - status/doctor;
   - secure local state.

3. **Update/recovery/rollback**
   - version transition;
   - failure injection;
   - last-known-good behavior.

4. **Uninstall/reinstall**
   - cleanup;
   - retained-data declaration;
   - no orphan authority.

5. **P12 exit**
   - explicit lifecycle matrix.

## 26. P13 proposed grain families

Do not treat these labels as canonical IDs. Derive actual grains live.

1. **Capability manifest and schema integrity**
2. **Security scenario/eval harness**
3. **Orchestrator/client isolation qualification**
4. **Dependency/license/SBOM/donor provenance**
5. **Final Windows E2E and release artifacts**
6. **P13 exit and project completion matrix**

If a live gap matrix proves a family should be split further, split it. Do not widen a grain for schedule convenience.

## 27. Testing strategy

Every Eve-derived or Eve-inspired change must include the appropriate combination of:

- unit tests;
- property/boundary tests;
- deterministic fake-adapter tests;
- restart/recovery tests;
- policy-negative tests;
- Windows qualification;
- integration tests;
- E2E tests;
- exact-head CI;
- genuine TypeSafe Jev;
- Alibaba Open Code Review;
- manual exact-diff security review.

CodeRabbit, Cubic, Qodo, or similar generic reviewers are not canonical qualification evidence.

## 28. Release threat cases added by this plan

P13 threat regression must explicitly test these donor-integration risks:

1. model-visible dynamic capability treated as authorization;
2. subagent claiming inherited parent approval;
3. skill text claiming broader permissions than kernel policy;
4. evaluator output interpreted as approval;
5. replayed durable callback duplicating a side effect;
6. restored workflow/session reviving an expired approval;
7. restored state reviving an input lease;
8. credential accidentally serialized into durable state;
9. generic connection bypassing destination-scoped network policy;
10. sandbox command reaching Cotra protected IPC;
11. client-side tool wrapper bypassing cotrad;
12. remote client identity trusted from an unauthenticated body field;
13. hosted telemetry/export silently enabled;
14. donor dependency introducing unrestricted egress;
15. donor source copied without required notice/provenance.

## 29. Definition of ready

This plan is implementation-ready when all of the following are true:

- SG-000040 remains unchanged by donor integration;
- Eve donor pin and license are recorded;
- adopt/adapt/reject decisions are explicit;
- Cotra/client authority boundary is explicit;
- durable operation state model is defined;
- replay classes and crash rules are defined;
- approval recovery rules are defined;
- capability manifest contract is defined;
- evaluator authority ceiling is defined;
- subagent/delegation authority ceiling is defined;
- credential/connection boundary is defined;
- sandbox decision is explicit;
- zero-cost and no-hosted-dependency rules are explicit;
- P12 grain families and exit criteria are explicit;
- P13 grain families and exit criteria are explicit;
- provenance/license/SBOM requirements are explicit;
- final E2E path remains the existing Cotra path;
- no new program beyond P13 is required by this donor adoption.

All conditions above are satisfied by this document.

## 30. Final architectural conclusion

The correct Cotra/Eve composition is not:

```text
Eve inside cotrad
```

and not:

```text
model -> Eve generic runtime -> unrestricted local machine
```

The correct composition is:

```text
agent/orchestrator
    -> narrow Cotra client surface
    -> cotrad authority kernel
    -> typed capability provider
    -> evidence
```

with Eve contributing proven design patterns around durability, context minimization, delegation isolation, evaluation, and restart-safe workflows.

This keeps Cotra small enough to remain a security boundary while making it robust enough to serve sophisticated durable agents without depending on Desktop Commander-style broad machine authority.

## 31. Canonical execution rule

This document does not itself activate new authority.

After it becomes canonical, implementation still follows the normal Cotra lifecycle:

```text
RE-ESTABLISH LIVE TRUTH
-> BUILD GAP MATRIX
-> DERIVE NARROW GRAIN
-> ACTIVATE
-> QUALIFY ACTIVATION
-> NORMAL MERGE
-> IMPLEMENT
-> TEST
-> PLATFORM QUALIFY
-> FREEZE EXACT HEAD
-> CI
-> GENUINE JEV
-> ALIBABA OCR
-> MANUAL SECURITY REVIEW
-> NORMAL MERGE
-> CLOSEOUT
-> QUALIFY CLOSEOUT
-> NORMAL MERGE
-> PROGRAM EXIT CHECK
```

Never skip exact-head qualification because an implementation is copied from or inspired by an upstream project.
