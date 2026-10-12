import assert from "node:assert/strict";
import { test } from "node:test";
import { classifyRequest, originOf, type FrameLike, type RequestLike } from "./routing.js";

const PRIMARY = { name: "primary" };
const POPUP = { name: "popup" };

function frame(url: string, parent: FrameLike | null = null, page: unknown = PRIMARY): FrameLike {
  return { url: () => url, parentFrame: () => parent, page: () => page };
}

function request(
  url: string,
  options: { type?: string; nav?: boolean; from?: string; sw?: boolean; frame?: FrameLike } = {}
): RequestLike {
  const from = options.from;
  return {
    url: () => url,
    resourceType: () => options.type ?? "document",
    isNavigationRequest: () => options.nav ?? false,
    redirectedFrom: () => (from ? request(from) : null),
    serviceWorker: () => (options.sw ? {} : null),
    frame: () => options.frame ?? frame("https://fixture.test/page")
  };
}

const primaryUrl = (): string => "https://fixture.test/page";

test("requests are described, never decided", () => {
  const top = frame("about:blank");
  assert.deepEqual(classifyRequest(request("https://fixture.test/a", { nav: true, frame: top }), PRIMARY, primaryUrl), {
    kind: "document",
    url: "https://fixture.test/a",
    origin: ""
  });
  assert.deepEqual(
    classifyRequest(request("https://fixture.test/b", { nav: true, from: "https://fixture.test/a", frame: top }), PRIMARY, primaryUrl),
    { kind: "redirect", url: "https://fixture.test/b", origin: "https://fixture.test" }
  );
  const child = frame("about:blank", frame("https://fixture.test/page"));
  assert.deepEqual(classifyRequest(request("https://other.test/f", { nav: true, frame: child }), PRIMARY, primaryUrl), {
    kind: "frame",
    url: "https://other.test/f",
    origin: "https://fixture.test"
  });
  const popup = frame("about:blank", null, POPUP);
  assert.deepEqual(classifyRequest(request("https://fixture.test/p", { nav: true, frame: popup }), PRIMARY, primaryUrl), {
    kind: "popup",
    url: "https://fixture.test/p",
    origin: "https://fixture.test"
  });
  for (const type of ["fetch", "xhr"]) {
    assert.equal(classifyRequest(request("https://fixture.test/api", { type }), PRIMARY, primaryUrl).kind, "fetch");
  }
  for (const type of ["script", "stylesheet", "image", "font", "media", "other", "eventsource", "manifest"]) {
    assert.deepEqual(classifyRequest(request("https://cdn.test/x", { type }), PRIMARY, primaryUrl), {
      kind: "subresource",
      url: "https://cdn.test/x",
      origin: "https://fixture.test"
    });
  }
  assert.equal(classifyRequest(request("https://fixture.test/sw.js", { sw: true }), PRIMARY, primaryUrl).kind, "service_worker");
});

test("origins are only derived from http(s) URLs", () => {
  assert.equal(originOf("https://fixture.test:443/x?y#z"), "https://fixture.test");
  assert.equal(originOf("https://fixture.test:8443/x"), "https://fixture.test:8443");
  for (const url of ["about:blank", "data:text/html,x", "blob:https://fixture.test/1", "javascript:1", "not a url"]) {
    assert.equal(originOf(url), "", url);
  }
});
