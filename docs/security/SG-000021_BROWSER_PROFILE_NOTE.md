# SG-000021 — Isolated browser profile with typed provider contract and origin binding security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P08
Grain: SG-000021

## Purpose

SG-000021 establishes the first QDRAL-P08 structured-browser foundation on
top of the SG-000018 replay-resistant foundation, SG-000019 STRONG
enforcement, and SG-000020 trust and revoke records: a dedicated isolated
Qdral automation browser profile with a typed provider contract and strict
origin binding, so destination policy, SSRF defenses, and redirect-widening
denial are proven before any navigation or DOM actuation authority exists.

No browser is launched or attached in this grain. The provider contract is
local policy only: profile status and origin-bound destination validation.
Navigation, DOM actuation, screenshots, downloads, uploads,
personal-profile mode, debugging, credential access, and network egress
remain absent.

## Profile isolation

The automation profile lives under Qdral protected local state
(`LOCALAPPDATA/Qdral/browser-profile` on Windows, with a
`QDRAL_BROWSER_STATE_DIR` override for tests), never inside a workspace and
never inside a personal browser directory. Creation writes a marker file
with fixed expected content; reopening verifies the marker and refuses
foreign storage fail-closed instead of reusing it.

Before any directory is created or touched, the requested root is checked
against the personal-profile recognizer: whole path components matching
known personal browser directories (`User Data`, `Chrome`, `Edge`,
`Firefox`, profiles, cookie and login store names) fail closed with
`CapabilityDenied`. UNC and device namespaces, relative paths, and parent
components are rejected. There is no request field that selects a profile
directory: `browser.profile/status` accepts an empty argument object, and
any `profile_root`, `root`, `path`, `argv`, `personal`, credential, or
scripting field is rejected. The agent cannot point automation at the
user's normal browser profile because no such input surface exists.

The profile carries fresh Qdral-owned storage only. No personal cookies,
passwords, sessions, extensions, or history are imported, and no secret,
cookie, or credential material enters prompts, records, history, logs, MCP
responses, or evidence packets. Profile status evidence binds the
workspace identity and the policy revision to the deterministic profile
identity.

## Typed provider contract

Only two typed operations exist: `browser.profile/status` and
`browser.destination/validate`. Every other browser-like shape
(`browser.navigate`, DOM verbs, downloads, uploads, personal-profile use,
launch, attach, debug, DevTools, CDP, scripting, external requests, and the
bare `devtools`, `cdp`, and `playwright` families) is denied at the policy
layer with `CapabilityDenied`, maps to the STRONG approval class so the
dispatch STRONG gate fails closed without SOFT downgrade, and is
unreachable at dispatch. Unknown operations and extra
authority-widening fields fail closed; `profile_root`, `argv`, `script`,
`cookies`, and `cdp` fields are denied as capability widening rather than
mere malformed input.

No MCP browser tool is registered in this grain. Even a forged kernel
request carrying browser capability reaches only the policy denial above,
so the agent cannot satisfy browser capability through MCP tools,
synthetic input, UI automation, coordinate proposals, child processes, or
replayed prior approvals.

## Origin binding

Destination URLs must use the `http` or `https` scheme with an explicit
host. Userinfo, backslashes, fragments, empty hosts, wildcard hosts,
zone identifiers, trailing dots, and out-of-range or non-numeric ports
are rejected. Hosts normalize to exactly one lowercase spelling:
bracketed IPv6 literals, plain dotted-decimal IPv4 literals, or dotted
DNS hostnames with at least one letter. Alternate numeric host syntaxes
(hexadecimal, octal, and bare-integer IPv4 forms) are rejected before
address policy so parser differentials cannot smuggle loopback or private
targets past validation. `localhost` and its aliases (`*.localhost`,
`*.local`, `*.internal`, `*.lan`, `*.home`) are denied. Origins compare
by normalized `scheme://host:port` identity with exact equality, never by
string prefix, so `example.com.evil.com` never matches `example.com` and
an unexpected port never matches the default-port origin.

## SSRF and redirect policy

Validation re-resolves the bound host through an injectable resolver and
applies post-resolution address policy: loopback, unspecified, multicast,
link-local, private, broadcast, documentation, shared, benchmarking, and
reserved addresses fail closed, including IPv4-mapped IPv6 forms. The
deterministic pinned address is the sorted-first public address. When
several addresses resolve, a non-public subset cannot be selected. No
explicitly trusted workspace browser destination is configured in this
grain, so every non-public destination is denied; successor trust
configuration must arrive through its own canonical grain.

Redirect targets revalidate fully: the target URL reparses, its exact
origin must equal the validated base origin, and resolution plus address
policy run again. A permitted public origin therefore cannot silently
widen into a broader authority through redirects.

## Retention

SG-000018 one-shot, expiry, digest, workspace, and policy bindings,
SG-000019 STRONG enforcement with no SOFT downgrade, SG-000020
STRONG-gated trust changes with epoch-bound revoke, and all P06
approval flows are unchanged. Browser destination policy is the only new
authority, and it authorizes no actuation, personal-profile, debugging,
credential, network-egress, reuse, delegation, or elevation capability.
