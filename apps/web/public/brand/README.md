# Orcel website brand assets

**Current**: `orcel-o-white.svg` — the pure-white Orcel O. The matching
app icon (`orcel-app-icon.svg`) presents exactly the same mark on a black
rounded-square backdrop, suitable for 16px browser or LLM integrations.

These assets are generated from the *single authored geometry* in
`scripts/sync-brand.mjs`; use `node scripts/sync-brand.mjs --check` to
verify exact byte identity across website, OpenAI and Codex distributions.

- Current white O SHA-256: `983e5bdc8b8f91e6615fc34c1892e7b24c83cdac48f90d38587b751f5c6520b3`
- Current dark app tile SHA-256: `bfc5c4b1808b8b9553a3a266db01b361159b6cdfc58e5b1630560da49dfcc2cc`

**Historical**: `orcel-mark-mask.svg` is the frozen SG-000087/P19
founder-approved `D + computer` mark. It is no longer displayed as
the current Orcel identity, but remains intact for historical evidence and
its exact checksum tests. Do not modify or overwrite it.

See `assets/brand/IDENTITY.md` for usage, licensing and rollout boundaries.
