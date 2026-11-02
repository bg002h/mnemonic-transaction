# Whole-diff review: mt FIX-B (R4 CountMismatch, R6 non-ASCII scan, R8 TooShort guard)

Reviewer: Sonnet 5.5, effort medium. Diff `3bfbc98..a5e51c2`. Scratch copy
`/scratch/cache/bughunt/mt-wd-b/w2`; the real worktree was not touched.
Not a re-review: no previous whole-diff round on this slice (0 findings from a fix commit).

## Verdict
No Critical, no Important. Each fix removes its defect; each RED row fails with the fix planted back out.

## Plant results (fix removed in the scratch copy, RED rows run with the doc's nextest filter)
- R4 check removed: 3 of 3 R4 rows FAIL for the stated reason. Variant row `left: None`, `right: Some("CountMismatch { expected: 2, found: 3, index: 2 }")`; CLI row gets `§8.2e, input is not a decodable Bitcoin transaction` instead of `§1.1, the set does not verify`.
- R6 scan and R8 guard removed together: 5 of 5 rows FAIL. Kelvin rows accept the string (`left: None`, `right: Some(InvalidChar { ch: 'K', position: 22 })`); R8 rows panic at pipeline.rs:374 (`ten_symbol` row gets `Ok("qqqqqqqq")`, so it does pin the 24 threshold).
- With all fixes in: full `cargo nextest run --locked --no-fail-fast` gives `292 run, 284 passed, 8 failed`; the 8 are rows 01,02,03,05,07,10,11,12 (not B). `check-refusal-coverage.sh` passes.

## Checks of the advice and wrong-result questions
- R8 callers: `invariant_prefix` at main.rs:754 (encode, own fresh strings, `unwrap_or_default`), 1265 and 1639 (`.ok()`). The new Err cannot refuse anything; it only nulls `set_prefix`. Matches the doc's stated effect.
- R4 ordering: reversed order `[b[1], a[0]]` gives expected 3, found 2, index 1 (fail closed); `[a[0], b[2]]` now gives CountMismatch where it was MissingChunk (better text, still exit 1). Verified by reading the loop; CLI output for `[a[0], b[1]]` run as printed: `mt decode: REFUSED — §1.1, the set does not verify` then `chunk set size mismatch: expected 2 chunks, chunk 2 carries 3`, rc 1.
- R6: `decode_string` has no caller outside bch.rs (grep). The pipeline goes through `to_symbols`, which uses `to_ascii_lowercase` and already refused U+212A. So R6 closes the public-API function only; no shipped CLI path changed. Correct and harmless; the scan uses `rfind('1')` on the original, the later code on the lowercased string, and S1's ruling (no non-ASCII char lowercases to `m`, `t` or `1`) holds, so the two separators agree.
- EXEMPT line in `check-refusal-coverage.sh`: justified. The row asserts a `§1.1` message with no `refusals.toml` entry; the check is in the codec, not a CLI guard fn, so `mutate-refusals.sh` has nothing to neuter; the codec row and the mutation gate (caught 15, missed 0) hold it. The planted R4 removal made both rows fail.

## Findings
### Critical
None.
### Important
None.
### Minor
- M1. `crates/mt-cli/src/main.rs:1706-1790` (`explain_failure`): when a mixed-count set also contains one unreadable string, the unreadable branch wins and prints "This set has 2 chunks. 2 read cleanly and 1 did not, so every chunk COULD be here — nothing is necessarily lost" with remedy "Re-read that plate". The set is in fact mixed (chunk 2 carries 3), so "nothing is necessarily lost" is false until the holder re-reads and then meets CountMismatch. Reproduced: stdin `a[0]`, `b[1]`, a 58-char garbage `mt1qqq...`, `mt decode`, rc 1. Fails closed, two unusual conditions (count disagreement under one set id, plus an unreadable string), no command fails; same shape already exists for SetIdMismatch. Possible fix: in the unreadable branch, if `readable` holds two distinct `header.count` values, fall through to the generic refusal. File only.
### Nit
- N1. `error.rs` CountMismatch text "chunk 2 carries 3" omits the noun ("carries a count of 3"); the FIX doc and RED rows pin the wording, so changing it needs a doc edit. File only.

C / I / M / N: 0 / 0 / 1 / 1
