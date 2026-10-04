# Qdral Privacy Policy

Status: DRAFT FOR OWNER LEGAL REVIEW. This text describes what the Qdral software does, as implemented in this repository. It must be reviewed and published by the owner before it is used for a provider directory listing.

## Who operates what

Qdral is software that you run. The Qdral daemon and its local MCP server run on your own computer. A Qdral relay, which remote AI clients such as ChatGPT reach, runs on a host that you or your organization operate (self-hosting). The Qdral project does not operate a hosted relay and does not receive your data.

## Data the software processes

On your computer (the daemon and local server):

- the requests an AI client sends: tool name, workspace identifier, target path, and arguments;
- the content those tools read or write inside workspaces you trusted, returned to the AI client that asked;
- a local audit log of each request (time, request and session identifiers, workspace, capability, operation, target path, policy revision, and outcome), stored in your local Qdral data folder;
- local approval history, workspace trust records, executable registrations, and device keys, stored in protected local Qdral state.

On a relay you operate:

- registered OAuth clients (client identifier, client name, redirect addresses, allowed scopes, creation time);
- paired devices (device public keys and registration time) and connection records (an opaque principal, connection, device, and client identifier, provider kind, and scope ceiling);
- refresh-token families and revocation records, and the relay's token signing key;
- operational log lines that name the event and HTTP status only, without request content.

Tool requests and results pass through the relay in transit to and from your computer; the relay does not store their content.

## Purposes

Data is processed only to carry out the requests you or your AI client make, to enforce your local trust, approval, and lease decisions, to authenticate the connection, and to keep an audit record on your own computer.

## Recipients

- The AI provider you connected (for example OpenAI for ChatGPT) receives the tool results it requested, under that provider's own privacy terms.
- The operator of the relay host receives the relay data listed above. When you self-host, that is you.
- The Qdral project receives nothing. Qdral sends no telemetry and no analytics.

## Retention

- Local audit and approval records stay on your computer until you delete them or uninstall Qdral with `qdral uninstall --purge-data`.
- Relay records stay in the relay state directory until the client, device, or connection is revoked or the relay operator deletes the state; revoked token identifiers are kept until the token would have expired.
- Content returned to an AI provider is retained according to that provider's policy.

## Your controls

You choose which workspaces are trusted, approve every change on your computer, can end any remote session, can revoke a paired device or connection, and can stop or uninstall Qdral at any time.

## Contact

Questions: https://github.com/TheHalfMoon/Qdral/issues. Security reports: see `SECURITY.md`.
