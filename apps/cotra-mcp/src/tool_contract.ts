import type { OAuthScope } from "./oauth_authorization.js";

/**
 * SG-000065 authoritative tool metadata contract.
 *
 * Every canonical MCP tool has exactly one entry. Registration,
 * annotations, profile-filtered discovery, the OAuth scope matrix, and the
 * local-only list are all derived from this record, and contract tests pin
 * it against the tool sources, the cotrad remote-session scope table, and
 * the parity inventory. Discovery filtering is never authorization: cotrad,
 * the remote-session lease, and the OAuth ceilings still enforce every call.
 *
 * Annotation semantics:
 * - `readOnlyHint`: the tool never changes local state.
 * - `destructiveHint`: the tool may delete or overwrite existing user data
 *   (file content, clipboard content, or arbitrary effects of a registered
 *   executable). Purely additive or index-only changes are not destructive.
 * - `idempotentHint`: repeating the call with the same arguments has no
 *   further effect.
 * - `openWorldHint`: the tool reaches beyond the local machine (network
 *   peers) or runs a registered executable whose effects are not modelled.
 */

export type ToolSurfaceProfile = "core" | "desktop_structured";

export type EffectClass =
  | "read"
  | "preview"
  | "write"
  | "destructive"
  | "execute"
  | "network"
  | "desktop_observe"
  | "clipboard";

export type ApprovalClass = "none" | "SOFT" | "STRONG";

export interface ToolAnnotations {
  readonly title: string;
  readonly readOnlyHint: boolean;
  readonly destructiveHint: boolean;
  readonly idempotentHint: boolean;
  readonly openWorldHint: boolean;
}

export interface ToolContractEntry {
  readonly capability: string;
  readonly operation: string;
  readonly effect: EffectClass;
  readonly approval: ApprovalClass;
  /** Remote OAuth scopes, or "local_only" for tools no remote path may call. */
  readonly remote: readonly OAuthScope[] | "local_only";
  readonly outputBound: string;
  readonly annotations: ToolAnnotations;
}

const read = (title: string): ToolAnnotations => ({
  title,
  readOnlyHint: true,
  destructiveHint: false,
  idempotentHint: true,
  openWorldHint: false
});

const change = (
  title: string,
  hints: { destructive: boolean; idempotent: boolean; openWorld?: boolean }
): ToolAnnotations => ({
  title,
  readOnlyHint: false,
  destructiveHint: hints.destructive,
  idempotentHint: hints.idempotent,
  openWorldHint: hints.openWorld ?? false
});

const READ: readonly OAuthScope[] = ["cotra.read"];
const WRITE: readonly OAuthScope[] = ["cotra.write"];
const EXECUTE: readonly OAuthScope[] = ["cotra.execute"];

