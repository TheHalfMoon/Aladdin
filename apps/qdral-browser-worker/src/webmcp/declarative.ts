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
//   the record set or drops the field rather than growing.
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
const TEXT_INPUT_TYPES = new Set(["email", "password", "search", "tel", "text", "url"]);
const READONLY_INPUT_TYPES = new Set([...TEXT_INPUT_TYPES, "date", "datetime-local", "month", "number", "time", "week"]);
const CONTROL_TAGS = new Set(["input", "select", "textarea"]);

// C0/C1 controls (whitespace is collapsed to spaces first).
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;
const CONTROLS = /[\u0000-\u001f\u007f-\u009f]/g;

/// Collapse whitespace, drop controls and cut at `max` code points, never
/// inside a surrogate pair (a lone surrogate would make the frame invalid
/// JSON for the host).
function bound(text: string, max: number): string {
  const clean = text.replace(/\s+/g, " ").replace(CONTROLS, "").trim();
  const points = Array.from(clean);
  return points.length > max ? points.slice(0, max).join("") : clean;
}

function codePoints(text: string): number {
  return Array.from(text).length;
}

function attr(element: DomElement, name: string): string | undefined {
  return element.attrs.get(name);
}

function inputType(element: DomElement): string {
  const type = (attr(element, "type") ?? "text").toLowerCase();
  return type === "" ? "text" : type;
}

function* descendants(element: DomElement): Generator<DomElement> {
  for (const child of element.children) {
    if (child.kind === "element") {
      yield child;
      yield* descendants(child);
    }
  }
}

function textOf(element: DomElement, skipControls: boolean): string {
  const parts: string[] = [];
  const walk = (node: DomNode): void => {
    if (node.kind === "text") {
      parts.push(node.text);
      return;
    }
    if (skipControls && ["button", "input", "meter", "output", "progress", "select", "textarea"].includes(node.tag)) return;
    for (const child of node.children) walk(child);
  };
  walk(element);
  return parts.join(" ");
}

function isDisabled(control: DomElement): boolean {
  if (control.attrs.has("disabled")) return true;
  for (let up = control.parent; up; up = up.parent) {
    if (up.tag === "fieldset" && up.attrs.has("disabled")) return true;
  }
  return false;
}

function isReadOnly(control: DomElement): boolean {
  if (!control.attrs.has("readonly")) return false;
  return control.tag === "textarea" || (control.tag === "input" && READONLY_INPUT_TYPES.has(inputType(control)));
}

/// The form's controls in tree order: its descendants (not inside a nested
/// form, not owned elsewhere by a `form` attribute) plus controls anywhere
/// in the document whose `form` attribute names this form's id.
function formControls(form: DomElement, all: readonly DomElement[]): DomElement[] {
  const id = attr(form, "id");
  const owned = new Set<DomElement>();
  for (const element of descendants(form)) {
    if (!CONTROL_TAGS.has(element.tag)) continue;
    const owner = attr(element, "form");
    if (owner !== undefined) continue;
    let nested = false;
    for (let up = element.parent; up && up !== form; up = up.parent) if (up.tag === "form") nested = true;
    if (!nested) owned.add(element);
  }
  if (id !== undefined && id !== "") {
    for (const element of all) if (CONTROL_TAGS.has(element.tag) && attr(element, "form") === id) owned.add(element);
  }
  return all.filter((element) => owned.has(element));
}

function labelText(control: DomElement, all: readonly DomElement[]): string {
  const labels: DomElement[] = [];
  const id = attr(control, "id");
  if (id) for (const element of all) if (element.tag === "label" && attr(element, "for") === id) labels.push(element);
  for (let up = control.parent; up; up = up.parent) {
    if (up.tag === "label" && !up.attrs.has("for")) {
      labels.push(up);
      break;
    }
  }
  return labels
    .map((label) => bound(textOf(label, true), LIMITS.paramDescription))
    .filter(Boolean)
    .join("; ");
}

function commonFieldset(form: DomElement, controls: readonly DomElement[]): DomElement | undefined {
  const first = controls[0];
  for (let up = first?.parent ?? null; up && up !== form; up = up.parent) {
    if (up.tag !== "fieldset") continue;
    const inside = new Set(descendants(up));
    if (controls.every((control) => inside.has(control))) return up;
  }
  return undefined;
}

