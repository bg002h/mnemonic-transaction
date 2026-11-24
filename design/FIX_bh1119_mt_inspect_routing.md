# FIX: inspect routing (bughunt-1119 FIX table row 37, "FIX-D inspect routing")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-23. Keys: bh-mt-I3 (I), bh-mt-I5 (I),
bh-mt-I6 (I). Track: small (one Opus review round). Not a funds-path (class B) row.
Rulings: none named for this row (architect table row 37: "none"); T17 ("inspect warns,
encode refuses", I16) is cited only where section 11 weighs it.
Base: origin/main `88ac6d9` (rows 22 and 23 merged; FIX-C's sniff-by-decoding is in).
Branch `fix/mt-inspect-routing`. Every new or changed holder-visible text is marked
**PROPOSED (architect confirms)**. **Two STOPs are open (section 11, Q1 and Q2)**; the
rules, rows and gate below follow the recommended option A of each, and section 11 gives
what changes if the architect picks another.

Not to be confused with the older `design/FIX_bughunt_mt_d.md` (a different hunt).

## 1. Findings (run by the author on a base-equivalent binary)

Binary: `/scratch/cache/wt-build-mt22/target/debug/mt`, built from `783647f`;
`git diff 749735d 88ac6d9 -- crates Cargo.lock` is empty, so it is the base's code.
Offline (`--bitcoin-cli /nonexistent`), umask 077, consultant fixtures (`final.b64`,
`final.psbt` = p5 `finalized_psbt_b64` and its decoding; `even.txt` = corpus `even`).

- **I5.** `mt inspect --json --in final.psbt` exits 1:
  `mt inspect: REFUSED — §1.1e, input is not text`. The same PSBT as base64 exits 0.
  `inspect` (`crates/mt-cli/src/main.rs:1761-1763`) calls `read_input`
  (`main.rs:1025-1082`), whose last step (`:1073-1081`) converts to UTF-8 before
  anything looks for the `psbt\xff` magic. `encode` sniffs raw bytes (`main.rs:420-421`).
- **I6.** Routing is `looks_like_an_mt1_set` (`main.rs:1648-1650`: the trimmed text
  starts `mt1`), at `main.rs:1780`. Four legal inputs that `verify` accepts (exit 0,
  `mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.` for the first two) go
  to the transaction sniffer and are refused:
  - `mt encode --group-size 2 --in final.b64 > grouped` (lines begin `mt 1p ge j7`),
    then `mt inspect --in grouped`: `mt inspect: REFUSED — §8.2e, input is 9 `mt1`
    strings — that is mt encode's OUTPUT, not its input`, and its remedy block lists
    `mt inspect   report what is IN the set`, the command that just failed
    (`input.rs:171-196`).
  - `--group-size 1` (lines begin `m t 1 p g e`): the same refusal.
  - `even.txt` with the first `mt1` typed `mtl`: `mt inspect: REFUSED — §8.2e, input is
    not a PSBT or a raw transaction (527 bytes)`.
  - `even.txt` with the first line's 11-character prefix elided: the same, `(516 bytes)`.
- **I3.** `inspect_transaction`'s PSBT arm (`main.rs:1662-1679`) reports
  `psbt.unsigned_tx` and passes no values to `Report::build` (`:1684`, `&[]`).
  - Legacy input (consultant, `tests/legacy.rs::legacy_psbt(true, None)`): inspect
    prints txid `015cfdd3…`, encode `f53f4701…` (the extracted transaction). A
    legacy `scriptSig` is part of the txid; the unsigned template's is empty. The
    comment at `:1672-1677` ("the txid is identical either way") is false for legacy
    and wrapped inputs. With a node, the same wrong txid is what `Report::build` asks
    `is_confirmed` about.
  - p5 (native segwit, both inputs carry `non_witness_utxo`): `mt encode` prints
    `FEE       0.00100000 BTC` and both inputs `TXID-BOUND`; `mt inspect --in final.b64`
    prints `FEE       UNKNOWN   (needs input values, which the transaction` and both
    inputs `UNKNOWN`; JSON `"fee": null`, `"provenance": "Unknown"`. Spec §1.1's row
    table already says FEE is present when "the input was a PSBT carrying values".
  - Found on the way, same cause: the no-node warning prints the legacy malleability
    caveat (`IF AN EXPLORER SAYS THIS TXID DOES NOT EXIST …`) for p5, a native-segwit
    PSBT, because `has_legacy` (`main.rs:1736`) reads the template's witnesses, which are
    always empty. Its fee lines also say `UNKNOWN` / `(the fee needs input values, which
    are not in the tx)` (`report.rs:684-691`), which after the fix would contradict the
    FEE row.

## 2. Rulings

None bind this row. Spec facts relied on: §1.1 "A supplied PSBT is compared against its
EXTRACTED transaction, per §10.13(c) … a PSBT holds two transactions whose txids differ
for every legacy and `sh(wsh(…))` input" (`SPEC_mt_v0_1.md:525-529`); §1.1 row table
`FEE` "a node is reachable, **or** the input was a PSBT carrying values" (`:687`).

## 3. Behaviour after the fix (rules)

- **R1 inspect reads bytes (I5).** `read_input` is split: a byte reader
  (file / argv material / stdin, the argv-unused warning, `txid_paste_guard`,
  `secret_guard`, all unchanged and in today's order) and the UTF-8 step
  (`main.rs:1073-1081`, text unchanged). `decode` and `verify` call both, so their
  behaviour is unchanged. `inspect` calls only the byte reader and routes on the bytes
  (R2). Text is made from the bytes only on the strings route.
- **R2 inspect routes by what the bytes ARE (I5, I6).** In this order:
  1. Bytes empty after trimming ASCII whitespace: strings route (unchanged; `no strings
     found in the input`).
  2. `input::sniff(bytes)` returns `Psbt` or `RawHex`: transaction route
     (`inspect_transaction`, R3). Sniff recognises a PSBT by its magic or by decoding
     (FIX-C, T23); an `mt1` set is never hex (it carries `m` and `t`) and never decodes
     to `psbt\xff`, so this step cannot capture a set.
  3. Sniff refused, the bytes are UTF-8, and `strings_subject(text)` is true: strings
     route, through the unchanged `read_strings::read` (the reader `decode` and `verify`
     use; it undoes grouping, `mtl`/`mti` and elision).
  4. Otherwise: sniff's own refusal, with the verb rewritten to `inspect` (today's
     `as_inspect`).
  `strings_subject(text)` (replaces `looks_like_an_mt1_set`): let `t` be `text` with
  every whitespace character removed, ASCII-lowercased. True iff `t` does not begin
  `chnidp` (the lowercased `PSBT_BASE64_STEM`) **and** (`t` contains `mt1` **or** `t`
  begins `mtl` or `mti`). `i` is not in the bech32 charset, so no `mt1` string or elided
  line begins `chnidp`; the exclusion keeps a damaged base64 PSBT that happens to
  contain `mt1` on the PSBT refusal (row 22). Consequences, all by existing texts:
  `mt encode`'s pasted-output refusal (`input.rs:171`) becomes unreachable from
  `inspect` (any input that starts `mt1` is a strings subject); a non-UTF-8 input that
  is not a PSBT gets sniff's step-4 refusal (`§8.2e, input is not a PSBT or a raw
  transaction (N bytes)`) instead of `§1.1e, input is not text` (row 14).
- **R3 a PSBT subject is its extracted transaction, with its records' values (I3).**
  In `inspect_transaction`'s PSBT arm, after the unchanged parse and `psbt_end_guard`:
  1. `validate::non_witness_utxo_guard(&psbt)`, verb rewritten to `inspect` (Q2,
     option A).
  2. `validate::finalized_guard_psbt(&psbt)`, verb rewritten to `inspect` (Q1, option A).
  3. The reported transaction is `psbt.extract_tx_unchecked_fee_rate()`.
  4. The values passed to `Report::build` are, per input, `validate::psbt_input_value`
     mapped `TxidBound -> Provenance::TxidBound`, `PsbtClaimed ->
     Provenance::PsbtClaimed`, exactly `encode`'s mapping (`main.rs:451-480` without
     the `--input-value` arm; `inspect` has no such flag). The builder moves the record
     part into one helper both verbs call, so the two cannot drift.
  The raw-hex arm is unchanged (no values). The strings route is unchanged.
- **R4 the no-node warning tells the truth about a known fee (I3).**
  `report::no_node_warning` gains a parameter `fee_known: bool`. When true, its two fee
  lines (`report.rs:684-691`) are replaced by the PROPOSED pair in section 4; when
  false they are unchanged. Callers: `inspect_transaction` passes `r.fee.is_some()`;
  `decode` (`main.rs:1479`) and the inspect strings route (`:1858`) pass `false` (a set
  carries no values). `has_legacy` keeps its expression, now evaluated on the extracted
  transaction, so the malleability caveat follows real witnesses (rows 8, 9).
- **R5 comments.** The false comment at `main.rs:1672-1677` and the routing comment at
  `:1764-1779` are rewritten to describe R2 and R3; the `looks_like_an_mt1_set` doc
  (`:1636-1647`) moves to `strings_subject`.

## 4. Holder-visible texts

- **No-node fee lines when the fee is known (R4), PROPOSED (architect confirms).** The
  first line keeps today's column (the status word starts where `UNKNOWN` starts):

      "           - what fee does it pay?                             FROM THE PSBT"
      "             (the PSBT's own input records give it: see the FEE row)"

  `TO RESOLVE ALL FOUR AT ONCE, either:` is unchanged: a node still confirms the
  values against the chain.
- **Q1 option A / Q2 option A refusals:** existing texts (`validate.rs:41-78`,
  `:342-383`), first line `mt inspect: REFUSED — §8.1, …` / `mt inspect: REFUSED —
  §8.2d, …`; mechanism and remedy unchanged. Holder-visible change: `inspect` now
  refuses these two inputs (today it reports them). Exit 1, nothing on stdout.
- **Changed by routing, existing texts:** a non-UTF-8, non-PSBT input to `inspect` gets
  `mt inspect: REFUSED — §8.2e, input is not a PSBT or a raw transaction (N bytes)` and
  its step-4 mechanism (`input.rs`, `unrecognised`) instead of `§1.1e, input is not text`.
- **Report rows** (no new text; existing rows now filled): for a finalized PSBT, `TX` is
  the extracted txid, `FEE` and the `INPUTS` value/provenance columns carry the PSBT's
  record values, JSON `fee` and `inputs[].sats` / `provenance` likewise; the
  malleability caveat appears only when an input has an empty witness.

## 5. Spec, README, help, manual, CHANGELOG

- **Spec delta, PROPOSED.** `design/SPEC_mt_v0_1.md` is a copy owned by
  mnemonic-engrave (CLAUDE.md "Design files are copies"). Applied there first, then this
  FIX's build re-syncs mt's copy and the `design/PROVENANCE.md` source-commit row.
  1. §1.1 row table, `TX` source: append `For a PSBT subject, the transaction it
     EXTRACTS to (§10.13 c), never its unsigned template.`
  2. §1.1 row table, `INPUTS` source: `values from \`gettxout\`` becomes `values from
     \`gettxout\`, or, where the node does not answer, the PSBT's own UTXO records with
     their provenance`.
  3. §1.1, after "`inspect` reports what is IN the artifact" (`:531`), one paragraph:
     `` `inspect` takes an `mt1` set, a PSBT (binary or base64) or a raw transaction.
     It reads the bytes first: whatever §8.2e's sniff recognises is a transaction
     subject; any other text that contains `mt1` (after whitespace is removed), or
     begins `mtl` or `mti`, goes to §1.1e's string reader, so grouped, elided and
     misread-separator sets read as they do in `verify`. An unfinalized PSBT, or one
     whose `non_witness_utxo` fails §8.2d, is refused as `encode` refuses it. ``
     (The last sentence follows Q1/Q2 option A.)
- **README:** none (it does not describe inspect's routing).
- **Toolkit manual:** none; `mnemonic-toolkit` `docs/manual/src/40-cli-reference/` has
  no `mt` page (41-mnemonic, 42-md, 43-ms, 44-mk-cli, 45-mr, 46-mr-sitting).
- **CHANGELOG:** none in this FIX (entries are written at release).

## 6. RED rows

All 23 rows are one new test binary `crates/mt-cli/tests/bh1119_mt_inspect_routing.rs`
(`mt-cli::bh1119_mt_inspect_routing`), exact text in the sidecar
`design/FIX_bh1119_mt_inspect_routing/inspect_routing.rs.txt` (sha256 `8f2b77965c903ae70b7bda337143d980737dec16779b2898094d664e844533d5`;
rustfmt edition 2024 clean, `cargo clippy -p mt-cli --test bh1119_mt_inspect_routing -- -D
warnings` clean). Command:

    cargo nextest run --locked --build-jobs 2 -j 2 -p mt-cli --test bh1119_mt_inspect_routing

Expected on base `88ac6d9`: `23 tests run: 6 passed, 17 failed` (section 6a gives the
author's measured result). Each row, expected base behaviour -> after:

| # | row (`bh1119_mt_` omitted) | base | after |
|---|---|---|---|
| 1 | `i3_inspect_reports_the_extracted_txid_of_a_legacy_psbt` | FAIL: JSON txid is the template's | txid = extracted |
| 2 | `i3_inspect_json_carries_a_txid_bound_value_and_fee` | FAIL: `"fee": null` | fee 100000 verified, input TxidBound 1000000000 |
| 3 | `i3_inspect_json_labels_a_witness_utxo_value_psbt_claimed` | FAIL: `"fee": null` | fee 100000 unverified, PsbtClaimed |
| 4 | `i3_inspect_and_encode_agree_on_a_legacy_psbt` | FAIL: TX rows differ | TX, FEE, INPUTS rows equal |
| 5 | `i3_inspect_reports_p5_input_values_offline` | FAIL: `FEE       UNKNOWN` | `FEE       0.00100000 BTC`, two TXID-BOUND rows |
| 6 | `i3_no_node_warning_follows_a_fee_known_from_the_psbt` | FAIL: UNKNOWN fee lines | PROPOSED fee lines |
| 7 | `i3_no_node_warning_keeps_unknown_fee_for_raw_hex` | pass (control) | pass |
| 8 | `i3_segwit_psbt_omits_the_legacy_caveat` | FAIL: caveat printed | absent |
| 9 | `i3_legacy_psbt_keeps_the_legacy_caveat` | pass (control) | pass |
| 10 | `i3_inspect_refuses_an_unfinalized_psbt` (Q1 A) | FAIL: exit 0 | `mt inspect: REFUSED — §8.1, 1 of 1 inputs are not finalized (input 0)` |
| 11 | `i3_inspect_refuses_a_non_witness_utxo_that_is_not_the_parent` (Q2 A) | FAIL: exit 0 | `mt inspect: REFUSED — §8.2d, input 0's non_witness_utxo hashes to <got>, not <want>` |
| 12 | `i5_inspect_reads_a_binary_psbt_file` | FAIL: `§1.1e, input is not text` | exit 0, JSON equal to the base64 form's |
| 13 | `i5_inspect_reads_a_binary_psbt_on_stdin` | FAIL: same | same, via stdin |
| 14 | `i5_inspect_names_non_text_junk_by_the_sniffer` | FAIL: `§1.1e, input is not text` | `mt inspect: REFUSED — §8.2e, input is not a PSBT or a raw transaction (11 bytes)` |
| 15 | `i5_decode_still_refuses_non_text` | pass (control) | pass |
| 16 | `i6_inspect_reads_group_size_2_output` | FAIL: pasted-output refusal | exit 0, txid `4665e0aa…` |
| 17 | `i6_inspect_reads_group_size_1_output` | FAIL: same | same |
| 18 | `i6_inspect_reads_an_mtl_first_line` | FAIL: `(527 bytes)` step-4 | exit 0, even's txid |
| 19 | `i6_inspect_reads_an_elided_first_line` | FAIL: `(516 bytes)` step-4 | exit 0, even's txid |
| 20 | `i6_a_lone_mtl_string_reaches_the_strings_reader` | FAIL: `(87 bytes)` step-4 | exit 1 from the strings reader/codec |
| 21 | `i6_foreign_text_keeps_the_sniffer_refusal` | pass (control) | pass |
| 22 | `i6_damaged_base64_psbt_containing_mt1_keeps_the_psbt_refusal` | pass (control) | pass |
| 23 | `i6_empty_input_stays_on_the_strings_path` | pass (control) | pass |

The base first lines quoted for rows 12-23 were measured on the base-equivalent binary
(section 1); rows 10 and 11's "after" lines are the existing guard texts.

### 6a. Measured

Run by the author on base `88ac6d9` with both sidecars installed (the command above;
run twice, before and after the rustfmt pass, same result):
`Summary  23 tests run: 6 passed, 17 failed, 0 skipped`. Passes are exactly the six
controls (rows 7, 9, 15, 21, 22, 23). Every failure is the defect: row 1 JSON txid
`015cfdd3…` vs extracted `f53f4701…` (the consultant's pair); rows 2, 3 `"fee": null`;
row 4 `TX` rows differ (same pair); row 5 `FEE       UNKNOWN   (needs input values, …`;
rows 10, 11 exit 0; rows 12, 13 `§1.1e, input is not text`; row 14 the same first line;
rows 16, 17 the pasted-output refusal; rows 18, 19, 20 step-4 at 527, 516, 87 bytes;
rows 6, 8 the UNKNOWN fee lines and the caveat present.

## 7. Tests that change

- `crates/mt-cli/src/main.rs:2492-2522` unit test
  `an_mt1_set_is_recognised_by_how_it_starts_not_by_a_substring` tests the removed
  `looks_like_an_mt1_set`. Replaced by `strings_subject_admits_recoverable_sets_only`
  over `strings_subject`, asserting: true for `mt1qqqqq`, `  \n mt1qqqqq`, `MT1QQQQQ`,
  `mt 1q qq` (grouped), `qqqq\nmt1qq` (elided first line), `mtlqqqq`, `mtiqqqq`; false
  for `cHNidP8BAHsCAAAAmt1AAAAA` (stem excluded), `hello`, `abababa`, `` (empty).
- `crates/mt-cli/src/report.rs:814` unit test `no_node_warning_names_both_ways_out`:
  the call gains the `fee_known` argument `false`; assertions unchanged.
- No integration test changes expected: every existing `inspect` test reads an `mt1`
  set that starts `mt1` (unchanged route) or a raw-hex / base64 input sniff already
  decides. `inspect.rs:732` (`cHNidP8BAHUCAAAA`) stays on the PSBT refusal (R2 step 2
  then 4); `bh1119_mt_input_parsing.rs:474-480` compares two PSBT reports, both now with
  values. The builder confirms by the full suite (gate).
- Added: the row binary and five `tests/refusals.toml` entries (sidecar
  `refusals-toml-append.txt`): rows 10 and 11 (the two PSBT guards newly called at
  `inspect`), rows 14 and 21 (`recognised_guard`, which `inspect` now reaches for
  non-UTF-8 input) and row 22 (`base64_psbt_guard`, held against the new
  `strings_subject` route). Coverage and mutation go 61 -> 66 (coverage measured with the
  sidecars installed: `check-refusal-coverage: 66 refusal tests over 18 ruled refusals`).
  Rows 15 and 23 are controls of inline refusals with no guard fn (`read_input`'s UTF-8
  step, `read_strings::read`'s empty check), so they have no entry and no mutation
  control; their refusal text is composed by the row helper `refused(r, verb, rest)`,
  as every row's is. Flagged for the reviewer: the coverage script's "asserts on a
  §-refusal message" heuristic sees only literal text in a test body, so the helper
  also keeps it from seeing rows 15 and 23; the five rows that have a guard are
  declared.

## 8. Build order

1. Install both sidecars; run the row command; see `23 tests run: 6 passed, 17 failed`.
2. R1 and R2 (byte reader, `strings_subject`, routing): rows 12-14, 16-20 GREEN; 15,
   21-23 stay GREEN.
3. R3 (extraction, values, two guards): rows 1-5, 10, 11 GREEN.
4. R4 (`fee_known`): rows 6, 8 GREEN; 7, 9 stay GREEN.
5. R5 comments; the two unit tests of section 7; spec re-sync and PROVENANCE row once
   mnemonic-engrave carries the section 5 delta.
6. Gate (section 10) once to a file.

## 9. Coverage map (rule -> rows)

| rule | rows |
|---|---|
| R1 inspect reads bytes | 12, 13, 14; control 15 |
| R2 routing (sniff first, `strings_subject`, sniff refusal otherwise) | 16, 17, 18, 19, 20; controls 21, 22, 23; unit test (section 7) |
| R3 extracted transaction, record values, provenance | 1, 2, 3, 4, 5 |
| R3.1 `non_witness_utxo_guard` at inspect (Q2 A) | 11 |
| R3.2 `finalized_guard_psbt` at inspect (Q1 A) | 10 |
| R4 no-node fee lines; caveat on real witnesses | 6, 8; controls 7, 9 |

## 10. Gate (builder, final head; run once to a file, then grep)

    bash design/FIX_bh1119_mt_inspect_routing/gate.sh > /scratch/cache/mt-inspect-routing-gate.log 2>&1

The script runs every step of the CI `test` job once, checks the sidecars are installed
unchanged, and pins the counts. WHAT IT DOES NOT COVER is in its header. Its totals are
`88ac6d9` + this FIX only (459 + 23 = 482 tests; refusal coverage and mutation 66); a head that also
carries rows 35, 38, 39 or the open `fix/mt-*` neighbours must add their counts.

## 11. Open questions (STOP: the architect rules Q1 and Q2)

- **Q1 (STOP) An unfinalized (or partly finalized) PSBT given to `inspect`.** Today it
  is reported as its unsigned template. For a legacy or wrapped input that txid is not
  the txid the transaction will have; for native segwit it is. No ruling covers it;
  T17 ("inspect warns, encode refuses") is about transaction validity (I16).
  - A (recommended): refuse with `finalized_guard_psbt`'s existing text, verb
    `inspect` (row 10). No new text; matches `encode` (§8.1) and `verify --transaction`
    (`main.rs:1582-1594`), the other two PSBT readers; `inspect` then never prints a
    txid that is not the one the plate would carry.
  - B: report the template, plus a warning on stderr and as a JSON `warnings` entry.
    PROPOSED text: `this PSBT is not finalized (N of M inputs): TX is its unsigned
    template's txid, which a legacy or wrapped input changes when it is finalized`.
    Row 10 becomes: exit 0, that warning present, JSON txid = template's.
  - C: report the template silently (today). Leaves the I3 wrong-fact class open for
    legacy inputs; not recommended.
- **Q2 (STOP) A PSBT whose `non_witness_utxo` does not hash to the input's txid, or has
  no output at its vout (§8.2d), given to `inspect`.** Without a check, `psbt_input_value`
  labels a non-matching record `TXID-BOUND`, a false fact; the fix must do something.
  - A (recommended): run `non_witness_utxo_guard` (both its §8.2d refusals), verb
    `inspect` (row 11). No new text; matches `encode`. Tension: its second refusal (no
    output at the vout) proves the transaction invalid, and T17's spirit is "inspect
    warns" for invalid transactions.
  - B: refuse nothing; take a record's value only when it hashes to the input's txid
    (else fall to `witness_utxo` as PSBT-CLAIMED, else UNKNOWN). Silent.
  - C: B plus a PROPOSED warning: `input N's non_witness_utxo is not the transaction it
    spends: its value was not used`.
- **O1 (wording, PROPOSED)** the no-node fee pair (section 4).
- **O2 (decided here, flag if wrong)** the `strings_subject` criterion (R2): the
  consultant asks for "genuine foreign-input diagnostic controls" to stay; this keeps
  every refusal the four controls see today, and routes only text that carries an `mt1`
  (or begins `mtl`/`mti`) to the string reader.
- **O3 (decided here, flag if wrong)** non-UTF-8 non-PSBT input to `inspect` gets sniff's
  step-4 refusal rather than `input is not text` (row 14): `inspect` reads binary PSBTs,
  so "not text" no longer names the problem; the step-4 text names all accepted forms
  except `mt1` strings, which binary input cannot be.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

Closes bh-mt-I3, bh-mt-I5, bh-mt-I6 (none has a line in `design/FOLLOWUPS.md` at
`88ac6d9`). Files none.

## 13. Files this FIX touches (for build ordering against rows 35, 38, 39)

- `crates/mt-cli/src/main.rs` (`read_input` split, `inspect`, `inspect_transaction`,
  `strings_subject`, the shared PSBT-values helper used by `encode` too, comments,
  unit test)
- `crates/mt-cli/src/report.rs` (`no_node_warning` signature and fee lines, unit test)
- `crates/mt-cli/tests/bh1119_mt_inspect_routing.rs` (new)
- `crates/mt-cli/tests/refusals.toml` (append)
- `design/SPEC_mt_v0_1.md`, `design/PROVENANCE.md` (re-sync, step 5)

`input.rs` and `validate.rs` are read, not changed. Row 38 (FIX-G, verdict/notice
integrity) changes notice gating in `main.rs` near the same report code; row 39 (FIX-J,
printed remedies) may change `finalized_guard_psbt`'s remedy (rows 10 here assert the
first line only). Text conflicts only expected.
