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
- `tests/sg000108_worker_host.rs`, run by `npm run browser-launcher:native` as a second, feature-enabled pass (2 tests). It drives the real binary under `Supervisor` against the fixture and checks:
  - open: 32-hex nonce, pin 127.0.0.1, the host-pid-named profile;
  - a second open is refused, typed;
  - navigate returns 200, and another origin is refused;
  - observe returns no raw page text;
  - close: job empty, profile removed;
  - a mid-lifetime shutdown takes the worker and the profile with it;
  - a second host is refused.
- **Without the feature:** `worker_mode_does_not_exist_without_the_test_feature` shows that `--worker` is an unknown argument.
- **Release guard:** `the_live_browser_test_feature_never_reaches_a_release_build` (release-blocker sweep) is a source scan. It allows the feature to be named only in:
  - the host crate's own definition and code;
  - its tests;
  - the native-suite launcher;
  - documentation.

  In the manifest it may appear only as its own empty definition: no default and no other feature enables it. No other crate depends on the host crate, and the release build compiles only `qdrald` and `qdral-lifecycle`.
- **Cleanup paths:**
  - a refused `open` removes the profile it made (`a_refused_open_leaves_no_profile_behind`);
  - `Supervisor::shutdown` and its drop also shut a worker-mode host down.
- **Unit tests under the feature:** the native launcher also runs the feature-enabled `--lib` unit tests (the closed vocabulary, closed error codes, frame before hello).
- **Follow-ups recorded:**
  - **L1, the blocking hello and shutdown reads.** `Supervisor`'s hello and shutdown reads block without a deadline, as the classic handshake already does. They need a reader thread with `recv_timeout`, as `BrowserWorker` has. Slice 1b.
  - **L3, the committed URL.** `Navigated.url` is the committed URL, which a same-origin redirect chooses. Slice 1b decides whether `qdrald` receives it or only the requested URL and a digest.
- **sg000085:** its certificate-flag rule accepts the feature gate, `cfg(any(test, feature = "live-browser-test"))`, besides `cfg(test)`.
