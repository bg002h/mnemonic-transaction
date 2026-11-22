# FIX: input parsing (bughunt-1119 FIX table row 22, "FIX-C input parsing")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Keys: bh-mt-I1 (I), bh-mt-I2 (I),
bh-mt-I20 (M). Ruling: architect T23 (`coordinator/agent-reports/architect-bughunt-1119-questions.md:61`),
binding. Track: small (one Opus review round). Not a funds-path (class B) row.
Base: origin/main `a885e62`. Branch `fix/mt-input-parsing`. Every new or changed
holder-visible text is marked **PROPOSED (architect confirms)**; everything else below is
decided. No new flag, type visible to a holder, or command.

Not to be confused with the older `design/FIX_bughunt_mt_c.md` (a different hunt).

## 1. Findings (at a885e62, each run by the author)

Run offline (`--bitcoin-cli /nonexistent`), umask 077, on the consultant's fixtures
(`p5_base.json` `raw_hex` = 339 bytes, `finalized_psbt_b64` = 927 bytes; corpus `even` =
222 bytes):

- **I1, creation.** `cat raw.hex even.hex | mt encode --qr` exits 0 and prints the
  682-byte record of `raw.hex` alone; reversed, it prints the 448-byte record of
  `even` alone. `decode_tx` (`crates/mt-cli/src/main.rs:2144-2158`) and `txid_display`
  (`main.rs:972-994`) read one transaction from a cursor with
  `Transaction::consensus_decode` and never check that the cursor is empty.
- **I1, recovery.** `mt_codec::string_layer::pipeline::encode(even ‖ 0xab, even's txid)`
  gives a set that `mt decode` prints (with the `ab` tail) at exit 0, and that `mt verify
  --transaction even.hex` reports OK with a full-txid match. `content_id_guard`
  (`main.rs:1942-1951`) reads the txid through `txid_display`, which ignores the tail.
  `mt encode` engraves `serialize(&tx)` (`main.rs:615`), so it never writes such a set.
- **I2, PSBT framing.** `cat final.psbt final.psbt | mt encode --qr` and the same with
  `final.b64` twice exit 0 with the one-PSBT record. bitcoin 0.32.102's
  `Psbt::deserialize` is `deserialize_from_reader` with no end check
  (`psbt/serialize.rs:79-127`); mt calls it at `main.rs:428` (encode), `:1470` (verify
  `--transaction`) and `:1563` (inspect).
- **I2, base64.** `base64_decode` (`crates/mt-cli/src/input.rs:254-272`) stops at the
  first `=`: `final.b64` + `=oops` exits 0 with the same record. It also drops a dangling
  last character and ignores the spare bits of the last character.
- **I20.** `sniff` (`input.rs:31-33, 52-65`) recognises base64 by the prefix `cHNidP8`.
  The 7th character carries the top 2 bits of the byte after the magic (the first key's
  compact-size length). A PSBT whose first global key is 64 bytes long (`b[:5] + [0x40,
  0xfa] + 63 zero bytes + [0x01, 0x2a] + b[5:]`) encodes at exit 0 in binary (796-byte
  output); its base64 begins `cHNidP9` and is refused: `mt encode: REFUSED — §8.2e, input
  is not a PSBT or a raw transaction (1328 bytes)`. The argv bearer classifier
  (`crates/mt-cli/src/validate.rs:776`) uses the same 7-character test, so `mt encode
  <that base64>` is not caught by §8.2f and clap echoes the whole PSBT to stderr
  (`error: invalid value 'cHNidP9A+gAAAA…`), measured by row 24 (section 6).

## 2. Ruling (binding, quoted)

- **T23 mt-Q16 (I20): sniff by decoding** (base64 then the `psbt\xff` magic, or raw
  magic), not by a 7-character prefix. Unblocks FIX-C.

I1 and I2 need no ruling: the grade (wrong result at exit 0) fixes the direction
(refuse), and only refusals are added.

## 3. Behaviour after the fix (rules)

