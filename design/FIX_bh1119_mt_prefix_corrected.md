# FIX: set prefix from corrected text, counts from printed plates (bughunt-1119 FIX table row 23, "FIX-F prefix from corrected text")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Keys: bh-mt-I8 (I),
bh-mt-I21 (I), bh-mt-I7 (M); sweep-1122 rows 13 `mt-elide-prefix-counts` and 14
`mt-decode-prefix-after-bch-correction` (both I). Rulings T14 and T19
(`coordinator/agent-reports/architect-bughunt-1119-questions.md:52, :57`), binding.
Track: small (one Opus review round). Base: origin/main `1f6cc6b`. Branch
`fix/mt-prefix-corrected`. One new holder-visible line template (rule E1 (c), the `string N of the input`
line, Brian #23 Q3); every other printed template stays as it is and only the
values printed into them change (section 4). Fold r1
(2026-11-22) adds Brian's elided-plate ruling (section 2, rules E1-E2, spec delta
in section 5) and review r1's I1 and M1. Fold r1b (2026-11-22) adds Brian's
`--json` ruling (E1 extended, rows 21-22) and measures Q2 (reachable; section 9).
Fold r1c (2026-11-22) adds Brian's Q2 ruling (E1, rows 23-25) and records Q3, the
CORRECTION-block layout the ruling leaves open. Fold r1d (2026-11-22) adds Brian's
Q3 ruling (E1, rows 26-29; section 9 Q3 settled).

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

