# FIX: transaction validity guards (bughunt-1119 FIX table row 20, "FIX-E tx validity guards")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Keys: bh-mt-I16, bh-mt-I17
(both Important). Rulings: architect T16 and T17
(`coordinator/agent-reports/architect-bughunt-1119-questions.md:54-55`), binding.
Track: full. **Review class B (funds path):** the grader files FIX-E as "full track:
funds path" (`grade-mt.md:93`); the dispatch says class B. Base: origin/main
`4d1461c`. Branch `fix/mt-tx-validity-guards`. Every new holder-visible text is
marked **PROPOSED (architect confirms)**; everything else below is decided.

Not to be confused with any older `design/FIX_bughunt_mt_*.md` (a different hunt).

## 1. Findings (at 4d1461c)

- **I16 (a), (b).** `mt encode` backs up a transaction Core can never accept:
  with every output removed it exits 0 and prints strings; with an output of
  2100000000000001 sat (one sat over 21,000,000 BTC) it exits 0. Core's
  `CheckTransaction` rejects both (`bad-txns-vout-empty`,
  `bad-txns-vout-toolarge`; `src/consensus/tx_check.cpp:16-30` at bitcoin
  `64a7c7cbb9`). `validate.rs:123` `value_guard` checks `vin` non-empty and
  duplicate outpoints but nothing about outputs.
- **I16 (c).** Every output sum is an unchecked `.sum()`: `validate.rs:151`,
  `validate.rs:220`, `main.rs:626`, `report.rs:268`. Two outputs of 2^63 sat make
  `encode`, `inspect` and `decode` exit 101 `attempt to add with overflow`
  (`core/src/iter/traits/accum.rs:149`) in the debug build. The release profile
  sets no `overflow-checks`, so a shipped binary wraps the sum instead (reasoned
  from the profile; not run, `--release` is not used here).
- **I17.** `non_witness_utxo_guard` (`validate.rs:251-273`) hashes the record and
  compares the txid, but never checks that the record has an output at the
  input's `vout`. `psbt_input_value` (`validate.rs:287-303`) then falls through to
  `witness_utxo` or to `--input-value`, and `encode` exits 0 with a fee computed
  from a value for an output that the record proves does not exist (triage
  `triage-mt-t4.md:15`: vout 99 of a 2-output parent, `--input-value 0:10`,
  "mt therefore shows a fee of 0.00100000 BTC"). With neither a `witness_utxo`
  nor `--input-value` it refuses with §8.2c's "input 0 carries no UTXO record and
  no supplied value", which is false: the input carries one.
- **The test that guarded the I17 label was vacuous.**
  `refusals.rs:1100` `a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound`
  asserts only `if ok { ... }`; at 4d1461c its input is refused by §8.2b's absurd
  fee (`fee rate 19209602.4520 sat/vB exceeds 25,000`), so it has never checked
  anything.

## 2. Rulings (binding, quoted)

- **T16 mt-Q8 (I17): refuse always** when the full parent is present and lacks the
  vout; the parent proves the outpoint does not exist, so no assertion can make it
  valid.
- **T17 mt-Q9 (I16): inspect warns, encode refuses.** inspect is report-only by
  design (grade-mt:73); the overflow panic becomes a checked sum in both.

## 3. Behaviour after the fix (rules)

`MAX_MONEY` below is 2100000000000000 sat (21,000,000 BTC; Core
`src/consensus/amount.h:26`). Values are compared as unsigned 64-bit satoshis, so
an output Core reads as negative (2^63 sat and up) is "above MAX_MONEY" here.

- **E1, the output-range check (one definition, used by E2 and E4).** Checked in
  this order, stopping at the first hit (Core `CheckTransaction` order):
  1. the transaction has no outputs -> verdict `the transaction has 0 outputs`;
  2. for each output `n` in index order: if its value `v` > MAX_MONEY -> verdict
     `output {n} pays {v} sat, more than 21,000,000 BTC`; otherwise add `v` to a
     running total `t`, and if `t` > MAX_MONEY -> verdict
     `outputs 0 to {n} total {t} sat, more than 21,000,000 BTC`.
  `{n}`, `{v}`, `{t}` are plain decimal integers with no separators. Exactly
  MAX_MONEY (one output, or a total) passes. The running total cannot overflow
  (each term and the prior total are at most MAX_MONEY).
