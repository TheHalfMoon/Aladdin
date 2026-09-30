# SG-000038 - Bounded clipboard read security note

Status: IMPLEMENTATION CANDIDATE
Program: COTRA-P11
Grain: SG-000038

## Purpose

SG-000038 establishes the first narrow COTRA-P11 clipboard grain on top
of the closed COTRA-P10 vision registry, the closed COTRA-P09
structured-UIA registry, the SG-000018 replay-resistant foundation,
SG-000019 class enforcement, SG-000020 trust and revoke records, and
the closed COTRA-P08 registry: a single explicit bounded
clipboard-read shape returning Unicode text only, with a hard size
bound, independent content-type verification, deterministic
secret-pattern denial, clipboard-sequence binding, fresh SOFT approval
digest binding, one-shot per-approval reads, and bounded secret-free
evidence.

This grain adds no clipboard write, no subscription, no polling, no
monitoring, no history collection, and no standing clipboard session.
Clipboard write remains successor work.

## Server-derived content remains the core control

The read shape accepts no caller arguments: format, size, sequence,
and content are server-derived, and any caller-supplied field is an
explicit widening denial in the policy layer. Only Unicode text is
returned; bitmap, file-drop, audio, shell-object, HTML/RTF objects,
and every other binary or object format is denied with no hidden
transfer. Empty content fails closed as stale. Oversized content
beyond 65536 bytes fails closed with no silent truncation. A locked
or otherwise unreadable clipboard fails closed as unavailable.

## Deterministic secret denial

Clipboard text matching a documented credential, token, key, seed, or
Cotra-protected family is denied before any return path and never
enters results, evidence, prompts, logs, or MCP responses. Denial
messages never quote the matched bytes. Detection is deterministic
pattern matching, honestly documented as imperfect: novel
exfiltration shapes outside the documented families are a stated
limitation, and the guarantee is the fail-closed direction (deny on
match, never leak on match), not perfect recall.

## Sequence-bound approval

The clipboard sequence number is observed immediately before approval
without moving clipboard bytes and is bound into the approval digest
alongside the workspace, the policy revision, the fixed Unicode-text
format bound, and the fixed size bound. Content itself cannot be
pre-bound without reading it, which would itself be a second
clipboard access; the sequence is the narrowest state binding that
avoids that. Dispatch revalidates the same sequence immediately after
approval, so clipboard content that changes in between fails closed
as stale. Approval is one-shot with nonce and expiry, so each
approval authorizes at most one sample.

## No surveillance

There is no polling loop, no subscription, no change listener, no
background monitor, no history collection, and no persistent
clipboard handle in the provider, policy, or dispatch layer. The
clipboard write, subscribe, poll, monitor, history, and watch shapes
are explicit denials mapped to the STRONG gate, which has no execution
authority for them. No MCP clipboard tool exists, and the agent cannot
reach reads through its own tool surface.

## Protected Cotra content and secrets

Clipboard text carrying Cotra approval, trust, emergency-revoke, or
secret markers is denied like any other secret family. No password,
credential, Windows Hello, cookie, token, session, private-key, or
seed material enters prompts beyond the bounded approval summary
(which carries format, size bound, and sequence only), and records,
history, logs, MCP responses, snapshots, and evidence packets carry
only format, sequence, size, digest, workspace, and policy revision,
never raw content beyond the approved bounded result itself.

## Windows qualification and honest limits

Live clipboard samples require an interactive Windows session, so
headless contexts report clipboard access as unavailable instead of
fabricating content. The native adapter queries the real clipboard
sequence number and reads real Unicode text through the Win32
clipboard APIs on Windows, and deterministic read, bound, denial,
secret, sequence, digest, and evidence behavior is proven through the
injected fake adapter on every platform. These limits are recorded
honestly and must not be read as interactive clipboard evidence
beyond what the native gating test genuinely proves.

## Retained behavior

All P10 vision behavior, all P09 structured-UIA behavior, SG-000018
one-shot, expiry, digest, and history behavior, SG-000019 SOFT and
STRONG class enforcement, SG-000020 trust and revoke behavior, and
the closed P08 registry behavior are retained unchanged. The
existing `%SystemRoot%\System32\whoami.exe` spawn boundary is
unchanged. No MCP clipboard tool exists, and the agent cannot reach
clipboard capability through its own tool or input surface.
Clipboard write and destination-scoped networking remain successor
work.