Refusals use the existing `Refusal` type (`refusal.rs`; first line `mt <verb>: REFUSED —
<section>, <verdict>`, then mechanism and remedy wrapped at 68 columns), exit 1, and
nothing on stdout. Texts are in section 4.

- **R1 One input is one PSBT (I2).** Every PSBT parse (`main.rs:428`, `:1470`, `:1563`)
  parses from a cursor with `bitcoin::Psbt::deserialize_from_reader` and records how
  many bytes it used. A parse error keeps today's refusal at that site, unchanged. On
  success, a new guard `pub fn psbt_end_guard(total: usize, used: usize, verb: &str) ->
  Result<(), Refusal>` in `input.rs` refuses with **E-PSBT** when `used != total`;
  called right after the parse, before any other PSBT check (`finalized_guard_psbt`
  and the rest run only on an exact PSBT). Verb: `encode`, `verify`, `inspect` as the
  site. `total` is the length of the bytes `sniff` returned (binary: after outer
  whitespace trim, as today; base64: the decoded bytes).
- **R2 One input is one transaction (I1, holder input).** `decode_tx` and
  `txid_display` parse with `bitcoin::consensus::encode::deserialize_partial` (returns
  the transaction and the bytes used); a parse error keeps today's refusal text at
  each, unchanged. Both gain a third parameter `subject: Subject`, a private
  `enum Subject { Input, Set }` in `main.rs`. For `Subject::Input` a new guard `pub fn
  tx_end_guard(total: usize, used: usize, verb: &str) -> Result<(), Refusal>` in
  `input.rs` refuses with **E-TX** when `used != total`. Input call sites:
  `decode_tx(&b, "encode", Input)` (`main.rs:523`), `decode_tx(&bytes, "inspect",
  Input)` (`:1579`), `txid_display(&supplied, "verify", Input)` (`:1502`).
- **R3 A set's bytes are one transaction (I1, recovery).** For `Subject::Set` a new
  guard `fn set_end_guard(total: usize, used: usize, verb: &str) -> Result<(), Refusal>`
  in `main.rs` refuses with **E-SET** when `used != total`. Set call sites:
  `txid_display(bytes, verb, Set)` in `content_id_guard` (`main.rs:1948`, reached first
  by `decode`, `verify` and `inspect` on mt1 strings), `txid_display(bytes, "verify",
  Set)` (`:1501`), `decode_tx(&set.bytes, "decode", Set)` (`:1307`),
  `decode_tx(&set.bytes, "inspect", Set)` (`:1688`). The last three are reached only
  after `content_id_guard` has passed, so in practice E-SET is printed from
  `content_id_guard`; `verify` therefore refuses before its `mt verify: OK` line.
- **R4 Base64 is decoded whole (I2).** `base64_decode` returns `Some(bytes)` iff the
  text (interior whitespace already removed by step 3) is exactly the RFC 4648 §4
  encoding of `bytes`: alphabet `A-Z a-z 0-9 + /`; when the text holds any `=`, it is
  the padded encoding (1 or 2 `=` at the end only, total length a multiple of 4);
  when it holds none, it is the unpadded encoding (length mod 4 is 0, 2 or 3). The
  spare low bits of the last data character must be zero. Anything else is `None`.
  Decided: unpadded base64 stays accepted (unchanged today; it decodes to one byte
  string and no text is dropped).
- **R5 Base64 is recognised by decoding (I20, T23).** `sniff` step 3a becomes, in
  order: (a) if `base64_decode(text)` is `Some(b)` and `b` begins `psbt\xff`, the
  input is `Input::Psbt(b)`; (a′) otherwise a new guard `pub fn base64_psbt_guard(text:
  &str) -> Result<(), Refusal>` in `input.rs` refuses with **E-B64** iff the text
  begins `cHNidP`. Steps 3b (hex), 3c (`mt1`), 3d (`tx:`) and 4 follow unchanged.
  `cHNidP` is the 36 bits of `psbt\xff` that do not depend on the next byte; it
  replaces the constant `PSBT_BASE64_PREFIX = "cHNidP8"` (`input.rs:33`) as
  `pub(crate) const PSBT_BASE64_STEM: &str = "cHNidP"`. Step 2 (binary magic before any
  whitespace is touched) is unchanged. No hex text can match (a): `H`, `N` and `P` are
  not hex digits. Today's inline "does not decode" refusal (`input.rs:54-64`) is
  replaced by E-B64 (same verdict, new mechanism).
