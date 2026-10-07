import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repo = resolve(import.meta.dirname, "..", "..", "..");
const hostDir = join(repo, "apps", "deskal-computer-host");

function go(args: string[]) {
  return spawnSync("go", args, {
    cwd: hostDir,
    encoding: "utf8",
    env: { ...process.env, GOTOOLCHAIN: "go1.25.0" },
    timeout: 240_000
  });
}

test("SG-000095 native Windows desktop input qualification runs the disposable-window suite", { timeout: 300_000 }, (t) => {
  if (process.platform !== "win32") {
    t.skip("SG-000095 native input qualification runs only on Node / windows-latest");
    return;
  }
  const version = go(["version"]);
  if ((version.error as NodeJS.ErrnoException | undefined)?.code === "ENOENT") {
    t.skip("local Go is unavailable; exact native qualification runs in Node / windows-latest");
    return;
  }
  assert.equal(version.status, 0, version.stderr);
  assert.match(version.stdout, /go1\.25\.0\b/);

  const result = go(["test", "-count=1", "-run", "^TestSG95", "./..."]);
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test("SG-000095 native mutation source stays private, bounded, and excludes donor widening", () => {
  const required = [
    "native_actions_windows.go",
    "native_input_windows.go",
    "native_window_actions_windows.go",
    "native_input_qualification_windows_test.go"
  ];
  for (const name of required) {
    assert.equal(existsSync(join(hostDir, name)), true, name);
  }
  const source = required
    .filter((name) => !name.endsWith("_test.go"))
    .map((name) => readFileSync(join(hostDir, name), "utf8"))
    .join("\n");

  assert.match(source, /SendInput/);
  assert.match(source, /GetLastInputInfo/);
  assert.match(source, /expected.*input/i);
  assert.match(source, /semantic/i);
  assert.match(source, /window_close/);

  for (const forbidden of [
    /ListenAndServe\s*\(/,
    /\bnet\.Listen\s*\(/,
    /["']net\/http["']/,
    /["']os\/exec["']/,
    /ffmpeg\.exe/i,
    /CreateProcess|ShellExecute/i,
    /runas/i,
    /TrustedInstaller/i,
    /SYSTEM authority/i
  ]) {
    assert.doesNotMatch(source, forbidden);
  }
});

test("SG-000095 authority can be granted only by local lifecycle code, never MCP or relay", () => {
  const desktop = readFileSync(join(repo, "apps", "qdral-mcp", "src", "desktop.ts"), "utf8");
  const lifecycle = readFileSync(join(repo, "crates", "qdral-lifecycle", "src", "full_control.rs"), "utf8");
  const daemon = readFileSync(join(repo, "crates", "qdrald", "src", "sg000039_main.rs"), "utf8");

  assert.match(lifecycle, /ApprovalPrompt::new_strong/);
  assert.match(lifecycle, /grant_full_user/);
  assert.match(lifecycle, /active_runtime_session/);
  assert.doesNotMatch(desktop, /grant_full_user|issue_full_control|LocalGrantProof|full-control grant/);
  assert.match(daemon, /is_full_user_desktop_shape/);
  assert.match(daemon, /Remote Full Control remains unavailable|remote full control/i);
});
