# Orcel project stewardship

Orcel is developed publicly under the [Apache License 2.0](../../LICENSE). The repository's maintainers manage review, versioning, security triage, and release decisions. Public source availability does not mean that every proposed change is approved for distribution.

## How decisions are made

Contributors propose changes through issues and pull requests. Maintainers evaluate usefulness, security impact, compatibility, licensing, tests, and the project's current priorities. Normal PRs must pass the applicable CI and review gates; high-risk changes may require additional qualification.

The canonical engineering plan and governance evidence live in [docs/canonical](../canonical/) and [docs/governance](../governance/). This document describes community participation and **does not replace** their source-of-truth controls or grant new tool authority.

## What contributors can expect

- A clear issue/PR template and documented testing instructions.
- Feedback where maintainer capacity allows; no guaranteed review or shipping date.
- Public acknowledgement through Git commit history and release notes when relevant.
- Open discussion of changes, with sensitive security details routed privately.

## Project boundaries

Orcel is a local-first computer access layer, **not** a general unrestricted remote-execution platform. Contributions must preserve explicit workspace trust, bounded operations, local approvals, and fail-closed behavior. No project materials should imply that using a client, plugin, or contributor role grants broader runtime authority.

## License and provenance

Code in this repository is distributed under Apache-2.0 except where expressly noted. Upstream dependencies, copied components, design research, and bundled assets must retain required notices and rights. See the [dependency and license review](../research/DEPENDENCY_LICENSE_REVIEW.md). Do not rewrite past approvals, signatures, or evidence records.
