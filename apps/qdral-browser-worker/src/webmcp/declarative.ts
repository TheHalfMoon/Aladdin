// SG-000108 slice 4: read-only discovery of WebMCP declarative form tools.
//
// Adapted from MCP-B `packages/webmcp-polyfill/src/declarative-forms.ts`
// (WebMCP-org/npm-packages 7db816b5a280c2d1d4b86559696e993c497c021a),
// MIT License, Copyright (c) 2025 mcp-b contributors; the full notice is in
// docs/aladdin/notices/WebMCP-org-npm-packages-MIT.txt and the import record
// in docs/aladdin/sg000108_webmcp_provenance.json. Modifications for Aladdin:
// - Works on an attribute-only DOM model taken from a CDP DOMSnapshot, so
//   no page script runs and nothing in the page changes (the donor reads the
//   live DOM, registers tools and listens for submits).
// - Read only: no registration, listeners, submission or mutation.
// - Never emits a control's value (only author-declared choice values for
//   radio, checkbox and option elements); password controls are omitted and
//   flag the tool instead.
// - Every string and collection is bounded; anything over a bound truncates
//   the record set or drops the field rather than growing. Strings are made
//   well formed and free of controls and bidi marks; values that must cross
//   unchanged (names, choices, patterns) are refused instead.
// - One linear tree pass under a CPU budget replaces the donor's per-form
//   DOM queries. Shadow roots are separate tree scopes (no form ownership,
//   labels, ids or disabled fieldsets across them); a wrapping label labels
//   only its first labelable descendant; a disabled fieldset spares its
//   first legend.
// - Number schemas carry no `pattern`, and contradictory bounds are dropped.
// - Editable regions (contenteditable, role=textbox) never contribute text.
// - Names shaped like host identities (`d<n>-...`) are not tools.
//
// The output is untrusted page data: the host validates and labels it.

export interface DomElement {
  readonly kind: "element";
  /// Lowercase tag name.
  readonly tag: string;
  readonly attrs: ReadonlyMap<string, string>;
  readonly children: readonly DomNode[];
  readonly parent: DomElement | null;
  /// The engine's backend node id (stable for the document's lifetime).
  readonly node: number;
}

export interface DomText {
  readonly kind: "text";
  readonly text: string;
  readonly parent: DomElement | null;
}

export type DomNode = DomElement | DomText;

export interface ParamSchema {
  type: "string" | "number" | "boolean" | "array";
  description?: string;
  enum?: string[];
  minimum?: number;
  maximum?: number;
  multiple_of?: number;
  pattern?: string;
  format?: string;
  /// For arrays: the allowed item values (unique items).
  items_enum?: string[];
}

export interface DeclaredParam extends ParamSchema {
  name: string;
  required: boolean;
}

export interface DeclaredTool {
  name: string;
  title: string;
  description: string;
  autosubmit: boolean;
  /// Backend node id of the form.
  node: number;
  /// The form carries a password control (omitted from `params`).
  has_password: boolean;
  params: DeclaredParam[];
}

export const LIMITS = {
  tools: 32,
  name: 64,
  title: 128,
  description: 1024,
  params: 32,
  paramDescription: 512,
  choices: 64,
  choice: 256,
  pattern: 256
} as const;

const NAME = /^[A-Za-z0-9_.-]{1,64}$/;
/// Names shaped like host identities (`d3-e12`, `d1-w1`) are reserved.
const HOST_IDENTITY_SHAPE = /^d\d+-/;
const TEXT_INPUT_TYPES = new Set(["email", "password", "search", "tel", "text", "url"]);
const READONLY_INPUT_TYPES = new Set([...TEXT_INPUT_TYPES, "date", "datetime-local", "month", "number", "time", "week"]);
const CONTROL_TAGS = new Set(["input", "select", "textarea"]);
/// Elements a `label` can label (HTML "labelable", hidden inputs excluded).
const LABELABLE = new Set(["button", "input", "meter", "output", "progress", "select", "textarea"]);
const PASSWORD_AUTOCOMPLETE = new Set(["current-password", "new-password", "one-time-code"]);
/// The worker's CPU budget for one discovery, well inside the host's
/// observation deadline; past it the result is truncated, never late.
export const DISCOVERY_BUDGET_MS = 4_000;

// C0/C1 controls (whitespace is collapsed to spaces first).
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;
const CONTROLS = /[\u0000-\u001f\u007f-\u009f]/g;
// Bidi controls, zero-width marks and the BOM: they make text read
// differently from what it is.
const BIDI = /[؜​-‏‪-‮⁠-⁤⁦-⁯﻿]/;
const BIDIS = /[؜​-‏‪-‮⁠-⁤⁦-⁯﻿]/g;
const LONE_SURROGATES = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/g;

