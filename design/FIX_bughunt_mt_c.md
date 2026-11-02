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

Five STOP sections (S1-S5) are open at the end. Which steps each one blocks is
stated in the build order.

## Build order

1. Wait for FIX-B to merge. Rebase `fix/bughunt-c` onto master
   (`git rebase master`). The RED commit adds only test code and this
   document, so the rebase is expected to be clean; if it is not, STOP and
   report the conflict.
2. Run the RED command (§3). Expected: 14 matched, 14 failed.
3. Spec delta (§1). Blocked by S1 (where the edit goes) and by S3 and S5 if
   their rulings change a quoted text.
4. R7, `read_strings.rs` (§2.1). Blocked by S2 for the elided case only.
5. R12, `read_strings.rs` (§2.2). Blocked by S4.
6. R9, `report.rs` escaping (§2.3). Blocked by S5.
7. R9, `main.rs` JSON warnings in decode and inspect (§2.4). Blocked by S3.
8. R10 and R11, `main.rs` guards plus three `refusals.toml` entries (§2.5).
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
       order of the prose margin report (most corrections first); the JSON
       omits the prose's before-and-after detail. Control characters
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
`full.get(..ELIDED_DROP)`. When it returns `None`, return
`Ok(candidates)` unchanged, so the string reaches the codec and is refused as
unreadable by the existing path. The `full.len() < ELIDED_DROP` refusal above
it stays as it is.

Result when the set has no elided line (row 07b), verified with this change as
a scratch patch on `3bfbc98`:

```
mt decode: REFUSED — §1.1, string 1 could not be read: more than 4 characters differ from what was engraved
```

exit 1, empty stdout. That is the ruling's existing `string {n} could not be
read` refusal. When elided lines are present (row 07c), see STOP S2.

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
Whether the `mtl` repair itself prints a notice: STOP S4.

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
| any other U+0000-U+001F | `\u00XX`, two hex digits (case: STOP S5) |
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
  `warnings` with `correction_warnings(&set.chunks)`. Position: STOP S3.
- `inspect` (the `warnings` vector, lines 1640-1658 on `3bfbc98`): add the
  duplicate loop, copied from decode verbatim (`chunk {} was present twice;
  kept the copy needing {} corrections`, `d.index + 1`, `d.kept_corrections`),
  then extend with `correction_warnings(&set.chunks)`. Position: STOP S3.
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

`crates/mt-cli/tests/refusals.toml`: append three entries:

```
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

The `ReadArgs.quiet` doc comment stays (R11).

## 3. RED rows

All rows are committed: `crates/mt-cli/tests/bughunt.rs` (CLI rows, FIX-C
section, with helpers `flat`, `exact_warnings`, `damage`) and
`crates/mt-cli/src/report.rs` (`#[cfg(test)] mod tests`, row 10e). The CLI
rows run `mt <verb> --bitcoin-cli /nonexistent/bitcoin-cli --in <0600 file>`
on the corpus `even` vector (6 strings).

RED command (also the GREEN command):

```
cargo nextest run --locked --no-fail-fast -E 'test(/^(report::tests::)?bughunt_mt_(07|07b|07c|10|10b|10c|10d|10e|11|11b|r11_.*|12|12b)_/)'
```

It matches 14 tests. Observed on unmodified `3bfbc98`: 14 run, 14 failed.

| row | input and expectation after GREEN | failure on unmodified code |
| --- | --- | --- |
| `bughunt_mt_07_non_ascii_near_line_start_does_not_panic` (reused) | non-ASCII near the line start; no panic, rc ≠ 101 | panic, rc 101 |
| `bughunt_mt_07b_non_ascii_cut_point_is_an_unreadable_string` | `MT1aaaaaaaéqqqqqqqqqq` on decode, inspect, verify: rc 1, empty stdout, no `panicked`, first stderr line starts `mt {verb}: REFUSED` and contains `string 1 could not be read` | decode rc 101, panic at `read_strings.rs:599:22` |
| `bughunt_mt_07c_non_ascii_in_the_prefix_line_with_elided_lines_refuses` | `even` string 1 with char 10 set to `é`, strings 2-6 elided (`[11..]`), on all three verbs: no panic, rc 1, empty stdout, first line starts `mt {verb}: REFUSED` (exact text: STOP S2) | decode panicked at `read_strings.rs:599:22` |
| `bughunt_mt_10_inspect_json_carries_duplicate_and_correction_notices` (reused) | inspect/decode `--json` carry a duplicate and a correction notice | no such entry |
| `bughunt_mt_10b_inspect_json_duplicate_text_is_exact` | `even` plus a second copy of string 1, `inspect --json`: `warnings` holds exactly one `chunk 1 was present twice; kept the copy needing 0 corrections` | count 0 (warnings hold only the no-bitcoind entry) |
| `bughunt_mt_10c_json_correction_text_is_exact` | `even` with string 1 damaged at char 10: the `needed`-entries equal [`chunk 1 needed 1 of 4 corrections`] on `inspect --json` (stdout) and `decode --json` (stderr) | entries `[]` |
| `bughunt_mt_10d_json_correction_entries_ordered_and_no_margin_left` | string 1 damaged at [10], string 2 at [10, 20, 30, 40]: entries equal [`chunk 2 needed 4 of 4 corrections; NO MARGIN LEFT`, `chunk 1 needed 1 of 4 corrections`] on both verbs | entries `[]` |
| `report::tests::bughunt_mt_10e_render_json_escapes_control_characters` | `render_json` with warning `a\nb\rc\td\u{8}e\u{c}f\u{0}g\u{1}h\u{1f}i"j\k`: output contains the `\n \r \t \b \f` escapes, parses with serde_json, `warnings[0]` round-trips | `\n not escaped` |
| `bughunt_mt_11_decode_json_quiet_does_not_silently_drop_json` (reused) | `decode --json --quiet`: not rc 0 with the JSON dropped | rc 0, JSON dropped |
| `bughunt_mt_11b_decode_json_quiet_is_refused` | rc 1, empty stdout, first line `mt decode: REFUSED — §10.10, --json and --quiet contradict each other`; flattened stderr contains the R10 mechanism and remedy (§2.5) | rc 0 |
| `bughunt_mt_r11_quiet_on_inspect_and_verify_is_refused` | `--quiet` on inspect and verify: rc 1, empty stdout, first line ``mt {verb}: REFUSED — §10.10, --quiet has no meaning for `{verb}` ``; flattened stderr contains the R11 mechanism and remedy | inspect rc 0 (R11 triage) |
| `bughunt_mt_r11_json_quiet_on_inspect_and_verify_gets_the_quiet_refusal` | `--json --quiet` on inspect and verify: rc 1, empty stdout, the R11 first line | inspect rc 0 (verify gives the `--json has no meaning` refusal) |
| `bughunt_mt_12_mtl_prefix_plus_confusable_still_decodes` (reused) | `mtl` prefix plus one `6`→`b`: rc 0, same stdout as clean | refused |
| `bughunt_mt_12b_mtl_repair_keeps_the_autocorrect_notice` | same input: rc 0, same stdout, stderr contains `you typed b, mt read it as 6` | rc 1, `mt decode: REFUSED — §1.1e, a string begins `mtl` — the `1` of `mt1` misread, and something else wrong too` |

