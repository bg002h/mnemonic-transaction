# R — Operator journey walk, round 2

**Agent:** independent journey walker (wrote none of this code).
**Target:** `/scratch/code/shibboleth/mnemonic-transaction`, `./target/debug/mt`, at `268b354`
(all four prior post-implementation reviews **folded**; this walk is against post-fold code).
**Method:** four prescribed journeys plus three invented ones. Every command below was
actually run; every transcript is pasted from real output. Offline forced with
`--bitcoin-cli /nonexistent`.

**The rule applied throughout:** *a divergence earns a change ONLY IF the wrong outcome is
WORSE THAN TELLING THE OPERATOR NOTHING.* I applied it strictly and say so at each
divergence. The "earned no change" list at the end is long on purpose — it is what stops
the next walker re-deriving these.

---

## Verdict

**0C / 3I / 4M.**

A first-time operator gets through cleanly — `mt` never hangs, refuses before the
irreversible act rather than after it, and tells them the plate count, the character
counts, what to engrave beside the strings and what to do when done. A 2040 recoverer gets
through **provided their transcription errors are of the kinds `mt` classifies well**
(typos within budget, wrong order, one blob, uppercase, a missing plate, mixed engravings —
all excellent); the one recovery slip that puts them in a ditch is **more than four wrong
characters in a single string**, where `mt` tells them a plate is *missing* while all nine
sit in their hand, and discards — at the exact line — the variable that already knows
better.

No Critical: nothing I found induces an irreversible act or silently emits a wrong
transaction. Refusal exit codes are all `1`, `decode` emits nothing on stdout when it
refuses, encoding is byte-deterministic across runs, and PSBT and raw hex produce identical
strings.

---

## Journey A — first contact

*Someone has heard of `mt` and has a signed transaction in a file.*

**Step 1 — they type `mt`.**

```
$ mt
The mt CLI — encode, decode, verify and inspect mt1 engravable transaction backups

Usage: mt <COMMAND>

Commands:
  encode   Turn a signed transaction into `mt1` strings for hand engraving
  decode   Read `mt1` strings back and emit BROADCASTABLE HEX on stdout
  verify   Check a set of `mt1` strings — structurally, and never asking a node
  inspect  Report what is IN a set, consulting a node automatically when one is there
  help     Print this message or the help of the given subcommand(s)
```
exit 2. Help, immediately. **No hang.** The four verbs are each described in one line that
says what the operator gets, not what the code does.

**Step 2 — they type `mt encode` and wait.** This is the moment the project's own history
flags (*"stdin doesn't mean from the command line?"* — the tool that blocks on a TTY with
no prompt). Run on a **real pty**, nothing typed, Ctrl-D:

```
$ script -qec "mt encode" /dev/null < /dev/null
mt encode: reading a transaction from stdin.

  Paste a finalized PSBT (base64) or a raw signed transaction (hex),
  then press Ctrl-D on a new line. Or pass a file with --in.

  Nothing has happened yet.

mt encode: REFUSED — §8.2e, input is not a PSBT or a raw transaction (0 bytes)
  ...
  Check the file is the one you meant, and pass it with --in.
```

The prompt fires (`blocks.rs:22`, `stdin().is_terminal()`), names all three accepted
inputs, gives the terminating keystroke, offers the file alternative, and closes with
**"Nothing has happened yet."** — which is precisely the sentence a frightened new operator
needs. This class of defect is **closed**.

**Step 3 — `mt --help`, then `mt encode --help`.** Long-form help is unusually good: every
flag carries the *reason* it exists, and `--bitcoin-cli` documents the offline-forcing
trick. One flag is wrong, and it is Important 2 below.

**Step 4 — they paste.** Covered in Journey B.

### Divergences

