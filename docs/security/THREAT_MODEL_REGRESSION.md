# Threat Model Regression (QUNTAL-P13)

Status: release regression of every threat in `docs/security/THREAT_MODEL.md` (T01–T28) against the closed implementation, plus the installer and update lifecycle added in QUNTAL-P12. Each row names the controls as implemented and the regression tests that exercise them. Tests are named as they appear in `cargo test -- --list` and the Node test suite; every one runs in exact-head CI (Windows-native tests on `windows-latest`). Where a control cannot be proven by an automated test, the row says so.

The regression suite is the whole CI suite: 542 Rust tests passing on Windows in post-merge CI run `36831437317` (plus the Ubuntu subset; the one ignored test is the release-qualification test, which runs in its own job), the Node MCP surface tests, and the release-qualification job that installs and drives a packaged release.

## Threat matrix

| Threat | Controls in the closed implementation | Regression evidence |
|---|---|---|
| T01 Public exposure of the local MCP server | No listener: MCP is stdio under the official outbound tunnel client; the only Quntal-observed socket is the tunnel client's loopback health endpoint, probed loopback-only | `health_listener_is_loopback_ephemeral_and_raw_http_logging_is_off`, `only_loopback_http_urls_are_accepted`, release qualification (MCP over stdio through the installed host) |
| T02 MCP caller escalates through tool metadata | Closed typed schemas; exactly 20 registered tools; kernel re-authorizes every request | `the MCP surface registers exactly the canonical closed tool set`, `SG-000015 MCP schemas accept only typed local Git mutation inputs`, `SG-000016 MCP schemas reject arbitrary URLs, refspecs, and credentials`, `SG-000017 MCP schemas reject arbitrary URLs, refspecs, force, and raw credentials`, `process.spawn schema exposes no raw command, env, stdin payload, or network widening`, release-qualification exact tool-set check |
| T03 Workspace escape through symlink/junction/reparse | Final-path containment; protected Quntal state excluded from every workspace; install tree refuses links before ACL reset | `rejects_parent_escape`, `denies_path_escape_traversal_device_unc_ads_and_reserved_destinations`, `sg000041_reparse_point_into_protected_state_is_denied_for_read_and_write`, `sg000041_refuses_workspace_equal_to_above_or_below_protected_state`, `junction_inside_install_root_is_refused_before_acl_reset` |
| T04 TOCTOU path swap | Expected-hash writes, stale HEAD/state checks, generation binding | `refuses_stale_existing_file`, `stale_head_and_unsafe_paths_fail_before_mutation`, `stale_generation_and_origin_fail_closed` |
| T05 Shell becomes universal bypass | No shell or generic PowerShell tool; Windows `process.spawn` limited to `whoami.exe` in a zero-capability AppContainer and kill-on-close job; protected state unreadable from the contained child | `process_spawn_rejects_escape_shell_network_stdin_env_and_unbounded_inputs`, `authorizes_only_bounded_argv_process_spawn`, `windows_restricted_child_cannot_read_quntal_protected_state` |
| T06 Tunnel credential exfiltration | Key by `file:` reference only; secret-like environment names dropped; key stored owner-only outside workspaces; tunnel output redacted | `launch_uses_file_reference_and_drops_secret_env`, `runtime_key_inside_workspace_is_rejected`, `child_environment_carries_workspaces_and_no_secrets`, `spaced_tabbed_pretty_and_later_assignments_are_redacted`, lifecycle E2E and release qualification assert the key appears in no output or log |
| T07 Agent self-approves via UI Automation | Quntal windows are protected surfaces; UIA tools are not exposed over MCP | `protected_quntal_surface_cannot_yield_frames_or_proposals`, `no UIA tool is registered on the MCP server surface` |
| T08 Agent self-approves via synthetic input | Input leases bound to the interruption epoch; Quntal-synthetic input distinguished; no input tool over MCP | `forged_synthetic_origin_claim_cannot_suppress_override`, `physical_click_key_and_foreground_change_each_revoke` |
| T09 Agent calls approval IPC directly | Approvals minted only by the local broker; forged attestations rejected; no MCP or child route to the broker | `forged_attestation_without_broker_token_cannot_authorize`, `forged_history_is_rejected`, `no MCP tool source reaches lifecycle, installer, update, trust, or approval surfaces` |
| T10 Approval replay | Nonce, expiry, one-shot consumption, digest binding | `ledger_consumes_once_then_rejects_replay`, `strong_consumes_once_then_rejects_replay`, `duplicate_consumption_is_replay_for_strong`, `nonces_are_fresh_unique_and_bound_to_digest` |
| T11 Target changed after approval | Digest over target state; drift fails closed | `ledger_rejects_expiry_mismatch_and_drift`, `upload_source_identity_is_one_shot_expiring_and_drift_checked` |
| T12 Browser SSRF / LAN pivot | Public-only resolution, address-set and peer binding, per-hop redirect revalidation | `navigation_preview_binds_origin_with_ssrf_denial`, `redirect_hops_apply_ssrf_policy_per_hop`, `redirect_widening_fails_closed`, SG-000040 fetch suite |
| T13 Browser credential abuse | Isolated profile; personal profile and credential shapes denied as STRONG | `isolated_profile_has_deterministic_identity_and_no_personal_data`, `denies_dom_download_upload_scripting_debugging_and_personal_shapes_as_strong` |
| T14 Page-provided tool injection | Page content never becomes tools; browser not exposed over MCP | `unknown_browser_shapes_are_unreachable_at_dispatch`, `no browser tool is registered on the MCP server surface` |
| T15 UI target confusion | Process/window identity binding; PID-reuse detection | `identity_detects_pid_reuse_shape_and_exit`, UIA stale-target suite |
| T16 Coordinate fallback widens authority | Proposals grant no input; no silent fallback; protected surfaces excluded | `proposals_grant_no_coordinate_or_input_authority`, `protected_quntal_surface_cannot_yield_proposals_or_coordinates` |
| T17 Human/agent input collision | Global interruption epoch revokes leases and pending approvals | `interruption_is_global_across_workspaces`, `physical_click_key_and_foreground_change_each_revoke` |
| T18 Clipboard secret leakage | Secret-pattern denial before return or placement; no clipboard MCP tool | `secret_and_empty_clipboard_fail_closed_in_dispatch`, `write_denies_empty_oversized_and_secret_text`, `no clipboard tool is registered on the MCP server surface` |
| T19 Process termination falsely reported | Termination reported only when observed (job quiescence, empty job, observed exits) | `windows_private_unverified_termination_never_promotes_timeout_or_output_limit`, `suspended_child_is_jobbed_before_resume_and_job_termination_empties_it`, lifecycle E2E asserting the `stopped_verified` outcome |
| T20 Output/log resource exhaustion | Bounded stdout/stderr, response, and log sizes with rotation | SG-000013 limit tests, `log_rotates_at_the_bound`, bounded redacted diagnostics tails in `quntal-lifecycle` |
| T21 Audit contains secrets | Redacted approval history; redacted lifecycle logs | `history_is_redacted_bounded_and_tamper_evident`, redaction suite |
| T22 Audit tampering | Checksum chains; doctor parse checks; owner-only state tree | `history_is_redacted_bounded_and_tamper_evident`, `corrupt_state_files_fail`, ACL verification tests |
| T23 Malicious update or dependency | Manifest SHA-256 verification at install, update, and every MCP session; downgrade refusal; automatic recovery; lockfiles; SBOM; provenance attestation; dependency audit | `tampered_release_installs_nothing`, `tampered_payload_fails_closed_for_each_session`, `downgrade_requires_explicit_flag`, `failed_self_check_restores_the_previous_version`, release qualification broken-release recovery |
| T24 Donor-code trust transfer | No donor code is adopted in the release; the unmerged donor plan (PR #115) is not canonical | Not applicable to this release (recorded) |
| T25 Quntal configuration tampering | Owner-only tree; schema-validated config; versions validated before use as paths; supervisor records restricted to installed images | `unsupported_schema_version_fails_closed`, `only_installed_version_cli_images_are_accepted`, `ids_are_validated` |
| T26 Secret storage misuse | Key outside every workspace and under the protected tree; never in environment | `runtime_key_inside_workspace_is_rejected`, `sg000041_state_defaults_lie_within_protected_state_roots` |
| T27 Local same-user compromise | Out of the application's control; mitigations are ACLs, AppContainer children, and STRONG presence | Residual risk (recorded below) |
| T28 Elevation bypass | No elevation requested; avoidable elevation refused; children contained | `elevated_old_windows_and_old_node_fail_closed`, `windows_appcontainer_child_is_job_assigned_before_resume` |

## Cross-capability compositions

| Composition | Why it matters | Evidence |
|---|---|---|
| MCP surface × every closed capability family | A tool added for one family must not expose another | exact canonical tool-set tests (Node and release qualification); per-family "no tool registered/forwarded" tests |
| MCP surface × lifecycle, trust, and approvals | Lifecycle and STRONG paths are human-only | `no MCP tool source reaches lifecycle, installer, update, trust, or approval surfaces` |
| Filesystem/Git/upload providers × protected Quntal state | Workspace tools must never read keys, trust, or approval history | SG-000041 suite; `windows_restricted_child_cannot_read_quntal_protected_state` |
| Approvals × workspaces × policy revisions | An approval for one target/workspace/revision must not authorize another | `cross_policy_reads_bind_their_own_revision`, digest/drift suites |
| SOFT × STRONG classes | A soft approval must never satisfy a strong operation | `weak_approval_cannot_satisfy_strong_expectation`, `strong_never_downgrades_to_soft` |
| Vision × coordinates × input leases × interruption | No structured denial may become coordinate input; human input revokes | P10 suites listed under T16/T17 |
| Clipboard write × input | Placement grants no paste or keyboard authority | SG-000039 suite |
| Updates × running runtime | No runtime starts on an unverified version | `start_is_refused_while_an_update_is_pending`, release qualification update-while-running |
| Tunnel output × logs × diagnostics | Tunnel or daemon output must not carry secrets into files | redaction suite; quntald diagnostics redacted per line before bounding |

## Specific checks requested for release

- Confused deputy: MCP tool set is closed; lifecycle, trust, and approval paths are human-only (above).
- Approval reuse, stale identity, TOCTOU: T10, T11, T04, T15 rows.
- Path traversal, reparse/junction escape, symlink races: T03 row.
- DNS rebinding, peer mismatch, redirect widening: T12 row and SG-000040.
- Secret leakage and child-environment leakage: T06, T18, T21 rows.
- Privilege boundary erosion: T05, T28 rows.
- Input lease reuse and visual retargeting: T16, T17 rows.
- Protected-surface bypass: T03, T07 rows.
- Installer/update tampering, rollback attacks, unsafe migration: T23, T25 rows (downgrade requires an explicit flag; configuration schema mismatch changes nothing; no automatic migration exists).
- MCP exposure drift: exact tool-set tests.

## Residual risks

- Same-user malicious code (T27) can read the user's own files and stop Quntal; Quntal's ACLs and AppContainer children reduce but cannot remove this.
- Real ChatGPT connectivity through the OpenAI tunnel, interactive SOFT/STRONG prompts, and SmartScreen behavior are not exercised in CI.
- Release binaries are unsigned; integrity relies on manifest hashes, SHA256SUMS, and the build-provenance attestation.
- `git_fetch`/`git_push` destination policies are not configurable through the lifecycle CLI, so those tools fail closed in the installed runtime.
