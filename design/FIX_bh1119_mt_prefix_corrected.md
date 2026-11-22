# FIX: set prefix from corrected text, counts from printed plates (bughunt-1119 FIX table row 23, "FIX-F prefix from corrected text")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Keys: bh-mt-I8 (I),
bh-mt-I21 (I), bh-mt-I7 (M); sweep-1122 rows 13 `mt-elide-prefix-counts` and 14
`mt-decode-prefix-after-bch-correction` (both I). Rulings T14 and T19
(`coordinator/agent-reports/architect-bughunt-1119-questions.md:52, :57`), binding.
Track: small (one Opus review round). Base: origin/main `1f6cc6b`. Branch
`fix/mt-prefix-corrected`. No new holder-visible text: every printed template
stays as it is; only the values printed into three of them change (section 4).

Not to be confused with the older `design/FIX_bughunt_mt_*.md` (a different hunt).

## 1. Findings (run at 1f6cc6b, `target/debug/mt`, `--bitcoin-cli /nonexistent/bitcoin-cli`)

Fixtures: the `even` vector of `crates/mt-codec/src/test_vectors/mt1_v1.json` (6
strings, prefix `mt1p9h8jqq9`) and `crates/mt-cli/tests/fixtures/p5_base.json`
`finalized_psbt_b64` (9 strings, prefix `mt1pgej7qqg`). All signet test material.

- **I8 / row 14 (printed prefix is the typed one).** `even` with string 1 offset 7
  `j` read as `x` (inside the prefix, within budget). `mt decode` exits 0 and prints
  `mt1 SET   6 strings, 1..6 all present, all begin mt1p9h8xqq9` while its own
  CORRECTION block says `pos   8   read x, corrected to j`. `--json` prints
  `"set_prefix": "p9h8xqq9",`; `mt inspect` prints the same in both forms. Cause:
  `main.rs:1312` (decode) and `main.rs:1698` (inspect) call
  `pipeline::invariant_prefix(&strings[0])`, which maps the typed symbols
  (`pipeline.rs:372-387`). A second shape: an unreadable first line followed by
  the whole set decodes (the set completes without it) and prints
  `all begin mt1zzzzzzqq`, the prefix of the line mt could not read.
- **I7 (elided lines inherit the full plate's typed prefix).** `restore_elided`
  (`read_strings.rs:608`) copies `full[..11]` as typed, before any BCH. With the
  prefix damage above on the one full plate and strings 2-6 elided: exit 0, but
  CORRECTION reports `6 chunks needed repair` and `pos   8   read x, corrected to j`
  under every chunk, on five plates that carry no position 8. Add 4 in-budget
  substitutions to elided plate 2: `mt decode` and `mt verify` exit 1
  `REFUSED — §1.1, string 2 could not be read: more than 4 characters differ from
  what was engraved` although every plate is within its budget. The same spill
  happens through transliteration: a typed `b` for `8` at position 7 of the full
  plate is reported six times (`pos   7   you typed b, mt read it as 8`).
- **I21 / row 13 (encode counts the unelided strings).** `mt encode --elide-prefix`
  on p5_base prints `CUT       9 strings, 787 characters` and `Count each string:
  strings 1-8 are 88, string 9 is 83.`; stdout holds 699 characters on plates of
  88, 77 (x7) and 72. Cause: `main.rs:747` `lengths` is taken from the unelided
  `strings`, used by `correction_coverage` (`:753`) and the CUT sum (`:778`), while
  `render` (`main.rs:942-963`) drops 11 characters from strings 2..n. The sweep
  grader (sweep-1122 grade-mt V2) names the count as the operator's only check for
  a missing or extra character (`blocks.rs:85-89`).

## 2. Rulings (binding, quoted)

- **T14 mt-Q6 = sweep MT-Q2 (I21, row 13): CUT counts what is printed** (the elided
  strings); the PREFIX row keeps stating the full-string prefix the holder
  restores. No refusal.
- **T19 mt-Q11 (I8, row 14): render the prefix from the BCH-corrected symbols,**
  which is what a clean plate carries.

I7 has no T-ruling of its own; grade-mt files it under FIX-F (`grade-mt.md:64, 95`)
and the triage's direction (`triage-mt-t2.md:17`) is T19's rule applied to the
restore source. Rule P2 applies T19 there; section 9 O1 names the one extension.

## 3. Behaviour after the fix (rules)

- **P0 (codec).** `mt_codec::string_layer::pipeline::DecodedChunk` gains
  `pub invariant_prefix: String`: the first `INVARIANT_PREFIX_SYMBOLS` (8) symbols
  of the chunk's data part **after** BCH correction, as lowercase charset
  characters, without `mt1`. `decode_chunk` fills it from the same symbol array it
  builds the header from (`pipeline.rs:172-197`, the shadowed `symbols`), so it
  agrees with `header` by construction. `pipeline::invariant_prefix` stays (encode
  uses it on minted strings, where it is correct).
- **P1 (I8, T19, row 14).** `decode` and `inspect` set `r.set_prefix` from the
  set's chunk 1, i.e. `set.chunks[0].invariant_prefix` (`set.chunks` is in header
  index order, `pipeline.rs:344-357`). Never from `strings[0]`. After a successful
  `pipeline::decode` it is always present, so on these two verbs the SET row always
  carries `all begin mt1<prefix>` and JSON `set_prefix` is never `null`. The
  `None` branch of `Report::render` / `render_json` stays (other constructors use it).
  All chunks of a decoded set share this value (version is checked by
  `header.rs:101`, set id and count equal by `pipeline.rs:292-304`), so the choice of chunk 1 is not a choice.
