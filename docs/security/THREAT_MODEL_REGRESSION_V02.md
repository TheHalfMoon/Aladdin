# Threat Model Regression for the Deskal v0.2 Release Candidate

Status: SG-000072 RELEASE REGRESSION
Date: 2026-10-04
Baseline: `docs/security/THREAT_MODEL.md` (T01-T28),
`docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md` (UC series),
`docs/security/THREAT_MODEL_REGRESSION.md` (QDRAL-P13 regression evidence,
retained as historical record; v0.2-relevant controls are re-proven below
with current tests).

Scope: every v0.2 surface added or exposed after P13 -- local MCP edge,
loopback HTTP transport, relay protocol and device channel, OAuth 2.1
authorization, device uplink leases and remote sessions, desktop observation,
clipboard transfer, bounded network fetch, the protected executable registry
and filesystem mutation tools, provider distribution packages (OpenAI, Claude,
Mistral/Vibe, Codex, generic MCP), cross-provider isolation, installer and
lifecycle (install, update from 0.1.0, rollback, recovery, uninstall), and the
supply chain (SBOM, provenance, reproducibility, audits, notices).

Method: each row names controls as implemented and the regression tests that
exercise them. Test names are quoted exactly as they appear in
`cargo test -- --list` and the Node `node --test` suites; every one runs in
exact-head CI (Windows-native tests on `windows-latest`, relay/container and
supply-chain jobs on `ubuntu-latest`). A claim with no named test is marked
UNVERIFIED. This regression never weakens a control to pass.

Product identity note: the current product is Deskal. `qdrald`,
`%LOCALAPPDATA%\Qdral` paths, `QDRAL_*` variables, `qdral.*` scopes, and the
`qdral` MCP compatibility name below are retained compatibility identifiers
governed by `docs/identity/DESKAL_RENAME.md`.

## 1. Local MCP edge (T01, T02)

Controls: MCP is served over stdio through the authoritative builder only;
transport adapters register no tools independently; the surface registers
exactly the canonical closed tool set; no MCP tool source reaches lifecycle,
installer, update, trust, or approval surfaces; contract metadata is
metadata-only and cannot reach the kernel.

Regression evidence:

- `the MCP surface registers exactly the canonical closed tool set`
- `no MCP tool source reaches lifecycle, installer, update, trust, or approval surfaces`
- `authoritative catalog matches the closed v0.1 tool set`
- `transport adapters register no tools independently`
- `stdio transport reuses the authoritative builder`
- `installed stdio entrypoint reuses the stdio transport`
- `result projection is the single authoritative source`
- `transport context is server-supplied metadata, never a tool field`
- `reserved transports fail closed without authority`
- `authoritative builder registers without spawning qdrald`
- `every canonical tool has exactly one contract entry`
- `the contract module is metadata only and can reach no kernel`
- `request builder preserves the declared bounded capability`
- `daemon environment drops generic and Qdral secret variables`

## 2. Loopback HTTP transport (T01)

Controls: loopback-only listener with a required credential, bound loopback
host names, local-origin validation, no wildcard preflight origin, typed
refusals for unauthenticated/forged/oversized/unknown traffic; the health
probe accepts only loopback URLs.

Regression evidence:

- `loopback credential and port fail closed`
- `host validation admits only the bound loopback names`
- `origin validation admits only local origins and non-browser clients`
- `loopback refuses to start without a credential`
- `loopback serves the authoritative catalog over an authenticated session`
- `loopback denies unauthenticated, forged, oversized, and unknown traffic`
- `loopback preflight never emits a wildcard origin`
- `only_loopback_http_urls_are_accepted`
- `serve_launch_points_at_the_loopback_entrypoint`
- `serve_without_credential_zero_port_or_tampering_fails_closed`
- `SG-000069: Vibe Code examples use the canonical entrypoint and the loopback bearer contract`

## 3. Relay protocol, device channel, and public edge (UC relay series)

Controls: frozen protocol vocabulary (version, frame kinds, envelope fields,
17 failure codes, bounds); authorization envelope binds principal, connection,
device, digest, epoch, and session; relay sources carry no proxy, executor, or
tool registration; device transport is a fail-closed skeleton; sessions are
route-bound and bounded; offline devices fail closed with no queue; restart
drops sessions with no replay; logs carry classes only, never secrets.

Regression evidence:

- `relay protocol version is frozen`
- `relay frame kinds are exactly the frozen set`
- `relay envelope requires every frozen field`
- `authorization envelope binds principal, connection, device, and digest`
- `relay failure vocabulary is frozen at 17 codes`
- `relay failures stay distinct from local failures`
- `denied relay capabilities are frozen and detectable`
- `relay bounds are frozen, positive, and ordered`
- `relay payload bound accepts only in-range sizes`
- `relay device transport stays a fail-closed skeleton`
- `relay vocabulary module carries no network or tool authority`
- `contract document freezes the vocabulary`
- `an authenticated MCP client reaches the paired device through the outbound-only channel`
- `negative: missing, malformed, expired, wrong-audience, wrong-issuer, revoked, and stale-epoch tokens fail before framing`
- `negative: missing scope and unknown tools are denied at the edge without device dispatch`
- `negative: sessions are bound to their route and bounded per route`
- `negative: an offline device fails closed with DEVICE_OFFLINE and nothing is queued`
- `negative: malformed protocol shapes, origins, hosts, sizes, and paths are rejected`
- `negative: device channel endpoints refuse unproven devices, bad tokens, and forged responses`
- `relay sources carry no generic proxy capability, executor, or tool registration`
- `refusals are known before any sequence is used`
- `a request that times out before delivery becomes a cancel and is never executed`
- `a request that times out after delivery reports an unknown outcome`
- `closing a channel distinguishes never-executed from unknown outcomes`
- `push accepts only exact, correlated, single-use responses for this device`
- `a revoked device loses its channel and an unproven device never opens one`
- `adversarial: two tenants on one relay never cross principal, device, route, client, or session`
- `adversarial: metadata and arguments can never select remote identity, and management tools do not exist`
- `adversarial: relay restart drops sessions, devices reconnect, and nothing replays or extends`
- `adversarial: per-route quota exhaustion fails closed before the device`
- `adversarial: relay and uplink logs carry classes only, never tokens, codes, identifiers, or payloads`
- `relay state survives restart, keeps its signing key, and refuses corrupt state`
- `quotas are explicit, bounded by hard ceilings, and fail closed without overflow`
- `self-host configuration is strict`

## 4. OAuth 2.1 authorization (UC OAuth series)

Controls: strict scope vocabulary (`read`, `write`, `execute`, default deny);
strict issuer/resource/redirect/client metadata; PKCE S256 only;
short-lived one-shot codes bound to client/redirect/resource/PKCE; RFC 9207
issuer on responses; tokens prove remote principal identity only and grant no
local authority; refresh rotates with fresh device proof and never widens
scope; replay revokes the family; provider client-identification hooks
default-deny.

Regression evidence:

- `oauth failures are a strict subset of the frozen relay vocabulary`
- `scope vocabulary is exactly read, write, execute and parsing denies unknown scopes`
- `issuer, resource, redirect, and client metadata URL shapes are strict`
- `authorization-server and protected-resource metadata are standards-shaped and validated`
- `PKCE accepts only S256 with a matching verifier`
- `authorization requests bind client, exact redirect, PKCE S256, exact resource, scopes, and state`
- `authorization codes are short-lived, one-shot, and bound to client, redirect, resource, and PKCE`
- `authorization response carries RFC 9207 issuer and rejects mix-up and CSRF`
- `a valid access token yields remote principal identity only`
- `minting refuses a foreign resource, widened or empty scopes, and revoked families`
- `negative: expired, not-yet-valid, and over-long tokens fail closed`
- `negative: revoked token, revoked family, and revoked route fail closed`
- `negative: wrong issuer, wrong audience, and wrong resource fail closed`
- `negative: unknown scope and missing scope fail closed`
- `negative: stale device epoch fails closed`
- `negative: malformed tokens, forged signatures, and algorithm confusion fail closed`
- `refresh rotates the token, requires fresh device proof, and keeps scope from widening`
- `negative: stolen refresh token cannot renew while the bound device is offline`
- `negative: missing, mismatched, replayed, and expired device proofs fail closed`
- `negative: stale device epoch after key rotation blocks refresh and revokes the family`
- `negative: refresh token replay after rotation revokes the whole family`
- `negative: revoked device, principal, and family block refresh`
- `negative: refresh binding to client, resource, family, and grant type`
- `provider client-identification hooks default-deny`
- `scope to tool matrix covers exactly the canonical catalog with default deny`
- `OAuth grants no local authority`
- `oauth module carries no network, tool, kernel, approval, or transport authority`
- `negative: authorization code replay is refused and revokes every token issued from the code`
- `negative: authorization requests and pairing codes are strictly validated`
- `a standards OAuth client links through device pairing and reaches the device via a self-hosted relay`

