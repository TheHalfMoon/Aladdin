# Getting Help with Orcel

## Self-service resources

Start with the [README](README.md) for platform requirements, installation, configuration, and the MCP client examples. For a failing installation or tunnel configuration, run `orcel doctor` and review its documented results before reporting an issue.

- [Install, configure, and MCP clients](README.md#install)
- [Known product limitations](README.md#current-limitations)
- [Architecture](docs/canonical/ARCHITECTURE_AND_DELIVERY_PLAN.md)
- [Security and threat model](docs/security/THREAT_MODEL.md)
- [Releases and verified downloads](https://github.com/TheHalfMoon/Orcel/releases)
- [Brand/identity downloads](https://thehalfmoon.github.io/Orcel/brand.html)

## Ask for help

Use [GitHub Issues](https://github.com/TheHalfMoon/Orcel/issues/new/choose) for reproducible bugs and feature requests. Include your Orcel version or commit, Windows build, a sanitized `orcel doctor` summary, steps to reproduce, expected vs actual behavior, and relevant logs after redaction.

This repository does not promise a public support SLA, managed hosting, or individual troubleshooting sessions. Community support depends on contributor availability.

## Security and privacy

**Do not attach** tunnel keys, personal files, bearer tokens, audit secrets, device identifiers, or unredacted logs. Suspected vulnerabilities belong in [private GitHub security advisories](https://github.com/TheHalfMoon/Orcel/security/advisories/new), not public issues.
