# FIX-C: mt bug hunt, reader, JSON and flags (#7, #10, #11, #12, R11; rulings R7, R9-R12)

Small-fix track. Repo: mnemonic-transaction only. Branch `fix/bughunt-c`.
Inputs: `design/agent-reports/grade-bughunt-mt.md` (rulings R7, R9, R10, R11,
R12 bind; §3 row FIX-C), `design/agent-reports/bughunt-triage-mt.md` (#7,
#10, #11, #12 CONFIRMED). No new flag, type or command. New printed text: the
JSON warning strings of R9 and the two §10.10 refusals of R10 and R11, all
quoted verbatim from the rulings.

**Base: FIX-B's merged head on master, not `3bfbc98`.** FIX-B and FIX-C both
touch the reader's error mapping (grade §3 "Order"); B builds and merges first.
This document and its RED rows were written on `3bfbc98`.

**R11 triage (grade O3): CONFIRMED.** Its RED row
`bughunt_mt_r11_quiet_on_inspect_and_verify_is_refused` fails on unmodified
code for the stated reason (`inspect --quiet` exits 0 and prints the report;
`verify --quiet` exits 0 and prints the same 65-byte verdict as without the
flag). R11 is kept.

The five STOP sections (S1-S5) at the end are all ruled by the architect
(`coordinator/reports/architect-bughunt-fixdocs.md`: Q0, QC2-QC5); "C M1" is
graded Minor and deferred (not folded). No STOP is open.

Commit message (GREEN) carries the line:
`Spec copy: mt is authoritative; engrave sync owed by F-321.`

## Build order

1. Wait for FIX-B to merge. Rebase `fix/bughunt-c` onto master
   (`git rebase master`). The rebase conflicts in
   `crates/mt-cli/tests/bughunt.rs`, where both branches append at the end
   of the file: keep both blocks, FIX-B's first (`bughunt_mt_04_cli_count_mismatch_refusal`
   and anything above it) and then this document's section, delete the
   three conflict marker lines, `git add` the file and
   `git rebase --continue`. Any other conflict: STOP and report it.
2. Run the RED command (§3). Expected: 15 matched, 15 failed.
3. Spec delta (§1), in the mt copy (S1, ruled Q0).
4. R7, `read_strings.rs` (§2.1; S2 ruled QC2).
5. R12, `read_strings.rs` (§2.2; S4 ruled QC4).
6. R9, `report.rs` escaping (§2.3; S5 ruled QC5).
7. R9, `main.rs` JSON warnings in decode and inspect (§2.4; S3 ruled QC3).
8. R10 and R11, `main.rs` guards plus four `refusals.toml` entries (§2.5).
9. Gate (§5), captured once to a file.

## 1. Spec delta (`design/SPEC_mt_v0_1.md`, §10.10b)

After the existing paragraph that ends "It does **not** relax §1.1a's rule
that stdout stays empty unless every check passes.", and before
"**THE FLAG SPELLINGS — RULED 2026-08-23.**", insert this paragraph, indented
like the `b.` item it extends (7 spaces):

```
       **Under `--json`, every notice is one single-line string in the
       `warnings` array, the same on `decode` and `inspect`.** A duplicate
       reads `chunk {n} was present twice; kept the copy needing {k}
       corrections`. Each repaired chunk reads `chunk {n} needed {c} of 4
       corrections`, with `; NO MARGIN LEFT` appended when {c} ≥ 4, in the
       order of the prose margin report (most corrections first), after the
       duplicate entries and, in `decode`, before the stdout-mode headline;
       the JSON omits the prose's before-and-after detail. Control characters
       U+0000-U+001F inside any JSON string are escaped (RFC 8259 §7).
       **`--json` with `--quiet` is refused** (§10.10, exit 1): one asks for
       the report as a document and the other for no report. **`--quiet` on
       `inspect` or `verify` is refused** (§10.10, exit 1), with or without
       `--json`: their output is their report.
```

