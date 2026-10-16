# Orcel — Identity System v1.0

**Status:** Founder-selected direction, 16 October 2026.
**Symbol:** White solid disc with **two rounded diagonal negative-space rails**.
**Working principle:** *Two rails, one trusted path.*

This file is the normative brand identity. The new visual system is not a security,
runtime, or authorization change. It cannot silently alter approvals, scopes,
installed binaries, published releases, or historical audit evidence.

## 1. The mark

The icon is a true circle, deliberately simple at a glance. Two parallel
45-degree capsules are removed from the white material, not painted over it.
They represent two necessary parts of an agent interaction: **intent** and
**local authorization**. Both converge in one dependable path. No human face,
robot eyes, literal monitor, arrows, or generic AI sparkle.

The distinctive geometry is authored once in `scripts/sync-brand.mjs`.
**Do not redraw the circle, move the slashes, rotate the design, or change their
spacing by hand.** Outer disc center (128,128), radius 108 within a 256 viewBox.
Rounded cuts are centered between (83,146)–(133,96) and (125,177)–(176,126),
with 10-unit cap radii. Slashes are true transparency in the stand-alone mark.

## 2. Primary / inverse / packaging

| Context | Asset | Background |
| --- | --- | --- |
| Dark UI / marketing | `orcel-o-white.svg` | Black or dark ink |
| Light UI / print | `orcel-o-black.svg` | White or cool off-white |
| ChatGPT, Codex, app tile | `orcel-app-icon.svg` | Supplied charcoal squircle |
| Light tile surfaces | `orcel-app-icon-light.svg` | Supplied near-white squircle |
| White wordmark, alone | `orcel-wordmark-white.svg` | Dark |
| Black wordmark, alone | `orcel-wordmark-black.svg` | Light |
| Brand lockups | `orcel-lockup-white.svg`, `orcel-lockup-black.svg` | As named |
| Social/OG templates | `orcel-social-dark.svg`, `orcel-social-light.svg` | As supplied |
| Supporting rhythm | `orcel-rails-pattern.svg` | Dark (never substitute for logo) |

**Never** place white symbol on a white background without an ink-colored
container. **Never** present the tile as the sole logo in a print lockup.

## 3. Palette

| Token | Hex | Role |
| --- | --- | --- |
| White | `#FFFFFF` | Core identity; primary mark |
| Ink | `#080808` | Dark ground; reverse symbol |
| Charcoal | `#17191B` | Supporting dark surfaces |
| Silver | `#D9DDE1` | Hairlines and supporting elements |
| Mist | `#F5F6F7` | Light application tile/background |
| Graphite | `#565D66` | Long-form supporting text on white |
| Slate | `#B8BCC1` | Supporting text on ink |

The core logo is **never a gradient, chrome, glass, glow, or multicolor badge**.
Silver is allowed only in the environment, not the primary white mark.
Accessible interface state colors may coexist outside logo artwork.

## 4. Proportions, spacing and small-size behavior

- Minimum clearspace: 25% of the outer disc diameter on every side,
  measured from visible circle to adjacent typography or UI chrome.
- Keep the 256 viewBox proportions. Never squish or stretch.
- For square icons use a `256×256` dark tile with radius 54 and an
  undistorted mark: circle radius 108, margin 20.
- The smallest target is **16px**, but 24/32/64px must also be checked
  manually on dark and light backgrounds before any external marketplace
  submission. At 16px, geometric detail inevitably compresses; do not
  add extra glyphs to compensate.
- Wordmark paths are authored in the generator; no externally licensed
  brand font file is embedded or distributed.
- Do not lock the logo inside another circle or add another O beside it.

## 5. Typography

- Wordmark: bespoke vector geometry; use shipped SVG files for branding.
- Product UI: system UI stack `Inter, -apple-system, BlinkMacSystemFont,
  "Segoe UI", sans-serif` (Inter if independently installed/licensed).
