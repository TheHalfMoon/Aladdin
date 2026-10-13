# SG-000108 A8: TesterArmy E2E evidence

Acceptance A8 asks for TesterArmy E2E evidence, run against the fixture origin only and needing no paid or external service. The spec's open question asks for the exact version, the engines and the network behaviour. This record answers both.

## What runs

`apps/qdral-browser-worker/testerarmy/` is a self-contained, test-only suite. It is never packaged, not part of any workspace, and its pinned lockfile is not installed by CI.

**Versions, installed with `npm ci --ignore-scripts`; every integrity value is in the lockfile:**
- `e2e` 0.20.0 (tester-army/e2e, Apache-2.0);
- `@e2e-dev/web` 0.14.0 (Apache-2.0);
- `playwright-core` 1.63.0, the version SG-000108 already admitted.

**No model.** The tests use only locators, `expect` assertions and test code. No agent and no model is configured, so no step can call one. A paid service, an API key and a local model server are all unnecessary.

**No telemetry.** The CLI sends anonymous usage data by default; `run.mjs` sets `E2E_TELEMETRY_DISABLED=1`.

**No browser download.** The web engine attaches over CDP (`connect.cdpEndpoint`) to the installed Edge, which is 155 on the evidence host. Playwright's bundled browsers are never installed.

## The production launch, with one difference

`run.mjs` builds the exact argv and engine environment that the host hands the worker for the `fixture.test` destination:
- the SG-000074 frozen flags (`FROZEN_FLAGS`);
- the profile;
- the pipe;
- the host's resolver and bypass grammar from `confinement.rs`: `MAP fixture.test 127.0.0.1:443, MAP * ~NOTFOUND` and `fixture.test;<-loopback>`;
- the worker's no-route proxy, WebRTC policy, QUIC, SharedWorker and #324 disabled-feature flags;
- the fixture's certificate;
- `about:blank`.

The engine environment is only the allowlisted keys (`ENGINE_ENV_ALLOWLIST`). Every fixed value is imported from the built worker's `dist/protocol.js`. **The worker's own contract check, `launchRefusal`, must accept that launch, or the run fails.** So the frozen flags, the order, the flag count and every fixed value are those production accepts.

`edge.ts` launches exactly that argv with that environment, with one change: TesterArmy attaches over CDP, so the pipe flag is replaced by DevTools on loopback at an OS-chosen port. The profile is seeded as the host seeds it, with network prediction off.

**What still differs from the production worker, and is not tested here:**
- the DevTools port in place of the pipe;
- no kill-on-close job object; the suite tracks and ends its Edge itself;
- no host process at all, so no policy routing, approvals or create-only commit;
- the resolver and bypass strings are written in `run.mjs` in the host's grammar, not built by the Rust host. The worker's contract check accepts them, and the native suite's argv vectors pin what the host builds.

## Enforcement evidence, not flags or request logs

`run.mjs` decides the outcome from outside the browser:
- **LAN listeners.** It opens a UDP and a TCP listener on this host's own LAN IPv4 address. A positive control first proves they are reachable there: the runner sends to them itself.
- **Negative control.** An Edge without the confinement, pointed at the same IP literal, must reach the TCP listener. This proves the probe is a real one.
  - That Edge reaches nothing else. The listener is its proxy (`--proxy-server=http://<LAN IP>:<port>`, `--proxy-bypass-list=<-loopback>`), and the services that ignore the proxy are off: the WebRTC policy, QUIC, and the #324 disabled features, plus the frozen `--disable-background-networking`.
  - In a separate probe, run with the proxy on a dead port and the whole process tree scanned for 25 s, its only non-loopback endpoint was the proxy address itself (`SynSent 192.168.100.5:9`).
  - The run fails if its process or profile is left behind.
- **OS connection tables.** During the suite, `scan.ps1` samples the TCP and UDP tables (`Get-NetTCPConnection` / `Get-NetUDPEndpoint`, read-only) every 0.5 to 3 s. It covers the whole process tree of the suite's Edge, helpers included: the recorded root and every descendant. The run fails if:
  - the scan never saw the suite's Edge;
  - any process held a TCP connection to a non-loopback address;
  - any process held a non-loopback TCP listener;
  - any process held a UDP 5353 endpoint;
  - **no ICE socket was seen,** because then the page's WebRTC probe never ran.

  The tables show bound UDP sockets, not where they send. WebRTC's ephemeral ICE sockets (bound to `0.0.0.0` and `::` in the network service) are counted and reported, not flagged.
- **Fixture log.** It reads the fixture's own request log. The run fails if any host other than `fixture.test` was contacted, if the latest report was not requested, if an older one was, or if the bypass page `/rtc` was never served.

**Nothing left behind.** After the suite, `run.mjs` asks each launched Edge to close over its own DevTools endpoint (`Browser.close`). It then ends any recorded process still left with its whole tree, but only while that pid is still an `msedge.exe`, never another program that reused it. It removes each profile. The run fails if a recorded Edge still runs or a profile remains.

## The journey

`tests/journey.e2e.ts` has three tests.

1. **The latest dated report is the exact document.**
   - It opens `/reports`, sees the three dated links, and picks the latest by its declared date.
   - It opens that report and checks that the response is `200 application/pdf`.
   - The built-in PDF viewer replaces the document's body with its own page, so the test reads the same URL again, same-origin, as a byte array, from its own browser. It compares the bytes exactly with the fixture's report, and their SHA-256.
