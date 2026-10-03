# SG-000017 — Bounded Git HTTPS push security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P06
Grain: SG-000017

## Purpose

SG-000017 introduces the second destination-scoped Git network capability: an
approved `git.push` that moves exactly one validated local source ref to one
validated remote destination ref on one workspace-bound canonical HTTPS
destination after fresh local approval, expected remote-ref protection,
non-fast-forward rejection, and resulting-ref verification.

The push path carries a protected credential-reference architecture. Raw
secrets never appear in request arguments, tool output, logs, audit, command
argv, persisted Git config, unrelated child environments, or CI output.
Ambient Git credential-manager authority is denied rather than inherited.

`git.fetch.preview` and approved `git.fetch` remain unchanged. Generic
network, shell, push-bypass, and ambient-credential authority are not
introduced.

## Destination policy boundary

Push destinations come from workspace-bound policy, never from caller URLs.
Each push destination binds a stable policy id, a canonical HTTPS URL, and
exactly one allowed credential reference id.

A destination is usable only when it is canonical HTTPS with a dotted DNS
hostname, implicit or explicit port 443, no userinfo, no query, no fragment,
and no alternate Git transport syntax. SSH, `git://`, `file://`, `ext`,
remote helpers, SCP-style syntax, HTTP without TLS, literal IP hostnames, and
duplicate or ambiguous policy ids are rejected.

The caller supplies short source and destination branch names. Both are
restricted to validated `refs/heads/<branch>` form. Full refs, refspec
separators, force markers, wildcards, and delete shapes are rejected before
any approval or network use.

## Resolver and address pinning

Resolution passes through the same testable resolver boundary as fetch.
Classification rejects loopback, private, link-local, multicast, unspecified,
documentation, special-use, and other non-public IPv4 and IPv6 results. One
validated public address is pinned into the actual Git HTTPS request with
`http.curloptResolve`, so a later unvalidated DNS result cannot silently widen
authority. Post-approval re-resolution and policy revalidation fail closed on
destination drift.

## Approval and stale protection

The local-only read-only preview performs no DNS resolution, no network
request, no approval request, no secret access, and no mutation. It returns
the exact current HEAD, the source-ref object id, deterministic source and
destination refs, and the current tracking prior (`ABSENT` or 40-hex).

Every push requires fresh local approval bound to workspace, repository
identity, policy revision, destination policy id, canonical URL, hostname,
port, pinned address, source ref, destination ref, expected HEAD, expected
prior tracking state, and credential-reference identifier without raw secret
material. The approval digest is versioned separately from fetch.

After approval and before network use, Qdral revalidates the preview state,
re-resolves and re-pins the destination, rechecks repository-local transport
configuration, and revalidates the credential binding. Any material drift
fails closed. The actual remote ref is then read with the hardened transport
and compared against the approved expected prior; a changed remote fails
closed before any push bytes are sent.

## Transport hardening

The hardened push uses HTTPS only, disables redirects, keeps TLS verification
enabled, inherits no proxy, cookies, extra headers, authorization material,
credential helpers, askpass, or interactive prompts outside the protected
credential-reference path, and rejects repository-local URL rewrites,
credential-helper overrides, or equivalent transport configuration. It pushes
exactly one explicit `source:destination` refspec with no force, delete, tags,
mirror, prune, submodule recursion, upstream assignment, or custom receive
pack. Repository-local hooks are neutralized with an empty hooks directory.

Non-fast-forward results fail closed as stale remote state. Transport failures
are typed without secret material.

## Credential-reference architecture

Authentication uses opaque credential reference ids, never raw secret values
in requests. The `anonymous` reference selects the credentialless path. Any
other reference must equal the destination-bound reference and is resolved
post-approval through a narrow resolver that reads only the single
`QDRAL_GIT_CREDENTIAL_<REFERENCE>` environment variable for the approved
reference. No other environment, helper, manager, or URL-embedded credential
is consulted.

The resolved secret is staged only in a restricted temporary askpass helper
that answers the Git username prompt with a fixed non-secret identity and the
password prompt from an isolated secret file. The secret never enters process
argv, persisted Git config, logs, audit, evidence, or unrelated child
environments; child processes receive an allowlisted environment plus the
single askpass path. Transport error text is scrubbed for secret material
before it is reported. The helper directory is removed after the operation on
both success and failure paths.

## Postconditions and evidence

After a push, Qdral re-reads the remote destination ref with the same
hardened transport and requires it to equal the pushed source object id. It
then records the Qdral-owned tracking ref and proves that checked-out HEAD,
current branch, index, and worktree material state are unchanged. Evidence
carries the credential reference id with an explicit redaction marker and no
secret value.

A no-cost real HTTPS qualification exercises the hardened transport against
the explicitly pinned canonical Qdral source. Anonymous ls-remote proves the
DNS, pinning, TLS, and HTTP wiring; the anonymous push attempt fails closed
as a typed transport denial without mutating local or remote state. A
successful authenticated HTTPS push has no no-cost destination in this
environment and is explicitly recorded as UNPROVEN rather than mocked.

## Explicitly denied

Denied in this grain: force push, force-with-lease, branch deletion, wildcard
or negative refspecs, tag or notes pushes, mirror or all-ref pushes, arbitrary
Git subcommands or argv, caller-supplied remote URLs, raw credential or token
arguments, URL-embedded credentials, ambient credential-manager authority,
credential helpers and askpass outside the protected path, interactive
prompts, HTTP without TLS, SSH or other transports, redirect following, proxy
inheritance, non-public destination addresses, submodule or tag side effects,
checkout or worktree mutation through push, general `network.fetch`, raw
shell, `cmd.exe`, PowerShell authority, additional `process.spawn` targets,
browser automation, UI automation, elevation, and approval bypass or reuse.