The flag-table row for `--quiet` is unchanged ("suppresses the inspection
report only; never warnings or refusals").

## 2. Code changes (production)

### 2.1 R7, `crates/mt-cli/src/read_strings.rs`, `restore_elided`

The panic is `let prefix = full[..ELIDED_DROP].to_string();` (line 599 on
`3bfbc98`): byte 11 of the full string is not a char boundary when a non-ASCII
character sits at char offset 10. Replace the slice with
`full.get(..ELIDED_DROP)`, and choose `full` differently (architect QC2, S2
ruled "(c) then (b)"): `full` is the first candidate that starts `mt1` and for
which `is_char_boundary(ELIDED_DROP)` holds, falling back to the first `mt1`
candidate. When `full.get(..ELIDED_DROP)` is `None`, return
`Refusal::new(verb, "§1.1", format!("string {n} could not be read: more than 4
characters differ from what was engraved"), <mechanism>)` with
`.with_remedy(<remedy>)`. Here `n` is the 1-based position of `full` in
`candidates`. The mechanism and remedy are the first paragraph and the remedy
of the existing §1.1 refusal in `main.rs`, verbatim: "BCH repairs up to 4
wrong characters per string. Past that it cannot tell which are wrong, so it
refuses rather than guessing — a wrong guess produces a valid-looking string
carrying the wrong bytes." / "Re-read that plate from the steel, character by
character. Confusable pairs to check first: 0/o, 1/l/i, b/6, 2/z, 5/s, 8/b."
The `full.len() < ELIDED_DROP` refusal above it stays as it is.

Results (architect QC2, probed on 15b5263): on row 07c's input (non-ASCII at
char 10 of the only full string, the other five elided) all three verbs give
rc 1, empty stdout, and

```
mt {verb}: REFUSED — §1.1, string 1 could not be read: more than 4 characters differ from what was engraved
```

Row 07b (no elided line) gives the same shape with `string 1`. With a second
intact full line (row 07d), decode takes the main path and gives the same first
line plus its richer mechanism ("This set has 6 chunks. 5 read cleanly ...").

### 2.2 R12, `crates/mt-cli/src/read_strings.rs`, step 2b of `read`

In the `mtl`/`mti` loop (lines 109-117 on `3bfbc98`), keep the repair when the
repaired string decodes directly OR any of `positional_autocorrect(&fixed)`
decodes:

```
if pipeline::decode_chunk(&fixed, None).is_ok()
    || positional_autocorrect(&fixed)
        .iter()
        .any(|x| pipeline::decode_chunk(x, None).is_ok())
```

This is `/scratch/cache/bughunt/mt/fix-12.patch`. Step 4's positional
autocorrect then repairs the confusable and records its existing
`Transliteration` note. Verified as a scratch patch: exit 0, stdout equals the
clean decode, and stderr carries `pos  38   you typed b, mt read it as 6`. The
"and something else wrong too" refusal still fires when neither path decodes.
The autocorrect notice prints; the free `mtl`/`mti` repair stays silent, as
today (architect QC4; R12 amended: "Both notices print: the existing prefix
notice and the existing autocorrect notice" is struck, because no prefix notice
exists).

### 2.3 R9, `crates/mt-cli/src/report.rs`, `render_json` escaping

Replace the `esc` closure (line 432 on `3bfbc98`), which escapes only `\` and
`"`, with one that maps each char:

| char | output |
| --- | --- |
| `\` | `\\` |
| `"` | `\"` |
| U+000A | `\n` |
| U+000D | `\r` |
| U+0009 | `\t` |
| U+0008 | `\b` |
| U+000C | `\f` |
| any other U+0000-U+001F | `\u00XX`, two lowercase hex digits (`format!("\\u{:04x}", c as u32)`; architect QC5) |
| anything else | itself |

### 2.4 R9, `crates/mt-cli/src/main.rs`, JSON warnings

Add one function next to `margin_report`:

```
fn correction_warnings(chunks: &[mt_codec::DecodedChunk]) -> Vec<String>
```

It keeps the chunks with `corrected > 0`, sorts them with the same stable
`sort_by(|a, b| b.corrected.cmp(&a.corrected))` that `margin_report` uses (so
ties keep `set.chunks` order), and maps each to
`format!("chunk {} needed {} of 4 corrections{}", c.header.index + 1,
c.corrected, if c.corrected >= 4 { "; NO MARGIN LEFT" } else { "" })`.

- `decode` (the `if args.json` block, lines 1274-1309 on `3bfbc98`): extend
  `warnings` with `correction_warnings(&set.chunks)`, after the duplicate
  entries and before the stdout-mode headline (architect QC3).
- `inspect` (the `warnings` vector, lines 1640-1658 on `3bfbc98`): add the
  duplicate loop, copied from decode verbatim (`chunk {} was present twice;
  kept the copy needing {} corrections`, `d.index + 1`, `d.kept_corrections`),
  then extend with `correction_warnings(&set.chunks)`, after the duplicate
  entries (architect QC3).
