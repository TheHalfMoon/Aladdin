# Security Policy

## Reporting a vulnerability

Report suspected vulnerabilities privately through GitHub security advisories for this repository: https://github.com/TheHalfMoon/Orcel/security/advisories/new

Do not open a public issue for a vulnerability. Include the affected version or commit, the component (`orceld`, the MCP server, the relay, or the installer), reproduction steps, and the impact you observed.

## Scope

Orcel's security boundary is currently implemented by the compatibility daemon `orceld`: workspace trust, approvals, the remote-session lease, executable registration, and protected state. Reports that a provider, relay, tool description, or MCP client can obtain authority that `orceld` did not grant are in scope and are treated as high severity.

The `orceld` identifier is retained as a compatibility contract during the Orcel product rename; it does not represent the current product brand.

## Supported versions

Security fixes are made on the `main` branch and shipped in the next release.