- **R6 The argv classifier follows (I20).** `looks_like_a_transaction`
  (`validate.rs:776`) tests `a.starts_with(crate::input::PSBT_BASE64_STEM)` instead of
  `"cHNidP8"`, so every valid base64 PSBT spelling (and a damaged one) on argv is
  refused by §8.2f (`command_line_guard`) and admitted by `argv_intake` under
  `--allow-argv-secret`, never echoed by clap. The §8.2f refusal text is unchanged.
- **Unchanged.** Hex sniffing (`input.rs:68-80`) and `hex_psbt_guard`; outer whitespace
  trim; the line-wrapped base64 path; every existing parse-error text; `verify`'s
  full-txid comparison of two well-formed transactions (`main.rs:1501-1514`).

## 4. Holder-visible texts (all PROPOSED, architect confirms)

Each block is the exact Rust string content: `verdict` is the first line's tail,
`mechanism` and `remedy` are wrapped by `Refusal`'s Display. `{n}` = `total - used`,
`{used}` = bytes used. `{n bytes}` / `{used bytes}` render as `1 byte` when the count
is 1, otherwise `<count> bytes` (decided; one helper).

**E-TX** — section `§8.2e`; verbs `encode`, `inspect`, `verify` (on the
`--transaction` file):

```text
verdict:   the input continues for {n bytes} after the end of the transaction
mechanism: The hex parses as one complete transaction ({used bytes}) and does not stop there. mt reads exactly one transaction per input. It will not use the first one and drop the rest, because the rest may be the transaction you meant.
remedy:    Pass a file that holds exactly one transaction.
```

**E-PSBT** — section `§8.2e`; verbs `encode`, `inspect`, `verify` (on the
`--transaction` file):

```text
verdict:   the input continues for {n bytes} after the end of the PSBT
mechanism: The input parses as one complete PSBT ({used bytes}) and does not stop there. mt reads exactly one PSBT per input. It will not use the first one and drop the rest, because the rest may be the PSBT you meant.
remedy:    Pass a file that holds exactly one PSBT.
```

**E-SET** — section `§1.1`; verbs `decode`, `verify`, `inspect` on mt1 strings:

```text
verdict:   the reassembled bytes continue for {n bytes} after the end of the transaction
mechanism: Every checksum held and the strings reassemble, but the bytes are one complete transaction ({used bytes}) followed by {n} more. mt encode never writes bytes after the transaction, so these strings were not made by mt encode from one transaction. mt will not print or vouch for a transaction while bytes it cannot account for ride along.
remedy:    Find out which tool made these strings before trusting anything recovered from them.
```

**E-B64** — section `§8.2e`; verb `encode` (re-labelled by the caller as today:
`inspect`, `verify`). The verdict and remedy are today's (`input.rs:58, 64`); the
mechanism is new:

```text
verdict:   input looks like a base64 PSBT but does not decode
mechanism: The text begins `cHNidP`, as every base64-encoded PSBT does, but it is not one clean base64 text that decodes to the `psbt\xff` magic: it holds a character outside the base64 alphabet, a `=` before its end, text after its padding, or a last character whose spare bits are not zero. It is probably truncated, joined to something else, or has had characters altered in transit.
remedy:    Re-export the PSBT and pass the file with --in.
```

**Step-4 refusal** (`unrecognised`, `input.rs:151-172`): one word in the mechanism,
`a base64 PSBT (beginning `cHNidP8`)` becomes `a base64 PSBT (beginning `cHNidP`)`.
Verdict and remedy unchanged.

