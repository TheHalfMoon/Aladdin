# SG-000108 engine attachment and resolved-address enforcement decision

Status: PROPOSED for security review. This record resolves the first two
`open_questions` of `.specgrain/specs/SG-000108.json`. It changes no code,
grants no authority and admits no dependency: the frozen SG-000074 command
line in `crates/qdral-browser-host/src/argv.rs` stays exactly as it is until
the implementation PR changes it with the tests listed below.

## Evidence

The evidence comes from throwaway spikes outside the repository. They ran on
Windows 11 (10.0.26300) against the installed Microsoft Edge 155.0.4283.45
with `playwright-core` 1.63.0. The package was installed with
`--ignore-scripts`, and its lockfile integrity equals the admitted
`sha512-rYCsBF/M…nyxo5mCg==`. No browser binary was downloaded. The
implementation PR turns each spike into a native Windows test.

| Probe | Result |
|---|---|
| `launchPersistentContext` with `ignoreDefaultArgs: true`, the seven frozen flags, `--user-data-dir`, `--remote-debugging-pipe`, `about:blank` | Launches and is driven, and an ARIA snapshot of a fixture page is returned. The OS-reported engine command line (`Win32_Process.CommandLine`) is exactly those arguments: no `--enable-automation`, no debugging port or address, no Playwright default flags. |
| `--host-resolver-rules` mapping only the admitted names to host-chosen addresses, then `MAP * ~NOTFOUND` | Admitted names reach only the pinned address. A real public name (`example.com`) and every unmapped name fail with `ERR_NAME_NOT_RESOLVED` even with the URL router removed. The fixture server saw only admitted requests. |
| Add `--proxy-server=http://0.0.0.0:9` and `--proxy-bypass-list=<admitted names>;<-loopback>` | With the URL router removed, IP literals (`127.0.0.1`, `10.0.0.1`, `169.254.169.254`) and unmapped names fail with `ERR_PROXY_CONNECTION_FAILED` in about 4 s. Admitted names still go direct to their pinned address. |
| WebRTC STUN to an IP literal (loopback and LAN) from page script | **Without a policy flag, UDP reaches an arbitrary address**: 4 packets, bypassing both layers above. `--force-webrtc-ip-handling-policy=…` has no effect. `--webrtc-ip-handling-policy=disable_non_proxied_udp` yields zero candidates and zero packets. TURN over TCP yields zero connections, because it must go through the dead proxy. |

## Decision 1: engine launch and attachment (option a)

**Launcher.** The private worker launches the verified engine through
`playwright-core` `launchPersistentContext` over `--remote-debugging-pipe`.

**What the host still owns:**
- engine discovery and signature verification;
- profile adoption and the fingerprint check;
- the scrubbed environment;
- construction of the complete argv.

**What the worker receives.** The worker gets the engine path, the profile
directory, the argv and the environment from the host, and passes them
unchanged with `ignoreDefaultArgs: true`. The worker has no code path that
adds a flag.

**Post-launch verification.** After launch the host reads the engine's
OS-reported command line, parent process and Job Object membership. Any
deviation from the host-built argv stops the engine and fails closed.

**Supervision.** The worker runs in the host's kill-on-close Job Object
without breakaway, so the engine and its child processes inherit the job.

**Option (b) rejected.** Option (b) would have the host launch the engine and
hand the pipe to the worker. It was rejected because Playwright has no public
API for an inherited pipe: it would need a private adapter that breaks on
upgrades, with no security gain over verifying the launched command line.

**SG-000074 argv amendment.** This is the explicit authority delta of the
implementation PR. `ALLOWED_FLAGS` gains `--remote-debugging-pipe` plus the
three value-carrying flags of Decision 2. Rules for the argv:
- The value of each value-carrying flag must equal the string built by the
  host for that launch; a flag present only by name is refused.
- `FORBIDDEN_FLAGS` is unchanged: `--remote-debugging-port`,
  `--remote-debugging-address` and `--enable-automation` stay forbidden.
- `--remote-debugging-pipe` is added to the closed set, never to a
  caller-reachable path.

