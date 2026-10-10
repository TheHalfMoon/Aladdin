// SG-000108 worker entry point: stdio framing around `Worker`, with the
// engine launched by playwright-core exactly as the host specified.
//
// stdout carries frames only; every console method writes to stderr so that
// no library output can corrupt the channel.

import { chromium } from "playwright-core";
import { refusedWorkerVariables } from "./environment.js";
import { encodeFrame, FrameDecoder, ProtocolViolation, type WorkerFrame } from "./protocol.js";
import { Worker, type EngineSession, type Launcher, type LaunchSpec } from "./worker.js";

const refused = refusedWorkerVariables(process.env);
if (refused.length > 0) {
  process.stderr.write(`qdral-browser-worker: refused environment variables: ${refused.join(", ")}\n`);
  process.exit(2);
}

const toStderr = (...parts: unknown[]): void => {
  process.stderr.write(`${parts.map(String).join(" ")}\n`);
};
// Redirect (not use) every console output method: stdout is the frame
// channel and nothing else may write to it.
for (const method of ["log", "info", "debug", "warn"] as const) {
  console[method] = toStderr;
}

const playwrightLauncher: Launcher = {
  async launch(spec: LaunchSpec): Promise<EngineSession> {
    // `ignoreDefaultArgs: true` keeps Playwright from adding any flag: the
    // engine receives exactly the host-built argv (which already carries the
    // profile, the pipe, and the blank page).
    const context = await chromium.launchPersistentContext(spec.profile_dir, {
      executablePath: spec.engine,
      ignoreDefaultArgs: true,
      args: spec.argv,
      env: spec.env,
      headless: true,
      acceptDownloads: false,
      serviceWorkers: "block",
      handleSIGINT: false,
      handleSIGTERM: false,
      handleSIGHUP: false,
      timeout: 30_000
    });
    let closing = false;
    return {
      async close() {
        closing = true;
        await context.close();
      },
      onUnexpectedExit(listener) {
        context.on("close", () => {
          if (!closing) listener();
        });
      }
    };
  }
};

function send(frame: WorkerFrame): void {
  process.stdout.write(encodeFrame(frame));
}

function finish(exitCode: number): void {
  process.stdin.pause();
  // Exit only after the last frame has been flushed to the host.
  process.stdout.write(Buffer.alloc(0), () => process.exit(exitCode));
}

const worker = new Worker(playwrightLauncher, send, finish);
const decoder = new FrameDecoder();

process.stdin.on("data", (chunk: Buffer) => {
  let frames: unknown[];
  try {
    frames = decoder.push(chunk);
  } catch (error) {
    if (error instanceof ProtocolViolation) {
      void worker.violation();
      return;
    }
    throw error;
  }
  for (const frame of frames) void worker.receive(frame);
});
process.stdin.on("end", () => {
  void (decoder.pendingBytes > 0 ? worker.violation() : worker.endOfInput());
});
