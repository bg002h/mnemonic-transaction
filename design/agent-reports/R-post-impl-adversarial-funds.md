## Verdict

**NOT SAFE** — 2 Critical / 8 Important / 4 Minor (`2C / 8I / 4M`). Every finding below was reproduced against `./target/debug/mt`, offline via `--bitcoin-cli /nonexistent`, with hostile inputs built for this review.

## Findings

### [Critical] 1 — `mt verify` prints "transaction re-derives" without ever deriving it; §1.1's content-id check does not exist

**Where**
- `crates/mt-cli/src/main.rs:694` — the OK line: `"mt verify: OK — {} chunks, set {set_id:#07x}, transaction re-derives."`
- `crates/mt-codec/src/string_layer/pipeline.rs:234-360` — `decode()` never derives a content id from `bytes` and never compares it to `set_id`.
- `crates/mt-codec/src/error.rs:119` — `Error::ContentIdMismatch` is **declared and never constructed anywhere in the workspace** (`grep -rn ContentIdMismatch crates/` returns one hit, the declaration).
- `crates/mt-cli/src/main.rs:705-766` — `verify()` never calls `decode_tx`, so it does not even parse the reassembled bytes as a transaction.

**What**
SPEC §1.1 (line 219-224) is normative and explicit:

> **`mt verify` is STRUCTURAL ONLY.** … It checks: every string parses, every BCH checksum holds, the set is complete …, every chunk carries the same `chunk_set_id`, **and the reassembled transaction re-derives that id.**

and §1.1 specifies the failure wording verbatim (spec line 459-461):

> `mt verify: FAILED — 14 chunks, set 0x0e17e, every checksum holds,` / `but the transaction does not re-derive its id.`

The last clause of the check is not implemented. `verify` asserts the outcome of a check that never runs, on every single invocation. The claim is unconditional — it is a string literal.

**Why it costs money or a recovery**
This is the exact case §1.1 wrote the check for, and the spec names it: `t = 4` means five or more damaged symbols in one string can land on a *different valid codeword*, and "the decoder then 'corrects' a chunk into something that checksums perfectly and is not what was engraved. Per-chunk verification cannot see this." The content id is the only thing in the design that sees it.

An operator who types a plate back, gets `OK … transaction re-derives.`, and puts the steel away has been told their engraving is sound when it is not. There is zero redundancy behind it (§1.8, by ruling), so the recovery is gone and nothing will tell them until they try to broadcast. `mt encode` itself prints the instruction that produces this moment — *"when you are done, verify the ENGRAVING — not this output … `mt verify < typed-from-steel.txt`"* — so `verify` is the one verb the whole workflow routes through, and it is the one that lies.

Worse, `mt decode` will happily emit the wrong transaction's hex on stdout for the same set (reproduced below), and `mt inspect` reports it as an ordinary transaction.

**How I reproduced it**
Forged the exact state a BCH mis-correction lands in — every checksum valid, every header intact, one chunk's payload not what was engraved — using the *independent* Python implementation in `mnemonic-engrave/scripts/gen-mt1-vectors.py`, so nothing in `mt-codec` produced the fixture.

Case A, payload no longer a transaction (mutate one payload symbol in chunk 0, recompute a valid checksum):

```
$ mt verify --bitcoin-cli /nonexistent --in clean.txt
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.
$ mt verify --bitcoin-cli /nonexistent --in miscorrected.txt
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.        <-- exit 0
$ mt decode --bitcoin-cli /nonexistent --in miscorrected.txt
mt encode: REFUSED — §8.2e, the reassembled bytes are not a Bitcoin transaction
```

`verify` says OK; `decode` refuses the same set. The two verbs disagree, and the one the operator is told to run is the one that passes.

Case B — the money case: the reassembled bytes still parse, so **nothing** objects. I took the P5 fixture transaction, subtracted 1,000,000 sat from its single output, and re-encoded it with the **original** transaction's txid in every chunk header (`set_id = 0x4665e`):

