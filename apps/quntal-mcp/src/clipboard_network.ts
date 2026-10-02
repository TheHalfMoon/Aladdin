import type { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import { KernelClient } from "./kernel.js";
import { projectKernelResult } from "./result.js";

/**
 * SG-000064 bounded clipboard and destination-scoped network tools. Each
 * tool forwards exactly one closed kernel shape and every call requires
 * fresh local Quntal approval bound to its exact digest:
 *
 * - `clipboard_read`: one Unicode-text sample bound to the clipboard
 *   sequence observed before approval; secret-bearing, empty, oversized,
 *   or changed content fails closed. There is no monitoring, polling,
 *   history, or subscription.
 * - `clipboard_write`: one placement of caller text bound to its digest;
 *   secret-bearing or oversized text is refused. Placement never pastes or
 *   sends input.
 * - `web_fetch`: one HTTPS GET to a public destination bound to the
 *   approved address set, with same-origin redirects and a 1 MiB body
 *   bound. There are no other methods, headers, bodies, schemes, private
 *   addresses, proxies, or sockets.
 */
const APPROVAL_TIMEOUT_MS = 10 * 60_000;
const MAX_CLIPBOARD_BYTES = 65_536;
const MAX_URL_CHARS = 2048;

export const CLIPBOARD_NETWORK_KERNEL_SHAPES = [
  ["clipboard", "read"],
  ["clipboard", "write"],
  ["network", "fetch"]
] as const;

export function clipboardReadSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: z.string().min(1).default(defaultWorkspace)
  });
}

export function clipboardWriteSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: z.string().min(1).default(defaultWorkspace),
    text: z
      .string()
      .min(1)
      .refine((value) => Buffer.byteLength(value, "utf8") <= MAX_CLIPBOARD_BYTES, {
        message: `text must be at most ${String(MAX_CLIPBOARD_BYTES)} UTF-8 bytes`
      })
  });
}

export function webFetchSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: z.string().min(1).default(defaultWorkspace),
    url: z
      .string()
      .min(1)
      .max(MAX_URL_CHARS)
      .refine((value) => value.startsWith("https://"), { message: "url must use https://" })
  });
}

const utf8 = new TextDecoder("utf-8", { fatal: true });

/**
 * Project the kernel's raw byte array into UTF-8 text, or report a binary
 * body by length and digest only. The kernel already bounds the body.
 */
export function projectFetchBody(result: Record<string, unknown>): Record<string, unknown> {
  const { body, ...rest } = result;
  if (!Array.isArray(body) || !body.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255)) {
    return rest;
  }
  const bytes = Uint8Array.from(body as number[]);
  try {
    return { ...rest, body_encoding: "utf-8", text: utf8.decode(bytes) };
  } catch {
    return { ...rest, body_encoding: "binary-omitted" };
  }
}

export function registerClipboardNetworkTools(server: McpServer, kernel: KernelClient, defaultWorkspace: string): void {
  server.registerTool(
    "clipboard_read",
    {
      description:
        "Read the current clipboard text once (at most 64 KiB) after fresh local Quntal approval. Secret-bearing content is refused, and content that changes after approval fails closed. There is no clipboard monitoring.",
      inputSchema: clipboardReadSchema(defaultWorkspace)
    },
    async ({ workspace_id }) =>
      projectKernelResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "clipboard",
          operation: "read",
          arguments: {},
          timeoutMs: APPROVAL_TIMEOUT_MS
        })
      )
  );

  server.registerTool(
    "clipboard_write",
    {
      description:
        "Place text on the clipboard once (at most 64 KiB) after fresh local Quntal approval. Secret-bearing text is refused. This never pastes or types anything.",
      inputSchema: clipboardWriteSchema(defaultWorkspace)
    },
    async ({ workspace_id, text }) =>
      projectKernelResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "clipboard",
          operation: "write",
          arguments: { text },
          timeoutMs: APPROVAL_TIMEOUT_MS
        })
      )
  );

  server.registerTool(
    "web_fetch",
    {
      description:
        "Fetch one public HTTPS URL with GET after fresh local Quntal approval of the destination. Private, loopback, and link-local addresses, other schemes, cross-origin redirects, and bodies over 1 MiB are refused. Returns UTF-8 text, or only the length and digest of a binary body.",
      inputSchema: webFetchSchema(defaultWorkspace)
    },
    async ({ workspace_id, url }) => {
      const response = await kernel.call({
        workspaceId: workspace_id,
        capability: "network",
        operation: "fetch",
        arguments: { url },
        timeoutMs: APPROVAL_TIMEOUT_MS
      });
      if (response.ok && response.result !== null && typeof response.result === "object" && !Array.isArray(response.result)) {
        return projectKernelResult({ ...response, result: projectFetchBody(response.result as Record<string, unknown>) });
      }
      return projectKernelResult(response);
    }
  );
}
