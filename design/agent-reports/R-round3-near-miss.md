# R-round3-near-miss — do the new guards fire when they should not, or fail to fire when they should?

Independent adversarial review. Reviewer wrote none of this code.
Scope: `0ffbc79..HEAD` plus the named guards from `681d3d7` (`length_report`'s
ambiguous branch, `final_chunk_seen_at_modal_length`).
Method: `cargo build`, then `./target/debug/mt` against constructed inputs, all
offline via `--bitcoin-cli /nonexistent`. Every finding below was reproduced;
none is reported from reading alone. Scratch files live in `/tmp/mtr`, nothing
was written into the repo except this file.

## Verdict

NOT SAFE — 0C / 8I / 4M.

## Findings

### [Important] 1 — the malleability caveat fires on SegWit transactions, where the txid cannot be malleated

**Where** `crates/mt-cli/src/blocks.rs:248` (`malleability_caveat`), called
unconditionally from `crates/mt-cli/src/report.rs:547`.

**What** `no_node_warning` takes `(&Lock, &txid)` — it has no access to the
transaction — so the caveat is appended to *every* no-node recovery report. Its
body asserts a mechanism: *"A legacy input's signature does not commit to its own
encoding, so a transaction can be altered in flight and confirm under a DIFFERENT
txid."* For a transaction with no legacy inputs that is simply false: the
scriptSig is empty, the txid commits to everything a third party could touch, and
"the explorer does not know this txid" means it was never broadcast.

**The near miss** The `even` test vector — one P2WPKH input, `witness_items: 2`,
the ordinary modern case and the one this tool will overwhelmingly be used on.
mt tells its recoverer the money has probably already moved under another name
and to go looking for the destination address, at the exact moment the correct
action is *broadcast the engraving*. The tool already distinguishes legacy inputs
(`validate.rs:393 legacy_unbound_warning`), so the fact is in hand and unused.

**How I reproduced it**
```sh
cd /tmp/mtr && python3 -c "
import json; d=json.load(open('/scratch/code/shibboleth/mnemonic-transaction/crates/mt-codec/src/test_vectors/mt1_v1.json'))
open('even.txt','w').write('\n'.join(d['vectors'][0]['strings'])+'\n')"
mt decode --bitcoin-cli /nonexistent < even.txt 2>&1 >/dev/null | tail -6
```
Prints the caveat. The raw hex for that vector is
`02000000` **`0001`** `01…` — SegWit marker/flag, one witness input, zero legacy
inputs.

**Suggested fix (non-authoritative)** Thread the legacy-input predicate into
`no_node_warning` (or into `Report`) and emit the caveat only when at least one
input has an empty witness. For an all-SegWit transaction the honest line is the
opposite one: *this txid cannot change; if an explorer does not have it, it was
not broadcast.*

---

### [Important] 2 — the new §8.2e "fetched" branch says the fee is real when the operator asserted it

**Where** `crates/mt-cli/src/main.rs:514` — `let body = if fee_sat.is_some()`.

**What** The fold split the raw-transaction warning in two because the old text
claimed mt "cannot see any input's value" in runs where it had. The new branch
gates on `fee_sat.is_some()` — *is the fee computable* — and then asserts *how* it
was obtained: **"mt fetched each input's value instead, so the fee above is
real."** But `fee_sat` is `Some` whenever the values arrived from any source,
including `--input-value` with no node in sight. `Provenance::is_verified()`
already exists, is the correct discriminator, and is used by the report two rows
above.

