# SG-000108 engine attachment and resolved-address enforcement decision

Status: PROPOSED; security review passed on PR #291. This record resolves
the first two `open_questions` of `.specgrain/specs/SG-000108.json`. The
record itself granted no authority and admitted no dependency. The frozen
SG-000074 command line in `crates/qdral-browser-host/src/argv.rs` stays
unchanged. Implementation progress is tracked at the end of this record.

## Evidence

The evidence comes from throwaway spikes outside the repository. They ran on
Windows 11 (10.0.26300) against the installed Microsoft Edge 155.0.4283.45
with `playwright-core` 1.63.0. The package was installed with
`--ignore-scripts`, and its lockfile integrity equals the admitted
`sha512-rYCsBF/M…nyxo5mCg==`. No browser binary was downloaded, and no Edge
or Chrome policy key was present (`HKLM`/`HKCU\Software\Policies\Microsoft\Edge`,
`HKLM\Software\Policies\Google\Chrome`). The implementation PR turns each
probe into a native Windows test.

The probes behind the second, third and fifth rows ran with the policy route
removed, so they test layers 1 and 2 alone. They used the full proposed flag
set from the next section.

| Probe | Result |
|---|---|
| `launchPersistentContext` with `ignoreDefaultArgs: true`, the seven frozen flags, `--user-data-dir`, `--remote-debugging-pipe`, `about:blank` | Launches and is driven, and an ARIA snapshot of a fixture page is returned. The OS-reported engine command line is exactly those arguments: no `--enable-automation`, no debugging port or address, no Playwright default flag. |
| Resolver rules mapping only admitted names, then `MAP * ~NOTFOUND` | A real public name (`example.com`) and unmapped names fail. The fixture saw only admitted requests. |
| Proxy `http://0.0.0.0:9` with an exact bypass list plus `<-loopback>`, and hostile TCP listeners on `0.0.0.0:9`, `127.0.0.1:9` and `[::]:9` | IPv4 literal, IPv6 literal (`[::1]`), IPv6 metadata (`[fd00:ec2::254]`) and unmapped names fail with `ERR_PROXY_CONNECTION_FAILED` in 55–75 ms. **Zero** connections reach any listener. The admitted name still connects directly. |
| `MAP <name> <ip>:<port>` | A request for the admitted name on another port is delivered to the pinned port. Without the port, any port is reachable. Pins therefore carry `:443`. |
| WebRTC STUN to IP literals (loopback and LAN), and TURN over TCP | Without a policy flag, UDP reaches an arbitrary address (4 packets), bypassing layers 1 and 2. On Edge 155, `--force-webrtc-ip-handling-policy=…` had no effect. `--webrtc-ip-handling-policy=disable_non_proxied_udp` yields zero candidates, zero packets and zero TURN-TCP connections. |
| WebTransport | Not reachable from the HTTP fixture: it requires a secure context. **Untested.** It is a required HTTPS-fixture test (see below). |

## Decision 1: engine launch and attachment (option a)

**Launcher.** The private worker launches the verified engine through
`playwright-core` `launchPersistentContext` over `--remote-debugging-pipe`.

**What the host still owns:**
- engine discovery and signature verification;
- profile adoption and the fingerprint check;
- the scrubbed environment;
- the complete argv.

**What the worker receives.** The worker gets the engine path, the profile
directory, the argv and the environment from the host, and passes them
unchanged with `ignoreDefaultArgs: true`. The worker has no code path that
adds a flag.

**Drift detection, not containment.** After launch, the host compares the
engine's OS-reported command line with the argv it built. It also enumerates
every process in its Job Object and refuses any second browser-process engine
(an engine process without `--type=`). This catches drift, for example a
Playwright upgrade that adds default flags, a wrong engine, or a stray
process. It does **not** defend against a compromised worker:
- the worker holds a full-access handle to the engine, so it could rewrite
  the command line the OS reports after the engine has parsed it;
- a compromised worker is a full-user process and needs no browser to reach
  the network.

Worker compromise is a residual risk (below).

**Supervision.** The worker runs in the host's kill-on-close Job Object
without breakaway, so the engine and its child processes inherit the job.

**Option (b) rejected.** Option (b) would have the host launch the engine and
hand the pipe to the worker. It was rejected because Playwright has no public
API for an inherited pipe: it would need a private adapter that breaks on
upgrades, and the drift check above gives the same assurance.

### SG-000074 argv amendment

This is the explicit authority delta of the implementation. It is made
without widening the SG-000074 path: `ALLOWED_FLAGS` and `assert_argv_clean`
stay frozen and keep refusing every flag below. A separate, exact worker-launch
builder (`build_worker_argv`) emits the frozen flags plus exactly these six,
and `assert_worker_argv_exact` accepts only that argv. `FORBIDDEN_FLAGS` is
unchanged:
`--remote-debugging-port`, `--remote-debugging-address`, `--enable-automation`
and `--disable-component-update` stay forbidden.

| Flag | Value |
|---|---|
| `--remote-debugging-pipe` | none |
| `--host-resolver-rules` | host-built for this launch: `MAP <name> <pin>:443, …, MAP * ~NOTFOUND` |
| `--proxy-server` | fixed: `http://0.0.0.0:9` |
| `--proxy-bypass-list` | host-built for this launch: `<name>;…;<-loopback>` |
| `--webrtc-ip-handling-policy` | fixed: `disable_non_proxied_udp` |
| `--disable-quic` | none |

