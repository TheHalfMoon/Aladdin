# SG-000108 product caller: the live worker under Deskal's authority

Status: **PROPOSED, pending review** (revision 3: revision 1 was blocked in design review; revision 2 was closed with follow-ups, all settled here). This is a design record only. It adds no code, no tool, no listener and no authority. Each slice below is a separate implementation PR, qualified on its exact head.

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

- **What `--worker` changes.** In `--worker` mode the host never spawns the engine itself, so there is never a second browser for `classify_job` to refuse. It discovers the engine itself (`discover_engine`; there is no `--engine` argument), and A10 re-verifies it at launch. It serves capability frames over `launch_worker`. Arguments, all from `qdrald` and never from a caller:
  - `--node` and `--worker-main` (see decision 7);
  - `--profile-root`;
  - `--temp-dir`, a `qdrald`-owned staging root, so staging never lands in `%SystemRoot%\Temp`;
  - `--host-id`.
- **Profiles.** The host itself creates its ephemeral profile with `ephemeral_profile_dir(--profile-root)`. That keeps the creator's pid on the name, so the #298 sweep works. It also writes and re-checks the A9 fingerprint marker before launch, which this slice adds to the ephemeral path.
- **Destinations.** The destination arrives in an `open` frame, never in argv.
- **One host at a time.** `Supervisor` admits one live host per `qdrald` process, so slices 1–3 support one browser lifetime at a time. A second concurrent lifetime is refused, typed. Lifting that is a later decision.
- **`Supervisor` worker mode.** Today `Supervisor::launch` and `shutdown` create and remove the host's profile themselves. A worker-mode variant leaves the profile to the host, and keeps the spawn, the job, the single-host rule and the deterministic shutdown.

### 2. The host's resolution decides the pin

