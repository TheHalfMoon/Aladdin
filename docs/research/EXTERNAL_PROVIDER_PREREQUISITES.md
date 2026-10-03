# Qdral External Provider Prerequisites

Status: IMPLEMENTATION-READY EXTERNAL-DEPENDENCY REGISTER
Date: 2026-10-01
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`

## 1. Purpose

Qdral can make its software provider-neutral, self-hostable, and free/open source, but it cannot control third-party product policy, directory review, account eligibility, publisher verification, domain requirements, plan limits, or provider service pricing.

This register prevents those external dependencies from being hidden inside a software completion claim.

## 2. Completion states

Qdral tracks provider integration using separate states:

- `SOFTWARE_READY` — Qdral implementation and security qualification are complete.
- `SUBMISSION_READY` — provider package, production endpoint, legal/support URLs, publisher material, and review evidence are prepared.
- `SUBMITTED` — external provider acknowledges a real submission.
- `APPROVED` — provider approval is actually observed.
- `PUBLISHED` — integration is publicly available on the provider surface.
- `ACCOUNT_VERIFIED` — the target user/account/region/surface can actually install/use it.

No state implies the next one.

## 3. ChatGPT web / OpenAI public plugin

External prerequisites may include, according to current OpenAI requirements at submission time:

- eligible OpenAI organization/workspace and role;
- publisher/organization verification;
- a stable public HTTPS Streamable HTTP MCP endpoint;
- production OAuth 2.1 configuration;
- domain ownership/verification where required;
- privacy policy URL;
- terms of use URL;
- support/contact URL;
- security/reporting contact;
- plugin metadata/assets;
- app/plugin review;
- compliance with current tool/action guidelines;
- actual directory publication;
- actual availability to the user's ChatGPT plan, workspace, role, region, and surface.

### Cost honesty

Qdral's software and reference deployment must not require a paid compute subscription.

However, if OpenAI requires an independently owned/verified custom domain or another external prerequisite that has unavoidable monetary cost and the project does not already possess a suitable donated/owned resource, Qdral must report that as an **external publication blocker** rather than violate the zero-founder-cost rule or pretend it is free.

A free provider subdomain such as a reference hosting domain may be used for development only if OpenAI accepts it for the relevant verification/publication requirement. Acceptance must be verified, not assumed.

The project may use an existing user-owned/donated domain without treating its historic acquisition cost as a Qdral software dependency, but it must not silently purchase one.

## 4. Claude

External prerequisites for remote custom-connector availability include:

- a Claude account/plan that currently supports the required connector mode;
- connector configuration permissions for that account/workspace;
- public HTTPS reachability for remote mode;
- provider-accepted authentication configuration;
- current per-plan connector limits;
- actual account/region availability.

Local Claude Desktop/Code integration does not depend on a public relay when local MCP support is available.

Anthropic service usage limits or subscription/API charges are not Qdral charges and cannot be made free by Qdral.

## 5. Mistral

External prerequisites for Vibe Work remote mode include:

- current connector support on the target account;
- public HTTPS endpoint;
- supported authentication method;
- current provider limits and account/region eligibility.

Vibe Code local stdio mode can avoid a Qdral-hosted relay, but Mistral model/service pricing or quotas remain external. A local model path can avoid provider inference charges where technically supported.

## 6. Codex / local ChatGPT-capable surfaces

Local MCP availability depends on the installed client/product surface exposing local MCP/plugin configuration to the user.

Qdral must not claim that every ChatGPT interface can launch local MCP merely because another OpenAI surface can.

If the target surface lacks local MCP, remote/public-plugin mode is required.

## 7. Public relay hosting

The reference community relay may target free-tier infrastructure, but external facts include:

- account eligibility;
- free-tier quotas;
- abuse limits;
- availability/SLA;
- product changes;
- egress/request/storage limits;
- terms of service;
- TLS/custom-domain capabilities.

Rules:

- no automatic paid upgrade/overflow;
- hard fail-closed quota behavior;
- self-host remains supported;
- local Qdral remains usable when the community relay is down;
- no promise of unlimited free relay capacity.

## 8. DNS and domain lifecycle

A production public endpoint needs stable naming.

Design requirements:

- endpoint/domain configuration is not hard-coded into local policy logic;
- relay deployment supports an operator-provided hostname;
- OAuth issuer/resource metadata changes are versioned/migrated safely;
- domain loss or TLS failure fails remote mode closed and does not affect local mode;
- no client automatically trusts a replacement domain solely because the old endpoint is unavailable;
- DNS ownership and certificate renewal are operational prerequisites, not local Qdral authority.

## 9. Legal/support pages

Privacy, terms, support, and security-reporting pages can be hosted using free static hosting when accepted by the provider.

Their content must match actual Qdral behavior, especially:

- transient plaintext processing by a shared relay;
- retained device/account metadata;
- no payload logging policy;
- deletion/revocation path;
- community-relay limits;
- self-host/local alternatives;
- no affiliation/endorsement claim with AI providers.

## 10. Code signing and Windows reputation

Qdral's zero-cost rule currently means release binaries may remain unsigned unless a trusted signing capability is available at no founder cost.

External consequences can include Windows SmartScreen warnings and reputation friction.

This is separate from GitHub provenance/checksum verification and must not be hidden.

A future donated/free signing path may be evaluated through a new governed grain; no paid certificate is introduced silently.

## 11. Provider inference/service cost

`Qdral is free` means Qdral software does not charge the user and local/self-hosted operation has no mandatory Qdral SaaS fee.

It does **not** mean every connected AI provider offers unlimited inference or account access for free.

Qdral documentation must distinguish:

- Qdral software cost;
- Qdral relay cost/limits;
- third-party AI provider subscription/API/inference cost.

Local-model integrations are the only path Qdral can offer without depending on third-party inference billing.

## 12. Provider policy drift

Before each provider-specific implementation/submission and before release claims:

1. re-check current official provider documentation;
2. record verification date;
3. compare requirements with the compatibility matrix;
4. update the plan through normal review if requirements materially changed;
5. never preserve a stale marketing claim just because it was true during planning.

## 13. External blocker reporting

If a provider requirement cannot be met without violating Qdral's security or zero-cost constraints:

- complete all independent software work;
- record the exact requirement and current evidence;
- mark the provider distribution state blocked externally;
- do not weaken local security to satisfy the directory;
- do not introduce surprise billing;
- keep local/generic/self-host integrations releasable.

## 14. Definition of success for the user's goal

The user's goal of using Qdral "here in ChatGPT web" is achieved only when all of the following are observed:

1. Qdral remote software is `SOFTWARE_READY`;
2. the public OpenAI plugin is actually `PUBLISHED`;
3. the user's actual ChatGPT account/region/surface is `ACCOUNT_VERIFIED` for the plugin;
4. the user's Qdral device is paired;
5. a local remote-session lease is active for the intended workspace/profile;
6. a real tool call from that ChatGPT conversation reaches the paired device and is governed by the expected local policy/approval path.

Anything less is progress toward the goal, not proof that the goal is complete.
