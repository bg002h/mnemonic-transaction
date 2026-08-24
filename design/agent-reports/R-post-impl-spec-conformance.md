# R — post-implementation SPEC-CONFORMANCE review

**Lens:** where does the IMPLEMENTATION disagree with the SPEC's normative rulings?
**Method:** spec-first. Walked §1.1 / §1.1a / §1.1e, §3 / §3b, §5, §6a, §8 (full) and
§8.4 ruling by ruling, and for each one located the code that implements it or
established that none does. Every divergence below was reproduced against a built
`./target/debug/mt` before it was written down; the reproduction commands are in
each finding's **How I checked**.

**Repo:** `/scratch/code/shibboleth/mnemonic-transaction` @ `1b2859c`
**Spec:** `mnemonic-engrave/design/SPEC_mt_v0_1.md`
**Working tree was untouched** — `git status --porcelain` empty before and after.

---

## Verdict

**DIVERGES — 4 Critical / 7 Important / 8 Minor.**

---

## Section-by-section

| § | ruling | verdict |
| --- | --- | --- |
| §1.1 | four verbs `encode` / `decode` / `verify` / `inspect` | IMPLEMENTED — `crates/mt-cli/src/main.rs:33-42` |
| §1.1 | all human-facing output numbers chunks from **1**; wire `index` appears nowhere | IMPLEMENTED — `main.rs:544,569,597`; `pipeline.rs:292,312` |
| §1.1 | `verify` is STRUCTURAL ONLY and never asks a node | IMPLEMENTED — `main.rs:684-745` makes no `Node` call |
| §1.1 | `verify` checks: strings parse, checksums hold, set complete, one `chunk_set_id` | IMPLEMENTED — `pipeline.rs:264-351` |
| §1.1 | **…and the reassembled transaction re-derives that id** | **NOT IMPLEMENTED — C-1** |
| §1.1 | the re-derivation FAILURE report (ranked suspect list) | **NOT IMPLEMENTED — C-1** |
| §1.1 | duplicate rule over `n` candidates, post-correction bytes, no majority vote | IMPLEMENTED — `pipeline.rs:304-334` |
| §1.1 | duplicate row 1 announced, row 2 silent, row 3 refuses | IMPLEMENTED — `pipeline.rs:310-313`; `main.rs:590-637` |
| §1.1 | the correction-coverage block, printed ALWAYS before cutting | IMPLEMENTED — `blocks.rs:65-92` (drops "or a lost PLATE", M-5) |
| §1.1 | "verify the STEEL, not this output" | IMPLEMENTED — `blocks.rs:100-106` |
| §1.1 | `verify` REPORTS ITS MARGIN, localised, with before-values, descending | IMPLEMENTED — `main.rs:517-574` |
| §1.1 | `inspect` OWNS the report; `encode` CALLS it and APPENDS | IMPLEMENTED — `main.rs:397-416`, `report.rs:137-326` |
| §1.1 | ROW-PRESENCE: `mt1 SET` for inspect/decode/verify | **DIVERGES — C-4, M-2**; and the `verify` entry is a SPEC defect (S-1) |
| §1.1 | ROW-PRESENCE: `TX`/`OUT`/`FEE`/`LOCKTIME`/`INPUTS`/`STATUS` always | IMPLEMENTED for `encode`+`inspect` (`report.rs:271-326`); **DIVERGES for `decode` — C-4** |
| §1.1 | `FEE` carries the WEAKEST provenance inline | IMPLEMENTED — `report.rs:213-229,283-301` |
| §1.1 | three provenance classes, not two | IMPLEMENTED — `report.rs:36-73` |
| §1.1 | `LOCKTIME` row bound to §8.4's five spellings, may not invent a sixth | **DIVERGES — C-2, C-3, M-1** |
| §1.1 | a row is never omitted — it reads `UNKNOWN` | IMPLEMENTED — `report.rs:271-326`, test `inspect.rs:63` |
| §1.1 | `--transaction` compares the FULL 32-byte txid | IMPLEMENTED for hex — `main.rs:725-742` |
| §1.1 | a supplied PSBT is compared against its EXTRACTED transaction | **NOT IMPLEMENTED — I-7** |
| §1.1a | `decode` emits raw hex on **stdout** | IMPLEMENTED — `main.rs:666-681` |
| §1.1a | stdout stays empty unless every check passes; non-zero exit otherwise | **DIVERGES — C-1** (the last check does not exist) |
| §1.1a | `decode` PRINTS THE INSPECTION SUMMARY on stderr | **DIVERGES — C-4** (two lines, not the report) |
| §1.1a | `--quiet` suppresses the report only | IMPLEMENTED — `main.rs:652`, `main.rs:387` |
| §1.1e | engrave uppercase, accept anything; `encode` WRITES LOWERCASE | IMPLEMENTED — `pipeline.rs:41-43`, `read_strings.rs:33` |
| §1.1e | `--elide-prefix`: first full, rest drop 11 chars; detection needs no flag; mixed legal | IMPLEMENTED — `main.rs:429-452`, `read_strings.rs:67-110` |
| §1.1e | all-elided → refuse, naming the 8 characters needed | IMPLEMENTED — `read_strings.rs:70-87` |
| §1.1e | split FIRST then strip; split at each `mt1` within a line | IMPLEMENTED — `read_strings.rs:24-45` |
| §1.1e | **length check from the MODAL length, before decoding** | **NOT IMPLEMENTED — I-1** |
| §1.1e | autocorrect: never touch a string that parses | IMPLEMENTED — `pipeline.rs:172-174` |
| §1.1e | **positional autocorrect** (`l/I/i`→`1` at idx 2; `1/i`→`l`, `o`→`0`, `b`→`6` at 3+) | **NOT IMPLEMENTED — I-2** |
| §1.1e | positions 1-based over the whole string, `pos = codeword_index + 4` | IMPLEMENTED — `main.rs:559` (`p + 1 + 3`) |
| §3b | `count = ceil(len/40)`, `bytes_per_chunk = ceil(len/count)` — BALANCED | IMPLEMENTED — `chunk.rs:35-51`, pinned `chunk.rs:71-142` |
| §3b/§10.13a2 | header 55 bits: version 5 \| set_id 20 \| count−1 15 \| index 15 | IMPLEMENTED — `consts.rs:43-52`, `header.rs:69-116` |
| §10.13b | HRP is `"mt"`, NOT `"mt1"` | IMPLEMENTED — `consts.rs:15` |
| §3b | 40-bit invariant prefix = exactly 8 symbols | IMPLEMENTED — `consts.rs:62,116`, `header.rs:124-129` |
| §3b | the bearer warning on stderr at encode time | IMPLEMENTED — `blocks.rs:46-55`, `main.rs:382` |
| §0a | stdout is the strings and nothing else | IMPLEMENTED — `main.rs:418-424` |
| §0a | `encode` PRINTS the five suggested legend fields on stderr | **NOT IMPLEMENTED — I-3** |
| §5 | `NO TIMELOCK`, that exact spelling, 11 chars, normative everywhere | PARTIAL — `report.rs:309` only, and only at `nLockTime == 0` (C-3) |
| §5 | `LOCKED TO BLOCK <n> ~<SEASON> <year>` | **NOT IMPLEMENTED — C-2, I-6, M-1** |
| §5 | `LOCKED UNTIL <time>` | **NOT IMPLEMENTED — C-2** |
| §5 | `FORMAT: mt1 codex32`, `FROM WALLET`, `TO` | **NOT IMPLEMENTED — I-3** |
| §5 | `n/m` beside each engraved unit | NOT IMPLEMENTED (subsumed by I-3) |
| §6a | node consulted automatically, `bitcoin-cli -stdin`, args on stdin | IMPLEMENTED — `node.rs:66-86` |
| §6a | `gettxout <txid> <vout> false`, `include_mempool` = **false** | IMPLEMENTED — `node.rs:99-105` |
| §6a | use the VALUE, and refuse on mismatch naming both numbers | IMPLEMENTED — `validate.rs:515-533`, `main.rs:296-307` |
| §6a | FIVE liveness states, `SPENT — ALREADY CONFIRMED` asked FIRST | IMPLEMENTED — `report.rs:75-117,145-147,231-245` |
| §6a | DEAD needs the parent **confirmed**; UNKNOWN without `-txindex` | IMPLEMENTED — `node.rs:115-143`, `report.rs:169-184` |
| §6a | **no node is a WARNING at ENCODE time** | **NOT IMPLEMENTED — I-4** |
| §6a | the recovery-time no-node warning, both ways out | IMPLEMENTED — `report.rs:339-392`; `inspect` only (M-3) |
| §8 preamble | three-part refusal, machine-parseable verdict naming the number | IMPLEMENTED — `refusal.rs:63-82` |
| §8 preamble | `REFUSED` reserved for refusals; warnings use `WARNING:` | IMPLEMENTED — `refusal.rs:84-…` (distinct type) |
| §8.1 | not fully finalized (PSBT vocabulary) → refuse | IMPLEMENTED — `validate.rs:41-78` |
| §8.3 | not signed (raw vocabulary, scriptSig **or** witness) → refuse | IMPLEMENTED — `validate.rs:85-113` |
| §8.2b | inputs ≥ outputs; absurd fee 25,000 sat/vB; no dup outpoints; `vin` non-empty | IMPLEMENTED — `validate.rs:123-202` |
| §8.2b | **warning** below 10 sat/vB, never a refusal | IMPLEMENTED — `validate.rs:209-227` |
| §8.2c | require values a **PSBT** lacks, per input | IMPLEMENTED — `validate.rs:297-326` (M-7: no gate entry) |
| §8.2c | legacy warning fires **only when the value is bound by NOTHING** | **DIVERGES — I-5** |
| §8.2d | `non_witness_utxo` must hash to the input's txid | IMPLEMENTED — `validate.rs:242-264` |
| §8.2e | ordered sniffing: binary → strip → base64 → hex; step 4 names what it saw | IMPLEMENTED — `input.rs:36-138` |
| §8.2e | hex-encoded PSBT named as the real problem | IMPLEMENTED — `input.rs:81-91` |
| §8.2e | raw signed transaction ACCEPTED with a loud warning | IMPLEMENTED — `main.rs:367-379` (M-4: the `[with node]` branch) |
| §8.2f | transaction as a command-line argument → refuse, shell-specific purge | IMPLEMENTED — `validate.rs:374-427` |
| §8.2g | `mode & 0o077 != 0` → **warn**, never refuse | IMPLEMENTED — `validate.rs:440-475` |
| §8.4 | **never refuse** | IMPLEMENTED — no locktime refusal exists |
| §8.4 | `LOCK_TIME_THRESHOLD = 500_000_000` branch | **NOT IMPLEMENTED — C-2** |
| §8.4 | locktime enforced only if some input has `nSequence != 0xFFFFFFFF` | **NOT IMPLEMENTED — C-3** |
| §8.4 | `MT_REF_HEIGHT` / `MT_REF_TIME`, the season estimate | **NOT IMPLEMENTED — I-6** |
| §8.4 | negative-subtraction warning | **NOT IMPLEMENTED — I-6** |
| §8.4 | compare like with like — height vs height, timestamp vs **MTP** | **NOT IMPLEMENTED — C-2** (no MTP query exists) |
| §8.5 | `gettxout` null **AND parent confirmed** → refuse | IMPLEMENTED — `validate.rs:488-502`, `main.rs:308-316` |
| §8.6a | non-`ALL` sighash → refuse; `SIGHASH_DEFAULT` accepted | IMPLEMENTED — `validate.rs:700-725` |
| §8.6b | no signature at all → refuse; scriptSig **and** witness; control block by shape | IMPLEMENTED — `validate.rs:566-692` |
| §8.7b | over the 32,768-chunk ceiling → refuse, naming both numbers | IMPLEMENTED — `validate.rs:749-766`, `main.rs:327-331` |
| §8.9 | secrets → refuse, before §8.2e can echo them | IMPLEMENTED — `validate.rs:779-808`, `main.rs:208`, `main.rs:498` |
| §10.10 | twelve ruled flags, spellings included | **DIVERGES — I-3** (4 of 12 parse and do nothing) |
| §10.10 | exit 0 = every check passed | IMPLEMENTED — `main.rs:156-164` |
| §10.10 | TTY welcome line | IMPLEMENTED — `blocks.rs:21-37`, `main.rs:187-189` |
| §10.10 | unrecognised input NAMED — a txid is recognisable as such | **NOT IMPLEMENTED — M-6** |
| §10.10a | grouping opt-in, stdout only, canonical artifact ungrouped | IMPLEMENTED — `main.rs:440-449` |
| §10.20 | the legacy-malleability recovery caveat, "somewhere a recoverer will read" | **NOT IMPLEMENTED — M-8** |

