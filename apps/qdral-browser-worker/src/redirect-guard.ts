// SG-000108 slice 3a/3d: host decisions for redirect hops, and inert
// capture of an approved download.
//
// Playwright's routes see only the first request of a navigation; Chromium
// follows redirects internally without calling them (measured: a
// cross-origin hop to another admitted name went out unasked). The CDP Fetch
// domain does pause every redirect hop at the request stage, marked with
// `redirectedRequestId`, so each page gets a Fetch session that forwards
// exactly those hops to the host as `redirect` queries and passes every
// other request through to the route stack (which asks the host itself).
//
// Slice 3d: headless Edge renders a PDF navigation in its built-in viewer
// (no download event, and the navigation response body is the viewer's
// wrapper page; both measured). While the host asks for a download, the
// guard also pauses the next main-frame document at the response stage,
// takes the original bytes with Fetch.getResponseBody (no new request), and
// fulfils the navigation with an inert blank page, so the document is never
// rendered.

import type { BrowserContext, CDPSession, Page } from "playwright-core";
import { originOf } from "./routing.js";
import type { RoutePolicy } from "./worker.js";

/// Upper bound on a captured document (mirrors the host's transfer bound).
export const MAX_CAPTURE_BYTES = 8 * 1024 * 1024;

interface RequestPaused {
  requestId: string;
  request: { url: string };
  resourceType: string;
  frameId: string;
  redirectedRequestId?: string;
  responseStatusCode?: number;
  responseHeaders?: Array<{ name: string; value: string }>;
}

export interface CapturedDocument {
  url: string;
  status: number;
  media: string;
  disposition: string;
  body: Buffer;
}

export interface RedirectGuard {
  /// Capture the next main-frame document response of this page instead of
  /// rendering it. Resolves with the original bytes; rejects on timeout or
  /// an oversized body. Only one capture at a time.
  captureNextDocument(timeoutMs: number): Promise<CapturedDocument>;
}

const REQUEST_STAGE = [{ urlPattern: "*", requestStage: "Request" as const }];
const WITH_DOCUMENT_RESPONSES = [
  ...REQUEST_STAGE,
  { urlPattern: "*", resourceType: "Document" as const, requestStage: "Response" as const }
];

export async function attachRedirectGuard(context: BrowserContext, page: Page, policy: RoutePolicy): Promise<RedirectGuard> {
  const session: CDPSession = await context.newCDPSession(page);
  // Interception ids of earlier hops map to their URLs, so each hop's
  // requesting origin is the origin of the response that redirected it.
  const urls = new Map<string, string>();
  let capture: { resolve: (doc: CapturedDocument) => void; reject: (error: Error) => void } | null = null;

  const onResponse = async (event: RequestPaused): Promise<void> => {
    const { requestId } = event;
    const pending = capture;
    // Only the main frame's document is captured; anything else (an iframe
    // racing the capture) continues untouched.
    if (!pending || event.frameId !== mainFrame) {
      await session.send("Fetch.continueRequest", { requestId });
      return;
    }
    capture = null;
    const header = (name: string): string =>
      event.responseHeaders?.find((h) => h.name.toLowerCase() === name)?.value ?? "";
    try {
      const declared = Number(header("content-length"));
      if (Number.isFinite(declared) && declared > MAX_CAPTURE_BYTES) throw new Error("oversized");
      const result = (await session.send("Fetch.getResponseBody", { requestId })) as { body: string; base64Encoded: boolean };
      const body = Buffer.from(result.body, result.base64Encoded ? "base64" : "utf8");
      if (body.length > MAX_CAPTURE_BYTES) throw new Error("oversized");
      pending.resolve({
        url: event.request.url,
        status: event.responseStatusCode ?? 0,
        media: header("content-type"),
        disposition: header("content-disposition"),
        body
      });
    } catch (error) {
      pending.reject(error as Error);
    } finally {
      // The document is never rendered: an inert blank page replaces it.
      await session.send("Fetch.fulfillRequest", {
        requestId,
        responseCode: 200,
        responseHeaders: [{ name: "content-type", value: "text/plain" }],
        body: ""
      });
      await session.send("Fetch.enable", { patterns: REQUEST_STAGE });
    }
  };

  session.on("Fetch.requestPaused", (event: RequestPaused) => {
    void (async () => {
      const { requestId } = event;
      const url = event.request.url;
      try {
        if (event.responseStatusCode !== undefined) {
          await onResponse(event);
          return;
        }
        if (urls.size > 512) urls.clear();
        urls.set(requestId, url);
        if (event.redirectedRequestId === undefined) {
          await session.send("Fetch.continueRequest", { requestId });
          return;
        }
        const from = urls.get(event.redirectedRequestId) ?? "";
        let allow = false;
        try {
          allow = await policy.decide({ kind: "redirect", url, origin: originOf(from) });
        } catch {
          allow = false;
        }
        if (allow) await session.send("Fetch.continueRequest", { requestId });
        else await session.send("Fetch.failRequest", { requestId, errorReason: "BlockedByClient" });
      } catch {
        // The target went away; nothing is left to decide.
      }
    })();
  });
  const tree = (await session.send("Page.getFrameTree")) as { frameTree: { frame: { id: string } } };
  const mainFrame = tree.frameTree.frame.id;
  await session.send("Fetch.enable", { patterns: REQUEST_STAGE });

  return {
    async captureNextDocument(timeoutMs: number): Promise<CapturedDocument> {
      if (capture) throw new Error("a capture is already armed");
      const captured = new Promise<CapturedDocument>((resolve, reject) => {
        capture = { resolve, reject };
      });
      await session.send("Fetch.enable", { patterns: WITH_DOCUMENT_RESPONSES });
      let timer: NodeJS.Timeout | undefined;
      const timeout = new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(Object.assign(new Error("capture timed out"), { name: "TimeoutError" })), timeoutMs);
      });
      try {
        return await Promise.race([captured, timeout]);
      } finally {
        clearTimeout(timer);
        if (capture) {
          capture = null;
          await session.send("Fetch.enable", { patterns: REQUEST_STAGE }).catch(() => {});
        }
      }
    }
  };
}