A host-valued flag is accepted only when its value equals the string the host
built for that launch. A fixed-value flag is accepted only with its fixed
value. A flag present by name only, or with any other value, is refused.

## Decision 2: actual-traffic enforcement of SG-000075

Three layers, each fail-closed on its own, plus the required flags and
launch preconditions below.

1. **Host-pinned resolution, port-pinned.** The engine never resolves names.
   - For each admitted destination, the host resolves the name and chooses
     the pin with the existing `select_public_address`. Public addresses
     only: loopback, private, link-local, metadata and mapped forms are
     denied by `is_public_address`.
   - The host emits `MAP <name> <pin>:443` for each name, then
     `MAP * ~NOTFOUND`. This enforces SG-000075's port-443 rule at the
     resolver.
   - DNS rebinding is impossible within an engine lifetime because the map is
     fixed at launch.
   - A redirect, frame or subresource to a name outside the map fails.
   - Widening the set means a host-mediated relaunch after a fresh
     `check_rebinding_consistent` check, never an in-place change.
2. **No route for IP literals or unmapped names.** The host sets
   `--proxy-server=http://0.0.0.0:9` and
   `--proxy-bypass-list=<admitted names>;<-loopback>`.
   - Only admitted names connect directly. Their address and port come from
     the layer 1 pin (`:443`), not from the bypass list.
   - Everything else, IPv4 and IPv6 literals included, goes to `0.0.0.0:9`,
     which accepted no connection even with listeners bound to `0.0.0.0:9`,
     `127.0.0.1:9` and `[::]:9`.
   - `<-loopback>` removes the engine's implicit loopback bypass.
   - On Linux, connecting to `0.0.0.0` reaches the local host, so this
     choice is valid only for the Windows release.
3. **Policy gate.** The worker's context-level route consults the host's
   SG-000075 decision for every request it sees: navigation, redirect,
   frame, popup, subresource and download. Layer 3 carries policy (scheme,
   origin, exact host, download rules); layers 1 and 2 carry addresses.
   - `serviceWorkers: "block"` is set.
   - WebSocket is denied through the context WebSocket route until a later
     grain admits mediated WebSocket.
   - Layer 3 does not see every request: preconnect, keepalive after unload,
     and internal fetches are not routed. Layers 1 and 2 therefore must hold
     without it, and the tests prove that.

**Required flags.**
- `--webrtc-ip-handling-policy=disable_non_proxied_udp`: WebRTC UDP otherwise
  bypasses layers 1 and 2.
- `--disable-quic`: SG-000075 denies QUIC.

Permission-gated page APIs stay denied by `permission_allowed`.

**Launch precondition: no mandatory policy.** Managed browser policy
outranks command-line settings. A machine proxy policy would turn layer 2
into a working proxy, which resolves names itself and so skips layer 1 too.

Before every launch, the host reads the mandatory policy keys for the
verified engine's family:
- `HKLM` and `HKCU\Software\Policies\Microsoft\Edge` for Edge;
- `HKLM` and `HKCU\Software\Policies\Google\Chrome` and
  `\Software\Policies\Chromium` for the Chromium family (Google Chrome reads
  the first, Chromium builds the second).

It refuses to launch, with a typed error, when **any** mandatory policy
value or subkey is present. This is simpler, and fails closed better, than
keeping lists of proxy, DNS, WebRTC and QUIC policy names.
`Recommended` subkeys do not override command-line settings, but the first
slice refuses them as well.

Cloud-managed policy that is cached locally outside these keys is a
hypothesis to test. Release qualification therefore also runs the layer 1
and 2 probes against the installed engine, which catches any override in
effect whatever its source.

### Hostname and value hygiene

- **Hostnames.** The host builds the rule and bypass strings from validated
  ASCII LDH hostnames only:
  - lowercase;
  - labels of 1 to 63 characters, 253 characters in total;
  - at least two labels;
  - a last label that is not all-numeric and does not start with `0x`, so
    that a name can never be parsed as an IPv4 address.
  - This also rules out wildcards and the separators `,` `;` and whitespace.
- **Addresses.** Values come from typed `IpAddr`, formatted by Rust. IPv6
  pins are bracketed.
- **Construction.** No page, caller or worker string reaches any flag.

### Test-only fixture pinning

Fixtures listen on loopback, which `is_public_address` correctly rejects.
Tests therefore need a test-only pin source. It must be compiled only into
test builds (a `cfg(test)` path or a non-default feature that the release
build never enables). A release-qualification check must prove that it is
absent from the shipped binaries. No runtime configuration may enable it.

## Residual risks (for review)

- **WebTransport to admitted names (#300, accepted by the founder on
  2026-10-11).** A page can open a WebTransport session, over QUIC on UDP,
  to an admitted name. The traffic reaches only that name's host-pinned
  address, because the resolver refuses unadmitted names and the no-route
  proxy refuses IP literals. Its contents are not inspected by the SG-000075
  policy route, which denies the equivalent WebSocket traffic. No Chromium
  switch disables the API, and patching it out of pages and workers in
  JavaScript has known bypass classes. Revisit if an engine-level measure
  appears. The native probe enforces the boundary.