- Exact order (QC3): decode `[no-bitcoind?, pos..., unreadable..., duplicates...,
  corrections..., stdout-mode headline?]`; inspect `[no-bitcoind?, pos...,
  unreadable..., duplicates..., corrections...]`. No existing entry moves.
- `inspect_transaction` (raw-transaction subject) has no chunks and does not
  change.

### 2.5 R10 and R11, `crates/mt-cli/src/main.rs`, guards

Two functions next to `json_unsupported_guard`, each
`-> Result<(), Refusal>`, built like it:

`fn json_quiet_guard(json: bool, quiet: bool, verb: &str)`: `Ok(())` unless
both are set; otherwise

- `Refusal::new(verb, "§10.10", "--json and --quiet contradict each other",
  <mechanism>)` with mechanism
  ``--json` asks for the inspection report as a document and `--quiet` asks for no report; mt refuses rather than silently ignoring one.``
- `.with_remedy(...)`:
  ``Drop `--quiet` for the JSON report on stderr, or drop `--json` for the hex alone.``

`fn quiet_unsupported_guard(quiet: bool, verb: &str)`: `Ok(())` unless
`quiet`; otherwise

- `Refusal::new(verb, "§10.10", format!("--quiet has no meaning for `{verb}`"),
  format!("{verb}'s output is its report; mt refuses rather than print it anyway."))`
- `.with_remedy("Drop `--quiet`.")`

Calls, each the first statement of its verb:

- `decode`: `json_quiet_guard(args.json, args.quiet, "decode")?;` before
  `read_input`.
- `verify`: `quiet_unsupported_guard(args.quiet, "verify")?;` before the
  existing `json_unsupported_guard(args.json, "verify")?;`.
- `inspect`: `quiet_unsupported_guard(args.quiet, "inspect")?;` before
  `read_input`.

`json_quiet_guard` is called from `decode` only: on `inspect` and `verify` R11
fires first (R10), so a call there would be unreachable.

Printed result (first line; mechanism and remedy follow, wrapped by
`Refusal`'s Display), stderr, exit 1, empty stdout:

```
mt decode: REFUSED — §10.10, --json and --quiet contradict each other
mt inspect: REFUSED — §10.10, --quiet has no meaning for `inspect`
mt verify: REFUSED — §10.10, --quiet has no meaning for `verify`
```

`mt inspect --json --quiet` and `mt verify --json --quiet` print the R11 line
for their verb.

`crates/mt-cli/tests/refusals.toml`: append four entries (the first is
architect QC2's, for the new `restore_elided` refusal):

```
[[refusal]]
spec  = "§1.1"
test  = "bughunt_mt_07c_non_ascii_in_the_prefix_line_with_elided_lines_refuses"
check = "src/read_strings.rs::restore_elided"

[[refusal]]
spec  = "§10.10"
test  = "bughunt_mt_11b_decode_json_quiet_is_refused"
check = "src/main.rs::json_quiet_guard"

[[refusal]]
spec  = "§10.10"
test  = "bughunt_mt_r11_quiet_on_inspect_and_verify_is_refused"
check = "src/main.rs::quiet_unsupported_guard"

[[refusal]]
spec  = "§10.10"
test  = "bughunt_mt_r11_json_quiet_on_inspect_and_verify_gets_the_quiet_refusal"
check = "src/main.rs::quiet_unsupported_guard"
```

`scripts/check-refusal-coverage.sh`: in the `EXEMPT` dict, after its
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
`explain_failure`, and `restore_elided` returns `Ok` on its input.

The `ReadArgs.quiet` doc comment stays (R11).

## 3. RED rows

All rows are committed: `crates/mt-cli/tests/bughunt.rs` (CLI rows, FIX-C
section, with helpers `flat`, `exact_warnings`, `damage`) and
`crates/mt-cli/src/report.rs` (`#[cfg(test)] mod tests`, row 10e). The CLI
rows run `mt <verb> --bitcoin-cli /nonexistent/bitcoin-cli --in <0600 file>`
on the corpus `even` vector (6 strings).

RED command (also the GREEN command):

```
cargo nextest run --locked --no-fail-fast -E 'test(/^(report::tests::)?bughunt_mt_(07|07b|07c|07d|10|10b|10c|10d|10e|11|11b|r11_.*|12|12b)_/)'
```

It matches 15 tests (14 plus the new 07d). Observed on unmodified `3bfbc98`
(after the architect fold): 15 run, 15 failed, each for the stated reason.

