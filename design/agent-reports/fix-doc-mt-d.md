# FIX-D document report: mt bug hunt #5 (`has_txindex`)

Author: implementer, FIX letter D. Worktree
`/scratch/code/shibboleth/mt-worktrees/bughunt-fix-d`, branch `fix/bughunt-d`
(base 3bfbc98), FIX commit 2e78306. Not pushed.

## Delivered

- `design/FIX_bughunt_mt_d.md`: the R5 behaviour read out case by case; no
  spec, README or help delta (the spec and README never mention
  `getindexinfo`, and the UNKNOWN row of the liveness table at spec lines
  579-583 already matches); RED rows; tests that change (none); build order;
  gate; three STOP points.
- `crates/mt-cli/tests/bughunt.rs`: six new rows (05b-05g) and a helper that
  runs `mt inspect` against a stub `bitcoin-cli` with a chosen
  `getindexinfo` reply. Formatted with fmt. No production code was changed.

## RED evidence (unmodified production code)

Filter `-E 'test(/^bughunt_mt_05(|b|c|d|e|f|g)_/)'` matches 7 tests.
Result: `7 tests run: 2 passed, 5 failed`
(`/scratch/cache/bughunt/mt/fixd-red.log`).
- Rows 05, 05b, 05c, 05d and 05e fail at their STATUS assertion, each for
  its stated reason.
- Guards 05f and 05g pass.

Full baseline run (`fixd-baseline-full.log`): `293 run: 278 passed, 15
failed`. The 15 are the 5 FIX-D rows plus 10 rows owned by FIX-A, B and C.

## Findings

1. The hint patch `fix-05.patch` does not satisfy R5 (Minor; it is only a
   hint).
   - It slices from `"txindex"` up to the first `}` and keeps the substring
     test.
   - Applied as a scratch change, it gives 4 passed, 3 failed: compact JSON
     (05b), a truncated reply (05c) and a nested `synced` (05d) still fail
     (`fixd-fix05.log`).
   - The FIX doc records this so the builder does not reuse the patch.
2. STOP-1: whether to parse with serde_json or a hand-rolled parser. This is
   undecided, not a defect.
   - The crate states it avoids runtime dependencies (`report.rs:424`).
   - Moving the existing `serde_json` dev-dependency to `[dependencies]`
     builds with `--locked` and leaves `Cargo.lock` unchanged
     (`fixd-optS.log`). The Cargo.toml change was then reverted.
3. STOP-2: what duplicate member names (`txindex`, or `synced` inside it)
   should return. R5 does not say. The doc proposes the fail-safe answer,
   false, but does not choose it.
4. STOP-3: which base to use, and the gate's expected failures.
   - The brief places B before C but does not place D.
   - The doc gives the expected counts both ways: on 3bfbc98 the gate expects
     exactly the 10 named failures; on the merged A/B/C head it expects 0
     failures.

No new Critical or Important. Finding #5 itself is already graded Important
by the grade.

Counts: 0 Critical, 0 Important, 1 Minor, 0 Nit; 3 STOP points.
