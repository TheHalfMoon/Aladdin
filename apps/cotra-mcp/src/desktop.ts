import type { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import { KernelClient } from "./kernel.js";
import { projectKernelResult } from "./result.js";

/**
 * SG-000063 read-only structured desktop observation. Exactly two live
 * native shapes are forwarded: visible top-level window listing in the
 * caller's interactive Windows session and a bounded UI Automation
 * control-view tree for one listed window. Observation never actuates,
 * focuses, captures, or injects input. Protected Cotra surfaces, other
 * sessions, and Cotra's own windows are never observed, password values
 * are never read, and observation fails closed without an interactive
 * desktop. No actuation, screenshot, visual, coordinate, or input shape is
 * exposed because the native adapter does not implement them.
 */
const MAX_TREE_DEPTH = 8;
const MAX_TREE_NODES = 256;
const WINDOW_ID = /^uia-win-[0-9a-f]{16}$/;

export const DESKTOP_KERNEL_SHAPES = [
  ["uia.window", "list"],
  ["uia.tree", "observe"]
] as const;

export function desktopWindowListSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: z.string().min(1).default(defaultWorkspace)
  });
}

export function desktopWindowTreeSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: z.string().min(1).default(defaultWorkspace),
    window_id: z.string().regex(WINDOW_ID),
    window_generation: z.number().int().min(0).max(Number.MAX_SAFE_INTEGER),
    max_depth: z.number().int().min(0).max(MAX_TREE_DEPTH).default(MAX_TREE_DEPTH),
    max_nodes: z.number().int().min(1).max(MAX_TREE_NODES).default(MAX_TREE_NODES)
  });
}

export function registerDesktopTools(server: McpServer, kernel: KernelClient, defaultWorkspace: string): void {
  server.registerTool(
    "desktop_window_list",
    {
      description:
        "List visible top-level windows of other applications in the current interactive Windows session (at most 64), with typed window and process identities. Protected Cotra surfaces are omitted and counted. This tool does not focus, click, type, or otherwise change the desktop.",
      inputSchema: desktopWindowListSchema(defaultWorkspace)
    },
    async ({ workspace_id }) =>
      projectKernelResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "uia.window",
          operation: "list",
          arguments: {}
        })
      )
  );

  server.registerTool(
    "desktop_window_tree",
    {
      description:
        "Read the bounded UI Automation control tree (depth at most 8, at most 256 elements) of one window returned by desktop_window_list, bound to its window_generation. Password values are never read. This tool does not focus, click, type, or otherwise change the desktop.",
      inputSchema: desktopWindowTreeSchema(defaultWorkspace)
    },
    async ({ workspace_id, window_id, window_generation, max_depth, max_nodes }) =>
      projectKernelResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "uia.tree",
          operation: "observe",
          arguments: {
            window_id,
            expected_window_generation: window_generation,
            max_depth,
            max_nodes
          }
        })
      )
  );
}
