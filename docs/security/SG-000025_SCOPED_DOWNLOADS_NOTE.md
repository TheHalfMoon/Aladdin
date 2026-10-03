# SG-000025 - Scoped bounded browser downloads security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P08
Grain: SG-000025

## Purpose

SG-000025 establishes scoped browser downloads on top of the SG-000021 isolated
automation profile, the SG-000022 typed page lifecycle with origin binding, the
SG-000023 read-only typed node observation, and the SG-000024 structured
actuation: a bounded download capability bound to exact page identity, page
generation, document generation, origin, policy revision, a server-allocated
one-shot download source identity, the declared download identity, destination
policy, and the approved workspace download root, with canonical relative
destinations, path-escape and reparse-point denial, extension and content-type
allowlisting, declared and actual size agreement, deterministic source expiry,
and fresh SOFT approval, while keeping downloaded content unexecuted, unopened,
and unextracted, and while keeping scoped uploads, archive extraction,
personal-profile mode, debugging, scripting, credential access, MCP browser
tools, and network egress beyond destination validation and DNS resolution
absent.

No browser is launched or attached in this grain, and Qdral performs no network
transfer: the source URL is validated and pinned exactly like an SG-000022
navigation destination. Actual page rendering and page loading by a browser
engine remain absent and are successor work.

## Authority model

`browser.download/preview` and `browser.download/download` bind one known active
page on the isolated automation profile, in the caller workspace, at the current
policy revision, with an exact expected origin, page generation, and document
generation. Unknown, closed, foreign-workspace, foreign-profile, origin-drifted,
generation-drifted, document-replaced, and policy-drifted pages fail closed.

`browser.download/preview` requires no approval, mutates nothing, and writes
nothing. It returns a server-allocated one-shot download source identity bound
under `QDRAL_BROWSER_DOWNLOAD_SOURCE_V1` to workspace, policy revision, profile
identity, page identity, origin, page generation, document generation, source
origin, source URL digest, canonical relative destination, declared media type,
declared size, download policy revision, and issue time, plus a deterministic
expiry of 120 seconds.

`browser.download/download` consumes exactly one such source identity. A consumed
source never authorizes a second download, an unknown identity fails closed, and
an expired identity fails closed. The source identity is recomputed against the
server record and the current page, so forged identities, drifted destinations,
drifted origins, and policy drift all fail closed.

## Destination policy

The destination root is always the approved workspace root from policy
configuration. It is never caller-selected: `destination_root`,
`download_root`, `root`, `path`, `drive`, and `unc` are explicit
authority-widening denials, and so are `execute`, `open`, `extract`, `spawn`,
`shell`, `run`, `archive`, `output`, and `overwrite`.

The caller may propose only a canonical relative destination. Absolute paths,
drive prefixes, UNC paths, device paths, NT namespace paths, alternate data
streams, `..` and `.` components, empty and duplicate separators, trailing dots
and spaces, reserved Windows device names, characters illegal in Windows path
components, control characters, and over-length components or paths all fail
closed with `PathEscape` or an invalid-request code.

Containment is decided with canonical filesystem identity, not string prefixes.
The approved root and the destination's existing parent directory are both
canonicalized through the filesystem before comparison, and the separator
boundary is part of the comparison key. A junction or symlink inside the root
therefore cannot extend the approved root: the destination fails closed with
`PathEscape`. After the write, the file's own real path is re-canonicalized and
re-checked, and a file that does not resolve inside the root is deleted and the
download fails closed.

Process-spawn disclosure: the only process this grain spawns anywhere in the
repository is inside the Windows test that builds a directory junction with
`cmd /c mklink /J` so the reparse-point escape can be proven. That call exists
only under `#[cfg(test)]`; its arguments are two test-generated temporary
paths and no capability, request field, or caller input reaches it. It grants no
production authority, and no shell, PowerShell, or process-spawning capability is
reachable from any Qdral capability, MCP tool, or browser shape.

Downloads are create-only. The file is opened with create-new semantics, an
existing destination fails closed with `PostconditionFailed`, parent directories
are never created, and no existing workspace file is ever overwritten, renamed,
deleted, or truncated. After writing, the byte length and SHA-256 digest are read
back and compared with the approved payload, and a mismatch deletes the file and
fails closed.

## File type and execution policy

The extension allowlist is the authority. Only `.txt`, `.md`, `.csv`, `.json`,
`.png`, `.jpg`, `.jpeg`, `.gif`, and `.webp` are authorized. Executables,
scripts, installers, command files, archives, shortcuts, macro-capable
documents, extensionless names, and every unlisted extension fail closed with
`CapabilityDenied`.

