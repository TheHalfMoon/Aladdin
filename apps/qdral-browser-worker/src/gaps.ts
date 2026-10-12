// Known SG-000108 confinement gaps, tracked as `todo` probes in
// native.test.ts. While this record is non-empty, the type check below
// refuses any host frame beyond the launch vocabulary: SG-000108 slice 3
// cannot add live-navigation frames until every listed gap is closed (or accepted
// as founder-reviewed residual risks) and their entries removed here. The
// todo probes reference these entries, so removing one is an explicit edit.

import type { HostFrame } from "./protocol.js";

// Empty: #300 is a founder-accepted residual risk (decision 2026-10-11) and
// #301 is closed (same-origin registration refused by the route stack;
// cross-site iframes refused by the slice-3a policy route; both proven by
// native probes). Add an entry, with a todo probe that references it, for
// any future known gap.
export const KNOWN_GAPS = {} as const;

/// The host frames the launch vocabulary allows while gaps remain.
export const LAUNCH_VOCABULARY = ["hello", "launch", "ping", "shutdown"] as const;

type ExtraFrames = Exclude<HostFrame["frame"], (typeof LAUNCH_VOCABULARY)[number]>;
type GateHolds = [keyof typeof KNOWN_GAPS] extends [never] ? true : [ExtraFrames] extends [never] ? true : false;

/// Fails to compile (build and typecheck) when a frame outside the launch
/// vocabulary exists while any known gap remains.
export const SLICE3_GATE: GateHolds = true;
