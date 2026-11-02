**⚠ 4 Important (I1-I4): the doc as written cannot reach GREEN (10d cannot pass, the rebase conflicts, 07d cannot fail, gate step 7 fails).**

# FIX review r1, X = C (mt bughunt FIX-C doc)

Reviewer: Opus, effort medium (per brief). W = mt-worktrees/bughunt-fix-c, head b84798c.
Scratch: /scratch/cache/bughunt/mt-fix-review-c/src (C head b84798c merged with B head 7820750,
then FIX-B §2 applied, then FIX-C §1-§2 applied from the doc text alone).
This is the first review round of this doc; no finding comes from an earlier fix commit.

## Findings (appended as found)


### I1 (Important): RED row 10d cannot turn GREEN; its `needed` filter also matches the duplicate entry

- Location: `crates/mt-cli/tests/bughunt.rs:583` (W head b84798c), in
  `bughunt_mt_10d_json_correction_entries_ordered_and_no_margin_left`:
  `let is_needed = |x: &String| x.starts_with("chunk ") && x.contains(" corrections");`
- Failure: the duplicate entry's ruled text (§1 spec paragraph, and what the
  code prints) is `chunk 3 was present twice; kept the copy needing 0
  corrections`. It starts with `chunk ` and contains ` corrections`, so the
  filter counts it as a `needed` entry. On the scratch copy with FIX-B and
  every FIX-C fix applied exactly, the second (duplicate) input fails:
  `left: ["chunk 3 was present twice; kept the copy needing 0 corrections",
  "chunk 2 needed 4 of 4 corrections; NO MARGIN LEFT", "chunk 1 needed 1 of
  4 corrections"]` vs `right: [chunk 2 ..., chunk 1 ...]`. The RED command
  gives 15 run, 14 passed, 1 failed (`green.log` in the scratch dir). No
  correct implementation can pass it, so a Sonnet builder either STOPs or
  edits the row on its own judgement. The decode-ordering branch had never
  run GREEN (the fold ran RED on base only).
- Fix text: in `crates/mt-cli/tests/bughunt.rs`, row 10d, replace
  `let is_needed = |x: &String| x.starts_with("chunk ") && x.contains(" corrections");`
  with
  `let is_needed = |x: &String| x.starts_with("chunk ") && x.contains(" needed ");`
  (this matches §3's wording "the `needed` entries"). Ran it: the RED command
  as printed in §3 then gives 15 run, 15 passed.

### I2 (Important): Build order step 1 says the rebase is clean and to STOP otherwise; it conflicts

- Location: `design/FIX_bughunt_mt_c.md:29-31` (Build order step 1).
- Failure: FIX-B's RED commit and FIX-C's RED commit e836360 both append at
  the end of `crates/mt-cli/tests/bughunt.rs`. Ran it: a clone of W, `git
  rebase` of b84798c onto FIX-B head 7820750 stops on commit 1 of 4 with
  `UU crates/mt-cli/tests/bughunt.rs` (conflict markers at 405/436/706).
  FIX-B's GREEN head will only add to that file. The doc tells a Sonnet
  builder to STOP, so the build cannot start from the document alone.
  (The step's premise is also off: the RED commit touches `src/report.rs`,
  a `#[cfg(test)]` block, not only test code.)
- Fix text: replace "The RED commit adds only test code and this document,
  so the rebase is expected to be clean; if it is not, STOP and report the
  conflict." with: "The rebase conflicts in `crates/mt-cli/tests/bughunt.rs`,
  where both branches append at the end of the file: keep both blocks,
  FIX-B's first (`bughunt_mt_04_cli_count_mismatch_refusal` and anything
  above it) and then this document's section, delete the three conflict
  marker lines, `git add` the file and `git rebase --continue`. Any other
  conflict: STOP and report it." Ran it: deleting the three marker lines
  and continuing finishes the 4-commit rebase, and the merged file compiles
  and passes all 15 FIX-C rows plus B's rows once both fixes are applied.

### I3 (Important): row 07d cannot fail when QC2's option (c) is missing

- Location: `crates/mt-cli/tests/bughunt.rs:493-496` (W head b84798c), the end of
  `bughunt_mt_07d_second_full_line_supplies_the_prefix`; doc §3 row 07d
  (`design/FIX_bughunt_mt_c.md:264`).
