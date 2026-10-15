// SG-000108 A8: the first journey, driven by TesterArmy with locators and
// assertions only (no model): open the controlled reports page, pick the
// latest report by its declared date, open it, and verify the document's
// exact bytes and SHA-256 against the fixture's report. run.mjs then checks,
// from the fixture's own log, that only the fixture origin was contacted
// and no older report was requested.
import { createHash } from 'node:crypto';
import { test } from '@e2e-dev/web';
import { expect } from 'e2e';

// The fixture's report, exactly as scripts/https-fixture.mjs builds it.
const report = (date: string): string =>
  `%PDF-1.4\n% /reports/${date}.pdf\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF\n`;
const sha256 = (text: string): string => createHash('sha256').update(text).digest('hex');

test('the latest dated report is the exact document', async ({ app, screen, browser }) => {
  await app.open('/reports');
  await expect(screen.getByRole('heading', 'Reports')).toBeVisible();
  const dates = ['2026-09-30', '2026-06-30', '2026-03-31'];
  for (const date of dates) {
    await expect(screen.getByRole('link', `Quarterly report ${date}`)).toBeVisible();
  }
  const latest = [...dates].sort().at(-1) ?? '';
  const opened = browser.waitForResponse(new RegExp(`/reports/${latest}\\.pdf$`));
  await screen.getByRole('link', `Quarterly report ${latest}`).tap();
  const response = await opened;
  expect(response.status).toBe(200);
  expect(response.headers['content-type']).toBe('application/pdf');
  // The viewer replaces the document's body with its own page, so the
  // bytes are read again from the same URL, same-origin, by this test's
  // own (independent) browser: a byte array, compared exactly.
  const codes = await browser.evaluate(
    async (url: string) => Array.from(new Uint8Array(await (await fetch(url)).arrayBuffer())),
    response.url,
  );
  const body = String.fromCharCode(...codes);
  expect(body).toBe(report(latest));
  expect(sha256(body)).toBe(sha256(report(latest)));
  console.log(`A8-EVIDENCE ${response.url} sha256 ${sha256(body)}`);
});

test('nothing outside the fixture origin is reachable', async ({ app, browser }) => {
  await app.open('/reports');
  // Every other name fails to resolve in this browser (fixture origin only).
  let reached = true;
  try {
    await browser.goto('https://other.test/');
  } catch {
    reached = false;
  }
  expect(reached).toBe(false);
});

test('an IP-literal bypass is attempted (run.mjs requires that it was denied)', async ({ app, browser }) => {
  // This test only makes the attempts. Whether they were denied is decided
  // outside the browser by run.mjs: its listeners at this address (this
  // host's own LAN address) must receive nothing, and the OS's connection
  // tables for every process of this browser must show nothing beyond
  // loopback. A failed navigation here proves nothing by itself.
  const ip = process.env.A8_LAN_IP;
  const tcp = process.env.A8_TCP_PORT;
  const udp = process.env.A8_UDP_PORT;
  if (!ip || !tcp || !udp) throw new Error('A8_LAN_IP, A8_TCP_PORT or A8_UDP_PORT is not set (run through run.mjs)');
  // From an admitted page: WebRTC ICE (STUN over UDP, TURN over TCP),
  // fetch, WebSocket and sendBeacon toward the same address, every 5 s.
  await app.open(`/rtc?ip=${ip}&udp=${udp}&tcp=${tcp}`);
  await new Promise((resolve) => setTimeout(resolve, 12_000));
  for (const target of [`https://${ip}:${tcp}/`, `http://${ip}:${tcp}/`]) {
    await browser.goto(target).catch(() => undefined);
  }
  console.log(`A8-EVIDENCE bypass attempted toward ${ip}:${tcp} (tcp) and :${udp} (udp)`);
});
