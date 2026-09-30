import json
from pathlib import Path

implementation_base = "ffaad8ad0d1ee189c7a41dc058ef6f7165e41e28"
qualified_head = "75862656012dd626445d10ac7686846bda67a98e"
implementation_merge = "0133e500802e9c9b8e6181df3c73614c67bb06e2"
pre_merge_ci = "36767820258"
pre_merge_review = "36767815559"
post_merge_ci = "36769671042"

spec_path = Path(".specgrain/specs/SG-000040.json")
spec = json.loads(spec_path.read_text(encoding="utf-8"))
assert spec["id"] == "SG-000040"
assert spec["state"] == "GRAIN"
spec["state"] = "CLOSED"
spec["qualified_head"] = qualified_head
spec["implementation_base"] = implementation_base
spec["implementation_merge"] = implementation_merge
spec["pre_merge_ci"] = pre_merge_ci
spec["pre_merge_review_gates"] = pre_merge_review
spec["post_merge_ci"] = post_merge_ci
spec["canonical_evidence"] = {
    "implementation_pr": 114,
    "implementation_base": implementation_base,
    "qualified_head": qualified_head,
    "pre_merge_ci": pre_merge_ci,
    "pre_merge_review_gates": pre_merge_review,
    "merge_sha": implementation_merge,
    "post_merge_ci": post_merge_ci,
    "jev": "PASSED: genuine TypeSafe Jev exact-diff review covered 13/13 hunks with zero findings and zero blocking findings, pin 31f89602797fb7bea007f8a480bf368bf564954e, TypeSafe SDK 0.6.0",
    "ocr": "PASSED: Alibaba Open Code Review v1.12.9 exact-range delegation over 12 changed files, 9 reviewable and 3 excluded files manually reviewed",
    "security_review": "PASS on exact qualified head: one internal network/fetch shape only; fixed HTTPS GET to port 443; exact bounded URL and dotted-DNS-hostname validation; public-only IPv4/IPv6 resolution with dangerous special-use ranges denied; exact post-approval and per-hop address-set consistency; pre-send WinHTTP resolution pin to one validated public address with original hostname retained for TLS identity and post-response exact connected-peer verification; manual exact-origin redirects with loop, downgrade, private-target, and >5-hop denial; direct/no-proxy fresh transport with cookies, authentication, keep-alive, caller headers, request bodies, and ambient credentials disabled; 1048576-byte incremental response ceiling and typed remote-error failure without body leakage; fresh SOFT exact-digest approval and inherited nonce, expiry, one-shot, workspace, policy, replay, and class enforcement; real Windows https://example.com/ qualification passed with OS certificate validation and peer pinning; no MCP network tool; generic sockets, CONNECT, WebSockets, standing sessions, filesystem/process/UI/elevation widening remain absent. The first Windows exact-head attempt exposed only transient unrelated SG-000013 PowerShell timing failures; the same exact head was rerun without code changes and the complete Windows Rust job, including all SG-000040 tests and the SG-000013 regressions, passed.",
    "unresolved_review_threads": 0,
    "authority_boundary": "One explicit bounded internal network/fetch HTTPS GET is newly closed: https only, port 443, public-only DNS, exact address-set and peer binding, manual same-origin redirects, bounded timeouts and 1 MiB response, no proxy or ambient credentials, fresh SOFT digest-bound one-shot approval, and bounded secret-free evidence. No MCP network surface, generic TCP/UDP/raw sockets, listeners, CONNECT proxying, WebSockets, other HTTP methods/schemes/ports, standing sessions, filesystem/process/browser/UI authority, elevation, or approval bypass is added; SG-000038 clipboard read and SG-000039 clipboard write remain unchanged."
}
spec_path.write_text(json.dumps(spec, indent=2) + "\n", encoding="utf-8")

