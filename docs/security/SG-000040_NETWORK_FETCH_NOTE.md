# SG-000040 Destination-Scoped Network Fetch Security Note

## Exact authority delta

SG-000040 adds one explicit internal `network/fetch` operation only. The request accepts exactly one caller field, `url`, and authorizes a single HTTPS `GET` to port 443 after fresh SOFT approval. The capability does not add generic sockets, other HTTP methods, request bodies, caller headers, proxies, WebSockets, standing sessions, ambient credentials, filesystem writes, process execution, browser navigation, UI input, or elevation.

## Destination binding

The provider canonicalizes the HTTPS destination, requires a dotted DNS hostname, rejects userinfo, fragments, literal/numeric addresses, alternate ports, unsafe labels, and URLs longer than 2048 characters. Every DNS answer must be public. Loopback, private, link-local, multicast, unspecified, documentation/special-use, CGNAT, mapped-private, and other non-public address classes fail closed.

The pre-approval address set is sorted, deduplicated, and bound into the approval digest. After approval the hostname is resolved again and an address-set mismatch returns `TARGET_STALE`. Every request also revalidates the current set. Before sending any bytes, the Windows transport pins WinHTTP DNS resolution to one address from that validated set while retaining the authorized hostname for TLS certificate identity. After the response, the actual connected peer must exactly match the pinned address; a mismatch returns `TARGET_STALE`.

## Redirects

Automatic redirects are disabled. Redirects are handled manually and are limited to five hops. Only the exact original HTTPS origin is accepted. Cross-origin redirects, scheme downgrade, loops, malformed locations, private-address widening, and address-set drift fail closed.

## Request and response ceilings

The method is fixed to GET. Callers cannot provide headers or a body. WinHTTP uses a fresh direct/no-proxy session, OS certificate validation remains enabled, and authentication, cookies, automatic redirects, and keep-alive are disabled. Response bodies are incrementally bounded to 1,048,576 bytes. Error responses are typed failures and their bodies are not returned as error text.

## Approval and evidence

Each fetch requires fresh one-shot SOFT approval. The approval digest binds workspace, policy revision, capability, operation, GET, HTTPS, host, port, path digest, resolved-address-set digest, response ceiling, and redirect ceiling. Bounded result metadata contains only destination metadata, status, lengths, digests, hop count, and the bounded response body. No credentials or ambient secrets are attached.

## MCP ceiling

SG-000040 is an internal quntald capability only. No MCP network tool is registered. Node guard tests fail if a network tool source or registration is introduced.

## Qualification limits

Deterministic resolver and transport adapters cover SSRF, rebinding, peer mismatch, redirects, and bounds without public Internet dependence. The Windows adapter uses WinHTTP with OS certificate validation and an exact-peer pre-send resolution pin. Windows CI includes one explicit live transport probe against `https://example.com/`; that probe is qualification evidence only when the exact-head Windows test passes. Headless/unavailable native paths fail closed rather than fabricating network success.
