# Security Policy

## Reporting a vulnerability

Report suspected vulnerabilities privately through GitHub security advisories for this repository: https://github.com/TheHalfMoon/Deskal/security/advisories/new

Do not open a public issue for a vulnerability. Include the affected version or commit, the component (`qdrald`, the MCP server, the relay, or the installer), reproduction steps, and the impact you observed.

## Scope

Deskal's security boundary is currently implemented by the compatibility daemon `qdrald`: workspace trust, approvals, the remote-session lease, executable registration, and protected state. Reports that a provider, relay, tool description, or MCP client can obtain authority that `qdrald` did not grant are in scope and are treated as high severity.

The `qdrald` identifier is retained as a compatibility contract during the Deskal product rename; it does not represent the current product brand.

## Supported versions

Security fixes are made on the `main` branch and shipped in the next release.
