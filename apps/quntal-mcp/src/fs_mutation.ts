import type { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import { KernelClient } from "./kernel.js";
import { projectKernelResult } from "./result.js";

/**
 * SG-000061 bounded filesystem tools. Every target is workspace-relative and
 * resolved by quntald with final path identity; links and reparse points are
 * never followed, moved, or removed. Mutations require fresh local Quntal
 * approval bound to the observed identity; removal requires the STRONG
 * destructive class. There is no recursive, wildcard, or overwriting
 * mutation.
 */
const APPROVAL_TIMEOUT_MS = 10 * 60_000;
const SHA256 = /^[0-9a-f]{64}$/;

const workspaceId = (defaultWorkspace: string) => z.string().min(1).default(defaultWorkspace);
const relativePath = z.string().min(1).max(4096);

export function fsReadRangeSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: relativePath,
    start_line: z.number().int().min(1).default(1),
    max_lines: z.number().int().min(1).max(2000).default(200)
  });
}

export function fsFindSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: relativePath.default("."),
    pattern: z.string().min(1).max(128),
    max_results: z.number().int().min(1).max(500).default(100),
    max_depth: z.number().int().min(0).max(16).default(8)
  });
}

export function fsMkdirSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: relativePath,
    parents: z.boolean().default(false)
  });
}

export function fsMoveSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: relativePath,
    to: relativePath
  });
}

export function fsRemoveSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: relativePath
  });
}

export function fsEditSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: relativePath,
    old: z.string().min(1).max(64 * 1024),
    new: z.string().max(64 * 1024),
    expected_sha256: z.string().regex(SHA256),
    replacements: z.number().int().min(1).max(100).default(1)
  });
}

async function call(
  kernel: KernelClient,
  input: { workspaceId: string; capability: string; operation: string; target: string; arguments: Record<string, unknown> }
) {
  return projectKernelResult(
    await kernel.call({
      workspaceId: input.workspaceId,
      capability: input.capability,
      operation: input.operation,
      target: input.target,
      arguments: input.arguments,
      timeoutMs: APPROVAL_TIMEOUT_MS
    })
  );
}

export function registerFsMutationTools(server: McpServer, kernel: KernelClient, defaultWorkspace: string): void {
  server.registerTool(
    "fs_read_range",
    {
      description: "Read a bounded line range (at most 2000 lines) of one UTF-8 text file inside a trusted workspace. This tool does not mutate the computer.",
      inputSchema: fsReadRangeSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, start_line, max_lines }) =>
      call(kernel, { workspaceId: workspace_id, capability: "fs.read_range", operation: "read", target: path, arguments: { start_line, max_lines } })
  );

  server.registerTool(
    "fs_find",
    {
      description: "Find files and directories by a name glob (* and ?) inside a trusted workspace with bounded depth and results. Links are not followed. This tool does not mutate the computer.",
      inputSchema: fsFindSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, pattern, max_results, max_depth }) =>
      call(kernel, { workspaceId: workspace_id, capability: "fs.find", operation: "find", target: path, arguments: { pattern, max_results, max_depth } })
  );

  server.registerTool(
    "fs_mkdir",
    {
      description: "Create one directory (optionally its missing parents) inside a trusted workspace after fresh local Quntal approval.",
      inputSchema: fsMkdirSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, parents }) =>
      call(kernel, { workspaceId: workspace_id, capability: "fs.mkdir", operation: "mkdir", target: path, arguments: { parents } })
  );

  server.registerTool(
    "fs_move",
    {
      description: "Move or rename one file or directory within the same trusted workspace after fresh local Quntal approval. Never overwrites an existing destination.",
      inputSchema: fsMoveSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, to }) =>
      call(kernel, { workspaceId: workspace_id, capability: "fs.move", operation: "move", target: path, arguments: { to } })
  );

  server.registerTool(
    "fs_remove",
    {
      description: "Remove one file or one empty directory inside a trusted workspace after STRONG local presence (Windows Hello). There is no recursive delete.",
      inputSchema: fsRemoveSchema(defaultWorkspace)
    },
    async ({ workspace_id, path }) =>
      call(kernel, { workspaceId: workspace_id, capability: "fs.remove", operation: "remove", target: path, arguments: {} })
  );

  server.registerTool(
    "fs_edit",
    {
      description: "Replace exact text in one UTF-8 file bound to its current SHA-256 (from fs_write_preview or a prior result), with the exact expected number of replacements, after fresh local Quntal approval.",
      inputSchema: fsEditSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, old, new: replacement, expected_sha256, replacements }) =>
      call(kernel, {
        workspaceId: workspace_id,
        capability: "fs.edit",
        operation: "edit",
        target: path,
        arguments: { old, new: replacement, expected_sha256, replacements }
      })
  );
}