- **Admitted host is hostile.** An admitted name pinned to its public address
  still serves whatever that host returns. Content is never policy (SG-000076
  redaction and SG-000077 approvals still apply).
- **Worker compromise.** A compromised worker equals user-level code with
  network access. The drift check does not contain it. Mitigations are the
  pinned, integrity-checked dependency, the scrubbed environment, no CLI and
  no install scripts. The implementation PR must evaluate running the worker
  under a restricted token or AppContainer without network capability, and
  record the outcome.
- **Same-user processes.** The debugging pipe is not reachable by other users
  or the network. But a same-user, medium-integrity process can duplicate the
  pipe handles out of the worker or engine and take full CDP control. This
  matches the existing threat model, where same-user code is not a security
  boundary. Tests must show the pipe handles are not inherited by the
  engine's child processes.
- **Browser-internal update and reputation traffic.** The no-route proxy also
  blocks:
  - component updates, including CRLSet revocation data (so SG-000074's
    choice not to pass `--disable-component-update` has no effect while
    this decision stands);
  - Safe Browsing / SmartScreen lookups.

  This is an explicit decision for the first slice. Admitted traffic stays
  TLS-verified, and downloads are staged and host-checked (size, type,
  digest) rather than trusted to SmartScreen. Revocation freshness relies on
  the OS certificate stack. Admitting specific update endpoints is a later
  grain decision.
- **Engine drift.** A future Edge may change how it honours these flags
  (the WebRTC switch name already differs from the `--force-` spelling). The
  A10 supported version window must re-run every probe per supported engine
  version and refuse versions where any probe fails.
- **Relaunch cost.** Relaunching to widen the destination set costs latency
  in exchange for no in-place mutation. This is accepted for the first
  journey (A6), whose destinations are known up front.

## Tests the implementation PR must add (native Windows, fixture origin only)

- **Argv.** The exact argv, and the OS command line equal to it, before and
  after the navigation. Refusal of every new flag without its host-built or
  fixed value. Refusal of every forbidden flag, as today. Refusal of a second
  browser-process engine in the job.
- **Layers 1 and 2, with the policy route disabled by a test-only hook.**
  Every case below is denied:
  - an unmapped name and a real public name;
  - IPv4 and IPv6 literals;
  - private, loopback and IPv4/IPv6 metadata addresses;
  - another port of an admitted name, which lands on the pinned port;
  - a connection to listeners bound on port 9 (zero accepts).
- **Rebinding.** A fixture resolver whose answers change; the engine keeps
  the pin.
- **WebRTC.** STUN and TURN (UDP and TCP) yield zero packets and zero
  connections.
- **QUIC and WebTransport.** An HTTPS fixture shows no QUIC and a refused
  WebTransport session, both to an admitted name and to an unmapped name.
- **Service workers and WebSocket.** A service worker registration and a
  WebSocket connection are refused.
- **Policy precondition.** A test registry hive with any mandatory Edge or Chrome policy makes the
  launch refuse.
- **Supervision.** The engine is a descendant of the worker inside the host
  job; killing the host leaves no engine process; the pipe handles are not
  inherited by engine child processes; the measured process count stays
  within SG-000074's job process limit.
- **Hostname hygiene.** Names with numeric, hex or wildcard labels, or
  containing separators, are refused.

## Implementation progress

**Slice 1: host-side confinement contract** (`crates/qdral-browser-host/src/confinement.rs`).
- `AdmittedDestination::admit(url, resolver)` is the only public
  constructor. It runs the existing `validate_navigation`, which resolves the
  name and pins its lowest public address with `select_public_address`. So
  every pin comes from the host's own resolution of that name. It then
  re-checks https, port 443 and `is_public_address`, and applies the stricter
  hostname hygiene.
- `resolver_rules`, `proxy_bypass_list` and `build_worker_argv` emit the
  six flags with the exact values in the table above.
- `assert_worker_argv_exact` refuses any argv that differs element for
  element from the host-built one.
- `assert_no_managed_policy` with `RegistryPolicySource` reads the Edge, Chrome or
  Chromium mandatory policy keys in HKLM and HKCU, both registry views, with
  `KEY_READ` only. It refuses on any value or subkey, and fails closed when
  a key is unreadable or the platform is not Windows.
- The SG-000074 `build_argv` and `assert_argv_clean` are unchanged and still
  refuse every confinement flag (a unit test proves this).
- The fixture pin constructor is compiled only into unit tests.
- No launch path uses this module yet, so the slice grants no authority.

**Native check of the emitted format (Edge 155).** An IPv4 pin and a
bracketed IPv6 pin (`MAP six.example [::1]:<port>`) each reached only their
fixture. Unmapped names failed with `ERR_PROXY_CONNECTION_FAILED`.

**Slice 2a: private worker package** (`apps/qdral-browser-worker`). It is standalone, like `apps/web`, so the identity-pinned npm workspaces stay unchanged.
- **Dependency.** `playwright-core` is exactly 1.63.0, with the admitted integrity. It is the only production package in the lockfile, has no install script and no runtime dependencies. All of this is checked mechanically against `dependency_admission`.
- **Protocol.** SG-000074 framing (u32 LE length + JSON, 64 KiB) with a closed generation-1 vocabulary:
  - host → worker: `hello`, `launch {engine, profile_dir, argv, env}`, `ping`, `shutdown`;
  - worker → host: `hello`, `launched`, `pong`, `bye`, `error {code}`.
  Unknown frames or fields are violations.
