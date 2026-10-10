# REA investigation — provenance and independent check

Status: ADOPTED planning direction (founder decision, 2026-10-09; Sol revision governs where it differs from the Opus baseline). Grants no authority: every capability still needs its own grain, review and qualification. Current execution state: `EXECUTION_FRONTIER.md`. Evidence baseline of this revision: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Historical Opus execution record

Opus reports REA6.1.0 registry installation/integrity verification, an out-of-memory full Desktop Commander package scan and successful scoped dist analysis. Original commands, reported totals and timings remain at [the immutable original report](https://github.com/TheHalfMoon/Aladdin/blob/87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985/docs/aladdin/REA_INVESTIGATION_REPORT.md). Those are attributed historical execution claims; Sol did not rerun that analysis or obtain the large raw artifact.

## REA and Desktop Commander verification

Opus lists an exact registry tarball/integrity, registry gitHead and output hash, but its large raw artifact and input file manifest were not available in the repository. Placeholder global npm paths and changing installed content prevent exact replay. The report distinguishes full-package OOM from scoped analysis; those run durations/file/AST totals remain **secondary evidence**, not independently measured here.

A fresh REA checkout is 6.1.0 at the revision above. `rtk proxy node scripts/rea.mjs doctor --json` exits1 with 'compiled runtime is missing'. No REA analysis was independently run and no toolchain was installed to simulate the earlier result. Native decompiler availability in Opus's historical host is not a universal capability claim about today's REA.

Independently inspected Desktop Commander source and installed0.2.52 dist show remote feature flags gating model-visible onboarding, with local/CLI/client exclusions and bounded display frequency. The inspected invitation text is bundled/static; remote activation is evidenced, arbitrary remotely supplied text/code execution is not. Keep useful file/session implementations but omit vendor instruction injection from tool results. Worker `eval:true` executes a fixed worker-program string with controlled module imports; user text/query appears as worker data. That pattern alone is not attacker-controlled eval or RCE. Child-process/network imports similarly require a traced untrusted-input-to-effect path before a vulnerability label.

A reproducible future REA run needs exact registry bytes/integrity, source/build pin, CLI/options, Node/OS/toolchain, input path manifest+hashes, bounded resources, output digest algorithm and raw artifacts with redaction/access policy. Its findings assist review; they do not qualify a donor by themselves.


No legal control, license or authorization was bypassed. Installed MIT Desktop Commander package and public source were inspected; hosted proprietary internals were not reverse engineered.