---

## Findings

### [Critical] 1 — The content id is never re-derived. `verify` asserts that it was, and `decode`'s stdout gate rests on a check that does not exist.

**Spec says**

> §1.1: *"It checks: every string parses, every BCH checksum holds, the set is
> complete … every chunk carries the same `chunk_set_id`, **and the reassembled
> transaction re-derives that id**."*
>
> §1.1a, the required-steps table: *"**prove the result is the right transaction**
> | re-derive the content id from the decoded transaction and compare (§10.13 c)"*
> — and *"That last row is the one that matters."*
>
> §1.1a: *"**`decode` WRITES NOTHING TO STDOUT UNLESS EVERY CHECK IN THE TABLE
> ABOVE PASSES, and exits non-zero otherwise.**"*
>
> §10.13(c): *"Reassembly re-derives the id from the transaction it decoded and
> compares."*

**Code does**

- `crates/mt-codec/src/string_layer/pipeline.rs:234-359` — `decode()` reassembles
  `bytes` and returns. It never hashes them and never compares against
  `first.header.chunk_set_id`.
- `crates/mt-codec/src/string_layer/pipeline.rs:23` —
  `content_id_from_txid_display` exists, and its only non-test call site is
  `pipeline.rs:54`, inside **`encode`**.
- `crates/mt-codec/src/error.rs:119` — `Error::ContentIdMismatch` is declared,
  documented, and **never constructed anywhere in the crate**.