export const TOOL_CONTRACT: Readonly<Record<string, ToolContractEntry>> = {
  system_status: {
    capability: "system.status", operation: "get", effect: "read", approval: "none", remote: READ,
    outputBound: "fixed-size daemon status object", annotations: read("Cotra status")
  },
  workspace_get: {
    capability: "workspace.get", operation: "get", effect: "read", approval: "none", remote: READ,
    outputBound: "fixed-size workspace metadata", annotations: read("Get workspace")
  },
  fs_stat: {
    capability: "fs.stat", operation: "stat", effect: "read", approval: "none", remote: READ,
    outputBound: "fixed-size metadata for one path", annotations: read("File metadata")
  },
  fs_list: {
    capability: "fs.list", operation: "list", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 5000 entries of one directory, truncation reported", annotations: read("List directory")
  },
  fs_read: {
    capability: "fs.read", operation: "read", effect: "read", approval: "none", remote: READ,
    outputBound: "one UTF-8 file of at most 1 MiB", annotations: read("Read file")
  },
  fs_search: {
    capability: "fs.search", operation: "search", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 200 matches over at most 2000 files of at most 512 KiB", annotations: read("Search files")
  },
  fs_read_range: {
    capability: "fs.read_range", operation: "read", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 2000 lines and 1 MiB of one UTF-8 file", annotations: read("Read line range")
  },
  fs_find: {
    capability: "fs.find", operation: "find", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 500 results, depth 16, 20000 entries visited", annotations: read("Find files")
  },
  git_status: {
    capability: "git.status", operation: "status", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 2 MiB of git output", annotations: read("Git status")
  },
  git_diff: {
    capability: "git.diff", operation: "diff", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 2 MiB of git output", annotations: read("Git diff")
  },
  git_log: {
    capability: "git.log", operation: "log", effect: "read", approval: "none", remote: READ,
    outputBound: "at most 100 commits within 2 MiB of git output", annotations: read("Git log")
  },
  fs_write_preview: {
    capability: "fs.write", operation: "preview", effect: "preview", approval: "none", remote: WRITE,
    outputBound: "digests and sizes for one file of at most 2 MiB", annotations: read("Preview file write")
  },
  git_fetch_preview: {
    capability: "git.fetch.preview", operation: "preview", effect: "preview", approval: "none", remote: WRITE,
    outputBound: "fixed-size local fetch plan", annotations: read("Preview git fetch")
  },
  git_push_preview: {
    capability: "git.push.preview", operation: "preview", effect: "preview", approval: "none", remote: WRITE,
    outputBound: "fixed-size local push plan", annotations: read("Preview git push")
  },
  fs_write: {
    capability: "fs.write", operation: "write", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only; writes at most 2 MiB", annotations: change("Write file", { destructive: true, idempotent: true })
  },
  fs_mkdir: {
    capability: "fs.mkdir", operation: "mkdir", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only", annotations: change("Create directory", { destructive: false, idempotent: false })
  },
  fs_move: {
    capability: "fs.move", operation: "move", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only", annotations: change("Move or rename", { destructive: false, idempotent: false })
  },
  fs_edit: {
    capability: "fs.edit", operation: "edit", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only; files of at most 2 MiB", annotations: change("Edit file", { destructive: true, idempotent: false })
  },
  fs_remove: {
    capability: "fs.remove", operation: "remove", effect: "destructive", approval: "STRONG", remote: WRITE,
    outputBound: "evidence only", annotations: change("Remove file or empty directory", { destructive: true, idempotent: false })
  },
  git_branch_create: {
    capability: "git.branch.create", operation: "create", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only", annotations: change("Create git branch", { destructive: false, idempotent: false })
  },
  git_stage: {
    capability: "git.stage", operation: "stage", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only; at most 128 paths", annotations: change("Git stage", { destructive: false, idempotent: true })
  },
  git_unstage: {
    capability: "git.unstage", operation: "unstage", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only; at most 128 paths", annotations: change("Git unstage", { destructive: false, idempotent: true })
  },
  git_commit: {
    capability: "git.commit", operation: "commit", effect: "write", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only; message at most 8 KiB", annotations: change("Git commit", { destructive: false, idempotent: false })
  },
  git_fetch: {
    capability: "git.fetch", operation: "fetch", effect: "network", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only", annotations: change("Git fetch", { destructive: false, idempotent: true, openWorld: true })
  },
  git_push: {
    capability: "git.push", operation: "push", effect: "network", approval: "SOFT", remote: WRITE,
    outputBound: "evidence only", annotations: change("Git push", { destructive: false, idempotent: true, openWorld: true })
  },
  process_spawn: {
    capability: "process.spawn", operation: "spawn", effect: "execute", approval: "SOFT", remote: EXECUTE,
    outputBound: "requested stdout of at most 16 MiB and stderr of at most 4 MiB",
    annotations: change("Run registered program", { destructive: true, idempotent: false, openWorld: true })
  },
  desktop_window_list: {
    capability: "uia.window", operation: "list", effect: "desktop_observe", approval: "none", remote: "local_only",
    outputBound: "at most 64 windows within 64 KiB", annotations: read("List desktop windows")
  },
  desktop_window_tree: {
    capability: "uia.tree", operation: "observe", effect: "desktop_observe", approval: "none", remote: "local_only",
    outputBound: "depth 8 and at most 256 elements within 64 KiB", annotations: read("Read window UI tree")
  },
  clipboard_read: {
    capability: "clipboard", operation: "read", effect: "clipboard", approval: "SOFT", remote: "local_only",
    outputBound: "one text sample of at most 64 KiB", annotations: read("Read clipboard")
  },
  clipboard_write: {
    capability: "clipboard", operation: "write", effect: "clipboard", approval: "SOFT", remote: "local_only",
    outputBound: "evidence only; places at most 64 KiB",
    annotations: change("Write clipboard", { destructive: true, idempotent: true })
  },
  web_fetch: {
    capability: "network", operation: "fetch", effect: "network", approval: "SOFT", remote: "local_only",
    outputBound: "one response body of at most 1 MiB",
    annotations: { title: "Fetch public HTTPS URL", readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: true }
  }
};

/** Tools any remote path may map: every entry with remote OAuth scopes. */
export const REMOTE_TOOL_NAMES: readonly string[] = Object.keys(TOOL_CONTRACT).filter(
  (name) => TOOL_CONTRACT[name]?.remote !== "local_only"
);

/** Tools no remote path may call. */
export const LOCAL_ONLY_TOOL_NAMES: readonly string[] = Object.keys(TOOL_CONTRACT).filter(
  (name) => TOOL_CONTRACT[name]?.remote === "local_only"
);

/**
 * Exact profile membership. `core` is the remote ceiling; local transports
 * serve `desktop_structured`, which adds the local-only tools. The
 * `developer` and `coordinate_fallback` profiles from the plan are not
 * mapped and deny.
 */
export const SURFACE_PROFILES: Readonly<Record<ToolSurfaceProfile, readonly string[]>> = {
  core: REMOTE_TOOL_NAMES,
  desktop_structured: Object.keys(TOOL_CONTRACT)
};

export function profileTools(profile: string): ReadonlySet<string> {
  if (!Object.hasOwn(SURFACE_PROFILES, profile)) {
    throw new Error(`tool surface profile is not mapped: ${profile}`);
  }
  return new Set(SURFACE_PROFILES[profile as ToolSurfaceProfile]);
}

/**
 * Provider-specific differences. In v0.2 every known remote provider shares
 * the `core` ceiling and no provider gains or loses a tool relative to
 * another; local transports serve `desktop_structured` regardless of the
 * configured provider kind, which is metadata and never proof of identity.
 * A remote connection from an unknown provider kind maps to no profile.
 */
export const REMOTE_PROVIDER_PROFILES: Readonly<Record<string, ToolSurfaceProfile>> = {
  generic: "core",
  openai: "core",
  anthropic: "core",
  mistral: "core",
  codex: "core"
};

export function remoteProfileForProvider(providerKind: string): ToolSurfaceProfile | null {
  return Object.hasOwn(REMOTE_PROVIDER_PROFILES, providerKind) ? (REMOTE_PROVIDER_PROFILES[providerKind] ?? null) : null;
}
