# SG-000069 Mistral Distribution Security Note

Status: IMPLEMENTATION FOR QDRAL-P17
SpecGrain: SG-000069
Base: `ff2c174090755f19e94f0cd323fe618ad89a569a`
Date: 2026-10-04

## What was qualified

- Vibe Code CLI local paths: the stdio entry and the commented loopback `streamable-http` entry are pinned by tests to the
  canonical entrypoint, the `127.0.0.1` loopback listener, and `api_key_env`-based bearer delivery with no literal
  credential and no remote URL.
- Vibe Work remote path: a dynamically registered OAuth client with an https callback pairs a device through the real
  relay and discovers exactly the `core` profile.

## Decision: no static credential on the relay

Vibe Code cannot use OAuth-protected servers. Adding a static API key or bearer path to the public relay would create a
second authorization path outside OAuth, device-backed pairing, scope ceilings, and token revocation, so it is excluded.
Tests prove static bearer, loopback-style, and Basic credentials receive 401 with an OAuth `WWW-Authenticate` challenge.

## Authority delta

None. Code changes are tests, examples verification, a distribution record, and documentation.