- **`open` takes one destination.** The host resolves and admits it (`AdmittedDestination::admit`, with SG-000075's public-address rules), launches the worker with that pin, and replies with a lifetime nonce and the pinned address.
- **Approvals bind the host's pin.** `qdrald`'s navigation approval binds the host-reported pin, not `qdrald`'s own preview resolution, so round-robin DNS cannot make the two disagree after an approval is spent. `qdrald` still runs its preview for SG-000075 validation before `open`.
- **`open` needs no approval.** It loads only `about:blank`, and no consequential action happens. It is counted against a lifetime limit.
- **Cross-origin navigation.** A lifetime admits exactly one destination, and one host runs at a time. So navigating a page to another origin works as follows:
  1. The current lifetime is shut down (profile removed) before the new one is opened and the prompt is shown, because the approval must bind the new lifetime's pin.
  2. The page becomes stale at that point, **whether the user then approves or denies**. A denied cross-origin navigation leaves the page without a live lifetime, and the next action on it needs a new navigation.
  3. The preview tells the user this before they decide.
  4. On approval, the page id is kept and its generation advanced. The prior lifetime's node ids are stale for good.

  A second, brief lifetime just for the switch would break the one-host rule and is not proposed.

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
- **The caller-supplied-bytes form is gone for live pages.** It is unreachable under `browser_structured`, which lists only the live form. The tables change together (decision 6).
- **`browser.download/preview` stays.** Under `browser_structured` it returns the host binding (the `bind` frame) and what the prompt will show. It writes nothing and consumes nothing.
- **Generations:** the store keeps its own page generation, which only increases. It records the host generation it corresponds to in the current lifetime. A committed download advances both, mirroring the host's `committed = (url, generation + 1)`. A new lifetime restarts the host generation, and the store maps it again from the next reply.

### 6. Exposure and profile selection (slice 3)

- **`ToolSurfaceProfile`** gains `browser_structured`.
- **`desktop_structured`** becomes an explicit list without browser tools, instead of every contract key, so adding browser tools cannot widen it.
- **Profile selection.** A local transport selects `browser_structured` only by explicit local configuration, never by a caller.
- **`qdrald` gains a profile gate.** It refuses live browser dispatch unless the request's profile is `browser_structured`. This closes the path where anything writing envelopes to `qdrald`'s stdin reaches a live browser.
  - **The source of truth for live shapes** is the TypeScript tool contract (`tool_contract.ts`), as for every other profile.
  - `qdrald` receives the profile in the envelope and checks it against a Rust list of the live shapes.
  - A cross-language agreement test fails if the Rust list, the contract's `browser_structured` list and `exposure.rs`'s `LIVE_BROWSER_MCP_TOOLS` differ. The SG-000079 tests already do this between the contract and the documentation.
- **Why local configuration may pick a profile but never a program.** Selecting `browser_structured` widens only which shapes a caller may ask for, and every consequential one still needs a broker approval from the user. A Node or worker path decides what code runs inside the worker job, with no approval in between, so it is never configurable (decision 7).
- **OAuth.** `browser_structured` joins `UNMAPPED_REMOTE_PROFILES` (denied), so remote mapping stays disabled.
- **The SG-000079 tables change in one PR,** with the mechanical agreement tests:
  - `LIVE_BROWSER_MCP_TOOLS`;
  - discovery;
  - documentation;
  - the OAuth scope tables;
  - runtime.

### 7. Node and worker sources

- **Never from a caller, and never from a user-writable runtime setting.** `validate_launch` only checks a file name. A same-user config file could otherwise make `qdrald` run any executable inside the worker job.
- **Until SG-000106: a build-time pin.** A non-default cargo feature, `live-browser-dev`, compiles the Node and worker paths in at build time, together with the SHA-256 of `node.exe` and of the worker's `main.js`, and the host verifies both hashes before every launch.
  - Without the feature there is no live browser path; this is how a development build is told apart from a release build.
  - The release build commands never enable it, and release qualification asserts it is absent.
  - A changed Node or worker file is refused, typed, until rebuilt.
  - SG-000106 later replaces this with a signed, pinned Node runtime and the worker files in the payload.
- **Release builds** report the live browser as typed unavailable. This is stricter than SG-000108's scope, which allows a separately installed Node, and is intended until SG-000106 pins both.

### 8. Gating until the binding and exposure land

- **Slices 1 and 2** compile `qdrald`'s live backing only under a test-only cargo feature, `live-browser-test`. The feature exists in both crates: in `qdral-browser-host` it also enables fixture admission and fixture-certificate trust in the **binary**, which today are `cfg(test)` only.
  - The fixture pin can be made only by `AdmittedDestination::fixture`, `#[cfg(test)] pub(crate)`.
  - The host binary that `Supervisor` spawns is not built with `cfg(test)`.
- **Release qualification asserts the feature is absent from both crates' artifacts,** as the spec's network delta requires for the fixture origin.
- **Release builds** of slices 1 and 2 therefore have no live browser path at all.
- **Slice 3** lifts the gate only together with the profile gate and the exposure change. Even then, the live path exists only in `live-browser-dev` builds (decision 7).

## Slices

1. **Host worker mode and the two-part binding, behind `live-browser-test`.**
   - The host binary gains `--worker`, the `open`, `bind`, navigate, observe, activate and download frames, the lifetime nonce and the ephemeral marker.
   - Native tests drive `qdrald`'s dispatch functions against the controlled fixture.
   - **Feature relationship.** `live-browser-test` does not enable `live-browser-dev`. Native tests take Node and the worker from `QDRAL_SG108_NODE` and `QDRAL_SG108_WORKER_MAIN`, a source compiled only into test builds and unreachable from a `live-browser-dev` or release build.
   - **No gap between check and launch.** The hashed `node.exe` and worker files stay open, without write or delete sharing, from the hash check until the worker has started. Otherwise the launched handle is the one verified. The engine's A10 `reverify` adopts the same hold.
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
   - Decisions 5 and 6. Live browsing becomes available in `live-browser-dev` builds (decision 7), and remains absent from release builds.
   - Native Windows evidence of the `done_when` journey on a public HTTPS site: a dated document list, the latest PDF downloaded create-only, and its bytes and SHA-256 verified by an independent checker. The site is chosen at that slice, and its URL and expected digest are recorded at evidence time.

## Open questions, each with an initial proposal

1. **Reconciliation after uncertainty.** If the host dies after `approved_folder::commit` renamed a file but before it replied, a committed file exists with no `qdrald` evidence. Proposal: `qdrald` records an intent (folder, file name, record id) before dispatch. After host death it checks the destination for that exact name, and never re-downloads automatically. It reports the file as **"present, unverified"** with its SHA-256 and size, or as absent. `qdrald` cannot tell the host's committed file from one a same-user process placed there, so it never records the file as a committed download. Partial writes are already safe: the temp file is delete-on-close and the rename never replaces.
2. **Several `qdrald` processes.** Each MCP connection runs its own `qdrald` over a shared on-disk page store. Proposal: the store records which process and lifetime owns each page, and refuses a page owned by another live process, typed.
3. **Timeouts.** `KernelClient` waits 30 s by default; worker waits go up to 45, 75 or 90 s, plus approval time. Proposal: live browser requests get their own budget, and the action runs to its bounded completion. If the MCP side has given up, the outcome is still recorded in the store and the audit.
4. **FailureCode mapping.** Proposal, using existing `qdral_contracts::FailureCode` variants only, with no contract change:
   - `HostError::Unavailable(reason)` maps to `ProviderUnavailable`;
   - a timeout maps to `ProcessTimeout`;
   - a crashed or dead lifetime maps to `TargetStale` for the page;
   - an unverified postcondition maps to `PostconditionFailed`;
   - a host binding mismatch at dispatch maps to `TargetStale`, because the approved action no longer matches the page, after the approval was consumed;
   - a binding that cannot be computed before the prompt maps to `InvalidRequest`.
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

## Implementation status

**Slice 1a: host worker mode (test builds only).** Its authority delta is the worker-mode host vocabulary, generation 2.

**What it adds:**
- **Feature.** `qdral-browser-host` gains the test-only `live-browser-test` feature. With it, the binary has a `--worker` mode, and fixture admission and fixture-certificate trust become available to that mode.
- **Worker mode (`worker_host.rs`).** It serves a closed generation-2 vocabulary: hello, `open` (loopback fixture only), navigate, observe, close and shutdown. It allows one lifetime at a time.
  - **Profile.** Each lifetime runs in the host's own `ephemeral_profile_dir` profile, carrying the A9 marker that is checked by an adopt-back before launch.
  - **Lifetime nonce.** A 128-bit nonce from the system RNG.
  - **What crosses.** Observations cross as identities and digests only. A reply that does not fit one frame drops nodes and says so. Errors cross as closed codes, never as message text.
- **Supervision.** `Supervisor::launch_worker_host` keeps the single-host rule across both modes and leaves the profile to the host.

**What it does not add:**
- the `bind` frame and the two-part binding (slice 1b);
- link activation and download (slice 1b);
- any `qdrald` dispatch;
- any exposure.

**Evidence:**
- `tests/sg000108_worker_host.rs`, run by `npm run browser-launcher:native` as a second, feature-enabled pass (4 tests). It drives the real binary under `Supervisor` against the fixture and checks:
  - open: 32-hex nonce, pin 127.0.0.1, the host-pid-named profile;
  - a second open is refused, typed;
  - navigate returns 200, and another origin is refused;
  - observe returns no raw page text;
  - close: job empty, profile removed;
  - a mid-lifetime shutdown takes the worker and the profile with it;
  - a second host is refused.
- **Managed-policy precondition.** The host binary reads the real registry (the SG-000074 managed-policy precondition), so on a host with a managed browser policy every `open` is refused, typed, before anything starts. CI's windows-latest runner has one.
  - There, the worker-host tests require exactly `unavailable:managedpolicy` with no profile left, and still check the single-host rule and shutdown.
  - The full lifetime journey is evidenced on the evidence host, which has no managed policy. This mirrors `the_real_registry_precondition_decides_the_launch`.
- **Without the feature:** `worker_mode_does_not_exist_without_the_test_feature` shows that `--worker` is an unknown argument.
- **Release guard:** `the_live_browser_test_feature_never_reaches_a_release_build` (release-blocker sweep) is a source scan. It allows the feature to be named only in:
  - the host crate's own definition and code;
  - its tests;
  - the native-suite launcher;
  - documentation.

  In the manifest it may appear only as its own empty definition: no default and no other feature enables it. No other crate depends on the host crate, and the release build compiles only `qdrald` and `qdral-lifecycle`.
- **Cleanup paths:**
  - a refused `open` leaves no profile:
    - checks that need no profile come first (`a_refused_open_leaves_no_profile_behind`);
    - a launch refused after the profile exists removes it (`a_launch_refused_after_the_profile_exists_removes_it`). With only that removal deleted, the test fails (mutation run);
  - `Supervisor::shutdown` and its drop also shut a worker-mode host down.
- **Unit tests under the feature:** the native launcher also runs the feature-enabled `--lib` unit tests (the closed vocabulary, closed error codes, frame before hello).
- **Follow-ups recorded:**
  - **L1, the blocking hello and shutdown reads.** `Supervisor`'s hello and shutdown reads block without a deadline, as the classic handshake already does. They need a reader thread with `recv_timeout`, as `BrowserWorker` has. Slice 1b.
  - **L3, the committed URL.** `Navigated.url` is the committed URL, which a same-origin redirect chooses. Slice 1b decides whether `qdrald` receives it or only the requested URL and a digest.
- **sg000085:** its certificate-flag rule accepts the feature gate, `cfg(any(test, feature = "live-browser-test"))`, besides `cfg(test)`.

**Slice 1b, host side: two-part binding and the bound actions (test builds only).** The worker-mode vocabulary moves to generation 3; generations 1 and 2 are refused at hello.

**`open` takes `policy_revision`.** It is bound into every host binding, together with the lifetime nonce and the workspace.

**New frames:**
- **`bind`** returns the host binding and the action's target, for the prompt. The binding is SHA-256 over a domain tag, the lifetime nonce, the workspace, the policy revision, and the action's material as the host sees it now:
  - **navigate:** the URL and the host's pin;
  - **click:** page, origin, document generation, node, role, and the link's resolved destination;
  - **download:** the same, plus the host's `download_binding` (the link, the approved folder's real path, the normalized file name).
