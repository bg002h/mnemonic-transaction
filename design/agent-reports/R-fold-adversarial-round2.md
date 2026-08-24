# R-fold-adversarial-round2 — independent adversarial review of the four folds

Scope: `git diff fb7c217..HEAD` (folds `dfc981c`, `95bad20`, `1df6614`, `7d30fb8`
and their merges), HEAD = `268b354`. Reviewer wrote none of this. Every finding
below was reproduced against `./target/debug/mt` built from HEAD; nothing is
reported from reading alone.

Scratch material used (constructed by me, not from the repo): byte-surgery
locktime variants of `p5_base.json`'s `raw_hex`, four hand-built legacy PSBTs,
a forged `mt1` set produced by calling `mt_codec::string_layer::pipeline::encode`
with transaction B's bytes under transaction A's txid, and a verbatim copy of
`locktime.rs`'s date arithmetic compiled standalone for differential testing.

## Verdict

NOT SAFE — 2C / 3I / 8M (three of the Minors are pre-existing, tagged inline).

## Findings

### [Critical] 1 — §8.2f refuses `--in <file>` when the FILE NAME starts with `mt1` and is 40+ characters

**Where** `crates/mt-cli/src/validate.rs:505` (`looks_like_a_transaction`, the
`mt1` branch — new in `95bad20`), reached from `main.rs:174`
(`command_line_guard`, which now runs before clap).

**What** The new branch is `lower.starts_with("mt1") && lower.len() >= 40`. It has
no charset constraint, so it matches any argument at all — including the path
handed to `--in`, `--transaction` or `--bitcoin-cli`. The boundary is exactly 40
characters: a 39-character name passes, a 40-character name is refused.

**Why it costs money, a recovery, or is false** This is the recovery path. An
operator holding steel, typing their strings into a file and running
`mt verify --in mt1-2026-08-23-cold-storage-transfer.txt`, is stopped with

> `mt verify: REFUSED — §8.2f, an mt1 set was passed as a command-line argument (40 characters)`

No mt1 set was passed — the statement in the verdict line is false. The refusal
then instructs them to purge their shell history for a filename that is not
bearer material, and its own mechanism paragraph says *"mt reads from a FILE or
STDIN only"* while refusing a `--in FILE`. A guard that blocks the exact usage it
tells the operator to adopt is worse than the silence it replaced. It also fires
on `mt encode --in mt1-source-psbt-for-the-cold-transfer.psbt` (42 chars), and on
any path under a directory literally named `mt1/` once the whole string reaches
40 characters.

The hex branch beside it is deliberately narrow (`len >= 100 && len % 2 == 0 &&
all ascii_hexdigit`) and does not have this problem. The `mt1` branch dropped
that discipline.

**How I reproduced it**
```sh
cd /scratch/code/shibboleth/mnemonic-transaction
cargo build
cd <scratch>
./mt encode --in final.psbt --bitcoin-cli /nonexistent > strings.txt 2>/dev/null
cp strings.txt mt1-2026-08-23-cold-storage-transfer.txt   # 40 chars
cp strings.txt mt1-2026-08-23-cold-storage-transfe.txt    # 39 chars
./mt verify --in mt1-2026-08-23-cold-storage-transfer.txt --bitcoin-cli /nonexistent
#  -> mt verify: REFUSED — §8.2f, an mt1 set was passed as a command-line argument (40 characters)
./mt verify --in mt1-2026-08-23-cold-storage-transfe.txt  --bitcoin-cli /nonexistent
#  -> mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.
cp final.psbt mt1-source-psbt-for-the-cold-transfer.psbt
./mt encode --in mt1-source-psbt-for-the-cold-transfer.psbt --bitcoin-cli /nonexistent
#  -> mt encode: REFUSED — §8.2f, an mt1 set was passed as a command-line argument (42 characters)
```

**Suggested fix (non-authoritative)** Require the body to be bech32: an argument
is an `mt1` set only if every character after `mt1` is in
`qpzry9x8gf2tvdw0s3jn54khce6mua7l`. A real string satisfies that (the shortest
this build emits is 83 characters); `mt1-2026-…-transfer.txt` fails on the first
`-`. Optionally also skip the argument immediately following a path-taking flag.
The defect is the missing charset test, not the length threshold — do not fix it
by raising 40, since a longer filename still collides.

