# SG-000108 product caller: the live worker under Deskal's authority

Status: **PROPOSED, pending review.** This is a design record only. It adds no code, no tool, no listener and no authority. Each slice below is a separate implementation PR, qualified on its exact head.

## Why

SG-000108's `done_when` asks for a real Windows user journey that downloads and verifies a public PDF through the isolated browser, under the existing policies and approval broker. What exists today does not reach that.

**What exists (closeout matrix on #290):**
- The live worker is proven: A3–A6, A9–A11, and the #324 network evidence.
- Its only callers are tests. `launch_worker` is called only by the native suite.
- The browser host's actuation path checks its own one-shot `actuation::ApprovalToken`. Nothing ties that token to `qdral_approval::ApprovalBroker`, the broker repaired by #286.
- `qdrald` already dispatches the twelve qualified browser shapes through the broker (`crates/qdrald/src/browser.rs`), with SOFT approval and exact digest binding. But it drives no browser: navigation updates a state machine (`apply_navigation`), and nothing is fetched or rendered.
- The SG-000079 live browser set is empty (`LIVE_BROWSER_MCP_TOOLS = []`).

**The product caller connects these** without moving any authority out of the Rust core.

## Topology

```text
MCP client ──> qdrald                      (workspace, policy revision, approval broker, MCP dispatch)
                 │ supervised spawn, piped stdio, kill-on-close Job Object
                 ▼
               qdral-browser-host          (SG-000074 binary: SG-000075 navigation policy, launch_worker, A10 window)
                 │ launch_worker: private pipe, nested Job Object
                 ▼
               worker (Node, playwright-core) ──pipe──> installed Edge / Chrome (confined argv)
```

- **`qdrald` stays the only authority** a caller talks to. It resolves the workspace, checks the shape against the active profile, previews with re-resolution, and mints and consumes the approval with the broker, exactly as today.
- **`qdral-browser-host` stays low-authority.** It never mints, renews or stores approvals. It keeps the SG-000075 navigation policy, the A10 engine window and the confinement argv, and refuses any action whose approval material does not match its own view of the action.
- **The worker and engine stay as qualified:** the same frames, the same argv, the same job.

## Decisions proposed

1. **Supervision.**
   - `qdrald` spawns `qdral-browser-host` through the existing `spawn_host_piped` supervision (piped stdio, kill-on-close Job Object). It runs one host per browser lifetime, with exactly one admitted destination.
   - The host's ephemeral profile comes from `ephemeral_profile_dir` under `qdrald`'s browser profile root (#298). The engine comes from `discover_engine`, and is re-verified at launch (A10).
   - End of the host's input, or the death of `qdrald`, takes the host, the worker and the engine down. That is already proven for the host-death path.
2. **Host vocabulary (an authority delta of slice 1).** The SG-000074 hello/ping/shutdown vocabulary gains closed capability frames, one per qualified shape that is to become live:
   - open a lifetime for one destination;
   - navigate;
   - observe;
   - click;
   - fill;
   - download, staged create-only into the approved folder.

   Frames carry typed fields only. There are no selectors, no script and no CDP. Unknown frames end the host, as today. Uploads stay model-only in the first slices.
3. **Approval binding.** The broker stays the single source of approvals.
   - For each consequential frame (navigate, click, fill, download), `qdrald` consumes the broker approval first, then sends the host the consumed record id and the approved digest.
   - The host recomputes the digest over its own action material: the committed document generation, the host node identity, the resolved target, and the bounded value or destination.
   - It refuses the frame unless the two digests are equal and the record id has not been used in this lifetime.
   - So an approval cannot be replayed into the host, cannot be redirected to another node or target, and cannot be minted by the host.
   - The digest formula is shared code. `qdrald` and the host compute it with the same function, with a test vector in each crate.
4. **The state machine becomes live-backed, not replaced.** `qdrald`'s existing page store, generations and evidence records stay. A live lifetime advances them only on the host's reply: committed URL, generation, observation or committed download. Model-only behaviour remains for any shape not yet live.
5. **Exposure.** The shapes become reachable only through the explicit `browser_structured` profile. Profiles `core` and `desktop_structured` gain nothing. One PR changes all of these together, with the SG-000079 mechanical agreement tests:
   - `LIVE_BROWSER_MCP_TOOLS`;
   - discovery;
   - documentation;
   - the OAuth scope tables;
   - runtime.

   Remote mapping stays disabled.
6. **Runtime prerequisites.** Until SG-000106 admits a signed, pinned Node runtime and puts the worker in the payload, the live browser works only where Node 22.14 or later and the built worker are present.
   - Release builds report the browser as typed unavailable.
   - The `done_when` journey is evidenced on the Windows evidence host, with a development build.

## Slices

1. **Host capability frames and `qdrald` supervision, with no exposure.**
   - The host serves the frames over `launch_worker`. `qdrald` can supervise it.
   - Native tests drive `qdrald`'s dispatch functions directly against the controlled fixture.
   - The live set stays empty.
2. **Approval binding.**
   - The shared digest function and host-side verification, with negative tests: a replayed record id, a digest for another node, another destination, and a stale generation.
   - The native journey runs through `qdrald` with a real `LocalApprovalBroker`, using a scripted presence verifier in test builds only.
3. **Exposure and the public journey.**
   - The `browser_structured` profile flips live, with every table together.
   - Native Windows evidence of the `done_when` journey on a public HTTPS site: a dated document list, the latest PDF downloaded create-only, and its bytes and SHA-256 verified by an independent checker.
   - The site is chosen at that slice and recorded with its URL and the expected digest at evidence time.

## What stays out

- Personal or signed-in profiles (SG-000100).
- Script evaluation, generic CDP, and selectors as authority.
- Remote browser control and relay mapping.
- Uploads beyond SG-000078's recorded sources.
- The SG-000106 packaging itself.

## Open questions for review

- **Lifetime scope.** Should a lifetime be bound to an MCP session or to a page id? The proposal: a page id, one destination, ended by close or idle timeout.
- **Discovery inputs.** How does `qdrald` find the worker entry point and Node in development builds? The proposal: explicit configuration, verified like the native suite's prerequisites. It must never come from a caller.
- **Idle timeout and per-workspace lifetime limits.**