```
$ mt verify  --bitcoin-cli /nonexistent --in miscorrected_valid.txt
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.        <-- exit 0

$ mt decode  --bitcoin-cli /nonexistent --in miscorrected_valid.txt
TX        bb30870c5cacd8191fe790040097d7b1eb45752da1d57b4e7358744a57e0082d
mt1 SET   9 strings, all present
<broadcastable hex on stdout, exit 0>

$ mt inspect --bitcoin-cli /nonexistent --in miscorrected_valid.txt
TX        bb30870c5cacd8191fe790040097d7b1eb45752da1d57b4e7358744a57e0082d
OUT       1 output(s)
            bc1q07h88fcj0j86excq5m9k97e26su7j5tdvldytq   7.98900000 BTC
```

Every chunk header carries set id `0x4665e`, the top 20 bits of the *engraved* transaction's txid `4665e0aa…`. The reassembled transaction's txid is `bb30870c…`, whose content id is `0xbb308`. They differ, and no verb notices.

Fixture generator: `/tmp/claude-1000/-scratch-code-shibboleth-mnemonic-engrave/991a44a2-d6c3-4fc0-a00c-fc7d9a16f46f/scratchpad/mt/miscorrect2.py`.

**Suggested fix (non-authoritative)**
Derive `content_id_from_txid_display(txid_of(bytes))` after reassembly and compare it to the header `set_id`; raise the `ContentIdMismatch` that already exists. Note the derivation needs the *txid*, i.e. it needs the bytes to parse as a transaction first — which is a second thing `verify` currently does not do — so the codec cannot do it alone and `mt-cli` has to supply the txid, exactly as `pipeline::encode` already takes it as a parameter.

---

### [Critical] 2 — §8.2f is bypassed by the invocation it exists to refuse, and clap echoes the whole bearer transaction

**Where**
- `crates/mt-cli/src/main.rs:145` — `let cli = Cli::parse();` runs in `main()`.
- `crates/mt-cli/src/main.rs:173` — `validate::command_line_guard(...)` is the first line of `encode()`, i.e. **after** clap has already parsed, failed, printed and exited.
- `crates/mt-cli/src/validate.rs:374` — the guard itself, which never runs for this input.

**What**
`EncodeArgs` declares no positional argument, so a transaction typed as a bare argument is an *unexpected argument* to clap. Clap errors out inside `Cli::parse()`, before `encode()` is entered — and its error message **prints the argument back verbatim**:

```
$ mt encode 0200000000010293506bde…00000000
error: unexpected argument '0200000000010293506bde743e0dd27e4933a9b45970dc97b8f55bcf07d6613d2db3d0bb7c3dd1…
(the entire 678-character signed transaction, on stderr)

Usage: mt encode [OPTIONS]
```

The operator gets: no §8.2f refusal, no purge command, no statement that the transaction is now in shell history and was visible in `ps`, and the bearer artifact re-emitted into terminal scrollback / logs / CI capture. `command_line_guard` is reachable only when the material happens to be the *value of a known flag* — which is precisely and only what the existing test does (`refusals.rs:349`, `--to-label <raw_hex>`).

Same hole on the reading verbs: `mt verify mt1p9h8…` — the spelling the sibling tools use (`md verify <STRINGS>…`, `mk verify [MK1_STRINGS]…`, and the spec calls that precedent out by name) — produces `error: unexpected argument 'mt1pgej7qq…' found`, echoing an engraved string with no advice at all.

**Why it costs money or a recovery**
SPEC §8.2f (spec line 2444-2465) is a normative operator ruling: *"A PSBT or transaction passed as a COMMAND-LINE ARGUMENT → refuse, and tell the operator how to clean up."* Its whole content is that a finalized transaction is a **bearer** instrument — anyone who reads it broadcasts it — and that as an argument it is already in the history file and was already in `ps`. The guarantee is unmet for the single most obvious way to trip it, and the tool then commits the *second* leak the guard's own comment forbids:

> `// NEVER echo the argument. Printing it back into the refusal would put the bearer material in a SECOND place -- the same defect the refusal exists to name.`

The operator who makes this mistake is left with the transaction in `~/.bash_history` and in the scrollback, and no reason to think anything happened beyond a usage error.

**How I reproduced it**
```
$ ./target/debug/mt encode $(cat raw.hex)          # raw.hex = fixtures/p5_base.json raw_hex
$ ./target/debug/mt verify "$(head -1 clean.txt)"
```
Both print the material back and exit through clap; neither prints `§8.2f`, `shell history`, or a purge command.