- Code/docs: `ui-monospace, "SFMono-Regular", Consolas, monospace`.
- Headings use semibold weight and tight tracking. Body uses high x-height
  and ordinary sentence case. Avoid stretched all-caps wordmarks.

## 6. Layout / graphic system

**Two-rail rhythm:** parallel diagonals from the symbol are the only
optional supporting motif. Use them as fine dividers, crops, or repeating
pair structures, always subordinate to content. The logo itself stays
flat and unchanged. Grids on 8px spacing; use large calm margins;
never wallpaper dashboards with repeating logos.

## 7. Voice and messaging

**Brand name:** Orcel (sentence case) or `orcel` for CLI/technical IDs.
**Tagline:** *Your computer, your terms.*
**Short descriptor:** Local-first, trusted computer access for AI assistants.
**Product promise:** Computer work stays within user-approved boundaries.
**Never claim:** unrestricted shell, blanket desktop access, approval bypass,
or invisible remote control. Keep language accurate to actual release.

Tone: precise, reassuring, capable, understated. Avoid AI hype,
anthropomorphic mascot language, or unverified superlatives.

## 8. Distribution mapping

- `apps/web/public/brand/orcel-o-white.svg`: website mark and CSS mask.
- `apps/web/app/icon.svg`: Next.js favicon export.
- `apps/web/public/brand/orcel-app-icon.svg`: public packaged tile.
- `distribution/openai/assets/logo.svg`: OpenAI/ChatGPT plugin asset.
- `distribution/codex/orcel/assets/logo.svg`: Codex plugin composer/logo.
- `assets/brand/exports/`: raster export sizes for app/LLM listings.
- `assets/brand/downloads/`: downloadable ZIP containing the production kit.

The repository controls **packaged assets**, not the appearance of an already
installed plugin in every ChatGPT account. Publication, cache updates,
and any marketplace approvals may still be required.

## 9. Source of truth and verification

Run:
```sh
node scripts/sync-brand.mjs
node scripts/sync-brand.mjs --check
npm ci --prefix apps/web --ignore-scripts
node scripts/render-brand.mjs
python scripts/export-brand.py
node scripts/check-current-identity.mjs
npm run web:test
npm run web:build
```

CI requires byte-identity of all generated SVG variants. The pinned web
sharp dependency renders PNGs; Python's standard library packages the ICO,
manifest, and downloadable ZIP during build. Compiled media are not committed
to Git, so semantic review receives only inspectable text and SVG sources.
The deterministic ZIP includes SHA-256 records and is published by Pages.

## 10. Historical boundary

The original `apps/web/public/brand/orcel-mark-mask.svg` is a frozen
historical SG-000087/P19 mark (formerly D + computer). Keep its original bytes,
signed/audited records and historic byte-pinning tests intact. Its existence
does **not** make it the active Orcel logo. Do not re-label old screenshots,
old releases, or the past provenance ledger as newly approved Orcel artifacts.

## 11. Research, use and provenance

This identity is newly authored Orcel-specific artwork. No Linear, Warp, or
Pentagram trademark artwork has been copied or imported. Pentagram is a design
research reference, **not** a collaborator, endorser, or commissioned agency.

Ideas adapted conceptually:
- Pentagram Windows — name-aligned geometric simplicity:
  https://www.pentagram.com/work/windows
- Pentagram Intrinsic — precise technical symbol, typography and functional icon system:
  https://www.pentagram.com/work/intrinsic
- Pentagram Tractable — legibility at small sizes and continuity across touchpoints:
  https://www.pentagram.com/work/tractable
- TheHalfMoon/Skelet `packages/assets/src/brandfont.ts` and `iconset.ts`:
  brand variants, provenance and safe SVG data handling.
- TheHalfMoon/Ninerr `packages/import-stack/src/assets.ts`:
  deterministic content-addressed asset records.

Other source repositories remain unchanged. This work defines the project
identity; it does not assert legal trademark clearance or exclusivity.
