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

**The production network confinement.** `edge.ts` starts a fresh Edge for the suite, headless, with a throwaway profile and DevTools on loopback at an OS-chosen port. Its network flags are the production ones:
- **Resolver:** `fixture.test` is the only admitted destination. The host's grammar is `--host-resolver-rules=MAP fixture.test 127.0.0.1:443, MAP * ~NOTFOUND`, and `--proxy-bypass-list=fixture.test;<-loopback>`.
- **The other five flags come from the built worker itself.** `run.mjs` imports `dist/protocol.js` and passes on its values, so the suite cannot drift from production:
  - the no-route proxy, `NO_ROUTE_PROXY`;
  - the WebRTC UDP policy, `WEBRTC_POLICY`;
  - `DISABLE_QUIC`;
  - `DISABLE_SHARED_WORKER`;
  - the #324 disabled features, `DISABLE_FEATURES`.
- **Profile seeding:** network prediction is off, as the host seeds it.
- **Certificate:** only the fixture's own certificate is trusted.

**Enforcement evidence, not flags or request logs.** `run.mjs` checks the outcome from outside the browser:
- **LAN listeners.** It opens a UDP and a TCP listener on this host's own LAN address. A positive control first proves they are reachable there: the runner sends to them itself.
- **Negative control.** An unconfined Edge, pointed at the same IP literal, must reach the TCP listener. This proves the bypass probe is a real one.
- **OS connection tables.** During the suite, `scan.ps1` samples the TCP and UDP tables (`Get-NetTCPConnection` / `Get-NetUDPEndpoint`, read-only) for the whole process tree of the suite's Edge, helpers included, about every 2 to 3 s. The run fails if:
  - the scan never saw the suite's Edge;
  - any process held a TCP connection to a non-loopback address;
  - any process held a non-loopback TCP listener;
  - any process held a UDP 5353 endpoint.

  Other non-loopback UDP sockets (WebRTC's ephemeral ICE sockets) are counted and reported.
- **Fixture log.** It reads the fixture's own request log. Any host other than `fixture.test` fails the run.

**Nothing left behind.** After the suite, `run.mjs` asks each launched Edge to close over its own DevTools endpoint (`Browser.close`), ends any recorded process still left with its whole tree, and removes each profile. The run fails if a recorded process still runs or a profile remains. A profile can be deleted only once no Edge process holds it.

## The journey, and what it proves

`tests/journey.e2e.ts` has two tests.

1. **The latest dated report is the exact document.**
   - It opens `/reports`, sees the three dated links, and picks the latest by its declared date.
   - It opens that report and checks that the response is `200 application/pdf`.
   - The built-in PDF viewer replaces the document's body with its own page, so the test reads the same URL again, same-origin, as a byte array, from its own browser. It compares the bytes exactly with the fixture's report, and their SHA-256.
2. **Nothing outside the fixture origin is reachable:** `https://other.test/` fails to load.
3. **An IP-literal bypass of the resolver is denied.** From an admitted page (`/rtc`), it runs WebRTC ICE (STUN over UDP, TURN over TCP), https `fetch`, `wss` and `sendBeacon` toward this host's LAN address, every 5 s for 12 s. Then it navigates directly to `https://<LAN IP>:<port>/` and `http://<LAN IP>:<port>/`, and both must fail. The denial is recorded where it matters: the listeners received nothing, and the OS tables show no endpoint beyond loopback.

`run.mjs` then requires, beyond the listener and OS-table checks above, from the fixture's log:
- the latest report was requested;
- no older report was requested;
- no other host was contacted.

**What this is evidence of.** TesterArmy drives its own browser. It cannot drive Aladdin's isolated browser, which has no DevTools port by design (SG-000074), and giving it one would break confinement. So this is an independent, black-box check of the first journey on the controlled fixture: the site, the selection by declared date and the exact document. It runs in a browser under the production network confinement, and enforcement is checked at the OS level. **It does not test the production worker.** The confinement, policies, approvals and create-only commit of Aladdin's own browser are proven by the native suite (`npm run browser-launcher:native`), whose OS-level checks are the same kind (#324).

## Evidence (Windows, Edge 155, 2026-10-13)

`npm ci --ignore-scripts && npm run e2e` in `apps/qdral-browser-worker/testerarmy/`, with the worker built first: `A8: PASSED`.
- **Tests:** 3 of 3 passed.
- **The report:** `https://fixture.test/reports/2026-09-30.pdf` with SHA-256 `5523ba2d22b98403ba0f7d9166a9a8cc48ef8330656637f28d8cbf125381994a`, equal to the fixture's report.
- **Negative control:** an unconfined Edge reached the LAN listener at the IP literal.
- **Bypass denied:** during the suite the LAN listeners received **0 datagrams and 0 connections**.
- **OS tables:** across the suite, up to 16 Edge processes, nothing flagged; 4 to 6 other UDP sockets (ICE) across runs.
- **Fixture log:** only `fixture.test` (`/reports`, `/favicon.ico`, `/reports/2026-09-30.pdf`, `/rtc?…`); no older report and no other host.
- **Cleanup:** one Edge, gone afterwards, with no profile left.

**Each check catches what it claims.** Each mutation below changed the suite's Edge flags temporarily, and was then reverted:

| Mutation | Result |
| --- | --- |
| `DISABLE_FEATURES` removed | **FAILED.** The OS tables flagged Microsoft 365 (`2603:1046:…:443`), Cast devices (`192.168.100.11:8009`, `192.168.100.16:8009`) and mDNS (`0.0.0.0:5353`, `[::]:5353`). |
| Resolver catch-all removed, no-route proxy kept | PASSED. The proxy alone denies the IP literal. |
| No-route proxy removed, resolver catch-all kept | PASSED. On Edge 155 the catch-all rule `MAP * ~NOTFOUND` also refuses IP literals, so each layer is sufficient on its own. |
| Both removed | **FAILED.** 14 connections reached the LAN listener, and the OS tables flagged connections to internet addresses on 443. |

## Limits

- **Not a firewall.** The denial is the browser's own network layers, the same ones production relies on. It is checked from outside at the OS level, but not enforced by the OS. An OS-enforced boundary, such as a Windows Filtering Platform filter or an AppContainer without network capabilities, needs system or security configuration (administrator rights, a loopback exemption for the fixture). This work does not make that configuration, so OS-enforced containment of the suite's browser remains open.
- **Sampling.** The OS tables are sampled every 2 to 3 s, so a short-lived connection can be missed between samples. The listeners are the primary evidence for the bypass. Connections in TIME_WAIT (owned by pid 0) and traffic delegated to Windows services are not attributed to the browser. Packets to arbitrary internet addresses would need packet capture.
- **Host-dependent parts.** The bypass check needs a non-loopback IPv4 interface, and the run fails without one. The `DISABLE_FEATURES` mutation finds a leak only on a host with something to leak to: a Microsoft account, or Cast devices.

- **Engines:** Chromium-family only, through the installed Edge. TesterArmy's Firefox and WebKit engines would need Playwright's browser downloads, which A8 excludes.
- **Platform:** Windows only. Edge is located at its standard install paths.
- **Model steps:** the suite has none. Adding `agent.act` or `agent.assert` steps would need a model, which A8 forbids unless one runs locally, and none is on the evidence host.
