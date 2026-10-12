#!/usr/bin/env node
// Test-only HTTPS fixture for the SG-000108 native launcher tests
// (crates/qdral-browser-host/src/worker_launch_tests.rs). Never packaged.
//
// Serves fixture.test and other.test on 127.0.0.1:443 with a throwaway
// certificate (Git for Windows' OpenSSL). Protocol on stdout, one line each:
//   READY <base64 SPKI SHA-256>
//   HIT <host> <path>
// It exits when its stdin closes, so the test owns its lifetime.

import { execFileSync } from "node:child_process";
import { createHash, X509Certificate } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:https";
import { tmpdir } from "node:os";
import { join } from "node:path";

const openssl = ["C:\\Program Files\\Git\\usr\\bin\\openssl.exe", "C:\\Program Files\\Git\\mingw64\\bin\\openssl.exe"].find(
  (path) => existsSync(path)
);
if (!openssl) {
  process.stderr.write("https-fixture: OpenSSL (Git for Windows) is required\n");
  process.exit(1);
}
const dir = mkdtempSync(join(tmpdir(), "sg108-fixture-"));
execFileSync(
  openssl,
  [
    "req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-nodes",
    "-keyout", join(dir, "key.pem"), "-out", join(dir, "cert.pem"), "-days", "2",
    "-subj", "/CN=fixture.test", "-addext", "subjectAltName=DNS:fixture.test,DNS:other.test"
  ],
  { stdio: "ignore" }
);
const cert = readFileSync(join(dir, "cert.pem"));
const key = readFileSync(join(dir, "key.pem"));
rmSync(dir, { recursive: true, force: true });
const spki = createHash("sha256")
  .update(new X509Certificate(cert).publicKey.export({ type: "spki", format: "der" }))
  .digest("base64");

const MIXED = `<!doctype html><title>mixed</title>
<img src="/pixel.png"><img src="https://other.test/pixel.png">
<iframe src="https://other.test/frame"></iframe><iframe src="/frame"></iframe>
<link rel="prefetch" href="https://other.test/prefetch">
<link rel="stylesheet" href="/style.css">
<link rel="prerender" href="/redirect-cross">
<script type="speculationrules">{"prerender":[{"source":"list","urls":["https://other.test/prerender"]}],"prefetch":[{"source":"list","urls":["https://other.test/specprefetch"]}]}</script>
<a id="pinged" href="/landing" ping="https://other.test/ping">x</a>
<script>
fetch("https://other.test/api").catch(() => {});
fetch("/api").catch(() => {});
try { navigator.sendBeacon("https://other.test/beacon", "x"); } catch {}
try { new EventSource("https://other.test/events"); } catch {}
try { new Worker(URL.createObjectURL(new Blob(['importScripts("https://other.test/worker-import.js")'], { type: "text/javascript" }))); } catch {}
try { new SharedWorker("/shared.js"); } catch {}
// Browser-process requests a judge measured outside every route and guard.
try { fetchLater("https://other.test/fetchlater", { activateAfter: 0 }); } catch {}
try { navigator.credentials.get({ identity: { providers: [{ configURL: "https://other.test/fedcm.json", clientId: "x" }] } }).catch(() => {}); } catch {}
try { new AudioContext().audioWorklet.addModule("https://other.test/audio.js").catch(() => {}); } catch {}
try { const s = document.createElement("img"); s.src = "http://insecure.other.test/csp-violation.png"; document.body.append(s); } catch {}
try { window.open("https://other.test/popup"); } catch {}
try { window.open("/redirect-cross"); } catch {}
</script>`;

// Slice 3b observation fixtures: a dated report list with a filled login
// form, and a page that moves itself after loading.
const REPORTS = `<!doctype html><title>Reports</title><h1>Reports</h1><ul>
<li><a href="/reports/2026-09-30.pdf">Quarterly report 2026-09-30</a></li>
<li><a href="/reports/2026-06-30.pdf">Quarterly report 2026-06-30</a></li>
<li><a href="/reports/2026-03-31.pdf">Quarterly report 2026-03-31</a></li></ul>
<form><label>User <input name=u value="alice"></label>
<label>Password <input type=password value="hunter2-secret"></label><button>Sign in</button></form>`;
const FILES = `<!doctype html><title>Files</title><ul>
<li><a href="/files/oversized.pdf">Oversized 2026-01-01</a></li>
<li><a href="/files/fake.pdf">Disguised 2026-01-02</a></li>
<li><a href="/files/poly.pdf">Polyglot 2026-01-03</a></li>
<li><a href="/reports/2026-09-30.pdf#page=2">Fragment 2026-01-04</a></li>
<li><a href="/files/stall.pdf">Stalled 2026-01-05</a></li></ul>`;
const ARCHIVE = `<!doctype html><title>Archive</title><ul>
<li><a href="/archive/2026-06-30">Annual review 2026-06-30</a></li>
<li><a href="/archive/2026-09-30">Annual review 2026-09-30</a></li>
<li><a href="/archive/2025-12-31">Annual review 2025-12-31</a></li></ul>`;
const ARCHIVE_HOSTILE = `<!doctype html><title>Hostile archive</title><ul>
<li><a href="/archive/2026-09-30">Annual review 2026-09-30</a></li>
<li><a href="https://other.test/archive/2099-12-31">Annual review 2099-12-31</a></li></ul>`;
// Slice 3c attacks a judge verified: a click redirected by script, an href
// swapped after observation, and a button named by a password field.
const ARCHIVE_TRICKY = `<!doctype html><title>Tricky archive</title><ul>
<li><a id=js href="/archive/2026-09-30" onclick="event.preventDefault(); location.href='/js-dest';">Annual review 2026-09-30</a></li>
<li><a id=swap href="/archive/2026-06-30">Annual review 2026-06-30</a></li></ul>
<script>setTimeout(() => { document.getElementById("swap").href = "/evil"; }, 1500);</script>`;
const SECRET_NAME = `<!doctype html><title>Secret name</title>
<input type=password id=pw value="HUNTER2PW"><button aria-labelledby=pw>x</button>`;
const SELF_MOVE = `<!doctype html><title>moving</title><script>setTimeout(() => { location.href = "/landing"; }, 300);</script>`;