- `crates/mt-cli/src/main.rs:692-696` — `verify` prints
  `"mt verify: OK — {n} chunks, set {id:#07x}, transaction re-derives."`
  unconditionally, as soon as `pipeline::decode` returns `Ok`.
- `crates/mt-cli/src/main.rs:666-681` — `decode` writes the hex to stdout and
  returns `Ok`.
- §1.1's entire FAILED report — *"These chunks do not add up to the transaction
  they name… Most likely first — re-type these from the steel, in this order"* —
  has no branch in `main.rs` and is unreachable.

**Consequence**

`mt` prints the words *"transaction re-derives"* as a statement of fact about a
check it did not perform, and exits 0. The one property §1.1a calls *"the check
that the engraving round-trips at all"* is asserted rather than computed, so the
documented pipeline `mt decode < plates.txt > tx.hex && bitcoin-cli
sendrawtransaction …` broadcasts on an unverified reassembly. The failure this
guards is not hypothetical: §1.1 spends four paragraphs on mis-correction (>4
symbol errors landing on a different valid code word), and this is the only check
that would see it.

**How I checked**

Built a probe against `mt-codec` that encodes the pinned `even` vector's own bytes
under a deliberately wrong txid (`ffff…ff`), so every BCH checksum holds and only
the set id disagrees with the payload:

```
$ mt verify --in lied.txt
mt verify: OK — 6 chunks, set 0xfffff, transaction re-derives.
$ echo $?
0
$ mt decode --in lied.txt --quiet | head -c 40
020000000001017c8da925af70e49a12b0cea7   (exit 0)
```

The reassembled transaction's real txid is `2dcf2b97…` — top 20 bits `0x2dcf2`,
not `0xfffff`. Also confirmed by grep: `content_id_from_txid_display` and
`ContentIdMismatch` have no call/construction site on the reading path.

---

### [Critical] 2 — §8.4's `LOCK_TIME_THRESHOLD` branch does not exist: a timestamp `nLockTime` is reported as a block height.

**Spec says**

> §8.4: *"**`nLockTime` IS NOT ALWAYS A BLOCK HEIGHT, and an earlier version of
> this section assumed it was.** Verified against source: `LOCK_TIME_THRESHOLD:
> u32 = 500_000_000`. Below that value `nLockTime` is a **block height**; at or
> above it, a **Unix timestamp**. `mt` branches on the threshold before it
> compares anything or engraves anything."*
>
> And the named consequence: *"A transaction with `nLockTime = 1800000000` would
> have engraved `LOCKED TO BLOCK 1800000000` — a block number some thirty
> thousand years out, for a plate that actually unlocks in 2027. A holder could
> reasonably read that as 'never' and discard it."*
>
> §5: *"**A timestamp is never presented as a height.**"*
> §8.4: *"**Compare like with like:** a height against the chain height, a
> timestamp against the chain's **median-time-past**."*

**Code does**

`crates/mt-cli/src/report.rs:307-312` is the whole of `mt`'s locktime rendering:

```rust
let (lt, height) = self.locktime;
let _ = match (lt, height) {
    (0, _)       => writeln!(s, "LOCKTIME  NO TIMELOCK"),
    (n, Some(h)) => writeln!(s, "LOCKTIME  block {n}, current height {h}"),
    (n, None)    => writeln!(s, "LOCKTIME  block {n}, current height UNKNOWN"),
};
```

`grep -rn '500_000_000\|LOCK_TIME_THRESHOLD\|is_block_height\|is_block_time'` over
`crates/` returns **nothing**. `LOCKED UNTIL` appears nowhere in the crate, and
`node.rs` has no median-time-past query — only `getblockcount` (`node.rs:89-91`).

**Consequence**

The exact defect §8.4 was written to close is present in the shipped binary. A
time-locked transaction reports a thirty-thousand-year block number, and the
comparison the operator is supposed to make (`current height 963663`) is between a
Unix timestamp and a block height — so a plate whose time-lock has **already
passed** looks enormously distant. §8.4 names both halves: a permanent falsehood,
and false reassurance.

**How I checked**

Took the pinned `even` vector's raw transaction, rewrote its last four bytes to
`nLockTime = 1_800_000_000`, ran `mt encode --bitcoin-cli /nonexistent`:

```
LOCKTIME  block 1800000000, current height UNKNOWN
```

---