/// Replace lone surrogates (they cannot cross the frame as valid JSON for
/// the host) with U+FFFD.
function wellFormedText(text: string): string {
  return text.replace(LONE_SURROGATES, "�");
}

/// No lone surrogates.
function wellFormed(text: string): boolean {
  return !new RegExp(LONE_SURROGATES.source).test(text);
}

/// Make well formed, collapse whitespace, drop controls and bidi marks, and
/// cut at `max` code points (never inside a surrogate pair).
function bound(text: string, max: number): string {
  const clean = wellFormedText(text).replace(/\s+/g, " ").replace(CONTROLS, "").replace(BIDIS, "").trim();
  const points = Array.from(clean);
  return points.length > max ? points.slice(0, max).join("") : clean;
}

function codePoints(text: string): number {
  return Array.from(text).length;
}

/// A value that must cross unchanged (a name, a choice, a pattern): bounded,
/// well formed, and free of controls and bidi marks, or refused.
function exactValue(text: string, max: number): boolean {
  return codePoints(text) <= max && !CONTROL.test(text) && !BIDI.test(text) && wellFormed(text);
}

function attr(element: DomElement, name: string): string | undefined {
  return element.attrs.get(name);
}

function inputType(element: DomElement): string {
  const type = (attr(element, "type") ?? "text").toLowerCase();
  return type === "" ? "text" : type;
}

function isInput(control: DomElement, type: string): boolean {
  return control.tag === "input" && inputType(control) === type;
}

function isLabelable(element: DomElement): boolean {
  return LABELABLE.has(element.tag) && !isInput(element, "hidden");
}

const SKIPPED_IN_LABELS = new Set(["button", "input", "meter", "output", "progress", "select", "textarea"]);
/// Bounds on the text gathered for one label or option before it is cut.
const MAX_TEXT_GATHERED = 4096;
const MAX_TEXT_NODES_VISITED = 4096;

/// Editable regions hold what the user typed; their text never crosses.
function isEditable(element: DomElement): boolean {
  const editable = attr(element, "contenteditable");
  if (editable !== undefined && editable.toLowerCase() !== "false") return true;
  return (attr(element, "role") ?? "").toLowerCase() === "textbox";
}

/// The text below `element` in tree order: iterative, bounded in nodes
/// visited and characters gathered, skipping controls (for labels),
/// editable regions and shadow roots.
function textOf(element: DomElement, skipControls: boolean): string {
  const parts: string[] = [];
  let gathered = 0;
  let visited = 0;
  const stack: DomNode[] = [...element.children].reverse();
  for (let node = stack.pop(); node && gathered < MAX_TEXT_GATHERED && visited < MAX_TEXT_NODES_VISITED; node = stack.pop()) {
    visited++;
    if (node.kind === "text") {
      parts.push(node.text);
      gathered += node.text.length + 1;
      continue;
    }
    if (node.tag === "#shadow-root" || isEditable(node)) continue;
    if (skipControls && SKIPPED_IN_LABELS.has(node.tag)) continue;
    for (let i = node.children.length - 1; i >= 0; i--) {
      const child = node.children[i];
      if (child) stack.push(child);
    }
  }
  return parts.join(" ");
}

/// What the single tree pass learns about one control.
interface ControlFacts {
  element: DomElement;
  /// The owner form (nearest form ancestor in its tree scope, or the form a
  /// `form` attribute names in that scope).
  owner: DomElement | null;
  disabled: boolean;
  /// Labels: the wrapping label (if this is its first labelable
  /// descendant) and any `label for=` naming this control.
  labels: DomElement[];
  /// Nearest fieldset ancestors, innermost first (bounded).
  fieldsets: DomElement[];
}

interface Frame {
  node: DomNode;
  form: DomElement | null;
  disabled: boolean;
  label: { element: DomElement; used: boolean } | null;
  select: DomElement | null;
  fieldsets: DomElement[];
  scope: number;
}

/// Nearest fieldsets remembered per control.
const MAX_FIELDSETS = 8;

interface Pass {
  forms: DomElement[];
  controls: ControlFacts[];
  options: Map<DomElement, DomElement[]>;
  truncated: boolean;
}

