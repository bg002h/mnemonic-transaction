# FIX: inspect routing (bughunt-1119 FIX table row 37, "FIX-D inspect routing")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-23. Keys: bh-mt-I3 (I), bh-mt-I5 (I),
bh-mt-I6 (I). Track: small (one Opus review round). Not a funds-path (class B) row.
Rulings: Brian 2026-11-23, row #37 Q1/Q2 (coordinator memory
`mt-inspect-unsigned-psbt-2026-11-23`, binding): `inspect` REPORTS an unsigned or
unfinalized PSBT, its TX labelled PROVISIONAL when signing changes the txid; a
`non_witness_utxo` that does not hash to the input's txid is REPORTED with that input's
amount and the fee UNVERIFIED (never TXID-BOUND), a loud fee-loss warning and
instructions to verify the input; `encode` keeps refusing both. T17 ("inspect warns,
encode refuses", I16) is cited where section 11 records Q3.
Brian 2026-11-24 (#37 Q3, Q4; `coordinator/status/brian-rulings-2026-11-24.md`): Q3, a
`non_witness_utxo` that hashes to the input's txid but has no output at its vout: "If
it's an invalid transaction there is no reason to encode it or engrave it, if we can
display/decode it with a warning." So `inspect` REPORTS it with a warning (option B) and
EVERY encode/engrave path REFUSES it (R6). Q4: `"txid_provisional": <bool>` on every
report JSON (option A).
Base: origin/main `88ac6d9` (rows 22 and 23 merged; FIX-C's sniff-by-decoding is in).
Branch `fix/mt-inspect-routing`. Every new or changed holder-visible text is marked
**PROPOSED (architect confirms)** and is listed for `design/WORDING_REVIEW.md`.
Re-folded 2026-11-23 by Claude Opus 5.5 (claude-opus-5-5): Q1/Q2 per the ruling, review
r1 I1 (line-start routing rule, control row 24) and M1 (guard order, row 26). Folded
2026-11-24 by Claude Sonnet 5.5 (claude-sonnet-5-5): Q3 and Q4 per Brian's rulings (no
STOP is open): R6, rows 28-33, spec delta sentence, gate counts. The texts new in this
fold are marked **PROPOSED (architect confirms)** and listed in section 11, O6.

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

Brian 2026-11-23 (#37 Q1/Q2, quoted from the coordinator memory):
- Q1: "REPORT it (inspect is the look-before-you-sign step). When any input is legacy
  (txid covers scriptSig, so it cannot exist until signed), the TX line is clearly
  labelled PROVISIONAL (no txid presented as final); segwit/taproot-only PSBTs show the
  real txid. Never refuse with the §8.1 "finalize first" text in inspect."
- Q2: "REPORT, with that input's amount and the fee UNVERIFIED (never TXID-BOUND), and a
  LOUD warning that funds will be lost to a huge fee if the real input is larger than the
  PSBT claims; give the holder instructions to verify the input before signing or
  broadcast. encode keeps refusing (T17 shape kept)." Instructions: "run with a node so
  mt checks it, or `bitcoin-cli gettxout <txid> <vout>` and compare the value."

Spec facts relied on: §1.1 "A supplied PSBT is compared against its
EXTRACTED transaction, per §10.13(c) … a PSBT holds two transactions whose txids differ
for every legacy and `sh(wsh(…))` input" (`SPEC_mt_v0_1.md:525-529`); §1.1 row table
`FEE` "a node is reachable, **or** the input was a PSBT carrying values" (`:687`).

- Q3 (2026-11-24): "If it's an invalid transaction there is no reason to encode it or
  engrave it, if we can display/decode it with a warning." Applied: `inspect` reports
  with a warning; every encode/engrave path refuses (R6).
- Q4 (2026-11-24): a `"txid_provisional": <bool>` key on every report JSON.

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
  `strings_subject(text)` (replaces `looks_like_an_mt1_set`; review r1 I1): let
  `opens(s)` be `s` begins `mt1`, `mtl` or `mti`. True iff `opens` holds for `text` with
  every whitespace character removed and ASCII-lowercased, **or** for some line of `text`
  (`str::lines`) with its whitespace removed and ASCII-lowercased. A line is the unit the
  strings reader splits on (`read_strings.rs:71-91`), so grouped (`mt 1p ge`),
  `mtl`/`mti` and elided-first-line sets are admitted, while text that merely contains
  `mt1` inside a line (a JSON-wrapped or damaged base64 PSBT, rows 22 and 24) keeps the
  sniffer's refusal. Consequences, all by existing texts:
  `mt encode`'s pasted-output refusal (`input.rs:171`) becomes unreachable from
  `inspect` (any input that starts `mt1` is a strings subject); a non-UTF-8 input that
  is not a PSBT gets sniff's step-4 refusal (`§8.2e, input is not a PSBT or a raw
  transaction (N bytes)`) instead of `§1.1e, input is not text` (row 14).
- **R3 a PSBT subject is its extracted transaction, with its records' values (I3), and
  is REPORTED whatever its finalization or records (Q1, Q2, Q3 ruled).**
  In `inspect_transaction`'s PSBT arm, after the unchanged parse and `psbt_end_guard`:
  1. Neither `finalized_guard_psbt` nor `non_witness_utxo_guard` is called (ruling: never
     refuse with §8.1's text in inspect; report a mismatched record). `encode` keeps
     calling both (`main.rs:442-443`) and `verify --transaction` keeps calling
     `finalized_guard_psbt` (`:1582`), unchanged.
  2. The reported transaction is `psbt.extract_tx_unchecked_fee_rate()`. For an input
     that is not finalized it carries an empty scriptSig and witness, so for such an
     input it is the unsigned template's.
  3. The values passed to `Report::build` are, per input, `validate::psbt_input_value`
     mapped `TxidBound -> Provenance::TxidBound`, `PsbtClaimed ->
     Provenance::PsbtClaimed`, exactly `encode`'s mapping (`main.rs:451-480` without
     the `--input-value` arm; `inspect` has no such flag). The builder moves the record
     part into one helper both verbs call, so the two cannot drift.
  4. **`psbt_input_value` checks the hash itself (Q2).** A `non_witness_utxo` whose
     `compute_txid()` is not the input's `previous_output.txid` still supplies the value
     of its output at the input's vout (the amount the PSBT claims), but labelled
     `PsbtClaimed`, never `TxidBound`; if it has no output at that vout the function
     falls through to `witness_utxo` as today. Only a record that hashes to the txid
     **and** has the output is `TxidBound`. `encode` is unaffected (its guard refuses a
     mismatched record before any value is read). The FEE then carries the weakest
     provenance as today, so it renders `(CLAIMED — no input value verified)` and JSON
     `"verified": false`.
  5. **Finalization and a PROVISIONAL txid (Q1).** An input is *finalized* by
     `finalized_guard_psbt`'s test (a non-empty `final_script_sig` or
     `final_script_witness`). An input is *native SegWit* when its spent scriptPubKey
     `is_witness_program()`, the scriptPubKey read from `witness_utxo`, else from the
     `non_witness_utxo`'s output at the vout **only if that record hashes to the input's
     txid**; an input with neither (or only a mismatched record) is not native SegWit.
     The txid is **PROVISIONAL** iff some input is neither finalized nor native SegWit
     (a legacy or P2SH-wrapped input, whose scriptSig signing fills, or one mt cannot
     classify). The ruling's reason ("txid covers scriptSig") is applied as written, so
     P2SH-wrapped SegWit counts as provisional. When provisional: the TX row is
     `TX        <txid>   (PROVISIONAL — changes when the PSBT is signed)`, JSON
     `"txid_provisional": true` (Q4), and the Q1 warning (section 4) is printed. When
     some input is not finalized but the txid is not provisional, the TX row is
     unchanged and the not-provisional Q1 warning is printed.
  6. **Warnings, in `encode`'s guard order (review r1 M1).** After the output-range
     warning (E4: stays element 0 and first on stderr) and before the no-node warning,
     in this order: the Q1 warning (the §8.1 condition, which `encode` checks first,
     `main.rs:442-443`), then, for each input in index order, its Q2 warning (hash
     mismatch) or its Q3 warning (matching record without the vout). The same order holds
     in JSON `warnings` and on stderr (row 26). `Report::build` and its node arm are
     unchanged: with a node, an input whose `gettxout` answers is shown `from node`, and
     the Q2 warning is still printed (the fault is in the PSBT, which a signer still
     reads).
  The raw-hex arm is unchanged (no values, `"txid_provisional": false`). The strings
  route is unchanged (`"txid_provisional": false`). `decode --json` (its document goes to
  stderr, as today) carries `"txid_provisional": false` too: every report JSON has the
  key, and no route other than R3.5's sets it true (Q4, rows 31-33).
- **R6 encode/engrave paths refuse a vout-less record (Q3 ruled).** Exit codes: `inspect`
  exits 0 with the Q3 warning (section 4); every encode/engrave path exits 1 with
  §8.2d's refusal. The mt CLI has four verbs (`main.rs:44-57`): `encode`, `decode`,
  `verify`, `inspect`. The encode/engrave paths are exactly these two, and both refuse:
  1. `mt encode` (default: `mt1` strings to engrave by hand or pipe into `me sysw pack`),
     with any of `--group-size`, `--separator`, `--input-value`, `--quiet`, `--json`
     (itself refused, `json_unsupported_guard`) and any input form (base64, binary,
     file, stdin).
  2. `mt encode --qr` (a `tx:` record for QR plates).
  Both call `validate::non_witness_utxo_guard` at `main.rs:443`, before the format
  split (`args.qr`, `main.rs:740`) and before any value is read (`require_psbt_input_values`
  `:444`), so no flag supplies a value that gets past it. **No code changes for R6**:
  `non_witness_utxo_guard` already refuses the matching-hash, vout-less record (E6, T16;
  `validate.rs:362-378`), with `mt encode: REFUSED — §8.2d, input <n> spends output
  <vout> of <want>, which has <k> output(s)`, the reason and remedy (`Re-export the PSBT
  from the wallet that built it.`) unchanged, exit 1, nothing on stdout. Rows 28 and 29
  pin it on both paths, and the guard keeps its existing `refusals.toml` entry
  (`a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound`). Not encode/engrave
  paths, so unchanged: `decode` (turns `mt1` strings into hex; it takes no PSBT),
  `verify` (compares a set against a supplied transaction, `--transaction` taking a PSBT
  through `finalized_guard_psbt` only, and writes no encoding), and `inspect`. The
  engraving tool `me` is another repo and consumes `mt encode`'s output, which a refused
  PSBT never produces.
- **R4 the no-node warning tells the truth about a known fee (I3).**
  `report::no_node_warning` gains a parameter `fee_known: bool`. When true, its two fee
  lines (`report.rs:684-691`) are replaced by the PROPOSED pair in section 4; when
  false they are unchanged. Callers: `inspect_transaction` passes `r.fee.is_some()`;
  `decode` (`main.rs:1479`) and the inspect strings route (`:1858`) pass `false` (a set
  carries no values). `has_legacy` for a PSBT subject is true iff some input is either
  finalized with an empty witness in the extracted transaction, or neither finalized nor
  native SegWit (R3.5's test); for a raw-hex subject it keeps today's expression. So the
  malleability caveat follows real witnesses (rows 8, 9) and an unsigned native-SegWit
  PSBT does not get it (row 25), while an unsigned legacy one does (row 10).
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
- **Q1, TX row when provisional (R3.5), PROPOSED (architect confirms).** Three spaces
  after the txid:

      "TX        <txid>   (PROVISIONAL — changes when the PSBT is signed)"

- **Q1 warning (R3.5), PROPOSED (architect confirms).** A `Warning` (stderr: `WARNING:
  <headline>`, blank line, body wrapped at 68 columns and indented two spaces, as every
  `Warning`; JSON `warnings`: the headline alone). `<b>` = number of inputs not
  finalized, `<m>` = number of inputs, `<list>` = their indexes joined `, ` (as §8.1's
  `(input 0, 1)`), `<plist>` = the indexes of the inputs that make the txid provisional,
  joined the same way. Provisional:

      headline: "this PSBT is not finalized (<b> of <m> inputs: input <list>), so TX is PROVISIONAL."
      body:     "These unsigned inputs are not native SegWit or Taproot, or carry no
                 UTXO record that shows they are: input <plist>. Signing puts their
                 signatures in their scriptSigs, which are part of the txid, so the
                 signed transaction will have a different txid from the one shown, and
                 no explorer or node knows the one shown. The outputs, fee, locktime and
                 inputs above do not change when it is signed. mt encode refuses this
                 PSBT until it is finalized (bitcoin-cli finalizepsbt)."

  Not provisional:

      headline: "this PSBT is not finalized (<b> of <m> inputs: input <list>)."
      body:     "Every unsigned input is native SegWit or Taproot, whose signatures are
                 not part of the txid, so TX is the txid the signed transaction will
                 have. mt encode refuses this PSBT until it is finalized (bitcoin-cli
                 finalizepsbt)."

  (The body lines above are one paragraph each; the wrap decides the breaks.)

- **Q2 warning (R3.4), PROPOSED (architect confirms).** Hand-formatted (like
  `no_node_warning`), not a wrapped `Warning`, so the command line is never split. One
  block per mismatched input, exactly these lines on stderr (`<n>` the input index,
  `<got>` the record's txid, `<want>` and `<vout>` the input's outpoint, `<amount>` the
  claimed value in the report's `X.XXXXXXXX BTC` form). The line `The PSBT says …` is
  omitted when R3.4 found no value for the input:

      WARNING: input <n>'s non_witness_utxo is NOT the transaction it spends,
               so its amount and the fee are UNVERIFIED.

        The PSBT says input <n> holds <amount>.
        Its record of the spent transaction hashes to
          <got>
        but input <n> spends output <vout> of
          <want>
        A buggy wallet writes this, and so does a HIDDEN-FEE ATTACK.

        IF INPUT <n> REALLY HOLDS MORE THAN THE PSBT SAYS, EVERY SATOSHI OF
        THE DIFFERENCE GOES TO A MINER AS FEE AND IS LOST. A signer that
        trusts this record shows you the wrong fee.

        Check input <n> before you sign or broadcast, either way:
          - let mt reach a bitcoind: it asks for the value with gettxout
            and shows it in INPUTS as `from node`, or
          - on a machine with a node, run
              bitcoin-cli gettxout <want> <vout>
            and compare its "value" with the amount your signer shows.

        mt encode refuses this PSBT: re-export it from the wallet that built it.

  JSON `warnings` entry (one string):
  `input <n>'s non_witness_utxo is NOT the transaction it spends, so its amount and the
  fee are UNVERIFIED. Check it before you sign or broadcast: bitcoin-cli gettxout <want>
  <vout>` (one space between each word; row 11 pins it exactly).

- **Q3 warning (R3.6; ruled, option B), PROPOSED (architect confirms).** A
  `Warning`:

      headline: "input <n> spends output <vout> of <want>, which has <k> output(s): that output does not exist."
      body:     "The PSBT's non_witness_utxo for input <n> is the transaction it spends
                 (its hash matches), and that transaction has no output <vout>, so this
                 transaction can never be valid. mt encode refuses this PSBT: re-export
                 it from the wallet that built it."

  JSON `warnings` entry: the headline alone (row 30 pins it exactly). `inspect` exits 0.
  The refusal at `encode`, `--qr` included, is §8.2d's existing text (R6); nothing new.

- **Q4 JSON key (ruled, option A).** Every report JSON (`inspect` both
  routes, `decode`) gains `"txid_provisional": <bool>` as the line after `"txid"`; true
  only under R3.5. The `decode --json` document (on stderr), `inspect --json` for a
  raw transaction, for an `mt1` set and for a PSBT all carry it (rows 1, 10, 25, 31-33).

- **Holder-visible change, all four:** `inspect` reports these PSBTs at exit 0 (base:
  exit 0 too, but with the template txid unlabelled, no values and no warning).
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
     subject; any other text whose first characters, or the first characters of one of
     its lines, are `mt1`, `mtl` or `mti` (whitespace removed, any case) goes to §1.1e's
     string reader, so grouped, elided and misread-separator sets read as they do in
     `verify`. `inspect` is the look-before-you-sign step, so it reports a PSBT that is
     not finalized, or whose `non_witness_utxo` fails §8.2d, where `encode` refuses it:
     an unfinalized PSBT's txid is PROVISIONAL when signing changes it (an input that is
     not native SegWit); a `non_witness_utxo` that does not hash to the input's txid
     leaves that input's value and the fee UNVERIFIED, with a warning naming the
     `bitcoin-cli gettxout` check. ``
     (Follows the Q1/Q2 ruling; the Q3 clause, "fails §8.2d" covering a matching record
     without the vout, follows the Q3 ruling.) Append to that paragraph, PROPOSED: `` The
     same holds for a `non_witness_utxo` that hashes to the input's txid but has no
     output at its vout: `inspect` reports it with a warning that the output does not
     exist, and `encode`, with or without `--qr`, refuses it (§8.2d). ``
- **README:** none (it does not describe inspect's routing).
- **Toolkit manual:** none; `mnemonic-toolkit` `docs/manual/src/40-cli-reference/` has
  no `mt` page (41-mnemonic, 42-md, 43-ms, 44-mk-cli, 45-mr, 46-mr-sitting).
- **CHANGELOG:** none in this FIX (entries are written at release).

## 6. RED rows

All 33 rows are one new test binary `crates/mt-cli/tests/bh1119_mt_inspect_routing.rs`
(`mt-cli::bh1119_mt_inspect_routing`), exact text in the sidecar
`design/FIX_bh1119_mt_inspect_routing/inspect_routing.rs.txt` (sha256 `0d1ac869906bc13ab978d5921bec12f9b679d3040c756313263655bd8dbaf1bd`;
rustfmt edition 2024 clean, `cargo clippy -p mt-cli --test bh1119_mt_inspect_routing -- -D
warnings` clean). Command:

    cargo nextest run --locked --build-jobs 2 -j 2 -p mt-cli --test bh1119_mt_inspect_routing

Expected on base `88ac6d9`: `33 tests run: 9 passed, 24 failed` (section 6a gives the
measured result). Each row, expected base behaviour -> after:

| # | row (`bh1119_mt_` omitted) | base | after |
|---|---|---|---|
| 1 | `i3_inspect_reports_the_extracted_txid_of_a_legacy_psbt` | FAIL: JSON txid is the template's | txid = extracted; `"txid_provisional": false` (Q4) |
| 2 | `i3_inspect_json_carries_a_txid_bound_value_and_fee` | FAIL: `"fee": null` | fee 100000 verified, input TxidBound 1000000000 |
| 3 | `i3_inspect_json_labels_a_witness_utxo_value_psbt_claimed` | FAIL: `"fee": null` | fee 100000 unverified, PsbtClaimed |
| 4 | `i3_inspect_and_encode_agree_on_a_legacy_psbt` | FAIL: TX rows differ | TX, FEE, INPUTS rows equal |
| 5 | `i3_inspect_reports_p5_input_values_offline` | FAIL: `FEE       UNKNOWN` | `FEE       0.00100000 BTC`, two TXID-BOUND rows |
| 6 | `i3_no_node_warning_follows_a_fee_known_from_the_psbt` | FAIL: UNKNOWN fee lines | PROPOSED fee lines |
| 7 | `i3_no_node_warning_keeps_unknown_fee_for_raw_hex` | pass (control) | pass |
| 8 | `i3_segwit_psbt_omits_the_legacy_caveat` | FAIL: caveat printed | absent |
| 9 | `i3_legacy_psbt_keeps_the_legacy_caveat` | pass (control) | pass |
| 10 | `i3_inspect_reports_an_unfinalized_legacy_psbt_with_a_provisional_txid` (Q1) | FAIL: TX row unlabelled | exit 0; `TX        <template txid>   (PROVISIONAL — changes when the PSBT is signed)`; stderr `WARNING: this PSBT is not finalized (1 of 1 inputs: input 0), so TX is PROVISIONAL.`; caveat present; JSON txid = template's, `"txid_provisional": true`, warnings hold the headline |
| 11 | `i3_inspect_reports_a_non_witness_utxo_that_is_not_the_parent_as_unverified` (Q2) | FAIL: `FEE       UNKNOWN` | exit 0; `FEE       0.00001000 BTC   (CLAIMED — no input value verified)`; input row `9b7c8a80ddc880a9…   9.99901000 BTC   PSBT-CLAIMED — unverified`; no `TXID-BOUND`; stderr holds the section 4 Q2 block exactly; JSON `PsbtClaimed`, 999901000 sats, fee unverified, the Q2 JSON entry |
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
| 24 | `i6_wrapped_base64_containing_mt1_keeps_the_sniffer_refusal` (r1 I1) | pass (control) | pass: first line starts `mt inspect: REFUSED — §8.2e, input is not a PSBT or a raw transaction (` |
| 25 | `i3_inspect_reports_an_unfinalized_segwit_psbt_with_its_real_txid` (Q1) | FAIL: no Q1 warning | exit 0; `TX        4665e0aa…` (p5's, unlabelled); stderr `WARNING: this PSBT is not finalized (2 of 2 inputs: input 0, 1).`; no `PROVISIONAL`, no caveat; JSON `"txid_provisional": false` |
| 26 | `i3_inspect_warns_in_encodes_guard_order` (r1 M1) | FAIL: `no Q1 warning` | exit 0; the Q1 warning precedes the Q2 warning on stderr and in JSON `warnings` |
| 27 | `i3_inspect_warns_on_a_non_witness_utxo_without_the_spent_output` (Q3 B) | FAIL: warning absent | exit 0; stderr `WARNING: input 0 spends output 99 of <parent txid>, which has <k> output(s): that output does not exist.` |
| 28 | `i3_encode_refuses_a_non_witness_utxo_without_the_spent_output` (Q3, R6) | pass (control: encode already refuses) | pass: exit 1, stdout empty, first stderr line `mt encode: REFUSED — §8.2d, input 0 spends output 99 of <parent txid>, which has <k> output(s)` |
| 29 | `i3_encode_qr_refuses_a_non_witness_utxo_without_the_spent_output` (Q3, R6) | pass (control) | pass: the same exit and first line with `--qr` |
| 30 | `i3_inspect_json_carries_the_q3_warning_headline` (Q3) | FAIL: no such `warnings` entry | exit 0; JSON `warnings` holds exactly `input 0 spends output 99 of <parent txid>, which has <k> output(s): that output does not exist.`; `txid` = the extracted txid |
| 31 | `q4_inspect_json_of_a_raw_transaction_carries_txid_provisional_false` (Q4) | FAIL: key absent (`Null`) | `"txid_provisional": false` |
| 32 | `q4_inspect_json_of_an_mt1_set_carries_txid_provisional_false` (Q4) | FAIL: key absent | `"txid_provisional": false` |
| 33 | `q4_decode_json_carries_txid_provisional_false` (Q4) | FAIL: key absent (document on stderr) | `"txid_provisional": false` |

The base first lines quoted for rows 12-23 were measured on the base-equivalent binary
(section 1). Row fixtures: row 10 the legacy PSBT unfinalized with its parent as record;
row 11 the finalized legacy PSBT whose record is the parent with the spent output lowered
to 999,901,000 sats (the hidden-fee shape: the PSBT shows a 1,000-sat fee, the real one
is 100,000); row 25 p5's `unfinalized_psbt_b64`; row 26 rows 10 and 11 combined; rows 27-30
the finalized legacy PSBT spending output 99 of the parent, with the parent as record; rows 31-33 the corpus raw-hex / `even` set.

### 6a. Measured

Re-fold, run on base `88ac6d9` with both sidecars installed (the command above, cargo
1.85.0): `Summary 27 tests run: 7 passed, 20 failed, 0 skipped`; the Q3/Q4 fold (Sonnet
5.5, same base code, same cargo) added rows 28-33 and measured `Summary 33 tests run: 9
passed, 24 failed, 0 skipped`. Passes are exactly the seven controls (rows 7, 9, 15,
21, 22, 23, 24) and the two encode controls (rows 28, 29: `encode` already refuses).
Rows 30-33 fail on the absent Q3 JSON entry and the absent `txid_provisional` key
(`left: Null`, `right: false`).
The re-fold's new-row failures are the defect: row 10
TX row without the label; row 11 `FEE       UNKNOWN …`; row 25 its TX row matches p5's
txid and it fails at the absent not-finalized warning; row 26 `no Q1 warning`; row 27
its warning absent. From the first fold (author's measurement, same again here): row 1 JSON txid
`015cfdd3…` vs extracted `f53f4701…` (the consultant's pair); rows 2, 3 `"fee": null`;
row 4 `TX` rows differ (same pair); row 5 `FEE       UNKNOWN   (needs input values, …`;
rows 12, 13 `§1.1e, input is not text`; row 14 the same first line;
rows 16, 17 the pasted-output refusal; rows 18, 19, 20 step-4 at 527, 516, 87 bytes;
rows 6, 8 the UNKNOWN fee lines and the caveat present.

## 7. Tests that change

- `crates/mt-cli/src/main.rs:2492-2522` unit test
  `an_mt1_set_is_recognised_by_how_it_starts_not_by_a_substring` tests the removed
  `looks_like_an_mt1_set`. Replaced by `strings_subject_admits_recoverable_sets_only`
  over `strings_subject`, asserting: true for `mt1qqqqq`, `  \n mt1qqqqq`, `MT1QQQQQ`,
  `mt 1q qq` (grouped), `qqqq\nmt1qq` (elided first line, via its second line),
  `mtlqqqq`, `mtiqqqq`; false for `cHNidP8BAHsCAAAAmt1AAAAA`, `{\n "psbt":
  "cHNidP8BAHUCAAAAmt1AAAAA"\n}` and `HNidP8BAHUCAAAAmt1AAAAA` (review r1 I1: `mt1`
  inside a line), `hello`, `abababa`, `` (empty).
- `crates/mt-cli/src/report.rs:814` unit test `no_node_warning_names_both_ways_out`:
  the call gains the `fee_known` argument `false`; assertions unchanged.
- `crates/mt-cli/src/report.rs:737`, `:765`, `:828`: the three unit tests that build a
  `Report` literal gain `txid_provisional: false` (Q4: the builder adds `pub
  txid_provisional: bool` to `Report`; `Report::build` sets false; `inspect_transaction`
  sets it under R3.5; `render` appends the PROVISIONAL label and `render_json` writes the
  key from it). Assertions unchanged.
- `validate.rs` unit tests, if any assert `psbt_input_value` on a mismatched record: none
  at `88ac6d9` (`git grep psbt_input_value` finds only `main.rs:459, 487` and
  `validate.rs`); R3.4 changes only the label of a record `encode` has already refused.
- No integration test changes expected: every existing `inspect` test reads an `mt1`
  set that starts `mt1` (unchanged route) or a raw-hex / base64 input sniff already
  decides. `inspect.rs:732` (`cHNidP8BAHUCAAAA`) stays on the PSBT refusal (R2 step 2
  then 4); `bh1119_mt_input_parsing.rs:474-480` compares two PSBT reports, both now with
  values. The builder confirms by the full suite (gate).
- Added: the row binary and six `tests/refusals.toml` entries (sidecar
  `refusals-toml-append.txt`): rows 14, 21 and 24 (`recognised_guard`, which `inspect`
  now reaches for non-UTF-8 input and holds against the new `strings_subject` route) and
  row 22 (`base64_psbt_guard`, likewise). Rows 10 and 11 are no longer refusals (ruling),
  so the first fold's §8.1 and §8.2d entries are dropped. Rows 28 and 29 (Q3: `encode`
  and `encode --qr` refuse a vout-less record) are declared on `non_witness_utxo_guard`.
  Coverage and mutation go
  61 -> 67 (coverage measured with the sidecars installed: `check-refusal-coverage: 67
  refusal tests over 18 ruled refusals`).
  Rows 15 and 23 are controls of inline refusals with no guard fn (`read_input`'s UTF-8
  step, `read_strings::read`'s empty check), so they have no entry and no mutation
  control; their refusal text is composed by the row helper `refused(r, verb, rest)`,
  as every row's is. Flagged for the reviewer: the coverage script's "asserts on a
  §-refusal message" heuristic sees only literal text in a test body, so the helper
  also keeps it from seeing rows 15 and 23; the four rows that have a guard are
  declared.

## 8. Build order

1. Install both sidecars; run the row command; see `33 tests run: 9 passed, 24 failed`.
2. R1 and R2 (byte reader, `strings_subject`, routing): rows 12-14, 16-20 GREEN; 15,
   21-24 stay GREEN.
3. R3.2-R3.4 (extraction, values, the hash check in `psbt_input_value`): rows 2-5 GREEN.
4. R3.5, R3.6 and Q4 (`txid_provisional`, the PROVISIONAL label, the Q1/Q2/Q3 warnings
   in order): rows 1, 10, 11, 25, 26, 27, 30, 31, 32, 33 GREEN; 28, 29 stay GREEN (R6
   changes no code).
5. R4 (`fee_known`, `has_legacy` for a PSBT subject): rows 6, 8 GREEN; 7, 9 stay GREEN.
6. R5 comments; the unit tests of section 7; spec re-sync and PROVENANCE row once
   mnemonic-engrave carries the section 5 delta.
7. Gate (section 10) once to a file.

## 9. Coverage map (rule -> rows)

| rule | rows |
|---|---|
| R1 inspect reads bytes | 12, 13, 14; control 15 |
| R2 routing (sniff first, `strings_subject` line-start rule, sniff refusal otherwise) | 16, 17, 18, 19, 20; controls 21, 22, 23, 24; unit test (section 7) |
| R3.1 no PSBT guard refuses at inspect (Q1/Q2/Q3 ruling) | 10, 11, 25, 26, 27 (each exits 0) |
| R3.2, R3.3 extracted transaction, record values, provenance | 1, 2, 3, 4, 5 |
| R3.4 a mismatched record is PSBT-CLAIMED, fee unverified, never TXID-BOUND (Q2) | 11 |
| R3.5 PROVISIONAL txid and the Q1 warnings; Q4 key | 10 (provisional), 25 (not provisional), 1, 31, 32, 33 (key false: PSBT, raw hex, `mt1` set, `decode --json`) |
| R3.6 warning order (r1 M1); Q2 block; Q3 warning (stderr, JSON) | 26; 11; 27, 30 |
| R6 every encode/engrave path (`encode`, `encode --qr`) refuses a vout-less record (Q3) | 28, 29 |
| R4 no-node fee lines; caveat on real witnesses and unsigned inputs | 6, 8, 25 (absent), 10 (present); controls 7, 9 |

## 10. Gate (builder, final head; run once to a file, then grep)

    bash design/FIX_bh1119_mt_inspect_routing/gate.sh > /scratch/cache/mt-inspect-routing-gate.log 2>&1

Measured RED (re-fold, base `88ac6d9`, sidecars installed, no implementation): fmt,
clippy and build pass, then `Summary [  22.249s] 486 tests run: 466 passed, 20 failed, 0
skipped` and `GATE FAIL: nextest` (exit 1). So the gate can fail, and its pins are
459 + 33 = 492 (Q3/Q4 fold; the 486 figure above is the 27-row re-fold's measurement).
The Q3/Q4 fold's measurement is in section 6a and the 2026-11-24 gate run below.

The script runs every step of the CI `test` job once, checks the sidecars are installed
unchanged, and pins the counts. WHAT IT DOES NOT COVER is in its header. Its totals are
`88ac6d9` + this FIX only (459 + 33 = 492 tests; refusal coverage and mutation 67); a head that also
carries rows 35, 38, 39 or the open `fix/mt-*` neighbours must add their counts.

## 11. Rulings applied and decisions (no STOP is open)

- **Q1 (ruled 2026-11-23).** Report; PROVISIONAL txid when signing changes it (R3.5,
  rows 10, 25, 26). Applied reading, flag if wrong: the ruling's reason "txid covers
  scriptSig" makes a P2SH-wrapped SegWit input, and an input with no record that shows
  its type, provisional too, not only a "legacy" one; and the not-provisional case
  still prints a short not-finalized warning (section 4), since `encode` will refuse the
  same PSBT.
- **Q2 (ruled 2026-11-23).** Report; amount and fee UNVERIFIED; loud fee-loss warning;
  `gettxout` instructions (R3.4, R3.6, row 11). Applied reading, flag if wrong: the
  amount shown is the mismatched record's own output at the vout (the amount the PSBT
  claims; falling through to `witness_utxo` only when the record has no such output);
  the block prints whether or not a node answered for that input (a signer still reads
  the record).
- **Q3 (ruled 2026-11-24, option B).** A `non_witness_utxo` that hashes to the input's
  txid but has no output at its vout (§8.2d's second refusal): `inspect` REPORTS it with
  the Q3 warning (R3.6, section 4, row 27, JSON row 30); EVERY encode/engrave path
  REFUSES it (R6: `encode` and `encode --qr`, rows 28, 29). Brian: "If it's an invalid
  transaction there is no reason to encode it or engrave it, if we can display/decode it
  with a warning." No value is read from such an input as TXID-BOUND (it falls through to
  `witness_utxo`, PSBT-CLAIMED, or UNKNOWN). Applied reading, flag if wrong: `verify
  --transaction` and `decode` are not encode/engrave paths (R6), so they are unchanged.
  The D6 #37 Opus r2 and the architect check R6 against §8.2d.
- **Q4 (ruled 2026-11-24, option A).** `"txid_provisional": <bool>` on every report JSON,
  the line after `"txid"` (section 4; rows 1, 10, 25, 31, 32, 33). No existing test
  asserts the JSON key set (`git grep` of `crates/mt-cli/tests` at `88ac6d9`).
- **O1 (wording, PROPOSED)** every text in section 4: the no-node fee pair, the
  PROVISIONAL label, the Q1, Q2 and Q3 warnings and the Q2 JSON entry. Listed for
  `design/WORDING_REVIEW.md`.
- **O6 (decided in the 2026-11-24 fold, flag if wrong)** the texts the Q3/Q4 fold
  chose: none new for holders in `mt` itself (the Q3 warning, its JSON headline and
  §8.2d's refusal text are the earlier PROPOSED/existing ones, unchanged); the spec-delta
  sentence appended to the §1.1 paragraph (section 5); and the R6 scope list: `encode`
  and `encode --qr` are the only encode/engrave paths, `decode` and `verify` are not.
- **O2 (folded from review r1 I1)** the `strings_subject` criterion is the line-start
  rule (R2), with control row 24.
- **O3 (decided here, flag if wrong)** non-UTF-8 non-PSBT input to `inspect` gets sniff's
  step-4 refusal rather than `input is not text` (row 14): `inspect` reads binary PSBTs,
  so "not text" no longer names the problem; the step-4 text names all accepted forms
  except `mt1` strings, which binary input cannot be.
- **O4 (folded from review r1 M1)** no guard refuses at `inspect` any more, so the
  r1 pair (unfinalized and mismatched) is reported with both warnings, in `encode`'s
  guard order (R3.6, row 26).
- **O5 (decided here, flag if wrong)** with a PROVISIONAL txid, `Report::build` and the
  no-node warning are unchanged: a node is still asked about the provisional txid (it
  will not know it), and the no-node block still offers the explorer lookup; the Q1
  warning, printed before both, says no explorer or node knows that txid.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

Closes bh-mt-I3, bh-mt-I5, bh-mt-I6 (none has a line in `design/FOLLOWUPS.md` at
`88ac6d9`). Files none.

## 13. Files this FIX touches (for build ordering against rows 35, 38, 39)

- `crates/mt-cli/src/main.rs` (`read_input` split, `inspect`, `inspect_transaction`,
  `strings_subject`, the shared PSBT-values helper used by `encode` too, comments,
  unit test)
- `crates/mt-cli/src/report.rs` (`no_node_warning` signature and fee lines; `Report`
  gains `txid_provisional`, `render`'s TX row and `render_json`'s key; unit tests)
- `crates/mt-cli/src/validate.rs` (`psbt_input_value` checks the record's hash, R3.4;
  the finalization and native-SegWit classification of R3.5 and the Q2/Q3 findings may
  live here beside the guards they mirror; the guards' texts are unchanged)
- `crates/mt-cli/tests/bh1119_mt_inspect_routing.rs` (new)
- `crates/mt-cli/tests/refusals.toml` (append)
- `design/SPEC_mt_v0_1.md`, `design/PROVENANCE.md` (re-sync, step 5)

`input.rs` is read, not changed. Row 38 (FIX-G, verdict/notice
integrity) changes notice gating in `main.rs` near the same report code; row 39 (FIX-J,
printed remedies) may change `finalized_guard_psbt`'s remedy (inspect no longer calls
it, so no row here depends on that text). Text conflicts only expected.