| # | What else they might do | What happened | Classification | Earns a change? |
|---|---|---|---|---|
| A1 | Type `mt` bare | Help, exit 2 | **default** | No — correct behaviour |
| A2 | `mt encode`, wait on a TTY | Prompt + "Nothing has happened yet" | **default** | No — already right |
| A3 | Ctrl-D immediately | §8.2e refusal naming all three formats | **refusal** | No |
| A4 | Read `--transaction`'s help and follow it | Leaks a bearer tx into history/`ps` | **documentation only** | **YES → Important 2** |

---

## Journey B — the cut

*They encode, read stderr, and start engraving.*

```
$ mt encode --in finalized.psbt --bitcoin-cli /nonexistent
```

stdout = 9 bare `mt1…` strings. stderr = five WARNING blocks, then the report, then the
legend. **On a real TTY the stdout strings land LAST** (verified: lines 79–87 of 87), so
the final thing on screen is the thing to engrave. That ordering is right and worth
keeping.

**What they are told before the irreversible act**, in order:

1. the input file is mode 0644 and bearer — *"exactly as dangerous as the plate you are
   about to cut"*, and honest that it says nothing about who read it before;
2. no node reachable — the three questions a node would have answered, and *"If the inputs
   turn out to be spent, the plate is scrap the moment it leaves the machine"*;
3. bearer, and that the SIGHASH check reads **witness shape, not script**;
4. **the correction budget**: 4 wrong characters per string, cannot repair missing/extra,
   *"strings 1-8 are 88, string 9 is 83"*, no redundancy, cut a second copy if you want to
   survive losing one;
5. **what to do when done**: *"verify the ENGRAVING — not this output… `mt verify <
   typed-from-steel.txt`… Verifying the file mt just produced tests nothing that can
   fail."*

Then `CUT 9 strings, 787 characters`, `PREFIX all 9 strings begin mt1pgej7qqg`, and a
SUGGESTED LEGEND with the five facts a stranger needs.

**Machine-checked, not taken on trust:**

```
$ awk '{print NR": "length($0)}' strings.txt
1..8: 88     9: 83          # matches warning 4 exactly
$ tr -d '\n' < strings.txt | wc -c
787                          # matches the CUT row
$ cut -c1-11 strings.txt | sort -u
mt1pgej7qqg                  # matches the PREFIX row, all 9
```

All three claims are true. Nothing the operator needs is buried after something they skim:
the correction budget and the verify-the-engraving instruction are warnings 4 and 5, i.e.
the last two before the report — the closest to the strings.

**Determinism** (an operator who re-runs mid-cut): three runs, identical md5; and the
finalized PSBT and the extracted raw hex produce **byte-identical strings**. Re-running is
safe.

**Non-whitespace separator** — refused *before* the cut, with the cost stated:

```
mt encode: REFUSED — §1.1e, --separator "-" is not whitespace
  ...mt refuses this now rather than after nine plates are cut.