/// One iterative pre-order pass over the model (linear in its size). Shadow
/// roots (`#shadow-root`) start a new tree scope: form ownership, labels,
/// ids and disabled fieldsets do not cross them.
function treePass(root: DomElement, deadline: number): Pass {
  const forms: DomElement[] = [];
  const controls: ControlFacts[] = [];
  const options = new Map<DomElement, DomElement[]>();
  const byId = new Map<string, DomElement>();
  const formAttr = new Map<ControlFacts, { scope: number; id: string }>();
  const labelFor: Array<{ label: DomElement; scope: number; id: string }> = [];
  const factsOf = new Map<DomElement, ControlFacts>();
  let scopes = 0;
  let truncated = false;
  let steps = 0;
  const stack: Frame[] = [{ node: root, form: null, disabled: false, label: null, select: null, fieldsets: [], scope: 0 }];
  for (let frame = stack.pop(); frame; frame = stack.pop()) {
    if ((++steps & 1023) === 0 && Date.now() > deadline) {
      truncated = true;
      break;
    }
    const { node } = frame;
    if (node.kind !== "element") continue;
    let { form, disabled, label, select, fieldsets, scope } = frame;
    if (node.tag === "#shadow-root") {
      scope = ++scopes;
      form = null;
      disabled = false;
      label = null;
      select = null;
      fieldsets = [];
    } else {
      const id = attr(node, "id");
      if (id) {
        const key = `${scope}#${id}`;
        if (!byId.has(key)) byId.set(key, node);
      }
      if (node.tag === "form") forms.push(node);
      if (node.tag === "option" && select) options.get(select)?.push(node);
      if (node.tag === "label") {
        const target = attr(node, "for");
        if (target !== undefined) labelFor.push({ label: node, scope, id: target });
      }
      if (CONTROL_TAGS.has(node.tag)) {
        const facts: ControlFacts = {
          element: node,
          owner: form,
          disabled: disabled || node.attrs.has("disabled"),
          labels: [],
          fieldsets
        };
        const named = attr(node, "form");
        if (named !== undefined) formAttr.set(facts, { scope, id: named });
        controls.push(facts);
        factsOf.set(node, facts);
      }
      if (label && !label.used && isLabelable(node)) {
        label.used = true;
        factsOf.get(node)?.labels.push(label.element);
      }
    }
    // What the children inherit.
    const childForm = node.tag === "form" ? node : form;
    const childLabel = node.tag === "label" && !node.attrs.has("for") ? { element: node, used: false } : label;
    const childSelect = node.tag === "select" ? node : select;
    if (node.tag === "select") options.set(node, []);
    const childFieldsets = node.tag === "fieldset" ? [node, ...fieldsets].slice(0, MAX_FIELDSETS) : fieldsets;
    const disables = node.tag === "fieldset" && node.attrs.has("disabled");
    // A disabled fieldset's first legend child stays enabled.
    const legend = disables ? node.children.find((child) => child.kind === "element" && child.tag === "legend") : undefined;
    for (let i = node.children.length - 1; i >= 0; i--) {
      const child = node.children[i];
      if (!child) continue;
      stack.push({
        node: child,
        form: childForm,
        disabled: disables && child !== legend ? true : disabled,
        label: childLabel,
        select: childSelect,
        fieldsets: childFieldsets,
        scope
      });
    }
  }
  // `form=` names a form by id in the control's own scope; any other value
  // leaves the control without an owner.
  for (const [facts, { scope, id }] of formAttr) {
    const target = byId.get(`${scope}#${id}`);
    facts.owner = target && target.tag === "form" ? target : null;
  }
  for (const { label, scope, id } of labelFor) {
    const target = byId.get(`${scope}#${id}`);
    const facts = target && isLabelable(target) ? factsOf.get(target) : undefined;
    facts?.labels.push(label);
  }
  return { forms, controls, options, truncated };
}

function isReadOnly(control: DomElement): boolean {
  if (!control.attrs.has("readonly")) return false;
  return control.tag === "textarea" || (control.tag === "input" && READONLY_INPUT_TYPES.has(inputType(control)));
}

function isPasswordLike(control: DomElement): boolean {
  if (isInput(control, "password")) return true;
  const autocomplete = (attr(control, "autocomplete") ?? "").toLowerCase().split(/\s+/);
  return autocomplete.some((token) => PASSWORD_AUTOCOMPLETE.has(token));
}

function labelText(facts: ControlFacts): string {
  return facts.labels
    .map((label) => bound(textOf(label, true), LIMITS.paramDescription))
    .filter(Boolean)
    .join("; ");
}

function commonFieldset(group: readonly ControlFacts[]): DomElement | undefined {
  const [first] = group;
  for (const fieldset of first?.fieldsets ?? []) {
    if (group.every((facts) => facts.fieldsets.includes(fieldset))) return fieldset;
  }
  return undefined;
}