- **E2, encode refuses.** `value_guard` (`validate.rs:123`) runs E1 immediately
  after its `vin` non-empty check and **before** the duplicate-outpoint check (Core
  order: vin, vout, output values, duplicate inputs). On a hit it returns a §8.2b
  refusal (text R1, §4). It runs on every input form (PSBT, raw hex, binary),
  with or without a node, and with or without input values, because it sits
  before `value_guard`'s "values unknown -> skip" return. Exit code 1 (the
  existing refusal exit), nothing on stdout. Every check that precedes
  `value_guard` in `encode` today still precedes it (unchanged order).
- **E3, checked output sums.** Every output-value sum in `mt-cli` is a checked
  sum (u64 `checked_add`, `None` on overflow); no `.sum()` over output values
  remains (`validate.rs:151`, `validate.rs:220`, `main.rs:626`, `report.rs:268`).
  No `unwrap`/`expect` on it: where E2 has already passed the total cannot be
  `None`, and the builder picks any non-panicking form there. In
  `Report::build` (`report.rs`), a `None` total makes FEE `None`, so the FEE row
  prints its existing `FEE       UNKNOWN   (needs input values, which the
  transaction` / `does not carry)` text; no new FEE text. `low_fee_warning`
  returns `None` on a `None` total.
- **E4, inspect warns.** Both `inspect` paths (raw transaction or PSBT:
  `main.rs:1529` `inspect_transaction`; a set of `mt1` strings: `main.rs:1628`
  `inspect`) run E1 on the transaction. On a hit, exit code stays 0 and stdout
  (the report) is unchanged; the warning W1 (§4) is added:
  - **text mode:** written to stderr **first**, before every existing stderr
    block, as a blank line followed by `refusal::Warning`'s Display (headline
    line, blank line, body wrapped at 68 and indented 2);
  - **`--json`:** W1's headline (without `WARNING: `) is element **0** of the
    `warnings` array; the existing entries follow in their existing order.
  No warning when E1 has no hit (guard row).
- **E5, decode.** `decode` builds the same `Report`, so E3 removes its panic: it
  exits 0 and prints the hex. `decode` gets **no** new warning in this FIX (T17
  names inspect and encode only; see §11 open point O2).
- **E6, encode refuses a nonexistent outpoint (T16).** In
  `non_witness_utxo_guard`, for each PSBT input in index order: first the
  existing hash comparison (a mismatch keeps its existing refusal, unchanged);
  then, if the hash matches and the record has no output at the input's `vout`,
  a §8.2d refusal (text R2, §4). The refusal fires whatever else is present: a
  `witness_utxo`, `--input-value` for that input, both, or neither. The first
  offending input in index order is named. It runs where the guard runs today
  (`main.rs:442`), before §8.2c's `require_psbt_input_values`, so §8.2c's false
  "carries no UTXO record" text is no longer reached for this case.
  `psbt_input_value`'s fall-through for a short record (`validate.rs:294-298`)
  stays as defence in depth; `encode` no longer reaches it.
- **E7, exit codes.** No exit code changes meaning: refusals exit 1, inspect and
  decode exit 0; exit 101 (panic) is no longer reachable by output values.
- **E8, out of scope (filed, not fixed).** The **input**-value sums
  (`validate.rs:156`, `validate.rs:219`, `main.rs:627`, `report.rs` `total_in +=`)
  stay as they are; they overflow on hostile `witness_utxo` values, a new finding
  filed in §12, not a key of this row.

## 4. Holder-visible texts (all PROPOSED, architect confirms)

