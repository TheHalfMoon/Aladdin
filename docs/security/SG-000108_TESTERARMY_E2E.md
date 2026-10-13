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

**No browser download.** The web engine attaches over CDP (`connect.cdpEndpoint`) to the installed Edge, which is 155 on the evidence host. `edge.ts` starts a fresh Edge for the suite with:
- headless mode, a throwaway profile and DevTools on loopback with an OS-chosen port;
- `fixture.test` mapped to loopback and every other name to nothing (`--host-resolver-rules=MAP fixture.test 127.0.0.1, MAP * ~NOTFOUND`);
- trust for only the fixture's own certificate (`--ignore-certificate-errors-spki-list`).

Playwright's bundled browsers are never installed.

**Fixture origin only.** `run.mjs` starts the existing HTTPS fixture (`scripts/https-fixture.mjs`) and reads its request log independently of TesterArmy. The run fails if any host other than `fixture.test` was contacted.

**Nothing left behind.** After the suite, `run.mjs` asks each launched Edge to close over its own DevTools endpoint (`Browser.close`), ends any recorded process still left with its whole tree, and removes each profile. The run fails if a recorded process still runs or a profile remains. A profile can be deleted only once no Edge process holds it.

## The journey, and what it proves

`tests/journey.e2e.ts` has two tests.

1. **The latest dated report is the exact document.**
   - It opens `/reports`, sees the three dated links, and picks the latest by its declared date.
   - It opens that report and checks that the response is `200 application/pdf`.
   - The built-in PDF viewer replaces the document's body with its own page, so the test reads the same URL again, same-origin, as a byte array, from its own browser. It compares the bytes exactly with the fixture's report, and their SHA-256.
2. **Nothing outside the fixture origin is reachable:** `https://other.test/` fails to load.

`run.mjs` then requires, from the fixture's log:
- the latest report was requested;
- no older report was requested;
- no other host was contacted.

**What this is evidence of.** TesterArmy drives its own browser. It cannot drive Aladdin's isolated browser, which has no DevTools port by design (SG-000074), and giving it one would break confinement. So this is an independent, black-box check of the first journey on the controlled fixture: the site, the selection by declared date and the exact document. The confinement, policies, approvals and create-only commit of Aladdin's own browser are proven by the native suite (`npm run browser-launcher:native`), not by this suite.

## Evidence (Windows, Edge 155, 2026-10-13)

`npm ci --ignore-scripts && npm run e2e` in `apps/qdral-browser-worker/testerarmy/`, three runs in a row, all `A8: PASSED`:
- **Tests:** 2 of 2 passed.
- **The report:** `https://fixture.test/reports/2026-09-30.pdf` with SHA-256 `5523ba2d22b98403ba0f7d9166a9a8cc48ef8330656637f28d8cbf125381994a`, equal to the fixture's report.
- **Fixture log:** only `fixture.test` (`/reports`, `/favicon.ico`, `/reports/2026-09-30.pdf`); no older report and no other host.
- **Cleanup:** one Edge per run, gone afterwards, with no profile left. A separate process listing found no Edge of the suite and no `qdral-a8-edge-*` folder.

## Limits

- **Engines:** Chromium-family only, through the installed Edge. TesterArmy's Firefox and WebKit engines would need Playwright's browser downloads, which A8 excludes.
- **Platform:** Windows only. Edge is located at its standard install paths.
- **Model steps:** the suite has none. Adding `agent.act` or `agent.assert` steps would need a model, which A8 forbids unless one runs locally, and none is on the evidence host.