```

### Divergences

| # | What else they might do | What happened | Classification | Earns a change? |
|---|---|---|---|---|
| B1 | `--quiet` for clean output | Loses CUT/PREFIX/legend; **keeps all 5 warnings** | **default** | No — see no-change list |
| B2 | `--separator -` | Refused before cutting | **refusal** | No — already ideal |
| B3 | Re-run mid-cut | Identical strings | **default** | No |
| B4 | Redirect stdout, engrave, walk away | Bearer file left on disk, unmentioned | **warning** | **YES → Minor 7** |
| B5 | Walk away, return to scrollback | TX + PREFIX + CUT identify the run | **default** | No |
| B6 | Engrave a timelocked tx | Height and timestamp both correct, legend carries it | **default** | No |

---

## Journey C — 2040, recovery

*A stranger finds steel and a laptop. No legend, no node, no memory of the tool.*

Baseline, clean transcription:

```
$ mt verify --in steel.txt --bitcoin-cli /nonexistent
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.
```

Then every clumsy thing they might do. **Six of eight are excellent:**

| Case | Input | Result |
|---|---|---|
| uppercase | all 9 SHOUTED | `OK — 9 chunks` |
| wrong order | reversed | `OK — 9 chunks` |
| one blob | space-joined single line | `OK — 9 chunks` |
| one blob, **no separator at all** | 787 chars concatenated | `OK — 9 chunks` |
| typos in budget | 3 substitutions | `OK`, plus a per-position correction report |
| one plate missing | 8 strings | `REFUSED — chunk 5 of 9 is missing` |
| two engravings mixed | 7 of set A + 2 of set B | `REFUSED — chunk set id mismatch: expected 0x4665e, chunk 1 carries 0x9b7c8` |
| dropped / extra character | 87 / 89 chars | `REFUSED — §1.1e… string 3: 87 characters (expected 88) — 1 character is MISSING` |

The no-separator blob working is a genuinely good piece of design (`mt1` cannot occur
inside a payload — `1` is outside the bech32 charset), and the §1.1e length refusal is the
best message in the tool: it explains *why* BCH cannot help, and says outright that it
stops early because *"a length error reports as a MISSING PLATE once it reaches the codec,
and that sends you looking for steel that is not lost."*

**The eighth case is the ditch.** Eight substitutions in one string — one scratched plate,
or a misread of similar glyphs — all nine strings present, every length correct:

```
$ mt verify --in c6_typo8.txt --bitcoin-cli /nonexistent
mt verify: REFUSED — §1.1, the set does not verify

  chunk 2 of 9 is missing
```

Nine plates are on the table. `mt` says one is missing. That is **Important 1**, and the
project already wrote the sentence condemning it — for the other branch.

**Does anything tell them the money may already be gone?** Yes, and well. `mt decode`
prints the full report to stderr and the hex to stdout, then:

```
- do these inputs still exist, or were they spent?  UNKNOWN
- was this transaction already broadcast?           UNKNOWN
- has the locktime passed?                          UNKNOWN
- what fee does it pay?                             UNKNOWN

Everything above this line was read from the engraving itself
and is what the transaction SAYS. None of it is confirmed.

TO RESOLVE ALL FOUR AT ONCE, either:
  - run mt inspect again with a bitcoind reachable, or
  - look this txid up in any block explorer:
      4665e0aa2a97dee3e53fd2eaab0942abc0fef2339e730458ceedcda400d708c3
