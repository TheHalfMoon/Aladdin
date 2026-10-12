// SG-000108 worker entry point: stdio framing around `Worker`, with the
// engine launched by playwright-core exactly as the host specified.
//
// stdout carries frames only: the global console is replaced by one whose
// every method writes to stderr, so no library output can corrupt the channel.

import { Console } from "node:console";
import { existsSync, writeFileSync } from "node:fs";
import { chromium, type Page } from "playwright-core";
import { refusedWorkerVariables, startupRefusal } from "./environment.js";
import { encodeFrame, FrameDecoder, ProtocolViolation, type WorkerFrame } from "./protocol.js";
import { installRoutes } from "./routes.js";
import { boundSnapshot } from "./snapshot.js";
import { discoverDeclarativeTools } from "./webmcp/declarative.js";
import { documentModel, type CapturedSnapshot } from "./webmcp/dom-snapshot.js";
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
      // Downloads never happen through the engine: an approved document is
      // captured by the redirect guard instead (slice 3d).
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
      async activate(ref: string) {
        const page = primary;
        let status = 0;
        const onResponse = (response: import("playwright-core").Response) => {
          const request = response.request();
          if (request.isNavigationRequest() && request.frame() === page.mainFrame()) status = response.status();
        };
        page.on("response", onResponse);
        try {
          const navigated = page.waitForEvent("framenavigated", {
            predicate: (frame) => frame === page.mainFrame(),
            timeout: 30_000
          });
          try {
            await page.locator(`aria-ref=${ref}`).click({ timeout: 5_000 });
          } catch {
            navigated.catch(() => {});
            throw new NavigationError("failed");
          }
          await navigated;
          await page.waitForLoadState("load", { timeout: 30_000 });
        } catch (error) {
          if (error instanceof NavigationError) throw error;
          if ((error as Error).name === "TimeoutError") throw new NavigationError("timeout");
          throw new NavigationError("failed");
        } finally {
          page.off("response", onResponse);
        }
        // A refused navigation lands on Chromium's error page.
        if (page.url().startsWith("chrome-error:")) throw new NavigationError("denied");
        return { url: page.url(), status };
      },
      async download(ref: string, path: string) {
        if (existsSync(path)) throw new NavigationError("failed");
        const page = primary;
        const guard = await routes.guardReady(page);
        // Arm before clicking: the click's navigation (each request and
        // redirect hop still decided by the host) ends in a document whose
        // original bytes are captured and never rendered.
        const captured = guard.captureNextDocument(30_000);
        // The capture can settle (for example refused as oversized) while
        // the click is still in flight; without a handler attached now that
        // rejection would be unhandled and end the worker (measured).
        captured.catch(() => {});
        // On any failure the page is closed: a document arriving after the
        // capture gave up would otherwise be rendered. The host requires a
        // fresh navigation (which opens a new page) before acting again.
        const fail = async (reason: "failed" | "timeout"): Promise<never> => {
          await page.close().catch(() => {});
          throw new NavigationError(reason);
        };
        try {
          await page.locator(`aria-ref=${ref}`).click({ timeout: 5_000, noWaitAfter: true });
        } catch {
          return fail("failed");
        }
        let document;
        try {
          document = await captured;
        } catch (error) {
          return fail((error as Error).name === "TimeoutError" ? "timeout" : "failed");
        }
        if (document.status < 200 || document.status >= 300) return fail("failed");
        writeFileSync(path, document.body, { flag: "wx" });
        return {
          url: document.url,
          name: suggestedName(document.url, document.disposition),
          media: document.media,
          bytes: document.body.length
        };
      },
      async observe() {
        const raw: unknown = await primary.ariaSnapshotJSON({ mode: "ai", depth: 16, timeout: 2_000 });
        return { url: primary.url(), snapshot: boundSnapshot(raw) };
      },
      async discover(discId: number) {
        // The engine serialises its own DOM: no page script runs.
        const before = primary.url();
        const session = await context.newCDPSession(primary);
        let captured: CapturedSnapshot;
        try {
          // Playwright's protocol typings name this result differently;
          // documentModel reads every field defensively (missing arrays are
          // empty, a missing document throws into discovery_failed).
          captured = (await session.send("DOMSnapshot.captureSnapshot", { computedStyles: [] })) as unknown as CapturedSnapshot;
        } finally {
          await session.detach().catch(() => {});
        }
        const model = documentModel(captured);
        // The snapshot must be of the page's document: if the page moved
        // during the capture, the result is stale and is refused.
        const url = primary.url();
        if (model.url !== url || before !== url) throw new Error("the document changed during discovery");
        const found = discoverDeclarativeTools(model.root);
        const tools = [...found.tools];
        let truncated = model.truncated || found.truncated;
        // Drop tools from the end until the reply frame fits.
        while (tools.length > 0) {
          try {
            encodeFrame({ frame: "discovered", disc_id: discId, url, truncated, tools });
            break;
          } catch {
            tools.pop();
            truncated = true;
          }
        }
        return { url, truncated, tools };
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

/// The file name a download would get: the Content-Disposition filename when
/// present, else the last URL path segment; never a path.
function suggestedName(url: string, disposition: string): string {
  const quoted = /filename="([^"]{1,200})"/i.exec(disposition)?.[1];
  const bare = /filename=([^;\s]{1,200})/i.exec(disposition)?.[1];
  let name = quoted ?? bare ?? "";
  if (!name) {
    try {
      name = decodeURIComponent(new URL(url).pathname.split("/").pop() ?? "");
    } catch {
      name = "";
    }
  }
  return name.replace(/[\\/:*?"<>|\x00-\x1f]/g, "_").slice(0, 200) || "download";
}

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
