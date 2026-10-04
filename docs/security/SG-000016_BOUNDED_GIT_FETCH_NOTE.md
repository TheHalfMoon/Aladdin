# SG-000016 — Bounded Git HTTPS fetch security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P06
Grain: SG-000016

## Purpose

SG-000016 introduces the first destination-scoped Git network capability. It is
deliberately narrower than generic HTTP, socket, or Git network authority.

The only newly reachable network capabilities are:

- `git.fetch.preview` — local-only and read-only stale-protection material with
  no DNS resolution, no network request, no approval request, and no mutation;
- `git.fetch` — fetch of exactly one validated `refs/heads/<branch>` source
  from one workspace-bound canonical HTTPS destination into the deterministic
  Qdral-owned `refs/remotes/qdral/<policy-id>/<branch>` ref after fresh local
  approval, public-address pinning, and post-approval revalidation.

`git.push` and force push remain hard-denied. No credential authority is added.

## Destination policy boundary

Fetch destinations come from workspace-bound policy, never from caller URLs.

A destination is usable only when it is canonical HTTPS with a dotted DNS
hostname, implicit or explicit port 443, no userinfo, no query, no fragment,
and no alternate Git transport syntax. SSH, `git://`, `file://`, `ext`,
remote helpers, SCP-style syntax, HTTP without TLS, literal IP hostnames, and
duplicate or ambiguous policy ids are rejected.

## Resolver and address pinning

Resolution passes through a testable resolver boundary. Classification rejects
loopback, private, link-local, multicast, unspecified, documentation,
special-use, and other non-public IPv4 and IPv6 results. One validated public
address is pinned into the actual Git HTTPS request with
`http.curloptResolve`, so a later unvalidated DNS result cannot silently widen
authority. Post-approval re-resolution and policy revalidation fail closed on
destination drift.

## Transport hardening

The hardened fetch uses HTTPS only, disables redirects, keeps TLS verification
enabled, inherits no proxy, cookies, extra headers, authorization material,
credential helpers, askpass, or interactive prompts, and rejects
repository-local URL rewrites or equivalent transport configuration. It fetches
without tags, prune, submodule recursion, or `FETCH_HEAD` mutation where
supported, and writes only the deterministic Qdral-owned destination ref.

## Approval and postconditions

Every fetch requires fresh local approval bound to workspace, repository
identity, policy revision, destination policy id, canonical URL, hostname,
port, pinned address, source ref, destination ref, expected HEAD, and expected
prior destination-ref state from preview. Material state is revalidated after
approval before network use.

Successful fetch evidence verifies the resulting destination-ref object id and
proves that checked-out HEAD, current branch, index, and worktree material
state remain unchanged.

## Explicitly denied

Denied in this grain: `git.push`, force push, raw or ambient credentials,
credential helpers and managers, askpass and interactive prompts, URL-embedded
credentials, caller-supplied URLs, arbitrary refspecs, general `network.fetch`,
raw shell, `cmd.exe`, PowerShell authority, additional `process.spawn` targets,
browser automation, UI automation, elevation, and approval bypass or reuse.
