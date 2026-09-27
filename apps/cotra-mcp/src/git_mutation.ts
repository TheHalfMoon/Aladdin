import type { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import { KernelClient, type KernelResponse } from "./kernel.js";

const EXPECTED_HEAD = /^[0-9a-f]{40}$/;
const APPROVAL_TIMEOUT_MS = 10 * 60_000;

const workspaceId = (defaultWorkspace: string) =>
  z.string().min(1).default(defaultWorkspace);

const repositoryPath = z.string().min(1).max(4096).default(".");
const expectedHead = z.string().regex(EXPECTED_HEAD);
const mutationPath = z.string().min(1).max(4096);

export function gitBranchCreateSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: repositoryPath,
    expected_head: expectedHead,
    branch: z.string().min(1).max(255)
  });
}

export function gitStageSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: repositoryPath,
    expected_head: expectedHead,
    paths: z.array(mutationPath).min(1).max(128)
  });
}

export function gitUnstageSchema(defaultWorkspace: string) {
  return gitStageSchema(defaultWorkspace);
}

export function gitCommitSchema(defaultWorkspace: string) {
  return z.object({
    workspace_id: workspaceId(defaultWorkspace),
    path: repositoryPath,
    expected_head: expectedHead,
    message: z.string().min(1).max(8 * 1024)
  });
}

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

async function callMutation(
  kernel: KernelClient,
  input: {
    workspaceId: string;
    capability: string;
    operation: string;
    target: string;
    arguments: Record<string, unknown>;
  }
) {
  return asToolResult(
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

export function registerGitMutationTools(
  server: McpServer,
  kernel: KernelClient,
  defaultWorkspace: string
): void {
  server.registerTool(
    "git_branch_create",
    {
      description:
        "Create and switch to one new local Git branch rooted at an exact expected HEAD after fresh local Cotra approval. Existing branches and network Git operations are not exposed.",
      inputSchema: gitBranchCreateSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, expected_head, branch }) =>
      callMutation(kernel, {
        workspaceId: workspace_id,
        capability: "git.branch.create",
        operation: "create",
        target: path,
        arguments: { expected_head, branch }
      })
  );

  server.registerTool(
    "git_stage",
    {
      description:
        "Stage only the listed literal repository-relative files after fresh local Cotra approval and exact HEAD/state revalidation.",
      inputSchema: gitStageSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, expected_head, paths }) =>
      callMutation(kernel, {
        workspaceId: workspace_id,
        capability: "git.stage",
        operation: "stage",
        target: path,
        arguments: { expected_head, paths }
      })
  );

  server.registerTool(
    "git_unstage",
    {
      description:
        "Unstage only the listed literal repository-relative files after fresh local Cotra approval and exact HEAD/index-state revalidation.",
      inputSchema: gitUnstageSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, expected_head, paths }) =>
      callMutation(kernel, {
        workspaceId: workspace_id,
        capability: "git.unstage",
        operation: "unstage",
        target: path,
        arguments: { expected_head, paths }
      })
  );

  server.registerTool(
    "git_commit",
    {
      description:
        "Create exactly one non-amending unsigned local Git commit from the exact approved staged state. Requires repository-local author identity and fresh local Cotra approval.",
      inputSchema: gitCommitSchema(defaultWorkspace)
    },
    async ({ workspace_id, path, expected_head, message }) =>
      callMutation(kernel, {
        workspaceId: workspace_id,
        capability: "git.commit",
        operation: "commit",
        target: path,
        arguments: { expected_head, message }
      })
  );
}
