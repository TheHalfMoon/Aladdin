# Deskal Post-P18 Launch Completeness Plan

Status: EXITED PROGRAM PLAN
Program: DESKAL-P19
Date: 2026-10-06

## Purpose

DESKAL-P19 turns the technically qualified post-P18 repository into a coherent
public product surface without widening Deskal's runtime authority. The program
starts with the public website because the repository currently has no canonical
Deskal web application despite the product identity, security model, provider
distribution, and computer-use qualification already being present.

## Governing rules

- Deskal is the current product identity.
- Existing `qdral` compatibility identifiers remain unchanged unless a later
  breaking-migration grain explicitly authorizes them.
- Public copy must be evidence-bounded and must not imply remote self-approval,
  arbitrary execution, silent fallback, or cloud authority.
- Public web work must not add telemetry, tracking, hosted control, credentials,
  or mandatory paid infrastructure.
- Website dependencies are isolated under `apps/web` and do not enter the
  privileged runtime dependency graph.
- Normal merge commits, exact-head qualification, Jev, Alibaba Open Code Review,
  manual review of excluded files, and post-merge verification remain mandatory.

## First grain

SG-000087 is CLOSED canonical. It established the website foundation,
brand assets, responsive marketing surfaces, developer MCP connection example,
and independent web CI build.

The first grain does not authorize production hosting, DNS, analytics,
authentication, forms, payments, installer replacement, or any runtime
authority change.

## Second grain

SG-000088 is CLOSED canonical. It published the already-qualified static
website through zero-cost GitHub Pages and connected the website and repository
to verified public launch entrypoints.

SG-000088 may use GitHub Pages workflow deployment with the minimum required
repository permissions and GitHub-provided short-lived Pages/OIDC credentials.
It does not authorize custom DNS, analytics, tracking, cookies, authentication,
forms, payments, hosted APIs, databases, release publication, alternate artifact
distribution, or any Deskal runtime authority change.

## Third grain

SG-000089 is CLOSED canonical. It made the canonical GitHub repository a
coherent Deskal launch entrypoint by pinning and applying an evidence-bounded
repository description, the canonical GitHub Pages homepage, and a bounded
public topic allowlist.

SG-000089 may mutate only the repository description, homepage, and topics. It
does not authorize release or tag changes, custom DNS, Pages configuration,
repository visibility, default branch, merge policy, branch protection, Actions
settings, secrets, environments, collaborators, installer changes, compatibility
migration, or any Deskal runtime authority change.

## Fourth grain

SG-000090 is CLOSED canonical. It hardened the existing public website UX,
accessibility, metadata, responsive navigation, and static performance while
preserving the exact approved Deskal brand geometry and evidence-bounded copy.

SG-000090 may modify only `apps/web` presentation source, website metadata,
website regression tests, and the minimum canonical governance records required
for the grain. It may not add new package dependencies, remote runtime assets,
analytics, telemetry, tracking, forms, authentication, payments, custom DNS,
Pages configuration changes, release/tag mutation, installer changes,
compatibility migration, or any Deskal runtime authority.

## Fifth grain

SG-000091 is CLOSED canonical. It completed the DESKAL-P19 exit and added no
new product capability. The final launch-completeness evidence matrix across
SG-000087 through SG-000090 proves every required row and classifies every
intentionally absent surface OUT_OF_SCOPE.

SG-000091 added only exit documentation, regression tests, and the minimum
canonical governance records required to close the program. It did not
authorize custom DNS, dynamic services, release or tag mutation, installer or
package changes, compatibility migration, repository-policy changes, or any
Deskal runtime authority.

## Successor work

DESKAL-P19 is exited. It authorizes no further work. DESKAL-P20 is separately
activated by SG-000092 from live canonical truth and is governed by
`docs/canonical/DESKAL_P20_UNIVERSAL_AGENT_RUNTIME_PLAN.md`. This exited plan
continues to authorize no custom domain, dynamic service, release/tag mutation,
installer change, compatibility migration, or runtime-authority change.