**Suggested fix (non-authoritative)**
Run the §8.2f scan over `std::env::args()` before `Cli::parse()`, and give the reading verbs the same treatment for `mt1` strings. Adding a positional to absorb them would be worse — it would make the leaking invocation *succeed*.

---

### [Important] 3 — §8.2c's legacy warning fires on TXID-BOUND inputs and states three things that are false

**Where**
- `crates/mt-cli/src/main.rs:350-351` — `let legacy = inp.witness.is_empty(); if legacy && !bound_by_chain[n] {`
- `crates/mt-cli/src/validate.rs:336-357` — `legacy_unbound_warning`.

**What**
The guard tests only *is this input non-witness* and *did the chain answer*. It never consults the provenance the code computed 100 lines earlier. So the common legacy path — a PSBT input carrying `non_witness_utxo`, which Bitcoin Core always attaches for legacy inputs, which §8.2d hashed and matched — fires the warning, and the warning's body says:

> `NOTHING HAS VERIFIED THAT VALUE. This input carries no non_witness_utxo, so mt could not bind it by txid (see 8.2d)`

while the report five lines later prints `TXID-BOUND` for that exact input. Two further false clauses in the same block:

- `You have told mt it holds: 10.00000000 BTC` — the operator told `mt` nothing; the number came from the PSBT record. With `--input-value 0:3.0` supplied and *ignored* (the record wins, correctly), it still says "You have told mt it holds: 10.00000000 BTC".
- `If that input actually holds 10 BTC, this transaction pays 9.01 BTC in fees and a miner will simply take it.` — a hardcoded illustration printed as though it described the numbers on screen. In the reproduction the input **does** hold exactly 10 BTC and the transaction pays 0.01 BTC.

The function's own doc-comment states this was already fixed: *"The earlier rule fired whenever any input is legacy while asserting mt could not bind the value by txid — which §8.2d now does — so in the common case … it printed a false, capitalised block."* The provenance test it describes is not in the code. `legacy_unbound_warning` has **zero tests** (`grep -rn "pre-SegWit\|legacy_unbound\|NOTHING HAS VERIFIED" crates/` hits only the two source lines), and the `legacy_parent_*` fixture material in `p5_base.json` is referenced by nothing.

**Why it costs money or a recovery**
The doc-comment already argues the cost and it is right: *"A warning that cries wolf on the normal path has negative value, because it trains the operator to ignore the rare case where it is true."* The rare case where it is true — a legacy input with no `non_witness_utxo` whose value came from `--input-value`, where the fee really does absorb the whole error — renders in identical capitals. And the inversion is complete: the genuinely unverified case, a segwit input with only `witness_utxo`, gets **no** warning at all, because `inp.witness` is non-empty.

**How I reproduced it**
Built a legacy PSBT from the unused `legacy_parent_*` fixture material: one P2PKH input (`9b7c8a80…:1`, 10.0 BTC), `non_witness_utxo` = the fixture parent (hashes to the fixture txid), finalized `scriptSig` = the fixture's own 71-byte DER sig (`…01`) + 33-byte pubkey.

```
$ mt encode --bitcoin-cli /nonexistent --in legacy1.psbt.b64
WARNING: input 0 is a legacy (pre-SegWit) input.
  …
  NOTHING HAS VERIFIED THAT VALUE. This input carries no
  non_witness_utxo, so mt could not bind it by txid (see 8.2d) …
…
INPUTS    1 input(s)
            9b7c8a80ddc880a9…   10.00000000 BTC   TXID-BOUND
```
Generator: `scratchpad/mt/mkpsbt.py legacy1`.

**Suggested fix (non-authoritative)**
Gate on the resolved provenance (`!values[n].1.is_verified()`), not on `witness.is_empty()`, and derive the "you told mt" clause from the provenance rather than asserting it.

---

### [Important] 4 — the same warning's fee arithmetic is per-input against TOTAL outputs, and contradicts the FEE row

**Where** `crates/mt-cli/src/validate.rs:337` — `let fee = claimed_sat.saturating_sub(out_total_sat);`, rendered at `validate.rs:340-342`.

**What**
The block prints, as fact:

```
The fee you will pay is:   (what is REALLY at that input) - <ALL outputs>
You have told mt it holds:  <this one input>
So mt shows a fee of:       <this input − all outputs, saturating>
```