---

### [Critical] 2 — the SUGGESTED LEGEND attributes the sum of ALL outputs, change included, to the named `TO` wallet

**Where** `crates/mt-cli/src/blocks.rs:124-175` (`legend`, entirely new in
`95bad20`), called from `main.rs:551-560` with
`out_total = tx.output.iter().map(|o| o.value.to_sat()).sum()`.

**What** `legend` takes one amount — the total of every output — and prints it on
the `TO` line beside the destination wallet the operator named. Any transaction
with a change output therefore engraves a number that is larger than what goes to
that wallet, by exactly the change.

**Why it costs money, a recovery, or is false** This is the text `mt` tells the
operator to cut into permanent steel, introduced as *"the five facts a stranger
needs BEFORE they can do anything with the steel"*. On a 2-output transaction
paying 2.0 BTC to a destination and returning 5.999 BTC as change, the legend
reads `TO alice-cold  7.99900000 BTC` — four times the amount that reaches
alice-cold. The report three lines above it lists both outputs correctly, so
`mt`'s own screen contradicts the plate it is proposing. The spec's §5 table
specifies the field as `TO <wallet id, fp or label>  <amount>` and never says
*which* amount, so this reading is the implementation's own choice, unreviewed;
§9 refuses fiat figures on steel for precisely this reason — a permanent number
that can be wrong. It happens to be right only for a no-change sweep.

The same total is printed on the `(None, Some(label))` and `(None, None)`
branches, so the misattribution survives when the destination is a free-text
label or absent.

**How I reproduced it** (`twoout.hex` = the fixture raw transaction with its
single output split into 2.0 BTC + 5.999 BTC change, both to the same script)
```sh
./mt encode --in twoout.hex --bitcoin-cli /nonexistent \
    --from ABCD1234 --to alice-cold --input-value 0:3 --input-value 1:5 2>&1 >/dev/null
# OUT       2 output(s)
#             bc1q07h88…   2.00000000 BTC
#             bc1q07h88…   5.99900000 BTC
# ...
#     TO alice-cold  7.99900000 BTC
```

**Suggested fix (non-authoritative)** `mt` cannot identify change, so it must not
imply it has. Either label the number for what it is (`TO alice-cold   7.99900000
BTC TOTAL OUT, 2 outputs`), or print an amount on the `TO` line only when the
transaction has exactly one output and omit it otherwise. Reproduce the
misattribution before choosing; the remedy is a design call for the operator.

---

### [Important] 3 — §1.1e's length report accuses the legitimately-short FINAL chunk of missing characters

**Where** `crates/mt-cli/src/read_strings.rs:84-160` (`length_report`, new),
called from `main.rs:1010` (`explain_failure`, new).

**What** The modal-length check runs on the failure path over every string that
failed to decode, and flags any whose length differs from the mode. Its own
comment argues this is safe because *"the legitimate short chunk PARSES: its
checksum holds, so it never reaches this path."* That is only true while the short
chunk is undamaged. A final chunk with more than `t = 4` wrong symbols does not
parse, does reach this path, and is then reported as having characters missing.

**Why it costs money, a recovery, or is false** Every set whose payload does not
divide evenly has a short final string — the fixture PSBT produces `strings 1-8
are 88, string 9 is 83`, and `mt encode` prints exactly that at cutting time. When
that plate is the damaged one, `mt verify` says:

> `1 string is the wrong length for this set (most are 88)`
> `A character is MISSING or EXTRA, not wrong.`
> `string 9: 83 characters (expected 88) — 5 characters are MISSING`

Every clause is false about the plate in front of the operator, and the tool is
contradicting its own encode-time instruction. The remedy sent — *"Re-read these
from the plate, counting characters"* — makes them count 83 characters on a plate
that is supposed to have 83, while the real diagnosis (too many wrong symbols in
that one string) is never named. With zero redundancy that string is the whole
recovery, and the operator has been pointed away from the only action that can
save it. The refusal's own second paragraph even states the correct rule —
*"every string but the last carries the same payload, so exactly one may be
shorter"* — while the verdict above it accuses that exact string.

