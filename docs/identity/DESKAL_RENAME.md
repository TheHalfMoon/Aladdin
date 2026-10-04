# Deskal Product Rename Record

Status: ACTIVE PRODUCT IDENTITY
Date: 2026-10-04

## Decision

The maintained product identity is now **Deskal**.

Current product identity:

- Product: `Deskal`
- Root package: `deskal`
- Public plugin name: `deskal`
- Public display name: `Deskal`

The earlier product names `Qdral`, `Quntal`, and `Cotra` are superseded product identities and must not be presented as the current product name.

## Compatibility boundary

This rename changes the maintained product identity immediately without silently breaking already-installed clients, release consumers, or protocol contracts. The following identifiers remain compatibility identifiers until a separately qualified breaking migration changes them:

- CLI command: `qdral`
- Local daemon: `qdrald`
- MCP package: `@qdral/mcp`
- Relay package: `@qdral/relay`
- Environment-variable prefix: `QDRAL_`
- OAuth scope prefix: `qdral.`
- Existing project-owned source paths beginning with `qdral-`
- Existing local storage and install paths that contain `Qdral` or `qdral`
- MCP server compatibility name: `qdral`
- Repository compatibility slug: `TheHalfMoon/Qdral` until the repository owner renames it

Those compatibility identifiers do not define the current product brand. New user-facing product copy must use **Deskal**.

## Repository rename

The repository owner may rename the GitHub repository from `TheHalfMoon/Qdral` to `TheHalfMoon/Deskal`. Until that metadata change occurs, repository URLs in build metadata may continue to use the existing slug so that links remain valid.

## Historical provenance

Historical evidence remains immutable. Files that record earlier identities, exact SHAs, release artifacts, protocol schemas, signed evidence, or third-party provenance must not be rewritten merely to erase an old name. In particular, `QDRAL_RENAME.md` and `QUNTAL_RENAME.md` remain historical records.

## Migration rule

Do not perform a blind global replacement. A future migration of compatibility identifiers must be explicit, test-qualified, and coordinated across Cargo packages, npm workspaces, MCP client configuration, environment variables, OAuth scopes, release archives, local storage, installers, and external consumers.
