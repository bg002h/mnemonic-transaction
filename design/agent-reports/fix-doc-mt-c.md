**⚠ R11 CONFIRMED (Important per grade): `--quiet` on `inspect`/`verify` is accepted and ignored on unmodified code. 5 STOPs open.**

# FIX-C document report: mt bug hunt (#7, #10, #11, #12, R11)

FIX doc: `design/FIX_bughunt_mt_c.md`, commit e836360 on `fix/bughunt-c`
(from 3bfbc98, not pushed). Build base: FIX-B's merged head.

## RED rows, run on unmodified 3bfbc98

Filter `test(/^(report::tests::)?bughunt_mt_(07|07b|07c|10|10b|10c|10d|10e|11|11b|r11_.*|12|12b)_/)`
matches 14 (verified with `cargo nextest list`); 14 run, 14 failed. Whole
bughunt filter: 22 run, 21 failed, #9 passes (FALSE POSITIVE, as settled).

| new row | failure on unmodified code (the stated reason) |
| --- | --- |
| 07b | decode rc 101, panic at read_strings.rs:599:22 |
| 07c | panic at read_strings.rs:599:22 |
| 10b | inspect `--json` duplicate entry count 0 |
| 10c | correction entries `[]` |
| 10d | correction entries `[]` |
| 10e | `render_json` leaves `\n` unescaped |
| 11b | `decode --json --quiet` rc 0 |
| r11_quiet | inspect `--quiet` rc 0; verify `--quiet` rc 0, same 65-byte output |
| r11_json | inspect `--json --quiet` rc 0; verify gives the `--json` refusal |
| 12b | rc 1, §1.1e "and something else wrong too" refusal |

R11 triage: CONFIRMED (row fails for the stated reason). R11 kept.

Planned GREEN for R7 (`full.get(..ELIDED_DROP)` None -> return candidates)
and R12 (fix-12 condition) were run as scratch patches only, then reverted;
outputs are quoted in the FIX doc.

`check-refusal-coverage.sh` on the RED commit fails naming exactly the three
new refusal tests; the FIX doc adds their `refusals.toml` entries in GREEN.
Clippy and fmt clean on the RED commit.

## STOPs (in the FIX doc)

- S1: which spec copy takes the §10.10b delta (PROVENANCE says copied from
  engrave, not edited here; grade says mt repo only). Same as FIX-B's S2.
- S2: R7 with elided lines present gives a §1.1e wrong-length refusal, not
  the ruled `string {n} could not be read`.
- S3: position of the new duplicate/correction entries inside `warnings`.
- S4: R12's "existing prefix notice" does not exist; the `mtl` repair is
  silent today.
- S5: hex case of `\u00XX` (recommend lowercase).

## Minor

- M1: R11's mechanism "verify's output is its report" sits beside
  `json_unsupported_guard`'s wording that verify answers with a verdict;
  slightly inconsistent texts on one verb. Text is ruled verbatim, so not
  changed.

Counts: 0 Critical, 1 Important (R11 confirmed, graded by the grader), 1 Minor, 0 Nit; 5 STOPs.