**How I reproduced it**
```sh
./mt encode --in final.psbt --bitcoin-cli /nonexistent > strings.txt 2>/dev/null
# corrupt 5 symbols (past t=4) in the LAST, legitimately-short string:
python3 - <<'EOF'
AL="qpzry9x8gf2tvdw0s3jn54khce6mua7l"
L=open('strings.txt').read().split(); s=list(L[8])
for i in (20,25,30,35,40): s[i]=AL[(AL.index(s[i])+7)%32]
L[8]=''.join(s); open('damaged_last.txt','w').write('\n'.join(L)+'\n')
EOF
./mt verify --in damaged_last.txt --bitcoin-cli /nonexistent
# -> REFUSED — §1.1e, 1 string is the wrong length ... 5 characters are MISSING
```
Control: the intended case still works — dropping one character from a
middle (modal-length) string reports it correctly.

**Suggested fix (non-authoritative)** Exclude the one string that may legitimately
differ: only flag a failed string as wrong-length if its length differs from the
mode **and** it is not the unique shortest string in the set, or only if it is
LONGER than the mode or shorter than the shortest legitimate chunk. Reproduce the
false accusation first — the discriminator, not the wording, is the defect.

---

### [Important] 4 — the §8.2b fee-rate refusal prints a number that does not exceed the ceiling it cites

**Where** `crates/mt-cli/src/validate.rs:181-190` (`value_guard`; the comparison
changed from division to multiplication in `95bad20`, and the displayed rate
became `{:.1}` of an f64).

**What** The refusal now triggers on `fee > 25_000 * vb` (correct, and a genuine
tightening) but reports `format!("fee rate {:.1} sat/vB exceeds {}", rate, …)`.
Any rate in `(25000.0, 25000.05]` rounds to `25000.0` in the message.

**Why it costs money, a recovery, or is false** The verdict line is the part the
spec declares machine-parseable and *"names the number that caused it"*. Here it
names a number that does not cause it:

> `mt encode: REFUSED — §8.2b, fee rate 25000.0 sat/vB exceeds 25,000`

An operator reading a refusal that contradicts itself has two plausible next
moves, and the wrong one is adjusting `--input-value` to make the tool relent —
which, per `parse_btc`'s own doc, changes only what `mt` prints while the fee
absorbs the difference in full. The mechanism paragraph does carry the true
integers, so this is recoverable, but the headline is false as printed. Secondary:
the rate lost its `thousands()` formatting in the same edit, so large rates now
read `6780226.0` and `11864402260452.0` beside a comma-formatted ceiling.

**How I reproduced it** (`vb = 177` for the fixture transaction, so the boundary
is a fee of 4,425,000 sat; outputs total 799,900,000 sat)
```sh
# exactly 25,000 sat/vB — allowed, correct:
./mt encode --in raw.hex --bitcoin-cli /nonexistent \
    --input-value 0:8.04325   --input-value 1:0 --quiet >/dev/null; echo $?   # 0
# 25,000.0056 sat/vB — refused, correct, but the message is not:
./mt encode --in raw.hex --bitcoin-cli /nonexistent \
    --input-value 0:8.04325001 --input-value 1:0 --quiet 2>&1 >/dev/null | head -1
# -> mt encode: REFUSED — §8.2b, fee rate 25000.0 sat/vB exceeds 25,000
```

**Suggested fix (non-authoritative)** Print enough precision to be true, or print
the integers that were actually compared (`fee 4,425,001 sat over 177 vB exceeds
25,000 sat/vB`). Restoring `thousands()` on the integer part would also close the
formatting regression.

---

### [Important] 5 — `--elide-prefix` tells the operator to count the UN-elided lengths — PRE-EXISTING, not fold-introduced

**Where** `crates/mt-cli/src/main.rs:516-517` and `:536` (`lengths` /
`correction_coverage` / the `CUT` row), with `render()` applying elision
afterwards at `:576-598`. Both the call site and `blocks::correction_coverage`
are byte-identical to `fb7c217` — this is **not** a fold defect. Reported because
it is unreported elsewhere and it attacks the operator's only check on the damage
BCH cannot repair.

