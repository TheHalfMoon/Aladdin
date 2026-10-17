# Orcel - Nested Monogram Brand Identity

**Edition:** 2.0, October 2026
**Approval:** Founder-selected nested symbol; this document is the production design specification.
**Character:** Precise, understated, technical, recognizable.
**Brand promise:** Your computer, your terms.

## 1. Idea and visual meaning

The identity is an original single-line system derived from the founder's sketches. Its outer rounded **O** holds successively nested **r, c and e** paths; the lowercase **l** stands independently to the right of the O. The separate terminal stroke gives the mark its deliberate **O |** silhouette. The nested paths are designed to form a coherent monogram, not a literal vertically stacked word.

The previous orbital and satellite logo is **retired as the current mark** but preserved as historical evidence in Git and prior release records. No aerospace motif, planetary rings, fingerprints, unrelated font glyphs or imported trademarks belong in the new mark.

## 2. Geometry and optical construction

The vector master is 256 by 256 units. All strokes are round-capped, unfilled and unskewed. Geometry lives in scripts/sync-brand.mjs:

| Element | Master construction |
| --- | --- |
| O outer enclosure | Rect at (18,20), width 188, height 216, radius 70, stroke 10 |
| r outer nested path | M154 63 H105 C76 63 63 83 63 110 V180 C63 194 77 201 95 201 H156 |
| c middle nested path | M169 92 H124 C100 92 89 109 89 132 V159 C89 176 103 185 123 185 H171 |
| e inner loop | M177 120 H145 C128 120 118 131 118 145 C118 158 128 169 145 169 H177 |
| e crossbar | M122 145 H175 |
| l detached stroke | M234 79 V201 |
| Interior stroke | 10 units |

The lowercase l is **outside** the O and never intersects it. Maintain its right offset, balanced center height, optical baseline, and open negative space between each nested layer. Do not typeset the symbol using regular r/c/e/l letters. The vector geometry itself is the mark.

## 3. Color system

| Role | HEX | Use |
| --- | --- | --- |
| Primary white | #FFFFFF | Monogram and wordmark on dark backgrounds |
| Primary ink | #080808 | Main background and inverted dark mark |
| Surface mist | #F5F6F7 | Light-surface presentations |
| Charcoal | #17191B | Secondary dark panels |
| Silver | #D9DDE1 | Rules and subdued details |
| Slate | #94A3B8 | Descriptive UI/editorial supporting text |
| Accent blue | #3B82F6 | Optional digital highlight only |
| Accent violet | #8B5CF6 | Optional gradient endpoint |

The official master is always flat monochrome; only the optional editorial accent icon may use a blue/violet linear gradient. Do not apply blue or violet to the production monochrome SVG masters, security indicators or text requiring contrast.

## 4. Logo variants and rules

- **Primary lockup:** white nested symbol and original lowercase vector wordmark on #080808.
- **Inverse lockup:** matching black paths on #F5F6F7.
- **Stacked:** symbol above lowercase wordmark in square or portrait layouts.
- **Standalone icon:** nested symbol plus detached l; never crop the l or merge it into the O.
- **App tiles:** dark and light rounded-corner tiles; do not place a second enclosing circle around the symbol.
- **Editorial accent:** blue/violet gradient permitted only in secondary marketing treatments.

Keep a minimum clear space of **32 master units** around the entire symbol including the detached l; for the full lockup reserve at least **20% of the symbol height**. Align the wordmark to the mark's optical center, not to its bounding-box extreme. The custom letterforms remain lower-case **orcel**; prose uses **Orcel**.

## 5. Optical sizes and accessibility

Outputs are prepared at 16, 24, 32, 48, 64, 128, 256, 512 and 1024 pixels. Inspect real rendered pixels before using sizes below 32px: the fine nesting is necessarily compressed at 16px. Where space permits, prefer a 32px or larger icon, and include an accessible visible text label alongside the symbol. For screen readers, use the short accessible description **Orcel monogram** and keep decorative repeats hidden from assistive technologies.

Use sufficient contrast on site backgrounds; never depend on a gradient to establish the logo's identity. No glow, shadows, faux metallic effects, 3-D extrusions, rotation, shearing, outlines around the l, or arbitrary recoloring of the primary emblem.

## 6. Wordmark, voice and typography

The official lowercase wordmark is hand-constructed in the SVG generator, with **no font dependency**. For surrounding graphic/editorial typography, Sora is a preferred design reference if legitimately available; use system sans-serif fallbacks by default. No downloaded font files or commercial font redistribution are required.

Tone: clear, precise, confident and technically grounded. The canonical product tagline is **Your computer, your terms.** Do not state unavailable browser/desktop UI capabilities, supported operating systems or shipping installer status without separate product qualification.

## 7. Authoritative file mapping

| Target | Generated asset |
| --- | --- |
| White/black source masters | assets/brand/orcel-o-white.svg, orcel-o-black.svg |
| Horizontal/stacked lockups | assets/brand/orcel-lockup-*.svg |
| Wordmark | assets/brand/orcel-wordmark-*.svg |
| Dark and light application tiles | assets/brand/orcel-app-icon*.svg |
| Optional accent | assets/brand/orcel-accent-icon.svg |
| Monogram supporting pattern | assets/brand/orcel-monogram-pattern.svg |
| Site assets and Next favicon | apps/web/public/brand/, apps/web/app/icon.svg |
| OpenAI/Codex distribution | distribution/openai/assets/logo.svg, distribution/codex/orcel/assets/logo.svg |
| Web brand gallery | apps/web/app/brand/page.tsx |
| Raster/ICO exports | assets/brand/exports/ (built locally/CI) |
| Kit and SHA-256 manifest | Orcel-Identity-Kit-v2.zip, ASSET_MANIFEST.json |

Legacy filename orcel-orbit-pattern.svg remains as a **compatibility alias** for consumers relying on the old path; its content is the new monogram pattern, not the orbital art. The previous historical artwork remains available in Git history, rather than being silently overwritten in release tags.

## 8. Reproducibility, security and provenance

From the repository root run, in order:

1. node scripts/sync-brand.mjs
2. node scripts/sync-brand.mjs --check
3. node scripts/render-brand.mjs
4. python scripts/export-brand.py
5. npm run web:test
6. npm run web:typecheck
7. npm run web:build

Brand media use local pinned Sharp through the existing web toolchain, and standard-library Python ZIP/ICO generation. The v2 brand kit packages the current SVG masters, raster sizes, usage docs and SHA-256 hashes; it does not claim to be a software installer. The generator synchronizes OpenAI/Codex icons, but installed clients or marketplaces may cache previous versions.

This logo was developed as original geometry from the founder's own sketches and approved brand references. No art, fonts or marks of another organization are copied. Preserve all existing security, trust and approval boundaries. This is a branding change only: no product runtime, authorization, deployment credential or paid service is added.