The formula is only correct for a single-input transaction. On any multi-input transaction the subtraction saturates to zero and the block asserts a fee of `0.00000000 BTC`, while `report.rs` prints the real fee about twenty lines below it.

**Why it costs money or a recovery**
The stated purpose of this paragraph is to tell the operator how much money a miner could take. It prints a number, labels it "So mt shows a fee of", and that number is not what `mt` shows. An operator who reads the warning and stops there sees a zero fee on a transaction paying 0.001 BTC; an operator who reads both sees `mt` contradicting itself about the fee immediately before they cut permanent steel. A false statement of fact inside a money warning is worse than no warning.

**How I reproduced it**
Two legacy inputs (10.00000000 + 39.99999853 BTC), one output of 49.99899853 BTC, fee 0.001 BTC:

```
WARNING: input 0 is a legacy (pre-SegWit) input.
  The fee you will pay is: (what is REALLY at that input) - 49.99899853 BTC
  You have told mt it holds: 10.00000000 BTC
  So mt shows a fee of: 0.00000000 BTC
…
FEE       0.00100000 BTC
```
(the identical block repeats for input 1). Generator: `scratchpad/mt/mkpsbt.py legacy2`.

---

### [Important] 5 — §5's legend is not implemented, and `--from` / `--to` / `--to-label` are accepted and silently discarded

**Where**
- `crates/mt-cli/src/main.rs:84-99` — `from`, `to`, `to_label` declared on `EncodeArgs`, documented in `--help` as *"Wallet id or fingerprint for the legend's `FROM` line"* / *"…`TO` line"*.
- `grep -n "args.from\|args.to\b\|to_label" crates/mt-cli/src/main.rs` returns **only the declarations**. Nothing reads them.

**What**
SPEC §0a (line 105-142) is normative: *"**`mt encode` therefore PRINTS suggested legend text on `stderr`**, which the operator may engrave beside their strings"*, with a table of what is printed **once** (`BEARER…`, `FROM`, `TO`, `LOCKED TO BLOCK n ~SEASON year`, `FORMAT: mt1 codex32`) and what is printed **per string** (`n/m`). §5's heading is *"LIVE for `mt encode`"* and its note says *"Five fields of this section are printed by `mt encode` in v0.1"*. §5 also requires `FROM` and `TO` to be *"loudly warned when absent/blank"* (§10.4).

None of it exists. No `FORMAT: mt1 codex32`, no `FROM WALLET`, no `TO`, no `LOCKED TO BLOCK … ~SEASON year` legend line, no `n/m` per string, and no warning when the fields are absent. (The `BEARER` *warning block* is present — `blocks::bearer_warning` — but that is the §8.6 hazard warning, not the legend line.) The legend is not in the deferred set: only `mt qr`, §8.7, §8.7c and §8.8 are.

**Why it costs money or a recovery**
Two separate costs.

1. §5 calls `FORMAT: mt1 codex32` *"the only field a recoverer cannot do without, and the only one naming a standard rather than this project"*. `mt` never suggests it, so it never gets cut, so a stranger or an heir holding the plate has 90-character strings and no name for the tool that reads them. §0a de-rates the legend to *"a convenience that shortens a recovery — never a component one depends on"*, which is why this is Important rather than Critical — but the field the spec singles out as load-bearing is the one being dropped.
2. The silent-flag half is the sharper one. An operator who runs `mt encode --to "cold storage" --to-label "safe deposit box 12"` has performed a deliberate act of labelling, been given `--help` text promising a legend line, and received **nothing, with exit 0**. They will believe the plate is labelled.

**How I reproduced it**
```
$ mt encode --bitcoin-cli /nonexistent --in legacy1.psbt.b64 \
      --from deadbeef --to "cold storage" --to-label "safe deposit box 12" \
      >so.txt 2>se.txt ; echo $?
0
$ grep -icE "FORMAT: mt1|FROM WALLET|LOCKED TO BLOCK|deadbeef|cold storage|safe deposit" se.txt so.txt
se.txt:0
so.txt:0
```

---

### [Important] 6 — §1.1e's length check does not exist, so a dropped character reports a MISSING PLATE

