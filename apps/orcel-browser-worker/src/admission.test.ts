// SG-000108 dependency admission and environment hygiene, checked
// mechanically against the lockfile and the installed package.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { refusedWorkerVariables } from "./environment.js";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const spec = JSON.parse(readFileSync(join(appDir, "..", "..", ".specgrain", "specs", "SG-000108.json"), "utf8")) as {
  dependency_admission: { package: string; version: string; integrity: string };
};

test("playwright-core is admitted exactly as the SG-000108 record states", () => {
  const manifest = JSON.parse(readFileSync(join(appDir, "package.json"), "utf8")) as {
    dependencies: Record<string, string>;
  };
  assert.deepEqual(manifest.dependencies, { [spec.dependency_admission.package]: spec.dependency_admission.version });
  const lock = JSON.parse(readFileSync(join(appDir, "package-lock.json"), "utf8")) as {
    packages: Record<string, { version?: string; integrity?: string; dependencies?: object; hasInstallScript?: boolean; dev?: boolean }>;
  };
  const entry = lock.packages[`node_modules/${spec.dependency_admission.package}`];
  assert.equal(entry?.version, spec.dependency_admission.version);
  assert.equal(entry?.integrity, spec.dependency_admission.integrity);
  assert.equal(entry?.hasInstallScript, undefined, "no install script");
  assert.equal(entry?.dependencies, undefined, "no runtime dependencies");
  // The only production package in the lockfile is playwright-core.
  const production = Object.entries(lock.packages)
    .filter(([key, value]) => key.startsWith("node_modules/") && value.dev !== true)
    .map(([key]) => key);
  assert.deepEqual(production, ["node_modules/playwright-core"]);
});

test("the installed playwright-core is the locked version", () => {
  const require = createRequire(join(appDir, "package.json"));
  const installed = JSON.parse(readFileSync(require.resolve("playwright-core/package.json"), "utf8")) as {
    version: string;
    license: string;
  };
  assert.equal(installed.version, spec.dependency_admission.version);
  assert.equal(installed.license, "Apache-2.0");
});

test("code-injection and debug variables are refused case-insensitively", () => {
  assert.deepEqual(refusedWorkerVariables({ SystemRoot: "C:\\Windows", PATH: "C:\\Windows\\System32" }), []);
  assert.deepEqual(
    refusedWorkerVariables({
      NODE_OPTIONS: "--require evil.js",
      node_path: "C:\\evil",
      NODE_TLS_REJECT_UNAUTHORIZED: "0",
      PLAYWRIGHT_BROWSERS_PATH: "C:\\x",
      PW_TEST: "1",
      Debug: "pw:*",
      HTTPS_PROXY: "http://evil:8080",
      SSLKEYLOGFILE: "C:\\keys.log",
      ELECTRON_RUN_AS_NODE: "1",
      IGNORED: undefined
    }),
    ["Debug", "ELECTRON_RUN_AS_NODE", "HTTPS_PROXY", "NODE_OPTIONS", "NODE_TLS_REJECT_UNAUTHORIZED", "PLAYWRIGHT_BROWSERS_PATH", "PW_TEST", "SSLKEYLOGFILE", "node_path"]
  );
});