Content is sniffed independently. PE, ELF, Mach-O, OLE compound, shell-script,
and ZIP signatures are classified as their own classes and are never in the
authorized allowlist, so an executable, script, installer, archive, or
macro-capable document cannot be smuggled past the extension allowlist by
declaring an inert extension. Text declarations require text content and image
declarations require the exact matching image class, so a filename, declared
type, and content mismatch fails closed. Evidence records the declared type, the
extension type, the sniffed type, and the consistency decision, so a type
mismatch is always visible in evidence.

Downloaded content is never executed, opened, extracted, or launched. No shell
execution, no PowerShell, no `cmd.exe`, no process spawning, and no file opening
are added. `browser.download/execute`, `browser.download/open`,
`browser.download/extract`, `browser.download/launch`, `browser.file/execute`,
`browser.file/open`, `browser.archive/extract`, and `browser.shell/run` are
denied capability shapes, as is every upload shape. Package-manager installation
of downloaded content remains EXECUTE plus WRITE plus NETWORK and is not
authorized here. Archive extraction is absent, and no archive can be written to
the workspace at all because archive extensions are denied.

## Size and resource bounds

A declared size must be between 1 byte and 8 MiB. The body is strict standard
base64 with an encoded-length bound, a padding-shape check, an alphabet check,
and a decoded-size bound, all applied before any byte reaches the filesystem.
Declared and actual size must agree; disagreement fails closed with
`PostconditionFailed`. At most 8 unconsumed download sources may be held per
workspace, so pending holds on protected state stay bounded. Filenames are
bounded to 128 bytes, path components to 128 bytes, and relative destinations to
512 bytes. Evidence is a fixed-shape packet with no caller-controlled text
volume.

## Origin binding

The source URL is parsed, re-resolved, and post-resolution address-checked with
the SG-000021 destination policy, and the resulting origin must equal the
authorized current page origin. A declared redirect chain is validated hop by
hop with the SG-000022 chain rules, which require every hop to share the exact
base origin; an unauthorized origin or any redirect widening fails closed with
`CapabilityDenied`. Private, loopback, link-local, and cloud metadata addresses
remain denied by the retained SSRF policy. SG-000022 navigation download-trigger
denial is retained unchanged: downloads are reachable only through the explicit

## Approval and evidence

Every download requires a fresh SOFT approval whose digest under
`QDRAL_BROWSER_DOWNLOAD_V1` binds workspace, policy revision, profile identity,
page identity, origin, page generation, document generation, source identity,
source origin, canonical relative destination, declared media type, declared
size, the actual content digest, and the download policy revision. Preview
reports the same digest with an empty content digest so the operator can see what
will be approved. Any material drift, including payload substitution, invalidates
the approval, and the SG-000018 one-shot consumption, expiry, and digest binding
are unchanged.

Download evidence is a fixed-shape packet carrying page identity, origin, source
origin, source URL digest, destination root identity, canonical relative
destination, declared filename, declared, extension, and sniffed media types,
type consistency, declared and actual sizes, content SHA-256, source and download
identities, state, and the approval reference. It records the platform origin
metadata of the download and explicitly reports `executed`, `opened`, and
`extracted` as false. Cookies, Authorization headers, passwords, tokens, session
secrets, and personal browser state never enter the evidence or the download
registry, and the raw source URL is never persisted: only the source origin and a
source URL digest are recorded, so a secret-bearing query string cannot leak into
protected state.

## Approval and trust retention

SG-000018 one-shot, expiry, digest, and history, SG-000019 STRONG enforcement,
SG-000020 trust and revoke, SG-000021 profile and destination, SG-000022 page
lifecycle and navigation, SG-000023 observation, and SG-000024 actuation behavior
are unchanged. Downloads map to the SOFT approval class and introduce no STRONG
execution authority. All P06 approval flows and predecessor regressions still pass
with digests unchanged outside download labeling. No MCP browser tool exists: the
existing MCP test that forbids browser capabilities on the MCP surface remains
intact.

## Tests

Deterministic unit and security tests prove canonical destination validation with
absolute, drive, UNC, device, NT namespace, alternate data stream, traversal,
reserved device name, trailing dot and space, illegal character, mixed separator,
and length denial; junction escape on Windows and symlink escape on Unix;
create-only writes with overwrite denial and read-back verification; extension
allowlisting with executable, script, installer, archive, shortcut, and
macro-capable denial; content sniffing with executable, script, archive, OLE,
PDF, and unknown-class denial; declared and actual size agreement; strict and
bounded base64 decoding; one-shot and expiring source identities with consumed,
expired, unknown, forged, and drifted denial; fresh SOFT approval with digest
binding including the content digest; destination, type, origin, and redirect
widening denial; caller-selected destination and execution widening-field denial;
stale page, generation, origin, and document denial; secret-free bounded
evidence; agent-forgery resistance through the existing MCP browser denial test;
and retained replay, expiry, digest, drift, and predecessor regressions.
