// SG-000108 slice 3a: host decisions for redirect hops.
//
// Playwright's routes see only the first request of a navigation; Chromium
// follows redirects internally without calling them (measured: a
// cross-origin hop to another admitted name went out unasked). The CDP Fetch
// domain does pause every redirect hop at the request stage, marked with
// `redirectedRequestId`, so each page gets a Fetch session that forwards
// exactly those hops to the host as `redirect` queries and passes every
// other request through to the route stack (which asks the host itself).

import type { BrowserContext, CDPSession, Page } from "playwright-core";
import { originOf } from "./routing.js";
import type { RoutePolicy } from "./worker.js";

interface RequestPaused {
  requestId: string;
  request: { url: string };
  redirectedRequestId?: string;
}

export async function attachRedirectGuard(context: BrowserContext, page: Page, policy: RoutePolicy): Promise<void> {
  const session: CDPSession = await context.newCDPSession(page);
  // Interception ids of earlier hops map to their URLs, so each hop's
  // requesting origin is the origin of the response that redirected it.
  const urls = new Map<string, string>();
  session.on("Fetch.requestPaused", (event: RequestPaused) => {
    void (async () => {
      const { requestId } = event;
      const url = event.request.url;
      if (urls.size > 512) urls.clear();
      urls.set(requestId, url);
      try {
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
  await session.send("Fetch.enable", { patterns: [{ urlPattern: "*", requestStage: "Request" }] });
}
