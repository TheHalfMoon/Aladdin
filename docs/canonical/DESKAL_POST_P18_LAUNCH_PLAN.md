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

SG-000087 is the sole active grain. It establishes the website foundation,
brand assets, responsive marketing surfaces, developer MCP connection example,
and independent web CI build.

The first grain does not authorize production hosting, DNS, analytics,
authentication, forms, payments, installer replacement, or any runtime
authority change.

## Successor work

After SG-000087 closes canonically, any remaining launch-completeness work must
be shaped as a new SpecGrain from the live frontier. Candidate work may include
documentation entry-point integration, verified release/download presentation,
or zero-cost hosting, but no successor is pre-authorized by this plan.
