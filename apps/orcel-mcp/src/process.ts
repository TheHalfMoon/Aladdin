import * as z from "zod/v4";

const MAX_TIMEOUT_MS = 30 * 60 * 1000;
const MAX_STDOUT_BYTES = 16 * 1024 * 1024;
const MAX_STDERR_BYTES = 4 * 1024 * 1024;

export const processSpawnInputSchema = z
  .object({
    workspace_id: z.string().min(1),
    executable: z.string().min(1).max(1024),
    argv: z.array(z.string().max(8192)).max(64).default([]),
    cwd: z.string().min(1).max(1024).default("."),
    timeout_ms: z.number().int().min(1000).max(MAX_TIMEOUT_MS).default(30_000),
    stdout_bytes: z.number().int().min(1).max(MAX_STDOUT_BYTES).default(2 * 1024 * 1024),
    stderr_bytes: z.number().int().min(1).max(MAX_STDERR_BYTES).default(256 * 1024),
    stdin_policy: z.literal("null").default("null"),
    network_class: z.literal("NONE").default("NONE")
  })
  .strict();

export type ProcessSpawnInput = z.infer<typeof processSpawnInputSchema>;