Rows 10b-10d assert membership and relative order, not the position inside
`warnings` (S3). Row 10e parses rather than pinning the `\u00XX` case (S5).
Row 07c pins only `REFUSED`, not the text (S2). Row 12b does not assert an
`mtl` notice (S4). If a ruling decides one of these, add its assertion to that
row in the GREEN commit.

## 4. Tests that change

- `crates/mt-cli/tests/bughunt.rs`: 9 new rows and 3 helpers. Committed.
- `crates/mt-cli/src/report.rs`: 1 new row in the test module. Committed.
- `crates/mt-cli/tests/refusals.toml`: 3 entries (§2.5), in the GREEN commit.
  Until then `./scripts/check-refusal-coverage.sh` fails on the RED commit,
  naming exactly those three tests (observed).
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
8. `./scripts/mutate-refusals.sh` (now includes the 3 new entries; each must
   report its test failing when its guard is neutered)
9. `./scripts/journeys.sh`
10. `bash ci/code-docs-check.sh`, `bash ci/docs-only.test.sh`,
    `bash ci/code-docs-check.test.sh`
11. `./scripts/check-provenance.sh` (see S1: an mt-only spec edit makes it
    report a mismatch for `design/SPEC_mt_v0_1.md`)
12. The §3 command: 14 matched, 14 passed. Quote the count in the GREEN commit
    message.

Then `coordinator/scripts/mutation-gate.sh <worktree> <FIX-B merge head>`
before review (production Rust changed). Review: class A (printed text).

## STOP S1: which copy of the spec the delta edits

`design/PROVENANCE.md` says the spec is COPIED from mnemonic-engrave and "not
edited in this repo", and `scripts/check-provenance.sh` compares the two
locally. The grade (§3) says every FIX is "mt repo only" and puts the FIX-C
delta in §10.10b. Same question as FIX-B's S2; one ruling should cover both.
Options: (a) edit the mt copy and file a follow-up to carry it to engrave;
(b) edit engrave, re-sync, update the PROVENANCE pin. Blocks step 3.

## STOP S2: R7's refusal when elided lines are present

R7 rules the existing `string {n} could not be read` refusal. With §2.1
applied as a scratch patch, row 07c's input (non-ASCII at char 10 of the only
full string, the other five elided) gives instead:

```
mt decode: REFUSED — §1.1e, 1 string is the wrong length for this set (most are 76)
```

because no prefix is restored and the elided lines stay short. It refuses
(exit 1, no panic), but not with the ruled text. Options: (a) accept that
§1.1e refusal for this case; (b) return the §1.1 `string {n} could not be
read` refusal from `restore_elided` itself when the cut point is not a char
boundary (it needs the reason text decided); (c) restore from another full
`mt1` line if one exists, else (a) or (b). Blocks the elided case of step 4
and the exact assertion in row 07c.

## STOP S3: where the new entries sit in `warnings`

R9 orders the correction entries among themselves but not against the other
entries. decode today pushes: no-bitcoind, transliterations, unreadable,
duplicates, stdout-mode headline. Recommended: duplicates, then corrections,
both before the stdout-mode headline, in both verbs (inspect has no headline).
Blocks step 7.

## STOP S4: R12's "existing prefix notice" does not exist

R12 says "Both notices print: the existing prefix notice and the existing
autocorrect notice". The step-2b `mtl`/`mti` repair is silent today; there is
no prefix notice to print (no `Transliteration` is recorded for it, and no
other text names it). Options: (a) keep the prefix repair silent, so only the
autocorrect notice prints (what fix-12 does); (b) record
`Transliteration { position: 3, from: 'l', to: '1' }` (or `'i'`) for it, so it
prints in the existing format `pos 3: you typed l, mt read it as 1`. (b)
changes printed text on the plain `mtl` path too, which the ruling says
should have no new text. Blocks step 5.

## STOP S5: hex case of the `\u00XX` escape

RFC 8259 allows either case. serde_json writes lowercase (`\u001f`).
Recommended: lowercase. Blocks step 6 only for the one-line mapping; no row
depends on it.