- Failure: QC2 (doc lines 99-100) rules that with an intact second full line
  decode takes the MAIN path (option c: pick the first `mt1` line whose byte 11
  is a char boundary). 07d asserts only rc 1, empty stdout and the first
  stderr line, which option (b)'s fallback refusal prints identically. Planted
  the fault on the fixed scratch copy (selection back to
  `.find(|s| s.starts_with("mt1"))`, i.e. (b) without (c)): all four 07 rows,
  07d included, still PASS. A builder who skips (c) ships GREEN and the row
  never notices.
- Fix text: in row 07d, after the final `assert_eq!(... first line ...);`, add
  (rustfmt form, so gate step 2 passes)

  ```
      let flat = e.split_whitespace().collect::<Vec<_>>().join(" ");
      assert!(
          flat.contains("read cleanly and 1 did not"),
          "not the main path:\n{e}"
      );
  ```

  and in §3's 07d expected-output cell append: "; stderr (whitespace-flattened)
  contains `read cleanly and 1 did not` (the main path's mechanism)". Ran it:
  GREEN 4/4 07 rows pass (`i3-green.log`); with the plant 07d FAILS "not the
  main path" (3 passed, 1 failed, `i3-plant.log`).

## Plants (fixed scratch copy, I1 and I3 fix texts applied; each plant reverted after its run; logs in `/scratch/cache/bughunt/mt-fix-review-c/plants/`)

| plant | rows that fail |
|---|---|
| `restore_elided` raw slice `&full[..ELIDED_DROP]` (07c) | 07, 07b, 07c (07d passes: main path, as ruled) |
| drop option (c) boundary-safe selection | 07d only, and only with the I3 assertion (before it: none) |
| drop inspect duplicate loop (10b) | 10, 10b |
| `correction_warnings` ascending sort | 10d |
| decode: corrections before duplicates (10d decode-ordering branch) | 10d |
| decode: no `correction_warnings` | 10, 10c, 10d |
| `esc` uppercase `\u{:04X}` (10e) | 10e |
| drop decode `json_quiet_guard` call (11b) | 11, 11b |
| drop inspect `quiet_unsupported_guard` call | r11 (both rows) |
| drop verify `quiet_unsupported_guard` call | r11 (both rows) |
| revert R12 patch (12b) | 12, 12b |

So 10d's decode-ordering branch and 10e's lowercase assertion are proved (once I1 lets 10d pass at all).

### I4 (Important): gate step 7 fails with the doc's four `refusals.toml` entries; 07d needs a coverage decision the doc does not make

- Location: `design/FIX_bughunt_mt_c.md:214-238` (§2.5, the four entries),
  §4 (the file list) and §5 step 7; the 07d row at
  `crates/mt-cli/tests/bughunt.rs:493-496` asserts on
  `mt decode: REFUSED — §1.1, ...`.
- Failure: on the scratch copy with FIX-B and all of FIX-C applied, gate
  step 7 `./scripts/check-refusal-coverage.sh` exits non-zero:
  `bughunt_mt_07d_second_full_line_supplies_the_prefix (crates/mt-cli/tests/bughunt.rs)
  asserts on a §-refusal message, but has no entry in refusals.toml -- add
  one, or add it to EXEMPT with a reason`. This fails with or without the I3
  assertion; the row's existing first-line assert is enough. The builder must
  choose, and both obvious choices are wrong. An entry with
  `check = "src/main.rs::explain_failure"` fails the script's shape check,
  because `explain_failure` returns `Refusal` and not `Result`/`Option`. An
  entry with `check = "src/read_strings.rs::restore_elided"` passes step 7,
  but it names the wrong guard: on 07d, `restore_elided` returns `Ok`. The
  doc's §4 also says step 7 fails only on the RED commit, which implies it
  passes after GREEN. It does not.
- Fix text:
  1. In §2.5, after the four-entry code block (before "The `ReadArgs.quiet`
     doc comment stays (R11)."), add:
     "`scripts/check-refusal-coverage.sh`: in the `EXEMPT` dict, after its
     last entry (FIX-B's `bughunt_mt_04_cli_count_mismatch_refusal`) and
     before the closing `}`, add:

     ```
         "bughunt_mt_07d_second_full_line_supplies_the_prefix":
             "the main path's §1.1 unreadable-string refusal, reached through "
             "explain_failure (it returns Refusal, so mutate-refusals.sh has no "
             "neutral return for it) once a later full line supplies the elided "
             "prefix (QC2 option c); the choice of line is held by row 07d itself",
     ```

     Row 07d is not given a `refusals.toml` entry. Its refusal comes from
     `explain_failure`, and `restore_elided` returns `Ok` on its input."
  2. In §4, after the `refusals.toml` bullet, add the bullet
     "- `scripts/check-refusal-coverage.sh`: one `EXEMPT` entry for row 07d
     (§2.5), in the GREEN commit."
- Ran it on the scratch copy:
  - `./scripts/check-refusal-coverage.sh` exits 0: "38 refusal tests over 18
    ruled refusals", and it lists the 07c, 11b and both r11 rows.
  - `./scripts/mutate-refusals.sh` (`mutate.log`) exits 0: "all 38 refusal
    tests go red when their check is removed". This covers `restore_elided`
    (§1.1), `json_quiet_guard` and `quiet_unsupported_guard` (twice).
  - `cargo fmt --check` exits 0.
  - The full `cargo nextest run --locked --no-fail-fast` (`full2.log`) gives
    302 run, 298 passed and 4 failed. The 4 failures are only A's and D's
    rows 01, 02, 03 and 05, as §5 step 5 expects.

## Gate on the fixed scratch copy (FIX-B + FIX-C + I1-I4 fix texts)

| step | result |
|---|---|
| 1 docs-only | ok |
| 2 fmt --check | ok after `cargo fmt` (see N6) |
| 3 clippy | ok |
| 4 build | ok |
| 5 nextest | 302 run / 298 pass; fails only A/D rows 01, 02, 03, 05 |
| 6 doc tests | ok |
| 7 check-refusal-coverage | ok only with the I4 fix; FAIL on the doc as written |
| 8 mutate-refusals | ok, 38/38 red without their check |
| 9 journeys | ok |
| 10 ci scripts (3) | ok |
| 11 check-provenance | SKIP: `../mnemonic-engrave` not found (see M2) |
| 12 §3 command | 15/15 with I1 |

Proved, as the brief asks:
- 10d's decode-ordering branch: the plants table shows it.
- 10e's lowercase assertion: the plants table shows it.
- Gate steps 7 and 8: both pass once the I4 fix is in.
- R10's remedy text: decode `--json` writes its JSON report to stderr, and
  decode `--quiet` writes the hex alone to stdout
  (`decode_warns_about_a_world_readable_stdout_and_still_exits_0`). So the
  remedy is accurate.

## Minor

- M1: The option (c) selection can pick a later `mt1` line with non-ASCII
  characters past byte 11 as the prefix source. This fails closed: the
  restored lines do not decode, so it refuses. It needs two unusual
  conditions: (1) non-ASCII damage on the first line inside its first 11
  bytes, and (2) a second full line with damage of its own. It is not a
  wrong result.
- M2: §5 step 11 cannot run as printed in a lone mt worktree. It prints
  "SKIP: ../mnemonic-engrave not found; set MT_SPEC_SOURCE", so the Q0
  mismatch the step expects is never observed. The step should name
  `MT_SPEC_SOURCE=<engrave checkout>/design/SPEC_mt_v0_1.md`, or say a SKIP
  is acceptable. It fails closed (SKIP, not a false ok).

## Nits

- N1: The §1 anchor quote spans a line wrap in the spec. A literal grep for
  it misses, though a careful reader finds it.
- N2: Row 10c's `needed` filter has the same loose pattern as the one I1
  fixes. It passes only because 10c has no duplicate input. Use
  `contains(" needed ")` there too.
- N3: §2.1 does not say how to compute `n`, the count of elided lines. It
  can be inferred from the restore loop.
- N4: An `mt1` first line shorter than 11 bytes is now skipped as a prefix
  source. That is correct, but the doc should say so.
- N5: "Add one function next to `margin_report`" (§2.4) can be read as
  "between `margin_report`'s doc comment and its `fn`". Say "above
  `margin_report`'s doc comment".
- N6: §2.4's `format!` with an inline `if` fails `cargo fmt --check` as
  printed. `cargo fmt` fixes it; the doc could say to run `cargo fmt` before
  step 2.
- N7: The existing "only {} characters" message counts with `full.len()`,
  which is a byte count, not a character count. This predates the fix and is
  outside its scope.

Design (ungraded): 07d's refusal can only be held by an EXEMPT entry. A
`Result`-returning wrapper around the main path's unreadable-string refusal
would let mutate-refusals.sh neuter it like the other refusals.

C / I / M / N = 0 / 4 / 2 / 7
r1: 0C/4I/2M/7N
