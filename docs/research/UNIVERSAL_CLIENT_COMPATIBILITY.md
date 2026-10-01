# Cotra Universal Client Compatibility Matrix

Status: RESEARCH SNAPSHOT FOR IMPLEMENTATION
Verified: 2026-10-01
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`

This document records current provider requirements that materially affect Cotra's transport, authentication, packaging, and distribution design. Provider product policy can change; implementation must re-verify these requirements before public submission or release claims.

## 1. OpenAI ChatGPT / Codex

Official references:

- Plugins quickstart: https://developers.openai.com/plugins/quickstart
- MCP server build guide: https://developers.openai.com/plugins/build/mcp-server
- Authentication: https://developers.openai.com/plugins/build/auth
- Connect/test: https://developers.openai.com/plugins/deploy/connect-chatgpt
- Submission: https://developers.openai.com/plugins/deploy/submission
- Remote MCP review: https://developers.openai.com/plugins/deploy/app-review
- Plugin guidelines: https://developers.openai.com/plugins/plugin-guidelines
- ChatGPT plugin availability: https://help.openai.com/en/articles/20001256-plugins-in-chatgpt

Verified constraints relevant to Cotra:

1. ChatGPT and Codex share a universal public plugin directory on supported surfaces.
2. A public remote MCP plugin requires a stable public HTTPS endpoint using Streamable HTTP; a private development tunnel is not sufficient for public submission.
3. User-specific/private data and write actions should use the MCP OAuth 2.1 authorization model.
4. Current OpenAI auth guidance requires protected-resource metadata, authorization-server metadata, PKCE S256, resource binding, and full token validation; OpenAI supports Client ID Metadata Documents (CIMD) and Dynamic Client Registration (DCR) client-identification paths.
5. OpenAI can present a managed mTLS client certificate to remote MCP servers; mTLS authenticates the ChatGPT client and does not replace end-user OAuth.
6. Each public tool must be independently exposed rather than hidden behind a generic executor/discovery operation.
7. Public tools require accurate `readOnlyHint`, `destructiveHint`, and `openWorldHint`; annotations are not authorization and Cotra must continue to enforce local policy.
8. Public submission requires publisher/organization verification and current submission permissions; domain verification and review apply to remote MCP publication.
9. The public plugin directory is available across ChatGPT plans, but whether a specific plugin/capability is installable or usable depends on plan, workspace, role, region, and surface. Cotra must not promise universal ChatGPT availability before actual publication/account verification.
10. Local plugin/package capabilities can differ from web capabilities; installing a package on web does not execute local hooks or local server processes on the user's PC.

Cotra design consequence:

- ChatGPT web requires the public Cotra relay edge and provider OAuth/pairing.
- Local ChatGPT/Codex surfaces that can run local MCP can use the local transport path and do not need the public relay.
- The production plugin must use a stable reviewed tool catalog and must not expose a raw shell/generic proxy surface.

## 2. Anthropic Claude

Official references:

- Remote custom connectors: https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp
- Local MCP / Desktop Extensions: https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop
- Desktop vs web connectors: https://support.claude.com/en/articles/11725091-when-to-use-desktop-and-web-connectors

Verified constraints relevant to Cotra:

1. Remote custom MCP connectors are available across Claude Free, Pro, Max, Team, and Enterprise; Free currently has a one-custom-connector limit.
2. Remote connector traffic originates from Anthropic infrastructure even when the user is running Claude Desktop. Therefore a remote connector needs a publicly reachable MCP endpoint.
3. Claude Desktop local MCP/desktop-extension support is a separate local mechanism and can use a local server on the user's machine.
4. Local desktop extensions are appropriate for local files/processes/OS access; remote connectors are appropriate when the connection must work across web/mobile/desktop surfaces.

Cotra design consequence:

- Claude Desktop/Code should prefer local stdio/Desktop Extension mode for privacy and zero hosted dependency.
- Claude web/everywhere mode uses the same Cotra public relay edge used by other hosted clients, with provider-neutral pairing and policy.
- A remote Claude connection must never be treated as local merely because the user launched it from Claude Desktop.

## 3. Mistral Vibe Code / Work

Official references:

- Vibe Code MCP servers: https://docs.mistral.ai/vibe/code/cli/mcp-servers
- Vibe Work MCP connectors: https://docs.mistral.ai/vibe/work/connectors/mcp-connectors
- Vibe Code install/setup: https://docs.mistral.ai/vibe/code/cli/install-setup

Verified constraints relevant to Cotra:

1. Vibe Code supports `stdio`, `http`, and `streamable-http` MCP transports.
2. Vibe Code currently does not support MCP servers requiring OAuth authentication; static credentials/API-key style headers are supported for HTTP mode.
3. Vibe Code can run with Mistral-hosted access, compatible provider keys, or fully offline local models.
4. Vibe Work custom MCP connectors accept custom MCP-compatible server URLs and require public HTTPS with a valid certificate; Streamable HTTP is the current standard.
5. On Free, Pro, and Student Vibe Work accounts, the account owner is the administrator by default for connector configuration.
6. Current Mistral custom MCP connectors do not yet support all MCP capabilities such as dynamic tool discovery, resources, and automatic prompt templates.

Cotra design consequence:

- Vibe Code should use local stdio by default, avoiding remote OAuth incompatibility.
- Vibe Work uses the public relay endpoint.
- Cotra's core remote workflow must not depend on MCP resources, dynamic tool discovery, or prompt templates.

## 4. Generic MCP compatibility

Official references:

- MCP TypeScript SDK connect guide: https://ts.sdk.modelcontextprotocol.io/v2/clients/connect
- MCP Streamable HTTP transport: https://ts.sdk.modelcontextprotocol.io/v2/api/%40modelcontextprotocol/node/streamableHttp.html
- MCP deployment/DNS-rebinding guidance: https://py.sdk.modelcontextprotocol.io/run/deploy/

Verified design requirements:

1. Local clients commonly launch MCP servers over stdio.
2. Remote/web clients use Streamable HTTP.
3. Loopback HTTP servers need Host/Origin validation and DNS-rebinding protection.
4. MCP protocol revisions now span legacy handshake-era sessions and the modern 2026-07-28 era; Cotra should use the SDK's negotiated behavior rather than inventing a parallel session protocol.
5. Session IDs, when present, are transport/session state and are not Cotra user identity or local authorization.

## 5. Reference zero-cost relay target

Official references:

- Cloudflare Durable Objects: https://developers.cloudflare.com/durable-objects/
- Durable Objects pricing: https://developers.cloudflare.com/durable-objects/platform/pricing/
- Workers pricing: https://developers.cloudflare.com/workers/platform/pricing/
- WebSocket hibernation: https://developers.cloudflare.com/durable-objects/examples/websocket-hibernation-server/

Verified facts relevant to the reference deployment:

1. SQLite-backed Durable Objects are currently available on the Workers Free plan.
2. The free plan has hard usage limits. If a free-tier limit is exceeded, operations fail rather than becoming an entitlement to unlimited free infrastructure.
3. Durable Objects can coordinate WebSocket clients; the hibernation API is the recommended design for reducing active-duration cost where applicable.
4. The free plan is suitable as a reference/community deployment target but cannot support an honest claim of unlimited free global relay capacity.

Cotra design consequence:

- the relay must be portable and self-hostable;
- community hosting is optional convenience, not a local product dependency;
- no automatic paid overflow;
- explicit typed failure on relay quota/unavailability;
- local stdio/loopback use remains functional when the community relay is unavailable.

## 6. Compatibility target matrix

| Client/surface | Preferred Cotra transport | Public relay required | Auth model | Target status |
| --- | --- | --- | --- | --- |
| Claude Desktop | stdio / Desktop Extension | No | local | P14 |
| Claude Code | stdio | No | local | P14 |
| Codex local/CLI | stdio or supported local plugin MCP config | No | local | P14 |
| Mistral Vibe Code | stdio | No | local | P14 |
| Generic local MCP | stdio | No | local | P14 |
| Generic local HTTP MCP | loopback Streamable HTTP | No | protected loopback token | P14 |
| ChatGPT web public plugin | HTTPS Streamable HTTP | Yes | OAuth 2.1 + paired device; OpenAI client identification defense in depth | P15/P17 |
| Claude web remote connector | HTTPS MCP | Yes | relay account/pairing auth compatible with Claude connector behavior | P15/P17 |
| Mistral Vibe Work | HTTPS Streamable HTTP | Yes | remote connector-compatible auth + paired device | P15/P17 |
| Hosted AI client using a self-hosted Cotra relay | HTTPS Streamable HTTP | Yes | standards-based remote auth | P15 |

## 7. Claims that are explicitly not yet proven

This research snapshot does not claim:

- that the Cotra public ChatGPT plugin will be approved by OpenAI;
- that an approved plugin will be visible to every ChatGPT plan/region/surface;
- that Claude/Mistral provider UI behavior will remain unchanged at v0.2 release time;
- that a free community relay can support unlimited users;
- that a shared relay provides provider-to-device end-to-end encryption;
- that local Desktop Commander replacement parity exists before P16 qualification.

Re-verify provider requirements at each provider integration grain and immediately before public submission/release claims.