function parameterDescription(group: readonly ControlFacts[]): string | undefined {
  if (group.length === 1) {
    const facts = group[0];
    if (!facts) return undefined;
    const control = facts.element;
    return attr(control, "toolparamdescription") || labelText(facts) || attr(control, "aria-description") || undefined;
  }
  const fieldset = commonFieldset(group);
  return (fieldset && attr(fieldset, "toolparamdescription")) || undefined;
}

function finiteNumber(raw: string | undefined): number | undefined {
  if (raw === undefined || raw.trim() === "") return undefined;
  const value = Number(raw);
  return Number.isFinite(value) ? value : undefined;
}

function validStep(input: DomElement, fallback: number): number {
  const step = Number(attr(input, "step"));
  return Number.isFinite(step) && step > 0 ? step : fallback;
}

function isStepBaseMultiple(stepBase: number, step: number): boolean {
  const quotient = stepBase / step;
  return Math.abs(quotient - Math.round(quotient)) < Number.EPSILON * 16;
}

function withPattern(schema: ParamSchema, input: DomElement): ParamSchema {
  const pattern = attr(input, "pattern");
  if (pattern === undefined || !exactValue(pattern, LIMITS.pattern)) return schema;
  try {
    new RegExp(pattern, "v");
  } catch {
    return schema;
  }
  return { ...schema, pattern };
}

/// A number schema. Bounds that contradict each other (minimum above
/// maximum) are dropped rather than sent: the host refuses them. `pattern`
/// does not apply to numbers (the donor carries it; the host's subset does
/// not).
function numberSchema(input: DomElement, defaults?: { minimum: number; maximum: number }): ParamSchema {
  const schema: ParamSchema = { type: "number" };
  let minimum = finiteNumber(attr(input, "min"));
  let maximum = finiteNumber(attr(input, "max"));
  if (defaults) {
    minimum ??= defaults.minimum;
    maximum ??= defaults.maximum;
  }
  if (minimum !== undefined && maximum !== undefined && minimum > maximum) {
    minimum = undefined;
    maximum = undefined;
  }
  if (minimum !== undefined) schema.minimum = minimum;
  if (maximum !== undefined) schema.maximum = maximum;
  if (attr(input, "step") !== "any") {
    const step = validStep(input, 1);
    // The step base is the minimum, else the author's default value, else 0.
    const base = finiteNumber(attr(input, "value"));
    const stepBase = minimum ?? base ?? 0;
    if (isStepBaseMultiple(stepBase, step)) schema.multiple_of = step;
  }
  return schema;
}

function temporalFormat(input: DomElement, datePrefix: string): string {
  const step = validStep(input, 60);
  if (step < 1) return `${datePrefix}(:[0-5][0-9](\\.[0-9]{1,3})?)?$`;
  if (step < 60) return `${datePrefix}(:[0-5][0-9])?$`;
  return `${datePrefix}$`;
}

function inputSchema(input: DomElement): ParamSchema | undefined {
  const type = inputType(input);
  if (TEXT_INPUT_TYPES.has(type)) return withPattern({ type: "string" }, input);
  switch (type) {
    case "hidden":
      return attr(input, "toolparamdescription") ? { type: "string" } : undefined;
    case "number":
      return numberSchema(input);
    case "range":
      return numberSchema(input, { minimum: 0, maximum: 100 });
    case "checkbox":
      return { type: "boolean" };
    case "date":
      return { type: "string", format: "date" };
    case "month":
      return { type: "string", format: "^[0-9]{4}-(0[1-9]|1[0-2])$" };
    case "week":
      return { type: "string", format: "^[0-9]{4}-W(0[1-9]|[1-4][0-9]|5[0-3])$" };
    case "time":
      return { type: "string", format: temporalFormat(input, "^([01][0-9]|2[0-3]):[0-5][0-9]") };
    case "datetime-local":
      return { type: "string", format: temporalFormat(input, "^[0-9]{4}-(0[1-9]|1[0-2])-[0-9]{2}T([01][0-9]|2[0-3]):[0-5][0-9]") };
    case "color":
      return { type: "string", format: "^#[0-9a-zA-Z]{6}$" };
    default:
      return undefined;
  }
}

/// Author-declared choice values, unchanged; undefined if any value cannot
/// cross exactly or there are too many (the parameter is then dropped).
function choices(values: readonly string[]): string[] | undefined {
  if (values.length > LIMITS.choices) return undefined;
  if (!values.every((value) => exactValue(value, LIMITS.choice))) return undefined;
  return [...values];
}