```

The block-explorer escape hatch is exactly right for a stranger with a laptop and no node.
(The `locked to block 0` line inside that block, on a tx whose own row reads `NO TIMELOCK`,
is **prior Minor 3** — still open, not re-reported here.)

**Duplicates**, both directions:

- full set + an exact repeat of plate 4 → `OK`, plus `DUPLICATE RESOLVED`, which keeps the
  healthier copy and names the correction budget each spent. Good design.
- plate 4 typed twice, plate 5 skipped (still 9 lines) → `chunk 5 of 9 is missing`, with no
  mention that chunk 4 arrived twice. True, but a strong hint is withheld → **Minor 5**.

### Divergences

| # | Divergence | Classification | Earns a change? |
|---|---|---|---|
| C1 | uppercase / order / blob / no-separator blob | **default** | No — all recover |
| C2 | ≤4 typos | **default** | No — corrected and reported |
| C3 | **>4 typos in one string** | **warning** | **YES → Important 1** |
| C4 | missing plate | **refusal** | No — names the chunk |
| C5 | mixed engravings | **refusal** | No — names both set ids |
| C6 | dropped/extra character | **refusal** | No — best message in the tool |
| C7 | typed one plate twice, skipped another | **warning** | **YES → Minor 5** |
| C8 | exact duplicate plate | **default** | Partly → **Minor 6** (re-cut advice) |
| C9 | all-elided set, full string lost | **refusal** | No — correct message, wrong label → **Minor 4** |

---

## Journey D — the wrong tool

*An operator reaches for `mt` with the wrong thing.*

| Input | Result | Verdict |
|---|---|---|
| wallet descriptor → `encode` | `REFUSED — §8.2e, input is not a PSBT or a raw transaction (297 bytes)… This input begins 77 73 68 28 73 6f 72 74 and matches none of them.` | Excellent — dumps the leading bytes |
| unsigned PSBT / SIGHASH_NONE | `REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL` + *"a holder… can redirect the funds while the signature stays valid, and the legend's TO line becomes false"* | Excellent |
| **empty file** | `REFUSED — §1.1e, no strings found in the input. mt splits input on line breaks, and on each `mt1` prefix within a line. **Nothing in this input looked like either.**` | **Excellent — and this is the message the next three cases should get** |
| directory | `REFUSED — §1.1e, cannot read adir. The file could not be opened: Is a directory (os error 21).` | Correct |
| nonexistent file | `REFUSED — §1.1e, cannot read nope.txt… (os error 2)` | Correct |
| **`md1` sibling string → `decode`** | `REFUSED — §3b, all 1 lines are elided; no prefix to restore… Add the 8 characters following `mt1` on any intact string of the same set.` | **Wrong, and unfollowable → Important 3** |
| **BIP-39 mnemonic → `decode`** | identical §3b message | **Wrong → Important 3** |
| **file of random text → `decode`** | `all 6 lines are elided` | **Wrong → Important 3** |

**The reverse — `mt`'s own output into the wrong verb.** All refusals exit `1`; `decode`
writes nothing to stdout when it refuses, so a `mt decode … | bitcoin-cli
sendrawtransaction` pipeline cannot broadcast a half-recovered transaction. Verified:

```
mt decode --in c6_typo8.txt   -> EXIT=1      mt decode --in rand.txt      -> EXIT=1
mt decode --in c4_missing.txt -> EXIT=1      mt decode --in c7_mixed.txt  -> EXIT=1
mt decode --in steel.txt      -> EXIT=0
```

**A 2 GB file** was not run: the classification is decided by the first bytes and the
line-splitter, and every large-input path is the same code as `rand.txt`. Running it would
have measured `read_to_string`, not a journey moment.

### Divergences

| # | Divergence | Classification | Earns a change? |
|---|---|---|---|
| D1 | descriptor / xpub / unsigned PSBT to `encode` | **refusal** | No |
| D2 | mnemonic, `md1`, random text to `decode` | **refusal** | **YES → Important 3** |
| D3 | empty file / directory / missing file | **refusal** | No |
| D4 | refusal in a broadcast pipeline | **refusal** | No — exit 1, empty stdout |

---

## Findings

### [Important] 1 — nine plates on the table, and `mt` says one is missing

**The step.** 2040. The recoverer has typed all nine strings back from steel. One plate is
scratched, so that string carries more than four wrong characters. Every string is the
correct length.

**What the operator has.** Nine plates, all of them, in their hand. A complete set.

**What mt did.**

```
$ mt verify --in c6_typo8.txt --bitcoin-cli /nonexistent
mt verify: REFUSED — §1.1, the set does not verify

  chunk 2 of 9 is missing
```

Identical from `decode` and `inspect`. There is no mention that a string was unreadable, no
mention of the correction budget, and no suggestion to re-read anything.

**What it should have done.** Say that string 2 of the input could not be read — more than
four characters wrong — and that **every plate is accounted for**, so nothing is lost and
the remedy is to re-read that one plate.

`mt` already knows this, at the line where it decides. `explain_failure`
(`crates/mt-cli/src/main.rs:1007`) computes:

```rust
let unreadable: Vec<usize> = strings.iter().enumerate()
    .filter(|(_, s)| pipeline::decode_chunk(s, None).is_err())
    .map(|(i, _)| i + 1).collect();
