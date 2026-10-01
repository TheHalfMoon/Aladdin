# Cotra Universal AI Access Plan

Status: IMPLEMENTATION-READY PROPOSAL
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`
Planning date: 2026-10-01
Target release: Cotra v0.2.x
Primary objective: make Cotra the provider-neutral trusted computer-access layer for ChatGPT, Claude, Mistral, Codex, local models, and generic MCP clients without making any one provider or hosted relay part of the local security boundary.

## 1. Product decision

Cotra is no longer defined as a ChatGPT-specific gateway.

The target product is:

> One local, auditable, approval-gated computer access layer that multiple AI clients can use without receiving unrestricted shell, filesystem, network, browser, or desktop authority.

The existing Cotra policy kernel, workspace identity, approval broker, audit/evidence model, typed providers, lifecycle tooling, installer, release hardening, and SpecGrain governance remain authoritative.

Provider integrations are adapters. They do not decide local policy and cannot mint approval or workspace trust.

Desktop Commander is not a dependency, fallback, qualification tool, or transport dependency.

## 2. Existing foundation that must be preserved

Cotra v0.1.0 already provides the security-critical local foundation:

- `cotrad` is the local policy and authority kernel.
- `cotra-mcp` is a low-authority MCP protocol edge.
- the current MCP process already serves over stdio;
- local kernel IPC is separate from external MCP transport;
- workspace, approval, trust, audit, provider ceilings, redaction, and postcondition rules are local;
- SOFT and STRONG approval remain independent of the model;
- browser, UIA, screenshot, coordinate, clipboard, and bounded-network capabilities have closed internal grains even where they are deliberately absent from the public v0.1 MCP surface;
- the public v0.1 MCP tool set is pinned by tests;
- installer, update, rollback, release qualification, SBOM, provenance, and reproducibility exist.

The universal-access program therefore refactors and extends the edge and distribution model. It must not move authority into a relay or provider integration.

## 3. Required operating modes

Cotra must support three independent modes.

### 3.1 Local process mode

For local AI clients that can launch MCP servers:

```text
AI client
  -> MCP stdio
  -> cotra-mcp
  -> authenticated local IPC
  -> cotrad
  -> policy / approval / typed provider
```

Target clients include Claude Desktop, Claude Code, Codex, Mistral Vibe Code, compatible IDEs, and generic MCP clients.

This mode requires no public endpoint and no Cotra-hosted infrastructure.

### 3.2 Loopback HTTP mode

For local clients that prefer Streamable HTTP:

```text
AI client on same machine
  -> http://127.0.0.1:<ephemeral-or-configured-port>/mcp
  -> cotra-mcp
  -> cotrad
```

Requirements:

- bind only to loopback by default;
- never bind `0.0.0.0` or a LAN interface through the normal command;
- Host and Origin validation with DNS-rebinding protection;
- protected local bearer/session credential or equivalent per-user access secret;
- bounded request body, header, stream, session, and idle limits;
- no browser CORS wildcard;
- no local admin/config endpoint on the MCP listener;
- no change to the local policy authority model.

### 3.3 Remote web mode

For hosted AI clients such as ChatGPT web, Claude web, and Mistral Vibe Work:

```text
Hosted AI provider
  -> HTTPS Streamable HTTP MCP
  -> Cotra relay edge
  -> authenticated user/device route
  -> outbound-only device link
  -> local cotra-mcp edge
  -> cotrad
  -> local policy / approval / provider
