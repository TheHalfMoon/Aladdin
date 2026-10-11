// Service-worker denial (closes #301).
//
// Playwright's `serviceWorkers: "block"` only replaces
// `navigator.serviceWorker.register` with an init script, so a page can
// register a real worker through `ServiceWorkerContainer.prototype.register`,
// and with "block" the script fetch is invisible to routing. The worker
// therefore launches with `serviceWorkers: "allow"` (which makes every
// service-worker request, including the script fetch, visible to context
// routes) and installs this route before any page navigates: every request
// carrying `Service-Worker: script` is aborted, so no registration can
// complete. The future SG-000075 policy route must keep this denial.

import type { BrowserContext, Route } from "playwright-core";

export async function denyServiceWorkerScripts(context: BrowserContext): Promise<void> {
  await context.route("**/*", async (route: Route) => {
    const headers = await route.request().allHeaders();
    if (headers["service-worker"] === "script") {
      await route.abort("blockedbyclient");
      return;
    }
    await route.fallback();
  });
}