**What** `lengths` is measured on the full pipeline output, before `--elide-prefix`
removes 11 characters from every string after the first. The mandatory
before-you-cut warning and the `CUT` row therefore both describe strings that are
not the ones on stdout.

**Why it costs money, a recovery, or is false** `blocks.rs`'s own test says it:
*"The operator counts characters against this. A wrong span sends them to re-cut a
string that is correct."* With `--elide-prefix` the actual stdout is
88, 77×7, 72 (699 characters) and `mt` says `strings 1-8 are 88, string 9 is 83`
and `CUT 9 strings, 787 characters`. An operator who counts diligently concludes
eleven characters are missing from seven correctly-cut plates — at ~21 minutes a
plate, that is the expensive direction.

**How I reproduced it**
```sh
./mt encode --in final.psbt --bitcoin-cli /nonexistent --elide-prefix 1>e.out 2>e.err
awk '{print NR": "length($0)}' e.out       # 88, then 77 x7, then 72   (total 699)
sed -n '/Count each string/,/^$/p' e.err   # "strings 1-8 are 88, string 9 is 83"
grep '^CUT' e.err                          # "CUT       9 strings, 787 characters"
./mt verify --in e.out --bitcoin-cli /nonexistent   # OK — the elided output is valid
```

**Suggested fix (non-authoritative)** Measure `lengths` on `render(&strings,
&args)` rather than on `strings`, so the numbers describe what the operator will
engrave.

---

### [Minor] 6 — `mt verify --transaction <not a transaction>` prints a refusal labelled `mt encode:`

**Where** `crates/mt-cli/src/main.rs:908` — the new PSBT path routes the supplied
file through `input::sniff`, whose refusals hardcode the verb `"encode"`
(`input.rs`, `hex_psbt_guard` and `recognised_guard`).

**What / why false** A `verify` run prints `mt encode: REFUSED — §8.2e, input is
not a PSBT or a raw transaction (11 bytes)`, and the following advice is
encode-flavoured. The verb in the verdict line is documented as part of the
machine-parseable format.

**How I reproduced it**
```sh
printf 'hello world\n' > junk.txt
./mt verify --in strings.txt --transaction junk.txt --bitcoin-cli /nonexistent
# -> mt verify: OK — 9 chunks ...
# -> mt encode: REFUSED — §8.2e, input is not a PSBT or a raw transaction (11 bytes)
```

**Suggested fix (non-authoritative)** Give `sniff`/`hex_psbt_guard`/
`recognised_guard` a `verb` parameter, as `secret_guard` already has.

---

### [Minor] 7 — `legacy_unbound_warning` says "This input carries no non_witness_utxo" about an input that carries one

**Where** `crates/mt-cli/src/validate.rs:414` (rewritten warning) gated at
`main.rs:496-499` on `Provenance::is_verified`.

**What / why false** `psbt_input_value` now correctly falls back to
`witness_utxo` — and to `ValueSource::PsbtClaimed` — when a `non_witness_utxo` is
present, hashes to the input's txid, but has no output at the input's `vout`
(`validate.rs:285-296`, the documented fall-through). The warning fired by that
path then asserts the record is absent. The label is right (`PSBT-CLAIMED —
unverified`); the sentence is wrong. Needs a crafted PSBT to reach, and the
operator's action is unchanged, hence Minor.

**How I reproduced it** Hand-built PSBT `L4_nwu_badvout.psbt`: one finalized
legacy input, `non_witness_utxo` = the fixture's `legacy_parent_hex` (hash
matches `legacy_parent_txid`), `previous_output.vout = 99`, plus a `witness_utxo`.
```sh
./mt encode --in L4_nwu_badvout.psbt --bitcoin-cli /nonexistent 2>&1 >/dev/null
#  WARNING: input 0 is a legacy (pre-SegWit) input whose value NOTHING has verified.
#    ...
#    This input carries no non_witness_utxo, so mt could not bind the value by txid
#  INPUTS  ... 10.00000000 BTC   PSBT-CLAIMED — unverified
```

**Suggested fix (non-authoritative)** Derive that clause from `ValueSource` too,
e.g. *"nothing binds this input's value by txid"* — which is true in both
fall-through shapes.

---

