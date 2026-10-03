# SG-000022 — Origin-bound bounded navigation security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P08
Grain: SG-000022

## Purpose

SG-000022 establishes bounded browser navigation on top of the SG-000021
isolated automation profile, typed provider contract, and origin binding: a
typed page identity lifecycle, SOFT-approved origin-bound transitions,
hop-by-hop redirect-chain validation, and download-trigger denial, while
keeping DOM observation, DOM actuation, downloads, uploads, personal-profile
mode, debugging, scripting, credential access, MCP browser tools, and network
egress absent.

No browser is launched or attached in this grain. Navigation transitions page
identity and origin in the Qdral registry only. Actual page rendering or page
loading by a browser engine remains absent and is successor work.

## Page lifecycle

Every page is bound to the isolated automation profile, one workspace, its
current origin, and a server-side lifecycle generation. Pages are allocated
by `browser.page.open` with no network activity and a per-workspace bound of
16 open pages. The page identity is always server-allocated with a `pg-`
prefix; only identities present in the registry file under Qdral protected
local state are valid. Caller-supplied strings that are absent fail closed
as stale handles with `TargetStale`.

Fresh pages start with an empty current origin and generation zero in the
`open` state. Each successful navigation bumps the generation and moves the
page to `active` with the validated final origin. Closed pages never become
valid again. Replaced generations invalidate old handles: a caller that
echoes a stale expected origin or expected generation fails closed before
any approval prompt and again after approval before mutation.

Foreign workspaces fail closed with `WorkspaceDenied`. Foreign profile
identities fail closed with `CapabilityDenied`. Policy revision drift fails
closed with `TargetStale`. Unknown pages fail closed with `TargetStale`.

## Navigation preview

`browser.navigation.preview` is local read-only stale-protection material.
It validates the target URL with full origin binding, DNS re-resolution, and
post-resolution address policy, plus optional redirect-chain validation,
without mutating page state and without requiring approval. The preview
returns the current origin and generation, the validated target origin, the
deterministically pinned public address, the redirect count, and the final
origin. The caller uses this material to build a fresh navigation request
with exact expected state.

## SOFT-approved navigation

`browser.navigation.navigate` requires a fresh SOFT approval with exact
digest binding over workspace, policy revision, profile identity, page
identity, expected origin and generation, target origin, pinned address,
redirect chain, and final origin. The digest uses length-prefixed SHA-256
fields under the `QDRAL_BROWSER_NAVIGATION_V1` domain. Any material drift
invalidates the approval.

The dispatch flow fails closed in order: unknown or closed pages, foreign
workspace or profile, policy drift, stale expected origin or generation,
target validation with SSRF policy, pinned-address drift against the
preview, download-trigger denial, fresh SOFT prompt with one-shot
consumption, post-approval stale re-check, and only then generation bump
with typed evidence. Reused approvals fail closed through one-shot
consumption. Expired approvals fail closed through expiry. Caller-supplied
approval fields such as `approval`, `token`, `nonce`, or `digest` are
rejected as authority widening with `CapabilityDenied`. The agent cannot
synthesize approval because only the local broker mints tokens through the
platform prompt.

## Origin binding and SSRF

Navigation reuses SG-000021 origin binding: exact `scheme://host:port`
identity with normalized lowercase hosts, no userinfo, no fragments, no
wildcards, no zone identifiers, no trailing dots, and rejection of alternate
numeric host syntaxes. Validation re-resolves through an injectable resolver
and applies post-resolution address policy: loopback, unspecified,
multicast, link-local, private, broadcast, documentation, shared,
benchmarking, and reserved addresses fail closed, including IPv4-mapped IPv6
forms. The pinned address is the sorted-first public address. No explicitly
trusted workspace browser destination is configured, so every non-public
destination is denied.

## Hop-by-hop redirect validation

Every redirect hop must share the exact validated origin. Each hop reparses,
checks exact origin equality, checks loop repetition, re-resolves, and
reapplies address policy. Scheme downgrade from HTTPS to HTTP fails closed
through exact origin mismatch. Alternate ports fail closed. Deceptive host
suffix and prefix tricks fail closed through exact equality, never prefix
comparison. Loops fail closed. Chains longer than 8 hops fail closed.
Malformed Location equivalents, userinfo targets, and unsafe DNS results
fail closed. Multi-hop widening fails closed at the first widening hop.
Download-trigger suffixes in any hop fail closed.

## Download-trigger denial

Executable, script, and archive destination paths are denied so navigation
cannot become a bypass around download policy. The check inspects only the
URL path component before query strings: `.exe`, `.msi`, `.msix`, `.dll`,
`.sys`, `.ps1`, `.bat`, `.cmd`, `.vbs`, `.vbe`, `.js`, `.jse`, `.wsf`,
`.wsh`, `.zip`, `.7z`, `.rar`, `.tar`, `.gz`, `.cab`, `.iso`, and `.img`
suffixes fail closed with `CapabilityDenied` for targets, redirect hops,
and final destinations.

## Explicit denial

All DOM observation, DOM actuation, download, upload, scripting, debugging,
and egress shapes fail closed as denied capability shapes with the STRONG
class and no SOFT downgrade. This includes `browser.navigate`,
`browser.snapshot`, `browser.dom` verbs, `browser.download`,
`browser.upload`, `browser.profile/use_personal`, `browser.debug`,
`browser.devtools`, `browser.cdp`, `browser.script`, `browser.launch`,
`browser.attach`, `browser.clear`, `browser.external_request`,
`browser.network`, and the bare `devtools`, `cdp`, and `playwright`
families, plus `browser.page/close` and `browser.navigation/back` siblings
that are not authorized in this grain.

No MCP browser tool is registered. Even a forged kernel request carrying
browser capability reaches only the policy denial above, so the agent cannot
satisfy navigation capability through MCP tools, synthetic input, UI
automation, coordinate proposals, child processes, or replayed prior
approvals. JavaScript URLs, personal profile paths, caller-selected profile
roots, browser argv, credential material, and arbitrary CDP remain
unreachable.

## Evidence

Navigation evidence carries only page identity, workspace, policy revision,
profile identity, prior origin and generation, target origin, pinned
address, redirect count, final origin, new generation, and approval record
linkage. Cookies, Authorization headers, credentials, tokens, passwords,
DOM content, and raw browser internals never enter evidence, prompts,
records, history, logs, or MCP responses.

## Retention

SG-000018 one-shot, expiry, digest, workspace, and policy bindings,
SG-000019 STRONG enforcement with no SOFT downgrade, SG-000020 STRONG-gated
trust changes with epoch-bound revoke, and SG-000021 profile and destination
behavior are unchanged. Browser navigation policy is the only new authority,
and it introduces no observation, actuation, download, upload,
personal-profile, debugging, scripting, credential, network-egress, reuse,
delegation, or elevation capability.
