import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import { createInterface } from "node:readline";
import test from "node:test";

const repo = resolve(import.meta.dirname, "..", "..", "..");
const hostDir = join(repo, "apps", "deskal-computer-host");

function go(args: string[], cwd = hostDir) {
  return spawnSync("go", args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, GOTOOLCHAIN: "go1.25.0" },
    timeout: 180_000,
  });
}

test("SG-000094 Windows host builds with pinned Go and speaks only private stdio protocol", { timeout: 240_000 }, async (t) => {
  if (process.platform !== "win32") {
    t.skip("native Windows host qualification runs only on Node / windows-latest");
    return;
  }

  const version = go(["version"]);
  assert.equal(version.status, 0, version.stderr);
  assert.match(version.stdout, /go1\.25\.0\b/);

  const verify = go(["mod", "verify"]);
  assert.equal(verify.status, 0, verify.stderr);

  const unit = go(["test", "./..."]);
  assert.equal(unit.status, 0, unit.stdout + unit.stderr);

  const dir = mkdtempSync(join(tmpdir(), "deskal-sg94-"));
  const binary = join(dir, "deskal-computer-host.exe");
  try {
    const build = go(["build", "-trimpath", "-o", binary, "."]);
    assert.equal(build.status, 0, build.stdout + build.stderr);

    const child = spawn(binary, [], {
      stdio: ["pipe", "pipe", "pipe"],
      env: Object.fromEntries(
        ["SystemRoot", "WINDIR", "COMSPEC", "TEMP", "TMP", "PATH"]
          .map((key) => [key, process.env[key]])
          .filter((entry): entry is [string, string] => typeof entry[1] === "string"),
      ),
      windowsHide: true,
    });
    const lines = createInterface({ input: child.stdout });

    const request = (id: number, op: string) => {
      child.stdin.write(JSON.stringify({ id, protocol: "deskal-computer-host/1", op }) + "\n");
    };

    request(1, "hello");
    const hello = JSON.parse((await once(lines, "line"))[0] as string);
    assert.deepEqual(
      { ok: hello.ok, protocol: hello.protocol, host: hello.host },
      { ok: true, protocol: "deskal-computer-host/1", host: "deskal-windows-computer-host" },
    );

    request(2, "ping");
    const pong = JSON.parse((await once(lines, "line"))[0] as string);
    assert.equal(pong.ok, true);
    assert.equal(pong.pong, true);

    child.stdin.write(
      JSON.stringify({ id: 3, protocol: "deskal-computer-host/1", op: "click" }) + "\n",
    );
    const denied = JSON.parse((await once(lines, "line"))[0] as string);
    assert.equal(denied.ok, false);
    assert.equal(denied.error?.code, "capability_denied");

    request(4, "shutdown");
    const shutdown = JSON.parse((await once(lines, "line"))[0] as string);
    assert.equal(shutdown.ok, true);
    await once(child, "exit");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