- **P2 (I7).** `restore_elided` keeps its full-line selection
  (`read_strings.rs:531-535`, the FIX-C order) and its refusals unchanged. Between
  the `full.len() < ELIDED_DROP` refusal (`:599`) and the typed-prefix
  extraction (`:608`) it resolves the full line exactly as step 3 of `read`
  resolves any line (`read_strings.rs:128-160`): `decode_chunk(full, None)` if it
  is Ok, else the `positional_autocorrect(full)` candidate that decodes with the
  fewest `corrected` (first on ties, as `min_by_key` there). If either resolves,
  the restore prefix is `"mt1"` + that chunk's `invariant_prefix`. If neither
  does, the prefix is the typed `full[..11]` exactly as today, including the
  non-char-boundary refusal at `:608-631` (CONTROL row 12). The builder factors
  the step-3 resolution into one helper both call, so the two cannot drift.
  Effect: a repair or transliteration is reported once, on the plate that carries
  it; an elided plate spends its own budget only.
- **C1 (I21, T14, row 13).** In `encode`, the per-string counts are the printed
  plates' lengths: for string `i` (1-based), its character count, minus 11
  (`"mt1".len() + INVARIANT_PREFIX_SYMBOLS`, the `drop` of `render`) when
  `--elide-prefix` is set and `i > 1`. Group separators are never counted (as
  today). Both the `correction_coverage` spans ("Count each string: ...") and the
  CUT total use this one list. The builder derives it from the same constant
  `render` uses (one definition). A one-string set under `--elide-prefix` is
  unchanged (nothing is elided). `--qr` with `--elide-prefix` is refused today
  (`main.rs:213` clap `conflicts_with`, exit 2) and stays refused.
- **C2 (T14).** The PREFIX row is unchanged: `PREFIX    all N strings begin
  mt1<prefix> — strings sharing that` / `          prefix belong together`, with the
  full-string prefix, under `--elide-prefix` too.

Exit codes: unchanged everywhere. The I7 case that exited 1 now exits 0 because the
input is within budget (no refusal is added or removed from `refusals.toml`).

## 4. Holder-visible texts

No template changes. Values that change (p5_base, `--elide-prefix`, with or
without `--group-size`):

    CUT       9 strings, 699 characters
    ... Count each string: string 1 is 88, strings 2-8 are 77, string 9 is 72.