ledger_path = Path(".specgrain/canonical-evidence.json")
ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
assert "SG-000040" not in ledger["grains"]
ledger["snapshot_base"] = implementation_merge
ledger["grains"]["SG-000040"] = {
    "implementation_pr": 114,
    "implementation_base": implementation_base,
    "qualified_head": qualified_head,
    "pre_merge_ci": pre_merge_ci,
    "merge_sha": implementation_merge,
    "post_merge_ci": post_merge_ci,
    "review_basis": "Exact-head genuine TypeSafe Jev 13/13 hunks with zero findings/blocking findings, Alibaba Open Code Review v1.12.9 exact-range delegation over 12 files with 9 reviewable and 3 excluded files manually reviewed, exact-diff manual security review, Windows/Ubuntu Rust Node Governance qualification including a real Windows WinHTTP HTTPS/TLS peer-pin test, successful post-merge CI, and zero unresolved review threads. One unrelated SG-000013 Windows timing flake on the first attempt passed on exact-head job rerun without any code change.",
    "unresolved_review_threads": 0,
    "authority_boundary": "One explicit bounded internal network/fetch HTTPS GET is closed with https/443-only destination policy, public-only resolution, exact address-set and connected-peer binding, manual same-origin redirects, bounded request/response/timeouts, direct no-proxy transport, no ambient credentials, fresh SOFT digest-bound one-shot approval, and bounded secret-free evidence. Generic sockets, CONNECT, WebSockets, alternate methods/schemes/ports, standing sessions, MCP network tools, elevation, and authority outside the closed P11 shapes remain absent."
}
ledger_path.write_text(json.dumps(ledger, indent=2) + "\n", encoding="utf-8")

current_path = Path("docs/canonical/CURRENT.md")
text = current_path.read_text(encoding="utf-8")
assert "Status: ACTIVE_GRAIN" in text
assert "Governance snapshot base: 633d0b32924e45669301994d90dc8ef40502e2e0" in text
text = text.replace("Status: ACTIVE_GRAIN", "Status: CLOSED_CANONICAL", 1)
text = text.replace(
    "Governance snapshot base: 633d0b32924e45669301994d90dc8ef40502e2e0",
    f"Governance snapshot base: {implementation_merge}",
    1,
)
text = text.replace(
    "`Governance snapshot base` records the exact canonical parent from which this activation snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.",
    "`Governance snapshot base` records the exact canonical implementation merge from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.",
    1,
)

start = text.index("## Active grain\n")
end = text.index("## Canonical public authority boundary retained\n", start)
closed_block = f'''## Closed SG-000040 destination-scoped network fetch

SG-000040 closed canonically: activation PR `#113` (activation base `633d0b32924e45669301994d90dc8ef40502e2e0`, activation head `e5c39e7802a5a93e4b1fc9f12d3d4ef95c3e8ac8`, CI `36742809059`, Review Gates `36742809200`, merge `ffaad8ad0d1ee189c7a41dc058ef6f7165e41e28`, post-merge CI `36744853181`), implementation PR `#114` (qualified head `{qualified_head}`, CI `{pre_merge_ci}`, Review Gates `{pre_merge_review}`, genuine TypeSafe Jev `13/13` hunks with zero findings and zero blocking findings, Alibaba Open Code Review v1.12.9 `9 reviewable + 3 excluded/manually reviewed`, merge `{implementation_merge}`, post-merge CI `{post_merge_ci}`), exact-diff manual security review, and zero unresolved review threads. The canonical SG-000040 authority adds only one explicit internal bounded `network/fetch` operation: fixed HTTPS GET to port 443, public-only destination resolution, exact pre/post-approval and per-hop address-set consistency, pre-send Windows peer pinning with post-response exact-peer verification, manual exact-origin redirects bounded to five hops, direct/no-proxy fresh transport, bounded timeouts, a 1048576-byte incremental response ceiling, no ambient credentials, and fresh SOFT digest-bound one-shot approval. No MCP network tool, generic sockets, CONNECT proxying, WebSockets, standing sessions, alternate methods/schemes/ports, filesystem/process/UI authority, elevation, or approval bypass is added.

SG-000040 proved the exact URL and destination ceilings; full dangerous-address denial across private, loopback, link-local, multicast, unspecified, documentation, CGNAT, mapped-private, and reserved IPv4/IPv6 classes; post-approval and per-hop DNS drift failure as stale; exact connected-peer enforcement; manual redirect loop, cross-origin, private-target, scheme-downgrade, and over-five-hop denial; exact 2048-character URL and 1048576-byte response bounds; typed remote HTTP failures without response-body leakage; direct no-proxy transport with cookies, authentication, automatic redirects, keep-alive, caller headers, request bodies, and ambient credentials absent; inherited replay, expiry, digest, workspace, policy, one-shot, and approval-class enforcement; no MCP network exposure; and real Windows WinHTTP HTTPS/TLS transport against `https://example.com/` with the validated peer pinned before send and verified after response. The first exact-head Windows run had only transient unrelated SG-000013 PowerShell timing failures; the exact same qualified head was rerun without code changes and the entire Windows Rust suite passed.

## Active grain

No COTRA-P11 grain is active. SG-000038 bounded clipboard read, SG-000039 bounded clipboard write, and SG-000040 destination-scoped network fetch are all closed canonically.

## Successor frontier

The next lawful unit is COTRA-P11 program-exit governance only. It must jointly prove from SG-000038, SG-000039, and SG-000040 that clipboard access remains bounded and non-surveillant, clipboard write grants no paste or input authority, destination policy and private-address denial remain enforced, network authority remains limited to the closed destination-scoped HTTPS GET shape, approval replay/expiry/digest/workspace/policy/class controls remain intact, and secrets are never emitted. No COTRA-P12 installer or lifecycle grain is authorized until that P11 exit is merged and its post-merge CI succeeds.

'''
text = text[:start] + closed_block + text[end:]