```

The remote relay is a routing and authentication boundary, not a local authorization boundary.

The default remote design must require no inbound port on the user's PC.

## 4. Trust boundaries

The implementation must explicitly preserve these boundaries.

### TB-0 — Model output

The model is untrusted input. Tool arguments are never authority.

### TB-1 — AI host/client

ChatGPT, Claude, Mistral, Codex, or another client may authenticate to Cotra and invoke exposed tools, but host metadata, display names, prompts, annotations, and model claims are not local authorization proof.

### TB-2 — Public Cotra relay edge

The relay may authenticate the provider/client and the end user, validate protocol envelopes, route to the correct device, and enforce transport quotas. It must not grant a workspace, approve an action, create local capability tokens, or execute OS operations.

### TB-3 — Device link

The device link is authenticated, encrypted transport between one registered Cotra installation and its relay route. A route cannot be switched to another device without explicit pairing/revocation state.

### TB-4 — Local MCP edge

`cotra-mcp` performs protocol validation and converts a selected public tool into the existing internal request contract. It remains low-authority.

### TB-5 — `cotrad`

`cotrad` remains the authority kernel. It decides capability, workspace, policy revision, provider ceiling, risk class, and approval requirements.

### TB-6 — Local approval authority

SOFT and STRONG approval stay on the user's computer. Remote providers and the relay never gain approval authority.

## 5. Provider-neutral MCP server structure

The current monolithic stdio entry point must be split without changing the tool catalog as the first implementation step.

Target structure:

```text
apps/cotra-mcp/src/
  server.ts            # build/register the tool catalog
  result.ts            # safe result projection
  transports/
    stdio.ts
    loopback_http.ts
    relay_device.ts
  entrypoints/
    stdio.ts
    loopback_http.ts
    relay_device.ts
