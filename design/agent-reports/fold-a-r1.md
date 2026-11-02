Fold of FIX-A review r1 finding I1 (Taproot ctx rows 01g-01j) into `design/FIX_bughunt_mt_a.md` and `crates/mt-cli/tests/bughunt.rs`.

Fold commit: 7cb5211 (`FIX-A doc: fold review r1 I1 (Taproot ctx rows 01g-01j)`). Only I1 applied; M1, N1-N3 untouched. No production code changed.

## Anchors applied

- Test file: rows 01g, 01h, 01i, 01j inserted after `bughunt_mt_01f_...` and before `// ---- #5`, verbatim from I1 (step 1).
- §4.2 intro: "plus two guards" -> "plus four guards" (step 4).
- Rows 01, 01b, 01c ruling cells: `R1` -> `R1 + QA1`, `R1 (annex)` -> `R1 + QA1 (annex)` (step 3).
- Rows 01g, 01h, 01i, 01j appended after the 03e row (step 2).
- Paragraph after "side." (end of the 01e/01f/03e paragraph): the new sentence is added as its own sentence on the same paragraph. Placement per the coordinator ruling: after line 198, not after line 197. This is a deviation from the literal "after line 197" in the fix text, which would have split a sentence (see the ruling, not a STOP report).
- Counts before §3: `18 tests run: 4 passed, 14 failed`; failed list and guards updated; after §3: `18 tests run: 18 passed`; "matches all 18 rows" (step 5).
- §6 step 10: `expected matched count 18, 18 passed` (step 6).

## Gate

- RED command, as the fold changes it, on base 3bfbc98 code (`crates/mt-cli/src/validate.rs` unchanged): `cargo nextest run --locked -p mt-cli -E 'test(/^bughunt_mt_0[123]/)'`, captured once to `/scratch/cache/bughunt/mt-fold-a/red.txt`.
- Result: `18 tests run: 4 passed, 14 failed`. Matches the stated count.
- Passed: 01d, 01g, 01h, 02c (the guards). Failed: 01, 01b, 01c, 01e, 01f, 01i, 01j, 02, 02b, 03, 03b, 03c, 03d, 03e. Matches the stated list.
- `cargo fmt --check`: exit 0 (`/scratch/cache/bughunt/mt-fold-a/fmt.txt`).

No counts differ, so no ⚠.

Not run here: the GREEN after §3 (build step, not this fold) and the rest of the gate.
