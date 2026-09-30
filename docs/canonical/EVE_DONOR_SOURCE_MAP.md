# Eve Donor Source Map for Cotra

Status: SUPPORTING IMPLEMENTATION MAP
Date: 2026-09-30
Donor repository: https://github.com/vercel/eve
Pinned donor SHA: 9c36b7c280fda89ae678cabfd8d906f4bde2216f
Donor tree: d3fed540181f1987a4699c6de2c9986eac5f673c
Donor license: Apache-2.0
Primary Cotra plan: `docs/canonical/EVE_DONOR_INTEGRATION_PLAN.md`

This file tells future Cotra implementation grains exactly which Eve source/document locations were reviewed and what may be adapted from them. It is not an authority grant and does not activate any new Cotra capability.

## 1. Durability and replay

Upstream:
- `docs/concepts/execution-model-and-durability.mdx`
- `docs/tools/overview.mdx`

Useful patterns:
- durable session/turn/step separation;
- completed checkpoint results do not rerun;
- interrupted steps can rerun;
- side-effecting operations require idempotency, operation records, or explicit reconciliation;
- approval is not a substitute for deduplicating an ambiguous non-idempotent effect;
- parked work can release compute while retaining workflow state.

Cotra adaptation:
- local privileged operation journal in P12;
- replay classes;
- `OUTCOME_UNKNOWN` for ambiguous non-idempotent crashes;
- no blind side-effect replay;
- approval/input lease invalidation across restart unless continuity is explicitly proven.

Do not copy:
- the full Eve workflow/session runtime into `cotrad`;
- model-conversation durability into Cotra's authority kernel.

## 2. Context minimization

Upstream:
- `docs/concepts/context-control.md`

Useful patterns:
- keep permanent constraints separate from optional procedures;
- expose only the narrowest context needed;
- keep large runtime data in workspace/files rather than always-on model context;
- use specialist isolation instead of giving one model every tool and every context item.

Cotra adaptation:
- P13 capability manifest;
- narrow MCP tool descriptions;
- model-visible surface must be no broader than the kernel mapping;
- release test that every MCP tool maps to exactly one bounded kernel capability.

Do not copy:
- Eve instructions/skills as kernel policy;
- client prompt state into Cotra authorization.

## 3. Trusted runtime vs sandbox boundary

Upstream:
- `docs/concepts/security-model.md`
- `docs/sandbox/index.mdx`

Useful patterns:
- secrets stay on the trusted side;
- model-controlled process/file execution does not receive `process.env` secrets;
- credentials should be brokered rather than handed to untrusted compute;
- inbound identity must come from authenticated transport, not body-supplied claims.

Cotra adaptation:
- preserve separate privileged processes and typed providers;
- prevent child processes from inheriting tunnel/internal credentials;
- no raw secret values in durable lifecycle state;
- any future credential broker uses opaque references and destination/principal binding.

Explicit rejection:
- unrestricted app-runtime network;
- generic model-facing `bash` sandbox as Cotra's authority primitive;
- `allow-all` egress;
- Vercel Sandbox as a required dependency.

## 4. Subagent isolation and delegation

Upstream:
- `docs/subagents/index.mdx`

Useful patterns:
- declared specialists can have distinct tools, skills, state, sandbox, and connections;
- child sessions do not implicitly see parent conversation history;
- tool availability is not the same thing as approval;
- subagent delegation should not itself be treated as an approval boundary.

Cotra adaptation:
- client/orchestrator subagents remain independent requesters;
- no parent-to-child approval token inheritance;
- no trust, input-lease, frame, coordinate, network, or secret authority inheritance;
- future lineage metadata is audit-only unless separately authenticated and governed.

Do not copy:
- an internal model/subagent runtime into `cotrad` for v1;
- remote-agent trust as an implicit Cotra trust relation.

## 5. Dynamic capabilities

Upstream:
- `docs/guides/dynamic-capabilities.md`

Useful patterns:
- capability sets can vary by authenticated principal/session state;
- runtime must re-check availability before execution;
- stale/manually constructed calls fail;
- connection instance identity must change when account/endpoint/auth identity changes.

Cotra adaptation:
- model/client visibility may vary, but cotrad always re-authorizes execution;
- P13 explicitly tests `MODEL_VISIBLE_CAPABILITY != AUTHORIZED_CAPABILITY`;
- no stale tool call bypass after policy/workspace revision changes.

Do not copy:
- dynamic tool visibility as policy authority;
- model-selected endpoint/auth material into generic privileged execution.

## 6. Typed tools and side effects

Upstream:
- `docs/tools/overview.mdx`

Useful patterns:
- typed schemas before execution;
- output shaping/minimization;
- tool execution and human approval are distinct concerns;
- interrupted side effects require idempotency handling;
- record unique operation identity before writes when upstream idempotency is unavailable.