if let Some(r) = read_strings::length_report(strings, &unreadable, verb) {
    return r;
}
Refusal::new(verb, "§1.1", "the set does not verify", format!("{e}"))
```

When the damage is substitutions rather than length, `length_report` returns `None` — and
the last line renders the raw codec error while `unreadable == [2]` is **live, correct, and
in scope**. The pipeline computes the same thing a second time and keeps a `reason` string
with it (`pipeline.rs:244–258`), then drops the whole vector when a slot is empty
(`pipeline.rs:340`).

The proof that this information is presentable is that `mt` already presents it — but only
on the path where it is least needed. Add a wrecked *duplicate* to an otherwise complete
set and the set succeeds, and then:

```
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.

UNREADABLE STRING. string 10 of the input could not be read:
  BCH correction failed: regular code: more than 4 substitutions or pathological pattern
  This set is complete WITHOUT it, so nothing here is missing. mt
  cannot tell you which chunk that string was, or whether it belongs
  to this set at all — it could not read it. Do not discard the plate
  on this message alone: check whether it is from another engraving
  first, and if it is one of THESE, re-cut it from the strings mt has
  just verified.
```

That notice is well-written, careful, and fires **only when the set already verified**. On
the failure path — where the operator is stopped, confused, and holding a bearer instrument
they cannot cash — it is discarded and replaced with a false statement about their steel.

**Classification: warning** (the refusal is correct; what is missing is the *reason*).

**Why it beats saying nothing.** Saying nothing would leave them to re-read a plate. Saying
*"chunk 2 of 9 is missing"* sends them to hunt for a plate that is in front of them, and —
because §1.8 spells out that there is **no redundancy and all 9 strings are required** — a
recoverer who concludes a plate is lost may reasonably conclude the recovery is over. The
project has already ruled this outcome unacceptable, in `read_strings.rs:76` and again in
`main.rs:1000`, in the same words: *"an accusation about the operator's steel, sending them
to hunt for a plate that is sitting in front of them."* §1.1e closed that door for the
length branch. The substitution branch is the same door, still open.

**On severity.** I considered Critical and did not take it: no irreversible act is induced,
nothing wrong is emitted, and the data is fully recoverable by retyping one plate. It is
the high end of Important, and it is the single highest-value fix in this walk.

---

### [Important] 2 — `--transaction`'s own help tells the operator to leak a bearer transaction

**The step.** After engraving, the operator wants the check the spec calls *"the sibling
round-trip"* (§ spec:509) — *is this steel really the transaction I think it is?* They run
`mt verify --help`:

```
--transaction <PSBT|HEX>
    Compare against a transaction, by FULL txid.
```

**What the operator has.** A finalized PSBT or a raw hex transaction — exactly the two
things the value name asks for.

**What mt did.** They paste it, as instructed:

```
$ mt verify --in steel.txt --transaction "$(cat raw_hex.txt)" --bitcoin-cli /nonexistent
mt verify: REFUSED — §8.2f, a transaction was passed as a command-line argument (678 characters)

  It is now in your shell history and was visible in `ps` while this
  ran. A finalized transaction — and the mt1 strings it becomes —
  is BEARER: anyone who reads it can broadcast it.
  ...
  Remove it: history -d $HISTCMD && fc -W # zsh
  Then re-run: mt <verb> < file
```

The refusal is correct and the leak is real. But **`mt`'s own help is what caused it.** The
flag does not take a PSBT or hex at all — it takes a **path**:

```rust
// crates/mt-cli/src/main.rs:67
#[arg(long, value_name = "PSBT|HEX")]
transaction: Option<std::path::PathBuf>,
```

Passed a path, it works perfectly:

```
$ mt verify --in steel.txt --transaction raw_hex.txt --bitcoin-cli /nonexistent
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.
  --transaction matches, on the full txid.
