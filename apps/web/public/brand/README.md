# Orcel web identity

Current mark: **Design #17** — one pure-white circle with two rounded
diagonal transparent cutouts. Master `orcel-o-white.svg`.

The same geometry is used for:
- `orcel-o-white.svg` and `orcel-o-black.svg` — transparent core identity;
- `orcel-app-icon.svg` and `orcel-app-icon-light.svg` — application tiles;
- `orcel-lockup-white.svg`, `orcel-lockup-black.svg` — vector wordmark lockups;
- `orcel-social-dark.svg`, `orcel-social-light.svg` — social presentation;
- `orcel-social-dark.png` — social/OG platform preview.

Source of truth: `scripts/sync-brand.mjs`; optional raster exports via
`scripts/render-brand.mjs` using the isolated website's existing `sharp`.
All current SVG destinations pass `node scripts/sync-brand.mjs --check`.

Full specifications, variants, and downloadable assets:
`docs/brand/README.md` or `https://thehalfmoon.github.io/Orcel/brand.html`.

**Historical artifact — do not alter:** `orcel-mark-mask.svg` remains an
immutable SG-000087/P19 legacy design with its previous SHA-256 to preserve
closed-grain evidence. It is not the active public logo.