Cotra adaptation:
- P12 operation IDs and replay classes;
- P13 schema/capability manifest integrity;
- bounded/redacted evidence outputs;
- no secret or unnecessary sensitive content in model-visible results.

Critical difference:
- Eve tools execute in a trusted full Node.js app runtime; Cotra privileged actions continue to execute only through bounded policy/provider paths.

## 7. Human approval

Upstream:
- `docs/tools/overview.mdx`
- `docs/patterns/multi-tenant-approvals.md`
- `packages/eve/src/tools/approval/policies.ts`

Useful patterns:
- approval occurs before execution;
- approval is a gate, not the entire authorization system;
- approval requests can park/resume durable work.

Cotra adaptation:
- retain Cotra's stronger nonce/digest/expiry/one-shot model;
- retain SOFT/STRONG classes;
- retain local-presence requirements;
- revalidate state after waits;
- no provider effect before approval consume;
- crash recovery must never duplicate an approved effect.

Do not copy:
- any evaluator-based approval as a replacement for Cotra human presence or hard policy;
- a generic approval helper that can bypass Cotra's digest/trust binding.

## 8. Evals and Jev

Upstream:
- `docs/guides/evaluate.md`
- `docs/evals/judge.mdx`
- `research/evaluation-model-judges.md`
- `packages/eve/src/evals/`

Reviewed Eve behavior:
- typed eval/judge patterns;
- default evaluation model support currently references `typesafe-ai/jev`;
- evaluation metadata can capture normalized scores/model identity/usage.

Cotra adaptation:
- P13 scenario/eval harness;
- deterministic expected decision/evidence assertions;
- optional Jev semantic scoring/findings;
- release evidence reporter.

Hard ceiling:
- Jev can score/classify/review/recommend;
- Jev cannot issue approval, change trust, satisfy STRONG presence, override hard denials, or grant authority;
- deterministic/platform requirements are not replaced by semantic evaluation.

## 9. Connections and credentials

Upstream:
- `docs/connections/overview.mdx`
- `docs/connections/openapi.mdx`
- `docs/guides/dynamic-capabilities.md`
- `docs/concepts/security-model.md`

Useful patterns:
- keep tokens out of model-visible state;
- bind authenticated connection identity to a stable non-secret instance identity;
- do not serialize live tokens into durable workflow state;
- combine authorization and approval rather than treating either as sufficient alone.

Cotra v1 decision:
- no generic connection broker in P11;
- SG-000040 stays credential-free bounded HTTPS GET;
- no caller `Authorization`, cookies, proxy auth, or arbitrary headers;
- future authenticated integrations require separate typed provider/grain authority.

## 10. Self-hosting / zero-cost relevance

Upstream:
- `docs/guides/deployment/self-hosting.md`
- `docs/concepts/execution-model-and-durability.mdx`

Useful observation:
- Eve's workflow architecture is not conceptually limited to Vercel-hosted execution; local/self-hosted worlds exist.

Cotra decision:
- no Vercel Workflow/Sandbox/AI Gateway dependency is introduced;
- donor-derived mechanisms must run within Cotra's existing zero-cost/local-first constraints;
- a hosted Eve deployment is never a release prerequisite.

## 11. License and notices

Upstream:
- `LICENSE`
- `NOTICE`

Reviewed state at donor pin:
- Eve is Apache-2.0;
- Eve NOTICE names Vercel, Inc. and additional bundled notices for code-mode dependencies.

Cotra requirements for direct source copying:
- preserve applicable copyright/license notices;
- mark modified copied files as required by Apache-2.0;
- add a Cotra third-party notice/provenance artifact before release;
- include only notices applicable to code actually copied/distributed;
- record donor file path and SHA for each reuse;
- do not use Vercel trademarks as Cotra branding.

## 12. Implementation review order

For each future P12/P13 Eve-derived grain, review the upstream sources in this order:

```text
1. EVE_DONOR_INTEGRATION_PLAN.md
2. this source map
3. exact pinned Eve source files listed for the feature
4. current Cotra architecture and live predecessor evidence
5. derive narrow SpecGrain from live truth
```

Never code directly from Eve `main` without first pinning the exact donor SHA used for that grain.

## 13. Donor update policy

The reviewed donor baseline is immutable for this plan:

```text
9c36b7c280fda89ae678cabfd8d906f4bde2216f
```

If a later Eve commit contains a materially better implementation:

1. record the new donor SHA;
2. review license/NOTICE changes;
3. compare the relevant source against the existing pin;
4. document why the donor pin changes;
5. qualify the resulting Cotra change normally.

Do not silently follow Eve `main` during implementation.

## 14. Final boundary

Use Eve to make Cotra more durable, testable, composable, and orchestrator-friendly.

Do not use Eve to make Cotra more privileged.
