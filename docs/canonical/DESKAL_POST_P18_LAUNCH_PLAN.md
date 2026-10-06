# Deskal Post-P18 Launch Completeness Plan

Status: ACTIVE PROGRAM PLAN
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

## Successor work

With SG-000088 closed canonically, any remaining launch-completeness work must
be shaped as a new SpecGrain from the live frontier. No custom domain, dynamic
service, release publication, installer change, or compatibility migration is
authorized by the current plan.