// The escape a judge proved against slice 3a: prerender a same-origin page
// that redirects cross-origin, then activate it with no new request.
const SPEC_ESCAPE = `<!doctype html><title>spec-escape</title>
<script type="speculationrules">{"prerender":[{"source":"list","urls":["/redirect-evil"]}]}</script>
<script>setTimeout(() => { location.href = "/redirect-evil"; }, 1500);</script>`;

const server = createServer({ key, cert }, (request, response) => {
  const host = (request.headers.host ?? "").split(":")[0];
  const path = request.url ?? "/";
  process.stdout.write(`HIT ${host} ${path}\n`);
  const redirect = (location) => {
    response.statusCode = 302;
    response.setHeader("location", location);
    response.end();
  };
  if (path === "/redirect-same") return redirect("/landing");
  if (path === "/redirect-cross") return redirect("https://other.test/landing");
  if (path === "/redirect-evil") return redirect("https://other.test/evil");
  if (path === "/loop-a") return redirect("/loop-b");
  if (path === "/loop-b") return redirect("/loop-a");
  const chain = /^\/chain\/(\d+)$/.exec(path);
  if (chain) return redirect(`/chain/${Number(chain[1]) + 1}`);
  if (path === "/shared.js") {
    response.setHeader("content-type", "text/javascript");
    return response.end('fetch("https://other.test/shared-fetch").catch(() => {}); try { importScripts("https://other.test/shared-import.js"); } catch {}');
  }
  if (path === "/style.css") {
    response.setHeader("content-type", "text/css");
    return response.end('@import url("https://other.test/imported.css"); body { color: black; }');
  }
  if (path.startsWith("/reports/") && path.endsWith(".pdf")) {
    // A small, valid-looking PDF whose bytes name the report.
    const body = Buffer.from(`%PDF-1.4
% ${path}
1 0 obj << /Type /Catalog >> endobj
trailer << /Root 1 0 R >>
%%EOF
`);
    response.setHeader("content-type", "application/pdf");
    response.setHeader("content-length", String(body.length));
    return response.end(body);
  }
  if (path === "/files/oversized.pdf") {
    response.setHeader("content-type", "application/pdf");
    return response.end(Buffer.concat([Buffer.from("%PDF-1.4\n"), Buffer.alloc(9 * 1024 * 1024, 0x20)]));
  }
  if (path === "/files/stall.pdf") {
    // Headers and the start of a PDF, then nothing: the body never ends
    // while this fixture runs.
    response.setHeader("content-type", "application/pdf");
    response.write("%PDF-1.4\n");
    return;
  }
  if (path === "/files/poly.pdf") {
    // A real PDF header that is also a batch script and HTML.
    response.setHeader("content-type", "application/pdf");
    return response.end("%PDF-1.4\r\n@echo POLYGLOT\r\n<script>alert(1)</script>\r\n");
  }
  if (path === "/files/fake.pdf") {
    // An executable disguised as a PDF.
    response.setHeader("content-type", "application/pdf");
    return response.end(Buffer.concat([Buffer.from("MZ"), Buffer.alloc(64, 0)]));
  }
  if (path.endsWith(".png")) {
    response.setHeader("content-type", "image/png");
    return response.end(Buffer.from("89504e470d0a1a0a", "hex"));
  }
  if (path === "/api") {
    response.setHeader("content-type", "application/json");
    return response.end("{}");
  }
  response.setHeader("content-type", "text/html");
  if (path === "/mixed") {
    // Reporting API: CSP violations are reported by the network service.
    response.setHeader("reporting-endpoints", 'csp="https://other.test/reports"');
    response.setHeader("content-security-policy", "img-src 'self'; report-to csp");
  }
  const pages = { "/mixed": MIXED, "/spec-escape": SPEC_ESCAPE, "/reports": REPORTS, "/self-move": SELF_MOVE, "/archive": ARCHIVE, "/archive-hostile": ARCHIVE_HOSTILE, "/files": FILES, "/archive-tricky": ARCHIVE_TRICKY, "/secret-name": SECRET_NAME };
  response.end(pages[path] ?? `<!doctype html><title>${host}${path}</title><p>fixture</p>`);
});
server.listen({ port: 443, host: "127.0.0.1" }, () => process.stdout.write(`READY ${spki}\n`));
process.stdin.resume();
process.stdin.on("end", () => process.exit(0));
process.stdin.on("error", () => process.exit(0));