| row | input and expectation after GREEN | failure on unmodified code |
| --- | --- | --- |
| `bughunt_mt_07_non_ascii_near_line_start_does_not_panic` (reused) | non-ASCII near the line start; no panic, rc ≠ 101 | panic, rc 101 |
| `bughunt_mt_07b_non_ascii_cut_point_is_an_unreadable_string` | `MT1aaaaaaaéqqqqqqqqqq` on decode, inspect, verify: rc 1, empty stdout, no `panicked`, first stderr line starts `mt {verb}: REFUSED` and contains `string 1 could not be read` | decode rc 101, panic at `read_strings.rs:599:22` |
| `bughunt_mt_07c_non_ascii_in_the_prefix_line_with_elided_lines_refuses` | `even` string 1 with char 10 set to `é`, strings 2-6 elided (`[11..]`), on all three verbs: no panic, rc 1, empty stdout, first line equals ``mt {verb}: REFUSED — §1.1, string 1 could not be read: more than 4 characters differ from what was engraved`` (QC2) | decode panicked at `read_strings.rs:599:22` |
| `bughunt_mt_07d_second_full_line_supplies_the_prefix` (new, QC2) | the 07c input with string 2 kept full, on decode: rc 1, empty stdout, first line equal to 07c's; stderr (whitespace-flattened) contains `read cleanly and 1 did not` (the main path's mechanism) | decode panicked at `read_strings.rs:599:22` |
| `bughunt_mt_10_inspect_json_carries_duplicate_and_correction_notices` (reused) | inspect/decode `--json` carry a duplicate and a correction notice | no such entry |
| `bughunt_mt_10b_inspect_json_duplicate_text_is_exact` | `even` plus a second copy of string 1, `inspect --json`: `warnings` equals exactly `["no bitcoind reachable: nothing here is confirmed against the chain", "chunk 1 was present twice; kept the copy needing 0 corrections"]` (QC3) | array holds only the no-bitcoind entry |
| `bughunt_mt_10c_json_correction_text_is_exact` | `even` with string 1 damaged at char 10: the `needed`-entries equal [`chunk 1 needed 1 of 4 corrections`] on `inspect --json` (stdout) and `decode --json` (stderr) | entries `[]` |
| `bughunt_mt_10d_json_correction_entries_ordered_and_no_margin_left` | string 1 damaged at [10], string 2 at [10, 20, 30, 40]. `inspect --json`: `warnings` equals exactly `["no bitcoind reachable: nothing here is confirmed against the chain", "chunk 2 needed 4 of 4 corrections; NO MARGIN LEFT", "chunk 1 needed 1 of 4 corrections"]`. `decode --json` (this input, and again with a duplicate of clean string 3 appended): the `needed` entries equal [chunk 2 ..., chunk 1 ...], every `needed` entry follows every `was present twice` entry (the duplicate is required to appear in the second input), and at most the stdout-mode headline follows the last `needed` entry (QC3) | inspect array holds only the no-bitcoind entry |
| `report::tests::bughunt_mt_10e_render_json_escapes_control_characters` | `render_json` with warning `a\nb\rc\td\u{8}e\u{c}f\u{0}g\u{1}h\u{1f}i"j\k`: output contains the `\n \r \t \b \f` escapes and `\u0000`, `\u0001`, `\u001f` (lowercase, QC5), parses with serde_json, `warnings[0]` round-trips | `\n not escaped` |
| `bughunt_mt_11_decode_json_quiet_does_not_silently_drop_json` (reused) | `decode --json --quiet`: not rc 0 with the JSON dropped | rc 0, JSON dropped |
| `bughunt_mt_11b_decode_json_quiet_is_refused` | rc 1, empty stdout, first line `mt decode: REFUSED — §10.10, --json and --quiet contradict each other`; flattened stderr contains the R10 mechanism and remedy (§2.5) | rc 0 |
| `bughunt_mt_r11_quiet_on_inspect_and_verify_is_refused` | `--quiet` on inspect and verify: rc 1, empty stdout, first line ``mt {verb}: REFUSED — §10.10, --quiet has no meaning for `{verb}` ``; flattened stderr contains the R11 mechanism and remedy | inspect rc 0 (R11 triage) |
| `bughunt_mt_r11_json_quiet_on_inspect_and_verify_gets_the_quiet_refusal` | `--json --quiet` on inspect and verify: rc 1, empty stdout, the R11 first line | inspect rc 0 (verify gives the `--json has no meaning` refusal) |
| `bughunt_mt_12_mtl_prefix_plus_confusable_still_decodes` (reused) | `mtl` prefix plus one `6`→`b`: rc 0, same stdout as clean | refused |
| `bughunt_mt_12b_mtl_repair_keeps_the_autocorrect_notice` | same input: rc 0, same stdout, stderr contains `you typed b, mt read it as 6` and neither `you typed l` nor `you typed i` (QC4) | rc 1, `mt decode: REFUSED — §1.1e, a string begins `mtl` — the `1` of `mt1` misread, and something else wrong too` |