- **Launch check (defense in depth).** The worker re-checks the launch frame against this contract:
  - the frozen flags and the profile;
  - each resolver entry, parsed with the host grammar: an admitted-host name, an IPv4 or bracketed IPv6 pin, `:443`, and `MAP * ~NOTFOUND` last;
  - a bypass list exactly equal to those names in order, followed by `<-loopback>`;
  - the fixed proxy, WebRTC and QUIC values, and the blank page;
  - engine and environment allowlists, and absolute paths without dot segments.

  Address class stays the host's policy, because test builds pin fixtures to loopback. The worker then hands the argv to Playwright unchanged, with `ignoreDefaultArgs: true`. There is at most one engine per worker. Any violation, refusal, unexpected error, broken pipe, engine exit or end of input closes the engine.
- **Console isolation.** The global console is replaced by one that writes only to stderr. Verified: `console.dir`, `table`, `trace` and `log` put 0 bytes on stdout.
- **Native smoke run (Edge 155, local).** `dist/main.js` went through `hello` → `launch` → `ping` → `shutdown` with exit 0. The OS reported one browser process with the host argv, with no debugging port and no `--enable-automation`. No engine process was left after shutdown.

- **CI.** The root `npm test` runs the worker's install (`npm ci --ignore-scripts`), typecheck and tests. Both Node jobs, ubuntu and windows, therefore cover it without a workflow change. A dedicated `npm audit` of the worker lockfile in the supply-chain job is **pending**: it needs a workflow edit, which the current push token cannot make (no `workflow` scope). Until then the audit is a local check (0 vulnerabilities), and the lockfile admits only `playwright-core`, which has no dependencies.

**Findings that bind slice 2b:**
- The OS command line quotes arguments that contain spaces (the resolver rules), so the host must parse it with `CommandLineToArgvW` rules before the element-for-element comparison.
- Node acts on `NODE_OPTIONS` (verified with `--require`) before any worker code runs. The host must therefore build the worker environment from an allowlist and never inherit it; the worker's own refusal is only a second line.
- Inspector activation: **decided and enforced by the worker in slice 2b-i**. The worker refuses to start (exit 2, nothing on stdout) unless Node is 22.14+ and its Node options are exactly `--disable-sigusr1` (`startupRefusal`, with tests). A later `--no-disable-sigusr1` would otherwise re-enable activation (verified), and no `--inspect*`, `--require` or `--import` may accompany the flag. The slice 2b-ii launcher passes the flag and reports the browser as typed unavailable when the worker refuses.
- Service workers: Playwright's `serviceWorkers: "block"` only replaces `navigator.serviceWorker.register` with an init script, so page script may be able to bypass it. Block service workers by engine or protocol means, and prove it with a native probe.
- Worker environment allowlist: also exclude the OpenSSL start-up variables (`OPENSSL_CONF`, `OPENSSL_MODULES`, `OPENSSL_ENGINES`, `SSL_CERT_FILE`, `SSL_CERT_DIR`). The worker refuses them as a second line.
- Release packaging (A2): the `playwright-core` NOTICE, the SBOM entry and provenance enter the release with the packaged worker. Packaging strips the `node_modules/.bin` link to the Playwright CLI, which is never exposed.

**Slice 2b-i: native confinement probes in CI** (`apps/qdral-browser-worker/src/native.test.ts`). These run against the installed Edge through the root `npm test`, so the **Node / windows-latest** job produces real-engine evidence. On Windows a missing engine fails the suite. On other platforms the suite is skipped, because only Windows results count. The probes use owned loopback fixtures on port 443 and an argv the worker contract accepts.

1. **Lifecycle.** The worker drives the engine through `hello` → `launch` → `ping` → `shutdown`.
   - Exactly one browser process exists. Its OS command line, split with `CommandLineToArgvW` rules, equals `[engine, ...argv]`.
   - No process carries `--remote-debugging-port`/`-address` or `--enable-automation`.
   - Exit code 0, and no engine process remains.