Help text: none changes (no `--help` string names `cHNidP8`; `git grep` at a885e62
finds it only in `input.rs`, `validate.rs`, two comments in `main.rs:1541, 1674`
(update them to `cHNidP`), a `main.rs:2363` unit-test literal (unchanged, still not an
mt1 set) and test fixtures).

## 5. Spec, README, help, manual, CHANGELOG

- **Spec delta (T23).** `design/SPEC_mt_v0_1.md` is a copy owned by mnemonic-engrave
  (CLAUDE.md). Applied to mnemonic-engrave's `design/SPEC_mt_v0_1.md` first (anchors at
  me `73db74e`: :2373, :2386, :2390-2391), then this FIX's build re-syncs mt's copy and
  updates the `design/SPEC_mt_v0_1.md` source-commit row of `design/PROVENANCE.md`
  (:8), which `scripts/check-provenance.sh` reads. Note: mt's copy already differs from
  me `73db74e` (first difference at line 224, byte 12748); the re-sync takes the whole
  file, so whoever builds first re-syncs, and the other rows re-check. PROPOSED text:
  1. §8.2e table, row `base64 PSBT`, column "recognised by": `the `cHNidP8` prefix`
     becomes `decoding: it decodes to bytes beginning with the magic (the text begins
     `cHNidP`)`.
  2. Step 3a: `a. base64 PSBT -- begins cHNidP8` becomes three lines:
     `a. base64 PSBT -- the whole text is canonical base64 (RFC 4648; padding
     optional, `=` only at the end, spare bits zero) and decodes to bytes beginning
     70 73 62 74 ff. Text beginning cHNidP that fails this is refused as a damaged
     base64 PSBT.`
  3. After step 4, a new step: `5. Whatever matched, the input is exactly ONE PSBT or
     ONE transaction: bytes left after its end are refused, naming their count, never
     dropped.`
- **README:** none (no `cHNidP` text; `README.md` describes forms, not prefixes).
- **Toolkit manual:** none. `mnemonic-toolkit` master `docs/manual/src/40-cli-reference/`
  has no `mt` page (41-mnemonic, 42-md, 43-ms, 44-mk-cli, 45-mr, 46-mr-sitting).
- **CHANGELOG:** none in this FIX (entries are written at release).

## 6. RED rows

All 26 rows are one new test binary `crates/mt-cli/tests/bh1119_mt_input_parsing.rs`
(`mt-cli::bh1119_mt_input_parsing`). Its exact text is the sidecar
`design/FIX_bh1119_mt_input_parsing/input_parsing.rs.txt` (sha256 `4d80254c7aa39e0891902fc3b5ee93c04c53dbfad9efc472eea6a2948b8d2041`); the
builder's RED commit copies it to that path unchanged. The 19 refusal rows get
`tests/refusals.toml` entries, exact text in the sidecar
`design/FIX_bh1119_mt_input_parsing/refusals-toml-append.txt` (sha256
`2cfca6b7168399cd14ac213f9c10766ab1973cf85d1984c205a2346be8dbe342`; appended to the end
of the file unchanged; it begins with a blank line). Rows drive the real `mt` binary offline on the public fixtures;
fixture PSBTs are built in the file from `finalized_psbt_b64` (an unknown global
record, type `0xfa`, placed first).

Command run by the author on a885e62 with the row file in place:

    cargo nextest run --locked --build-jobs 4 -j 4 -p mt-cli --test bh1119_mt_input_parsing --no-fail-fast

Recorded: `26 tests run: 4 passed, 22 failed` (`/scratch/cache/mtip-red-final.log`).
The 4 passes are rows 6, 19, 20 (controls) and 25 (fixture guard). Every FAIL was read
from the log; the reason is the last column.