Each goes to the master wording list (`mnemonic-refugium`
`design/WORDING_REVIEW.md`) as a row after merge (listed in the author's report).
Rendered by the existing `Refusal` / `Warning` Display (`refusal.rs`): verdict or
headline on one line, then a blank line, then the body wrapped at 68 columns and
indented 2. The source strings are single paragraphs (no `\n`).

- **Verdicts V1-V3** (E1; shared by R1 and W1, one source):
  - V1 `the transaction has 0 outputs`
  - V2 `output {n} pays {v} sat, more than 21,000,000 BTC`
  - V3 `outputs 0 to {n} total {t} sat, more than 21,000,000 BTC`
- **Shared sentence S** (begins R1 and W1 bodies):
  `Bitcoin's consensus rules reject a transaction with no outputs, with an output above 21,000,000 BTC (2100000000000000 sat, every satoshi there will ever be), or with outputs that together exceed that.`
- **R1, encode refusal** (E2). First line `mt encode: REFUSED — §8.2b, {V}`.
  Mechanism: S + ` This transaction can never be broadcast, so engraving it produces a plate that is not a backup of anything.`
  Remedy: `Re-export the transaction from the wallet that built it.`
- **W1, inspect warning** (E4). Headline: `{V}, so no node will ever accept this transaction.`
  Body: S + ` These bytes can never be broadcast, whatever the fee or the node. mt shows what they say; it cannot make them valid.`
- **R2, encode refusal** (E6). First line
  `mt encode: REFUSED — §8.2d, input {n} spends output {vout} of {txid}, which has {k} output(s)`
  (`{txid}` the input's previous-output txid in Core display order, `{k}` the
  record's output count, plain integers). Mechanism:
  `This input's non_witness_utxo is the WHOLE previous transaction, and it hashes to the input's txid, so it IS that transaction. It has no output {vout}, so the output this input spends does not exist and no node will ever accept the transaction. No value supplied with --input-value, and no witness_utxo, can change that: the record proves the output is not there.`
  Remedy: `Re-export the PSBT from the wallet that built it.` (the existing §8.2d
  remedy, verbatim).

Exactly as printed by the author's scratch build (stderr):

    mt encode: REFUSED — §8.2b, the transaction has 0 outputs

      Bitcoin's consensus rules reject a transaction with no outputs, with
      an output above 21,000,000 BTC (2100000000000000 sat, every satoshi
      there will ever be), or with outputs that together exceed that. This
      transaction can never be broadcast, so engraving it produces a plate
      that is not a backup of anything.

      Re-export the transaction from the wallet that built it.

    (inspect, stderr begins:)

    WARNING: the transaction has 0 outputs, so no node will ever accept this transaction.

      Bitcoin's consensus rules reject a transaction with no outputs, with
      an output above 21,000,000 BTC (2100000000000000 sat, every satoshi
      there will ever be), or with outputs that together exceed that.
      These bytes can never be broadcast, whatever the fee or the node. mt
      shows what they say; it cannot make them valid.

    mt encode: REFUSED — §8.2d, input 0 spends output 99 of 9b7c8a80ddc880a9c8f0c6f4a98f2861dd03851f245c14edd25c66717839b715, which has 2 output(s)

      This input's non_witness_utxo is the WHOLE previous transaction, and
      it hashes to the input's txid, so it IS that transaction. It has no
      output 99, so the output this input spends does not exist and no
      node will ever accept the transaction. No value supplied with
      --input-value, and no witness_utxo, can change that: the record
      proves the output is not there.

      Re-export the PSBT from the wallet that built it.

## 5. Spec, README, help and manual delta

- **Spec (`design/SPEC_mt_v0_1.md`, the mt copy; precedent Q0 of
  `FIX_bughunt_mt_a.md` §7 STOP 2: edit the mt copy; the mnemonic-engrave sync is
  owed as for the sibling FIXes; no PROVENANCE edit).** Four edits, verbatim:
  1. §8.2b: replace line 2224 `   - **no duplicate outpoints**, and **`vin` non-empty**.` with the two lines

         - **no duplicate outpoints**, and **`vin` non-empty**;
         - **`vout` non-empty, every output at most 21,000,000 BTC, and the outputs' running total at most 21,000,000 BTC** (bughunt-1119 FIX-E, ruling T17), checked in Core's `CheckTransaction` order: `vin`, then `vout`, then each output's value and the running total in index order, then duplicate outpoints. The first hit is refused, naming the output or the range of outputs. Every output sum is a checked sum. `inspect` never refuses: it prints the same finding as the first warning.

     (each indented three spaces like the neighbouring bullets).
  2. §8.2d: after line 2348 (ends "A mismatch is a refusal naming both txids.")
     append to the same paragraph:
     ` A record whose hash matches but which has no output at the input's `vout` is also refused, always: it proves the outpoint does not exist, so no witness_utxo and no --input-value can make it valid (bughunt-1119 FIX-E, ruling T16).`
  3. §8.2e table: after line 2440 (`| §8.2b value balance | ...`) add the row
     `   | §8.2b outputs in range | ✓ | ✓ | ✓ |`.
  4. The `--json` paragraph of item `b.` (the `--quiet` item beginning at line
     3476): in the sentence at line 3489, immediately after "before the
     stdout-mode headline;", insert
     ` in `inspect`, §8.2b's output-range warning, when present, is the first entry;`
     so the sentence reads "... after the duplicate entries and, in `decode`,
     before the stdout-mode headline; in `inspect`, §8.2b's output-range warning,
     when present, is the first entry; the JSON omits ...".
- **README:** none (grep of `README.md` for `0 inputs`, `non_witness_utxo`,
  `21,000,000`, `refus`: no text that changes).
- **`--help`:** none (no flag added or changed).
- **Toolkit manual:** none. mt has no chapter in `mnemonic-toolkit`
  `docs/manual/src/40-cli-reference/` (checked at origin/master: 41-mnemonic,
  42-md, 43-ms, 44-mk-cli, 45-mr, 46-mr-sitting only).

## 6. RED rows

All 20 rows are in a new test binary `crates/mt-cli/tests/tx_validity.rs`
(`mt-cli::tx_validity`). Its exact text is the sidecar
`design/FIX_bh1119_mt_tx_validity_guards/tx_validity.rs.txt` (sha256
`9d65ef325b4f1f1aa38c15d0e4e8861f554e24e1ecde2e325f1bd40d4039830a`); the builder's
RED commit copies it to that path unchanged. Rows drive `mt encode`, `mt inspect`
and `mt decode` offline (`--bitcoin-cli /nonexistent/bitcoin-cli`) on
`tests/fixtures/p5_base.json`: I16 rows mutate `raw_hex` (two signed segwit
inputs, one output); I17 rows build a finalized legacy PSBT over
`legacy_parent_hex` (2 outputs, real vout 1). One row uses `common::node_stub`.
The texts are constants at the top of the file (`ZERO_OUT`, `SUFFIX`,
`BODY_PHRASE`, helpers `refused` and `warned`), so a wording change edits only them
(and the verdict literals in the rows).

Command (run by the author on 4d1461c with the rows and §7's test changes; log
`/scratch/cache/mt-tx-validity-red-full.log`):

    cargo nextest run --locked --build-jobs 6 -j 6 --workspace --no-fail-fast

Recorded: `360 tests run: 343 passed, 17 failed`; the 17 are the 16 RED rows below
and §7's changed test, each at the assertion named. The 4 guards pass.
`cargo nextest list --locked -p mt-cli --test tx_validity` lists 20.

| row (`bh1119_mt_` prefix omitted) | input | GREEN expects | at 4d1461c |
|---|---|---|---|
| `i16_encode_refuses_zero_outputs` | outputs cleared | rc 1, stdout empty, line R1 with V1 | FAIL: rc 0 |
| `i16_encode_refuses_output_above_money_range` | output 0 = 2100000000000001 | rc 1, R1 `output 0 pays 2100000000000001 sat, more than 21,000,000 BTC` | FAIL: rc 0 |
| `i16_encode_refuses_output_total_above_money_range` | two outputs of 1100000000000000 | rc 1, R1 `outputs 0 to 1 total 2200000000000000 sat, more than 21,000,000 BTC` | FAIL: rc 0 |
| `i16_encode_output_sum_overflow_refuses_not_panics` | two outputs of 2^63 | rc 1 (not 101), R1 `output 0 pays 9223372036854775808 sat, ...` | FAIL: rc 101 |
| `i16_encode_output_range_checked_before_duplicate_inputs` | input 1 repeats input 0's outpoint; output 0 = 2100000000000001 | R1 V2 (not the duplicate refusal) | FAIL: `§8.2b, input 1 repeats outpoint ...` |
| `i16_encode_output_at_money_range_is_accepted` (guard) | output 0 = 2100000000000000 | rc 0, strings on stdout, no REFUSED | PASS |
| `i16_inspect_warns_zero_outputs` | outputs cleared | rc 0, stderr line `WARNING: ` + W1(V1), body phrase, stdout `OUT       0 output(s)` | FAIL: warning missing |
| `i16_inspect_warns_output_above_money_range` | output 0 = 2100000000000001 | rc 0, W1(V2) | FAIL: warning missing |
| `i16_inspect_output_sum_overflow_warns_not_panics` | two outputs of 2^63 | rc 0, W1(V2 with 9223372036854775808) | FAIL: rc 101 |
| `i16_inspect_with_node_overflow_fee_unknown` | two outputs of 2^63; node_stub gettxout `{"value": 1.00000000, ...}` | rc 0, W1, stdout line starting `FEE       UNKNOWN` | FAIL: rc 101 |
| `i16_inspect_json_carries_the_warning_first` | outputs cleared; `--json` | `warnings[0]` = V1 + `, so no node will ever accept this transaction.` | FAIL: warnings[0] is "no bitcoind reachable: ..." |
| `i16_inspect_strings_warns_zero_outputs` | mt1 strings (mt-codec `pipeline::encode`) of the output-less tx | rc 0, W1(V1) | FAIL: warning missing |
| `i16_inspect_valid_tx_has_no_validity_warning` (guard) | `raw_hex` unchanged | rc 0, no "no node will ever accept" | PASS |
| `i16_decode_output_sum_overflow_does_not_panic` | mt1 strings of the 2^63 x 2 tx; `decode` | rc 0, stdout = the tx hex | FAIL: rc 101 |
| `i17_encode_refuses_missing_vout_with_input_value` | vout 99; `--input-value 0:10` | rc 1, stdout empty, R2 `input 0 spends output 99 of 9b7c8a80...b715, which has 2 output(s)` | FAIL: rc 0 |
| `i17_encode_refuses_missing_vout_with_witness_utxo` | vout 99; witness_utxo = parent output 1 | R2 | FAIL: rc 0 |
| `i17_encode_refuses_missing_vout_without_any_value` | vout 99; nothing else | R2 | FAIL: `§8.2c, input 0 carries no UTXO record and no supplied value` |
| `i17_encode_names_the_first_offending_input` | inputs vout 1 and vout 99; `--input-value 1:10` | R2 with `input 1 spends output 99` | FAIL: rc 0 |
| `i17_hash_mismatch_still_refused_first` (guard) | outpoint txid `11..11`, vout 99 | existing `§8.2d, input 0's non_witness_utxo hashes to 9b7c...b715, not 1111...1111` | PASS |
| `i17_existing_vout_still_encodes` (guard) | vout 1 | rc 0, strings, no REFUSED | PASS |

## 7. Tests that change

Exact diff: sidecar `design/FIX_bh1119_mt_tx_validity_guards/test-changes.diff`
(sha256 `a8d2a1274d3ac30a4922aff9fde0c941d16d79072b9c418cd967c73f18edc35a`),
applied in the RED commit with `git apply`.

- `crates/mt-cli/tests/refusals.rs`
  `a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound` (name kept): the
  vacuous `if ok { ... }` becomes unconditional: `assert_refused(.., "§8.2d")`,
  stderr contains `REFUSED — §8.2d, input 0 spends output {high} of {txid}, which
  has {k} output(s)`, and no `TXID-BOUND`. At 4d1461c it fails (refused by §8.2b's
  absurd fee instead).
- `crates/mt-cli/tests/refusals.toml`: four entries, inserted after the existing
  `§8.2d` entry: `§8.2b` x3 (`bh1119_mt_i16_encode_refuses_zero_outputs`,
  `..._output_above_money_range`, `..._output_total_above_money_range`, check
  `src/validate.rs::value_guard`) and `§8.2d`
  `a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound` (check
  `src/validate.rs::non_witness_utxo_guard`). This is why E1 lives **inside**
  `value_guard` and E6 inside `non_witness_utxo_guard`: `mutate-refusals.sh`
  neuters those functions by name, and each entry's test goes red when it does
  (author's scratch run: all 42 entries red).
- No other existing test changes: with the author's scratch fix the whole suite
  passes with only these two files changed.

## 8. Build order

1. **RED commit:** this document and its sidecars (already on this branch), plus
   `crates/mt-cli/tests/tx_validity.rs` copied from the sidecar and
   `test-changes.diff` applied. Expect §6's `343 passed, 17 failed`.
2. **GREEN commit:** `crates/mt-cli/src/validate.rs` (E1 as one function giving
   the first verdict, a `MAX_MONEY` constant of 2100000000000000 citing Core
   `amount.h`, a checked output-total helper, E2 in `value_guard`, E6 in
   `non_witness_utxo_guard`, R1/R2/W1 texts, doc comments of both guards updated),
   `report.rs` (E3), `main.rs` (E3 at :626; E4 in both inspect paths), and §5's
   spec text. No `Cargo.toml` change. Expect 0 failures.
3. **Interaction with row 2 (FIX-A node tri-state, `fix/mt-node-tristate`):**
   both touch `report.rs` `Report::build` and `main.rs` `encode`/`inspect`, in
   different lines (row 2: node answers and STATUS; this FIX: output sums and the
   warning). Whichever merges second rebases; no rule interacts (row 2's I15 rule
   already treats a node value above 21,000,000 BTC as unanswered). The
   `i16_inspect_with_node_overflow_fee_unknown` row asserts only rc, W1 and the
   `FEE       UNKNOWN` prefix, so row 2's stub changes do not move it.
4. The builder lists §12's lines in its report; nothing is appended to
   `design/FOLLOWUPS.md` in the PR.

Author's scratch evidence (not normative; no production code on this branch): a
scratch implementation of E1-E6 with the test changes gave `360 tests run: 360
passed`, and `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D
warnings`, `./scripts/check-refusal-coverage.sh` (`42 refusal tests over 18 ruled
refusals`), `./scripts/mutate-refusals.sh` (`all 42 refusal tests go red`),
`./scripts/journeys.sh` and `cargo test --locked --workspace --doc` all exit 0
(logs `/scratch/cache/mt-tx-validity-green-*.log`; diff
`/scratch/cache/mt-tx-validity-scratch-fix.diff`).

## 9. Coverage map (rule -> rows; the `bh1119_mt_` prefix is omitted)

| rule | rows |
|---|---|
| E1.1 no outputs (V1) | i16_encode_refuses_zero_outputs, i16_inspect_warns_zero_outputs, i16_inspect_strings_warns_zero_outputs, i16_inspect_json_carries_the_warning_first |
| E1.2 output above MAX_MONEY (V2) | i16_encode_refuses_output_above_money_range, i16_inspect_warns_output_above_money_range |
| E1.2 running total (V3) | i16_encode_refuses_output_total_above_money_range |
| E1 boundary (exactly MAX_MONEY passes) | i16_encode_output_at_money_range_is_accepted |
| E2 order (before duplicates) | i16_encode_output_range_checked_before_duplicate_inputs |
| E2 every form / no values | all i16_encode rows (raw hex, no node, no values) |
| E3 checked sums | i16_encode_output_sum_overflow_refuses_not_panics, i16_inspect_output_sum_overflow_warns_not_panics, i16_inspect_with_node_overflow_fee_unknown, i16_decode_output_sum_overflow_does_not_panic |
| E4 text placement (first on stderr) | i16_inspect_warns_zero_outputs (line present; first-ness is checked by review of the transcript in §4, no row: a row would pin the existing no-node block too) |
| E4 JSON element 0 | i16_inspect_json_carries_the_warning_first |
| E4 both inspect paths | i16_inspect_warns_* (raw), i16_inspect_strings_warns_zero_outputs (strings) |
| E4 no hit, no warning | i16_inspect_valid_tx_has_no_validity_warning |
| E5 decode | i16_decode_output_sum_overflow_does_not_panic |
| E6 with --input-value / witness_utxo / neither | i17_encode_refuses_missing_vout_with_input_value, i17_encode_refuses_missing_vout_with_witness_utxo, i17_encode_refuses_missing_vout_without_any_value; refusals.rs a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound |
| E6 first offending input | i17_encode_names_the_first_offending_input |
| E6 hash check first | i17_hash_mismatch_still_refused_first |
| E6 real vout unaffected | i17_existing_vout_still_encodes |
| E7 exit codes | every row asserts rc |
| R1, R2, W1 texts | rows assert the verdict line and W1's headline exactly, and W1's body by `BODY_PHRASE`; R1/R2 bodies by review of §4's transcript |

## 10. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
The CI `test` job's list (`.github/workflows/ci.yml`) plus the rows:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expect 360 run, 0 failed)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh` (expect `42 refusal tests over 18 ruled refusals`)
7. `./scripts/mutate-refusals.sh` (expect `all 42 refusal tests go red`)
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh` and `bash ci/code-docs-check.sh`
10. Rows by name: `cargo nextest run --locked -p mt-cli --test tx_validity`
    -> `20 tests run: 20 passed`. Fewer than 20 is a FAIL.
11. `grep -n 'to_sat()).sum' crates/mt-cli/src/*.rs` -> no output (E3).

Review: class B (funds path); up to three rounds (full track).

## 11. Open points for the architect (no STOP)

- **O1.** R1, R2, W1 wording (PROPOSED), and W1's position (first on stderr,
  element 0 in JSON).
- **O2.** `decode` prints the same report and gets no warning here (E5). Adding W1
  to `decode` (non-`--quiet` text and `--json`) is one call per path; the
  architect may fold it in, with rows mirroring the inspect ones.
- **O3.** E8: the input-value sums (§12's new line). The natural rule mirrors E1
  (Core `CheckTxInputs`, `bad-txns-inputvalues-outofrange`, `tx_verify.cpp:187`),
  but a refusal text and its interaction with asserted values (two
  `--input-value` of 21,000,000 each) are product points, so it is filed, not
  folded.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

One line owed to this build (closed by its GREEN commit) and one new Important
filed by the author (E8): the input-value sums overflow on hostile `witness_utxo`
values (exit 101 in the debug build, wrap in release).

## 13. Review fold r1

Source: reviews s1 (Opus 5.5) and s2 (Sonnet 5.5) of head bebedc7f.

- **s2 I1, I2** (two E4 behaviours with no failing row): rows
  `bh1119_mt_i16_inspect_strings_json_carries_the_warning_first` (strings path,
  `--json`, `warnings[0]` = V1 + SUFFIX) and
  `bh1119_mt_i16_inspect_warning_is_first_on_stderr` (W1 precedes the no-node
  warning, raw and strings input). No code change; each fails on its plant.
- **s1 I1 part (a)** (E8 input sums): every input-value sum is now checked, via
  `validate::checked_in_total` (`Ok(None)` = a value unknown, `Err(())` = known
  values overflow u64): `value_guard`, `low_fee_warning`, `main.rs` encode
  `in_total`, and `report.rs` `total_in` (now `Option<u64>`, `checked_add`). An
  overflow gives no total and no fee figure, so the report prints the existing
  `FEE UNKNOWN` row; `value_guard` skips, as for an unknown value. No new text.
  Rows: `bh1119_mt_e8_input_sum_overflow_does_not_panic` and
  `bh1119_mt_e8_input_sum_overflow_prints_no_fee_figure`.
- **E8 is no longer filed open as an input-sum line** (§1 E8, §12): the sums are
  fixed here. Only **s1 I1 part (b)** stays open: whether `encode` refuses a PSBT
  whose input values break Core's `MoneyRange` (open point O3, FOLLOWUPS).
- Gate row 11 extends to input sums: `grep -nE 'to_sat\(\)\)\.sum|sum::<Option|total_in \+=' crates/mt-cli/src/*.rs` prints nothing.
  Rows by name: 24 in `tx_validity` (20 + 4).
