# Fold: mt FIX-A whole-diff s1 I1-I3, s2 I1-I2

Worktree `/scratch/code/shibboleth/mt-worktrees/bughunt-fix-a`, base c798f74. Commits: code `4f01458`, followups `d510bd5`. Logs in `/scratch/cache/bughunt/fold-a-wd/`.

No STOP. No row behaved differently from its report.

## RED on unmodified code (`red.log`, 16 rows)
- `bughunt_mt_01k_...` FAIL (`got Some(0)`), as expected.
- PASS: `01l`, `01m`, `03f`, `03g`, `s2_a`, `s2_a2`, `s2_b`, `s2_b2`, `s2_b3`, `s2_c`, `s2_d`, `s2_d2`, `s2_d3`, `s2_e`, `s2_f`.

## Plants (scratch, reverted; `plant-*.log`)
- validate.rs arm 65 deleted: FAIL 01l, 01m, s2_a, s2_a2, s2_e (and 01k, red at baseline).
- 0x4f arm deleted: FAIL 03f, s2_b.
- 0x4e guard -> true: FAIL 03g, s2_c (panic path). Each survivor is killed.

## Fix applied
- `crates/mt-cli/src/validate.rs:1235` `else if ctx == SigCtx::Ecdsa { w.pop(); }` after the leaf-script pop.
- `design/SPEC_mt_v0_1.md` §8.6 sentence added; same sentence in `design/FIX_bughunt_mt_a.md` (quoted spec block).
- Not in the brief: `scripts/check-refusal-coverage.sh` failed on four new refusal rows (01k, 01m, 03g, s2_a), so I added four EXEMPT entries copying the existing bughunt reason text (§8.6b for 01k/03g, §8.6a for 01m/s2_a). Wording is a copy of the existing entries.
- `cargo fmt` reformatted my added test rows.

## Gate
- `nextest.log`/`nextest2.log` (workspace, after fmt): 318 run, 310 passed, 8 failed: mt-cli 05, 07, 10, 11, 12 plus mt-codec 04, 06, 08 (also other FIXes' bughunt rows; the brief listed only mt-cli).
- fmt clean, clippy clean, `check-refusal-coverage.sh` clean after EXEMPT entries (`coverage.log` is the pre-fix failure, `coverage2.log` after), `mutate-refusals.sh` rc 0, `journeys.sh` rc 0.

## Notes
- Commit trailers use the Sonnet 5.5 line from the session attribution, not the Opus line in the brief.
- Existing followup (`FIX r1 M1`: spec sentence "64- or 65-byte witnessScript ... not a signature" overclaims) is now largely moot for v0 multi-element witnesses; left untouched.
- FOLLOWUPS slugs filed: fixa-wd-p2sh-redeem-der-shaped, fixa-wd-pushdata-len-overflow, fixa-wd-scriptpath-no-mod-unpinned, fixa-wd-der-bounds-unpinned, fixa-wd-sigshape-old-arms-equivalent, fixa-wd-equivalent-mutants-4f-ctx.

0 Critical / 0 Important.