function optionValues(options: readonly DomElement[]): string[] {
  // An option without a value attribute submits its text.
  return options.map((option) => attr(option, "value") ?? bound(textOf(option, false), LIMITS.choice));
}

function parameterSchema(group: readonly ControlFacts[], options: ReadonlyMap<DomElement, DomElement[]>): ParamSchema | undefined {
  const controls = group.map((facts) => facts.element);
  const [first] = controls;
  if (!first) return undefined;
  let schema: ParamSchema | undefined;
  if (controls.every((control) => isInput(control, "radio"))) {
    const values = choices(controls.map((control) => attr(control, "value") ?? "on"));
    schema = values && { type: "string", enum: values };
  } else if (controls.length > 1) {
    if (controls.every((control) => isInput(control, "checkbox"))) {
      const values = choices(controls.map((control) => attr(control, "value") ?? "on"));
      schema = values && { type: "array", items_enum: values };
    }
  } else if (first.tag === "textarea") {
    schema = { type: "string" };
  } else if (first.tag === "select") {
    const values = choices(optionValues(options.get(first) ?? []));
    schema = values && (first.attrs.has("multiple") ? { type: "array", items_enum: values } : { type: "string", enum: values });
  } else {
    schema = inputSchema(first);
  }
  if (!schema) return undefined;
  const description = parameterDescription(group);
  const dateHint = isInput(first, "date") ? "Dates MUST be provided in 'YYYY-MM-DD' format." : undefined;
  const combined = description && dateHint ? `${description} (${dateHint})` : description || dateHint;
  return combined ? { ...schema, description: bound(combined, LIMITS.paramDescription) } : schema;
}

export interface Discovery {
  tools: DeclaredTool[];
  /// Some declarations were dropped by a bound or the CPU budget.
  truncated: boolean;
}

/// Every declarative tool in the document, in tree order. A form needs a
/// valid `toolname` and a `tooldescription`; the first form with a name wins
/// and later duplicates are dropped (as in the donor). Linear in the size of
/// the model, and bounded by `budgetMs` of CPU time.
export function discoverDeclarativeTools(root: DomElement, budgetMs: number = DISCOVERY_BUDGET_MS): Discovery {
  const deadline = Date.now() + budgetMs;
  const pass = treePass(root, deadline);
  let truncated = pass.truncated;
  const owned = new Map<DomElement, ControlFacts[]>();
  for (const facts of pass.controls) {
    if (!facts.owner) continue;
    const list = owned.get(facts.owner);
    if (list) list.push(facts);
    else owned.set(facts.owner, [facts]);
  }
  const tools: DeclaredTool[] = [];
  const names = new Set<string>();
  for (const form of pass.forms) {
    if (Date.now() > deadline) {
      truncated = true;
      break;
    }
    const name = attr(form, "toolname");
    const description = attr(form, "tooldescription");
    if (name === undefined || description === undefined) continue;
    const valid = NAME.test(name) && !HOST_IDENTITY_SHAPE.test(name);
    if (!valid || names.has(name)) {
      truncated ||= !valid;
      continue;
    }
    if (tools.length >= LIMITS.tools) {
      truncated = true;
      break;
    }
    names.add(name);
    const controls = owned.get(form) ?? [];
    // Password-like controls flag the tool whether or not they are enabled,
    // and never become parameters.
    const hasPassword = controls.some((facts) => isPasswordLike(facts.element));
    const groups = new Map<string, ControlFacts[]>();
    for (const facts of controls) {
      const control = facts.element;
      if (isPasswordLike(control) || facts.disabled || isReadOnly(control)) continue;
      const key = (attr(control, "name") ?? "").trim();
      if (!key) continue;
      const group = groups.get(key);
      if (group) group.push(facts);
      else groups.set(key, [facts]);
    }
    const params: DeclaredParam[] = [];
    for (const [key, group] of groups) {
      if (!exactValue(key, LIMITS.name)) {
        truncated = true;
        continue;
      }
      if (params.length >= LIMITS.params) {
        truncated = true;
        break;
      }
      const schema = parameterSchema(group, pass.options);
      if (!schema) continue;
      params.push({ name: key, required: group.some((facts) => facts.element.attrs.has("required")), ...schema });
    }
    if (codePoints(description) > LIMITS.description) truncated = true;
    tools.push({
      name,
      title: bound(attr(form, "tooltitle") ?? "", LIMITS.title),
      description: bound(description, LIMITS.description),
      autosubmit: form.attrs.has("toolautosubmit"),
      node: form.node,
      has_password: hasPassword,
      params
    });
  }
  return { tools, truncated };
}
