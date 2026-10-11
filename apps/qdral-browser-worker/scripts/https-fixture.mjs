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
<script>fetch("https://other.test/api").catch(() => {}); fetch("/api").catch(() => {});</script>`;

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
  if (path === "/loop-a") return redirect("/loop-b");
  if (path === "/loop-b") return redirect("/loop-a");
  const chain = /^\/chain\/(\d+)$/.exec(path);
  if (chain) return redirect(`/chain/${Number(chain[1]) + 1}`);
  if (path.endsWith(".png")) {
    response.setHeader("content-type", "image/png");
    return response.end(Buffer.from("89504e470d0a1a0a", "hex"));
  }
  if (path === "/api") {
    response.setHeader("content-type", "application/json");
    return response.end("{}");
  }
  response.setHeader("content-type", "text/html");
  response.end(path === "/mixed" ? MIXED : `<!doctype html><title>${host}${path}</title><p>fixture</p>`);
});
server.listen({ port: 443, host: "127.0.0.1" }, () => process.stdout.write(`READY ${spki}\n`));
process.stdin.resume();
process.stdin.on("end", () => process.exit(0));
process.stdin.on("error", () => process.exit(0));