## 5. Device identity, uplink leases, and remote sessions

Controls: tenant-scoped opaque device identity; private keys never leave the
device; challenge/response binds device and epoch; one-time pairing with
entropy/expiry/one-shot/rate limits; hard, principal-wide, all-route, and
emergency revocation; envelope verifies principal/connection/device/digest/
epoch/session; compromised-relay remap fails closed; local lease creation and
revocation only -- the remote caller cannot create, widen, or silently renew;
reconnect never extends or rebinds; route/profile/policy/trust drift
deactivates; cross-provider contexts never share a lease.

Regression evidence:

- `device identity format is tenant-scoped and opaque`
- `protected device key separates public identity from private key`
- `challenge and response authentication binds device and epoch`
- `challenge verification fails closed on remap, epoch, expiry, and forgery`
- `one-time pairing enforces entropy, expiry, one-shot, and rate limits`
- `pairing rate limiting invalidates after five failures without trust`
- `hard, principal-wide, all-route, and emergency revocation invalidate sessions`
- `authorization envelope verifies exact principal, connection, device, digest, epoch, and session`
- `compromised relay cannot remap principal A onto device B`
- `device identity uses only the frozen relay failure vocabulary`
- `device identity module carries no network, tool, or approval authority`
- `relay device transport stays a fail-closed skeleton`
- `a verified request dispatches once with server-supplied remote context and answers on the same route`
- `negative: replayed nonce, duplicate correlation, and repeated sequence never re-execute`
- `negative: reordered and gapped sequences fail with RELAY_SEQUENCE_INVALID`
- `negative: expired, future, and over-long frames fail closed`
- `negative: oversized and malformed frames never reach dispatch`
- `negative: wrong channel, wrong device, unknown route, and cross-route reuse fail without dispatch`
- `negative: principal remap, digest tampering, stale epoch, and revoked device fail the envelope`
- `negative: scopes beyond the paired ceiling and tools beyond token scopes are denied before dispatch`
- `negative: queue-after-revoke and queue-after-expiry never execute`
- `bounded queue, per-connection concurrency, device rate, and connection count apply backpressure`
- `cancel removes queued work and suppresses in-flight results`
- `reconnect grace is bounded and responses are re-authenticated with the new channel`
- `uplink state loading fails closed`
- `the MCP frame dispatcher serves the authoritative catalog and surfaces typed lease denials`
- `the outbound-only client authenticates with the device key, long-polls, and pushes responses`
- `a relay that cannot prove the device challenge binding never gets a channel`
- `uplink sources open no listener and carry no generic proxy capability`
- `the relay transport fails closed with PAIRING_REQUIRED when protected pairing state is absent`
- `refresh proofs are signed only for this exact device, epoch, and a fresh bounded challenge`
- `active_exact_lease_permits_and_pins_connection`
- `cross_provider_contexts_never_share_a_lease`
- `no_lease_expired_lease_and_revoked_lease_are_inactive`
- `reconnect_does_not_extend_or_rebind_the_lease`
- `route_profile_policy_and_trust_drift_are_inactive`
- `lock_logoff_and_unknown_workstation_state_are_inactive`
- `workspace_scope_and_profile_ceilings_fail_closed`
- `local_authority_management_is_never_remote`
- `lease_creation_is_bounded_and_requires_trusted_configured_workspaces`
- `approval_digest_binds_every_lease_field`
- `store_is_atomic_checksummed_and_fails_closed_when_tampered`
- `lease_arguments_default_to_the_paired_ceiling_and_cap_duration`
- `relay_origins_are_https_or_exact_loopback_without_paths`
- `device_identity_and_pairing_records_fail_closed`

## 6. Desktop observation and approval-surface protection (T07, T15, UC-T25)

Controls: exactly two read-only desktop shapes (`desktop_window_list`,
`desktop_window_tree`) over typed window identity; no actuation, capture,
input, focus, or termination; Deskal approval/trust/revoke surfaces are
denied; discovery exposes only qualified shapes per profile.

Regression evidence:

- `only desktop.ts forwards UIA capabilities, and only the two read-only shapes`
- `the exposed desktop shapes are exactly the live shapes pinned in the provider`
- `the tree tool binds a typed window identity and the provider bounds`
- `no MCP tool source forwards a browser capability to the kernel`
- `no browser tool is registered on the MCP server surface`
- `no MCP tool claims browser capability`
- `every structured browser shape is recorded as not exposed and denied in the inventory`
- `the structured browser layer launches, attaches, and connects to nothing outside tests`
- `hidden and denied shapes are real qdrald shapes, and hidden ones are not exposed`
- `missing and hidden entries name an owning grain; denials name a security reason`

## 7. Clipboard transfer (T18)

Controls: only read and write shapes forwarded, Unicode-text only, hard size
bound, secret-pattern denial, one-shot per-approval writes, sequence-bound
evidence; no surveillance, paste, or transport reuse.

Regression evidence:

- `only clipboard_network.ts forwards clipboard shapes, and only read and write`
- `clipboard write text is bounded by UTF-8 bytes before reaching the kernel`
- `the fetch tool accepts only bounded https URLs`
- `fetch bodies project to UTF-8 text or binary metadata only`

## 8. Bounded network fetch (T12-adjacent)

Controls: https/443-only destination policy, public-only resolution, exact
address-set and connected-peer binding, manual same-origin redirects, bounded
request/response/timeouts, direct no-proxy transport, no ambient credentials,
fresh one-shot approval.

Regression evidence:

- `only clipboard_network.ts forwards network/fetch, with only a url`
- `network_fetch_is_soft_and_exact_shape_only`
- `alternate_network_shapes_are_strong_denied`
- `malformed_or_widened_urls_fail_closed`
- `SG-000016 MCP schemas reject arbitrary URLs, refspecs, and credentials`
- `SG-000017 MCP schemas reject arbitrary URLs, refspecs, force, and raw credentials`

## 9. Approvals, presence, trust, and revocation (T09, T10, T11)

Controls: broker-level nonces with short expiry, one-shot exact-digest
binding, SOFT/STRONG classes with platform-mediated presence, no caller-made
approvals, no reuse, emergency revoke with epoch invalidation, redacted
bounded tamper-evident history.

Regression evidence:

- `nonces_are_fresh_unique_and_bound_to_digest`
- `broker_contract_distinguishes_soft_and_strong_classes`
- `ledger_consumes_once_then_rejects_replay`
- `strong_consumes_once_then_rejects_replay`
- `soft_button_never_satisfies_strong_class`
- `strong_requires_platform_presence_and_fails_closed_when_unavailable`
- `strong_never_downgrades_to_soft`
- `forged_attestation_without_broker_token_cannot_authorize`
- `ledger_rejects_expiry_mismatch_and_drift`
- `denied_and_unavailable_records_never_authorize`
- `history_records_class_and_presence_without_secrets`
- `history_is_redacted_bounded_and_tamper_evident`
- `forged_history_is_rejected`
- `expired_strong_approval_fails_closed`
- `wrong_nonce_workspace_and_revision_fail_closed_for_strong`
- `emergency_revoke_requires_strong_and_invalidates_prior_tokens`
- `emergency_revoke_with_soft_class_fails_closed`
- `emergency_revoke_without_presence_fails_closed`
- `revoked_tokens_fail_closed_after_reload`
- `approvals_after_revoke_use_new_epoch`

## 10. Secrets, logs, and audit integrity (T20, T21, T22)

Controls: child environments carry workspaces and no secrets; key-like tokens
redacted in logs; transcripts never written through links; bounded rotation;
typed error envelopes; structured secret-free events; doctor reports state
without secrets; corrupt state fails closed.

Regression evidence:

- `child_environment_carries_workspaces_and_no_secrets`
- `key_like_tokens_and_assignments_are_redacted`
- `spaced_tabbed_pretty_and_later_assignments_are_redacted`
- `ordinary_words_containing_markers_are_not_redacted`
- `transcript_never_writes_through_a_link`
- `log_rotates_at_the_bound`
- `writes_structured_event_without_arguments`
- `remote_integration_reports_state_without_secrets`
- `uninstalled_root_fails_integrity_without_claiming_health`
- `corrupt_state_files_fail`
- `error_envelopes_are_typed`

## 11. Executable registry and filesystem mutation (T03, T04, T05)