function parameterDescription(form: DomElement, controls: readonly DomElement[], all: readonly DomElement[]): string | undefined {
  if (controls.length === 1) {
    const control = controls[0];
    if (!control) return undefined;
    return attr(control, "toolparamdescription") || labelText(control, all) || attr(control, "aria-description") || undefined;
  }
  const fieldset = commonFieldset(form, controls);
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
  if (pattern === undefined || codePoints(pattern) > LIMITS.pattern || CONTROL.test(pattern) || !wellFormed(pattern)) return schema;
  try {
    new RegExp(pattern, "v");
  } catch {
    return schema;
  }
  return { ...schema, pattern };
}

function numberSchema(input: DomElement): ParamSchema {
  const schema: ParamSchema = { type: "number" };
  const minimum = finiteNumber(attr(input, "min"));
  const maximum = finiteNumber(attr(input, "max"));
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
      return withPattern(numberSchema(input), input);
    case "range": {
      const schema = numberSchema(input);
      schema.minimum ??= 0;
      schema.maximum ??= 100;
      return schema;
    }
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

/// No lone surrogates (they cannot cross the frame as valid JSON).
function wellFormed(text: string): boolean {
  return !/[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/.test(text);
}

/// Author-declared choice values, bounded; undefined if any value is too
/// long, carries controls or a lone surrogate, or there are too many (the
/// parameter is then dropped).
function choices(values: readonly string[]): string[] | undefined {
  if (values.length > LIMITS.choices) return undefined;
  if (values.some((value) => codePoints(value) > LIMITS.choice || CONTROL.test(value) || !wellFormed(value))) return undefined;
  return [...values];
}

function optionValues(select: DomElement): string[] {
  const values: string[] = [];
  for (const element of descendants(select)) {
    if (element.tag !== "option") continue;
    // An option without a value attribute submits its text.
    values.push(attr(element, "value") ?? bound(textOf(element, false), LIMITS.choice));
  }
  return values;
}

function isInput(control: DomElement, type: string): boolean {
  return control.tag === "input" && inputType(control) === type;
}

function parameterSchema(form: DomElement, controls: readonly DomElement[], all: readonly DomElement[]): ParamSchema | undefined {
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
    const values = choices(optionValues(first));
    schema = values && (first.attrs.has("multiple") ? { type: "array", items_enum: values } : { type: "string", enum: values });
  } else {
    schema = inputSchema(first);
  }
  if (!schema) return undefined;
  const description = parameterDescription(form, controls, all);
  const dateHint = isInput(first, "date") ? "Dates MUST be provided in 'YYYY-MM-DD' format." : undefined;
  const combined = description && dateHint ? `${description} (${dateHint})` : description || dateHint;
  return combined ? { ...schema, description: bound(combined, LIMITS.paramDescription) } : schema;
}

export interface Discovery {
  tools: DeclaredTool[];
  /// Some declarations were dropped by a bound.
  truncated: boolean;
}

/// Every declarative tool in the document, in tree order. A form needs a
/// valid `toolname` and a `tooldescription`; the first form with a name wins
/// and later duplicates are dropped (as in the donor).
export function discoverDeclarativeTools(root: DomElement): Discovery {
  const all = [root, ...descendants(root)];
  const tools: DeclaredTool[] = [];
  const names = new Set<string>();
  let truncated = false;
  for (const form of all) {
    if (form.tag !== "form") continue;
    const name = attr(form, "toolname");
    const description = attr(form, "tooldescription");
    if (name === undefined || description === undefined) continue;
    if (!NAME.test(name) || names.has(name)) {
      truncated ||= !NAME.test(name);
      continue;
    }
    if (tools.length >= LIMITS.tools) {
      truncated = true;
      break;
    }
    names.add(name);
    const controls = formControls(form, all).filter((control) => !isDisabled(control) && !isReadOnly(control));
    const groups = new Map<string, DomElement[]>();
    let hasPassword = false;
    for (const control of controls) {
      if (isInput(control, "password")) {
        hasPassword = true;
        continue;
      }
      const key = (attr(control, "name") ?? "").trim();
      if (!key) continue;
      const group = groups.get(key);
      if (group) group.push(control);
      else groups.set(key, [control]);
    }
    const params: DeclaredParam[] = [];
    for (const [key, group] of groups) {
      if (codePoints(key) > LIMITS.name || CONTROL.test(key) || !wellFormed(key)) {
        truncated = true;
        continue;
      }
      const schema = parameterSchema(form, group, all);
      if (!schema) continue;
      if (params.length >= LIMITS.params) {
        truncated = true;
        break;
      }
      params.push({ name: key, required: group.some((control) => control.attrs.has("required")), ...schema });
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
