# FIX review r1: FIX-B (mt bughunt codec robustness, #4 #6 #8)

Reviewer: Opus, effort medium. W head 7820750 (fold 2babc10). Scratch:
/scratch/cache/bughunt/mt-fix-review-b/src (clone of W at 7820750, fixes applied
from the doc's §1-§2 text verbatim). First round on this doc: none of the
findings below come from a previous round's fix commit.

## Progress log
- Every §1/§2 anchor matched exactly once (spec lines 221-223; `let symbols =
  to_symbols(s)?;` in `invariant_prefix`; `SetIdMismatch` block in `decode`;
  `use crate::Error;` in `decode_string`). Build clean on cargo 1.85.0.
- RED command as printed, after fixes: `7 tests run: 7 passed, 284 skipped`.
- `decode_string` (R6's target) has no production caller (grep: only
  `bch.rs:679` definition and tests); `decode_chunk` uses `to_symbols`
  (`to_ascii_lowercase`). Consistent with the grade's "CLI behaviour is
  unchanged"; the fix is library hardening.
- R6 position: the scan returns the FIRST non-ASCII char, so every char before
  it in the data part is ASCII and byte offset == char offset; a builder using
  `char_indices` gets the same value. No ambiguity.

## Findings (appended as found)

### M1. R8 changes printed CLI output on a reachable path; the doc says it does not
Location: FIX doc §2 R8 ("No new text"), §4 last bullet ("no flag or printed
CLI text changes here"), and triage #8's premise ("CLI callers ... pass
strings[0] after they decoded ... so short input was not reached").
`decode` tolerates an unreadable string when another copy fills the slot, so
`strings[0]` reaching `invariant_prefix` (main.rs:754, 1265, 1639) can be a
truncated plate. Reproduction (scratch, test vector 1's 6 strings with
`mt1p9h8jqq9qqqqg` (13 symbols) prepended):
- base 3bfbc98: `mt decode` stderr `mt1 SET   6 strings, 1..6 all present, all begin mt1p9h8jqq9`; `mt inspect --json` `"set_prefix": "p9h8jqq9"`.
- fixed: `mt1 SET   6 strings, 1..6 all present` (clause dropped); `"set_prefix": null`. rc 0 both.
Under 8 symbols the base refuses earlier (§3b "the full string is only 5
characters"), so the panic is still unreachable from the CLI. The new output is
not wrong (it drops a claim made from a damaged plate), and the path needs 2
conditions (first string truncated to 8..23 symbols, and a complete set
otherwise), so Minor. Fix text: in §2 R8 after "No new text." add: "Observable
effect: when the first input string is unreadable and has 8..23 data symbols
while the set still decodes, the `mt1 SET` row drops its `all begin mt1…`
clause and `--json` `set_prefix` becomes null (it was the damaged string's
first 8 symbols). No test pins this; it is accepted." and change §4's last
bullet to "no flag changes; the one printed-text change is R8's above".

### M2. R8's threshold (24 symbols) is pinned by no row; a guard at 8 survives
- Location: FIX §2 R8 guard in `invariant_prefix`; RED rows 08 only use inputs
  under 8 symbols and the CLI journey, so `len < INVARIANT_PREFIX_SYMBOLS`
  passes every row.
- Plant P6 (threshold 8 instead of `HEADER_SYMBOLS + CHECKSUM_SYMBOLS`): 0 of 7
  RED rows fail. Fails closed either way (the bigger guard only shrinks what
  is printed); the mutation gate after the build will probably flag it.
- Fix text (run: passes with the fixes, fails under P6 with
  `left: Ok("qqqqqqqq")`): add to `crates/mt-codec/tests/bughunt.rs` a row
  `bughunt_mt_08_ten_symbol_string_is_too_short` whose body is
  `assert_eq!(mt_codec::string_layer::invariant_prefix("mt1qqqqqqqqqq"),
  Err(Error::TooShort { len: 10, header: 11, checksum: 13, min: 24 }));`
  and list it among the 08 rows of the RED table.

### N1. CountMismatch variant snippet lacks the blank line the enum uses between variants
The §2 R4 new text puts `CountMismatch { .. }` directly under the previous
variant; `cargo fmt` does not add it. Cosmetic.

### N2. Mixed-set holder sees "chunk 1 of 2 is missing" in the ordering case
When the first readable string has the smaller count, the doc's decided
ordering names a missing chunk rather than the count mismatch. Fails closed,
decided in the doc; optional wording only.

## Plant results (fixes applied, then original fault re-planted; 7 RED rows each)
| plant | fault | RED rows failing |
|---|---|---|
| P1 | R4 count check removed | 3 (all 04) |
| P2 | R6 early non-ASCII scan removed | 2 (both 06) |
| P3 | R8 TooShort guard removed | 2 (both 08) |
| P4 | CountMismatch index `idx` not `idx+1` | 2 (variant_and_text, cli) |
| P5 | expected/found swapped | 2 (same) |
| P6 | R8 threshold 8 instead of 24 | 0 (M2) |
| P7 | R6 position off by 3 | 1 (06 kelvin offset) |

## Gate on the fixed scratch copy (cargo 1.85.0)
fmt rc 0; clippy -D warnings rc 0; nextest 291 run, 283 passed, 8 failed: the
8 failures are exactly the non-B bughunt rows (01, 02, 03, 05, 07, 10, 11, 12),
owned by FIX A/C/D; doc tests ok; check-refusal-coverage rc 0;
mutate-refusals "all 34 refusal tests go red" rc 0; journeys A, B, C pass.

## Fold check (Q0, QB1)
Both rulings are carried exactly in the header, build step 2.1, §1, S1, S2 and
the commit-message line `Spec copy: mt is authoritative; engrave sync owed by
F-321.` No contradiction between sections.

## Out of scope (pre-existing on base, for filing; not graded here)
- Wrong printed prefix after BCH correction: a set whose first string has one
  error inside its first 8 data symbols (`mt1p9h8jxq9…` for `mt1p9h8jqq9…`,
  file /scratch/cache/bughunt/mt-fix-review-b/corr.txt). `mt decode` prints
  `all begin mt1p9h8jxq9` while `mt verify` prints `OK — 6 chunks, set
  0x2dcf2`, on base and with the fixes. Cause: `invariant_prefix(&strings[0])`
  reads the uncorrected input (main.rs:754/1265/1639). The holder is told to
  look for a prefix that no plate carries.

Design concern (ungraded): R4 compares `count` only; two chunk sets with the
same txid-derived set id and count (re-engraving the same tx) can still be
spliced, the documented witness-coverage limit.

r1: 0C/0I/2M/2N
