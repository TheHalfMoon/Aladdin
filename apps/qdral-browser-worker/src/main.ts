// SG-000108 worker entry point: stdio framing around `Worker`, with the
// engine launched by playwright-core exactly as the host specified.
//
// stdout carries frames only: the global console is replaced by one whose
// every method writes to stderr, so no library output can corrupt the channel.

import { Console } from "node:console";
import { chromium, type Page } from "playwright-core";
import { refusedWorkerVariables, startupRefusal } from "./environment.js";
import { encodeFrame, FrameDecoder, ProtocolViolation, type WorkerFrame } from "./protocol.js";
import { installRoutes } from "./routes.js";
import { NavigationError, Worker, type EngineSession, type Launcher, type LaunchSpec, type RoutePolicy } from "./worker.js";

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
  async launch(spec: LaunchSpec, policy: RoutePolicy): Promise<EngineSession> {
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
      // "allow" keeps service-worker requests visible to routing; the
      // route stack refuses every registration (#301).
      serviceWorkers: "allow",
      handleSIGINT: false,
      handleSIGTERM: false,
      handleSIGHUP: false,
      timeout: 30_000
    });
    // Each host navigation runs in a fresh page: Chromium can commit an
    // earlier refused navigation's error page late (even more than once),
    // which would interrupt a navigation in the same page. The previous
    // page is closed once the new one has its redirect guard.
    let primary: Page = context.pages()[0] ?? (await context.newPage());
    const routes = await installRoutes(context, () => primary, policy);
    let closing = false;
    return {
      async navigate(url: string) {
        const previous = primary;
        const page = await context.newPage();
        await routes.guardReady(page);
        primary = page;
        await previous.close().catch(() => {});
        try {
          const response = await page.goto(url, { timeout: 30_000, waitUntil: "load" });
          return { url: page.url(), status: response?.status() ?? 0 };
        } catch (error) {
          const message = String((error as Error).message);
          if (message.includes("ERR_BLOCKED_BY_CLIENT")) throw new NavigationError("denied");
          if ((error as Error).name === "TimeoutError") throw new NavigationError("timeout");
          throw new NavigationError("failed");
        }
      },
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