Controls: registry admits only locally registered hash-pinned native
non-interpreter executables with constrained argv; shells, interpreters,
scripts, and UNC paths never registrable; tamper checksum fails closed;
filesystem mutation is six bounded static tools with no recursive, wildcard,
overwriting, link, or out-of-workspace authority; protected state excluded
from every workspace; reparse/parent-escape/device/ADS/reserved denied.

Regression evidence:

- `shells_interpreters_scripts_and_unc_paths_are_never_registrable`
- `workspace_executables_are_refused_and_ids_are_strict`
- `argv_grammar_is_enforced`
- `modified_or_moved_executables_fail_closed_before_launch`
- `registry_store_is_checksummed_and_fails_closed_when_tampered`
- `authorizes_only_bounded_argv_process_spawn`
- `process_spawn_rejects_escape_shell_network_stdin_env_and_unbounded_inputs`
- `process_spawn_rejects_unqualified_shell_and_powershell_executables`
- `process.spawn schema accepts only the bounded argv contract`
- `process.spawn schema exposes no raw command, env, stdin payload, or network widening`
- `process.spawn schema enforces explicit bounds`
- `sg000041_refuses_workspace_equal_to_above_or_below_protected_state`
- `sg000041_refuses_workspace_containing_each_override`
- `rejects_parent_escape`
- `rejects_windows_root_and_device_paths_portably`
- `SG-000015 MCP schemas accept only typed local Git mutation inputs`
- `SG-000015 MCP schemas reject missing or malformed expected HEAD`
- `allows_only_bounded_local_git_mutations`
- `rejects_network_destructive_and_unsafe_git_requests`

## 12. Donor-code trust transfer (T24)

Controls: Desktop Commander, Kernux, and UI-TARS inputs are pinned reference
evidence only; no donor runtime or dependency in the shipped authority; future
P18 grains remain outside the active registry; authority-neutral exit evidence.

Regression evidence:

- `SG-000066 exit evidence is pinned and authority-neutral`
- `Desktop Commander, Kernux source pool, and UI-TARS donors are exact reference-only pins`
- `the donor reuse matrix is complete, pinned, obligation-bearing, and import-neutral`
- `UI-TARS authority is either deferred or denied, never imported by SG-000066`
- `SG-000066 imports no Desktop Commander, Kernux, or UI-TARS runtime`
- `future P18 grains remain outside the active SpecGrain registry`
- `local authority remains unavailable to agents`
- `the parity matrix covers all 36 workflows exactly once using the SG-000066 vocabulary`
- `Desktop Commander high-authority shapes remain explicit denials`
- `the unsafe Desktop Commander shapes are recorded as security decisions, not gaps`

## 13. Provider distribution packages (P17)

Controls: builders and validators only; no kernel capability, MCP tool,
OAuth scope, profile, approval, trust, lease, shell, or network authority;
per-tool security schemes; loopback-redirect matching per RFC 8252; provider
directory and account states remain NOT_OBSERVED and are never promoted by
software; cross-provider isolation with no shared credentials, sessions,
routes, or revocation.

Regression evidence:

- `the Deskal package builds reproducibly from clean source and validates`
- `OpenAI-visible tools are exactly the core profile with contract annotations and schemes`
- `tampered packages fail validation`
- `configuration and endpoint validation fail closed`
- `listing URLs point at real repository documents and no paid service is required`
- `the review bundle has exactly five positive and three negative cases over core tools`
- `provider-controlled distribution states are never promoted by software`
- `authorization metadata advertises only what the relay implements`
- `exactly one grain is active and no later grain is activated ahead of it`
- `the Deskal Codex plugin declares exactly the canonical local stdio entrypoint`
- `the repository marketplace lists exactly the Deskal plugin from this repository`
- `shared tools carry identical metadata on local and remote transports`
- `SG-000068: Claude directory and account states are never promoted by software`
- `SG-000068: Claude web and Claude Code connector flows reach the device with the core profile only`
- `SG-000068 loopback redirects match port-agnostically and nothing else widens`
- `SG-000069: Vibe Code examples use the canonical entrypoint and the loopback bearer contract`
- `SG-000069: Mistral directory and account states are never promoted by software`
- `SG-000069: a Vibe Work-style OAuth connector reaches the core profile and static bearer credentials are refused`
- `SG-000070: a Codex-style OAuth client with a loopback redirect reaches the core profile`
- `SG-000071: two providers on one device never share credentials, sessions, routes, or revocation`
- `SG-000071: approvals, scopes, and tool sets are identical for every provider`
- `remote discovery lists only the core profile for every known provider`
- `provider-specific mappings are explicit and unknown providers deny`
- `profile membership is exact and unmapped profiles fail closed`
- `client examples use only supported installed entrypoints`
- `client examples carry no secrets and no remote endpoints`
- `desktop extension manifest matches the canonical catalog`