### [Minor] 8 — `separator_guard` refuses a `--separator` that would never be used, with a mechanism that is false in that case

**Where** `crates/mt-cli/src/main.rs:246` (unconditional) and `:1225`
(`separator_guard`, new).

**What / why false** The guard runs whether or not `--group-size` was given. With
no `--group-size` the separator is never applied, so the refusal's stated
mechanism — *"A separator of any other kind lands on stdout — the stream you
engrave"* — is untrue of the run being refused, and a run that would have produced
a perfectly good artifact is stopped.

**How I reproduced it**
```sh
./mt encode --in final.psbt --bitcoin-cli /nonexistent --separator '-' ; echo $?
# -> mt encode: REFUSED — §1.1e, --separator "-" is not whitespace     (exit 1)
```

**Suggested fix (non-authoritative)** Run the guard only when
`args.group_size.is_some_and(|n| n > 0)`; keeping the refusal for the case that
actually reaches stdout costs nothing and stops claiming something untrue.

---

### [Minor] 9 — `--group-size 0` is silently ignored

**Where** `crates/mt-cli/src/main.rs:589` — `Some(n) if n > 0 => …, _ => base`.

**What / why it matters** A flag that quietly does nothing is the exact class
`7d30fb8` was folding out for `--from`/`--to`/`--to-label`. `--group-size 0` exits
0 with ungrouped output and no word about it.

**How I reproduced it**
```sh
./mt encode --in final.psbt --bitcoin-cli /nonexistent --group-size 0 --quiet | head -1
# ungrouped, exit 0
```

**Suggested fix (non-authoritative)** Refuse `--group-size 0` with §1.1e's voice,
alongside `separator_guard`.

---

### [Minor] 10 — the legend's NOT-SUPPLIED block states plural facts when only one is missing, and contradicts the `TO` line it just printed

**Where** `crates/mt-cli/src/blocks.rs:178-195`.

**What / why false** The body is written for the both-missing case and is reused
verbatim for the one-missing branches: with `--from` supplied it still says *"The
transaction does not carry **either** fact"*, *"A plate that says **neither**"*,
and *"Supply --from / --to"*. And with only `--to-label`, the block says *"TO is
NOT SUPPLIED"* three lines under a `TO alice cold wallet  7.99900000 BTC   <--
LABEL ONLY, unverified` line it printed itself.

**How I reproduced it**
```sh
./mt encode --in raw.hex --bitcoin-cli /nonexistent \
    --from ABCD1234 --to-label "alice cold wallet" 2>&1 >/dev/null | sed -n '/SUGGESTED LEGEND/,$p'
```

---

### [Minor] 11 — `--json` is a spec'd flag that does nothing, on all four verbs — PRE-EXISTING

**Where** `crates/mt-cli/src/main.rs:76` and `:150` — the field is declared on
both arg structs and read nowhere in the crate (`grep -n 'json' src/main.rs`
returns only the two declarations). Unchanged by the fold. Spec line 3261 lists
`--json` as *"machine-readable report. `md` has it"*.

**How I reproduced it**
```sh
./mt inspect --in strings.txt --json --bitcoin-cli /nonexistent | head -2   # human report
./mt encode  --in raw.hex    --json --bitcoin-cli /nonexistent | head -1    # plain strings
```

---

### [Minor] 12 — `decode`/`inspect` accept `--transaction` and silently ignore it — PRE-EXISTING

**Where** `crates/mt-cli/src/main.rs:63-70` — `--transaction` lives on the shared
`ReadArgs`, documented *"`verify` only"*, and only `verify()` reads it. Unchanged
by the fold.

**Why it matters** `mt decode --in steel.txt --transaction original.psbt > tx.hex`
exits 0 and emits broadcastable hex having compared nothing, while the operator
believes they checked their steel against the source. Silent, on the recovery
path, in the direction of a false pass.

**How I reproduced it**
```sh
./mt decode --in strings.txt --transaction lt_time.hex \
    --bitcoin-cli /nonexistent --quiet >d.out; echo $?   # 0, 679 bytes of hex
