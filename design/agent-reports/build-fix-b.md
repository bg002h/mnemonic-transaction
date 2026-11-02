# Build report: FIX-B (mt bughunt codec robustness, #4 #6 #8)

Worktree /scratch/code/shibboleth/mt-worktrees/bughunt-fix-b, branch
fix/bughunt-b, local only (no push, no PR). cargo 1.85.0. Gate logs in
/scratch/cache/bughunt/mt-build-b/.

## Commits (after 83b14d5)
- 9badcc3 review M1 (R8 observable-effect sentence, §4 last bullet) and M2 (row `bughunt_mt_08_ten_symbol_string_is_too_short` in bughunt.rs and the doc's RED table/command/counts)
- c115432 step 1: spec delta in design/SPEC_mt_v0_1.md (same `count` in every chunk); carries the F-321 line
- 67d07ce step 3: R8 `invariant_prefix` TooShort guard
- 734648d step 4: R4 `Error::CountMismatch` and the check in `decode`
- 02cdf05 step 5: R6 early non-ASCII scan in `decode_string` (message quotes the RED count)
- 9554498 FOLLOWUPS: review N1, N2, plus `decode-prefix-after-bch-correction` (open, needs triage)

## RED / GREEN
- RED (before step 3, log red.log): `8 tests run: 0 passed, 8 failed, 284 skipped`.
- GREEN (after step 5, log green.log): `8 tests run: 8 passed, 284 skipped`.

## Gate
1. fmt --check rc 0 (1-fmt.log)
2. clippy -D warnings rc 0 (2-clippy.log)
3. build rc 0 (3-build.log)
4. nextest --no-fail-fast: 292 run, 284 passed, 8 failed (4-nextest.log). The 8 are exactly the non-B rows 01, 02, 03, 05, 07, 10, 11, 12 (owned by FIX-A/C/D); no other failure.
5. doc tests rc 0 (5-doc.log)
6. check-refusal-coverage rc 0 (6-coverage.log)
7. mutate-refusals: all 34 go red, rc 0 (7-mutate.log)
8. journeys A, B, C pass (8-journeys.log)
9. RED command: 8 matched, 8 passed.
Not run: `coordinator/scripts/mutation-gate.sh` (the coordinator runs it before review). Expected equivalent mutant: the unreachable `!c.is_ascii()` branch in `decode_string`'s data loop.

## Deviations
- Doc RED/GREEN observed-text lines (171, 192) edited to say 8 rows (7 before M2); otherwise none.
- Review N1 (blank line): the variant was written with a blank line before it in error.rs (matches enum style); N1 is still filed in FOLLOWUPS per the brief, though the code already satisfies it.
- §6 mk/md Unicode leads not filed in FOLLOWUPS (for the coordinator, per brief): mk `275a994` bch.rs:673 lowercases before the non-ASCII check (same #6 pattern); md `07d85472` chunk.rs:528 and codex32.rs:73 use `c as u8` truncation (unverified).