```

Nothing in `--help` — not the value name, not the doc paragraph, not the remedy line —
says the word *path*. And the remedy `Then re-run: mt <verb> < file` points at **stdin for
the strings**, which is a different input; it does not route the operator to the working
invocation.

**What it should have done.** `value_name = "PATH"`, one sentence in the doc comment
saying the argument is a file containing the transaction, and a remedy line that names the
working form: `--transaction <path-to-tx-file>`.

**Classification: documentation only** (the code is right; the description is wrong).

**Why it beats saying nothing.** Saying nothing would have left the operator to guess, and
a guess of "path" is as likely as "hex". Saying `<PSBT|HEX>` actively directs them to the
one input form that §8.2f exists to refuse, and the cost is a bearer transaction written
into shell history and exposed in `ps` to every user on the machine — a security event
`mt` itself describes as *"exactly as dangerous as the plate"*. A tool whose documentation
induces the leak its own refusal was built to prevent is worse than one that says nothing.

---

### [Important] 3 — a mnemonic, a descriptor, or a text file is reported as "elided `mt1` lines"

**The step.** Someone reaches for `mt` with the wrong artifact — the `md1`/`mk1` sibling
string, a BIP-39 phrase, or a file they hoped was the right one.

**What the operator has.** Something that is not an `mt1` string and never was.

**What mt did.** All three get the same refusal:

```
$ mt decode --in mnem.txt --bitcoin-cli /nonexistent      # "abandon abandon … about"
mt decode: REFUSED — §3b, all 1 lines are elided; no prefix to restore

  An elided line carries only its index and payload — the set's
  invariant prefix was cut once, on another line. Without at least one
  full string there is nothing to restore from, and mt will not guess.

  Add the 8 characters following `mt1` on any intact string of the
  same set. (They are recoverable by search if that string is lost —
  mt v0.1 does not implement the search.)
```

`md1` string → `all 1 lines are elided`. 6 lines of random base64 → `all 6 lines are
elided`.

The cause is that nothing is ever tested for being plausibly `mt1`. `read()` treats any line
with no `mt1` in it as one whole candidate (`read_strings.rs:34–37`), and `restore_elided`
declares the set elided if none of them starts with `mt1`:

```rust
// crates/mt-cli/src/read_strings.rs:163
let full = candidates.iter().find(|s| s.starts_with("mt1"));
let Some(full) = full else { /* "all N lines are elided" */ };
```

There is no charset check anywhere on that path.

**What it should have done.** Give the message it already gives for an empty file — which
is exactly right for this case, and which the operator never sees because it fires only
when zero candidates survive:

```
mt decode: REFUSED — §1.1e, no strings found in the input

  mt splits input on line breaks, and on each `mt1` prefix within a
  line. Nothing in this input looked like either.
