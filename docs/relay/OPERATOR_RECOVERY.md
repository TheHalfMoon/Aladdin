# Deskal Relay and Recovery Operations

Status: OPERATOR GUIDE (SG-000072)
Date: 2026-10-04

This guide covers relay-state backup and restore, signing-key compromise
response, incident response, and recovery objectives for a self-hosted Deskal
relay and the paired computer. It creates no new authority: every procedure
below uses only mechanisms closed by earlier grains (pairing, revocation,
emergency revoke, checksum-guarded state, lifecycle install/update/rollback).

`qdral` below is the retained compatibility command. The product is Deskal.

## 1. Relay-state backup (canonical procedure)

1. Stop the relay (`docker stop qdral-relay`, or stop the Node.js process).
   Stopping first makes the copy deterministic; live copies are atomic files
   but stopping removes all doubt about which write won.
2. Copy the single state file. With the Docker layout from `SELF_HOSTING.md`:
   `cp /srv/qdral-relay/state/relay-state.json /srv/qdral-relay/backup/relay-state-$(date -u +%Y%m%dT%H%M%SZ).json`.
   There is exactly one file to copy; configuration (`relay.json`) is separate
   and should be backed up alongside it.
3. Set owner-only permissions on the copy (`chmod 600`). The file contains
   the relay's token signing key: anyone holding it can mint tokens for this
   relay. It contains no MCP payloads, tool arguments, results, bearer
   tokens, or device private keys -- but it is still secret material. Never
   commit it, never paste it into tickets or logs, never publish it.
4. Record which relay origin and software version the copy belongs to. A
   backup is valid only for the same `publicOrigin`; tokens bind issuer and
   audience to that origin.

## 2. Relay-state restore (canonical procedure)

1. Stop the relay.
2. Replace `stateDir/relay-state.json` with the backup copy (keep owner-only
   permissions).
3. Start the relay. Startup re-validates schema, checksum, size bound, and
   ceilings. A corrupt or tampered file refuses to start instead of silently
   forgetting revocations.
4. Verify before serving clients:
   - `GET /healthz` returns `{"ok":true}`;
   - a previously paired device reconnects and its token authenticates;
   - every revocation performed before the backup is still enforced
     (revoked tokens fail; revoked devices stay revoked);
   - client registrations present before the backup still resolve.
5. Re-apply any revocation issued after the backup was taken. A restore
   returns the relay to the backup's instant; revocations recorded later on
   the lost state are not part of the restored file. Treat post-backup
   revocations as outstanding work, not as preserved state.

Regression evidence (exact-head CI, `apps/qdral-relay/src/self-host.test.ts`):

- `relay backup restores device identities, client registrations, and revocations into a clean instance`
- `a tampered backup fails closed and a fresh state mints a new signing key`
- `relay state survives restart, keeps its signing key, and refuses corrupt state`
- `adversarial: relay and uplink logs carry classes only, never tokens, codes, identifiers, or payloads`

## 3. Signing-key compromise response

There is no automatic rotation and no silent trust of old state. Rotation is
an explicit operator act:

1. Stop the relay.
2. Delete `stateDir/relay-state.json` (keep a sealed forensic copy offline if
   investigation requires it, under the same owner-only handling).
3. Start the relay. It mints a fresh signing key on first start.
4. Re-pair every device (`qdral remote enable`, `qdral remote pair`) and
   re-register every client. Re-authorize routes with fresh local leases.

What becomes invalid at rotation, by mechanism rather than by promise:

- every previously minted access token (unknown signing key);
- every refresh token and refresh family (families live in the deleted state);
- every device pairing, client registration, route, and session;
- every recorded revocation (the slate is fresh; re-issue the revocations
  that must persist, then re-verify them as in section 2 step 4).

The fresh-state test proves old tokens fail against the new key; nothing in
the relay re-accepts the compromised key.

## 4. Incident-response runbook

Detect with `qdral doctor`, relay logs (classes only, never secrets), and the
client authorization pages. Contain first, then recover, then verify. Every
step below names the mechanism; none invents one.

1. Relay state corruption (relay refuses to start, checksum/size/schema
   error). Contain: keep the relay stopped so no stale authority is served.
   Recover: restore the newest known-good backup per section 2, or start
   fresh per section 3 if no good backup exists. Verify: section 2 step 4.