2. **Nothing outside the fixture origin is reachable:** `https://other.test/` fails to load. `other.test` is a name nothing resolves, so this test alone proves little; the run-level checks above carry the proof.
3. **An IP-literal bypass is attempted.** From an admitted page (`/rtc`), it runs WebRTC ICE (STUN over UDP, TURN over TCP), https `fetch`, `wss` and `sendBeacon` toward this host's LAN address, every 5 s for 12 s. Then it navigates directly to `https://<LAN IP>:<port>/` and `http://<LAN IP>:<port>/`.
   - **This test only makes the attempts.** A failed navigation proves nothing by itself: the listener closes every connection, and its certificate is not trusted.
   - **Whether they were denied is decided by `run.mjs`:** nothing at the listeners, nothing beyond loopback in the OS tables.

**What this is evidence of.** TesterArmy drives its own browser. It cannot drive Aladdin's isolated browser, which has no DevTools port by design (SG-000074), and giving it one would break confinement. So this is an independent, black-box check of the first journey on the controlled fixture: the site, the selection by declared date and the exact document. It runs in a browser launched as production launches it, apart from the attachment, and enforcement is checked at the OS level. **It does not test the production worker.** The confinement, policies, approvals and create-only commit of Aladdin's own browser are proven by the native suite (`npm run browser-launcher:native`), whose OS-level checks are the same kind (#324).

## Evidence (Windows, Edge 155, 2026-10-13)

`npm ci --ignore-scripts && npm run e2e` in `apps/qdral-browser-worker/testerarmy/`, with the worker built first: `A8: PASSED`.
- **Production launch contract:** accepted by `launchRefusal`.
- **Tests:** 3 of 3 passed.
- **The report:** `https://fixture.test/reports/2026-09-30.pdf` with SHA-256 `5523ba2d22b98403ba0f7d9166a9a8cc48ef8330656637f28d8cbf125381994a`, equal to the fixture's report.
- **Negative control:** it reached the LAN listener at the IP literal.
- **Bypass denied:** during the suite, the LAN listeners received **0 datagrams and 0 connections**.
- **OS tables:** 28 samples, up to 16 Edge processes, nothing flagged; 6 ICE UDP sockets (4 to 8 across runs).
- **Fixture log:** only `fixture.test` (`/reports`, `/favicon.ico`, `/reports/2026-09-30.pdf`, `/rtc?…`); no older report and no other host.
- **Cleanup:** the suite's Edge and the control's Edge gone, no profile left.

**Each check catches what it claims.** Each mutation below dropped or changed flags of the suite's Edge in `edge.ts`, after the contract check, so only the listeners and the OS tables could catch it. Each was reverted afterwards.

| Mutation | Result |
| --- | --- |
| `--disable-features` removed | **FAILED.** The OS tables flagged Microsoft 365 (`2603:1046:…:443`), Cast devices (`192.168.100.11:8009`, `192.168.100.16:8009`) and mDNS (`5353`). |
| WebRTC policy removed | **FAILED.** 12 STUN datagrams reached the LAN listener; the OS tables flagged nothing. For UDP to an IP literal, the WebRTC policy is the single layer, and only the listener sees it. |
| Resolver catch-all removed, no-route proxy kept | PASSED. The proxy alone denies the IP literal. |
| No-route proxy removed, resolver catch-all kept | PASSED. On Edge 155 the catch-all rule `MAP * ~NOTFOUND` also refuses IP literals. |
| Catch-all and proxy both removed | **FAILED.** 16 connections reached the LAN listener, and the OS tables flagged connections to internet addresses on 443. |

**So for TCP, each of the two layers is sufficient on its own. For UDP, the WebRTC policy alone stands, and the check can see it.**

## Limits

- **Not a firewall.** The denial is the browser's own network layers, the same ones production relies on. It is checked from outside at the OS level, but not enforced by the OS. An OS-enforced boundary, such as a Windows Filtering Platform filter or an AppContainer without network capabilities, needs system or security configuration (administrator rights, a loopback exemption for the fixture). This work does not make that configuration, so OS-enforced containment of the suite's browser remains open.
- **What the observation covers.**
  - The OS tables are sampled, so a connection opened and closed between two samples can be missed. The listeners are the primary evidence for the bypass.
  - UDP is observed only at the listener's address: where else an ICE socket might send is not visible without packet capture, which needs administrator rights.
  - Connections in TIME_WAIT (owned by pid 0) and traffic delegated to Windows services are not attributed to the browser.
- **Host-dependent parts.** The bypass check needs a non-loopback IPv4 interface, and the run fails without one. The `--disable-features` mutation finds a leak only on a host with something to leak to: a Microsoft account, or Cast devices.
- **Engines:** Chromium-family only, through the installed Edge. TesterArmy's Firefox and WebKit engines would need Playwright's browser downloads, which A8 excludes.
- **Platform:** Windows only. Edge is located at its standard install paths.
- **Model steps:** the suite has none. Adding `agent.act` or `agent.assert` steps would need a model, which A8 forbids unless one runs locally, and none is on the evidence host.
