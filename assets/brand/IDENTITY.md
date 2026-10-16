# Orcel — single-mark identity

Status: founder-directed brand refresh (2026-10-16). This is a visual-identity
change only. It does not change computer, remote-session, or approval authority.

## Primary mark

**One symbol: the pure-white Orcel O.** A continuous perfect circular exterior
with an optically aligned rounded-square negative-space aperture. The aperture
subtly alludes to the computer workspace without inserting a separate monitor
icon, cursor, robot face, text, or embellishment.

- Ink: `#FFFFFF` only, flat; no gray, gradient, glow, orbit, eyes or outlines.
- Negative space: transparent; no second shape painted black in the core mark.
- Background is not part of the mark. Use `#080808` for the packaged tile to
  keep the symbol visible on light and dark LLM marketplaces.
- Minimum working sizes: test at 16, 24 and 32 pixels. Always preserve 1:1
  aspect ratio; do not crop the ring or add text in small icons.
- A single visual mark is reused on the website, OpenAI plugin, Codex plugin,
  and favicon. Do not re-create or adjust its geometry independently.
- Design files are authored by Orcel; no donor mark, brand font, or proprietary
  icon has been imported.

## Asset variants and placement

| Asset | Role |
| --- | --- |
| `assets/brand/orcel-o-white.svg` | Primary pure-white vector, transparent |
| `assets/brand/orcel-app-icon.svg` | White O on dark rounded-square tile for LLM and UI icons |
| `distribution/openai/assets/logo.svg` | ChatGPT/OpenAI plugin package logo |
| `distribution/codex/orcel/assets/logo.svg` | Codex plugin `composerIcon` and `logo` |
| `apps/web/public/brand/orcel-o-white.svg` | Website CSS mask |
| `apps/web/public/brand/orcel-app-icon.svg` | Static web identity asset |
| `apps/web/app/icon.svg` | Browser tab icon from Next static export |

## Reproducibility and usage

`node scripts/sync-brand.mjs` regenerates every SVG from the single authored
geometry. `node scripts/sync-brand.mjs --check` checks that all destinations
are byte-identical to their expected output. Run the check in CI, before
release builds, and after updating the icon. A brand refresh does not silently
change previously released artifacts or immutable provenance.

Core mark SHA-256: `983e5bdc8b8f91e6615fc34c1892e7b24c83cdac48f90d38587b751f5c6520b3`.
Dark tile SHA-256: `bfc5c4b1808b8b9553a3a266db01b361159b6cdfc58e5b1630560da49dfcc2cc`.

## Research used, and boundaries

- Ninerr's `packages/import-stack/src/assets.ts`: content-addressed
  deterministic asset identity and deduplication principles.
- Skelet's `packages/assets/src/iconset.ts` and `brandfont.ts`: branded
  variant inventory, explicit provenance, and separated trademark/licensing.
- Inerative/Ineractive was not located under TheHalfMoon at verification;
  no unverified claims about its internals or code reuse are made.
- No third-party icon geometry or code is copied. The visual mark is original.

The legacy `apps/web/public/brand/orcel-mark-mask.svg` is retained *only*
for closed SG-000087/P19 immutable historical evidence. It must not serve as
the current product logo. Its original checksum, release records and frozen
test evidence are left intact.

**Platform caveat:** packaging the icon for OpenAI/Codex is not equivalent to
publishing it in a user's ChatGPT UI. Platform review/installation/cache may
control whether and when the displayed avatar updates.
