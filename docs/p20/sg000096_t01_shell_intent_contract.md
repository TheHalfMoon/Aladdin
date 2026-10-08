# SG-000096 T01 — Pure Shell-Session Intent Contract

This slice is **contract-only**. It does not start processes, grant authority,
register MCP tools, accept caller-selected PIDs, load donor runtime, access the
network, or alter the release/installer. Safe process.spawn retains its exact
registered-executable, sanitized-environment and AppContainer boundaries.

## Owned Rust API

- `qdral_policy::shell_session::ShellSessionIntent` is versioned and rejects
  unknown JSON fields, extraneous caller-controlled authority, malformed
  opaque handles, NUL/control characters, ambiguous path previews, unbounded
  argv/script input, invalid environment-value digests, and invalid quotas.
- `shell_intent_approval_digest` is a SHA-256 **intent commitment**, with
  domain separation and a length-prefixed deterministic serialization of
  operation, executable SHA-256/path, shell kind, exact argv/script, lexical
  cwd, environment-value digests, limits, owner/device/session/workspace,
  policy revision, epoch, nonce, and generation expectations. **It is not an
  approval token or proof of grant**, and is insufficient to execute anything.
- `verify_shell_intent_binding` accepts a proposed intent only when all
  fields and its digest match a trusted expected binding. A fresh server-side
  nonce-consumption ledger and actual one-shot approval receipt are deferred
  to the later dispatch implementation; no T01 code claims replay protection
  solely from comparing a stateless SHA-256 digest.
- `authorize_shell_session_t01` rejects every authority mode, including
  Full User carrying an otherwise valid unexpired FullControlLease. It checks
  existing lease/context bindings but **has no success path**. The future
  profile ceiling is not runtime authorization.

## Hard native boundary for successors

The proposed cwd string is a **lexical preview**, not proof of canonical
filesystem identity. T02 must open and resolve executable/cwd handles on the
target Windows device, deny reparse/junction races, prove live SID and logon
generation, and bind a newly created process handle to a suspended child
assigned to the existing owned Windows Job Object before resuming it.
Unmanaged `shell:true`, donor `node:local`, command interpolation, foreign
PID termination and detached process launches are excluded.

T03 owns live session output, stdin, timeouts, cancel/revoke and bounded Job
Object teardown. T04 owns exact local caller-facing registration only after
T02/T03 native qualifications. Full User ambient filesystem and network
effects are **not** isolated merely by using a Job Object or environment
filter. Privileged and remote shells remain separate successor gates.

## Provenance

This T01 contract is Deskal-owned source written for the current Rust
policy/lease model; **no Desktop Commander source code was imported**.
Approved candidate donor pins, reuse decisions and MIT obligations remain
recorded in `docs/p20/sg000096_shell_session_import.json`, tied to issue
#262 and the SG-000096 activation merge `08086c19a6cb610aa2323e327dd02497f01f3740`.

Qualify this source with exact-head CI, genuine TypeSafe Jev, Alibaba Open
Code Review delegation plus manually reviewed excluded documents, frozen
Safe-process and SG-000095 desktop regressions, Windows native tests,
no unresolved review threads, SSH-signed DCO commits, a normal merge, and
post-merge CI before recording T01 as complete.
