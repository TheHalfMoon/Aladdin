# SG-000061 Filesystem Mutation Completion Note

Status: IMPLEMENTATION FOR COTRA-P16
SpecGrain: SG-000061
Base: `fb4203f9ef1638e32cdf7c9afdb3f9e49b082222`
Date: 2026-10-02
Code: `crates/cotra-provider-fs/src/mutation.rs`,
`crates/cotra-policy/src/sg000040.rs` (`is_fs_mutation_shape`,
`validate_fs_mutation`), `crates/cotrad/src/fs_mutation.rs`,
`apps/cotra-mcp/src/fs_mutation.ts`.

## 1. New MCP tools

| Tool | Shape | Approval | Bounds |
| --- | --- | --- | --- |
| `fs_read_range` | `fs.read_range/read` | none (read) | 1-2000 lines, 1 MiB, 4096 chars per line, UTF-8 only |
| `fs_find` | `fs.find/find` | none (read) | name glob `*`/`?`, no separators, 500 results, depth 16, 20000 entries visited |
| `fs_mkdir` | `fs.mkdir/mkdir` | SOFT | one directory or its missing parents (at most 32 levels); existing targets refused |
| `fs_move` | `fs.move/move` | SOFT, identity-bound | same workspace, never overwrites, no move into itself |
| `fs_remove` | `fs.remove/remove` | STRONG (destructive), identity-bound | one regular file or one empty directory; no recursion |
| `fs_edit` | `fs.edit/edit` | SOFT, digest-bound | exact search-and-replace, 1-100 replacements that must equal the occurrence count, 64 KiB needles, 2 MiB files |

The legacy generic `fs.delete/delete` shape is unchanged and stays denied;
`fs.remove` is the reviewed single-entry removal.

## 2. Path identity and containment

- Targets are workspace-relative and validated by policy; `fs.move` also
  validates its destination; the workspace root itself is never a mutation
  target.
- Every operation resolves the final Windows path (handle-based
  `GetFinalPathNameByHandleW`) and requires it to stay inside the trusted
  workspace root, so junctions or links pointing outside, including into
  protected Cotra state, fail with `PATH_ESCAPE`.
- Links, junctions, and any reparse point are never moved or removed
  (`CAPABILITY_DENIED`) and never traversed by `fs_find`.
- `fs_mkdir` creates one component at a time from the nearest existing
  ancestor and verifies each created directory's final path; a created
  component that resolves outside is removed and the call fails.
- `fs_move` on Windows uses `MoveFileExW` with no flags: never replaces an
  existing destination and never copies across volumes. The destination is
  built from the resolved parent's final path.

## 3. Identity binding and postconditions

- `fs_move` and `fs_remove` observe the entry identity (kind, size or entry
  count, SHA-256 for files) before approval, bind it into the approval
  digest, and re-verify it after approval (`TARGET_STALE` on change).
- `fs_edit` requires the caller's expected SHA-256, counts exact
  occurrences, binds current and new digests and the count into the
  approval, and writes through the existing digest-checked writer.
- Postconditions: created directories resolve inside the workspace; moved
  entries exist only at the destination with unchanged file content;
  removed entries no longer exist; edited files hash to the approved digest.
  Failures report `POSTCONDITION_FAILED`.

## 4. Remote and scope behavior

`fs_read_range` and `fs_find` require `cotra.read`; the four mutations
require `cotra.write` in the OAuth matrix and in the cotrad lease scope
table. Remote calls additionally need an active lease and the same
approvals; `fs_remove` therefore always needs a human at the computer.

## 5. Evidence

- Provider tests (7): bounded UTF-8 ranges, bounded name search without
  links, workspace-only mkdir, non-overwriting identity-bound moves (no move
  into itself, no escape), identity-bound non-recursive removal, digest and
  count bound edits, and a real Windows junction to an outside directory
  that is never followed, moved, removed, or used for mkdir.
- cotrad tests (3): denied approvals change nothing, removal requires STRONG
  presence (denied, unavailable, and SOFT-only brokers remove nothing), and
  approved mkdir, move, edit, ranged read, and find work end to end while
  the generic `fs.delete` shape is not handled.
- Catalog: 26 tools pinned in the surface, transport, OAuth matrix, release
  qualification, Inspector, Desktop Extension manifest, and parity
  inventory, which now records all six filesystem workflows as exposed.