- **Brian 2026-11-22 (#23, `coordinator/memory/mt-elided-plates-ruling-2026-11-22.md`),
  binding.** Elided plates stay (`mt encode --elide-prefix` is kept): "the text
  that is not present is presumably available on a separate plate and could be
  supplied when necessary". On an elided plate, CORRECTION and transliteration
  rows print the PLATE position, counted from the first character engraved on
  that plate (the 6th plate character prints `pos 6`, not 17). With no full plate,
  the v0.1 refusal naming the 8 characters after `mt1` stays; the 2^20
  chunk_set_id recovery search is a later feature (section 11), not built here.
- **Brian 2026-11-22 (#23 Q1, `coordinator/memory/mt-json-plate-position-2026-11-22.md`),
  binding.** The `--json` warning `pos N: you typed X, mt read it as Y` in
  `mt decode --json` and `mt inspect --json` uses the plate position on an elided
  line, matching the human CORRECTION/transliteration rows: "a holder must never
  see two different positions for one character".
- **Brian 2026-11-22 (#23 Q2, `coordinator/memory/mt-prefix-typo-once-on-full-plate-2026-11-22.md`),
  binding (as the coordinator's brief for fold r1c records it).** "When a
  correction or transliteration falls inside the restored prefix (whole-string
  positions 1-11 on the one full `mt1` plate) and elided plates inherit that
  prefix, the CORRECTION row, the transliteration row and the `--json` warning
  print ONCE, on the full plate, at its whole-string position there (e.g. `pos
  8`). Elided plates print no row for it. No negative, duplicated or underflowed
  positions." Rejected: repeating it on every plate (labelled), and refusing.
- **Brian 2026-11-22 (#23 Q3, `coordinator/memory/mt-unreadable-full-plate-correction-layout-2026-11-22.md`),
  binding (as the coordinator's brief for fold r1d records it).** Input A (the one
  full `mt1` plate is UNREADABLE; the set completes from elided plates that
  inherited its prefix typo): "(a) the CORRECTION header keeps the real chunk
  count (`6 chunks needed repair`); (b) one `pos   8   read n, corrected to j` row
  prints ONCE, under a line that names that string (the full plate, string 1 of
  the input); (c) each chunk line still prints its `N of 4 symbols`, with no `pos`
  row for the inherited prefix symbol; (d) the `--json` correction warnings (no
  position) stay unchanged." The exact text of the line in (b) is the FIX
  author's (rule E1). Rejected: dropping prefix-only chunks from the count.

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
  extraction (`:608`) it resolves lines exactly as step 3 of `read`
  resolves any line (`read_strings.rs:128-160`): `decode_chunk(line, None)` if it
  is Ok, else the `positional_autocorrect(line)` candidate that decodes with the
  fewest `corrected` (first on ties, as `min_by_key` there). P2 resolves each
  candidate beginning `mt1`, in input order, and takes the prefix from the first
  that resolves (`"mt1"` + its `invariant_prefix`). If none resolves, the prefix
  is the typed `full[..11]` of the line the existing selection picks, exactly as
  today, including the non-char-boundary refusal (CONTROL row 12). The builder factors
  the step-3 resolution into one helper both call, so the two cannot drift.
  Effect: a repair or transliteration is reported once, on the plate that carries
  it; an elided plate spends its own budget only.
- **E1 (Brian 2026-11-22, rows 17-19, 21-29).** An elided line is a line of the input
  that does not begin `mt1` (§3b detection, unchanged). In every CORRECTION row
  (`main.rs:1128`, `              pos {:>3}   read X, corrected to Y`) and every
  transliteration row (`main.rs:1180`, `  pos {:>3}   you typed X, mt read it as
  Y`) whose chunk was read from an elided line, the number printed is the plate
  position: the whole-string position minus 11 (`ELIDED_DROP`), i.e. a data-part
  offset `p` prints `p + 1 + 3 - 11` and a codeword index `k` prints `k - 7`.
  Position 1 is the first character of the index, as engraved; grouping spaces are
  not counted (unchanged). A full line keeps its whole-string position (the two
  counts coincide on a plate that begins `mt1`; CONTROL row 19). The templates and
  the `{:>3}` width are unchanged. The rule applies wherever those rows print
  (`decode`, `verify`, `inspect`: the `transliteration_notices` and `margin_report`
  call sites `main.rs:1388-1390`, `:1438-1440`, `:1768-1770`). When a chunk is
  present twice, its rows describe the copy kept (`set_notices`' KEPT line), so
  they count that copy's line. The same number is printed in the `--json`
  transliteration warning string `pos N: you typed X, mt read it as Y` of
  `decode --json` (`main.rs:1331`, document on stderr) and `inspect --json`
  (`main.rs:1712`, document on stdout): on an elided line N is the plate
  position, the same N the human row prints for that character (Brian
  2026-11-22, #23 Q1; rows 21-22). Its template and the other warnings are
  unchanged; `correction_warnings` (`main.rs:1060`) prints no position and is
  untouched. **Inside the restored prefix (Brian 2026-11-22, #23 Q2; rows 23-25).**
  A transliteration or CORRECTION at whole-string position 4-11 of an elided line
  (data-part offset `p < 8`, all of it inherited from the full plate's typed
  prefix, which happens only when P2 falls back) prints ONCE, at that whole-string
  position (e.g. `pos   7`, `pos   8`), counting the full plate; the elided lines
  print no row and no `--json` warning for it. So input B of section 9 prints one
  `  pos   7   you typed b, mt read it as 8` row and one `--json` warning
  `pos 7: you typed b, mt read it as 8`, where the base prints six of each. No
  printed position is ever negative, zero or duplicated. The builder must not
  compute an elided plate position in `usize` for a data-part offset below 7
  (`p + 1 + 3 - 11` underflows there); a position inside the prefix is never
  turned into a plate position at all. **Unreadable full plate (Brian 2026-11-22,
  #23 Q3; rows 26-29).** When P2 falls back to the typed prefix of a line that is
  itself unreadable (no chunk of the set; section 9 input A) and the elided chunks'
  BCH corrects a symbol inside that inherited prefix: (a) the header `CORRECTION
  APPLIED. N chunks needed repair:` counts every chunk that spent a symbol, the
  inherited one included; (b) each such chunk keeps its `  chunk {:>3}   k of 4
  symbols` line, `k` still counting the inherited symbol (the `<-- NO MARGIN
  LEFT` tag and the correction-limit paragraph as today), and prints no `pos` row
  for the inherited symbol (its own repairs outside the prefix keep their rows, at
  plate positions); (c) after the last chunk line and its rows, and before the
  correction-limit paragraph when there is one, one new line names the plate:
  `  string {} of the input   its prefix was restored onto the elided strings`
  (2-space indent, as the chunk lines; `{}` is that line's 1-based place in the
  input, the same number its `UNREADABLE STRING. string {} of the input could not
  be read:` line prints), and under it, in the existing CORRECTION row template
  (14-space indent), one row per corrected prefix symbol at its whole-string
  position, in the order the chunk rows use (all chunks agree on these symbols,
  P1); (d) the `--json` `correction_warnings` (`chunk N needed k of 4
  corrections`, one per chunk, no position) are unchanged and no warning is
  added. The `string` line prints only in this case: when the full plate is
  readable, P2 takes its corrected prefix, the elided chunks inherit no typo, and
  the full plate's own chunk prints its row under its chunk line as today.
- **E2 (Brian 2026-11-22, CONTROL row 20).** With no full line the §3b refusal is
  unchanged: exit 1, stdout empty, its remedy `Add the 8 characters following
  `mt1` on any intact string of the same set. (They are recoverable by search if
  that string is lost — mt v0.1 does not implement the search.)`. No search is
  built.
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

No template changes except the one `string` line of rule E1 (c) (Brian #23 Q3).
Values that change (p5_base, `--elide-prefix`, with or
without `--group-size`):

    CUT       9 strings, 699 characters
    ... Count each string: string 1 is 88, strings 2-8 are 77, string 9 is 72.

(was `787` and `strings 1-8 are 88, string 9 is 83.`; the warning block wraps, so
tests match it whitespace-flattened). Decode/inspect SET row and JSON `set_prefix`
show the corrected prefix (`mt1p9h8jqq9` / `p9h8jqq9`). CORRECTION and
CHARACTERS-READ-DIFFERENTLY blocks list fewer chunks (section 1, I7). On an
elided plate (E1, `even` with strings 2-6 elided):

                  pos   6   read w, corrected to d
      pos  15   you typed o, mt read it as 0

and in the `--json` `warnings` array of `decode` and `inspect`:

    "pos 15: you typed o, mt read it as 0"

(were `pos  17`, `pos  26` and `"pos 26: ..."`). For a typo inside the restored
prefix (section 9 input B), the CHARACTERS-READ-DIFFERENTLY block of `decode`
holds one row and the `--json` `warnings` of `decode` and `inspect` hold one
string, at the full plate's position:

      pos   7   you typed b, mt read it as 8
    "pos 7: you typed b, mt read it as 8"

(were six of each). For section 9 input A (full plate unreadable, its prefix typo
inherited by all six elided chunks), the CORRECTION block of `decode`, `verify`
and `inspect` becomes (one new line template, the `string` line, rule E1 (c)):

    CORRECTION APPLIED. 6 chunks needed repair:
      chunk   1   1 of 4 symbols
      chunk   2   1 of 4 symbols
      chunk   3   1 of 4 symbols
      chunk   4   1 of 4 symbols
      chunk   5   1 of 4 symbols
      chunk   6   1 of 4 symbols
      string 1 of the input   its prefix was restored onto the elided strings
                  pos   8   read n, corrected to j

(was `              pos   8   read n, corrected to j` under each of the six chunk
lines and no `string` line). Its `--json` warnings are unchanged. PROPOSED texts:
none (the `string` line is authored under Brian's Q3 ruling).

## 5. Spec, README, help, manual, CHANGELOG

- `design/SPEC_mt_v0_1.md` is a copy owned by mnemonic-engrave (CLAUDE.md). It
  states neither the restore source nor the elided counts (§3b at :906-958 says
  "takes the prefix from string 1" and "restores the prefix and only then parses,
  checksums or corrects", which concerns the elided lines and still holds; the
  CUT example at :747 is unelided). For those, no spec change (section 9 O3).
- **Spec delta (Brian 2026-11-22, E1-E2).** Applied to mnemonic-engrave's
  `design/SPEC_mt_v0_1.md` (the owner) first; this FIX's build then re-syncs mt's
  copy byte-for-byte. Three edits, anchors as in mt's copy at 1f6cc6b:
  1. §3b, the sentence ending the "AN ELIDED STRING IS NOT A VALID `mt1` STRING"
     block (:936-937): `Nothing downstream of that restoration knows elision
     happened.` becomes `Nothing downstream of that restoration knows elision
     happened, except the positions it prints, which count the plate (below).`
  2. §3b, a new block quote paragraph after the "The prefix is not lost with the
     string that carried it" paragraph (after :952):
     `> **Elided plates stay, and an elided plate is reported in its own
     > coordinates.** Operator ruling 2026-11-22: the text that is not present on
     > an elided plate "is presumably available on a separate plate and could be
     > supplied when necessary". Where `mt` prints a position on an elided line
     > (a CORRECTION row, `pos N   read X, corrected to Y`, a transliteration
     > row, `pos N   you typed X, mt read it as Y`, or the `--json` warning
     > `pos N: you typed X, mt read it as Y` of `mt decode` and `mt inspect`),
     > N counts from the first character engraved on that plate: the 6th
     > character of an elided plate prints `pos   6`, not its place in the
     > restored string (17). A character inside the restored prefix is reported
     > once, where it is engraved: at its position on the full string that
     > supplied the prefix (e.g. `pos   8`), and no elided plate prints a row or
     > warning for it (operator ruling 2026-11-22). If that full string is itself
     > unreadable, each chunk still counts the symbol it spent, and its one row
     > prints under `string N of the input   its prefix was restored onto the
     > elided strings`, after the chunk lines (operator ruling 2026-11-22). With
     > no full string the refusal above stays; the search is a later feature.`
  3. The positions rule (:1091-1095): after `Where the operator engraved with
     grouping, positions do **not** count the spaces.` add `**On an elided line
     (§3b), CORRECTION and transliteration rows, and the `--json` transliteration
     warning, count the plate as engraved:**
     position 1 is the first character of the index, the restored prefix is not
     counted, and a BCH codeword index `k` is position `k − 7`. A character inside
     the restored prefix is reported once, at its position on the full string, and
     never on an elided line; when the full string is unreadable, that row prints
     under a line naming it (§3b).`
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
`389c569ca593c03d237ce34b261b3d778693ec7f6657e8c59089b756dc735b05`, rustfmt
edition 2024 clean, clippy `-D warnings` clean when installed). Install as
`crates/mt-cli/tests/prefix_corrected.rs`, then:

    cargo nextest run --locked --build-jobs 6 -j 6 -p mt-cli --test prefix_corrected --no-fail-fast

At 1f6cc6b (run on a `git archive` copy of 001f688, docs-only over it): `20 tests
run: 5 passed, 15 failed` (the 5 are CONTROL rows 12, 14, 15, 19, 20). Fold r1b
(rows 21-22 added; sidecar sha256
`f9ea479976b80cd1e5e5826307df5c592b2248295bed236c59eecd7bb4033278`), run on a
`git archive` copy of 59f021c: `22 tests run: 5 passed, 17 failed`. Fold r1c
(rows 23-25 added; sidecar sha256
`8b3ba4139d4b97484c9641ce97953e615197b98b55997871d9c01242e32fb30d`), run on
`git archive` copies of 59f021c and of 4c4f70f (identical code): both `25 tests
run: 5 passed, 20 failed`. Fold r1d (rows 26-29 added; sidecar sha256 as above),
run on a `git archive` copy of 59f021c (code identical to 9c93fa4): `29 tests run:
6 passed, 23 failed` (the 6 are CONTROL rows 12, 14, 15, 19, 20, 29). Each
failure is at the row's first assertion after the exit-status check, for the
defect:

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
| 16 | second_full_copy_supplies_prefix (P2, review r1 M1) | s1 damaged at 7,20,25,30,35, clean s1, s2-6 elided, 4 subs on elided s2 (offsets 5,10,15,20); decode | exit 1 `REFUSED — §1.1, string 1, 3 could not be read: more than 4 characters differ from what was engraved` | exit 0, `CORRECTION APPLIED. 1 chunk needed repair:` |
| 17 | elided_correction_prints_plate_position (E1) | even, s2-6 elided, elided s2 offset 5 (6th plate char) `d`->`w`; decode | `              pos  17   read w, corrected to d` | `              pos   6   read w, corrected to d` |
| 18 | elided_transliteration_prints_plate_position (E1) | even, s2-6 elided, elided s2 offset 14 `0` typed `o`; decode | `  pos  26   you typed o, mt read it as 0` | `  pos  15   you typed o, mt read it as 0` |
| 19 | full_plate_correction_keeps_string_position (E1, CONTROL) | even unelided, s2 offset 16 `d`->`w`; decode | `              pos  17   read w, corrected to d` | same |
| 20 | all_elided_still_names_the_prefix (E2, CONTROL) | even, all six lines elided; decode | exit 1, stdout empty, remedy `Add the 8 characters following `mt1` on any intact string of the same set. (They are recoverable by search if that string is lost — mt v0.1 does not implement the search.)` (whitespace-flattened) | same |
| 21 | decode_json_elided_transliteration_prints_plate_position (E1, Q1 ruling) | row 18's input; decode --json (document on stderr, parsed) | `warnings` = `["no bitcoind reachable: nothing here is confirmed against the chain", "pos 26: you typed o, mt read it as 0"]` | `warnings` = `["no bitcoind reachable: nothing here is confirmed against the chain", "pos 15: you typed o, mt read it as 0"]` (exactly) |
| 22 | inspect_json_elided_transliteration_prints_plate_position (E1, Q1 ruling) | row 18's input; inspect --json (document on stdout, parsed) | same as row 21, `"pos 26: ..."` | same as row 21, `"pos 15: ..."` (exactly) |
| 23 | prefix_transliteration_inherited_by_elided_prints_once (E1, Q2 ruling) | section 9 input B: even s1 subs at 20, 25, 30, 35, 40 and off6 `8` typed `b` (unreadable), then s1-s6 all elided; decode | `  pos   7   you typed b, mt read it as 8` x6 (`left: 6`, `right: 1`) | exactly one `  pos   7   you typed b, mt read it as 8` line, exactly one `you typed` in stderr |
| 24 | decode_json_prefix_transliteration_inherited_by_elided_prints_once (E1, Q2 ruling) | input B; decode --json (document on stderr, parsed) | `pos` warnings = `"pos 7: you typed b, mt read it as 8"` x6 | `pos` warnings = `["pos 7: you typed b, mt read it as 8"]`; the others = `["no bitcoind reachable: nothing here is confirmed against the chain", "string 1 could not be read: character 'b' at data-part offset 3 is not in the bech32 alphabet"]` (exactly; order across the two groups not asserted) |
| 25 | inspect_json_prefix_transliteration_inherited_by_elided_prints_once (E1, Q2 ruling) | input B; inspect --json (document on stdout, parsed) | same as row 24 | same as row 24 |
| 26 | unreadable_full_plate_prefix_correction_prints_once (E1, Q3 ruling) | section 9 input A: even s1 next-symbol subs at 7, 20, 25, 30, 35 (unreadable, prefix typed `mt1p9h8nqq9`), then s1-s6 all elided; decode | `pos   8   read n, corrected to j` x6 (`left: 6`, `right: 1`) | exactly one `pos   8   read n, corrected to j` row; each `  chunk   N   1 of 4 symbols` (N = 1..6) once; stderr contains exactly once the section 4 input A block, from its leading blank line through the `pos   8` row and the blank line after it |
| 27 | verify_unreadable_full_plate_prefix_correction_prints_once (E1, Q3 ruling) | input A; verify | same as row 26 | same as row 26 |
| 28 | inspect_unreadable_full_plate_prefix_correction_prints_once (E1, Q3 ruling) | input A; inspect | same as row 26 | same as row 26 |
| 29 | decode_json_unreadable_full_plate_correction_warnings_unchanged (E1, Q3 ruling (d), CONTROL) | input A; decode --json (document on stderr, parsed) | `warnings` = `["no bitcoind reachable: nothing here is confirmed against the chain", "string 1 could not be read: BCH correction failed: regular code: more than 4 substitutions or pathological pattern", "chunk 1 needed 1 of 4 corrections", ..., "chunk 6 needed 1 of 4 corrections"]` (exactly, eight) | same |

GREEN evidence (author's scratch fix, not committed; it is not the builder's
code): with P0-P2 and C1 applied the 15 rows pass and the whole workspace runs
`416 tests run: 416 passed` (402 on base + 14 at the time; row 8 was added after,
and passes with the same scratch: `15 tests run: 15 passed`). Review r1 measured
row 16 GREEN with that scratch plus the P2 candidates loop (`16/16`, workspace
`418 tests run: 418 passed`). Rows 17-18 and 21-28 (E1) have no GREEN run yet; rows 19-20
and 29 pass on base. No existing test asserts a position on an elided line (the two
position tests, `decode_verify.rs:303, :565`, and `journeys.sh:211` use full
plates), and none asserts a `--json` transliteration warning on an elided line
(`grep -rn 'you typed' crates/mt-cli/tests` at 59f021c), and none asserts a
repeated prefix row on elided lines, and none asserts a CORRECTION block or
`correction_warnings` for an unreadable full plate (`grep -rn 'of 4 symbols\|of 4
corrections' crates/mt-cli/tests` at 59f021c finds only `decode_verify.rs:300, :557`
and `bughunt.rs:1298, :1327-1328`, all on unelided full-plate sets). So **no existing
test changes**; base plus this file is 431.

Also wanted (GREEN-only, cannot be RED because the field does not exist on base):
a unit test in `pipeline.rs` `mod tests`,
`decoded_chunk_invariant_prefix_is_the_corrected_one`: `decode_chunk` of the even
string 1 with offset 7 `j`->`x` returns `invariant_prefix == "p9h8jqq9"`, and of the
clean string returns the same as `invariant_prefix(&clean)`.

## 7. Tests that change

None (section 6 GREEN evidence). Added: `crates/mt-cli/tests/prefix_corrected.rs`
(29 rows, verbatim from the sidecar) and the one codec unit test above.
`refusals.toml`: unchanged (no refusal added or removed; row 20 asserts the
remedy text only, so `check-refusal-coverage.sh` still reports `42 refusal tests
over 18 ruled refusals` with the sidecar installed on base).

Coverage map (rule -> rows): P0 -> codec unit test; P1 -> 1-5; P2 -> 6-9, 12, 16;
E1 -> 17, 18, 19, 21-29 (21-22: the `--json` sentence, Brian #23 Q1; 23-25: the
inside-the-prefix sentence, Brian #23 Q2, text and `--json`; 26-28: the
unreadable-full-plate sentence (a)-(c), Brian #23 Q3, decode, verify and inspect
text; 29: its (d), `--json` unchanged); E2 -> 20; C1 -> 10, 11, 13, 15; C2 -> 14.
Spec delta 1-3 -> 17-29.

## 8. Build order

1. Install the sidecar; run the row command; see `29 tests run: 6 passed, 23 failed`.
2. P0, then the codec unit test (GREEN).
3. P1: rows 1-5 GREEN.
4. P2 (with the shared step-3 helper): rows 6-9 and 16 GREEN, row 12 stays GREEN.
5. C1: rows 10, 11, 13 GREEN; 14, 15 stay GREEN.
6. E1: rows 17, 18, 21-28 GREEN; 19, 20, 29 stay GREEN. Re-sync the spec copy (section 5
   spec delta) once mnemonic-engrave carries it.
7. Gate (section 10) once to a file.

## 9. Open points for the architect (no STOP)

- **O1.** Row 9 (transliteration) extends I7 from BCH repairs to the step-3
  transliteration of the full line. Same T19 principle, same code path; cut row 9
  and P2's autocorrect half if the architect wants only the triaged case.
- **O2. Settled by Brian 2026-11-22** (section 2): plate positions, now rule E1
  with rows 17-19 and the section 5 spec delta.
- **Q1. Settled by Brian 2026-11-22** (section 2): the `--json` warning prints
  the plate position; now part of rule E1 with rows 21-22 and spec delta edits 2-3.
- **Q2. Settled by Brian 2026-11-22** (section 2): a correction or
  transliteration inside the restored prefix prints "ONCE, on the full plate, at
  its whole-string position there (e.g. `pos 8`). Elided plates print no row for
  it." Now part of rule E1 with rows 23-25 and spec delta edits 2-3. The case is
  reachable (measured in fold r1b at 59f021c, `git archive` copy): it needs the
  only line beginning `mt1` to be unreadable while the set completes from elided
  lines, so that P2 falls back to that line's typed `full[..11]` (rule P2, last
  sentence) and its typo is copied into every elided line.
  - Input A: `even` string 1 with the next charset symbol at 0-based offsets 7,
    20, 25, 30, 35 (prefix typed `mt1p9h8nqq9`), then all six strings elided
    (strings 1-6, each without its first 11 characters). `mt decode`, `mt verify`
    and `mt inspect` exit 0; `UNREADABLE STRING. string 1 of the input could not
    be read`; `CORRECTION APPLIED. 6 chunks needed repair:` and under each chunk
    `              pos   8   read n, corrected to j`. `--json` (`decode` stderr,
    `inspect` stdout) carries no `pos` warning for it; its warnings hold
    `chunk 1 needed 1 of 4 corrections` ... `chunk 6 needed 1 of 4 corrections`
    (`correction_warnings`, no position). Rows 26-29 (Q3).
  - Input B: `even` string 1 with substitutions at offsets 20, 25, 30, 35, 40 and
    its offset 6 `8` typed `b`, then all six strings elided. `mt decode` exits 0;
    `  pos   7   you typed b, mt read it as 8` six times; the `--json` warnings of
    `decode` and `inspect` carry `"pos 7: you typed b, mt read it as 8"` six
    times. Rows 23-25 (once each, after the fix).
  The builder must not compute an elided plate position in `usize` for a data-part
  offset below 7 (`p + 1 + 3 - 11` underflows); inside the prefix no plate
  position is computed (E1).
- **Q3. Settled by Brian 2026-11-22** (section 2), for input A's CORRECTION block
  (fold r1c had found it open: the full plate is the UNREADABLE line, not a chunk,
  and the block is per chunk): "(a) the CORRECTION header keeps the real chunk
  count (`6 chunks needed repair`); (b) one `pos   8   read n, corrected to j` row
  prints ONCE, under a line that names that string (the full plate, string 1 of
  the input); (c) each chunk line still prints its `N of 4 symbols`, with no `pos`
  row for the inherited prefix symbol; (d) the `--json` correction warnings (no
  position) stay unchanged." The naming line (author's text, fold r1d):
  `  string 1 of the input   its prefix was restored onto the elided strings`,
  printed after the chunk lines and before any correction-limit paragraph. Now
  part of rule E1 (unreadable-full-plate sentence) with rows 26-29, section 4's
  input A block and spec delta edits 2-3.
- **O3.** If the architect wants the restore source stated in the spec (§3b: "the
  prefix is taken from string 1 as BCH and transliteration resolve it"), that is an
  engrave spec edit and a re-sync, not part of this FIX.

## 10. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
The CI `test` job (`.github/workflows/ci.yml`) plus the rows:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expect 432 run, 0 failed: 402 base
   + 29 rows + 1 codec unit test; if another mt FIX lands first, base count moves
   by its own additions)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh` (expect `42 refusal tests over 18 ruled
   refusals`, as at 1f6cc6b)
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh` and `bash ci/code-docs-check.sh`
10. Rows by name: `cargo nextest run --locked -p mt-cli --test prefix_corrected`
    -> `29 tests run: 29 passed`. Fewer than 29 is a FAIL.
11. `grep -n 'r.set_prefix = pipeline::invariant_prefix' crates/mt-cli/src/main.rs` -> no output (P1; encode's PREFIX row at the `let prefix =` line keeps `pipeline::invariant_prefix(&strings[0])`, C2).

## 11. FOLLOWUPS (listed in the author's report, not appended here)

Closes `decode-prefix-after-bch-correction` (`design/FOLLOWUPS.md:46`). O2 is
settled by E1 (no line). Files one line for the deferred search (Brian
2026-11-22), listed in the fold r1 report, not appended here.
