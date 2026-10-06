# Deskal Canonical Planning Index

Status: ACTIVE NORMATIVE
Date: 2026-10-04

This index fixes planning precedence so no implementation agent can treat an
obsolete Cotra-era or Qdral-era architectural statement as current authority.
Historical evidence is preserved verbatim; preservation is not authorization.

## 1. Product identity rule

The maintained product identity is Deskal. Qdral and Cotra are
superseded product identities and must not be presented as the current product
(the rename records in `docs/identity/` govern the full earlier history).
The compatibility boundary (CLI `qdral`, daemon `qdrald`, npm packages
`@qdral/mcp` and `@qdral/relay`, `QDRAL_*` environment variables, `qdral.*`
OAuth scopes, `qdral-*` source paths, install/storage paths containing
`Qdral`/`qdral`, and the `qdral` MCP compatibility name) is governed by
`docs/identity/DESKAL_RENAME.md` and is never renamed blindly.

"QDRAL-P18 is a retained compatibility/program identifier within the Deskal
product; it is not the current product brand."

COTRA-P16, COTRA-P17, and QDRAL-P18 name governed programs, not the product.
The word "Qdral" surviving inside `QDRAL_*` document titles and program text
names that retained program/compatibility namespace, not the product brand.

## 2. Planning precedence

Conflicts resolve in this order; lower levels never override higher levels:

1. `docs/canonical/CURRENT.md` (as corrected by the identity notice at its top)
2. the single active SpecGrain (exactly one `GRAIN`-state spec at a time)
3. the active program plan (`QDRAL_COMPUTER_USE_PLAN.md` once P18 activates)
4. the current architecture baseline (closed-grain design records)
5. supporting normative matrices, contracts, and qualification records
6. historical and superseded plans (evidence only, never authority)

One active SpecGrain at a time unless canonical governance explicitly permits
otherwise. A successor grain activates only after its predecessor closes
canonically.

## 3. Document classification

- `ACTIVE_NORMATIVE`: current binding authority.
- `ACTIVE_SUPPORTING`: current supporting input; binding only through the
  normative level that cites it.
- `COMPATIBILITY_CONTRACT`: frozen interoperation surface; changes are breaking
  migrations, not edits.
- `HISTORICAL`: immutable evidence of a past state; never current authority.
- `SUPERSEDED`: must not be followed; retained only until an explicit migration
  removes it.

### ACTIVE_NORMATIVE

| Document | Authority |
| --- | --- |
| `docs/canonical/CURRENT.md` | Canonical frontier record. Only the corrected identity notice at its top and the precedence in this index speak for the present; dated ledger entries below the historical marker are evidence. |
| `docs/canonical/PLANNING_INDEX.md` | This precedence and classification index. |
| `docs/identity/DESKAL_RENAME.md` | Product identity and compatibility boundary. |
| `docs/governance/DIFFCIPLINE.md` | Exact-diff execution discipline. |
| `.specgrain/specs/SG-000087.json` | The single active grain (DESKAL-P19 public website foundation and launch surface). |
| `docs/canonical/DESKAL_POST_P18_LAUNCH_PLAN.md` | Active DESKAL-P19 program plan for post-P18 launch completeness. |

### ACTIVE_SUPPORTING