# lt_time.hex is a DIFFERENT transaction (txid 1704c704…, vs 4665e0aa… on the strings)
```

**Suggested fix (non-authoritative)** Refuse `--transaction` on `decode`/`inspect`,
or honour it there.

---

### [Minor] 13 — panic (exit 101, no refusal) on a non-ASCII character straddling byte 11 of the first `mt1` line — PRE-EXISTING

**Where** `crates/mt-cli/src/read_strings.rs:193` — `full[..ELIDED_DROP]` slices a
`String` by byte index. `restore_elided` is unchanged by the fold; only its call
site was reshaped.

**Why it matters** The recovery path loses its refusal entirely: no verdict, no
mechanism, no remedy, exit 101 with a Rust panic message. A non-ASCII character in
any other position produces a clean §1.1 refusal naming the character, so the
class is otherwise handled.

**How I reproduced it**
```sh
printf 'mt1qqqqqqq\xc3\xa9qqqqqqqqqqqqqqq\n' > utf8b.txt
./mt decode --in utf8b.txt --bitcoin-cli /nonexistent; echo $?
# thread 'main' panicked at crates/mt-cli/src/read_strings.rs:193:22:
# byte index 11 is not a char boundary; it is inside 'é' (bytes 10..12)   (exit 101)
```

**Suggested fix (non-authoritative)** Take the prefix by `chars().take(ELIDED_DROP)`,
or reject non-ASCII before slicing.

---

### Nits (recorded, not gating)

- `main.rs:373,399` — `bound_by_chain` is assigned and never read; the legacy
  warning's gate now derives the same fact from `Provenance::is_verified`.
- `main.rs:1289-1305` — `check_input_value_indices`'s doc-comment describes the
  *range* guard's defect ("still has no value… mt prints FEE UNKNOWN"), which is
  untrue of a duplicate index (`.find()` takes the first, so §8.2b does run), and
  claims it is *"Checked against the transaction's real input count"*, which it is
  not. It also takes `_args: &EncodeArgs` and ignores it.
- `main.rs:918-934` — the unfinalized-`--transaction` refusal concatenates
  `r.verdict` with the following sentence and no punctuation: *"…(input 0, 1) mt
  compares against the transaction a PSBT EXTRACTS to…"*.
- `parse_btc` accepts a trailing dot (`--input-value 0:1.` = 1 BTC), and the
  over-supply case reports headline *"is not a BTC amount"* for `21000001`, with
  the true reason only in the body.
- `locktime.rs:198` — `season_year`'s doc says `None` *"when the target is at or
  below the reference"*; the code returns `Some` at exactly the reference, which
  is the right behaviour (verified: `963759 -> SUMMER 2026`). The comment is the
  wrong half.
- `Lock::NotEnforced`'s report row omits the `current height …` column that the
  other four rows carry, so the two-numbers-side-by-side layout breaks for that
  one state; the column padding in `report_row` is fixed-width and misaligns for
  any height other than six digits.

## What I checked in the new code and found CLEAN

**`locktime.rs` §8.4 — differential-tested, not read.** I compiled a verbatim copy
of `civil_from_unix` / `iso_minutes` / `season_year` standalone and diffed it
against a reference implementation:

- `iso_minutes` is exact on **every one of 2,932,897 days from 1970-01-01 to
  9999-12-31** — 0 mismatches. That covers every leap year, every century rule
  (1900/2000/2100/2200 style), and the Gregorian 400-year era boundaries the
  Hinnant algorithm turns on.
- `season_year` is exact on **4,000,004 consecutive heights** from `MT_REF_HEIGHT`
  upward (~76 years of projection) — 0 mismatches, so no month boundary is
  mis-bucketed anywhere in the projection range.
- `MT_REF_TIME = 1_787_507_701` **is** `2026-08-23T17:55:01Z` as claimed, and the
  cited tip `nTime = 1787509876` is 2,175 s = 36.25 min ahead of it, matching the
  comment's "36 minutes ahead" exactly.
- No overflow anywhere in the range: `season_year(u32::MAX)` is
  `(4294967295−963759)×600 + MT_REF_TIME ≈ 2.6e12`, far inside `u64`.

**Every `Lock` branch at its boundary, end to end through the CLI** (byte surgery
on the fixture transaction's `nSequence` and `nLockTime` fields):

| nLockTime | nSequence | report row | legend |
| --- | --- | --- | --- |
| 0 | all final | `NO TIMELOCK … current height unknown (no node)` | `NO TIMELOCK` |
| 96 | all final | `nLockTime 96 present but NOT ENFORCED (all inputs final)` | `NO TIMELOCK` |
| 96 | one non-final | `LOCKED TO BLOCK 96 …` + below-reference warning | `LOCKED TO BLOCK 96` |
| 900,000 | non-final | below-reference warning, no estimate | `LOCKED TO BLOCK 900000` |
| 963,758 (ref−1) | non-final | below-reference warning | no estimate |
| 963,759 (ref) | non-final | — | `LOCKED TO BLOCK 963759 ~SUMMER 2026` |
| 1,383,520 | non-final | — | `~SUMMER 2034` (matches the pinned test; the spec's `~FALL` is the known, filed disagreement) |
| 499,999,999 | non-final | height branch | `~WINTER 11514` — honest, height is the fact |
| 500,000,000 | non-final | `LOCKED UNTIL 1985-11-05T00:53Z … current MTP` | timestamp branch |
| 1,800,000,000 | non-final | `LOCKED UNTIL 2027-01-15T08:00Z` | same |

The threshold is decided at exactly the right place (499,999,999 → height,
500,000,000 → timestamp), the nSequence rule matches consensus (`nLockTime` is
enforced iff at least one input is non-final), and the below-reference warning
fires on exactly `n < MT_REF_HEIGHT`. The timestamp branch has no
below-reference analogue, which I judged correct rather than a hole: a human can
compare a date to today without help, which is the whole reason the height case
needs one.

**`parse_btc`** — 15 inputs at and around its boundaries. Refused: `.5`, `-1`,
`inf`, `NaN`, `1e5`, `1_000`, `"1 "`, `0.000000001` (nine places), `0.1234567891`,
`21000001` (over the 21M supply). Accepted and correct: `20999999.99999999`,
`21000000`, `007`, `0`, `1`. No panic on any of them (the f64 path that panicked
on `inf`/`1e30` is gone), and `checked_mul`/`checked_add` cover the overflow.

**The four new index guards let the boundary through.** `--input-value 1:…` on a
2-input transaction is accepted; `2` and `4294967295` are refused by name;
`4294967296` is caught as a non-`u32`; a repeated index is refused; and the
range guard fires with the right count on both the PSBT and raw-hex paths.

**The fee-rate comparison is now correct in the direction that matters.** With
`vb = 177`, a fee of exactly 4,425,000 sat (25,000.0 sat/vB) is accepted and
4,425,001 sat is refused — the old truncating `fee / vb` accepted everything below
25,001. Only the printed number is wrong (finding 4).

**`legacy_unbound_warning`, on four hand-built legacy PSBTs.** The six-way defect
is genuinely fixed: with `non_witness_utxo` present and hashing to the input's
txid the warning does **not** fire and the row reads `TXID-BOUND`; with only
`witness_utxo` it fires and says *"it came from the PSBT's witness_utxo, which
nothing has checked"*; with neither record and `--input-value` it fires and says
*"you supplied it with --input-value"*; with neither record and no value §8.2c
refuses first. Every derived clause checks out against the numbers beside it —
`Outputs total 9.99900000 BTC` equals the transaction's output sum, `Claimed for
input 0 10.00000000 BTC` equals the record, and *"mt therefore shows a fee of
0.00100000 BTC"* is byte-identical to the `FEE` row twenty lines below, which was
the specific thing defect 5 got wrong.

**`psbt_input_value` returning `ValueSource`** puts the label and the number on the
same path: the vout-out-of-range record renders `PSBT-CLAIMED — unverified`, not
`TXID-BOUND`. The `--input-value` disagreement warning fires with the right
numbers and the right justification (`--input-value 0:2.50000000 BTC disagrees
with the PSBT, and mt used the PSBT. … it is bound to the input's txid by §8.2d`).