2. Relay state theft (backup or live file disclosed). Contain: treat the
   signing key as compromised and follow section 3 in full -- theft of the
   file is key compromise even if nothing looks abused yet. Recover: fresh
   key, re-pair, re-register. Verify: old tokens fail; only re-paired
   devices authenticate.
3. Signing-key compromise (suspected or confirmed). Same as theft: section 3
   in full. There is no partial rotation.
4. Device private-key compromise (one computer's key suspected). Contain:
   revoke that device (`qdral remote revoke` for its route, or relay-side
   device revocation) so its families die and its tokens stop verifying.
   Recover: generate a new device key on the computer (`qdral remote enable`
   again), re-pair, and authorize a fresh route. Other devices are
   unaffected: revocation is per-device. Verify: the old device's tokens
   fail; the new pairing works; no other device needed re-enrollment.
5. Suspected credential exposure (tunnel runtime key, loopback bearer token,
   pairing code, OAuth client secret where applicable). Contain: rotate the
   exposed credential at its source (new tunnel key file, new loopback
   token, expire the pairing flow, revoke the client registration) and stop
   the affected runtime. Recover: reconfigure with the new credential and
   restart. Verify: `qdral doctor` healthy, old credential rejected, logs
   contain no secret material.
6. Malicious remote principal (abusive paired client or hijacked provider
   account). Contain: revoke the route (`qdral remote revoke`) and, if the
   principal itself is hostile, revoke the principal/family so refresh and
   renewal stop. Recover: only after the account is clean, pair again under
   a new client registration with least scopes. Verify: revoked tokens and
   families fail; the new registration carries no widened scope.
7. Policy-state tampering on the computer (local trust/approval/audit state
   edited by hand or by malware). Contain: stop the runtime. The
   checksum-chained trust and approval history fails closed as untrusted
   rather than accepting forged state. Recover: restore the protected state
   from the operator's own backup, or purge and re-establish trust
   explicitly (re-add workspaces, re-grant trust with Windows Hello).
   Verify: `qdral doctor` healthy; forged history rejected; emergency revoke
   available.
8. Emergency revoke (suspected active abuse, no time for diagnosis).
   Execute `qdral emergency-revoke` (Windows Hello): it advances the epoch
   and invalidates pending approvals and pre-revoke tokens. Then work the
   matching scenario above. Verify: pre-revoke tokens fail; post-revoke
   approvals use the new epoch.
9. Recovery verification (every scenario ends here). Confirm: relay healthy,
   `qdral doctor` healthy on the computer, revocations re-applied and
   enforced, only intended devices paired, only intended routes authorized
   with finite leases, logs secret-free, and a fresh backup taken of the
   recovered state.

## 5. Recovery objectives

Deskal is self-hosted and local-first; there is no operated service behind
these numbers and no SLA is claimed. Objectives are operator targets backed
by the evidence named, or marked UNVERIFIED where unmeasured.

| Objective | Target | Evidence |
| --- | --- | --- |
| Relay restore to a known-good backup | Operator-paced; the mechanism (stop, copy one file, start, verify) completes in minutes on the operator's own hardware | Restore and tamper tests above; UNVERIFIED as a measured wall-clock value across operator environments |
| Relay recovery point (RPO) | The instant the relay was stopped for backup; no transaction newer than the copied file survives a restore | Single atomic state file; UNVERIFIED beyond the file's own integrity guarantee |
| Signing-key rotation | Complete invalidation of old tokens, families, pairings, and routes on fresh start | Fresh-state test above |
| Computer runtime recovery (failed update) | Automatic restore of the previous version with the failure recorded | `failed_self_check_restores_the_previous_version`, `rollback_recovers_a_crash_between_pointer_and_install_record`, and the packaged release-qualification suite in exact-head CI |
| Computer update 0.1.0 to 0.2.0 | Clean transition with the previous version retained for rollback | Release-qualification suite installs a 0.1.0-versioned image and updates it to the candidate in exact-head CI |
| Uninstall/purge | Programs removed; user data retained until explicit purge | Release-qualification suite in exact-head CI |
| Remote-session continuity across relay recovery | None promised: sessions and leases do not survive relay restart by design; clients reconnect and the computer re-authorizes | `adversarial: relay restart drops sessions, devices reconnect, and nothing replays or extends` |
| Log/secret safety during incidents | No secret material in relay, tunnel, supervisor, lifecycle, or MCP outputs at any stage | Redaction and secret-free test families in sections 1-4 of the v0.2 threat regression |
