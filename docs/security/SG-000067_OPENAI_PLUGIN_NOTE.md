# SG-000067 OpenAI Plugin Package Security Note

Status: IMPLEMENTATION FOR QDRAL-P17
SpecGrain: SG-000067
Base: `479594786197978411b259cfbc101984ec618b35`
Date: 2026-10-04

## Changes

1. `apps/qdral-mcp/src/openai_plugin.ts` builds and validates the package; `scripts/build-openai-plugin.mjs` writes it.
   `apps/qdral-mcp/src/discovery.ts` answers `tools/list` in process through the authoritative builder with a kernel that
   throws if called, so provider-facing tool metadata is never hand-written.
2. Every remotely mappable tool now declares `_meta.securitySchemes = [{ "type": "oauth2", "scopes": [<contract scope>] }]`
   on every transport, derived from the SG-000065 tool contract. No tool is anonymous; local-only tools declare none.
3. Defect fixed: the authorization-server metadata advertised `client_id_metadata_document_supported: true`, but the
   relay's authorization endpoint accepts only dynamically registered clients. A provider preferring client ID metadata
   documents would have been refused. The metadata now advertises `false`; dynamic client registration remains the
   supported path. No authorization path was added.
4. The relay can answer `GET /.well-known/openai-apps-challenge` with an operator-configured token (8 to 512 URL-safe
   characters, validated at configuration load and again at response time) as `text/plain` with `no-store`; absent
   configuration returns 404. Tests prove the token is not a bearer credential: `/mcp` still returns 401 with it.
5. `SECURITY.md`, `docs/legal/PRIVACY.md`, and `docs/legal/TERMS.md` (owner-review drafts) back the listing URLs, which
   are free `github.com` URLs.

## Authority delta

None. No kernel capability, MCP tool, OAuth scope, profile, approval class, trust right, executable admission, lease
right, browser or desktop actuation, shell, interpreter, or generic network authority is added. The package declares one
server, the existing relay edge.

## Secrets

Package and review artifacts are scanned for private keys, OpenAI-style keys, JWTs, client secrets, access and refresh
tokens, JWK private components, GitHub and AWS keys, and passwords; any match fails the build. No endpoint, token, or
credential is committed.

## Distribution states

`distribution/openai/review/distribution-state.json` keeps SOFTWARE_READY, SUBMISSION_READY, SUBMITTED, APPROVED,
PUBLISHED, and ACCOUNT_VERIFIED distinct. Provider-controlled states stay NOT_OBSERVED unless directly observed provider
evidence is recorded; tests fail if one is promoted without evidence. SUBMISSION_READY is BLOCKED on owner-controlled
prerequisites listed there.