denied_old = "- generic network fetch/socket authority;"
assert denied_old in text
text = text.replace(
    denied_old,
    "- generic network fetch/socket authority beyond the closed SG-000040 destination-scoped HTTPS GET shape;",
    1,
)

stale_old = "COTRA-P07, COTRA-P08, and COTRA-P09 are exited at this frontier, so a later lawful P10 grain must build only on top of the SG-000018 replay-resistant foundation, SG-000019 class enforcement, SG-000020 trust and revoke records, the closed P08 structured-browser registry, and the closed P09 structured-UIA registry. Raw credentials must never be accepted merely because Git can consume them."
assert stale_old in text
text = text.replace(
    stale_old,
    "COTRA-P07 through COTRA-P10 are exited at this frontier. COTRA-P11 now has its three planned capability grains closed, but P11 itself does not exit until the dedicated joint program-exit governance unit merges and passes post-merge CI. P12 remains unauthorized until that exit. Raw credentials must never be accepted merely because Git or a network transport can consume them.",
    1,
)

boundary_marker = "\n\nStill denied or absent outside the closed COTRA-P07 scope, the closed COTRA-P08 scope, and the closed COTRA-P09 scope:"
assert boundary_marker in text
network_boundary = "\n\nExisting SG-000040 destination-scoped bounded network fetch remains subject to fixed HTTPS GET and port 443 only, dotted DNS hostname validation, public-only resolution, exact pre/post-approval and per-hop address-set consistency, pre-send Windows peer pinning with exact post-response connected-peer verification, manual exact-origin redirects bounded to five hops, direct no-proxy fresh transport, bounded timeouts, a 1048576-byte incremental response ceiling, no caller headers or body, no cookies or ambient credentials, fresh SOFT exact-digest one-shot approval, bounded secret-free evidence, and denial of generic sockets, CONNECT, WebSockets, standing sessions, alternate methods/schemes/ports, MCP network exposure, and elevation."
text = text.replace(boundary_marker, network_boundary + boundary_marker, 1)

current_path.write_text(text, encoding="utf-8")

json.loads(spec_path.read_text(encoding="utf-8"))
json.loads(ledger_path.read_text(encoding="utf-8"))
final = current_path.read_text(encoding="utf-8")
assert "Status: CLOSED_CANONICAL" in final
assert "## Closed SG-000040 destination-scoped network fetch" in final
assert "The next lawful unit is COTRA-P11 program-exit governance only." in final
