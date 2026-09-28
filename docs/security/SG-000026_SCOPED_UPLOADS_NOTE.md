# SG-000026 - Scoped bounded browser uploads security note

Status: IMPLEMENTATION CANDIDATE
Program: COTRA-P08
Grain: SG-000026

## Purpose

SG-000026 establishes scoped bounded browser uploads on top of the SG-000021
isolated automation profile, the SG-000022 typed page lifecycle with origin
binding, the SG-000023 typed node observation, the SG-000024 structured
actuation, and the SG-000025 approved download root: an upload capability that
may read exactly one file, and only a file Cotra itself downloaded and recorded
under the approved workspace download root, and may stage it only to one known
enabled typed file-input node, bound to exact page identity, page generation,
document generation, origin, node identity, expected role, expected input type,
expected node state, workspace trust revision, source artifact identity, content
digest, size, media type, policy revision, approval, and expected state, while
keeping page byte transfer, form submission, archive extraction,
downloaded-file execution, personal-profile mode, debugging, scripting,
credential access, MCP browser tools, and network egress beyond destination
validation and DNS resolution absent.

No browser is launched or attached in this grain. No page byte transfer or form
submission is performed, and evidence explicitly reports
`page_transfer_performed` as `false` so no consumer can mistake an approved
upload record for a completed network transfer. Actual page rendering and
rendered-page form submission remain successor work.

## The source class is the core security control

No upload shape accepts a path, file, content, directory, recursive, glob, or
page-transfer field of any kind. Those fields are explicit `CapabilityDenied`
widening denials, so a caller can never name the file to read and can never
inline file content.

The only admissible source is a file already recorded by the SG-000025 download
registry inside the approved workspace download root, carrying a recorded
content digest, and re-verified against that record at upload time for canonical
filesystem identity, regular-file type, byte length, and SHA-256.

The consequence is that upload is strictly weaker than generic filesystem read.
Credential files, browser profile files, OS secret stores, user home files,
workspace-authored files, unrelated project files, and directories are
unreachable **by construction**, not merely by policy: there is no code path
from any request field to any path, and the only path that can be read is the
one a Cotra download record already names inside the approved root.

## Destination and target binding

The target must be one known node whose server-recorded role is `textbox` and
whose server-recorded input type is `file`, and whose server-recorded state is
`enabled`. Wrong role, wrong input type, disabled state, replaced file input,
stale node, wrong page, wrong origin, wrong generation, wrong workspace, and
policy drift all fail closed. The node identity is recomputed against the
current profile, page, generation, origin, index, and policy revision, so a
forged or replaced identity cannot be used.

A file input is explicitly denied as a `fill` target, so a path is never typed
into a file input. That denial is a required consequence of this grain: without
it, the newly observable file input would have widened SG-000024 value entry.

## Source confinement and re-verification

The source parent directory and the source file are canonicalized through real
filesystem identity and compared to the approved download root with a
separator-boundary comparison, so a Windows junction or a Unix symlink inside
the root cannot redirect the read. A removed artifact, a directory at the
recorded destination, an empty artifact, an oversize artifact, and an artifact
whose on-disk length or SHA-256 no longer matches the recorded download all fail
closed. The file is only ever read; it is never written, renamed, deleted, or
truncated, and no file content is returned to any caller.

## Trust binding

The workspace trust revision is bound into both the upload source identity and
the approval digest. An untrusted workspace, an emergency revoke, a trust
change, and any trust-revision change between preview and submit all fail
closed, so an upload can never complete against a revoked workspace.

## Approval, one-shot consumption, and bounds

Every upload requires a fresh SOFT approval whose digest under
`COTRA_BROWSER_UPLOAD_V1` binds workspace, policy revision, profile identity,
page identity, origin, page and document generation, node identity, expected
role, input type and state, trust revision, upload source identity, artifact
relative destination, artifact media type, artifact content digest, and artifact
size. Preview reports the same digest so the operator sees exactly what will be

## Evidence

Upload evidence is a fixed-shape packet carrying page identity, origin,
generations, node identity, role, input type, node state, trust revision, upload
and artifact identities, artifact relative destination, media type, content
digest, size, upload policy revision, state, and the approval reference. It
explicitly reports `page_transfer_performed`, `executed`, `opened`,
`extracted`, `cookies`, and `credentials` as `false`. It never carries file
content, file bytes, cookies, Authorization headers, passwords, tokens, session
secrets, or personal browser state, and it contains no path field of any kind.

## Denied authority

`browser.upload/upload`, `browser.upload/directory`,
`browser.upload/multiple`, `browser.upload/execute`, `browser.upload/open`,
`browser.upload/extract`, `browser.file/read`, `browser.fs/read`, and
`browser.directory/upload` are denied capability shapes. No file listing,
directory enumeration, or generic filesystem traversal shape is authorized
either, so `browser.file/list` and every other such shape is denied as an
unauthorized capability. Filling a file input is denied. Uploading a directory
or multiple files is denied. Archive extraction, downloaded-file execution, file
opening, form submission, and page byte transfer are absent. Package
installation and credential stores remain unreachable.

## Approval and trust retention

SG-000018 one-shot, expiry, digest, and history, SG-000019 STRONG enforcement,
SG-000020 trust and revoke, SG-000021 profile and destination, SG-000022 page
lifecycle and navigation, SG-000023 observation, SG-000024 actuation, and
SG-000025 download behavior are unchanged. The observation template gains one
structural enabled file-input entry so the typed file-input identity is
reachable, and the download registry record gains the verified content digest so
a later consumer can re-verify the exact bytes; records written before this
grain default to an empty digest and are therefore never eligible as an upload
source. Uploads map to the SOFT approval class and introduce no STRONG
execution authority. No MCP browser tool exists.

Process-spawn disclosure: the only process spawned anywhere in the repository
remains the `#[cfg(test)]` `cmd /c mklink /J` call that builds a junction so the
reparse-point escape denial can be proven. Its arguments are test-generated
temporary paths, and no capability, request field, or caller input reaches it.

## Tests

Deterministic unit and security tests prove upload source identity binding with
one-shot, expiry, forged destination, upload-policy drift, and foreign-profile
denial; artifact re-verification with mutated, removed, directory, empty, and
oversize denial; Windows junction and Unix symlink escape denial from the
approved download root; node gating with wrong role, wrong input type, disabled
state, replaced file input, wrong page, stale generation, and password-target
denial; workspace untrusted and trust-revision-drift denial; registry
append-only, one-shot, and bounded behavior; evidence secret-freedom and
`page_transfer_performed` false; digest drift across source, node, trust,
destination, media type, digest, size, and origin; and, at the dispatch layer,
preview without approval, submit requiring approval, one-shot replay denial,
mutated and removed artifact denial, unrecorded source denial, unauthorized
target denial, every path, file, content, directory, and page-transfer widening
field denial, stale page and origin denial, denial of unrelated credential-
shaped files, and unhandledness of every upload-adjacent denied shape.
