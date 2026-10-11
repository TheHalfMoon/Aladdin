// SG-000108 worker entry point: stdio framing around `Worker`, with the
// engine launched by playwright-core exactly as the host specified.
//
// stdout carries frames only: the global console is replaced by one whose
// every method writes to stderr, so no library output can corrupt the channel.

import { Console } from "node:console";
import { chromium } from "playwright-core";
import { refusedWorkerVariables, startupRefusal } from "./environment.js";
import { encodeFrame, FrameDecoder, ProtocolViolation, type WorkerFrame } from "./protocol.js";
import { Worker, type EngineSession, type Launcher, type LaunchSpec } from "./worker.js";

const startup = startupRefusal(process.execArgv, process.version);
if (startup !== null) {
  process.stderr.write(`qdral-browser-worker: refused to start: ${startup}\n`);
  process.exit(2);
}
const refused = refusedWorkerVariables(process.env);
if (refused.length > 0) {
  process.stderr.write(`qdral-browser-worker: refused environment variables: ${refused.join(", ")}\n`);
  process.exit(2);
}

globalThis.console = new Console({ stdout: process.stderr, stderr: process.stderr });

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

// A broken pipe in either direction ends the worker; the engine is closed.
process.stdin.on("error", () => void worker.violation());
process.stdout.on("error", () => void worker.violation());

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