Rows 10b and 10d (inspect) assert the whole `warnings` array; decode rows
assert relative order (QC3). Row 10e pins lowercase `\u00XX` (QC5).

## 4. Tests that change

- `crates/mt-cli/tests/bughunt.rs`: 10 new rows (incl. 07d) and 3 helpers. Committed.
- `crates/mt-cli/src/report.rs`: 1 new row in the test module. Committed.
- `crates/mt-cli/tests/refusals.toml`: 4 entries (§2.5), in the GREEN commit.
  Until then `./scripts/check-refusal-coverage.sh` fails on the RED commit,
  naming the three R10/R11 tests (observed before the fold; the 07c entry
  is added by the fold and not yet run).
- `scripts/check-refusal-coverage.sh`: one `EXEMPT` entry for row 07d
  (§2.5), in the GREEN commit.
- No existing test expectation changes: no current test passes `--quiet` to
  `inspect` or `verify` (the `--quiet` uses in `encode.rs`,
  `decode_verify.rs` and `declined_crate_items.rs` are `encode` and
  `decode` without `--json`), and no test pins the JSON `warnings` length.
- README: no change (it does not mention `--quiet` or `--json`). `--help`:
  no change (R11 keeps the `ReadArgs.quiet` doc). O1's lockstep target is
  therefore the spec only (§1).

## 5. Gate (run once to a file, then grep)

The repo has no phase-close list; its gate is CI (`.github/workflows/ci.yml`)
plus the doc tests. With `$HOME/.cargo/bin` first in PATH and
`cargo --version` = 1.85.0 (`rust-toolchain.toml`):

1. `bash ci/docs-only.sh`
2. `cargo fmt --check`
3. `cargo clippy --all-targets --locked -- -D warnings`
4. `cargo build --locked`
5. `cargo nextest run --locked --no-fail-fast` (expected failures: only the
   bughunt rows of FIXes not yet merged, A and D; none of FIX-B's or FIX-C's)
6. `cargo test --locked --workspace --doc`
7. `./scripts/check-refusal-coverage.sh`
8. `./scripts/mutate-refusals.sh` (now includes the 4 new entries; each must
   report its test failing when its guard is neutered)
9. `./scripts/journeys.sh`
10. `bash ci/code-docs-check.sh`, `bash ci/docs-only.test.sh`,
    `bash ci/code-docs-check.test.sh`
11. `./scripts/check-provenance.sh` (Q0: an mt-only spec edit makes it
    report a mismatch for `design/SPEC_mt_v0_1.md`; expected, sync owed by F-321)
12. The §3 command: 15 matched, 15 passed. Quote the count in the GREEN commit
    message.

Then `coordinator/scripts/mutation-gate.sh <worktree> <FIX-B merge head>`
before review (production Rust changed). Review: class A (printed text).

## STOPs: all ruled

Source: `coordinator/reports/architect-bughunt-fixdocs.md`.

- **S1 (which spec copy): RULED Q0.** Edit the mt copy `design/SPEC_mt_v0_1.md`
  (§10.10b here); engrave does not get the edit now. The sync is owed by
  engrave FOLLOWUPS F-321 (the coordinator appends "bughunt FIX-A..C add
  mt-spec text at §8.6, §1.1, §10.10; carry them in the sync."). No PROVENANCE
  edit. The commit message carries `Spec copy: mt is authoritative; engrave
  sync owed by F-321.`
- **S2 (R7 with elided lines): RULED QC2**, option (c) then (b); text in §2.1,
  rows 07c and 07d, and a `refusals.toml` entry (below).
- **S3 (order inside `warnings`): RULED QC3**, the recommendation; §2.4 and
  rows 10b, 10d.
- **S4 (R12's "existing prefix notice"): RULED QC4**, option (a); §2.2, row 12b.
- **S5 (`\u00XX` case): RULED QC5**, lowercase; §2.3, row 10e.
- **C M1 (R11 wording on verify): graded Minor, not folded.** R11's text
  stands; FOLLOWUPS `deferred-minor` (the coordinator files it).