**`content_id_guard` and its ranked suspect list**, exercised live by forging a set
(transaction B's payload under transaction A's txid) with the codec:
- clean forgery → `9 chunks, set 0x4665e, … re-derives 0x1704c` (0x1704c is
  correct — it is the first five hex of the payload transaction's real txid), and
  the `ranked.is_empty()` remedy fires: *"No chunk needed any correction, so this
  is not miscorrection…"*.
- forgery + 3 damaged symbols in chunk 4 and 1 in chunk 2 → the verbatim block
  renders un-reflowed with the columns intact, ranked most-corrected first,
  `<-- most suspect` on the top row, and *"The other 7 chunks needed no correction"*
  — 9 − 2 = 7, correct.
- `chunks[0]` cannot panic: `ChunkHeader::from_symbols` computes
  `count = field + 1`, so `count >= 1` and `pipeline::decode` fills every slot or
  returns `MissingChunk`.

**`margin_report`'s character positions are exactly right.** I damaged 0-based
string indices 20 and 60 of chunk 3; `mt` reported `pos 21` and `pos 61` with the
correct read/corrected characters in both directions. The `p + 1 + 3` mapping from
data-part offset to 1-based whole-string position is confirmed against the codec's
own offset convention (`character 'é' at data-part offset 0` for `mt1é…`).

**`set_notices` arithmetic.** A damaged duplicate of chunk 1 reports `KEPT 0 of 4`,
`DISCARDED 2 of 4`, *"the discarded plate has 2 corrections left"* — 4 − 2 = 2,
correct, and it correctly kept the healthier copy. A stray note line typed into
the file reports `UNREADABLE STRING. string 10` — the right 1-based position — and
the rewritten body no longer directs a physical action on a plate `mt` never
identified.

**stdout discipline — 12 failure paths, all clean.** `decode` (length refusal, §1.1
refusal, no-strings, §8.9 `ms1`, forged content id), `verify` (unfinalized
`--transaction`, mismatched `--transaction`), `encode` (unfinalized PSBT, fee
ceiling, bad separator, duplicate index, `ms1`), `inspect` (damaged set) — every
one wrote **0 bytes** to stdout and exited non-zero. `inspect`'s report on stdout
is spec'd (§ line 752: *"`inspect` prints it on stdout (it is the artifact of that
verb)"*), and `decode` prints its new report to stderr with the hex alone on
stdout. `decode --quiet` and non-quiet agree on which sets are refused, because
`content_id_guard`'s `txid_display` already decodes the transaction.

