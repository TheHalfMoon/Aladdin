# SG-000096 T02: Native Windows path/handle identity qualification

Status: test-only, unarmed; parent #264 and #271, program #260.

## Predecessor
Canonical main is da5dedfa15345f9559d0b9922cd8d0a478e060e1, merged from PR #276; post-main CI 37864887390 was observed SUCCESS 9/9 before this successor branch.

## Mechanism
Windows OpenOptionsExt is used to hold read-only file and directory handles without share-write or share-delete, with OPEN_REPARSE_POINT (and BACKUP_SEMANTICS for a directory). MetadataExt on each actual handle supplies volume serial and file index, while SHA-256 is streamed over the same open file handle with a finite 32-MiB test bound. Directory and executable identities are reopened and rechecked immediately. Lexical prefix metadata rejects observed reparse/junction components.

The disposable native tests cover correct file/volume/digest, wrong digest and file index, lexical path drift, wrong directory/executable kind, relative path, forbidden cwd digest, and edited bytes after the initial guard is released. No process is spawned.

## Remaining security gates
Ancestor prefix checks are not an atomic Windows namespace transaction. This test-only prototype does NOT demonstrate race-free CreateProcess image binding, a verified native executable, an installed-runtime Full User grant, real SOFT user action, actual path/cwd handle pinning through dispatch, revoked/expired shell lifecycle, T03 interactive I/O, T04 local MCP or T05 adversarial closeout. Production ordinary-user ShellProcess stays disabled. No remote/admin elevation, package/release route, donor import or paid compute is added. A Job Object is not a network or filesystem sandbox.

## Required external qualification
Native Windows tests, full provider regression, release-blocker sweep, Clippy -D warnings, fmt/whitespace, real Graft wiring with semantic limitations, SSH signed+DCO, genuine Jev and Alibaba OCR (manual excluded-file review), exact-head CI 9/9, normal merge and post-main CI 9/9; otherwise leave T02 OPEN.