2. **Worker kill.** Killing the worker takes the engine down with its pipe. The Job Object of slice 2b-ii remains the guarantee.
3. **Layers 1 and 2 with no policy route at all.** The admitted name reaches the fixture, and another port of it lands on the pinned port. All of the following fail:
   - an unmapped name;
   - a real public name;
   - IPv4 and IPv6 literals (including the fixture's own address and port);
   - a private address;
   - IPv4 and IPv6 metadata addresses.

   Listeners on `0.0.0.0:9`, `127.0.0.1:9` and `[::]:9` accept nothing, and the fixture sees only the two admitted requests.
4. **WebRTC.** STUN to loopback and to the first LAN address, and TURN over TCP, gather no ICE candidate, send no UDP packet and open no TCP connection.
5. **Inspector.** With `--disable-sigusr1`, `process._debugProcess` from a same-user process is refused and no listener appears on 9229. A negative control without the flag shows that the probe detects an activated inspector.

**Evidence validity (review of PR #295).**
- Each probe that could pass vacuously has a control:
  - WebRTC is re-run with the default policy, outside the contract, and must then gather candidates and reach the STUN listener. ICE gathering is awaited to completion, not a fixed sleep. The UDP trap listens on every interface, so the LAN case can be observed.
  - The inspector control activates a plain Node process and must see a listener.
- Each denied destination must fail with `ERR_PROXY_CONNECTION_FAILED` specifically, so that a regression in layer 2 cannot hide behind layer 1.
- A failing process query throws instead of reporting "no processes".
- Cleanup runs last-in, first-out and always runs every step, so a failed probe cannot leave an engine, a server or a profile behind. A fresh run leaves 0 profiles.

**Contract tests not yet covered (slice 2b-ii, with the host launcher):**
- comparing the command line after a navigation;
- checking that every engine process is a descendant of the worker inside the host Job Object;
- TURN over UDP;
- QUIC and WebTransport, and the service-worker block (these need an HTTPS fixture);
- the managed-policy refusal on a test hive;
- DNS-rebinding resistance with a fixture resolver whose answers change;
- refusal of a second browser-process engine in the job;
- the job process limit;
- whether pipe handles are inherited.

**Local result (Windows 11, Edge 155):** all 25 worker tests pass, 5 of them native, and the run exits cleanly in about 45 s.

**Slice 2b-ii-a: host launcher** (`crates/qdral-browser-host/src/worker_launch.rs`, `worker_process.rs`). The launch sequence:

1. Validate the request. A missing Node or worker is typed `UnsupportedRuntime`; a missing engine is `NoEngine`.
2. Apply the managed-policy precondition. Any policy, or an unreadable key, is typed `ManagedPolicy` and refuses before anything starts.
3. Build the engine argv with `build_worker_argv`.
4. Build the worker environment from an explicit allowlist (SystemRoot, SystemDrive, PATH=System32, TEMP, TMP). It is never inherited.
5. Start Node with exactly `--disable-sigusr1` and the worker entry point:
   - **suspended**, then assigned to a kill-on-close Job Object (the SG-000074 limit of 32 active processes), then resumed;
   - with `DETACHED_PROCESS`, so there is no console and no `conhost.exe`;
   - with inheritance restricted by `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` to its stdin, stdout and a `NUL` stderr.
6. Exchange `hello`, then `launch`, over the closed protocol with bounded waits (15 s handshake, 60 s launch). A worker that exits with code 2 before answering is typed `UnsupportedRuntime`; a silent worker is typed `LaunchFailed` and its job is terminated.
7. **Verify the job** (`classify_job`):
   - its only members are the worker, running with exactly its host-built arguments, and processes of the verified engine image;
   - no member carries a forbidden flag, in any spelling Chromium accepts on Windows (`--`, `-` or `/` prefix, any case);
   - exactly one browser process exists, where the process type is derived the way Chromium derives it: switches end at a bare `--` or at `--single-argument` (matched in any spelling, which fails closed), the last `type` switch in any accepted spelling wins, and an absent or empty value means the browser;
   - that process's OS command line, read with `NtQueryInformationProcess` and split with `CommandLineToArgvW`, equals `[engine, …host argv]`;
   - a member that cannot be inspected is accepted only if it has left the job in the meantime.

The engine in a launch request is a `discovery::Engine`, so only an engine that discovery verified (kind, version, publisher) can be launched. `node` and the worker entry point must come from Deskal's install records.

Verification runs once, when the launch completes. Afterwards, only the job limit and kill-on-close constrain the worker. The product caller must call `verify` again before any authority is granted for a navigation.

Shutdown asks for `bye` and waits for exit 0. It then terminates the job regardless and confirms it is empty, and finally removes an ephemeral profile. Kill and drop take the same path without the exchange.

**Native tests** (`worker_launch_tests.rs`, real Node worker and Edge 155). These are ignored in a plain `cargo test` with a documented reason. The root `npm test` runs them via `npm run browser-launcher:native`, so the **Node / windows-latest** job runs them. The runner fails unless exactly 12 tests run and pass, so a filter that matches nothing cannot pass vacuously. All 12 pass locally in two consecutive runs. On the first CI run, 10 of the then 11 passed on the hosted runner. The lifecycle test failed there because the runner carries Edge policies, which led to the split below.

| Test | What it proves |
|---|---|
| Lifecycle | With no managed policy (a stand-in source), the job verifies and `ping` answers. Shutdown is graceful, the job empties, every recorded member dies, and the launcher removes the profile. |
| Real registry precondition | On the actual machine, the launch happens exactly when the policy check passes, and is typed `ManagedPolicy` with nothing started exactly when it refuses. The log records which branch ran: GitHub's Windows runners carry Edge policies, this development host does not. |
| Drop | Dropping supervision kills every member and removes the profile. |
| Crash and restart | The killed worker is detected (`ping` fails) and the job empties. The same profile relaunches, answers `ping`, and shuts down clean. |
| Host death | A separate host process is hard-killed with no destructor, and kill-on-close takes the worker, the browser and every helper down. |
| Foreign or second browser | `PING.EXE`, then a second Edge browser process (started suspended, so it cannot exit or start helpers outside the job first), are assigned into the job. `verify` refuses each one, and the job still empties. |
| Handle inheritance | An inheritable pipe that the host created reaches EOF as soon as the host closes its copy, so the worker and engine did not inherit it. Negative control: a process started without a handle list does inherit it. |
| Silent worker | It is typed `LaunchFailed` after the handshake timeout, and its process is dead. |
| Refused runtime | It is typed `UnsupportedRuntime`. |
| Managed policy | `Populated` and `Unreadable` are typed `ManagedPolicy`, and nothing starts. |
| Registry reader | A real test key under `HKCU\Software\QdralTest` is read as absent, then empty, then populated (by a value, and by a subkey alone), then absent. |

**Findings:**
- **Discovery defect (fixed here; SG-000074 path).** The engine version was probed by running `msedge.exe --version`. On Windows, Chromium prints no version for that, and instead hands the command to an already running browser session ("Opening in existing browser session."), so it can touch the user's personal profile. Discovery therefore always failed on Windows, which made SG-000074's `live_host_or_typed_unavailable_with_lifecycle` pass as "unavailable" without exercising the live host. The version is now read from the executable's version resource, without executing anything. On this host, discovery verifies Edge 155.0.4283.45 (publisher verified) and the SG-000074 live host launches.
- **`conhost.exe`.** `CREATE_NO_WINDOW` still attaches a hidden console host to Node, which the job check correctly refused as a foreign process. `DETACHED_PROCESS` removes it.
- **Kernel-held profile files.** After every job member is dead, Restart Manager has reported `System` (pid 4) briefly holding memory-mapped profile files. Under load this exceeded 10 s. No Edge or Node process outside the job held a profile in these runs. Removal now retries for up to 30 s and reports `profile_removed: false` if the files are still held. The tests fail unless the launcher itself reported removal; on failure they diagnose the held file and clean up.
- **Profile lock after a crash.** Chromium's exclusive `lockfile` in the profile stays held for 0.2–4.4 s after every job member has exited. A relaunch on the same profile inside that window makes the new engine exit (typed `LaunchFailed`). The crash test now waits, with a bound, for the lock to be released before it relaunches. A fresh profile per lifetime, as production uses, avoids this entirely.
- **No out-of-job residue.** At the end of every supervised native test, a machine-wide native process snapshot (Toolhelp, plus each process's OS command line) must find no inspectable process that still names the profile on its command line. A leftover engine of the same user is always inspectable; processes of other users or protected processes are skipped. A control confirms that the same query does see the engine while it runs.
- **Known limitation: host death leaves the profile directory.** Kill-on-close takes the worker and the engine down, but nothing removes the ephemeral profile once the host itself has died. The sweep belongs with the product caller, which decides where profiles live. It is tracked in #298.
- **Crash-session restore.** A profile reused after a crash restores the previous session (9 renderers instead of 2). Production must use a fresh ephemeral profile per lifetime, because reusing one carries state over. The launcher does not enforce this (`ephemeral_profile: false` is accepted, and the crash test reuses a profile on purpose), so the future product caller must.
- **Unit variants in serde.** With `deny_unknown_fields`, serde still accepts unknown fields on unit variants of an internally tagged enum. The worker replies use empty struct variants instead. The SG-000074 host protocol (`proto.rs`: `Shutdown`, `Bye`) has the same looseness; it is tracked in #296.
- **No crash-report uploads from a crash.** An abrupt worker kill leaves only Crashpad metadata (no report or dump), so a crash produces no upload candidate.

**Slice 2b-ii-b, part 1: HTTPS-fixture probes** (`apps/qdral-browser-worker/src/native.test.ts`).

The fixture origin is served over TLS on 127.0.0.1:443, advertising `alt-svc: h3`. Its certificate is a throwaway one, generated at test time with Git for Windows' OpenSSL, and only the direct test launches trust it (`--ignore-certificate-errors-spki-list`). The production argv carries no certificate flag. UDP listeners on 127.0.0.1:443 (the admitted name's pin) and 127.0.0.1:4433 (an IP literal) count datagrams.

| Probe | Result (Edge 155, local) |
|---|---|
| Alt-Svc HTTP/3 upgrade under the confinement argv | **Holds.** 0 datagrams over two navigations; `--disable-quic` stops it. |
| WebTransport to an IP literal | **Holds.** 0 datagrams; the no-route proxy refuses it. Control: without the confinement flags, the same attempt sends UDP. |
| WebTransport to an admitted name | **Accepted residual risk (#300; founder decision 2026-10-11).** QUIC Initial packets reach the admitted name's pinned address directly, uninspected by the SG-000075 policy route. No engine switch disables it (`--disable-features=WebTransport`, `WebTransportOverHTTP3` and `--disable-blink-features=WebTransport` were all tested), and Playwright routing does not see it. The probe asserts the boundary that must hold: unadmitted names, unmapped public names and IP literals all fail with `WebTransportError`, and no datagram reaches the pin or the literal for those attempts. It logs the admitted-name datagram count as evidence. |
| Service-worker denial | **Closed (#301).** The worker launches with `serviceWorkers: "allow"`, which keeps service-worker requests routable, and the route stack aborts every request carrying `Service-Worker: script`, so same-origin registration (direct or through the prototype) fails. A cross-site iframe could still register, because its script fetch never reaches routing. Since slice 3a, the policy route refuses cross-origin frames (`mediate_frame`), so that iframe never loads. A native probe asserts the frame was asked about and refused, `other.test` was never contacted, and no web service worker runs. A second probe pins the route order by running the stack with an allow-everything policy. |

In these probes the no-route proxy refuses the IP literal and nothing leaves the admitted pin. The earlier HTTP-fixture probe covers unadmitted names. Both gaps stayed inside admitted destinations but evaded the policy route, which already denies the equivalent WebSocket traffic. #300 is now a founder-accepted residual risk. #301 is closed by slice 3a, so `KNOWN_GAPS` is empty and the gate is open. This is enforced in code. `gaps.ts` lists the known gaps (`KNOWN_GAPS`), which the `todo` probes reference. While that list is non-empty, a conditional type makes `tsc` (build and typecheck) fail if `HostFrame` gains any frame name outside the launch vocabulary, however that frame is written. Named types, quoting, spacing and a widened `string` frame were each verified to fail. The gate covers frame names only: a field added to an existing frame is not caught by it, though `parseHostFrame`'s exact-key checks would have to change visibly for such a field to be accepted. `gaps.test.ts` mirrors the check at runtime (`parseHostFrame` rejects other frame names) and requires every gap to keep its `todo` probe.

**Still to be done in slice 2b-ii-b:**
- revisiting #300 if Chromium gains an engine-level WebTransport kill switch;
- a real public name pinned by the host reaching only its pin;
- the job process limit under a heavy page;
- the release packaging of the worker (NOTICE, SBOM, `.bin` strip).

**Slice 3a: live navigation decided by the host.** Code: `worker_navigation.rs`; the worker's `routes.ts`, `redirect-guard.ts` and `routing.ts`; worker protocol generation 2.

The host stays the authority. **An engine lifetime admits exactly one destination** (`validate_launch`). `navigate` first runs `NavigationPolicy::begin`, which requires an HTTPS URL on that admitted name.

Two layers then govern traffic:
- **Network layer (complete).** The pinned resolver and the no-route proxy make every other host unreachable, whatever issues the request: pages, workers, or the browser process itself. The one exception is the accepted residual #300: WebTransport UDP to the admitted name's own pin.
- **Per-request layer.** Engine requests on the admitted origin are described to the host as `route_request`s and continue only on an explicit allow, by the means listed below.

A fresh-context judge measured that the browser process issues some requests outside every route and guard: `fetchLater`, Reporting API and CSP reports, FedCM, audio-worklet modules, and NoStatePrefetch (`<link rel=prerender>`). With a second admitted name, those reached it undecided. With one admitted name they can reach only the page's own origin, which SG-000075 permits for subresources anyway. No capability is lost: the policy is exact-origin, and another site means another lifetime.

The per-request means are:
- **Routed requests.** Documents, frames, popups, fetch and XHR, and other subresources, including dedicated-worker scripts and their imports.
- **Redirect hops.** Playwright routes never see them. A per-page CDP `Fetch` guard pauses each hop and forwards it as a `redirect` query.
- **Service workers and SharedWorkers.** Service-worker scripts are refused by the route stack. SharedWorker requests are never routed (measured), so the API is removed from the engine; see the argv amendment below.
- **Speculation rules.** Speculation-rules prefetch and prerender run in the browser process, outside every route and guard. This was measured: an undecided request to another admitted name, and a prerender whose activation moved the page with no request at all. Engine switches did not stop them (`Prerender2`, `SpeculationRulesPrefetchProxy`, `--disable-blink-features=SpeculationRules`, and CDP `Page.setPrerenderingAllowed` were tested). The profile preference "Preload pages: never" (`net.network_prediction_options` = 2) does stop them, so the host writes it into the profile before every launch (`seed_profile_preferences`). Existing preferences are kept, and malformed ones fail closed.

The decisions are the existing SG-000075 policy:
- the first document must be the requested URL;
- a page-initiated top-level navigation must stay on the page's origin;
- redirects of a navigation are checked by `RedirectChain` (same origin, at most 5 hops, no loops);
- a later subresource redirect must stay on the page's origin;
- frames, popups, fetch and subresources are checked by `mediate_*` (exact origin).

All of these are checked against the **host's own** page origin: the begun origin during a navigation, otherwise the committed one. A request whose reported origin disagrees with the host's is refused, so a page that moved itself without a decision gains nothing. The committed document must be on the navigation's origin, and a failed navigation is aborted so its state authorizes nothing.

**Argv amendment (authority delta, narrowing):** the confinement argv gains `--disable-blink-features=SharedWorker`. It is applied by the host builder (`confinement.rs`), required by the worker's exact-argv contract, and covered by the OS command-line verification.

**Implementation findings:**
- A refused navigation's error page commits late, sometimes more than once, and interrupts the next navigation in the same page. Each host navigation therefore runs in a fresh, guarded page.
- Edge runs a built-in component extension service worker even with `--disable-extensions`. The probes count only web service workers.

**Known limits:**
- Browser-process requests (see above) on the admitted origin are not individually decided. They are bounded by the one-destination network layer, but the host cannot decide, log or rate-limit them, and a deferred `fetchLater` POST or a batched report can reach the admitted site after the host has moved on. A later slice that adds write approval or a non-GET audit must account for this. Options are to close these APIs with engine switches, or to close the lifetime before trusting such controls.
- (Fail closed) The host decides only while it is in a call. A page's own requests made while the host is idle wait for up to 10 s and are then refused.
- A popup's first navigation is not completed, so popups do not work, but nothing leaks.
- WebTransport to admitted names remains the founder-accepted residual risk #300.

**Native end-to-end evidence:** the real host, worker and Edge run against a two-name HTTPS fixture (`scripts/https-fixture.mjs`, test only). The ground truth is the fixture's request log.
- An admitted page loads, and a same-origin redirect commits.
- A cross-origin redirect, a redirect loop and a 7-hop chain are refused, typed as denied.
- Non-admitted, `http:` and IP-literal URLs are refused before anything is sent.
- The mixed page contains cross-origin iframe, image, fetch, `<link rel=prefetch>`, speculation-rules prefetch and prerender, beacon, `EventSource`, a blob worker importing a cross-origin script, a CSS `@import`, `<a ping>`, a SharedWorker, and popups including one that redirects cross-origin. It never contacts `other.test`, while the same-origin resources load.
- In the prerender-then-activate escape, the page's navigation to `/redirect-evil` is observed and its cross-origin hop is refused.

The mixed page also issues the browser-process requests the judge measured (`fetchLater`, a CSP report-to violation with `Reporting-Endpoints`, FedCM, an audio worklet, `<link rel=prerender>` of a cross-origin redirect); none reaches `other.test`. The judge's runs with two admitted names are the control: with two names admitted, these requests did reach `other.test`.

The test passed in repeated local runs. The fresh-context judge's rounds 1 and 2 on this slice were BLOCK. The speculation-rules, SharedWorker and host-origin findings, and the one-destination rule, are their fixes.

**Test-only certificate trust:** `WorkerLaunch.test_trusted_spki` exists only under `cfg(test)` and appends `--ignore-certificate-errors-spki-list`. The worker accepts that flag only when every admitted pin is loopback; production pins are never loopback. A governance test pins the flag to `cfg(test)` code.

**Slices 3b and 3c: observation and approval-gated activation** (worker protocol generations 3 and 4).

- **Observation (SG-000076).**
  - The worker sends a bounded tree from `ariaSnapshotJSON({mode:"ai"})`, covering shadow DOM and iframes. Each node carries a ref, a role, an accessible name and, for links only, a bounded URL. Field values are never forwarded; this was measured, because the snapshot shows a password value in clear text under role `textbox`.
  - An accessible name can still carry a field's text: a button labelled by a password field through `aria-labelledby` was measured. Names are therefore keyed with a per-lifetime secret from the system RNG before they enter SG-000076, so a caller cannot check a digest against a guessed value. Within one lifetime, equal names still produce equal digests, so a caller can tell that two names match, and could confirm a value it planted itself through an approved fill. Raw names stay in a host-only index, and no product API exposes them.
  - The snapshot must be of exactly the committed URL. Node identities are bound to the document generation.
  - Worker truncation is propagated, and only nodes the caller actually received can be acted on.
- **Activation (SG-000077).**
  - Selection is host-side and typed: `select_latest_dated_link`. It scans bytes only, so non-ASCII names cannot crash the host, and it refuses ties.
  - The host does not model element state. A target's `state` must instead be the link's host-resolved destination (`link_destination`; absolute `https://` or root-relative links only), so the one-shot approval covers where the click goes.
  - The destination is checked against the page's origin before the approval is consumed.
  - During the click, the host accepts exactly one page-initiated document request, and it must be that destination. A script redirect and an `href` swapped after observation were both measured and are refused at the request. A click that requests no document (`pushState`, a script action) is not a verified activation.
  - The destination's server may still redirect same-origin. That is decided hop by hop by `RedirectChain`, so the committed URL can differ from the approved destination; the server already controls what it serves there.
  - The host does not see the HTTP method, so a click handler that submits a form to the destination is accepted as well. That gives the page nothing it cannot already do with a same-origin `fetch`.
  - Matching is by exact string. A link that Chromium normalizes before requesting it (spaces, `..`, a fragment) is therefore refused, failing closed, and its approval is spent.
  - The worker clicks by `aria-ref`. A replaced node or an overlay makes the click time out rather than hit another element (measured).
- **Known limits.**
  - Off-screen or `opacity:0` links appear in the snapshot and can win the date selection. Host policy still applies to whatever they open.
  - A click that has a side effect without opening a document (for example `fetch`) is refused as unverified only after it has happened, and its approval is consumed.
  - A `target=_blank` click that opens a popup is not an activation; the popup itself is mediated by host policy.

**Evidence (local, Edge 155), native end-to-end tests on the HTTPS fixture:**
- Observation never shows a raw name, the user name or the password to the caller, and the secret-named button's digest is keyed. The host index holds the dated links and their URLs, and a page that moved itself is refused as stale.
- Activation follows the latest dated link with an approval. A forged digest, a replayed approval, the stale node, the cross-origin latest link (without consuming its approval), the script redirect and the swapped `href` are all refused, and none of their destinations is requested.

A fresh-context judge's first round on these slices was BLOCK. The byte-safe date scan, keyed names, destination-bound approvals, host-side state and propagated truncation are its fixes.