**The near miss** The air-gapped posture this constellation is built for: a raw
signed transaction, no node, values supplied by hand. mt prints
`OPERATOR-ASSERTED` twice and `FEE … (CLAIMED — no input value verified)`, then
ten lines later upgrades the same number to "real". This is R6 adversarial I-5
(quoted verbatim in `Provenance`'s own doc comment) reappearing in the warning
text instead of the report.

**How I reproduced it**
```sh
mt encode --in /tmp/mtr/fin.hex --bitcoin-cli /nonexistent \
   --input-value 0:4.0 --input-value 1:4.0 >/dev/null
```
(`fin.hex` is `p5_base.json`'s `raw_hex`.) Same run, stderr:
```
FEE       0.00100000 BTC   (CLAIMED — no input value verified)
INPUTS    2 input(s)
            d13d7cbbd0b32d3d…   4.00000000 BTC   OPERATOR-ASSERTED
            b6c8e4075b8481b7…   4.00000000 BTC   OPERATOR-ASSERTED
...
WARNING: this is a RAW TRANSACTION, not a PSBT.
  mt fetched each input's value instead, so the fee above is real.
```

**Suggested fix (non-authoritative)** Three branches, gated on provenance, not on
`is_some()`: fetched-from-node → "the fee above is real"; asserted/claimed →
"the fee above is computed from values YOU supplied and nothing checked them";
absent → the existing UNKNOWN text.

---

### [Important] 3 — "EVERY PLATE IS ACCOUNTED FOR. Nothing is lost" counts lines typed, not chunks present — and the stack hint is dropped in exactly the case it was written for

**Where** `crates/mt-cli/src/main.rs:1127` (`let accounted = match count`), and
`main.rs:1096` (`stack_hint`) never reaching the `main.rs:1111`
`if !unreadable.is_empty()` branch.

**What** The guard tests `strings.len() >= n`. `strings.len()` is *how many lines
the operator typed*, not *how many distinct chunks are present*. Type one plate
twice and skip the next — the single mechanical slip the stack hint was written
for — and the count still reaches `n`, so mt asserts that nothing is lost. The
duplicate-chunk hint, computed live four lines earlier, is appended only to the
fall-through `Refusal` at the end of the function, so the one message that would
have explained the situation is the one branch it does not reach.

**The near miss** Working from a stack: plate 2 typed twice, plate 3 skipped,
plate 5 damaged past `t = 4`. Chunk 3 is genuinely absent, and mt says the
opposite, categorically.

**How I reproduced it**
```sh
cd /tmp/mtr && python3 - <<'EOF'
e=open('even.txt').read().split()
a=[e[0],e[1],e[1],e[3],e[4],e[5]]          # plate 2 twice, plate 3 skipped
t=list(a[4]); n=0
for i in range(20,90,7):
    if n==5: break
    t[i]='q' if t[i]!='q' else 'p'; n+=1   # plate 5: 5 substitutions
a[4]=''.join(t)
open('dup_dmg.txt','w').write('\n'.join(a)+'\n')
EOF
mt verify --bitcoin-cli /nonexistent < dup_dmg.txt
```
Output:
```
mt verify: REFUSED — §1.1, string 5 could not be read: more than 4 characters differ …
  You typed 6 strings and this set has 6 chunks, so EVERY PLATE IS
  ACCOUNTED FOR. Nothing is lost — one of them is damaged past what
  BCH can repair.
```
No mention of the duplicate. The control — the same slip with nothing damaged —
does print the hint (`mt verify < dup_only.txt` → "Chunk 2 arrived TWICE"), which
is what makes the omission a branch bug rather than a missing feature.

**Suggested fix (non-authoritative)** Count *distinct readable `header.index`
values* against `n`, not `strings.len()`; and append `stack_hint` to this branch
as well as the fall-through.

---

### [Important] 4 — "mt cannot tell how many chunks this set should have" is printed when mt can tell

**Where** `crates/mt-cli/src/main.rs:1134` — the `_` arm of the same `match count`.

**What** The arm handles two different states with one message: *count unknown*,
and *count known but fewer strings typed than chunks*. In the second state
`count` is `Some(6)` — read from the strings that decoded fine — and the printed
sentence is false. Worse, the true state (a plate is **missing** as well as
damaged) is never named, so the operator re-reads the damaged plate, succeeds,
and fails again.

**The near miss** Five plates of a six-plate set typed, one of them damaged past
`t = 4` — a plausible pairing, since the plate you cannot find and the plate you
misread are independent events.

**How I reproduced it**
```sh
cd /tmp/mtr && python3 - <<'EOF'
e=open('even.txt').read().split()
a=e[:5]                                    # plate 6 absent
t=list(a[3]); n=0
for i in range(20,90,7):
    if n==5: break
    t[i]='q' if t[i]!='q' else 'p'; n+=1
a[3]=''.join(t)
open('short_dmg.txt','w').write('\n'.join(a)+'\n')
EOF
mt verify --bitcoin-cli /nonexistent < short_dmg.txt
```
→ `mt cannot tell how many chunks this set should have, because the damaged
string is where that count is written.` Four readable strings each carry
`count = 6`.

**Suggested fix (non-authoritative)** Split the arm:
`Some(n) if distinct < n` → *"you typed N strings and this set has n chunks — a
plate is missing AS WELL as damaged"*; `None` → the existing text.

---

### [Important] 5 — the ambiguous branch covers one suspect only, so a second failure makes mt invent a character deficit on the legitimate short final chunk

**Where** `crates/mt-cli/src/read_strings.rs:240` — `let ambiguous = suspect.len() == 1 && …`.

**What** `final_chunk_seen_at_modal_length` is consulted only to decide the
*headline*, and only when exactly one string is suspect. The per-string list at
`read_strings.rs:211-222` is computed from `modal` unconditionally. So as soon as
a second string fails, the set's legitimately short final chunk is listed with a
fabricated, specific, actionable count — under a remedy that says *"Re-read these
from the plate, counting characters"*.

**The near miss** The `uneven` vector (8 chunks, final string 79 characters by
design): one middle string has a dropped character, and the final string has five
substitutions — a length error and a read error on two different plates, which is
not exotic.

**How I reproduced it**
```sh
cd /tmp/mtr && python3 - <<'EOF'
u=open('uneven.txt').read().split()
a=list(u)
a[3]=a[3][:40]+a[3][41:]                   # string 4: one character DROPPED
t=list(a[7])
for i,ch in [(20,'k'),(30,'r'),(40,'s'),(50,'t'),(60,'w')]: t[i]=ch
a[7]=''.join(t)                            # string 8: 5 substitutions, length intact
open('un_two.txt','w').write('\n'.join(a)+'\n')
EOF
mt verify --bitcoin-cli /nonexistent < un_two.txt
```
Output:
```
  string 4: 84 characters (expected 85) — 1 character is MISSING
  string 8: 79 characters (expected 85) — 6 characters are MISSING
```
String 8 is exactly as engraved. The body two lines above states the rule that
makes its own accusation wrong: *"every string but the last carries the same
payload, so exactly one may be shorter."*

**Suggested fix (non-authoritative)** Apply `final_chunk_seen_at_modal_length`
per suspect, not as a whole-refusal gate: when it is false, the single shortest
suspect gets no delta claim regardless of how many others failed.

---

### [Important] 6 — the separator repair only helps a string with no second defect; otherwise the string is silently lengthened by 11 and accused of having EXTRA characters

**Where** `crates/mt-cli/src/read_strings.rs:72` — the repair is kept only if the
repaired string decodes, and `restore_elided` (`read_strings.rs:293`) then treats
the un-repaired `mtl…` line as elided and prepends the 11-symbol prefix.

**What** The guard added in `5462bab` is correct about not corrupting a
legitimate elided line (verified — see the clean list). But its fall-through is
the very misdiagnosis the repair was written to prevent, now with the direction
inverted: the operator is told to remove ten characters from a plate that is one
character short.

**The near miss** A misread separator plus any second defect the repair cannot
absorb — a dropped character, or more than four substitutions. Both are exactly
the conditions under which the operator is already struggling.

**How I reproduced it** (non-elided set)
```sh
cd /tmp/mtr && python3 - <<'EOF'
lines=open('even.txt').read().split()
s='mtl'+lines[2][3:]        # separator misread
s=s[:40]+s[41:]             # plus one character dropped
lines[2]=s
open('sep_drop.txt','w').write('\n'.join(lines)+'\n')
EOF
mt decode --bitcoin-cli /nonexistent < sep_drop.txt
```
The plate carries 86 characters. Output:
```
  string 3: 97 characters (expected 87) — 10 characters are EXTRA
```

And on an `--elide-prefix` set, the same damage on line 1:
```sh
python3 -c "
l=open('/tmp/mtr/even_elided.txt').read().split()
s='mtl'+l[0][3:]; s=s[:40]+s[41:]; l[0]=s
open('/tmp/mtr/el_sep_drop.txt','w').write('\n'.join(l)+'\n')"
mt verify --bitcoin-cli /nonexistent < el_sep_drop.txt
```
→ `REFUSED — §3b, all 6 lines are elided; no prefix to restore`, with the remedy
*"Add the 8 characters following `mt1` on any intact string of the same set"* —
there is no other full string in an elided set, and the 8 characters are on the
plate in the operator's hand.

(The clean-separator case is fixed and works: `mtl` on one line of an otherwise
intact set decodes byte-identically. The gap is only the compound case.)

**Suggested fix (non-authoritative)** When the repaired candidate does not decode
*and* the original does not either, prefer the repaired form for the purpose of
classification if its length matches the set's modal full length rather than the
elided length — or, more simply, exclude any candidate beginning `mt` + a
separator confusable from the elided-line population, so the length report talks
about what the operator typed.

---

### [Important] 7 — `b` has two in-alphabet originals; the autocorrect tries one, lies about what was read, and spends a BCH repair on its own guess

**Where** `crates/mt-cli/src/read_strings.rs:126` — `(n, 'b') if n > 2 => Some('6')`.

**What** The doc comment claims *"any of them is a misreading of something else,
and there is only one candidate each."* mt's own refusal remedy
(`main.rs:1157`) disagrees on the same page: *"Confusable pairs to check first:
0/o, 1/l/i, **b/6**, 2/z, 5/s, **8/b**."* When the plate says `8` and the
operator writes `b`, the autocorrect substitutes `6`, and two things follow.

**The near miss (a) — the margin report names a symbol that was never engraved
and never typed.** `corrected_from` is captured *after* the rewrite, so the row
reads `read 6`. Its doc comment says it "is also the only way to tell a mis-cut
from a mis-READ: if the steel really says `d`, the plate is fine and the typist
slipped." Here the steel says `8`, the typist wrote `b`, and mt reports `6` —
neither question is answerable from the evidence printed.
```sh
cd /tmp/mtr && python3 -c "
l=open('even.txt').read().split(); s=list(l[0]); s[6]='b'; l[0]=''.join(s)
open('b8.txt','w').write('\n'.join(l)+'\n')"
mt verify --bitcoin-cli /nonexistent < b8.txt
```
→ `pos   7   read 6, corrected to 8`

**The near miss (b) — a set inside the `t = 4` budget is refused.** The wrong
guess is itself an error, so it consumes one of the four repairs.
```sh
cd /tmp/mtr && python3 - <<'EOF'
b=open('even.txt').read().split()
t=list(b[0])
for i,ch in [(20,'k'),(30,'r'),(40,'s'),(50,'t')]: t[i]=ch   # 4 genuine errors
open('four_only.txt','w').write('\n'.join([''.join(t)]+b[1:])+'\n')
t[6]='b'                                                      # + the plate's 8 read as b
open('four_plus_b.txt','w').write('\n'.join([''.join(t)]+b[1:])+'\n')
EOF
mt verify --bitcoin-cli /nonexistent < four_only.txt    # OK, 4 of 4 symbols, NO MARGIN LEFT
mt verify --bitcoin-cli /nonexistent < four_plus_b.txt  # REFUSED
```
The refusal states *"string 1 could not be read: more than 4 characters differ
from what was engraved"* — false. Exactly four differ; the fifth difference is
mt's own. And the remedy then advises checking `8/b`, naming the reading it
declined to try.

**Suggested fix (non-authoritative)** For `b`, try both `6` and `8` (and keep
whichever decodes; refuse if both do, which cannot happen in practice at 65
checksum bits). Alternatively, and more generally: capture `corrected_from`
against the operator's *original* string, and treat a rewritten symbol as
provisional so the repair is retried with the alternative candidate before the
set is refused.

---

### [Important] 8 — `--json` was wired into `inspect` only; it is still inert on `encode`, `decode` and `verify`

**Where** `crates/mt-cli/src/main.rs:82` (`ReadArgs::json`),
`crates/mt-cli/src/main.rs:156` (`EncodeArgs::json`); the only read is
`main.rs:1026`, inside `inspect`.

**What** `render_json`'s own doc comment states the defect precisely: *"The flag
parsed and did nothing, so a caller who passed it got prose and no error — worse
than the flag not existing, because a script that asks for machine output and
receives human output will parse something out of it."* That remains true of
three of the four verbs that advertise the flag, with `--json  Machine-readable
report` still in each one's `--help`.

**The near miss** `mt verify --json` is the most obviously scriptable verb in the
tool, and it emits nothing on stdout at all with the flag set. `mt decode --json`
emits raw hex. `mt encode --json` emits the strings and a prose report on stderr.

**How I reproduced it**
```sh
mt verify --json --bitcoin-cli /nonexistent < /tmp/mtr/even.txt   # no stdout
mt decode --json --bitcoin-cli /nonexistent < /tmp/mtr/even.txt   # raw hex, unchanged
mt encode --json --in /tmp/mtr/fin.hex --bitcoin-cli /nonexistent # prose on stderr
```
(Same class, not part of this diff and not counted as a finding: `--quiet` is
honoured only at `main.rs:543` (encode) and `main.rs:858` (decode) — it is inert
on `verify` and `inspect`. `mt inspect --quiet` still prints the full report.)

**Suggested fix (non-authoritative)** Either wire `render_json` into the other
verbs, or remove the flag from the arg structs that ignore it. A flag that parses
and does nothing is the thing the fold set out to delete.

---

### [Minor] 9 — the `'I'` arm of `positional_autocorrect` is dead

**Where** `crates/mt-cli/src/read_strings.rs:121` — `(2, 'l' | 'i' | 'I') => Some('1')`.

**What** `read()` lowercases every candidate at `read_strings.rs:34` before
either autocorrect runs, so `'I'` cannot reach index 2. The behaviour is correct
(uppercase `MTL…` is repaired — verified), but the arm and the doc comment's
*"an `l`, `i` or `I` there"* describe a path that does not exist.

**How I reproduced it** `mt verify < upper.txt` where line 1 is
`('MTL'+s[3:]).upper()` → OK. Reaching the `'I'` arm requires a caller that skips
the lowercasing, and there is none.

**Suggested fix (non-authoritative)** Drop `'I'` from the pattern, or note in the
comment that it is defensive against a future non-lowercasing caller.

---

### [Minor] 10 — the redirected-output warning describes a file whenever stdout is not a terminal

**Where** `crates/mt-cli/src/blocks.rs:269`, gated at `main.rs:594` by
`!is_terminal(stdout)`.

**What** A pipe is not a terminal and not a file. `mt encode < tx | less` prints
*"the file you just wrote is BEARER … DESTROY IT: `shred -u <file>`"* when no
file exists. Conservative in the right direction, but a false statement, and it
teaches the operator that the block is boilerplate.

**How I reproduced it**
`mt encode --in /tmp/mtr/fin.hex --bitcoin-cli /nonexistent 2>&1 >/dev/null | grep BEARER`

**Suggested fix (non-authoritative)** Soften to "the destination you redirected
to", or detect `S_ISREG` on fd 1 and only then name a file and `shred`.

---

### [Minor] 11 — the `mt1 SET` row prints the set id in hex, which is not the form on the steel

**Where** `crates/mt-cli/src/report.rs:328` — `"mt1 SET   {n} strings, 1..{n} all present, set {id:#07x}"`.

**What** The formatting is right (`{:#07x}` is exactly 5 hex digits for a 20-bit
id — verified for `0x2dcf2`, `0x4665e`, `0x11dc1`). The rationale is not: the doc
comment says the row exists so "someone holding two engravings" has something "to
match against their steel". What is on the steel is the bech32 invariant prefix,
which `encode` prints as `PREFIX    all 9 strings begin mt1pgej7qqg`
(`main.rs:567`) and `decode`/`inspect` never print. `set 0x4665e` is also a
verbatim restatement of the first five characters of the `TX` row on the very
next line.

**Suggested fix (non-authoritative)** Print the invariant prefix on the SET row
instead of (or beside) the hex id, so the recovery-time row and the encode-time
row name the same string the operator can read off the plate.

---

### [Minor] 12 — `txid_paste_guard` is on `encode` only

**Where** `crates/mt-cli/src/main.rs:1290`, called once at `main.rs:254`.

**What** The guard is sound and cannot fire on a legitimate transaction (see the
clean list). But pasting a txid into `mt decode`/`verify`/`inspect` is at least
as likely as pasting one into `encode` — the recoverer is the one with an
explorer open — and there it falls through to the §1.1e/§3b messages, one of
which is actively wrong.

**How I reproduced it**
```sh
echo -n 2dcf2b973d52044b1e58c988a5a59d388073ff05598b0a1e93eeb04c72ebf630 | mt decode --bitcoin-cli /nonexistent
# → "this input is not an mt1 set (1 line(s), none of them mt1)"  — acceptable
echo -n 2dcf2ae73d52044ae58c988a5a59d388073ff05598ae0a3e93eeaf4c72eaf630 | mt decode --bitcoin-cli /nonexistent
# → "all 1 lines are elided; no prefix to restore"  — a txid containing no 'b' and no '1'
#   is entirely inside the bech32 charset, so it is classed as an elided mt1 line
```

**Suggested fix (non-authoritative)** Call `txid_paste_guard` from `read_input`
so all four verbs share it.

---

## What I tried against each new guard and found CLEAN

**`read_strings` step 2b — the separator repair (`read_strings.rs:71`)**
- A legitimate **elided** line whose first three symbols really are `m`,`t`,`l` —
  the exact regression `5462bab` fixed. Built by overwriting `even_elided[1]`'s
  first three characters with `mtl` (3 substitutions, inside `t = 4`). mt did
  **not** promote it: the prefix was restored and BCH repaired all three
  (`pos 12 read m → q`, `pos 13 read t → q`, `pos 14 read l → p`), verify OK.
  The `decode`-before-keep guard holds.
- The same on `mti…`: unreachable by construction, because `i` is not in the
  bech32 alphabet, so an elided line can never begin `mti`. Only a genuinely
  misread separator can.
- Structural bound on a false promotion: for the repaired form to be kept it must
  pass a 13-symbol (65-bit) BCH checksum at a length 11 symbols shorter than a
  full string. Combined with the ~2⁻¹⁵ chance of an elided line beginning `mtl`,
  this is not reachable.
- Clean `mtl` separator on one line of an otherwise intact set: decodes
  byte-identically to the undamaged set (`diff` against `even.out`, no
  differences).
- Uppercase `MTL…`: lowercased first, repaired, verify OK.

**`positional_autocorrect` (`read_strings.rs:115`)**
- *Does it touch a string that already parses?* No. Verified two ways: the
  `decode_chunk(&s).is_ok()` early return at `read_strings.rs:88`, and by
  decoding the clean `even` and `even_elided` sets and diffing the emitted hex —
  byte-identical to the vector's `raw_hex`.
- *Index 2 on a restored string.* Every string reaching the final map begins
  `mt1` (either as typed or via `restore_elided`), so index 2 is always `1` and
  the `(2, 'l'|'i'|'I')` arm cannot fire on it. `--elide-prefix` input included.
- *Can it produce a different valid string carrying the wrong bytes?* 3,000
  randomised trials: one out-of-alphabet confusable (`1`/`i`/`o`/`b`) placed at a
  random data position, plus four ordinary substitutions, on a random string of
  the 6-string set; `mt decode` run on each and stdout compared to the clean hex.
  **81 accepted, 2,919 refused, 0 wrong bytes.** The 2.7% acceptance rate matches
  the ~1/32 chance that the confusable's mapping happens to be the true symbol,
  which independently confirms the budget analysis in finding 7(b). Harness:
  `/tmp/mtr/probe.py`, 12 seeds × 250 trials.
- The 65-bit BCH checksum plus the 20-bit content-id guard (`content_id_guard`,
  `main.rs:1329`) are two independent backstops on a miscorrection; I did not
  find a way past either.

**`length_report` ambiguous branch / `final_chunk_seen_at_modal_length`**
All four corner cases behave correctly:
- `uneven`, dropped character in the **final** string → **ambiguous** fires
  ("string 8 did not read, and it is the only one shorter than 85 — which is also
  what a final chunk looks like"). Correct: the final chunk of that set really is
  short, and mt cannot decide.
- `uneven`, dropped character in a **middle** string → **definite** branch.
  Correct: two strings are below modal (the damaged one and the legitimate final
  one), so the ambiguity does not arise.
- `even`, dropped character in the **final** string → **ambiguous** fires.
  Correct even though the set divides evenly: mt cannot know that until it has
  the last chunk.
- `even`, dropped character in a **middle** string → **definite** branch, because
  string 6 is readable, is `index + 1 == count`, and is at modal length. This is
  precisely the discrimination `final_chunk_seen_at_modal_length` was added for,
  and it works.
- `strings.get(pos - 1)` cannot underflow: `failed` is built as `i + 1`.

**`txid_paste_guard` (`main.rs:1290`) — what legitimate input is 64 hex characters?**
- **None reachable.** A transaction mt will accept has at least one input, hence
  ≥ 4 (version) + 1 + 41 (outpoint 36 + script len 1 + sequence 4) + 1 + 9
  (minimum output) + 4 = 60 bytes = 120 hex characters. 32 bytes of transaction
  is only constructible with zero inputs, which §8.6 refuses anyway.
- A base64 PSBT always begins `cHNidP8`, whose `H`, `N`, `P` and `s` are not hex
  digits; a binary PSBT fails `from_utf8` and the guard returns `Ok` on the empty
  string. Neither can trip it.
- The guard does **not** echo the pasted bytes, so it introduces no §8.2f-class
  bearer leak of its own.
- Whitespace-split input (a txid pasted across four lines) is still caught —
  correct, and the intended behaviour.

**`report::render_json` (`report.rs:400`) — fed to a parser, never read**
- Full 6-string set, 2 outputs, no node: `json.load` → valid.
- **One** output: valid (`fin.strings`, from `p5_base.json`'s `raw_hex`).
- **An output that is a raw script, not an address**: built an OP_RETURN output
  whose pushed data deliberately begins `0x22 0x5c` (`"` and `\`). rust-bitcoin
  renders it as ASM with the data hex-encoded —
  `"to": "OP_RETURN OP_PUSHBYTES_8 225c414243444546"` — so no quote, backslash,
  newline or control character can reach the JSON string. Valid.
- `Provenance` and `Status` are both unit-only enums, so `{:?}` yields a bare
  identifier; `Lock::legend()` is single-line for all four variants. The
  hand-rolled `esc` (backslash then quote, in that order) is correct for
  everything that can actually appear.
- `null` handling for `strings`, `set_id`, `fee`, `height`, `mtp` and per-input
  `sats`: all emitted unquoted and parsed as `None`. Comma placement on empty and
  single-element arrays is correct.

**`report::no_node_warning(&Lock, …)` — the new N/A branch (`report.rs:498`)**
- `nLockTime = 0` (`Lock::None`): report row `NO TIMELOCK`, warning row
  `has the locktime passed?  N/A` + `NO TIMELOCK … current height unknown (no
  node)`. No contradiction — this is the defect the fold fixed, and it is fixed.
- Non-zero `nLockTime` with every input final (`Lock::NotEnforced`): warning row
  `N/A` + `nLockTime 96 present but NOT ENFORCED (all inputs final)`. Correct.
- `Lock::Height`: `UNKNOWN` + `LOCKED TO BLOCK 96 … current height unknown (no
  node)`. Correct. `Chain::default()` is the right argument here — there is no
  node by construction on this path.
- Both fixtures built by re-serialising the `even` vector with a patched
  `nLockTime` / `nSequence` (`/tmp/mtr/txedit.py`) and re-encoding.

**`blocks::legend`'s `n/m` clause (`blocks.rs:187`)**
- 9-string set → `...and on EACH plate, its number:  1/9, 2/9, … 9/9`, 1-based
  and consistent with the `mt1 SET   9 strings, 1..9 all present` row and with
  `explain_failure`'s `string N` numbering.
- `count > 1` is not a live discriminator: a 1-chunk set needs a transaction
  under one chunk's payload, which is smaller than any transaction with an input.
  The guard is harmless either way.
- `--elide-prefix` does not change `strings.len()`, so the count is unaffected.

**`blocks::legend`'s amount clause (the prior round's fix), re-checked for its own near miss**
- One output + `--to alice-cold` → `TO alice-cold  7.99900000 BTC`. True: there is
  one output and the named party receives it.
- Two outputs + `--to` → no amount, plus the "mt cannot tell which is CHANGE"
  paragraph. Correct.

**`Report::set_id` / the `mt1 SET` row**
- `{:#07x}` verified against three real ids (`0x2dcf2`, `0x4665e`, `0x11dc1`) —
  always `0x` plus exactly 5 digits, the full width of a 20-bit id, with leading
  zeros preserved.
- `set.chunks[0]` cannot panic on this path: `pipeline::decode` returns an error
  rather than an empty chunk set, and `explain_failure` intercepts it.

**Raw-transaction warning, the other branch**
- Raw hex, no node, no `--input-value` → the UNKNOWN branch, which is accurate
  ("THE FEE IS UNKNOWN … 0.0001 BTC or 9 BTC"). Only the `fee_sat.is_some()`
  branch is wrong (finding 2).

**Stack hint, control case**
- Plate 2 typed twice, plate 3 skipped, nothing damaged → `chunk 3 of 6 is
  missing` **plus** `Chunk 2 arrived TWICE. If you are working from a stack…`.
  The hint is correct, well-targeted and correctly plural-safe. It is only the
  unreadable-strings branch that loses it (finding 3).