**Where**
- `crates/mt-codec/src/string_layer/pipeline.rs:162` — `decode_chunk` checks only a *minimum* symbol count; no expected length is computed or compared.
- `crates/mt-codec/src/string_layer/pipeline.rs:234-360` — `decode()` computes no modal length. `grep -rn modal crates/` hits only a JSON `note` field.
- `crates/mt-cli/src/main.rs:660-690` — on the failure path, `pipeline::decode`'s `Err` is returned and the `unreadable: Vec<Unreadable>` diagnostics collected at `pipeline.rs:248-259` are **discarded entirely**; `set_notices` is only called after `decode` returns `Ok`.

**What**
SPEC §1.1e (line 993-1035) is normative and detailed: *"Every string in a set has a KNOWN length, checked before decoding, because it catches the one damage class BCH cannot"*, *"AT DECODE TIME THE EXPECTED LENGTH COMES FROM THE STRINGS THEMSELVES — the MODAL length across the set"*, and it specifies the message:

> `string 7: 89 characters (expected 90) — a character is MISSING, not wrong. BCH repairs substitutions; an omission shifts every symbol after it and cannot be corrected. Re-read the plate.`

Nothing implements it. And because the P6 unreadable-string diagnostics are thrown away whenever decode fails, the operator is not even told *which string* they mistyped.

**Why it costs money or a recovery**
`mt encode` prints, always, *"Count each string: strings 1-4 are 90, string 5 is 83"* — so the tool tells the operator this damage class exists and then never checks for it. What they get instead is an accusation about their steel: `chunk 3 of 9 is missing`. An operator holding a complete, undamaged, correctly-engraved set of nine plates is told a plate is gone. The correct action (retype string 3, you dropped a character) and the action the message provokes (go hunting for a plate, conclude it was lost) are opposite, and the second one abandons a recoverable transaction with zero redundancy behind it.

**How I reproduced it**
Deleted one character from string 3 of a clean nine-string set, no duplicate present:

```
$ mt verify --bitcoin-cli /nonexistent --in dropped.txt
mt verify: REFUSED — §1.1, the set does not verify

  chunk 3 of 9 is missing
```
String 3 is 87 characters; every other full string is 88. Nothing says so.

---

### [Important] 7 — `--separator` accepts non-whitespace, and `mt encode` then emits an artifact its own verbs refuse

**Where** `crates/mt-cli/src/main.rs:114-117` (`--separator`, `default_value = " "`, unconstrained) and `main.rs:447` (`.join(&args.separator)`), against `crates/mt-cli/src/read_strings.rs:25` which strips only `split_whitespace()`.

**What**
`--group-size N --separator -` puts hyphens on **stdout**, the canonical artifact stream, the thing the operator engraves. `read_strings::read` strips only whitespace, so the codec then sees `-` as a data character and refuses.

**Why it costs money or a recovery**
`mt encode` closes by telling the operator *"when you are done, verify the ENGRAVING — not this output"*. So the sequence is: choose a separator, engrave nine plates over several hours, type them back, and find that `mt` cannot read what `mt` produced. There is no warning at encode time and no constraint on the flag. The plates are scrap.

**How I reproduced it**
```
$ mt encode --bitcoin-cli /nonexistent --in raw.hex --group-size 5 --separator - > sep.txt
$ head -c 40 sep.txt
mt1pg-ej7qq-gqqqq-gqqqq-qqqyp-fx5rt-me6r
$ mt verify --bitcoin-cli /nonexistent --in sep.txt
mt verify: REFUSED — §1.1, the set does not verify
  character '-' at data-part offset 2 is not in the bech32 alphabet
```
`--separator .` behaves the same; whitespace separators round-trip correctly.

**Suggested fix (non-authoritative)**
Either restrict `--separator` to whitespace at parse time, or make `read_strings` strip whatever `encode` can emit. The first is smaller and does not widen what the recovery path accepts.

---

### [Important] 8 — §8.2d binds the txid but not the vout, and an unverified `witness_utxo` then renders as TXID-BOUND

**Where**
- `crates/mt-cli/src/validate.rs:278-288` — `psbt_input_value` prefers `non_witness_utxo`, but silently falls through to `witness_utxo` when `prev.output.get(vout)` is `None`.
- `crates/mt-cli/src/main.rs:245` — `let bound = psbt.inputs[n].non_witness_utxo.is_some();` — the label is decided by *presence of the record*, not by *which record the value came from*.
- `crates/mt-cli/src/validate.rs:242-266` — `non_witness_utxo_guard` compares `compute_txid()` to `previous_output.txid` and never checks that `previous_output.vout` addresses an existing output of it.