**`explain_failure`'s unreadable set is consistent with the codec's.** Both call
`pipeline::decode_chunk(s, None)`, so the strings named in the length report are
exactly the ones the codec could not read; `input_position` is 1-based in both.

**`--transaction`'s new PSBT path** accepts a binary PSBT, a base64 PSBT and raw
hex, all matching on the full txid; refuses a different transaction naming both
txids in full; and refuses an unfinalized PSBT rather than matching on a txid whose
bytes were never engraved. Exit code 1 in both refusal cases.

**`Report::build`'s fallback chain leaves no hole** that I could construct. For
`decode`/`inspect` it is inert (`claimed = &[]`). For `encode` the only states it
changes are (a) parent unconfirmed → PENDING, where the old code discarded a
txid-bound value and printed `FEE UNKNOWN` with a node while showing it without
one, and (b) this transaction already confirmed, where every input is spent by
itself — and in that state `spent_input_guard` is deliberately skipped, so the
success case now shows its fee instead of reporting as the theft case. A
genuinely-spent input still refuses at `spent_input_guard` before the report is
built.

**`Refusal::with_verbatim`** renders after the remedy, two-space indented, with
line breaks preserved — confirmed on both consumers (the length report's
per-string table and the content-id ranked list).

**The `mt-codec` changes in this range are documentation only.** `git diff -U0`
over `bch.rs` and `bch_decode.rs` shows zero non-comment lines changed, and the
provenance correction is true: `design/PROVENANCE.md:26-37` says the string layer
is ported from `mk-codec` 0.5.0 *"not `md-codec`"*, and both siblings exist on
disk (`mnemonic-toolkit/vendor/mk-codec`, `descriptor-mnemonic/crates/md-codec`).

**Elided round trip.** `mt verify` accepts `mt encode --elide-prefix`'s own stdout,
and the restored strings are byte-identical to the full ones (so the length check
and the `input_position` indices operate on a consistent basis).