## 14. Install, update, rollback, recovery, uninstall (T23)

Controls: verified manifest-based install under owner-only `%LOCALAPPDATA%`
trees, atomic version pointer, lifecycle lock, offline update with
downgrade-refusal by default, interrupted/failed-update recovery, rollback to
the previous version, retention-aware uninstall, link/Junction refusal before
ACL reset, supervisor-busy uninstall block.

Regression evidence (unit plus Windows-native):

- `install_verifies_stages_and_records_state`
- `tampered_release_installs_nothing`
- `same_version_reinstall_repairs_and_other_version_is_refused`
- `uninstall_retains_user_data_by_default`
- `purge_removes_data_only_when_requested`
- `running_supervisor_blocks_uninstall`
- `link_inside_install_tree_blocks_acl_reset`
- `install_root_must_not_be_a_link`
- `valid_release_verifies`
- `tampered_payload_fails_closed`
- `unlisted_file_fails_closed`
- `missing_required_file_fails_closed`
- `hostile_paths_are_rejected`
- `check_reports_direction_without_changes`
- `downgrade_requires_explicit_flag`
- `failed_self_check_restores_the_previous_version`
- `start_is_refused_while_an_update_is_pending`
- `rollback_recovers_a_crash_between_pointer_and_install_record`
- `rollback_without_an_install_reports_not_installed`
- `rollback_without_previous_is_refused`
- `real_cli_installs_verifies_and_uninstalls_from_a_release`
- `install_configure_start_status_doctor_stop_restart_and_uninstall`
- `packaged_release_installs_runs_updates_recovers_and_uninstalls`
- `protect_tree_resets_foreign_access_and_verification_detects_it`
- `junction_inside_install_root_is_refused_before_acl_reset`

The release-qualification suite exercises update from the 0.1.0 line,
failed-update recovery, rollback, and uninstall against the packaged 0.2.0
candidate on `windows-latest` in exact-head CI.

## 15. Supply chain (T23)

- Dependency vulnerability audit: `cargo audit` 0.22.2 (RustSec database,
  1,290 advisories at 2026-10-03) and `npm audit` both report 0
  vulnerabilities for the SG-000072 tree, verified on native Windows on
  2026-10-04 and enforced by the CI `Supply chain audit` job on every change.
- License review: `docs/research/DEPENDENCY_LICENSE_REVIEW.md` (SG-000072 v0.2
  refresh) records every shipped dependency as permissive (MIT, Apache-2.0,
  Unlicense, Unicode-3.0) with no copyleft obligations.
- Notices: `THIRD_PARTY_NOTICES.txt` is regenerated from the release payload
  by `scripts/third-party-notices.mjs` and shipped inside it; the generator
  fails on any component without a declared license or shipped license file,
  and the current release has none.
- SBOM: `scripts/generate-sbom.mjs` emits a CycloneDX SBOM for the packaged
  candidate in the release-qualification job.
- Provenance: `scripts/provenance.mjs` records the build provenance attested
  by the draft-only release workflow.
- Reproducibility: the `reproducibility-check` action performs an independent
  byte-identical rebuild of the candidate in exact-head CI.

## 16. Residual risks and UNVERIFIED states

- T27 (same-user compromise): same-user malicious code can read the user's own
  files and stop Deskal; ACLs, AppContainer children, and STRONG presence
  reduce but cannot remove this. No new claim is made.
- Provider-controlled states (directory submission/listing, account
  verification, published/approved store states) remain NOT_OBSERVED in every
  `distribution/*/distribution-state.json` and are never promoted by software.
- Performance/latency/throughput targets: no targets are claimed, so none are
  verified. Resource ceilings are enforced by the hard bounds tested above
  (queues, concurrency, sizes, counts); quantitative performance measurement
  is UNVERIFIED and not required for this release.
- Live relay reachability across the public internet is operator-dependent and
  UNVERIFIED; the relay container probe in CI proves the packaged image
  serves, pairs, and fails closed, not that any particular host is reachable.
