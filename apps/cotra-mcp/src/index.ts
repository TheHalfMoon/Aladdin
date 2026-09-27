import { McpServer } from "@modelcontextprotocol/server";
import { serveStdio } from "@modelcontextprotocol/server/stdio";
import * as z from "zod/v4";
import { registerGitFetchTools } from "./git_fetch.js";
import { registerGitMutationTools } from "./git_mutation.js";
import { registerGitPushTools } from "./git_push.js";
import { KernelClient, type KernelResponse } from "./kernel.js";
import { processSpawnInputSchema } from "./process.js";

const DEFAULT_WORKSPACE = process.env.COTRA_DEFAULT_WORKSPACE ?? "default";
const clients = new Set<KernelClient>();

function asToolResult(response: KernelResponse) {
  if (!response.ok) {
    return {
      isError: true,
      content: [
        {
          type: "text" as const,
          text: JSON.stringify(
            {
              error: response.error ?? {
                code: "INTERNAL_ERROR",
                message: "cotrad returned an unspecified failure"
              }
            },
            null,
            2
          )
        }
      ]
    };
  }

  return {
    content: [
      {
        type: "text" as const,
        text: JSON.stringify(
          {
            result: response.result,
            evidence: response.evidence
          },
          null,
          2
        )
      }
    ]
  };
}

function createServer(): McpServer {
  const kernel = new KernelClient();
  clients.add(kernel);

  const server = new McpServer({
    name: "cotra",
    version: "0.1.0"
  });

  const workspaceSchema = z.object({
    workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE)
  });

  server.registerTool(
    "system_status",
    {
      description: "Read Cotra daemon status. This tool does not mutate the computer.",
      inputSchema: workspaceSchema
    },
    async ({ workspace_id }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "system.status",
          operation: "get"
        })
      )
  );

  server.registerTool(
    "workspace_get",
    {
      description: "Read the configured trusted workspace metadata.",
      inputSchema: workspaceSchema
    },
    async ({ workspace_id }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "workspace.get",
          operation: "get"
        })
      )
  );

  server.registerTool(
    "fs_stat",
    {
      description: "Read metadata for a relative path inside a trusted workspace.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1)
      })
    },
    async ({ workspace_id, path }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "fs.stat",
          operation: "stat",
          target: path
        })
      )
  );

  server.registerTool(
    "fs_list",
    {
      description: "List one directory inside a trusted workspace without following symlinks.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1).default(".")
      })
    },
    async ({ workspace_id, path }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "fs.list",
          operation: "list",
          target: path
        })
      )
  );

  server.registerTool(
    "fs_read",
    {
      description: "Read one bounded UTF-8 text file inside a trusted workspace.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1)
      })
    },
    async ({ workspace_id, path }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "fs.read",
          operation: "read",
          target: path
        })
      )
  );

  server.registerTool(
    "fs_search",
    {
      description:
        "Search bounded UTF-8 text files inside a trusted workspace. Symlinks are not followed.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1).default("."),
        query: z.string().min(1),
        max_results: z.number().int().min(1).max(200).default(50)
      })
    },
    async ({ workspace_id, path, query, max_results }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "fs.search",
          operation: "search",
          target: path,
          arguments: { query, max_results }
        })
      )
  );

  server.registerTool(
    "fs_write_preview",
    {
      description:
        "Preview an exact UTF-8 file write. This does not mutate the computer and returns the current/new SHA-256 values required for an approved write.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1),
        content: z.string().max(2 * 1024 * 1024)
      })
    },
    async ({ workspace_id, path, content }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "fs.write",
          operation: "preview",
          target: path,
          arguments: { content }
        })
      )
  );

  server.registerTool(
    "fs_write",
    {
      description:
        "Write exact UTF-8 content inside a trusted workspace after an independent local Cotra approval. Existing files require the current SHA-256 from fs_write_preview.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1),
        content: z.string().max(2 * 1024 * 1024),
        expected_current_sha256: z.string().regex(/^[0-9a-f]{64}$/).optional(),
        create_if_missing: z.boolean().default(false)
      })
    },
    async ({
      workspace_id,
      path,
      content,
      expected_current_sha256,
      create_if_missing
    }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "fs.write",
          operation: "write",
          target: path,
          arguments: {
            content,
            expected_current_sha256,
            create_if_missing
          },
          timeoutMs: 5 * 60_000
        })
      )
  );

  server.registerTool(
    "process_spawn",
    {
      description:
        "Execute one argv-only process after fresh local approval inside the selected trusted workspace. No shell string, caller environment, stdin payload, background mode, or process network authority is exposed.",
      inputSchema: processSpawnInputSchema
    },
    async ({
      workspace_id,
      executable,
      argv,
      cwd,
      timeout_ms,
      stdout_bytes,
      stderr_bytes,
      stdin_policy,
      network_class
    }) => {
      const processKernel = new KernelClient();
      clients.add(processKernel);
      try {
        return asToolResult(
          await processKernel.call({
            workspaceId: workspace_id,
            capability: "process.spawn",
            operation: "spawn",
            arguments: {
              executable,
              argv,
              cwd,
              timeout_ms,
              stdout_bytes,
              stderr_bytes,
              stdin_policy,
              network_class
            },
            timeoutMs: timeout_ms + 5 * 60_000
          })
        );
      } finally {
        clients.delete(processKernel);
        processKernel.close();
      }
    }
  );

  server.registerTool(
    "git_status",
    {
      description:
        "Read Git working-tree and branch status for a repository inside a trusted workspace. This tool does not mutate Git state.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1).default(".")
      })
    },
    async ({ workspace_id, path }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "git.status",
          operation: "status",
          target: path
        })
      )
  );

  server.registerTool(
    "git_diff",
    {
      description:
        "Read a bounded Git diff for a repository inside a trusted workspace. External diff and textconv are disabled.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1).default("."),
        staged: z.boolean().default(false)
      })
    },
    async ({ workspace_id, path, staged }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "git.diff",
          operation: "diff",
          target: path,
          arguments: { staged }
        })
      )
  );

  server.registerTool(
    "git_log",
    {
      description:
        "Read bounded Git commit history for a repository inside a trusted workspace.",
      inputSchema: z.object({
        workspace_id: z.string().min(1).default(DEFAULT_WORKSPACE),
        path: z.string().min(1).default("."),
        max_count: z.number().int().min(1).max(100).default(20)
      })
    },
    async ({ workspace_id, path, max_count }) =>
      asToolResult(
        await kernel.call({
          workspaceId: workspace_id,
          capability: "git.log",
          operation: "log",
          target: path,
          arguments: { max_count }
        })
      )
  );

  registerGitMutationTools(server, kernel, DEFAULT_WORKSPACE);
  registerGitFetchTools(server, kernel, DEFAULT_WORKSPACE);
  registerGitPushTools(server, kernel, DEFAULT_WORKSPACE);

  server.server.onclose = () => {
    kernel.close();
    clients.delete(kernel);
  };

  return server;
}

const handle = serveStdio(createServer);
process.stderr.write("[cotra-mcp] serving Cotra SG-000017 tools over stdio\n");

function shutdown(): void {
  for (const client of clients) {
    client.close();
  }
  clients.clear();
  void handle.close().finally(() => process.exit(0));
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
