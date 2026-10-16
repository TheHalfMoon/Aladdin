# Orcel public brand assets

**Current approved mark:** three-part Orbital Identity (open C-shaped outer orbit, complete circular inner ring, detached dot), with the lowercase orcel vector wordmark.

The canonical generator is scripts/sync-brand.mjs. Run node scripts/sync-brand.mjs --check to detect asset drift. All public SVG files and the OpenAI, Codex and Next.js icons derive from that script.

Production imagery: orbital SVG masters, horizontal and vertical vector lockups, monochrome app tiles, optional neutral editorial accent, orbital background pattern and 1200 by 630 social-card artwork. The dark social PNG, ZIP and checksum JSON in this directory are built on CI/Pages from reviewable sources.

Complete identity specifications: assets/brand/IDENTITY.md.
Public gallery: https://thehalfmoon.github.io/Orcel/brand.html

**Historic artifact only:** orcel-mark-mask.svg is the frozen predecessor from a closed governance record. Do not edit, overwrite or rebrand that file. It is not used as the current public mark.

The GitHub repo defines the package icons, but installed ChatGPT/Codex interfaces may require separate distribution steps and cache refreshes before showing them.
