/**
 * SG-000073 frozen ComputerActionProposal vocabulary.
 *
 * Normative data only: the exact proposal action verbs, target reference
 * fields, and forbidden authority-bearing fields for the QDRAL-P18
 * computer-use program. This module carries no executable authority, opens
 * no transport, launches nothing, and is imported by no provider. The
 * provider-neutral proposal IR and its adapters arrive in SG-000083; until
 * then nothing in the runtime may act on these names.
 *
 * A proposal is untrusted input. Adapters may normalize provider syntax
 * into these shapes but can never create or override Deskal target IDs,
 * approvals, approval classes, trust, workspace authority, policy revision,
 * leases, execution results, or postcondition results.
 */

/** Frozen Deskal proposal action verbs (proposal-only, never executable). */
export const PROPOSAL_ACTION_VERBS = [
  "browser_navigate",
  "browser_click",
  "browser_fill",
  "browser_select",
  "browser_snapshot",
  "uia_invoke",
  "uia_set_value",
  "uia_select",
  "uia_toggle",
  "uia_scroll",
  "uia_screenshot",
  "coordinate_click",
  "coordinate_double_click",
  "coordinate_right_click",
  "coordinate_scroll",
  "coordinate_type"
] as const;

export type ProposalActionVerb = (typeof PROPOSAL_ACTION_VERBS)[number];

/** Fields a proposal target reference may carry. All values are opaque to
 * the caller: workspace, page, window, node, and generation bindings are
 * server-issued identities, never caller-minted. */
export const PROPOSAL_TARGET_FIELDS = [
  "target_kind",
  "workspace_id",
  "browser_profile_id",
  "page_id",
  "window_id",
  "node_id",
  "element_id",
  "page_generation",
  "document_generation",
  "window_generation",
  "capture_generation",
  "origin"
] as const;

export type ProposalTargetField = (typeof PROPOSAL_TARGET_FIELDS)[number];

/** Envelope fields a proposal may carry. */
export const PROPOSAL_ENVELOPE_FIELDS = [
  "action",
  "target",
  "parameters",
  "provider_hint"
] as const;

export type ProposalEnvelopeField = (typeof PROPOSAL_ENVELOPE_FIELDS)[number];

/**
 * Authority-bearing fields that must never appear in proposal data.
 * Adapters, providers, models, and remote callers cannot supply, mint, or
 * override these; Deskal binds them server-side after validation.
 */
export const FORBIDDEN_PROPOSAL_FIELDS = [
  "approval",
  "approval_class",
  "trust",
  "workspace_authority",
  "policy_revision",
  "capture_lease",
  "input_lease",
  "remote_lease",
  "execution_result",
  "postcondition_result"
] as const;

export type ForbiddenProposalField = (typeof FORBIDDEN_PROPOSAL_FIELDS)[number];

/** Every frozen verb, for exact-set regression tests. */
export function proposalActionVerbs(): readonly ProposalActionVerb[] {
  return PROPOSAL_ACTION_VERBS;
}

/** True only for exact frozen verb membership (unknown verbs fail closed). */
export function isProposalActionVerb(value: string): value is ProposalActionVerb {
  return (PROPOSAL_ACTION_VERBS as readonly string[]).includes(value);
}

/** True when a field name would smuggle authority into proposal data. */
export function isForbiddenProposalField(value: string): boolean {
  return (FORBIDDEN_PROPOSAL_FIELDS as readonly string[]).includes(value);
}