| # | row | input | GREEN expects (first line, exit 1, empty stdout unless "accept") | at a885e62 |
|---|---|---|---|---|
| 1 | `i1_encode_refuses_raw_then_even_hex` | `encode --qr`, raw.hex then even.hex | `mt encode: REFUSED — §8.2e, the input continues for 222 bytes after the end of the transaction` | FAIL: rc 0 (raw's record) |
| 2 | `i1_encode_refuses_even_then_raw_hex` | reversed | `… continues for 339 bytes after the end of the transaction` | FAIL: rc 0 (even's record) |
| 3 | `i1_encode_refuses_one_trailing_byte_on_hex` | even.hex + `ab` | `… continues for 1 byte after the end of the transaction` | FAIL: rc 0 |
| 4 | `i1_inspect_refuses_one_trailing_byte_on_hex` | `inspect`, even.hex + `ab` | `mt inspect: REFUSED — §8.2e, the input continues for 1 byte after the end of the transaction` | FAIL: rc 0 |
| 5 | `i1_verify_refuses_trailing_byte_on_transaction_file` | `verify --transaction (even.hex + ab)` on even's strings | `mt verify: REFUSED — §8.2e, the input continues for 1 byte after the end of the transaction`; no `--transaction matches` | FAIL: rc 0, "matches" |
| 6 | `i1_verify_still_compares_two_well_formed_transactions` | `verify --transaction uneven.hex` on even's strings | control: rc 1, `the supplied transaction is not the one on these strings` | PASS |
| 7 | `i1_decode_refuses_set_with_trailing_byte` | `decode`, set of even ‖ 0xab with even's txid | `mt decode: REFUSED — §1.1, the reassembled bytes continue for 1 byte after the end of the transaction` | FAIL: rc 0, hex ends `ab` |
| 8 | `i1_verify_refuses_set_with_trailing_byte` | same set, `verify --transaction even.hex` | `mt verify: REFUSED — §1.1, the reassembled bytes continue for 1 byte …`; no `mt verify: OK` | FAIL: rc 0, OK + match |
| 9 | `i1_inspect_refuses_set_with_trailing_byte` | same set, `inspect` | `mt inspect: REFUSED — §1.1, the reassembled bytes continue for 1 byte …` | FAIL: rc 0 |
| 10 | `i2_encode_refuses_two_binary_psbts` | final.psbt twice | `mt encode: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT` | FAIL: rc 0 |
| 11 | `i2_encode_refuses_two_unpadded_base64_psbts` | final.b64 twice (newline between) | same as 10 | FAIL: rc 0 |
| 12 | `i2_encode_refuses_padded_base64_then_second_record` | padded PSBT's base64 (ends `==`), then final.b64 | `mt encode: REFUSED — §8.2e, input looks like a base64 PSBT but does not decode` | FAIL: rc 0 |
| 13 | `i2_encode_refuses_text_after_padding` | padded base64 + `oops` | same as 12 | FAIL: rc 0 |
| 14 | `i2_encode_refuses_equals_then_junk` | final.b64 + `=oops` | same as 12 | FAIL: rc 0 |
| 15 | `i2_encode_refuses_nonzero_padding_bits` | padded base64 with the last data character `A` → `B` | same as 12 | FAIL: rc 0 |
| 16 | `i2_encode_refuses_dangling_base64_character` | final.b64 + `A` (length ≡ 1 mod 4) | same as 12 | FAIL: rc 0 |
| 17 | `i2_inspect_refuses_two_base64_psbts` | `inspect`, final.b64 twice | `mt inspect: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT` | FAIL: rc 0 |
| 18 | `i2_verify_refuses_two_psbts_as_transaction_file` | `verify --transaction (final.psbt twice)` on p5's strings | `mt verify: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT` | FAIL: rc 0 |
| 19 | `i2_encode_reads_line_wrapped_base64` | final.b64 wrapped at 64 | control: accept, stdout `tx:<raw_hex>\n` | PASS |
| 20 | `i2_encode_reads_unpadded_base64` | padded base64 with and without `==` | control: accept both, stdout `tx:<raw_hex>\n` | PASS |
| 21 | `i20_encode_reads_base64_with_long_first_key` | I20 PSBT: binary, then base64 (`cHNidP9…`) | accept; base64 stdout = binary stdout; `--qr` = `tx:<raw_hex>\n` | FAIL: rc 1 "not a PSBT or a raw transaction (1328 bytes)" |
| 22 | `i20_inspect_reads_base64_with_long_first_key` | `inspect`, I20 base64 vs final.b64 | accept; same stdout | FAIL: rc 1, same refusal |
| 23 | `i20_encode_names_a_damaged_long_key_base64_psbt` | I20 base64 with char 200 → `*` | `mt encode: REFUSED — §8.2e, input looks like a base64 PSBT but does not decode` | FAIL: "not a PSBT or a raw transaction" |
| 24 | `i20_argv_classifier_refuses_long_key_base64` | `mt encode <I20 base64>` on argv | `mt encode: REFUSED — §8.2f,` …; the base64 not echoed; empty stdout | FAIL: clap `error: invalid value 'cHNidP9A+g…` (echoed) |
| 25 | `i1_fixture_set_carries_even_txid` | codec decode of row 7's set | fixture guard: bytes = even ‖ `ab` | PASS |
| 26 | `i20_unrecognised_names_the_stem` | `encode`, `hello` | rc 1; stderr holds `` `cHNidP`), `` and not `` `cHNidP8` `` | FAIL: `` `cHNidP8` `` |

GREEN measured by the author on a throwaway prototype of section 3 (not committed; its diff is kept outside the repo at `/scratch/cache/mtip-prototype.diff` for reference only):
rows `26 tests run: 26 passed`; full workspace `428 tests run: 428 passed`
(`/scratch/cache/mtip-green-full.log`); `cargo fmt --check` and
`cargo clippy --all-targets --locked -- -D warnings` clean;
`check-refusal-coverage: 61 refusal tests over 18 ruled refusals`; `mutate-refusals: all 61 refusal tests go red when their check is removed`
(`/scratch/cache/mtip-mutate.log`); journeys, `docs-only.test.sh` (73 passed),
`code-docs-check.test.sh` (41 passed) and `code-docs-check.sh` (ok) clean. The committed `gate.sh` itself was run once on the prototype with both sidecars installed: `GATE PASS` (`/scratch/cache/mt-input-parsing-gate-proto.log`).

## 7. Tests that change

None. With the prototype, all 402 existing tests passed (428 run = 402 + 26 rows).
Added: the row binary (section 6) and 19 `tests/refusals.toml` entries (42 → 61; the
19 rows whose first line is a §-refusal of a new or changed guard). Row 6 is a control
of an existing refusal and asserts its verdict only; row 26 asserts one word of the
step-4 mechanism; neither is a new refusal, so neither has an entry.
Interaction found and avoided: `inspect` reads a binary PSBT as text first (bh-mt-I5,
FIX-D, row 2 of that table), so the inspect rows here use base64 input; once FIX-D
lands, binary inspect rows may be added there, not here.

## 8. Build order

1. Install both sidecars; run the row command; see `26 tests run: 4 passed, 22 failed`.
2. R4 (strict `base64_decode`) and R5 (`sniff` step 3a, `PSBT_BASE64_STEM`,
   `base64_psbt_guard`, step-4 text): rows 12-16, 21-23, 26 GREEN; 19, 20 stay GREEN.
3. R1 (`psbt_end_guard` at the three PSBT sites): rows 10, 11, 17, 18 GREEN.
4. R2 and R3 (`Subject`, `tx_end_guard`, `set_end_guard`): rows 1-5, 7-9 GREEN; rows
   6 and 25 stay GREEN.
5. R6 (classifier): row 24 GREEN.
6. Comments `main.rs:1541, 1674`; spec re-sync and PROVENANCE row once
   mnemonic-engrave carries the section 5 delta.
7. Gate (section 10) once to a file.

## 9. Coverage map (rule -> rows; `bh1119_mt_` omitted)

| rule | rows |
|---|---|
| R1 one input is one PSBT (E-PSBT; encode, inspect, verify `--transaction`) | 10, 11, 17, 18 |
| R2 one input is one transaction (E-TX; encode, inspect, verify `--transaction`) | 1, 2, 3, 4, 5; control 6 |
| R3 a set is one transaction (E-SET; decode, verify, inspect) | 7, 8, 9; fixture 25 |
| R4 canonical base64 (`=` mid-text, text after padding, spare bits, dangling char, two records) | 12, 13, 14, 15, 16; controls 19, 20 |
| R5 recognised by decoding; E-B64 on the `cHNidP` stem; step-4 word | 21, 22, 23, 26 (+ 12-16 for E-B64) |
| R6 argv classifier on the stem | 24 |
| Unchanged: line-wrapped base64, unpadded base64, verify's txid compare | 19, 20, 6 |

## 10. Gate (builder, final head; run once to a file, then grep)

    bash design/FIX_bh1119_mt_input_parsing/gate.sh > /scratch/cache/mt-input-parsing-gate.log 2>&1

The script runs every step of the CI `test` job once, checks the sidecars are
installed unchanged, and pins the counts. WHAT IT DOES NOT COVER is in its header (the
PROPOSED wording, the `docs-only` short-circuit job, the release workflow, the spec
re-sync in mnemonic-engrave). Its totals are measured on a885e62 + this FIX alone; a
builder landing after row 21 (`fix/mt-arg-intake`) or row 23
(`fix/mt-prefix-corrected`) adds their counts (section 13).

## 11. Open points for the architect (no STOP)

- O1 (wording) E-TX, E-PSBT, E-SET, E-B64 mechanism and the step-4 word: PROPOSED.
- O2 (decided here, flag if wrong) unpadded base64 stays accepted; non-zero spare bits
  in the last character are refused. Both are edge cases of R4. The first keeps every
  input accepted today that loses no text; the second is the canonical rule (RFC 4648
  §3.5 lets a decoder refuse it) and means every character either carries data or is
  refused. P1/B1 (near-canonical input) is scoped to mk and me and is not applied here:
  mt engraves the re-serialized transaction, never the base64 text.
- O3 (decided here) E-SET's remedy points at the producer, not at mixed plates: a
  trailing-byte set passes every checksum and the content id, so mixing plates is not
  the cause.
- O4 (adjacent, not fixed) `decode_tx`'s parse-error text (`main.rs:2148-2156`) says
  "the reassembled bytes … The strings reassembled cleanly" also on the input sites
  (`encode` and `inspect` raw hex, `:523`, `:1579`), where no strings exist. Filed as
  a FOLLOWUPS line (report), not fixed here: it changes a text outside I1/I2/I20.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

Closes bh-mt-I1, bh-mt-I2, bh-mt-I20 (none has a line in `design/FOLLOWUPS.md` at
a885e62). Files two lines (O4 and the spec-copy drift), verbatim in the author's
report.

## 13. Files this FIX touches (for build ordering against rows 21 and 23)

- `crates/mt-cli/src/input.rs` (R1-R5, step-4 text)
- `crates/mt-cli/src/main.rs` (R1-R3 call sites, `Subject`, `set_end_guard`,
  `psbt_exact` helper if the builder wants one, two comments)
- `crates/mt-cli/src/validate.rs` (R6, one line at :776)
- `crates/mt-cli/tests/bh1119_mt_input_parsing.rs` (new)
- `crates/mt-cli/tests/refusals.toml` (append)
- `design/SPEC_mt_v0_1.md`, `design/PROVENANCE.md` (re-sync, step 6)

Row 21 (`fix/mt-arg-intake`) also changes `main.rs`, `validate.rs` and
`refusals.toml`; row 23 (`fix/mt-prefix-corrected`) changes the spec copy. Text
conflicts only; no rule overlaps.