### [Critical] 3 — §8.4's `nSequence` rule does not exist: a transaction with every input final is reported as locked.

**Spec says**

> §8.4: *"**`nSequence` is not optional, and omitting it causes the dangerous
> error.** `nLockTime` is enforced only when at least one input has `nSequence !=
> 0xFFFFFFFF`. A transaction with every input final ignores its locktime — so
> reading `nLockTime` alone would engrave `LOCKED TO BLOCK 900000` on a plate
> anyone can broadcast today. That is **false reassurance on steel, the worst
> failure available here**."*
>
> §8.4's normative spellings include `nLockTime 900000 present but NOT ENFORCED
> (all inputs final)`, and: *"`NO TIMELOCK` is reserved for a transaction with
> `nLockTime = 0` **or with all inputs final**."*

**Code does**

`crates/mt-cli/src/report.rs:261-264` stores only
`(tx.lock_time.to_consensus_u32(), node.and_then(Node::block_count))` — the
per-input `nSequence` is never read. `report.rs:309` emits `NO TIMELOCK` on
`lt == 0` alone. `grep -rn 'sequence' crates/*/src` finds no reference to
`TxIn::sequence` anywhere in `mt-cli`.

**Consequence**

`mt` states a lock that consensus will not enforce, in the words the operator is
told to engrave, for a transaction any holder can broadcast immediately. §8.4
calls this the worst failure available here, and it is the one the `nSequence`
rule was added to close.

**How I checked**

Took the same vector transaction, rewrote its single input's `nSequence` from
`fdffffff` to `ffffffff` (all inputs final, `nLockTime` still 96), and ran
`mt encode --bitcoin-cli /nonexistent`:

```
LOCKTIME  block 96, current height UNKNOWN
```

Expected under §8.4: `NO TIMELOCK`, or `nLockTime 96 present but NOT ENFORCED
(all inputs final)`. See spec defect **S-2** on which of the two — the spec names
both for this state.

---

### [Critical] 4 — `decode` does not print §1.1's report. It prints two lines of its own composition.

**Spec says**

> §1.1a: *"**`decode` PRINTS THE INSPECTION SUMMARY ON `stderr`, and does not stay
> silent.** … A silent `decode` therefore hands a stranger sixty kilobytes of hex
> — a bearer instrument, in the single most broadcastable form that exists —
> **before anything has told them what it does**. The next command they type is
> plausibly `sendrawtransaction`, and the first thing they learn about the
> destination, the amount and the locktime is whatever the chain does with it. So
> `decode` emits **§1.1's `inspect` report on `stderr`**"* — and the guarantee it
> buys: *"no path through this tool broadcasts a transaction the operator was
> never shown."*
>
> §1.1: *"`inspect` prints it on **stdout**; `decode` prints it on **stderr**
> beside the hex; `encode` prints it on **stderr** before the strings … **No
> caller reorders, reformats, or drops a row.**"*
>
> §1.1's row-presence table makes `TX`, `OUT`, `LOCKTIME`, `INPUTS` and `STATUS`
> **always** present.

**Code does**

`crates/mt-cli/src/main.rs:652-665` — `decode` never constructs a
`report::Report`. It writes two hand-composed lines:

```rust
let _ = writeln!(stderr, "TX        {}", txid_display(&set.bytes)?);
let _ = writeln!(stderr, "mt1 SET   {} strings, all present", set.chunks.len());
```

`decode` also never calls `node::Node::find` (the only two call sites are
`main.rs:288` in `encode` and `main.rs:765` in `inspect`), so it has no chain rows
and never prints §6a's no-node warning either.

**Consequence**

The recoverer §1.1a is written for — the one who reaches for `decode` because it
is the obvious verb — is shown a txid and a chunk count, and **not** the
destination, the amount, the locktime, the fee or the liveness status. That is
precisely the state §1.1a describes and rules against, and the stated guarantee
("no path … broadcasts a transaction the operator was never shown") is unmet. The
second line is also a second implementation of the `mt1 SET` row in a different
format from `report.rs:274` — the drift the single-owner rule exists to prevent.

**How I checked**

```
$ mt decode --in good.txt > /dev/null
TX        2dcf2b973d52044b1e58c988a5a59d388073ff05598b0a1e93eeb04c72ebf630
mt1 SET   6 strings, all present
```

versus `mt inspect --in good.txt` on the same set, which prints `mt1 SET`, `TX`,
`OUT` (two outputs with addresses and amounts), `FEE`, `LOCKTIME`, `INPUTS`,
`STATUS`. The P4 gate that would have caught this is
`crates/mt-cli/tests/inspect.rs:161 encode_and_inspect_agree_on_the_rows_they_share`
— it compares **encode and inspect only**; `decode` is not in it, and
`decode_verify.rs:249-250` asserts the two-line form as correct.

---

### [Important] 1 — §1.1e's length check is not implemented, and the fallback message accuses the operator of a missing plate.

**Spec says**

> §1.1e: *"**Every string in a set has a KNOWN length, checked before decoding**,
> because it catches the one damage class BCH cannot."*
>
> *"**AT DECODE TIME THE EXPECTED LENGTH COMES FROM THE STRINGS THEMSELVES — the
> MODAL length across the set.**"*
>
>       string 7: 89 characters (expected 90) — a character is MISSING, not
>                 wrong. BCH repairs substitutions; an omission shifts every
>                 symbol after it and cannot be corrected. Re-read the plate.
>
> §1.1's damage table: a missing or extra character is caught by *"the **length
> check** (decision 1e in §1)"*; a **missing string** is caught by `count`; a
> **lost plate** by *"nothing. The transaction is gone"*.

**Code does**

`grep -rni modal crates/` returns nothing. `pipeline.rs:160-169` checks only a
**minimum** (`HEADER_SYMBOLS + CHECKSUM_SYMBOLS`); no per-set expected length is
computed anywhere. A short string therefore fails BCH, lands in
`DecodedSet::unreadable` (`pipeline.rs:249-258`), and — when no duplicate covers
that index — `pipeline.rs:340` returns `Error::MissingChunk`, **discarding the
`unreadable` list with its reason and its 1-based input position**.

**Consequence**

An operator who drops one glyph while typing 1,242 characters back from steel is
told `chunk 3 of 6 is missing`. Per §1.1's own damage table a missing string means
the plate is gone and the transaction is unrecoverable — so the tool's message
points at the one failure mode the spec calls terminal, for the one that is a
five-second retype. Nothing in the output names which of the input strings could
not be read, or why.

**How I checked**

Deleted a single character from string 3 of the pinned `even` vector:

```
$ mt verify --in short.txt
mt verify: REFUSED — §1.1, the set does not verify

  chunk 3 of 6 is missing
```

---

### [Important] 2 — §1.1e's positional autocorrect is not implemented; the excluded-character cases fail with the same false "missing chunk" message.

**Spec says**

> §1.1e: *"**Correction is POSITIONAL, because `mt1` has a fixed HRP.**"* — with
> the normative table: index 2, `l`/`I`/`i` → `1`; index 3+, `1`/`i` → `l`,
> `o` → `0`, `b` → `6`.
>
> *"The prefix case matters most: **every string a person types contains the
> single most confusable glyph in the set**, and `mtl…` or `mtI…` does not merely
> fail its checksum — it has no separator and will not parse at all."*
>
> *"**Autocorrect announces itself, localises, and states its verdict.** Operator
> ruling: never silently."* — with the two message forms
> (`corrected 'o' -> '0' at position 41. Checksum now valid.` and
> `… Checksum STILL INVALID.`).

**Code does**

`crates/mt-codec/src/string_layer/pipeline.rs:65-81` — `to_symbols` maps each
character through `ALPHABET` and returns `Error::InvalidChar` for anything absent.
No substitution table, no position-dependent rule, no announcement. There is no
`'o' => '0'` mapping anywhere in the crate, and `read_strings.rs` does only
splitting, whitespace stripping, lowercasing and prefix restoration.

**Consequence**

Both cases §1.1e names produce the same wrong diagnosis as I-1 — a claim that a
plate is missing when the operator is holding it. The spec's argument that
correcting excluded characters *"costs nothing from the `t = 4` budget"* is
therefore unrealised: an `o` or a `b` in the data part is not a repaired symbol,
it is an unreadable string.

**How I checked**

```
$ mt verify --in mtl.txt      # string 1 typed as `mtl…` instead of `mt1…`
mt verify: REFUSED — §1.1, the set does not verify

  chunk 1 of 6 is missing

$ mt verify --in oh.txt       # an `o` substituted at one data position
mt verify: REFUSED — §1.1, the set does not verify

  chunk 1 of 6 is missing
```

---

### [Important] 3 — The suggested legend text is never printed, and four of §10.10's twelve ruled flags parse and do nothing.

**Spec says**

> §0a: *"**`mt encode` therefore PRINTS suggested legend text on `stderr`**, which
> the operator may engrave beside their strings."* — split as: printed **once**,
> `BEARER…`, `FROM`, `TO`, `LOCKED TO BLOCK n ~SEASON year`, `FORMAT: mt1
> codex32`; printed **per string**, `n/m`.
>
> §5 on `FORMAT: mt1 codex32`: *"the only field a recoverer cannot do without …
> **What no amount of inspection recovers is which program to run.**"*
>
> §10.10's input table: `FROM` wallet id → *"absent → warn, engrave blank"*;
> `TO` → same; §10.10's flag table rules `--from`, `--to`, `--to-label`, `--json`
> (*"machine-readable report. `md` has it"*).

**Code does**

- `crates/mt-cli/src/main.rs:85-99` declares `from`, `to`, `to_label`;
  `main.rs:63-65` and `main.rs:131-133` declare `json`.
  `grep -rn 'args\.from\|args\.to\|to_label\|args\.json' crates/mt-cli/src/`
  returns **only the declarations** — no read site exists for any of the four.
- `grep -rn 'FORMAT: mt1\|FROM WALLET\|LOCKED TO BLOCK' crates/` returns nothing.
- The only per-string identifier `encode` emits is the `PREFIX` row
  (`main.rs:409-414`); there is no `n/m` label.
- No warning fires when `--from` / `--to` are absent.

**Consequence**

A v0.1 plate carries the string and nothing else — which §0a anticipated — but
`mt` also *withholds the words*, which §0a explicitly rules it does not: *"`mt`
does not control the layout and does not withhold the words."* An operator who
supplies `--from`/`--to` gets silence, and `--json` succeeds while emitting prose,
which is worse for a caller than the flag not existing.

**How I checked**

```
$ mt encode --in tx.hex --from AABBCCDD --to deadbeef --bitcoin-cli /nonexistent 2>&1 >/dev/null
# stderr: file-mode warning, raw-transaction warning, BEARER, correction coverage,
#         verify-the-steel, then the report + CUT + PREFIX. No legend block.
#         AABBCCDD and deadbeef appear nowhere in the output.

$ mt inspect --in good.txt --json --bitcoin-cli /nonexistent 2>/dev/null | head -2
mt1 SET   6 strings, 1..6 all present
TX        2dcf2b97…
```

---

### [Important] 4 — §6a's encode-time no-node warning is not implemented.

**Spec says**

> §6a: *"**NO NODE IS A WARNING, NOT A SILENCE.** Operator ruling 2026-08-23:
> *'bitcoind might not be available and we need a warning for that.'* An earlier
> draft made every check in this section conditional on a node being reachable and
> said nothing when one was not — so the quietest possible run was also the
> least-verified one, and the operator could not tell the difference."*
>
>       WARNING: no bitcoind reachable. These checks did NOT run:
>         - are the inputs still unspent?        (§8.5)   UNKNOWN
>         - do the PSBT's input values match …   (§6a)    UNKNOWN
>         - has the locktime already passed?     (§8.4)   UNKNOWN
>       … Consider re-running with a node before you start.
>
> The §1.1a note distinguishes this **encode-shaped** wording from the
> recovery-time one, because *"before cutting"* names a decision a recoverer made
> years ago.

**Code does**

`crates/mt-cli/src/main.rs:288-319` — `encode` calls `Node::find` and, when it
returns `None`, simply skips the loop. There is no `else` and no warning.
`report::no_node_warning` (`report.rs:339-392`) is the recovery-time wording and
its only call site is `main.rs:775-783`, inside **`inspect`**.

**Consequence**

The quietest `mt encode` run is the least-verified one, which is the state §6a
rules against by name. The operator gets `FEE UNKNOWN` and `STATUS UNKNOWN` rows
with no statement that the inputs were never checked for spentness and no
suggestion to re-run with a node — the one moment before an evening of engraving
where that advice is actionable.

**How I checked**

`mt encode --in tx.hex --bitcoin-cli /nonexistent 2>&1 >/dev/null` — full stderr
transcript captured; it contains no string matching `no bitcoind`, `did NOT run`
or `§8.5`. Same command with a PSBT fixture: same result.

---

### [Important] 5 — §8.2c's legacy warning fires on a value §8.2d has just bound, and prints a false statement while doing it.

**Spec says**

> §8.2c: *"**The legacy warning fires only when the value is UNBOUND** — not on
> every legacy input. R3's information lens found the earlier rule actively
> harmful: it fired *'whenever any input is legacy'* while its body asserted `mt`
> could not bind the value by txid, **which §8.2d now does**. In the common case —
> a legacy input carrying `non_witness_utxo`, which BIP-174 requires — that
> printed a false, capitalised, eleven-line block, **training the operator to
> ignore the rare case where it is true.** … So it fires when, and only when, the
> value is bound by nothing: no `non_witness_utxo` (§8.2d), no chain fetch (§6a)."*

**Code does**

`crates/mt-cli/src/main.rs:349-360`:

```rust
let legacy = inp.witness.is_empty();
if legacy && !bound_by_chain[n] {
    if let Some((claimed, _)) = values[n] {
        … validate::legacy_unbound_warning(n, claimed, out_total) …
```

The provenance is discarded (`(claimed, _)`), so a value carrying
`Provenance::TxidBound` — set at `main.rs:245-256` precisely because
`non_witness_utxo` is present and `validate::non_witness_utxo_guard` has hashed it
— trips the warning identically to an operator-asserted one. The warning body
(`validate.rs:345-347`) then states: *"This input carries no non_witness_utxo, so
mt could not bind it by txid (see 8.2d)"*.

**Consequence**

The deleted behaviour is back verbatim: the common, BIP-174-mandated path prints
a false capitalised block on every run, which is exactly the cry-wolf failure the
ruling removed. And the block asserts something the tool has already disproved two
checks earlier — the operator is told the value is bound by nothing while the
report's `INPUTS` row for the same input reads `TXID-BOUND`.

**How I checked**

Hand-built a PSBT with one legacy input (empty witness, `final_scriptSig` carrying
a DER-shaped signature) spending the fixture's `legacy_parent_hex`, with a matching
`non_witness_utxo`, and encoded it offline:

```
$ mt encode --in legacy.psbt.b64 --bitcoin-cli /nonexistent 2>&1 >/dev/null | head -14
WARNING: input 0 is a legacy (pre-SegWit) input.
  …
  NOTHING HAS VERIFIED THAT VALUE. This input carries no
  non_witness_utxo, so mt could not bind it by txid (see 8.2d), …
```

The `10.00000000 BTC` it quotes was read *from* that `non_witness_utxo`
(`validate.rs:281-285`) after `non_witness_utxo_guard` matched its hash to the
input's txid.

---

### [Important] 6 — §8.4's embedded reference pair, the season estimate and the negative-subtraction warning do not exist.

**Spec says**

> §8.4: *"**The block height is MANDATORY, and the estimate names a SEASON.** …
> `LOCKED TO BLOCK 1383520 ~FALL 2034`"*
>
>       MT_REF_HEIGHT = 963_759
>       MT_REF_TIME   = 1_787_507_701   // 2026-08-23T17:55:01Z
>
> *"**The estimate uses the embedded constant, and ONLY the embedded constant.**"*
>
> *"**A NEGATIVE subtraction means the lock is already behind us — warn.**"* —
>
>       WARNING: nLockTime 900000 is BELOW this build's reference height 963759.
>                This transaction is not meaningfully time-locked …

**Code does**

`grep -rni 'MT_REF\|963_759\|963759\|787507701\|SEASON\|SPRING\|SUMMER\|WINTER'`
over `crates/` returns nothing. There is no reference constant, no `600 s`
projection, no `~<SEASON> <year>` rendering and no below-reference-height warning
anywhere in the workspace.

**Consequence**

*"A height means nothing to a human"* is §8.4's own statement of the problem this
closes; the operator gets the raw height and no orientation. And a transaction
whose lock height is already behind the build's reference passes silently, where
§8.4 requires *"Treat it as spendable now."*

**How I checked**

Rewrote the vector's `nLockTime` to 900,000 (below `MT_REF_HEIGHT` 963,759) and
ran `mt encode --bitcoin-cli /nonexistent`: stderr carried
`LOCKTIME  block 900000, current height UNKNOWN` and nothing else about the lock.
No warning fired.

---

### [Important] 7 — `--transaction` refuses the PSBT half of its own ruling.

**Spec says**

> §1.1: *"**Optionally, `--transaction <psbt|hex>`** — the sibling round-trip …
> **A supplied PSBT is compared against its EXTRACTED transaction**, per §10.13(c)
> — the same resolution that section already made, for the same reason: a PSBT
> holds two transactions whose txids differ for every legacy and `sh(wsh(…))`
> input, so leaving the basis unstated lets `--transaction` report a **mismatch on
> the correct transaction**."*
>
> §10.10's flag table: `--transaction <psbt|hex>` | on `verify`.

**Code does**

`crates/mt-cli/src/main.rs:713-724` — the `Input::Psbt` arm returns a refusal:
*"PSBT comparison lands with PSBT support / … Extraction arrives with the rest of
§8."* PSBT support and extraction both landed in P5 (`main.rs:220-265` calls
`extract_tx_unchecked_fee_rate`), so the deferral message is stale as well as the
behaviour.

**Consequence**

Half the ruled flag surface is unavailable, and the refusal blames a phase that has
shipped. An operator holding the PSBT their wallet exported — the ordinary artifact
at this point in the workflow — cannot use `--transaction` at all.

**How I checked**

```
$ mt verify --in good.txt --transaction fin.psbt.b64
mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.
mt verify: REFUSED — §1.1, PSBT comparison lands with PSBT support
  A supplied PSBT is compared against its EXTRACTED transaction
  (§10.13 c). Extraction arrives with the rest of §8.
(exit 1)
```

---

### [Minor] 1 — The `LOCKTIME` row invents a sixth spelling.

§1.1 binds the row to *"§8.4's five normative spellings, by reference — this row
may not invent a sixth"*. `report.rs:310-311` emits `block {n}, current height
{h}` / `block {n}, current height UNKNOWN`, where §8.4's spellings are `LOCKED TO
BLOCK 1383520          current height 963663` and `LOCKED TO BLOCK 900000
current height unknown (no node)`. §5 records why spelling drift is not a style
question: *"two `mt` versions would cut different plates for the same
transaction, and a recoverer matching against documentation would find neither."*
Checked by rendering both branches (`mt inspect` offline, and the `(n, Some(h))`
arm by reading).

### [Minor] 2 — The `mt1 SET` row drops the set id, and `decode` prints a second format of it.

§1.1's normative layout block is `mt1 SET   0x0e17e    14 strings, 1..14 all
present`. `report.rs:273-275` emits `mt1 SET   6 strings, 1..6 all present` —
`Report::set` is `Option<(usize, usize)>` (`report.rs:124`) and carries no id — and
`main.rs:658-662` emits a third wording, `mt1 SET   6 strings, all present`. The
block declares itself *"the only place the layout appears"* and ends *"No caller
reorders, reformats, or drops a row."* Checked by running `mt inspect` and
`mt decode` on the same set.

### [Minor] 3 — The offline warning says "locked to block 0" for a transaction whose own row reads `NO TIMELOCK`.

`report.rs:363-365` interpolates the locktime unconditionally. On a transaction
with `nLockTime = 0`, `mt inspect` prints `LOCKTIME  NO TIMELOCK` on stdout and
`locked to block 0, current height unknown` on stderr, in the same run. Two
statements about one field, contradicting each other — the class §8.4 declared a
defect. Reproduced with a locktime-0 variant of the pinned vector.

### [Minor] 4 — The raw-transaction warning ignores §8.2e's `[with node]` branch.

§8.2e gives the warning two forms: `[no node] The fee is UNKNOWN…` and
`[with node] mt fetched each input's value from the chain: fee 0.00012 BTC, 3.2
sat/vB.` `main.rs:367-379` prints the no-node text unconditionally whenever the
input was raw hex — including after `main.rs:290-319` has fetched every value from
the chain, so the warning says *"mt cannot see any input's value and cannot check
the fee"* immediately above a `FEE` row that shows it. Traced by reading; the
warning is outside the `if let Some(nd) = &node` block and has no node predicate.

### [Minor] 5 — The correction-coverage block drops "or a lost PLATE".

§1.1's normative text: *"It cannot repair a missing STRING or a lost PLATE. There
is no redundancy…"*, and the damage table's fourth row is *"a lost plate | no |
**nothing. The transaction is gone**"*. `blocks.rs:84-87` prints *"It cannot repair
a missing STRING either."* The plate is the unit the operator physically loses, and
it is the row with no mitigation.

### [Minor] 6 — §10.10's txid classification is not implemented.

§10.10: *"**Unrecognised input is NAMED, not merely rejected.** … A txid is 64 hex
characters and recognisable as such: `mt encode: that is a transaction ID (a
64-character hash), not a transaction.`"* `input.rs:69-92` accepts any even-length
hex as `RawHex`, so a pasted txid reaches `decode_tx` and refuses with the generic
*"input is not a decodable Bitcoin transaction … The bytes are valid hex but do not
parse as a transaction"* (`main.rs:459-473`). No length-64 branch exists.

### [Minor] 7 — §8.2c's refusal has no entry in `refusals.toml`, so neither gate covers it.

`validate::require_psbt_input_values` (`validate.rs:297-326`) implements §8.2c's
*"where a UTXO record is absent from a PSBT, `mt` requires the operator to supply
that input's value"*, and §10.10's input table rules it `absent → refuse`. It has
no `[[refusal]]` entry in `crates/mt-cli/tests/refusals.toml` and no test in
`refusals.rs`, so the exhaustiveness bijection does not demand one and
`mutate-refusals.sh` never proves the check is load-bearing. The refusal itself
works; the gate around it does not exist.

### [Minor] 8 — §10.20's malleability caveat is carried nowhere.

§10.20: *"if a malleated version confirms first, the confirmed txid will not match
the plate's — the plate is not wrong, it is superseded … Worth a sentence somewhere
a recoverer will read."* `grep -rni 'malleab\|superseded' crates/` finds only an
unrelated comment in `consts.rs:60`. Neither `inspect` nor `decode` says it.

### Nit

`report.rs:278,314` render `OUT       2 output(s)` and `INPUTS    1 input(s)`;
§1.1's layout block reads `OUT       1 output` and `INPUTS    1 input`.

---

## Spec defects (the spec is ambiguous, not the code)

### [S-1] §1.1's row-presence table names `verify` as a report caller; §1.1's own `verify` example does not.

The table row reads *"`mt1 SET` | the caller had strings — `inspect`, `decode`,
`verify`"*, and `TX`/`OUT`/`FEE`/`LOCKTIME`/`INPUTS`/`STATUS` are marked
**always**. But the same section's worked `verify` output is only
`mt verify: OK — 14 chunks, set 0x0e17e, transaction re-derives.` plus the margin
block, and the per-caller note names three streams — *"`inspect` prints it on
stdout; `decode` prints it on stderr beside the hex; `encode` prints it on stderr
before the strings"* — with no `verify` entry. The code (`main.rs:684-745`) follows
the worked example and prints no report rows. **Two readings are supported; the
implementation picked the one the example shows.** This needs a ruling, not a code
change, and it should say so in one place. (Note the tension with §1.1's own
statement that `verify` is structural only and never asks a node: a full report
from `verify` would carry `FEE` and `STATUS` rows that are chain-derived.)

### [S-2] §8.4 gives one state two normative spellings.

For a transaction with a non-zero `nLockTime` and every input final, §8.4's report
block lists `nLockTime 900000 present but NOT ENFORCED (all inputs final)`, while
forty lines below it rules *"`NO TIMELOCK` is reserved for a transaction with
`nLockTime = 0` **or with all inputs final**."* One reading resolves it — the
`NO TIMELOCK` sentence sits in the paragraph about the **legend**, the `NOT
ENFORCED` line in the list of **`stderr` report** spellings — but that split is
never stated, and §1.1 binds the report's `LOCKTIME` row to *"§8.4's five normative
spellings"* as a single set. This is the same two-spellings-one-input class §8.4
itself calls a real defect (R6 implementability I-8). C-3 stands either way: the
code emits neither spelling.

---

## What I checked and found CONFORMANT

Traced into source and, where behavioural, exercised against the built binary.

**The wire format (§3 / §3b / §10.13).** `HRP = "mt"` not `"mt1"`
(`consts.rs:15`). The 55-bit header, `version 5 | chunk_set_id 20 | count−1 15 |
index 15`, every field a whole number of symbols, no `chunked` bit
(`consts.rs:43-52`, `header.rs:69-116`), with compile-time assertions at
`consts.rs:104-116`. `count` stored minus one, asserted directly rather than only
via a round trip (`header.rs:156-166`). The chunking rule **balanced, not filled** —
`count = ceil(len/40)`, `bytes_per_chunk = ceil(len/count)`, last takes the
remainder (`chunk.rs:35-51`), pinned against every artifact in §3b's table and
against a discriminating test that a filling chunker would fail
(`chunk.rs:96-142`). The 40-bit invariant prefix is exactly 8 symbols
(`consts.rs:62,116`), `--elide-prefix` drops exactly 11 characters
(`main.rs:430`), first string full, mixed input legal, all-elided refused by name
(`read_strings.rs:67-110`). NUMS constant derived from
`"shibbolethnumstransaction"` and asserted distinct from both siblings
(`consts.rs:126-156`).

**§1.1's reporting mechanics.** One-based chunk numbers everywhere in output, with
the wire `index` appearing in no message. Position arithmetic `codeword_index + 4`
(`main.rs:559`). The margin report: counts against `t = 4`, `<-- NO MARGIN LEFT`
at the limit, positions **and** before-values, descending order
(`main.rs:517-574`). Duplicate resolution over `n` candidates on post-correction
bytes, keeping the healthier copy, announcing which was discarded, and refusing on
two distinct valid payloads without a vote (`pipeline.rs:304-334`,
`main.rs:590-620`). `encode` calls the report rather than composing one, and
appends `CUT` and `PREFIX` **below** `STATUS` (`main.rs:397-416`). Three
provenance classes with `is_verified()` asked once (`report.rs:36-73`), and the
`FEE` row carrying the weakest provenance inline (`report.rs:213-229`).

**§1.1a's stdout discipline.** Raw hex on stdout, one line, nothing else; stdout
empty on every refusal path that exists; `--quiet` suppresses the report and
nothing else; exit 0 means every check passed (`main.rs:156-164`).

**§6a's node access.** Shelling out to `bitcoin-cli -stdin` with every argument on
stdin, never on the command line (`node.rs:66-86`). `gettxout <txid> <vout>
false` — `include_mempool` literally false (`node.rs:99-105`). The value used, not
merely its null-ness, with a refusal naming both numbers on mismatch
(`validate.rs:515-533`). Five liveness states with `SPENT — ALREADY CONFIRMED`
asked **first** (`report.rs:145-147,231-245`); `DEAD` requiring the parent
**confirmed** rather than merely found, `InMempool` → PENDING, `NotFound` without
`-txindex` → its own indeterminate state rather than DEAD (`node.rs:115-143`,
`report.rs:169-184`). `LIVE` qualified as *"unspent in the UTXO set (mempool not
consulted)"*. The recovery-time no-node warning naming both a node and a block
explorer, and printing the **txid** rather than the wtxid (`report.rs:339-392`,
`main.rs:459-473`).

**§8 in full.** §8.1 finalization by PSBT vocabulary and §8.3 by raw vocabulary,
with the `scriptSig` **or** witness disjunction that §10.16's legacy acceptance
needs. §8.2b's four checks under one number — `vin` non-empty, no duplicate
outpoints, inputs ≥ outputs, and `AbsurdFeeRate` at rust-bitcoin's own
`DEFAULT_MAX_FEE_RATE = 25,000` — with the value checks **skipped** rather than
guessed when a value is unknown, so §8.2e's *"never refuses the bytes"* holds.
§8.2b's sub-10-sat/vB **warning**, naming CPFP and out-of-band submission. §8.2c's
per-input requirement with the *"from a PSBT"* scoping, and `--input-value`
refusing a bare total. §8.2d hashing `non_witness_utxo` to the input's txid, with
`psbt_input_value` preferring the bound record over `witness_utxo`
(`validate.rs:278-287`) so a `TXID-BOUND` label is never applied to an unchecked
number. §8.2e's ordered procedure — binary magic tested **before** interior
whitespace is stripped, then base64, then hex with optional `0x` and either case —
and the hex-encoded-PSBT ambiguity refused by its real name. §8.2f refusing a
transaction on the command line **before reading any input**, never echoing the
argument back, with a `$SHELL`-specific purge command. §8.2g warning on
`mode & 0o077 != 0` including through a redirect, and staying silent on a FIFO or
TTY rather than guessing. §8.4 **never refuses** — no locktime refusal exists.
§8.5 requiring both facts (`null` **and** parent confirmed). §8.6(a) accepting
`SIGHASH_ALL` and taproot's `SIGHASH_DEFAULT` and refusing the rest by name, and
§8.6(b) requiring a signature-shaped element in **`scriptSig` and witness alike**,
with the annex stripped and the taproot script-path control block and leaf script
excluded from the count — the 65-byte control-block ambiguity the RCW fixture
proves. §8.7b naming both the count and the ceiling. §8.9 refusing `ms1` **before**
§8.2e's step-4 refusal can echo any of it, and never interpolating the input.
§8's refusal format: `mt <verb>: REFUSED — §<ref>, <reason with the number>`, three
parts, remedy omitted when there is nothing, and `Warning` a distinct type so a
warning cannot be printed in the shape that means "this stopped".

**§0a / §3b's stream boundary.** stdout carries the strings and nothing else;
grouping is opt-in and never the default; the artifact is lowercase and ungrouped;
the TTY welcome line fires only when stdin is a terminal.