(was `787` and `strings 1-8 are 88, string 9 is 83.`; the warning block wraps, so
tests match it whitespace-flattened). Decode/inspect SET row and JSON `set_prefix`
show the corrected prefix (`mt1p9h8jqq9` / `p9h8jqq9`). CORRECTION and
CHARACTERS-READ-DIFFERENTLY blocks list fewer chunks (section 1, I7).
PROPOSED texts: none.

## 5. Spec, README, help, manual, CHANGELOG

- `design/SPEC_mt_v0_1.md` is a copy owned by mnemonic-engrave (CLAUDE.md). It
  states neither the restore source nor the elided counts (§3b at :906-958 says
  "takes the prefix from string 1" and "restores the prefix and only then parses,
  checksums or corrects", which concerns the elided lines and still holds; the
  CUT example at :747 is unelided). No spec change; section 9 O3.
- README, `--help`, `scripts/journeys.sh` (reads only the CUT string count, :134):
  no change. Toolkit manual: has no mt chapter (`40-cli-reference/` lists 41-46,
  none mt); no lockstep text.
- CHANGELOG: none in this FIX (entries are written at release, commit 78f03c9).
  mt-codec gains a public field (P0); the version question belongs to release prep.
  mnemonic-engrave pins mt-codec by rev and constructs no `DecodedChunk`
  (`me-cli/src/sysw/mt.rs` uses `pipeline::decode` and reads fields), so it is
  unaffected until it bumps the pin.

## 6. RED rows

Sidecar: `design/FIX_bh1119_mt_prefix_corrected/prefix_corrected.rs.txt` (sha256
`9652db473e5411c781607e9f0c9df4d654c494a44fcb6e95d23828aba0b94676`, rustfmt
edition 2024 clean, clippy `-D warnings` clean when installed). Install as
`crates/mt-cli/tests/prefix_corrected.rs`, then:

    cargo nextest run --locked --build-jobs 6 -j 6 -p mt-cli --test prefix_corrected --no-fail-fast

At 1f6cc6b: `15 tests run: 3 passed, 12 failed` (the 3 are CONTROL rows). Each
failure is at the row's first assertion, for the defect:

| # | row (`bh1119_mt_f_` omitted) | input | base (RED) | after fix |
|---|---|---|---|---|
| 1 | decode_prefix_is_corrected | even, s1 off7 j->x; decode | `all begin mt1p9h8xqq9` | `mt1 SET   6 strings, 1..6 all present, all begin mt1p9h8jqq9` |
| 2 | decode_json_prefix_is_corrected | same; decode --json | `"set_prefix": "p9h8xqq9",` | `"set_prefix": "p9h8jqq9",` |
| 3 | inspect_prefix_is_corrected | same; inspect | `...begin mt1p9h8xqq9` | `...begin mt1p9h8jqq9` |
| 4 | inspect_json_prefix_is_corrected | same; inspect --json | `p9h8xqq9` | `"set_prefix": "p9h8jqq9",` |
| 5 | prefix_not_from_unreadable_line | unreadable line, then even; decode | `all begin mt1zzzzzzqq` | `all begin mt1p9h8jqq9` |
| 6 | elided_restore_uses_corrected_prefix | s1 damaged as row 1, s2-6 elided; decode | `6 chunks needed repair`, pos 8 x6 | `CORRECTION APPLIED. 1 chunk needed repair:`, pos 8 once |
| 7 | full_plate_prefix_damage_does_not_spend_elided_budget | row 6 + 4 subs on elided s2; decode | exit 1, string 2 refused | exit 0, stdout = clean decode, `2 chunks needed repair:` |
| 8 | verify_full_plate_prefix_damage_within_budget | row 7 input; verify | exit 1 | `mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.` |
| 9 | transliterated_prefix_noted_once | s1 off6 8->b, s2-6 elided; decode | `you typed b, mt read it as 8` x6 | x1 |
| 10 | cut_counts_elided_characters | p5; encode --elide-prefix | `CUT       9 strings, 787 characters` | `CUT       9 strings, 699 characters` |
| 11 | count_each_string_matches_elided_plates | same | `strings 1-8 are 88, string 9 is 83.` | `string 1 is 88, strings 2-8 are 77, string 9 is 72.` |
| 12 | undecodable_full_line_falls_back_to_typed_prefix (CONTROL) | s1 with 5 subs, clean s1, s2-6 elided; decode | exit 0, stdout = clean | same |
| 13 | cut_ignores_group_separators | p5; --elide-prefix --group-size 4 | 787 / 88-83 | 699 / 88-77-72 |
| 14 | prefix_row_unchanged_under_elision (CONTROL) | p5; --elide-prefix | `PREFIX    all 9 strings begin mt1pgej7qqg — strings sharing that` | same |
| 15 | unelided_counts_unchanged (CONTROL) | p5; encode | 787, `strings 1-8 are 88, string 9 is 83.` | same |

