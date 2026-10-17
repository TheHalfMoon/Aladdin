# Orcel nested monogram — design candidate (not published)

**Status:** Proposed brand identity, 17 October 2026; not deployed, not adopted as current production mark.

## Design intent

Use the founder's hand-drawn nested lettering concept in one emblem. The **O** is the enclosing contour; **r** curves inward from the outer top-left; **c** is a smaller open curve; **e** contributes the central crossbar; and **l** is the separate right-hand stroke. The glyphs are intentionally *nested*, not printed vertically or arranged as a space/orbit motif. This remains an abstract monogram and should be tested for recognizability.

## Geometry

A 256 × 256 viewBox. O is a rounded-square enclosure at x=18, y=18, size 220, corner radius 76, 11.5 stroke. Nested strokes: r `M72 189 V99 C72 75 91 60 115 60 H169`; c `M184 100 H133 C110 100 100 113 100 134 V145 C100 167 112 180 134 180 H180`; e crossbar `M112 143 H161`; l `M202 111 V191`. Interior strokes 10.5 with rounded caps. All are intentionally independent and should not touch. No gradients, planetary orbit, satellite, external font, embedded bitmap, or imported trademark.

## Brand system

- Primary: #FFFFFF on #080808; inverse: #080808 on #F5F6F7.
- Secondary neutral surfaces: #17191B, #D9DDE1, #A1A1AA. Primary artwork remains monochrome.
- Existing Orcel custom lowercase vector wordmark can accompany symbol at 256:900 horizontal proportions, or stacked. Use Orcel for editorial text and orcel for artwork.
- Preserve existing tagline “Your computer, your terms.” Do not describe unshipped products/capabilities.
- Keep ≥ 12.5% of monogram width as clear space. Validate at 16, 24, 32, 64, 256 and 1024px before production; smaller sizes need human optical inspection.

## Production integration plan (not applied)

Current active canonical files are maintained by `scripts/sync-brand.mjs` (29 SVG destinations), `scripts/render-brand.mjs`, and `scripts/export-brand.py`. The active brand and gallery are verified by `apps/web/tests/brand.test.mjs` and deployed via `.github/workflows/pages.yml`.

After the founder approves this proposed design, implement it in **the single generator**, update generated logo/favicon/OpenAI/Codex/website packages consistently, adjust brand gallery and tests, issue a newly versioned ZIP rather than silently replacing the historical kit, and requalify exact GitHub CI, Jev semantic review, Alibaba OCR delegation and separately inspected exclusions before merging. Verify post-merge Pages publication; distribution icons can remain cached independently.

**Preserve** current orbital identity and its generated files on `main`, historical signature/provenance and archives, security/runtime authority, and other agents' worktrees. This candidate is a new proposal under `docs/brand/proposals` only.

## In-repository sources reviewed

`scripts/sync-brand.mjs`, `scripts/render-brand.mjs`, `scripts/export-brand.py`, `apps/web/tests/brand.test.mjs`, `assets/brand/IDENTITY.md`, `docs/brand/README.md`, `apps/web/app/brand/page.tsx`, `.github/workflows/pages.yml`, and current distribution icon destinations. No third-party artwork copied. The current published icon remains unchanged.