```

The discriminator is cheap and total: an elided line is bech32-charset-only, and the
charset `qpzry9x8gf2tvdw0s3jn54khce6mua7l` excludes `b`, `i`, `o` and `1`. Every case above
fails it — the mnemonic on `b`/`o`, the descriptor on `(`, the base64 on `+`/`/`/`=`. A
genuinely elided set passes it, so the existing §3b message keeps firing where it is
correct (verified: an all-elided set with the full string removed still gets it, and it is
the right message there).

**Classification: refusal** (the refusal is right; the stated reason and the remedy are
wrong).

**Why it beats saying nothing.** This is the brief's *"a message they cannot act on"* in
its literal form: there is no *"intact string of the same set"*, because there is no set.
The operator is told, with specificity and confidence, to go find eight characters that do
not exist — and in the mnemonic case `mt` has just described a **seed phrase** as an `mt1`
string missing its prefix, which is a false statement about a security-critical object. The
correct message is already written, already tested, and one predicate away.

---

### [Minor] 4 — `mt verify` and `mt inspect` announce themselves as `mt decode`

**The step.** The 2040 recoverer runs `mt verify` on a set whose one full string is lost.

**What mt did.**

```
$ mt verify  --in elided_lost1.txt   ->  mt decode: REFUSED — §3b, all 8 lines are elided…
$ mt inspect --in elided_lost1.txt   ->  mt decode: REFUSED — §3b, all 8 lines are elided…
$ mt decode  --in elided_lost1.txt   ->  mt decode: REFUSED — §3b, all 8 lines are elided…
```

All three verbs print `mt decode:`. The cause is that `read_strings::read()` takes no verb
and hardcodes the string (`read_strings.rs:48`, `:167`, `:186`), unlike every other refusal
site, which threads `verb` through.

**What it should have done.** Thread the verb, as `explain_failure` and `read_input`
already do.

**Classification: documentation only.** **Earns a change** — but Minor: the message *body*
is correct and actionable, so the operator is misled about which command they ran, not
about what to do. It is one parameter.

---

### [Minor] 5 — "chunk 5 is missing" when chunk 4 arrived twice

**The step.** Working from a stack of nine plates, the recoverer types one plate twice and
skips the next — the single most likely mechanical slip in the whole procedure. Nine lines
typed.

**What mt did.**

```
$ mt verify --in c11_dup_skip.txt --bitcoin-cli /nonexistent
mt verify: REFUSED — §1.1, the set does not verify

  chunk 5 of 9 is missing
```

True — chunk 5 genuinely is absent from the input. But `mt` accepted a byte-identical
duplicate of chunk 4 on the way (the `duplicates` path), and *"one chunk arrived twice
while its neighbour is absent"* is the exact fingerprint of a double-type-and-skip.

**What it should have done.** Add one clause: *"chunk 4 arrived twice. If you are working
from a stack, check whether you typed one plate twice and skipped another."*

**Classification: warning.** **Earns a change**, at Minor: unlike Important 1 the statement
is *true*, so the operator is not misdirected — merely left to find the cause themselves,
which they will, by counting. The hint is nearly free because the duplicate is already
detected.

---

### [Minor] 6 — a pristine plate is nominated for re-cutting

**The step.** The recoverer typed one plate twice; both copies are perfect.

**What mt did.**

```
DUPLICATE RESOLVED. chunk 4 was present twice.
  KEPT       the copy needing 0 of 4 corrections
  DISCARDED  the copy needing 0 of 4 corrections
  Both carry the same payload, so nothing is ambiguous. mt kept the
  healthier copy — but the discarded plate has 4 corrections left
  before it is unrecoverable, and it is the one to re-cut.
```

The discarded copy needed **0** corrections and has its **full** budget. Advising a re-cut
is a 21-minute instruction with nothing behind it, and it contradicts the two lines above
it.

**What it should have done.** Emit the re-cut advice only when `discarded_corrections > 0`.

**Classification: default.** **Earns a change**, at Minor — the two health lines immediately
above contradict the advice, so an attentive operator sees the plate is fine; the cost is
wasted time, not a wrong recovery.

---

### [Minor] 7 — nothing says to destroy the file you engraved from

**The step.** The end of Journey B. `mt encode --in tx.psbt > plates.txt`, nine plates cut
over about three hours, done.

**What the operator has.** `plates.txt` — which `mt` itself calls bearer: *"A finalized
transaction — **and the mt1 strings it becomes** — is BEARER: anyone who reads it can
broadcast it."*

**What mt did.** Warned, in detail and unprompted, that the **input** file was mode 0644
and that anyone who can read it can broadcast it. Said nothing about the output. `mt` uses
`IsTerminal` for stdin (`blocks.rs:22`) but never for stdout, so it does not know the
strings were redirected — but it does know they are bearer, because it says so.

**What it should have done.** Warning 5 (*"when you are done, verify the ENGRAVING"*) is the
natural home: it is already the end-of-procedure block, and already tells them to type the
strings back from steel. One line — *"then destroy any file you redirected these strings
to; it is bearer"* — closes the loop `mt` opened with the 0644 warning.

**Classification: warning.** **Earns a change**, at Minor: `mt` has already said twice that
the strings are bearer, so the baseline is not silence and the operator is not misled — the
specific disposal step is simply absent at the one moment it belongs.

---

## Divergences I walked that earned NO change

Recorded so the next walker does not re-derive them.

1. **`mt` bare prints help and exits 2.** Correct and immediate. No hang.
2. **`mt encode` on a TTY with nothing typed.** Prompts, names all three formats, gives the
   keystroke, and says *"Nothing has happened yet."* The historical hang class is closed.
3. **`--quiet` drops CUT, PREFIX and the SUGGESTED LEGEND.** It keeps all five warnings,
   exactly as its help promises. The operator explicitly asked for the report to be
   suppressed and the legend is part of that report; the safety-critical content survives,
   and one re-run without the flag brings the legend back. Not worse than saying nothing —
   they asked.
4. **Strings carry no visible index 1..9.** Order is irrelevant on recovery (proved: a
   reversed set verifies), the index lives in each header, and string 9 is identifiable as
   the short one. Numbering stdout would corrupt the artifact.
5. **The legend does not state the plate count.** The count is recoverable from any single
   string's header, and `mt` reports it (`chunk 5 of 9 is missing`) without the legend.
6. **The legend does not name a tool or a URL.** `FORMAT: mt1 codex32` is the searchable
   token; a URL engraved in 2026 is a worse bet than the format name in 2040.
7. **Uppercase, reversed, space-joined and fully concatenated input all recover.** Nothing
   to fix; the no-separator blob is a deliberate and good design (`mt1` cannot occur inside
   a payload because `1` is outside the bech32 charset).
8. **Refusal exit codes.** All refusals exit `1`; success exits `0`; `decode` writes nothing
   to stdout when it refuses. A `mt decode | bitcoin-cli sendrawtransaction` pipeline cannot
   broadcast a partial recovery. Checked across eight inputs.
9. **`decode` prints the full report, not a summary.** Deliberate, and documented at
   `main.rs:818` with the reason: the recoverer reaches for `decode` first and *"the next
   command they plausibly type is `sendrawtransaction`"*.
10. **`mt` never tells the recoverer the literal `sendrawtransaction` command.** Correct
    restraint — the transaction is bearer, the report deliberately front-loads destination,
    amount and locktime *before* anything broadcastable, and printing the command would
    invite the act before the reading.
11. **Empty file, directory, and nonexistent path** all produce accurate, distinct,
    actionable refusals including the OS error.
12. **Unsigned PSBT and SIGHASH_NONE are refused at encode** with the funds consequence
    spelled out (*"the legend's TO line becomes false"*).
13. **A 2 GB file was not run.** Classification is decided by the first bytes and the
    line-splitter; it is the same code path as the random-text file. Running it would have
    measured `read_to_string`, not a journey moment.
14. **Height vs timestamp locktime.** Both correct (`LOCKED TO BLOCK 900000`;
    `LOCKED UNTIL 2030-03-17T17:46Z`), and both propagate into the legend.
15. **Encode determinism.** Three runs byte-identical, and the finalized PSBT and extracted
    raw hex produce identical strings — so an operator who re-runs mid-cut is safe. Worth
    keeping a test on.
16. **`--transaction` on `decode` and `inspect`** while its help says *"`verify` only"*. It
    is accepted and honoured on all three. Harmless: the comparison it performs is
    meaningful on every verb, and refusing it would be a regression. Only the doc sentence
    is stale — folded into Important 2 rather than counted twice.
17. **Warning ordering on a TTY.** stdout lands last, so the strings are the final thing on
    screen. Right as-is; do not "fix" it.
18. **`locked to block 0` on a `NO TIMELOCK` transaction** in `decode`'s offline block —
    this is **prior Minor 3**, still open, deliberately not re-reported.

---

## What I did not cover

- **Node-attached behaviour.** No `bitcoind` was reachable; every run was forced offline.
  The live-node path has its own prior review (`R-post-impl-live-node.md`).
- **Correctness of BCH correction, the codec, and `t = 4` bounds.** Out of lens, and
  covered by 160 tests plus four prior reviews.
- **The known-and-filed items** named in the brief (mainnet address rendering on regtest
  fixtures, the spec's `~FALL 2034` example, `mt qr`, script validity).
