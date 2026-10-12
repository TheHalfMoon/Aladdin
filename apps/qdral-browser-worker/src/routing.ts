// SG-000108 slice 3a: classify each engine request for the host's
// navigation policy. The worker never decides: it describes the request
// (kind, URL, requesting origin) and applies the host's answer.

export type RouteKind = "document" | "redirect" | "frame" | "popup" | "fetch" | "subresource" | "service_worker";

export interface RouteQuery {
  kind: RouteKind;
  url: string;
  /// The requesting context's origin; empty for a top-level document.
  origin: string;
}

/// The subset of Playwright's Request and Frame this module reads, so it can
/// be tested without an engine.
export interface FrameLike {
  url(): string;
  parentFrame(): FrameLike | null;
  page(): unknown;
}

export interface RequestLike {
  url(): string;
  resourceType(): string;
  isNavigationRequest(): boolean;
  redirectedFrom(): RequestLike | null;
  serviceWorker(): unknown;
  frame(): FrameLike;
}

/// `https://host[:port]` for an http(s) URL, otherwise empty.
export function originOf(url: string): string {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:" || parsed.protocol === "http:" ? parsed.origin : "";
  } catch {
    return "";
  }
}

/// Describe a request. `primary` is the page the host navigates; any other
/// page is a popup, whose origin is that of the primary page (its opener).
export function classifyRequest(request: RequestLike, primary: unknown, primaryUrl: () => string): RouteQuery {
  const url = request.url();
  if (request.serviceWorker()) {
    return { kind: "service_worker", url, origin: originOf(url) };
  }
  const frame = request.frame();
  if (request.isNavigationRequest()) {
    const redirected = request.redirectedFrom();
    if (redirected) return { kind: "redirect", url, origin: originOf(redirected.url()) };
    const parent = frame.parentFrame();
    if (parent) return { kind: "frame", url, origin: originOf(parent.url()) };
    if (frame.page() !== primary) return { kind: "popup", url, origin: originOf(primaryUrl()) };
    return { kind: "document", url, origin: "" };
  }
  const type = request.resourceType();
  const kind: RouteKind = type === "fetch" || type === "xhr" ? "fetch" : "subresource";
  return { kind, url, origin: originOf(frame.url()) };
}
