import assert from "node:assert/strict";
import test from "node:test";
import {
  FORBIDDEN_PROPOSAL_FIELDS,
  PROPOSAL_ACTION_VERBS,
  PROPOSAL_ENVELOPE_FIELDS,
  PROPOSAL_TARGET_FIELDS,
  isForbiddenProposalField,
  isProposalActionVerb
} from "./computer_action_proposal.js";

test("SG-000073: the proposal verb set is exactly the frozen sixteen", () => {
  assert.deepEqual([...PROPOSAL_ACTION_VERBS], [
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
  ]);
});

test("SG-000073: unknown proposal verbs fail closed", () => {
  for (const verb of PROPOSAL_ACTION_VERBS) {
    assert.equal(isProposalActionVerb(verb), true);
  }
  for (const unknown of [
    "",
    "click",
    "type",
    "drag",
    "browser_evaluate",
    "run_command",
    "shell",
    "BROWSER_CLICK",
    "browser_click "
  ]) {
    assert.equal(isProposalActionVerb(unknown), false, unknown);
  }
});

test("SG-000073: target and envelope fields are exactly the frozen sets", () => {
  assert.deepEqual([...PROPOSAL_TARGET_FIELDS], [
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
  ]);
  assert.deepEqual([...PROPOSAL_ENVELOPE_FIELDS], [
    "action",
    "target",
    "parameters",
    "provider_hint"
  ]);
});

test("SG-000073: authority-bearing fields are all forbidden in proposal data", () => {
  assert.deepEqual([...FORBIDDEN_PROPOSAL_FIELDS], [
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
  ]);
  for (const field of FORBIDDEN_PROPOSAL_FIELDS) {
    assert.equal(isForbiddenProposalField(field), true);
    assert.ok(
      !(PROPOSAL_TARGET_FIELDS as readonly string[]).includes(field),
      `${field} must not appear as a target field`
    );
    assert.ok(
      !(PROPOSAL_ENVELOPE_FIELDS as readonly string[]).includes(field),
      `${field} must not appear as an envelope field`
    );
  }
  for (const allowed of ["action", "target_kind", "node_id", "origin", "provider_hint"]) {
    assert.equal(isForbiddenProposalField(allowed), false, allowed);
  }
});

test("SG-000073: the frozen vocabulary is data only with no provider imports", () => {
  assert.equal(PROPOSAL_ACTION_VERBS.length, 16);
  assert.equal(PROPOSAL_TARGET_FIELDS.length, 12);
  assert.equal(PROPOSAL_ENVELOPE_FIELDS.length, 4);
  assert.equal(FORBIDDEN_PROPOSAL_FIELDS.length, 10);
});
