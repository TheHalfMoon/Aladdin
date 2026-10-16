# Orcel — Brand kit

**Selected identity:** Design #17. Solid white disc with two rounded diagonal
cut-out rails. The mark is flat, mono, and mechanically repeatable. The
primary SVG has true transparency in the two slashes, not painted black lines.

## Get the kit

[Download the Orcel Brand Kit v1 ZIP](../../assets/brand/downloads/Orcel-Identity-Kit-v1.zip)

[Full identity specifications](../../assets/brand/IDENTITY.md) ·
[Asset SHA-256 manifest](../../assets/brand/ASSET_MANIFEST.json) ·
[Production master generator](../../scripts/sync-brand.mjs) ·
[Raster exporter](../../scripts/export-brand.py)

## Primary assets

| Use | SVG |
| --- | --- |
| White master | [White disc + dual rails](../../assets/brand/orcel-o-white.svg) |
| Reverse master | [Black disc + dual rails](../../assets/brand/orcel-o-black.svg) |
| ChatGPT / Codex | [Dark app tile](../../assets/brand/orcel-app-icon.svg) |
| Light app tile | [Light app tile](../../assets/brand/orcel-app-icon-light.svg) |
| White wordmark | [Custom vector wordmark](../../assets/brand/orcel-wordmark-white.svg) |
| Black wordmark | [Custom vector wordmark](../../assets/brand/orcel-wordmark-black.svg) |
| White horizontal logo | [Horizontal lockup](../../assets/brand/orcel-lockup-white.svg) |
| Black horizontal logo | [Horizontal lockup](../../assets/brand/orcel-lockup-black.svg) |
| Social dark | [OG/social dark](../../assets/brand/orcel-social-dark.svg) |
| Social light | [OG/social light](../../assets/brand/orcel-social-light.svg) |
| Secondary pattern | [Two-rail rhythm](../../assets/brand/orcel-rails-pattern.svg) |

Raster files from 16px to 1024px, a print-friendly brand overview PNG, ICO,
and PNG social cards are stored in
[`assets/brand/exports/`](../../assets/brand/exports/).

## Website

Public identity preview: https://thehalfmoon.github.io/Orcel/brand.html

The site and product plugin icons all derive from the exact same source mark,
generated with `node scripts/sync-brand.mjs`. The original frozen logo and
closed-grain evidence remain unchanged and are no longer presented as active.

## Launch checklist

1. Download ZIP; compare manifest hashes after extraction.
2. Confirm the two slots remain visible at 16, 24, 32 and 64px.
3. Test logo on both `#080808` and `#F5F6F7` surfaces.
4. Release the updated OpenAI/Codex package assets through the applicable
   platform workflow. Changing repository contents does not automatically
   update an already installed ChatGPT connector icon.
5. Keep original release signatures and provenance immutable.

**Research:** design-system principles informed by public Pentagram case
studies (Windows, Intrinsic, Tractable) and internal repositories
TheHalfMoon/Skelet and TheHalfMoon/Ninerr. No other brand artwork copied;
Pentagram is not the author of this Orcel identity.

**No external font files are included.**