| Document | Role |
| --- | --- |
| `docs/canonical/QDRAL_COMPUTER_USE_PLAN.md` | Closed-program design record for QDRAL-P18. It remains supporting evidence for the delivered computer-use authority boundary but authorizes no new implementation. |
| `docs/canonical/QDRAL_COMPUTER_USE_STATUS.md`, `..._ACTIVATION_GATE.md`, `..._IMPLEMENTATION_ORDER.md`, `..._GOVERNANCE_NOTE.md`, `..._SCOPE.md`, `..._README.md` | Program sequencing and activation state. |
| `docs/canonical/QDRAL_COMPUTER_USE_DECISIONS.md`, `..._AUTHORITY_MODEL.md`, `..._EXECUTION_RULE.md`, `..._NO_FALLBACK.md`, `..._LOCAL_FIRST.md`, `..._ZERO_COST.md`, `..._SECURITY_DENIALS.md` | Planned authority boundaries; normative for P18 grains once activated. |
| `docs/canonical/QDRAL_COMPUTER_USE_ACCEPTANCE_MATRIX.md`, `..._TEST_MATRIX.md`, `..._FAILURE_SEMANTICS.md`, `..._RESOURCE_BOUNDS.md`, `..._PROFILE_MODEL.md`, `..._LICENSE_POLICY.md`, `..._DONOR_MATRIX.md`, `..._REVIEW_GATE.md` | Planned qualification contracts; normative for P18 grains once activated. |
| `docs/canonical/P12_INSTALLER_LIFECYCLE_DESIGN.md` | Delivered COTRA-P12 design basis for the shipped installer/lifecycle. `COTRA-P12` naming is retained program history. |
| `docs/security/THREAT_MODEL.md`, `docs/security/THREAT_MODEL_REGRESSION_V02.md` | Living threat baseline and the live v0.2 regression. |
| `docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md` | Connectivity threat analysis supporting P17 distribution work. |
| `docs/security/SG-*.md` per-grain notes for closed grains | Design records of closed authority; read with the grain's evidence entry. |
| `docs/security/CLIENT_CONNECTION_PROFILE_MODEL.md`, `..._REMOTE_PRINCIPAL_AUTH_MODEL.md`, `..._REMOTE_SESSION_AUTHORIZATION.md`, `..._RELEASE_SECURITY_REVIEW.md` | Authentication, session, and release review models. |
| `docs/p16/CAPABILITY_PARITY.md`, `..._TOOL_CONTRACT.md`, `..._BROWSER_QUALIFICATION.md`, `..._DESKTOP_QUALIFICATION.md`, `..._capability_parity_inventory.json` | Capability inventory, tool contract, and pre-P18 qualification records. |
| `docs/p17/OPENAI_PLUGIN.md`, `..._CLAUDE.md`, `..._MISTRAL.md`, `..._CODEX_AND_GENERIC_MCP.md` | Provider distribution records for the P17 sequence. |
| `docs/relay/SELF_HOSTING.md`, `..._REFERENCE_DEPLOYMENT.md`, `..._OPERATOR_RECOVERY.md` | Operator deployment, backup/restore, compromise response, runbook, and recovery objectives. |
| `docs/legal/PRIVACY.md`, `..._TERMS.md` | Legal notices. |

### COMPATIBILITY_CONTRACT

| Document | Contract |
| --- | --- |
| `docs/security/RELAY_PROTOCOL_CONTRACT.md` | Frozen relay protocol vocabulary. |
| Code surfaces listed in `docs/identity/DESKAL_RENAME.md` | CLI, daemon, npm, env, scope, path, and MCP-name compatibility identifiers. |

### HISTORICAL

| Document | Why historical |
| --- | --- |
| `docs/canonical/ARCHITECTURE_AND_DELIVERY_PLAN.md` | Delivered through COTRA-P13. Its Cotra-era product statements are superseded; the current state is recorded in `CURRENT.md`. |
| `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md` | Proposal whose provider-neutral decisions were absorbed by the closed P17 grains; its `Cotra v0.2.x` product naming is superseded. |
| `docs/identity/` rename records (`DESKAL_RENAME.md`, `QDRAL_RENAME.md`, and the earlier record beside them) | Historical rename records; immutable provenance. |
| `docs/security/THREAT_MODEL_REGRESSION.md` | Point-in-time QDRAL-P13 regression evidence with P13-era counts; superseded for v0.2 by `THREAT_MODEL_REGRESSION_V02.md`. |
| `docs/security/RELEASE_SECURITY_REVIEW.md` | SG-000046 program-level review for the P13 release line; superseded for v0.2 by the SG-000072 exit record. |
| `docs/p16/sg000066_exit_evidence.json` | Closed P16 exit evidence. |
| `.specgrain/specs/SG-000001.json` through `SG-000071.json` and `.specgrain/canonical-evidence.json` | Closed-grain provenance; machine-readable evidence ledger. |
| Dated `CURRENT.md` ledger entries below the historical marker | Past frontiers recorded verbatim. |

### SUPERSEDED

No SUPERSEDED document remains. A document that presents Qdral or Cotra as the current product, or that contradicts the precedence above, must
be reclassified here or corrected before it is followed.

## 4. Rules for implementation agents

1. Never treat a HISTORICAL plan statement (notably any `Cotra is` / `Qdral is`
   product sentence in the documents above) as permission to build, expose, or
   widen authority.
2. Never rename a compatibility identifier on the basis of this index; rename
   migrations are separately governed breaking changes.
3. Never edit HISTORICAL evidence to modernize names; fix drift only in active
   documents and record the correction in the governing grain.