- **`select_latest_dated_link`** is the host's typed selection. Raw names never leave the host. It replaces any caller-supplied selector.
- **`navigate`, `activate` and `download`** each carry the broker record id and the binding. The host refuses:
  - a record id already presented in this lifetime: `approval_replay`;
  - a binding it can no longer derive, or that differs: `binding_mismatch`. That covers another URL, node, file name or lifetime, and a page that moved on.

  A record id is spent once presented. Only then does the host build its own SG-000077 target and run its existing activation or download path: create-only commit, origin, type and size checks.
- **`downloaded`** returns the relative name, the SHA-256, the size and the source URL.

**`navigated` keeps returning the committed URL.** It is always on the lifetime's single origin, because the policy refuses anything else, and `qdrald`'s page store records it. *Decision on 1a follow-up L3.*

**Supervision has deadlines.** `LiveWorkerHost` reads replies on a reader thread. Hello and shutdown wait at most 30 s, and each request at most `WORKER_HOST_REPLY_TIMEOUT` (150 s, above the worker's longest bound). A silent host is a typed timeout. *1a follow-up L1.*

**Evidence,** worker-host pass with 6 tests:
- **`the_journey_downloads_the_latest_report_through_bound_approvals_and_verifies_it`:**
  - the latest dated report, selected by the host, is committed create-only through a bound download;
  - an independent checker compares the bytes with the fixture's report and its SHA-256, `5523ba2d…994a`;
  - the same record is then refused as a replay;
  - a fresh record with the old binding is refused as a mismatch;
  - a second download to the same name is refused, and the file is unchanged.
- **`a_binding_names_one_lifetime_node_and_file_and_nothing_else`:**
  - a navigation binding presented for another URL is refused;
  - a download binding presented for another file name or another node is refused;
  - a binding from a previous lifetime is refused;
  - nothing was written.
- **Mutation runs:**
  - with the binding comparison removed, the binding test fails: another URL was navigated;
  - with record ids never remembered, the journey's replay check fails.
- **The launcher accepts the pass only with each lifetime test's evidence line,** or with the managed-policy refusal line. With `QDRAL_SG108_REQUIRE_JOURNEY=1`, set per run on the evidence host, the journeys themselves are required. *1a follow-up.*

**What remains for slice 1b:** `qdrald`'s dispatch over these frames, behind the feature, with the broker's record ids.

**Slice 1b host side, review fixes (round 1).**
- **Replies stay in step.** Replies carry no request id, so a reply that missed its deadline would have been read as the answer to the next request.
  - `LiveWorkerHost` reads through a `ReplyChannel` that is poisoned after any timeout or channel failure. Every later request on that host is refused, and only shutdown remains (`is_usable` reports it).
  - Unit tests cover the late reply and a failed frame. The mutation that leaves the channel unpoisoned fails both.
- **Only sent nodes can be acted on.** When an observation reply drops nodes to fit one frame, the host forgets every node it did not send (`retain_host_nodes`). `bind`, `activate`, `download` and selection can therefore only target nodes Deskal received.
- **What runs is what was verified.** Dispatch builds the host's SG-000077 target from the state verified against the binding: the link's destination, or the host's `download_binding`. It never re-derives that state. The actuation path then checks the verified state against its current state, so a folder junction swapped between verification and dispatch is refused.
- **The size limit is bound.** Download bindings include the size limit, `MAX_TRANSFER_BYTES`, as decision 3 lists.
- **The digest is a pure function.** `binding_digest`'s unit test calls it directly. With the workspace dropped inside `binding_digest`, the test fails. If `host_binding` passed an empty workspace instead, no test would notice yet, because every lifetime under test uses one workspace and the nonce already separates lifetimes.
- **Replies are also poisoned on a failed or partial write.**
- **Follow-ups, closed in the qdrald-side PR:**
  - **The workspace at the call site.** The binding test recomputes the host binding independently from the documented formula (nonce, workspace, revision, action) and requires an exact match. With the workspace replaced by an empty string where `host_binding` is called, the test fails (mutation run).
  - **A trimmed observation.** The fixture's `/crowded` page does not fit one reply frame. The host now trims depth-first, dropping the last leaf first, so a reply keeps the largest prefix of the tree: 483 of 512 nodes. Dropping whole top-level subtrees, the earlier behaviour, could empty a page whose content sits under one root. `nodes_a_trimmed_observation_dropped_cannot_be_acted_on` requires every node the host observed but did not send to be unbindable. Without `retain_host_nodes`, the dropped node `d1-e486` was bindable, and the test fails (mutation run).
- **Click path.** `a_bound_click_follows_the_approved_link_once`: a bound activation opens exactly the approved link. Its replay is refused, and the old binding with a fresh record is refused.
- **Exact codes.** The other-origin navigation and the create-only refusal both assert the exact code `invalid`: the binding matched, and the policy or the commit refused.
- **Launcher.** Under a managed policy, every lifetime-opening test must report the refusal itself: four journeys plus the shutdown test.

**Slice 1b, qdrald side: the approval broker in front of the host (test builds only).**

**The feature.** `qdrald` gains a test-only `live-browser-test` feature, which enables the host crate's feature, and an optional dependency on the host crate. A default or release `qdrald` build has neither. The release-blocker sweep allows exactly that feature definition and that optional dependency in `qdrald`'s manifest.

**`qdrald/src/live_browser.rs` runs decision 3's two-part approval for each consequential action:**
1. The host binds the action (`bind`).
2. `qdrald` builds the approval digest: SHA-256 over a domain tag, the workspace, `qdrald`'s policy revision, the lifetime nonce, the action, and the host binding.
3. It prompts. The broker mints the approval, then consumes it against that exact digest, workspace and revision.
4. Only then does it send the frame, with the broker's record id and the host binding.

So a denied, unavailable or mismatched approval never reaches the host. Host refusals map onto existing `FailureCode`s: binding mismatch → `TargetStale`, replay → `ApprovalDenied`, timeout → `ProcessTimeout`, unavailable → `ProviderUnavailable`.

**No envelope dispatch yet.** The native tests drive the module directly. The live-backed page store arrives in slice 2, and exposure in slice 3.

**Deferred to slice 2, together with the page store** (review of this PR):
- **`qdrald`'s half of the digest** does not yet bind the page id or the store generation that decision 3 lists. There is no store yet. Today it binds the workspace, the policy revision, the lifetime and the action.
- **The "consumed, not dispatched" record and reconciliation** (open question 1) are not implemented.
  - **What already fails closed:** a lost reply poisons the host connection, so later actions fail at `bind`, before any prompt, and no further approval is spent.
  - **What is still open:** a download the host committed whose reply was lost leaves a file with no evidence record.

**The download prompt names the destination.** Its summary carries the source URL, the approved folder and the file name, which are also bound into the digest.

**Evidence: a fourth native pass, `qdrald` live tests, 2 tests.** The launcher first builds the host binary with the feature, then hands its path to the tests through a test-only variable, `QDRAL_SG108_HOST_BINARY`.
- **`the_journey_runs_under_the_broker_and_commits_the_verified_report`:** broker-approved navigation, observation, the host's selection, and a broker-approved, create-only download. An independent checker verifies the bytes and the SHA-256, `5523ba2d…994a`.
- **`a_denied_or_mismatched_approval_never_reaches_the_host`:**
  - a denied navigation and a denied download fail with `ApprovalDenied`;
  - an approval whose consume fails, because the token's digest was changed, is refused;
  - nothing was written;
  - the lifetime stays usable, and the next approved download commits.
- **Mutation run:** with the broker's consume removed, the mismatched approval's download is committed, and the test fails.
- **Under a managed policy** (CI), both tests require the typed refusal instead. The launcher requires their evidence lines, or the refusal lines. With `QDRAL_SG108_REQUIRE_JOURNEY=1` it requires the journeys.

**Slice 2a: the real ledger, lost replies and reconciliation (test builds only).**

**The real broker in tests.** `qdral-approval` gains a test-support-only constructor, `LocalApprovalBroker::with_path_and_soft_decider`. It replaces exactly one step, the SOFT consent dialog (a system-modal `MessageBoxW`), with a scripted decision. Everything else is the real broker: the ledger, the nonce, expiry, single use, digest and scope checks, and the records. STRONG approvals still need the presence verifier.
- `test-support` is enabled only through dev-dependencies, and the workspace uses resolver 2, so release builds never contain it.
- Unit tests over a real ledger: an approval is consumed once, a second consume is refused, a wrong digest is refused, the record is marked consumed, and a scripted denial issues nothing.

**A lost reply ends the lifetime.** `qdrald`'s `LiveBrowser` dispatches every consequential frame through one path, which records each consumed approval's outcome: Executed, Refused (closed code), or Lost. This is decision 3's "consumed, not dispatched" record.
- A lost reply (timeout, or a dead or broken host) ends the lifetime. The next action fails with `TargetStale` before any prompt, so no further approval is spent.
- **A lost reply also ends the host.** A silent host may not be a dead one: a suspended or slow host could still commit the file after qdrald gives up on it. So on any lost reply `LiveBrowser` shuts the whole host family down at once (job termination if it does not leave by itself) and records whether its end was confirmed. That record is set once and never revised, because a second shutdown finds no host and would report a vacuous success.
- Every dispatch error path records Lost, including a dispatch that found no host.
- Transport failures (a broken pipe, a platform fault) map to `ProviderUnavailable`; a silent host maps to `ProcessTimeout`.

**Reconciliation.** A download's intent (record id, folder, file name) is recorded before its frame is sent. After a lost reply, `reconcile()` reports what is at that destination: present, unverified (with SHA-256 and size), or absent. It never re-downloads and never records the file as committed.
- **Only after the host's end is confirmed.** Until then `reconcile()` refuses with `ProviderUnavailable` and keeps the intent; it is never silently dropped.
- **Path safety.** The file name must pass the host's own destination validator (`validate_relative_destination`: no root, traversal, alternate data streams, trailing dots or spaces, reserved device names, control or format characters). No segment below the approved folder may be a reparse point, intermediate segments must be directories, and the name must be a regular file; anything else is refused, not reported.
- The file is opened without following a reparse point (`FILE_FLAG_OPEN_REPARSE_POINT`), the opened handle's own metadata is checked again, and the hash is streamed from that handle, never past `MAX_TRANSFER_BYTES`. A larger file is refused.

**Evidence: the `qdrald` live pass, 4 tests.**
- **The journey now runs on the real ledger** (scripted consent). The ledger holds exactly the two approvals, both consumed. Both outcomes are Executed. Reconciling the committed name reports exactly the committed file's SHA-256 and size.
- **`a_host_lost_after_the_consume_is_recorded_reconciled_and_ends_the_lifetime`:**
  - A test-only hook kills the host after the broker consumed the download's approval and before the frame reached it.
  - The approval is consumed in the ledger, and its outcome is Lost.
  - The lifetime is over: the next navigation is `TargetStale`, and no new prompt is recorded.
  - Reconciliation reports the destination absent.
  - The host process itself is confirmed exited (a handle opened before the kill is waited on), and the family's end is confirmed before reconciling.
  - **Mutation run:** with the lifetime kept after a lost reply, the test fails.
- **`a_suspended_host_is_ended_before_its_lost_download_is_reconciled`** (the judge's reproduction of the gap, now a regression test):
  - The hook suspends the host (`NtSuspendProcess`) instead of killing it, so the download's reply is lost while the host is alive. A test-only reply budget (5 s) stands in for the 150 s production one.
  - The outcome is Lost with `ProcessTimeout`, the family's end is confirmed, and the suspended process itself has exited.
  - Reconciliation reports the destination absent, and it is still absent 3 s later.
  - **Mutation run:** with the shutdown skipped (the end recorded as confirmed without it), the test fails: the suspended host is still running.
- **Unit tests:** the reconciliation gate (running or unconfirmed: refused and the intent kept), a hostless dispatch recorded Lost, the I/O and platform error mapping, and path safety (streams, trailing dots and spaces, reserved names, an intermediate junction, a junction at the name, the size limit at and past `MAX_TRANSFER_BYTES`). Mutations removing the gate, the size limit, or the hostless record each fail a test.
  - **Not covered:** removing the explicit reparse-attribute check is not caught, because junctions and symlinks are already refused as neither directory nor file. The check remains as defense in depth for reparse points that are not name surrogates (cloud placeholders such as OneDrive's, deduplicated files); the unit test cannot create one portably, so it is uncovered.
  - **Residual (accepted, judge round 2):** intermediate segments are checked by path and the file is then opened by path, so a same-user process swapping an intermediate directory for a junction in between could have an outside file's hash and size reported as present, unverified. That process could equally place the file at the destination itself, and the result is only reported, never adopted. Optional later hardening: compare the opened handle's final path with the canonical folder.
- **Deferred from slice 2a to slice 2b** (done below): the page id and store generation in `qdrald`'s half of the digest, the live-backed page store, and persisting the intent across a `qdrald` restart. The MCP-side timeout budget (open question 3) moved on to slice 3.

**Slice 2b: the page store and the intent journal (test builds only).**

**The lifetime's page lives in the page store.** `open` allocates a page in the SG-000022 `PageStore`, bound to the workspace and to the lifetime (profile identity `live-lifetime:{lifetime}`).
- Every approved navigation applies the committed document to the store: a new origin and a bumped generation.
- A lost reply or a shutdown closes the page. A closed page is stale for good, including after a reload from disk (`PageStore::close_page`).
- If the store cannot follow the host (a write fails, or the committed URL does not parse), qdrald ends the lifetime rather than act on a page record that no longer matches.

**The page store's half of the digest.** `qdrald`'s approval digest is now `deskal-sg108-live-approval/2`. It binds the page id, the generation and the origin, as well as the workspace, policy revision, lifetime, action and host binding. An approval prompted for one document cannot be spent on the next.

**Registry writes fail closed.** `PageStore::persist` used to drop write errors silently, so the in-memory registry could claim a state its file did not hold. A failed or unsynced append now fails the transition with `ProviderUnavailable` before the in-memory state changes. This applies to every page-store caller, not only the live browser.

**The intent journal (open question 1 across restarts).**
- **Durable before the approval is spent.** A download's `pending` line (record id, folder, file name, and the host's pid and creation time) is appended and synced before the approval is consumed. If it cannot be written, the approval is not consumed and nothing is sent.
- A reply, whether success or refusal, appends `settled`. An approval that failed to consume appends `abandoned`. A report of the destination appends `reconciled`, so each lost download is reported once.
- **Recovery at start.** `LiveBrowser::start` reconciles every open intent whose host is gone before launching a new host.
  - `qdrald` holds the host's job, which is set to kill its processes when the job closes. When `qdrald` ends, the kernel ends the host family, so after a restart the recorded host is gone.
  - "Gone" is checked against the OS, not assumed. The pid either names no running process, or names one with a different creation time (a reused pid).
  - An intent whose host still runs belongs to a live lifetime, possibly another `qdrald` process's (open question 2), and is left untouched. A query the OS refuses to answer counts as running.
  - A report that cannot be made yet (the destination is unreadable for now) stays open for the next run.
- **Torn lines.** Only a torn final line, from a crash mid-append before the approval was consumed, is skipped. Any other unreadable line, unknown event, unknown schema, or a pending line without its destination fails closed.

**Open question 3 (the MCP-side timeout budget) moves to slice 3.** No MCP path reaches the live browser until exposure lands there, so the budget is designed together with the `browser_structured` shapes it applies to.

**Evidence: the `qdrald` live pass, 5 tests.**
- **The journey** now also checks:
  - the page store: generation 0 with an empty origin at open; after navigation, generation 1 with `https://fixture.test:443` and Active; Closed on disk after shutdown;
  - an unwritable journal refuses the download with `ProviderUnavailable`, leaves its approval unconsumed in the ledger, and sends no frame;
  - the download's journal lines are exactly `pending` then `settled`, with nothing left open.
- **The lost test** also checks that the page is gone (closed on disk) and that the intent stays open until reconciled.
- **`a_restart_reconciles_the_download_a_lost_host_left_open`:**
  - the host is killed after the consume, and `qdrald`'s browser ends without reconciling, so only the journal remains;
  - the next `LiveBrowser::start` reconciles that exact record as absent, and nothing is left open;
  - a third start reports nothing.
- **Unit tests:**
  - the journal fold (settled, abandoned and reconciled close an intent; a torn tail is skipped; five kinds of corruption fail closed);
  - liveness (running, exited, and a reused pid by creation time);
  - recovery (gone-present reported with SHA-256 and size; gone-absent; reused pid; a live host untouched; each reported once);
  - the page store: close is idempotent and survives a reload, a closed page refuses navigation and actuation, and an unwritable registry changes nothing;
  - the digest covers page id, generation and origin, with a fixed vector computed independently.
- **Mutation runs,** each failing a test:
  - recovery that ignores host liveness;
  - recovery that does not journal its report;
  - a lost lifetime that leaves its page open;
  - consuming the approval before the intent is durable.

**Still deferred to slice 3:**
- the profile gate;
- `browser_structured` exposure;
- the MCP timeout budget;
- routing the SG-000022 page shapes to the live page;
- the public-PDF journey.