GREEN evidence (author's scratch fix, not committed; it is not the builder's
code): with P0-P2 and C1 applied the 15 rows pass and the whole workspace runs
`416 tests run: 416 passed` (402 on base + 14 at the time; row 8 was added after,
and passes with the same scratch: `15 tests run: 15 passed`). So **no existing
test changes**; base plus this file is 417.

Also wanted (GREEN-only, cannot be RED because the field does not exist on base):
a unit test in `pipeline.rs` `mod tests`,
`decoded_chunk_invariant_prefix_is_the_corrected_one`: `decode_chunk` of the even
string 1 with offset 7 `j`->`x` returns `invariant_prefix == "p9h8jqq9"`, and of the
clean string returns the same as `invariant_prefix(&clean)`.

## 7. Tests that change

None (section 6 GREEN evidence). Added: `crates/mt-cli/tests/prefix_corrected.rs`
(15 rows, verbatim from the sidecar) and the one codec unit test above.
`refusals.toml`: unchanged (no refusal added or removed).

## 8. Build order

1. Install the sidecar; run the row command; see `15 tests run: 3 passed, 12 failed`.
2. P0, then the codec unit test (GREEN).
3. P1: rows 1-5 GREEN.
4. P2 (with the shared step-3 helper): rows 6-9 GREEN, row 12 stays GREEN.
5. C1: rows 10, 11, 13 GREEN; 14, 15 stay GREEN.
6. Gate (section 10) once to a file.

## 9. Open points for the architect (no STOP)

- **O1.** Row 9 (transliteration) extends I7 from BCH repairs to the step-3
  transliteration of the full line. Same T19 principle, same code path; cut row 9
  and P2's autocorrect half if the architect wants only the triaged case.
- **O2. New, out of scope (proposed I, for the grader).** On an elided plate the
  CORRECTION and transliteration positions count the RESTORED string: one
  substitution at the 6th character of elided plate 2 is printed `pos  17   read
  w, corrected to d` (run at 1f6cc6b). The spec rule (SPEC :1091-1095, "counted over
  the whitespace-stripped string including the HRP") predates elision and is
  silent on it. A printed position the holder matches against steel, 11 off. Not
  fixed here: what to print (plate position, or whole-string position with a
  note) is a wording decision. Filed as a proposed FOLLOWUPS line in the author's
  report.
- **O3.** If the architect wants the restore source stated in the spec (§3b: "the
  prefix is taken from string 1 as BCH and transliteration resolve it"), that is an
  engrave spec edit and a re-sync, not part of this FIX.

## 10. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
The CI `test` job (`.github/workflows/ci.yml`) plus the rows:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expect 418 run, 0 failed: 402 base
   + 15 rows + 1 codec unit test; if another mt FIX lands first, base count moves
   by its own additions)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh` (expect `42 refusal tests over 18 ruled
   refusals`, as at 1f6cc6b)
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh` and `bash ci/code-docs-check.sh`
10. Rows by name: `cargo nextest run --locked -p mt-cli --test prefix_corrected`
    -> `15 tests run: 15 passed`. Fewer than 15 is a FAIL.
11. `grep -n 'invariant_prefix(&strings\[0\])' crates/mt-cli/src/main.rs` -> no output (P1).

## 11. FOLLOWUPS (listed in the author's report, not appended here)

Closes `decode-prefix-after-bch-correction` (`design/FOLLOWUPS.md:46`); files O2.