**What**
A PSBT input whose `non_witness_utxo` hashes correctly to the input's txid but whose `vout` is out of range for that parent takes its value from the `witness_utxo` — which nothing has checked — and renders it under the `TXID-BOUND` heading, with the `FEE` row printed *without* the `(CLAIMED — no input value verified)` qualifier.

**Why it costs money or a recovery**
This is verbatim the defect class `report.rs`'s own module doc says the three-column split exists to prevent: *"Collapsing it put an unverified number in the verified column — R6 adversarial I-5"*, and `psbt_input_value`'s doc says the record preference is *"the whole point"* precisely so that *"Reading the weaker record while labelling the row `TXID-BOUND` would put an unverified number under a verified heading"*. The fee absorbs the whole of any error in an input value, and the label is what tells the operator whether to trust the figure before cutting.

**How I reproduced it**
PSBT with one input at `9b7c8a80…:5`, `non_witness_utxo` = the fixture parent (hashes correctly; it has 2 outputs), `witness_utxo` claiming 1.00100000 BTC, one output of 1.00000000 BTC:

```
$ mt encode --bitcoin-cli /nonexistent --in voob.b64
FEE       0.00100000 BTC
INPUTS    1 input(s)
            9b7c8a80ddc880a9…   1.00100000 BTC   TXID-BOUND
```
Nothing verified 1.00100000. Generator: `scratchpad/mt/mk2.py vout_oob`.

Secondary: the transaction spends an outpoint that cannot exist, and no §8 refusal sees it.

---

### [Important] 9 — the UNREADABLE-STRING notice asserts something it cannot know, and tells the operator a plate is scrap

**Where** `crates/mt-cli/src/main.rs:642-655` — `set_notices`, the `for u in &set.unreadable` arm.

**What**
For every string that failed to read, `mt` prints:

> `Its chunk came from another copy, so this SET is complete — but that plate is scrap. Re-cut it from the strings mt has verified.`

`decode` could not read the string, so it does not know which chunk it was, or whether it belonged to this set at all. The claim is unknowable by construction, and it is stated as fact.

**Why it costs money or a recovery**
It directs a physical action on steel. A string that fails to read may be a plate from a *different* engraved transaction that happened to get typed in with this pile, or a stray line — and `mt` tells the operator it is scrap and to re-cut it from this set's strings. Acting on that overwrites or discards a plate `mt` never identified.

**How I reproduced it**
Appended one syntactically-valid but checksum-invalid `mt1…` line to a complete, clean nine-string set:

```
$ mt verify --bitcoin-cli /nonexistent --in extra.txt
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.

UNREADABLE STRING. string 10 of the input could not be read:
  BCH correction failed: regular code: more than 4 substitutions or pathological pattern
  Its chunk came from another copy, so this SET is complete — but
  that plate is scrap. Re-cut it from the strings mt has verified.
```
String 10 was never a chunk of this set.

---

### [Important] 10 — `--input-value` amounts go through `f64` with a saturating cast, and the sum is unchecked

**Where** `crates/mt-cli/src/main.rs:841` — `Ok((idx, (btc * 100_000_000.0).round() as u64))`; consumed at `crates/mt-cli/src/validate.rs:156` — `input_values.iter().copied().sum::<Option<u64>>()`. Same cast at `crates/mt-cli/src/node.rs:160`.

**What**
`"inf"`, `"nan"`, `"-5"` and `1e300` all parse successfully as `f64`. `as u64` then saturates or zeroes them silently — `inf` → `u64::MAX`, `nan` → `0`, `-5` → `0` — and the values reach the fee arithmetic with no validation. The sum then overflows.

```
$ mt encode --bitcoin-cli /nonexistent --in raw.hex --input-value 0:inf --input-value 1:0.1
thread 'main' panicked at library/core/src/iter/traits/accum.rs:149:1:
attempt to add with overflow                                      (exit 101)

$ mt encode --bitcoin-cli /nonexistent --in raw.hex --input-value 0:1e300 --input-value 1:1e300
attempt to add with overflow                                      (exit 101)

$ mt encode --bitcoin-cli /nonexistent --in raw.hex --input-value 0:nan --input-value 1:4.0
mt encode: REFUSED — §8.2b, outputs total 799900000 sat but inputs total only 400000000 sat
```