## Decision 2: actual-traffic enforcement of SG-000075

Three independent layers are used. Each layer fails closed by itself.

1. **Host-pinned resolution.** The engine never resolves names.
   - For each admitted destination, the host resolves the name itself and
     chooses the pin with the existing `select_public_address`. Public
     addresses only: loopback, private, link-local, metadata and mapped
     forms are denied by `is_public_address`.
   - The host then emits `--host-resolver-rules=MAP <name> <pin>, …, MAP * ~NOTFOUND`.
   - DNS rebinding is impossible within an engine lifetime because the
     mapping is fixed at launch.
   - A redirect, frame or subresource to a name outside the map fails at
     the resolver.
   - Widening the set means a host-mediated relaunch after a fresh
     `check_rebinding_consistent` check, never an in-place change.
2. **No route for IP literals or unmapped names.** The host sets
   `--proxy-server=http://0.0.0.0:9` and
   `--proxy-bypass-list=<admitted names>;<-loopback>`.
   - Only admitted names connect directly, and only to their pinned address.
   - Everything else, including IP literals, is sent to an address that
     cannot accept a connection on Windows.
   - `<-loopback>` removes the engine's implicit loopback bypass.
   - `0.0.0.0` is chosen over a loopback port because a local process could
     listen on a loopback port and act as a proxy. Connecting to `0.0.0.0`
     on Linux reaches the local host, so this choice is valid only for the
     Windows release.
3. **Policy gate.** The worker's context-level route consults the host's
   SG-000075 decision for every request it sees: navigation, redirect,
   frame, popup, subresource and download. Layer 3 carries policy (scheme,
   origin, exact host, download rules); layers 1 and 2 carry addresses.
   - `serviceWorkers: "block"` is set.
   - WebSocket is denied through the context WebSocket route until a later
     grain admits mediated WebSocket.

**Required with the layers:**
- `--webrtc-ip-handling-policy=disable_non_proxied_udp`, because the WebRTC
  probe proves that layers 1 and 2 do not cover UDP.
- Page-level APIs that need permissions stay denied by
  `permission_allowed`.

### Hostname and value hygiene

- **Hostnames.** The host builds the rule and bypass strings from validated
  ASCII LDH hostnames only:
  - lowercase;
  - labels of 1 to 63 characters, 253 characters in total;
  - no wildcard, comma, semicolon, whitespace, `=` or `~`.
- **Addresses.** Values come from typed `IpAddr`, formatted by Rust. IPv6
  pins are bracketed.
- **Construction.** No page, caller or worker string reaches either flag.

## Residual risks (for review)

- **Admitted host is hostile.** An admitted name pinned to its public address
  still serves whatever that host returns. Content is never policy (SG-000076
  redaction and SG-000077 approvals still apply).
- **Engine bugs.** A future Edge may change how it honours the resolver,
  proxy or WebRTC flags. A10's supported version window must re-run the
  probes in release qualification on every supported engine version, and
  refuse versions where they fail.
- **Relaunch cost.** Relaunching to widen the destination set costs latency
  in exchange for no in-place mutation. This is accepted for the first
  journey (A6), whose destinations are known up front.
- **No-router case.** In the spikes, the router itself (layer 3) was removed
  only to test layers 1 and 2. In production all three are active.

## Tests the implementation PR must add (native Windows, fixture origin only)

- **Argv.** The exact argv, and the OS command line equal to it, before and
  after the navigation. Refusal of a pipe flag without the host-built values.
  Refusal of every forbidden flag, as today.
- **Name pinning.** Unmapped name, real public name, IP literal (v4 and v6),
  private, loopback and metadata are denied with the router disabled by a
  test-only hook.
- **Rebinding.** Rebinding attempt via a fixture resolver that changes
  answers; the engine keeps the pin.
- **WebRTC.** STUN and TURN probes send zero packets and make zero
  connections.
- **Service workers and WebSocket.** A service worker registration and a
  WebSocket connection are refused.
- **Supervision.** The engine is a descendant of the worker inside the host
  job; killing the host leaves no engine process.
