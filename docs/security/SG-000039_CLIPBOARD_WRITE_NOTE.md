# SG-000039 - Bounded clipboard write security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P11
Grain: SG-000039

## Purpose

SG-000039 establishes the second narrow QUNTAL-P11 clipboard grain on
top of the closed SG-000038 bounded clipboard-read registry, the
closed QUNTAL-P10 vision registry, the closed QUNTAL-P09
structured-UIA registry, the SG-000018 replay-resistant foundation,
SG-000019 class enforcement, SG-000020 trust and revoke records, and
the closed QUNTAL-P08 registry: a single explicit bounded
clipboard-write shape placing Unicode text only, with a hard size
bound, caller-text validation, deterministic secret-pattern denial,
fresh SOFT approval digest binding over the text digest, one-shot
per-approval writes, sequence-advancing bounded secret-free evidence,
and explicit separation proving placement never authorizes paste,
input, keyboard, or desktop authority.

This grain adds no subscription, no watcher, no polling, no
monitoring, no history collection, and no standing clipboard session.
Destination-scoped networking remains successor work.

## Server-derived binding with caller text

The write shape accepts exactly one explicit `text` argument holding
Unicode text within the 65536-byte bound; format and sequence are
server-derived, and every other caller-supplied field is an explicit
widening denial in the policy layer. Empty text fails closed as
malformed. Oversized text fails closed with no silent truncation.
Only Unicode text is placed; binary, object, file-drop, image, and
shell formats are denied with no hidden transfer. A locked or
otherwise unwritable clipboard fails closed as unavailable.

## Deterministic secret denial

Caller text matching a documented credential, token, key, seed, or
Quntal-protected family is denied in the policy layer and again in the
provider before any placement, and never enters results, evidence,
prompts, logs, or MCP responses. Denial messages carry digests and
bounds only, never the matched bytes. The audit log records
capability metadata without request arguments, so denied text cannot
leak through audit either. Detection is deterministic pattern
matching, honestly documented as imperfect: the guarantee is the
fail-closed direction, not perfect recall.

## Digest-bound approval

The approval digest binds the workspace, the policy revision, the
content digest of the exact caller text, the byte length, the fixed
Unicode-text format bound, and the fixed size bound. Unlike reads,
write content is known before approval, so the payload itself is
bound; the provider still revalidates bounds and secrets after
approval. The approval summary carries digests and bounds only, never
caller text. Approval is one-shot with nonce and expiry, so each
approval authorizes at most one placement.

## Write != paste

Placement touches only the clipboard through `SetClipboardData` with
`CF_UNICODETEXT`. The adapter contract forbids synthesizing input,
keystrokes, paste, Ctrl+V, focus changes, mouse activity, SendInput,
and UI Automation of any kind, and no dispatch path exists from a
write outcome to any input shape. Tests prove write evidence carries
no input-capable material and that UIA leases and interruption epochs
are untouched by the write path. A paste, if ever wanted, would be a
separate explicit input operation under its own approval, which does
not exist in this grain.

## No surveillance

There is no polling loop, no subscription, no change listener, no
watcher creation, no background monitor, no history collection, and
no persistent clipboard handle in the provider, policy, or dispatch
layer. The clipboard subscribe, poll, monitor, history, and watch
shapes remain explicit denials mapped to the STRONG gate, which has
no execution authority for them. No MCP clipboard tool exists, and
the agent cannot reach writes through its own tool surface.

## Protected Quntal content and secrets

Write text carrying Quntal approval, trust, emergency-revoke, or
secret markers is denied like any other secret family. No password,
credential, Windows Hello, cookie, token, session, private-key, or
seed material is placed or retained. Prompts, records, history, logs,
MCP responses, snapshots, and evidence packets carry only format,
size, digests, sequence, workspace, and policy revision, never raw
content beyond the approved bounded request itself.

## Windows qualification and honest limits

Live clipboard placement requires an interactive Windows session, so
headless contexts report clipboard access as unavailable instead of
fabricating success. The native adapter places real Unicode text
through the Win32 clipboard APIs on Windows and returns the real
resulting sequence number, with global-memory ownership transferred
to the system only on success and freed on every failure path.
Deterministic write, bound, denial, secret, digest, separation, and
evidence behavior is proven through the injected fake adapter on
every platform, including a write-then-read round trip. These limits
are recorded honestly and must not be read as interactive clipboard
evidence beyond what the native gating tests genuinely prove.

## Retained behavior

All SG-000038 read behavior, all P10 vision behavior, all P09
structured-UIA behavior, SG-000018 one-shot, expiry, digest, and
history behavior, SG-000019 SOFT and STRONG class enforcement,
SG-000020 trust and revoke behavior, and the closed P08 registry
behavior are retained unchanged. The existing
`%SystemRoot%\System32\whoami.exe` spawn boundary is unchanged. No
MCP clipboard tool exists, and the agent cannot reach clipboard
capability through its own tool or input surface. Destination-scoped
networking remains successor work.
