# Orcel — Orbital Identity System

**Status:** Approved visual direction, 16 October 2026.
**Primary lockup:** black background, white orbital symbol, lowercase white orcel.
**Tagline:** Your computer, your terms.

This document is the source of truth for the founder-selected orbital mark. The identity is a branding change only; it does not change Orcel software permissions, trusted workspace scopes, product compatibility, release status, or authority boundaries.

## 1. The symbol

Orcel's emblem consists of three intentional parts: a C-shaped open outer white arc, a full inner white ring, and a detached satellite dot near the upper-right opening. Its negative space and asymmetry are integral. Do not connect the open ends, move the dot, add rings, rotate the mark, or replace it with a solid disc. The lowercase wordmark is separate from the symbol; it is not squeezed inside the orbit.

## 2. Construction

All dimensions use an undistorted 256 by 256 coordinate system. The outer orbit has SVG path M188 36 A108 108 0 1 0 128 232, a 10-unit stroke, rounded ends, and no fill. The inner circle has center (137,127), radius 78, and a 12-unit stroke. The dot has center (208,49) and radius 14. Geometry is canonical in scripts/sync-brand.mjs.

Primary white: #FFFFFF. Primary black background: #080808. Supporting neutrals: charcoal #17191B, mist #F5F6F7, silver #D9DDE1, graphite #565D66, slate #B8BCC1. The primary logo must be flat monochrome with no glow, gradient, shadow, chrome, or extra colors.

## 3. Asset inventory

| File | Usage |
| --- | --- |
| orcel-o-white.svg / orcel-o-black.svg | Transparent white/black symbols |
| orcel-lockup-white.svg / orcel-lockup-black.svg | Main horizontal mark plus lowercase wordmark |
| orcel-lockup-stacked-white.svg / orcel-lockup-stacked-black.svg | Vertical arrangement |
| orcel-wordmark-white.svg / orcel-wordmark-black.svg | Original lowercase vector letters |
| orcel-app-icon.svg / orcel-app-icon-light.svg | App, plugin and favicon tiles |
| orcel-accent-icon.svg | Secondary editorial treatment only |
| orcel-orbit-pattern.svg | Subordinate supporting graphic rhythm |
| orcel-social-dark.svg / orcel-social-light.svg | 1200 x 630 social headers |
| exports and downloads directories | Generated PNGs, favicon ICO, previews, SHA-256 manifest and ZIP |

The release ZIP includes the vector masters, editorial guidance, image exports, and cryptographic manifest. Raster outputs are built in CI, not checked into source control, allowing inspectable code review.

## 4. Usage guidelines

- Preserve the original circle geometry and dot position. Never mirror, stretch, fill the inner circle, or close the outer arc.
- For white-on-black use the master white SVG; use the black inverse on pale backgrounds.
- Keep clearspace around the symbol equal to at least the dot diameter. Around a lockup, use 20% of symbol height.
- Target sizes are 16, 24, 32, 64, 256 and 1024 px. Verify the detached dot and open arc remain visibly separate. Omit the wordmark in square icons.
- Use lowercase orcel in artwork and Orcel in editorial prose. The vector wordmark uses hand-authored original glyphs without any distributed fonts.
- Do not use the mark to suggest unrestricted remote shell, approval bypasses, or functionality unavailable in a product release.
- Historical brand evidence, signatures, manifests and prior releases remain immutable.

## 5. Distribution mapping

| Destination | Production file |
| --- | --- |
| Web favicon | apps/web/app/icon.svg |
| Website identity | apps/web/public/brand/orcel-o-white.svg |
| ChatGPT/OpenAI package | distribution/openai/assets/logo.svg |
| Codex package | distribution/codex/orcel/assets/logo.svg |
| Website brand page | apps/web/app/brand/page.tsx |
| Public media kit | https://thehalfmoon.github.io/Orcel/brand/Orcel-Identity-Kit-v1.zip |

Source files do not by themselves change cached app or marketplace icons. Such updates require the respective product distribution workflow.

## 6. Design research, authorship and source reuse

The mark was reconstructed as original vector geometry from the founder's approved reference image. It does not reuse artwork, logotypes or trademarks belonging to other organizations.

Research methods were informed by Pentagram's MIT Media Lab identity system (https://www.pentagram.com/work/mit-media-lab/story). Reusable engineering practices were informed by TheHalfMoon/Skelet's icon sanitization and rights metadata modules (packages/assets/src/iconset.ts and brandfont.ts), TheHalfMoon/Ninerr's content-addressed asset manifest approach (packages/import-stack/src/assets.ts), and Orcel's own deterministic generator, safety tests and release workflow. These organizations did not design, license or endorse the Orcel symbol.

## 7. Reproduce

Run the following commands at the repo root in order:

1. node scripts/sync-brand.mjs
2. node scripts/sync-brand.mjs --check
3. node scripts/render-brand.mjs
4. python scripts/export-brand.py
5. npm run web:test
6. npm run web:typecheck
7. npm run web:build

No cloud graphics provider, font download, or paid service is required. The website and release qualification pipelines verify current SVGs and generated raster artifacts before publication. Do not rewrite historical closed-grain records.
