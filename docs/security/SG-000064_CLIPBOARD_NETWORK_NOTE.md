# SG-000064 Clipboard and Bounded Network Exposure Note

Status: IMPLEMENTATION FOR COTRA-P16
SpecGrain: SG-000064
Base: `2463c02cf875b736ecbda913ffccf7a09807cee3`
Date: 2026-10-02
Code: `apps/cotra-mcp/src/clipboard_network.ts`,
`apps/cotra-mcp/src/oauth_authorization.ts` (`LOCAL_ONLY_TOOL_NAMES`),
`crates/cotrad/src/clipboard.rs` (pre-approval refusal), unchanged closed
kernel paths `crates/cotra-provider-clipboard` (SG-000038, SG-000039) and
`crates/cotra-provider-network` with `crates/cotrad/src/git_fetch.rs`
(SG-000040).

## 1. New MCP tools

| Tool | Kernel shape | Approval | Bounds |
| --- | --- | --- | --- |
| `clipboard_read` | `clipboard/read` | SOFT, bound to workspace, policy revision, and the clipboard sequence observed before approval | one Unicode-text sample, 64 KiB |
| `clipboard_write` | `clipboard/write` | SOFT, bound to the text digest and byte length | one placement, 64 KiB |
| `web_fetch` | `network/fetch` | SOFT, bound to host, port, path digest, address-set digest, body bound, and redirect bound | one HTTPS GET, 1 MiB body, 5 same-origin redirects |

Each tool forwards exactly one closed kernel shape. No kernel shape,
policy rule, or approval class is added.

## 2. Live qualification

Each shape was qualified against its native implementation on real
Windows 11 before exposure:

- Clipboard: `sg000064_live_clipboard_round_trip_is_sequence_bound_and_secret_safe`
  places benign marker text through the Win32 clipboard, reads it back
  bound to the observed sequence, proves a stale sequence fails closed as
  `TARGET_STALE`, and proves secret-bearing text is refused with the
  clipboard left unchanged. Windows advanced the clipboard sequence by three
  after a placement (clipboard history and synthesized formats); the
  dispatch path is unaffected because a read binds the sequence observed
  immediately before its own approval and revalidates it afterwards.
- Network: `live_public_https_fetch_is_address_bound_and_bounded` (run with
  `--ignored` because it needs internet) resolves a public destination with
  the system resolver, performs one WinHTTP GET with proxies, cookies,
  authentication, keep-alive, and automatic redirects disabled, verifies
  the connected peer is in the approved address set, and returns status 200
  within the body bound. `private_loopback_and_non_https_destinations_are_refused_before_any_request`
  proves non-HTTPS schemes, loopback, private, link-local metadata, IPv6
  loopback, `localhost`, and userinfo URLs are refused before any request.

## 3. Preserved denials

- No clipboard surveillance: subscribe, poll, monitor, history, and watch
  shapes stay denied; each approval authorizes one read or one write.
- Write is not paste: placement synthesizes no input, keystrokes, focus, or
  paste, and no MCP tool combines placement with desktop input.
- Secrets: clipboard reads refuse secret-bearing content; writes refuse it
  before the approval prompt (new in SG-000064, so the human is never asked
  to approve a placement that can never happen) and again after approval.
- Network: HTTPS GET only, public addresses only, address set bound across
  approval and every redirect hop, same-origin redirects only, no proxy, no
  caller method, headers, or body, and no generic socket, tunnel, or proxy
  shape. The MCP schema accepts only `https://` URLs of at most 2048
  characters; the kernel revalidates everything.

## 4. Result projection

The kernel returns the fetched body as a byte array. The MCP tool projects
it to `text` when it is valid UTF-8 and otherwise omits it, reporting
`body_encoding: "binary-omitted"` with the length and digest. The
projection only removes data; it never adds authority.

## 5. Remote reach

All three tools are local-only (`LOCAL_ONLY_TOOL_NAMES`): absent from the
OAuth scope matrix, denied as unmapped by the relay edge and the device
uplink, and absent from cotrad's remote-session scope table. A remote
clipboard read would hand a remote principal whatever the user last
copied, and a device-side fetch would make the user's machine an egress
point for a remote principal that can fetch public URLs itself. A later
governed grain may map them under a locally enabled profile.

## 6. Evidence

- Real Windows 11: the live clipboard and network tests above.
- cotrad: `refused_write_text_never_reaches_the_approval_prompt` and the
  closed SG-000038, SG-000039, and SG-000040 dispatch tests.
- Contract tests: `apps/cotra-mcp/src/clipboard.test.ts`, `network.test.ts`
  (only `clipboard_network.ts` forwards these shapes; no socket, HTTP
  client, method, header, or proxy primitive in the MCP source; URL schema;
  body projection), `oauth-authorization.test.ts`, `parity-inventory.test.ts`,
  `surface.test.ts`, `transport-contract.test.ts`, and the release
  qualification catalog pin the 31-tool surface and the local-only decision.
