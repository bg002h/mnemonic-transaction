# Build report: FIX-D (has_txindex JSON parse, ruling R5)

All gate steps pass. Nothing pushed.

## Rebase
`git fetch origin`; `git rebase origin/main` (9b961bed) applied all 5 doc/RED commits with no
conflicts at all (none to resolve in bughunt.rs, EXEMPT dict or FOLLOWUPS.md).
Review M1 is satisfied by this rebase, so it has no FOLLOWUPS entry.

## Commits
- c1c1f91 GREEN: `node.rs` `has_txindex` parses with serde_json; `Cargo.toml` moves serde_json to
  `[dependencies]` (Cargo.lock unchanged); `report.rs` comment re-worded per doc §6; row 05's
  `#[ignore]` removed.
- 6207ed0 FOLLOWUPS: `fixd-r5-unrowed-cases` (M2), `fixd-redundant-trim` (N1), both deferred-minor.
- This report, committed alone.

## RED / GREEN
Doc RED command (log `/scratch/cache/bughunt/mt-build-d/red.log`), with row 05's ignore removed:
7 run, 2 passed, 5 failed (rows 05-05e). Matches the doc. After GREEN: 7 run, 7 passed (`10rows.log`).

## Gate (logs in `/scratch/cache/bughunt/mt-build-d/`, final run after the last source edit)
1. fmt --check: ok. 2. clippy -D warnings: ok. 3. build: ok.
4. nextest --no-fail-fast: **340 tests run: 340 passed, 0 failed** (doc expected 293 plus A/B/C rows; 0 failures confirmed).
5. doc tests: ok. 6. check-refusal-coverage: ok. 7. mutate-refusals: all 38 refusal tests go red.
8. journeys: pass. 9. docs-only.test.sh: 73 passed, 0 failed. 10. FIX-D rows: 7 run, 7 passed.

## Deviations
None. The `trim()` before `from_str` is kept as the doc prescribes (N1 filed).