**Why it costs money or a recovery**
Under the workspace's dev/test profile (`debug-assertions` on) this is an unhandled panic on the fee path — not money, but not a refusal either. The money vector is the same line under `--release`, where the identical sum **wraps** instead of panicking: `u64::MAX + X` is `X − 1`, so a value pair can be chosen that wraps to a plausible total, passes §8.2b's ceiling, and engraves with a fee figure computed from a wrapped sum. No release binary is produced today (CI builds debug only, and v0.1 publishes nothing), which is the only thing keeping this from being a wrong engraved number rather than a crash.

The `nan → 0` case is separately worth noting: `mt` refuses, but the refusal states `inputs total only 400000000 sat` without ever saying that it could not understand `nan`.

**Suggested fix (non-authoritative)**
Parse the amount as a decimal string to satoshis rather than through `f64`, reject non-finite and negative values by name, and use `checked_add` for the total.

---

### [Minor] 11 — an out-of-range or duplicate `--input-value` index is silently ignored, and §8.2b then silently does not run

**Where** `crates/mt-cli/src/main.rs:243-268` and `270-277` (`asserted.iter().find(|(i,_)| *i as usize == n)`), `crates/mt-cli/src/validate.rs:156` (the `else { return Ok(()) }` that skips every value check when any input value is `None`).

**What / why**
`--input-value 1:4.0 --input-value 2:4.0` on a two-input transaction — a plain 1-based/0-based slip — puts input 0's value on input 1, drops the other, and says nothing. `value_guard` then returns `Ok(())` before any of §8.2b's arithmetic, so *no* value check runs. Duplicated indices take the first silently.

```
$ mt encode --bitcoin-cli /nonexistent --in raw.hex --input-value 1:4.0 --input-value 2:4.0
FEE       UNKNOWN   (needs input values, which the transaction …
INPUTS    2 input(s)
            d13d7cbbd0b32d3d…   UNKNOWN
            b6c8e4075b8481b7…   4.00000000 BTC   OPERATOR-ASSERTED
```

Minor rather than Important because the degradation is to `UNKNOWN`, not to a wrong number: index 0 is always the one left unset by a 1-based slip, and `UNKNOWN` suppresses the fee. But a supplied flag that vanishes without comment, and a whole §8 check that stops running as a result, is the shape that becomes a wrong number the moment a fallback is added.

---

### [Minor] 12 — `mt decode`'s "not a transaction" refusal is labelled `mt encode:` and gives encode-path advice on the recovery path

**Where** `crates/mt-cli/src/main.rs:459-473` — `txid_display` hardcodes `"encode"` as the verb and the remedy `"Check this is the output of \`finalizepsbt\`, not a template."`; called from `decode()` at `main.rs:678`.

**What / why**
```
$ mt decode --bitcoin-cli /nonexistent --in miscorrected.txt
mt encode: REFUSED — §8.2e, input is not a decodable Bitcoin transaction
  … mt reads an ALREADY-SIGNED transaction; it does not build one.
  Check this is the output of `finalizepsbt`, not a template.
```
A recoverer holding steel is told to check the output of `finalizepsbt`. They have no PSBT, no wallet, and no template — and the real cause (a mis-corrected string) is not mentioned. `decode_tx` at `main.rs:821` takes a `verb` parameter for exactly this reason; `txid_display` does not.

---

### [Minor] 13 — a `--input-value` that contradicts a PSBT record is discarded without a word

**Where** `crates/mt-cli/src/main.rs:243-268` — `psbt_input_value(...).map(...).or_else(|| asserted…)`.

**What / why**
The precedence is right (a record beats an assertion), but §6a refuses a *chain*-vs-record mismatch by name — *"One of the two is wrong, and mt cannot tell which"* — and there is no equivalent for an *operator*-vs-record mismatch, even though the operator's number is the one they went out of band to obtain. With `--input-value 0:3.0` against a record of 10 BTC, `mt` shows 10 BTC, prints no notice, and (see Finding 3) tells them "You have told mt it holds: 10.00000000 BTC".

