# SG-000108 product caller: the live worker under Deskal's authority

Status: **PROPOSED, pending review** (revision 2, after a design review that blocked revision 1). This is a design record only. It adds no code, no tool, no listener and no authority. Each slice below is a separate implementation PR, qualified on its exact head.

## Why

SG-000108's `done_when` asks for a real Windows user journey that downloads and verifies a public PDF through the isolated browser, under the existing policies and approval broker. What exists today (closeout matrix on #290) does not reach that:

- **The live worker is proven** (A3–A6, A9–A11, #324), but only tests call `launch_worker`.
- **The browser host has its own approvals.** Its actuation checks its own one-shot `actuation::ApprovalToken`. Nothing ties that token to `qdral_approval::ApprovalBroker`, the broker repaired by #286.
- **`qdrald` drives no browser.** It already dispatches the twelve qualified browser shapes through the broker (`crates/qdrald/src/browser.rs`), with SOFT approval and exact digest binding. Its navigation only advances a state machine (`apply_navigation`). Its download writes caller-supplied bytes.
- **Nothing is exposed.** The SG-000079 live browser set is empty.

## What the code is today (facts the slices build on)

- **Host binary.** `qdral-browser-host` (`main.rs`) accepts only `--engine`, `--profile-dir` and `--host-id`. It spawns the engine itself (`spawn_engine_null`, SG-000074 `build_argv`) and serves only hello, ping and shutdown. It never calls `launch_worker`.
- **Supervision.** `Supervisor` (`host.rs`) spawns the host through `spawn_host_piped`, admits one live host at a time, and makes the host's profile with a marker (`setup_ephemeral_profile`, then `adopt_profile`). The host's environment sets TEMP to `%SystemRoot%\Temp`.
- **Worker frames.** Navigate, FetchText, Observe, Discover, Extract, Activate, Download and Execute. There is **no fill frame**.
- **What a live click can do.** `activate` follows only an observed, same-origin link; it does not model element state.
- **Pins.** The host pins every destination from its own DNS resolution: `AdmittedDestination::admit` is the only constructor, and the pin is fixed into `--host-resolver-rules` when the lifetime launches.
- **`qdrald`'s digests bind material only `qdrald` has:**
  - its profile identity and `page_id`;
  - `expected_state`;
  - its own resolved `pinned_address`;
  - for downloads, a declared `source_id` and the SHA-256 of caller-supplied bytes.
- **Host-side bindings for approval prompts already exist:** `download_binding` and `execution_binding`.
- **`qdrald` reads no profile.** It does not read `tool_surface_profile` (only `remote_session.rs` does, admitting `core` only). In `apps/qdral-mcp/src/tool_contract.ts`, `desktop_structured` is every contract key, so a browser tool added to the contract would silently widen it.

## Topology

```text
MCP client ──> qdrald                      (workspace, policy revision, approval broker, page store, profile gate)
                 │ Supervisor: spawn_host_piped, piped stdio, kill-on-close Job Object
                 ▼
               qdral-browser-host --worker (low authority: SG-000075 policy, own DNS pin, A10 window, host bindings)
                 │ launch_worker: private pipe, nested Job Object
                 ▼
               worker (Node, playwright-core) ──pipe──> installed Edge / Chrome (confined argv)
```

- **`qdrald` stays the only authority** a caller talks to. Only it mints and consumes approvals, with the broker.
- **The host never mints, renews or stores approvals.** Its checks guard the action against staleness and redirection; they are not a second authority. The host trusts its parent, which is the only process holding its pipe, to have consumed the approval.

## Decisions proposed

### 1. The host binary gains a worker mode (an authority delta of slice 1)

- **What `--worker` changes.** In `--worker` mode the host never spawns the engine itself, so there is never a second browser for `classify_job` to refuse. It serves capability frames over `launch_worker` instead. Arguments, all from `qdrald` and never from a caller:
  - `--node` and `--worker-main` (see decision 7);
  - `--profile-root`;
  - `--temp-dir`, a `qdrald`-owned staging root, so staging never lands in `%SystemRoot%\Temp`;
  - `--host-id`.
- **Profiles.** The host itself creates its ephemeral profile with `ephemeral_profile_dir(--profile-root)`. That keeps the creator's pid on the name, so the #298 sweep works. It also writes and re-checks the A9 fingerprint marker before launch, which this slice adds to the ephemeral path.
- **Destinations.** The destination arrives in an `open` frame, never in argv.
- **One host at a time.** `Supervisor` admits one live host per `qdrald` process, so slices 1–3 support one browser lifetime at a time. A second concurrent lifetime is refused, typed. Lifting that is a later decision.

### 2. The host's resolution decides the pin

- **`open` takes one destination.** The host resolves and admits it (`AdmittedDestination::admit`, with SG-000075's public-address rules), launches the worker with that pin, and replies with a lifetime nonce and the pinned address.
- **Approvals bind the host's pin.** `qdrald`'s navigation approval binds the host-reported pin, not `qdrald`'s own preview resolution, so round-robin DNS cannot make the two disagree after an approval is spent. `qdrald` still runs its preview for SG-000075 validation before `open`.
- **`open` needs no approval.** It loads only `about:blank`, and no consequential action happens. It is counted against a lifetime limit.
- **Cross-origin navigation.** Navigating a page to another origin ends the lifetime: shutdown, profile removed, and a new `open` with a fresh profile. The page id is kept and its generation advanced; its prior lifetime's node ids become stale.

### 3. Two-part approval binding

For each consequential action, `qdrald` and the host each contribute what only they know:

1. **The host binding comes first.** `qdrald` sends a `bind` frame describing the intended action. The host computes a host binding over its own material and returns it:
   - the lifetime nonce;
   - the action kind;
   - the committed document generation;
   - the host node identity;
   - the resolved target URL;
   - for downloads, the approved folder's real path, the file name and the size limit;
   - for any value-carrying action, the value's digest.
2. **`qdrald` builds the approved digest** as H(`qdrald`'s material ‖ host binding). Its material is the workspace, the policy revision, the page id and the store generation. It prompts the user, and the broker mints and consumes the approval as today.
3. **Dispatch.** `qdrald` sends the action frame with the broker record id and the host binding. The host re-derives its binding from its current state and refuses unless:
   - the binding is unchanged: no generation advance, no node change, no other lifetime, no other target;
   - the record id is unused in this lifetime.

**Why this holds:**
- **No replay across lifetimes:** the nonce is in every binding.
- **No redirection to another node or target:** the host re-derives its own binding.
- **No approval from the host:** it never mints one.
- **The shared code is small.** Only the framing of the two parts is shared, with a test vector in each crate. Each side's material stays its own.

**Consume-first.** `qdrald` consumes before dispatch, so a host refusal after consumption spends the approval. That fails closed. `qdrald` records the approval as "consumed, not dispatched".

### 4. What is live in slices 1–3

- **Live:**
  - navigate;
  - observe;
  - link activation (`browser.dom/click` on an observed link);
  - staged create-only download of an observed node.
- **Not live:**
  - `browser.dom/click` on a button, menu item, tab or checkbox returns typed unsupported live.
  - `browser.dom/fill` is deferred to its own slice. It needs a new worker frame, a protocol generation bump and a postcondition that detects a changed element.
  - Uploads stay model-only.

### 5. The live download changes `qdrald`'s download shape (authority delta, slice 3)

- **What changes:** the live form downloads an observed node of the committed document, keyed by node id. The current model-only form takes caller-supplied `body` bytes and binds their SHA-256.
- **What the approval binds:** the host binding (node, resolved URL, folder real path, file name, size limit), not a content digest. The bytes do not exist before approval.
- **What the evidence carries:** the committed file's SHA-256 and size.
- **The model-only form is retired for live pages:** the tables change together (decision 6).
- **Generations:** the store advances the page generation on a committed download, mirroring the host's `committed = (url, generation + 1)`.

### 6. Exposure and profile selection (slice 3)

- **`ToolSurfaceProfile`** gains `browser_structured`.
- **`desktop_structured`** becomes an explicit list without browser tools, instead of every contract key, so adding browser tools cannot widen it.
- **Profile selection.** A local transport selects `browser_structured` only by explicit local configuration, never by a caller.
- **`qdrald` gains a profile gate.** It reads the same table source and refuses live browser dispatch unless the request's profile is `browser_structured`. This closes the path where anything writing envelopes to `qdrald`'s stdin reaches a live browser.
- **OAuth.** `browser_structured` joins `UNMAPPED_REMOTE_PROFILES` (denied), so remote mapping stays disabled.
- **The SG-000079 tables change in one PR,** with the mechanical agreement tests:
  - `LIVE_BROWSER_MCP_TOOLS`;
  - discovery;
  - documentation;
  - the OAuth scope tables;
  - runtime.

### 7. Node and worker sources

- **Never from a caller, and never from a user-writable runtime setting.** `validate_launch` only checks a file name. A same-user config file could otherwise make `qdrald` run any executable inside the worker job.
- **Until SG-000106:** the Node and worker paths are compiled into a test-only build (decision 8). SG-000106 admits a signed, pinned Node runtime and the worker files, with hash or signature verification at launch.
- **Release builds** report the live browser as typed unavailable. This is stricter than SG-000108's scope, which allows a separately installed Node, and is intended until SG-000106 pins both.

### 8. Gating until the binding and exposure land

- **Slices 1 and 2** compile `qdrald`'s live backing only under a test-only cargo feature (`live-browser-test`). The host's fixture trust is already test-only.
- **Release qualification asserts the feature is absent,** as the spec's network delta requires for the fixture origin.
- **Release builds** of slices 1 and 2 therefore have no live browser path at all.
- **Slice 3** lifts the gate only together with the profile gate and the exposure change.

## Slices

1. **Host worker mode and the two-part binding, behind `live-browser-test`.**
   - The host binary gains `--worker`, the `open`, `bind`, navigate, observe, activate and download frames, the lifetime nonce and the ephemeral marker.
   - Native tests drive `qdrald`'s dispatch functions against the controlled fixture.
   - Binding negative tests:
     - a replayed record id;
     - a binding from another lifetime;
     - another node;
     - a stale generation;
     - another folder or file name.
2. **The full journey through `qdrald` with a real `LocalApprovalBroker`, still behind the feature.**
   - The presence verifier is scripted only in test builds.
   - Failure paths:
     - host death after a commit, reconciled (open question 1);
     - a dead lifetime marks its page stale;
     - the MCP-side timeout versus the worker's longer waits (open question 3).
3. **Exposure and the public journey.**
   - Decisions 5 and 6, and the feature gate lifted for development builds.
   - Native Windows evidence of the `done_when` journey on a public HTTPS site: a dated document list, the latest PDF downloaded create-only, and its bytes and SHA-256 verified by an independent checker. The site is chosen at that slice, and its URL and expected digest are recorded at evidence time.

## Open questions, each with an initial proposal

1. **Reconciliation after uncertainty.** If the host dies after `approved_folder::commit` renamed a file but before it replied, a committed file exists with no `qdrald` evidence. Proposal: `qdrald` records an intent (folder, file name, record id) before dispatch. After host death it checks the destination for that exact name, adopts it as committed and hashed or reports it absent, and never re-downloads automatically. Partial writes are already safe: the temp file is delete-on-close and the rename never replaces.
2. **Several `qdrald` processes.** Each MCP connection runs its own `qdrald` over a shared on-disk page store. Proposal: the store records which process and lifetime owns each page, and refuses a page owned by another live process, typed.
3. **Timeouts.** `KernelClient` waits 30 s by default; worker waits go up to 45, 75 or 90 s, plus approval time. Proposal: live browser requests get their own budget, and the action runs to its bounded completion. If the MCP side has given up, the outcome is still recorded in the store and the audit.
4. **FailureCode mapping.** Proposal:
   - `HostError::Unavailable(reason)` maps to Unavailable;
   - a timeout maps to ProcessTimeout;
   - a crashed or dead lifetime maps to TargetStale for the page;
   - an unverified postcondition maps to PostconditionFailed;
   - a host binding mismatch maps to ApprovalMismatch.
5. **Page data on disk and in logs.**
   - Raw accessible names stay in the host, keyed per lifetime.
   - `qdrald`'s stores receive only the bounded, redacted observation it already models.
   - `qdrald`'s stderr (forwarded by `kernel.ts`) must never contain URLs or page text.
6. **Process limits.** Proposal: raise the host's job limit to cover the nested worker job, which also has a limit of 32. Add a native test of the real `qdrald` → `spawn_host_piped` → host binary death path. The existing host-death test uses a test binary as the host.
7. **Idle timeout and per-workspace lifetime limits.**

## What stays out

- Personal or signed-in profiles (SG-000100).
- Script evaluation, generic CDP, and selectors as authority.
- Remote browser control and relay mapping.
- Uploads beyond SG-000078's recorded sources.
- The SG-000106 packaging itself.