```

Requirements:

- one tool registration source of truth;
- transport-specific code cannot register additional tools;
- transport-specific identity is supplied as request context, never through caller-controlled tool fields;
- each transport maps to the same kernel contract;
- existing v0.1 tools remain byte-for-schema compatible until an explicit capability grain changes them;
- protocol version negotiation and compatibility are covered by tests;
- current stdio behavior remains available throughout migration.

## 6. Client/session identity model

Cotra must distinguish transport identity, remote principal identity, device identity, local MCP session identity, workspace identity, and approval identity.

They must never be collapsed into one string.

Required internal context fields:

- `transport_kind`: `stdio`, `loopback_http`, `relay`;
- `provider_kind`: `openai`, `anthropic`, `mistral`, `codex`, `generic`, or locally configured identifier;
- `remote_principal_id`: opaque relay-authenticated identifier when remote; absent locally;
- `device_id`: stable random Cotra installation identifier;
- `remote_connection_id`: stable provider MCP connection bound to one `remote_principal_id` and one exact `device_id`;
- `connection_id`: short-lived remote transport connection identity bound to the stable `remote_connection_id` and current route/connection epoch;
- `client_session_id`: locally generated session ID used by Cotra audit/policy;
- `protocol_version`;
- `tool_surface_profile`;
- `workspace_id` from the tool request, still validated by local policy;
- `policy_revision` bound by `cotrad` as today.

Remote provider names and account labels are audit metadata only unless verified by the relay's authentication flow.

## 7. Device identity, pairing, and revocation

Remote access requires explicit device pairing.

### 7.1 Device identity

On first remote setup, Cotra creates a device key pair in protected local storage.

Requirements:

- private key never leaves the device;
- relay stores only the device public key, opaque device ID, route state, and minimum account mapping required to deliver traffic;
- device proof uses challenge/response and never sends the private key;
- rotation is supported;
- lost/corrupt key fails closed;
- no machine password, Windows Hello secret, OpenAI token, Anthropic token, or Mistral token is stored by the relay.

### 7.2 Pairing

Pairing is an explicit local action.

Properties:

- one-time pairing code;
- short expiry;
- high entropy; display formatting must not reduce effective entropy below the documented threshold;
- rate limited;
- successful exchange binds the remote account/session to exactly one selected device;
- code is invalid after success, expiry, revoke, or excessive failures;
- pairing alone does not grant workspace trust or action approval;
- pairing history is locally visible and revocable.

### 7.3 Revocation

The user must be able to revoke:

- one remote provider connection;
- one remote principal;
- all remote sessions on one device;
- all paired devices/account routes;
- emergency local access through the existing revoke boundary.

Revocation must invalidate active and refreshable remote sessions, not only hide them from UI.

## 8. Remote authentication and OAuth

The public MCP edge must implement the current MCP authorization model and provider-specific requirements without weakening local authorization.

For OpenAI public plugin compatibility, the production edge must support OAuth 2.1 authorization-code + PKCE, protected-resource metadata, authorization-server metadata, correct `resource` binding, issuer/audience/scope verification, token expiry/revocation, and the supported client registration paths required by the OpenAI host.

Design requirements:

- prefer standards-compliant OAuth 2.1 rather than provider-specific bearer hacks;
- support OpenAI Client ID Metadata Documents where practical, with DCR fallback only when required;
- use exact issuer comparison and RFC 9207 issuer identification where enabled;
- validate token signature, issuer, audience/resource, expiry/not-before, scopes, and revocation state on every remote request;
- no machine-to-machine grant bypass for end-user access;
- expose no raw API-key authentication for the public ChatGPT plugin;
- authentication proves the remote Cotra account/principal, not local workspace trust;
- remote token scopes are an outer ceiling, while `cotrad` policy and local approval are the final local ceiling.

Initial scope vocabulary:

- `cotra.read`
- `cotra.write`
- `cotra.execute`

A tool may require more than one scope. Local policy can always deny a scope-authorized request.

A normative default-deny scope-to-tool/profile matrix must be reviewed before public remote exposure. Every tool and profile states its required scopes, and missing or unknown scope/tool mappings deny.

## 9. Public relay protocol

The relay protocol must not be a generic TCP, HTTP, CONNECT, SOCKS, WebSocket-proxy, or arbitrary destination proxy.

It may carry only the messages needed to represent an authenticated MCP connection and its responses.

### 9.1 Routing

Each accepted remote request resolves to:

`authenticated principal -> paired device -> active device channel -> local MCP session`

Cross-tenant/device routing ambiguity is a hard failure.

### 9.2 Message envelope

Relay frames must carry at minimum:

- protocol version;
- route/device ID;
- stable `remote_connection_id` and short-lived `connection_id`;
- monotonically scoped message sequence or replay nonce;
- request/response correlation ID;
- message kind;
- payload length;
- creation/expiry time;
- integrity/authentication information for the device channel;
- a device-verifiable authorization envelope that binds the authenticated principal, connection, selected device, and request digest before local dispatch.

The relay must reject duplicate, expired, oversized, malformed, cross-route, and impossible-order frames. A compromised relay must not be able to redirect principal A's authenticated request to device B and have it accepted locally; local dispatch verifies the device-verifiable envelope.

### 9.3 Limits and backpressure

Hard limits are required for:

- request size;
- result size;
- concurrent MCP requests per connection;
- concurrent remote connections per device;
- queued requests while a device is temporarily disconnected;
- queue lifetime;
- idle session lifetime;
- streamed output rate;
- authentication failures;
- pairing attempts.

The default offline policy is fail closed. Cotra must not queue operations for later surprise execution. Mutating operations must never be automatically retried after dispatch unless a stable operation ID is deduplicated by cotrad with result replay; retries are limited to idempotent reads and must preserve original request/approval expiry semantics. A bounded retry/short grace period may exist only for transport reconnection and must preserve the original request/approval expiry semantics.

### 9.4 Cancellation

Remote cancellation is propagated to the local MCP edge and kernel request where supported. Cancellation is never described as verified process termination unless the existing provider establishes termination.

## 10. Relay privacy model

The privacy claim must be precise.

A shared public relay terminates provider HTTPS and therefore may handle MCP request/result plaintext transiently in process memory. Cotra must not claim provider-to-device end-to-end encryption unless a provider transport actually supports it.

Requirements for the reference relay:

- no payload persistence;
- no raw tool arguments/results in application logs;
- no clipboard/file content, process output, screenshot bytes, secrets, approval payloads, or browser content in metrics;
- minimum routing/auth metadata only;
- documented retention for auth/rate-limit records;
- configurable self-hosted mode for users that do not trust a shared relay;
- support deletion/revocation of stored account/device metadata;
- no training, profiling, advertising, or sale of relay traffic.

## 11. Zero-cost rule

Local Cotra use remains fully independent of hosted infrastructure.

The project must ship an open-source self-hostable relay.

A community relay may be offered for convenience, but it is not part of the local security boundary and is not required for local clients.

The reference community deployment may target free infrastructure where technically appropriate, with these constraints:

- no automatic paid overflow;
- no founder-funded required subscription;
- hard quota/fail-closed behavior rather than surprise billing;
- quotas and availability disclosed honestly;
- self-hosting remains a supported escape path;
- the software does not claim unlimited free global relay capacity.

A Cloudflare Workers + Durable Objects free-tier reference is acceptable for the initial community deployment because current free-tier support includes stateful Durable Objects and WebSockets, but production design must tolerate free-tier exhaustion and provider limits.

## 12. Tool surface profiles

Universal connectivity and Desktop Commander replacement are separate authority questions.

Cotra must support explicit tool-surface profiles so a provider cannot silently obtain every implemented capability.

Initial profiles:

### `core`

The current v0.1 public MCP tools, unchanged until separately authorized.

### `developer`

`core` plus carefully governed developer-machine operations added in later grains, including a locally managed executable registry and bounded filesystem/Git capabilities.

No raw shell is implied.

### `desktop_structured`

Adds separately authorized structured browser/UIA/screenshot/clipboard tools with their existing internal ceilings.

### `coordinate_fallback`

Adds the already-governed coordinate fallback only when explicitly enabled. It remains lower-ceiling, human-interruptible, and cannot target Cotra protected surfaces.

A remote account's OAuth scope is not sufficient to enable a profile. The profile must be locally enabled for the device/workspace.

## 13. Desktop Commander replacement requirements

Cotra cannot claim Desktop Commander replacement merely because multiple AI clients can connect.

The v0.2 program must include an explicit parity inventory and then fill only the gaps that fit Cotra's security model.

The replacement target is useful developer/workstation operation, not unrestricted remote shell parity.

Required parity categories:

- workspace file read/list/search;
- create/write/rename/move/delete/mkdir with exact path identity and approval as appropriate;
- Git read/mutation/network operations under repository/destination policy;
- bounded developer process execution using a locally managed executable registry;
- process status/termination only if independently authorized and verified;
- structured browser automation;
- structured Windows UI Automation;
- screenshot/visual proposal/coordinate fallback;
- bounded clipboard read/write;
- diagnostics, status, logs, update, revoke, and emergency stop.

Explicit non-parity by design:

- unrestricted shell strings;
- unrestricted PowerShell script execution;
- arbitrary executable launch without local registry/policy;
- generic socket/proxy/VPN behavior;
- remote approval delegation;
- credential/security-dialog automation;
- silent elevation;
- arbitrary user-profile filesystem authority.

## 14. Executable registry design

To make Cotra useful for development without restoring a generic shell, process execution must become policy-configurable rather than hard-coded to one executable.

The registry is local protected configuration.

Each executable entry binds:

- stable executable identity with an immutable verification rule (hash and, where available, publisher/signature) or a protected-path plus ACL ownership/writeability rule that prevents user-writable replacement, revalidated immediately before launch;
- optional publisher/hash rule;
- allowed argument grammar or argument policy;
- allowed cwd roots;
- environment allowlist;
- stdin policy;
- network class;
- timeout/output ceilings;
- risk/approval class;
- postcondition policy where applicable.

Changing the registry is PRIVILEGED and requires STRONG local presence.

No remote MCP tool can add or widen an executable entry.

The first developer pack should begin with individually reviewed, non-interpreting binaries (for example Git and Cargo); Node/npm, Python, and test runners require separate grains with script/config provenance and descendant containment before exposure, and must be qualified on the target Windows environment.

## 15. Provider integration matrix

### 15.1 ChatGPT / Codex

Two paths are required:

1. Local plugin/MCP configuration for local-capable ChatGPT/Codex surfaces.
2. Public remote plugin for ChatGPT web and other supported remote surfaces.

Public plugin requirements include:

- stable public HTTPS Streamable HTTP `/mcp` endpoint;
- OAuth 2.1 for user-specific/private/write access;
- exact, separately exposed tools rather than a generic executor;
- explicit `readOnlyHint`, `destructiveHint`, and `openWorldHint` on every tool;
- production auth/resource metadata;
- accurate privacy policy, support, terms, and publisher identity;
- plugin package/metadata and review materials;
- domain verification;
- provider review and publication before claiming ChatGPT-web availability.

OpenAI-managed mTLS should be verified when the hosting layer permits client-certificate validation, as defense in depth. OAuth remains required for the end user.

### 15.2 Claude

Support both:

- local MCP / Desktop Extension path for Claude Desktop and Claude Code;
- remote HTTPS MCP path for Claude custom connectors.

The remote connector uses the same relay endpoint and Cotra account/device pairing rather than a separate Claude-specific backend.

### 15.3 Mistral

Support:

- stdio and Streamable HTTP configuration for Vibe Code;
- public HTTPS MCP connector for Vibe Work.

Keep the exposed tool list static and independently callable because current Mistral custom connectors do not require Cotra to depend on MCP resources or dynamic tool discovery.

### 15.4 Generic MCP

Publish tested examples for stdio and Streamable HTTP clients without provider-specific assumptions.

## 16. Tool metadata contract

The public tool catalog becomes a reviewed API contract.

Every tool must define:

- stable name;
- clear title/description;
- strict input schema;
- bounded outputs;
- `readOnlyHint`;
- `destructiveHint`;
- `openWorldHint`;
- `idempotentHint` where accurate;
- required remote OAuth scopes;
- local capability/effect mapping;
- timeout/cancellation behavior;
- model-safe errors.

Annotations are hints only. `cotrad` remains authoritative.

No tool may implement `run_anything`, generic operation discovery, schema-selected execution, arbitrary proxying, or hidden subcommands that widen the reviewed surface.

## 17. Remote result/data minimization

Remote mode must return only what the invoked tool requires.

Do not return by default:

- internal relay IDs;
- raw device identifiers;
- raw audit-chain IDs;
- IP addresses;
- auth/token material;
- approval secrets;
- timestamps unrelated to the requested result;
- server logs;
- unrelated local machine metadata.

Provider-specific result shaping may remove metadata but may never add authority or fabricate evidence.

## 18. Failure model additions

Add typed failures for the universal-access layer:

- `TRANSPORT_UNAVAILABLE`
- `REMOTE_AUTH_REQUIRED`
- `REMOTE_AUTH_INVALID`
- `REMOTE_SCOPE_DENIED`
- `DEVICE_OFFLINE`
- `DEVICE_REVOKED`
- `PAIRING_REQUIRED`
- `PAIRING_EXPIRED`
- `PAIRING_DENIED`
- `ROUTE_MISMATCH`
- `RELAY_REPLAY_DETECTED`
- `RELAY_SEQUENCE_INVALID`
- `REMOTE_RATE_LIMITED`
- `REMOTE_QUEUE_EXPIRED`
- `REMOTE_SESSION_INACTIVE`
- `MCP_PROTOCOL_UNSUPPORTED`
- `TOOL_SURFACE_DENIED`

These remain distinct from local `CAPABILITY_DENIED`, `WORKSPACE_DENIED`, and approval failures.

## 19. Threats that must be proven before remote release

At minimum:

1. cross-user or cross-device request routing;
2. stolen pairing code;
3. OAuth CSRF and authorization-server mix-up;
4. access-token replay;
5. refresh-token theft/revocation failure;
6. forged provider/client identity;
7. MCP session fixation or session-ID collision;
8. relay message replay/reordering;
9. device channel hijack;
10. malicious relay attempting to widen local authority;
11. offline queued mutation surprise execution;
12. approval drift across reconnect;
13. workspace/policy revision drift across reconnect;
14. tool-catalog drift between local and public edges;
15. cross-provider scope confusion;
16. local loopback DNS rebinding;
17. malicious webpage driving localhost MCP;
18. payload/log leakage at relay;
19. result leakage to the wrong remote principal;
20. provider prompt injection attempting local exfiltration;
21. denial-of-service and free-tier quota exhaustion;
22. cancellation/timeout ambiguity;
23. generic proxy/tunnel abuse;
24. protected Cotra-surface automation through newly exposed desktop tools;
25. executable-registry widening from a remote client.

Each threat requires an explicit control and a CI/manual qualification reference before program exit.

## 20. Test architecture

### 20.1 Transport contract suite

Run the same tool-catalog contract tests against:

- in-process server builder;
- stdio child process;
- loopback Streamable HTTP;
- simulated relay device transport.

The tool schemas/annotations must remain identical for the same surface profile.

### 20.2 Relay integration harness

Provide a deterministic local harness with:

- mock OAuth issuer/verifier;
- mock public MCP request edge;
- relay route broker;
- device uplink;
- deliberate disconnect/reconnect;
- duplicate/reordered/expired frames;
- two users and at least two devices to prove tenant separation.

### 20.3 Provider qualification

Record real qualification where accounts/surfaces are available:

- ChatGPT public plugin/developer test surface;
- Claude local and remote connector paths;
- Mistral Vibe Code and Work paths;
- Codex local plugin/MCP path.

CI must not fabricate provider availability. Provider UI/account approval remains explicit external evidence.

### 20.4 Chaos and negative tests

Required cases include:

- relay restart;
- device restart;
- network loss during read and write;
- duplicate POST;
- cancellation during long request;
- token expiry mid-session;
- device revoke mid-session;
- workspace revoke mid-session;
- local approval expiry while the remote connection survives;
- quota exhaustion;
- stale relay route;
- malformed MCP version;
- oversized frames;
- cross-device route substitution.

## 21. Deployment architecture

The relay implementation must be portable.

Reference targets:

- self-hosted Node/TypeScript or Rust service using standard HTTPS/WebSocket infrastructure;
- Cloudflare Workers + SQLite-backed Durable Objects reference deployment for zero-cost development/community operation, using WebSocket hibernation where appropriate;
- no Cloudflare-specific behavior in local Cotra contracts.

The public `/mcp` endpoint and OAuth metadata must remain standards-compatible if the hosting backend changes.

## 22. Privacy / legal / directory readiness

Before public plugin submission, the project must have:

- a public privacy policy describing transient relay processing and retained account/device metadata;
- terms of use;
- support/contact page;
- security reporting policy;
- data deletion/revocation path;
- verified publisher identity required by the target directory;
- dependency/license review for new relay/auth packages;
- abuse prevention/rate limits;
- no claim that Cotra is made by or endorsed by an AI provider;
- no claim of unlimited free relay capacity.

## 23. Delivery programs and grains

The following sequence is intentionally ordered so no remote authority is added before local transport neutrality, identity, and threat controls exist.

### COTRA-P14 — Transport neutrality and local clients

#### SG-000047 — Universal-access architecture and transport contract

- ratify this plan and the universal-connectivity threat model;
- document the provider compatibility matrix;
- freeze invariants and transport identity fields;
- no runtime authority change.

#### SG-000048 — Transport-neutral MCP server builder

- extract one server/tool registration source of truth;
- preserve the existing v0.1 tool catalog and schemas;
- transport contract tests.

#### SG-000049 — Supported local stdio entrypoint

- expose `cotra mcp stdio` or equivalent installed entrypoint;
- protected environment and lifecycle behavior;
- package configuration examples;
- no OpenAI tunnel requirement.

#### SG-000050 — Loopback Streamable HTTP

- `cotra mcp serve` or equivalent;
- loopback-only bind;
- local auth token;
- Host/Origin validation and DNS-rebinding tests;
- resource/session limits.

#### SG-000051 — Local client packages

- Claude Desktop extension/config;
- Codex/ChatGPT local plugin package where supported;
- Mistral Vibe Code config;
- generic MCP config and inspector qualification.

P14 exit:

- at least three independent local MCP clients use Cotra without OpenAI Secure MCP Tunnel;
- identical Cotra policy/approval behavior across transports;
- no new OS authority introduced by transport work.

### COTRA-P15 — Remote relay, identity, and web access

#### SG-000052 — Relay protocol and remote threat contract

Design-only first grain: freeze routing, frame, replay, queue, cancellation, quota, privacy, and failure semantics before network code.

#### SG-000053 — Device identity, pairing, and revocation

Implement protected device keys, challenge/response registration, one-time pairing, rotation, and hard revocation.

#### SG-000054 — OAuth 2.1 authorization boundary

Implement protected resource metadata, authorization-server metadata, PKCE, resource/audience/scope enforcement, revocation, and provider client-identification hooks.

#### SG-000055 — Outbound-only device uplink

Implement authenticated device channel with route binding, replay/sequence protection, bounded reconnect, and no generic proxy semantics.

#### SG-000056 — Public Streamable HTTP MCP edge

Implement standards-compliant `/mcp`, remote auth context, safe backpressure/offline behavior, and exact mapping to the local tool registry.

#### SG-000057 — Open-source relay and zero-cost reference deployment

Ship self-host mode plus a free-tier reference deployment with hard quotas and no automatic paid overflow.

#### SG-000058 — P15 adversarial/chaos exit

Cross-tenant, OAuth, replay, reconnect, offline mutation, privacy/log, quota, and route-confusion qualification.

P15 exit:

- a remote authenticated MCP client can safely reach one paired device without inbound PC ports;
- every remote MCP dispatch requires both device-backed pairing/OAuth and an active finite local remote-session lease checked by `cotrad` and bound to the exact remote connection, device, client profile, workspaces, policy revision, tool surface, and expiry;
- the relay and the provider cannot create, widen, extend, or renew the local remote-session lease;
- lease expiry, reconnect, token refresh, relay restart, lock/logoff invalidation, profile narrowing, policy revision change, workspace revoke, and device route revocation all deny remote dispatch, including read-only calls, with typed `REMOTE_SESSION_INACTIVE` where no active lease exists;
- relay compromise does not grant local approval/workspace authority in the tested model, and read exfiltration through a compromised relay is bounded by the active lease scope and hard maximum duration;
- self-host mode works independently of any Cotra-operated relay;
- community relay limits are explicit and fail closed.

### COTRA-P16 — Safe Desktop Commander replacement surface

#### SG-000059 — Capability/parity inventory

Map every implemented provider and Desktop Commander-class workflow to `implemented+exposed`, `implemented+hidden`, `missing`, or `intentionally denied` before widening MCP.

#### SG-000060 — Protected executable registry

Replace the one-executable public ceiling with a STRONG-gated local registry and bounded developer profiles; retain argv-only execution and no raw shell primitive.

#### SG-000061 — Filesystem mutation completion

Expose only individually reviewed mkdir/rename/move/delete-style operations required for workstation use, with final-path identity and destructive approval controls.

#### SG-000062 — Structured browser MCP exposure

Expose the previously closed structured browser shapes with static, reviewed tools and no generic script/debugger authority.

#### SG-000063 — Structured desktop MCP exposure

Expose UIA observation/actions and screenshot/visual proposal/coordinate fallback under explicit profiles and existing protected-surface/input-lease ceilings.

#### SG-000064 — Clipboard and bounded network MCP exposure

Expose only the already-qualified bounded clipboard and destination-scoped network shapes where provider-directory policy permits them; preserve secret and surveillance denials.

#### SG-000065 — Tool profiles and metadata contract

Pin each surface profile, annotations, OAuth scopes, effect classes, output bounds, and provider-specific allow/deny differences in tests.

#### SG-000066 — P16 security/parity exit

Prove practical development/workstation replacement without introducing unrestricted shell, unrestricted PowerShell, generic sockets, elevation, or remote approval.

P16 exit:

- Cotra can perform the intended everyday local development/workstation tasks without Desktop Commander;
- deliberately denied Desktop Commander-style authority is documented as a security choice, not a missing feature;
- public and local tool profiles are exact and regression-tested.

### COTRA-P17 — Provider distribution and v0.2 release

#### SG-000067 — OpenAI public plugin package

- production public endpoint integration;
- exact tool annotations/security schemes;
- OAuth and provider client verification;
- privacy/support/terms/security URLs;
- directory package and review fixtures;
- no claim of approval until OpenAI actually approves it.

#### SG-000068 — Claude distribution

- local Desktop Extension/config;
- remote custom connector qualification;
- same Cotra pairing and relay backend.

#### SG-000069 — Mistral distribution

- Vibe Code stdio/Streamable HTTP qualification;
- Vibe Work remote connector qualification.

#### SG-000070 — Codex and generic MCP distribution

- packaged local integration;
- generic MCP examples and interoperability tests.

#### SG-000071 — Cross-provider isolation qualification

Prove that provider A cannot reuse provider B credentials/session/route, and that identical local approval/policy boundaries apply across providers.

#### SG-000072 — v0.2 release hardening

- updated threat-model regression;
- dependency/license review;
- SBOM/provenance/reproducibility;
- relay artifact/container provenance where applicable;
- fresh Windows E2E;
- clean install/update/rollback;
- provider integration evidence;
- no known blocking findings.

#### SG-000073 — P17 exit and v0.2 canonical closeout

Close only after all software-controlled acceptance is proven. Directory approval/publication is recorded as external evidence and must never be fabricated.

P17 exit target:

- Cotra v0.2 software is release-qualified;
- local integrations are usable without Desktop Commander;
- remote relay is interoperable with the targeted hosted clients;
- the OpenAI plugin is submitted/approved/published only when external review actually establishes those states;
- Claude/Mistral availability is recorded from actual client evidence, not assumptions.

## 24. External blockers versus software completion

The plan distinguishes software completion from third-party directory decisions.

Cotra can complete implementation, security qualification, and release packaging without being able to force OpenAI, Anthropic, or Mistral to approve or distribute an integration.

For the user's specific goal of using Cotra in ordinary ChatGPT web conversations without developer mode, the OpenAI public plugin must actually be approved/published and available to that account/region/surface. Until then, that user-facing goal remains externally blocked even if Cotra software is complete.

This must be reported honestly.

## 25. Definition of done

The universal-access program is not complete merely because a relay responds or one provider can call one tool.

Completion requires:

- provider-neutral server builder;
- supported local stdio and loopback HTTP paths;
- local client packages/recipes;
- device identity, pairing, rotation, and revocation;
- standards-based remote OAuth;
- outbound-only authenticated device link;
- public Streamable HTTP MCP edge;
- open-source self-host relay;
- zero-cost reference deployment with hard no-billing behavior;
- Desktop Commander replacement parity inventory and safe capability expansion;
- static reviewed tool profiles and annotations;
- cross-provider isolation tests;
- chaos/offline/replay/route-confusion tests;
- privacy/logging/data-minimization review;
- updated threat model and supply-chain evidence;
- Windows E2E and release qualification;
- documentation that clearly separates local-free, self-hosted, and community-relay behavior;
- no known blocking security finding;
- no unsupported claim about provider review/availability.

## 26. Implementation start rule

Implementation begins only after this plan, the universal-connectivity threat model, and compatibility matrix are reviewed and merged through the existing exact-head qualification process.

SG-000047 is the grain that ratifies and activates this plan; no implementation grain proceeds until SG-000047 closes.

No P15 remote relay code may merge before P14's transport-neutral local contract is canonical. No P16 authority widening may merge before the capability/parity inventory is canonical. No public provider submission may occur before the remote threat model, auth, cross-tenant isolation, privacy policy, and production endpoint are qualified.