---

### [Minor] 14 — the absurd-fee ceiling truncates, so the effective ceiling is 25,001 sat/vB

**Where** `crates/mt-cli/src/validate.rs:177` — `let rate = fee / vb.max(1) as u64;` then `if rate > MAX_FEE_RATE_SAT_VB`.

**What / why**
Integer division, so a true rate of 25,000.9 sat/vB computes as 25,000 and passes. `low_fee_warning` at `validate.rs:214` uses `f64` for the same quantity, so the two halves of §8.2b measure the rate differently. Sub-1-sat/vB at the ceiling; recorded for consistency, not because it moves money.

## What I checked and found CLEAN

- **§8.6 satisfaction guard.** `satisfaction_elements` strips the annex before the control block and leaf script; taproot key-path (64-byte and 65-byte), P2WPKH, P2WSH and legacy P2PKH scriptSig pushes all classify correctly; `sighash_all_guard` refuses `0x02`/`0x03`/`0x81`/`0x82`/`0x83` by name and skips only taproot's implicit `SIGHASH_DEFAULT`. The 64-byte-DER-signature aliasing hole needs ~2^56 grinding and is not reachable. The crafted-witness limit is stated in the spec and in the bearer warning and is not a finding.
- **§8.9 secret guard.** Refuses `ms1` on `encode`, `decode`, `verify` and `inspect`, and never echoes any part of the body. I traced every echo path that could reach an `ms1` body: `Error::InvalidHrp(String)` does carry the whole string, but `read_strings::restore_elided` prepends `mt1<prefix>` to every non-`mt1` candidate, so `to_symbols` never raises it from the CLI, and the worst leak is a single character name (`character '1' at data-part offset 10`). The 64-byte sniff window *is* bypassable by a long first line — the guard then misses — but nothing downstream echoes the body, so there is no leak.
- **§8.2f's recogniser** (`looks_like_a_transaction`): correctly narrow. `--to-label "cold storage, safe deposit box 12"` is not mistaken for a transaction; a base64 PSBT and a ≥100-char even-length hex string both trip it. The failure is in *when* it runs, not what it matches (Finding 2).
- **PSBT input-count mismatch** (more input maps than `unsigned_tx.input` entries) is refused by `rust-bitcoin`'s deserializer before `non_witness_utxo_guard` can index out of bounds. No panic.
- **`--elide-prefix` + `--group-size` round trip.** Encode → verify → decode reproduces `raw_hex` byte for byte, with whitespace separators, mixed full/elided lines, single-line blobs and uppercase input.
- **Duplicate resolution** (`pipeline.rs:304-332`): byte-identical copies are accepted, the healthier copy is kept, distinct valid payloads raise `AmbiguousChunk`, and the `DUPLICATE RESOLVED` arithmetic (`T − discarded_corrections`, singular/plural) is correct.
- **Chunking** (`chunk::plan`) is balanced, not filled, and `chunk_ceiling_guard`'s `div_ceil(PAYLOAD_CEILING_BYTES)` matches `plan`'s `count` formula exactly, so §8.7b's refusal cannot quote a ceiling the codec does not use.
- **Header round trip**, `count − 1` offset, version rejection, `index >= count` rejection: all correct and directly asserted.
- **`extract_value_sats`** parses Core's `gettxout` JSON correctly (`"value"` is the first matching key in Core's field order) and the `f64→sat` conversion is exact for every amount up to 21e14 sat.
- **Node absence** is treated as absence, not failure, everywhere; `--bitcoin-cli /nonexistent` forces offline cleanly and no code path panics on it.
- **Refusals produce no stdout.** Every refusal I triggered wrote nothing to stdout, so the documented `mt decode | bitcoin-cli` pipeline cannot carry a transaction that failed a check.
- **Already filed, deliberately not re-reported:** the §8.5-vs-already-confirmed Critical and the node-reachable/`gettxout`-null provenance regression, both in `design/agent-reports/R-post-impl-live-node.md`. I confirmed in the source that both are still unfixed (`encode`'s node loop at `main.rs:285-315` never asks the AlreadyConfirmed question; `Report::build`'s `Utxo::Null` arm still terminates in `Provenance::Unknown`) and that both are distinct from everything above.
