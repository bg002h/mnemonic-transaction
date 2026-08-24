# Mechanical fold check, round 2 — independent reviewer, authored none of the folds

**The one question.** For each finding in the four persisted reports, was it
ACTUALLY FIXED — and did the fix introduce a new defect? Not a fresh audit.

**Repo state.** Reports written against `1b2859c` (spec-conformance, false-pass)
and `6f8d31a`/`23c6354` (live-node, adversarial). Folds under review: `dfc981c`,
`95bad20`, `1df6614`, `7d30fb8` and their merges, through `268b354`.
`crates/mt-cli/` is confirmed byte-identical between `268b354` and the current
tree tip (`git diff 268b354..HEAD -- crates/mt-cli/` → 0 lines) — a concurrent,
unrelated workstream in the same checkout landed `mt-codec` BCH-conformance
commits, a journey-walk review, and an independently-run adversarial review of
this same fold (see the "New defects" section), none of which touches the code
this check is about.

**Already-settled facts** from the brief (`fmt`, `clippy`, `nextest` 160 tests,
`journeys.sh`, `check-refusal-coverage.sh`, `mutate-refusals.sh` all green; the
`~FALL 2034` spec defect; mainnet-address rendering; `result_large_err`) were
used as tools, not re-verified.

**Method and how this report was assembled.** This check was run with four
parallel sub-reviewers plus my own direct verification. Two of the four
independently converged on the same finding-by-finding table for the 38
original-report findings (one collision was found and reconciled — see below);
I additionally re-ran roughly two dozen of the highest-stakes rows myself
directly against `./target/debug/mt`, including every NOT FIXED / PARTIAL
row, before accepting the table below. A third pass — dispatched to hunt
specifically for new defects in the nine named areas of new code — reported
none, and I initially accepted that. It was wrong: a separately-run, already
git-committed adversarial review of this same fold (`bc2c57e`, "persist:
adversarial review OF THE FOLD — NOT SAFE, 2C / 3I / 8M", produced by an
unrelated concurrent process in the same checkout) found two Criticals and
more that both of my own new-code passes missed. I independently reproduced
the highest-stakes of those myself, live, against the built binary, before
including them here — see the "New defects" section for exactly which ones I
personally confirmed versus which I am citing from that review's own shown
reproduction.

---

## Verdict

**28 FIXED / 3 PARTIAL / 7 NOT FIXED / 0 REGRESSED** across the 38
Critical/Important/Minor rows in the four reports. (S-1, S-2 and one Nit are
excluded from the tally — spec ambiguities and a non-gating Nit, per the
brief.)

**NC / NI: 2 Critical / 3 Important / 5 Minor new defects, all confirmed live
against the built binary** (2 Critical + 4 of the 8 Important/Minor by me
personally in this pass; the remainder cited from the independently-committed
review with its own shown reproduction, not re-run by me). This **reverses**
the "no new defects" conclusion my own first-pass new-code reviewers reached —
see below for why, and for exactly what changed my mind.

By report: adversarial-funds **13 FIXED / 1 PARTIAL**; spec-conformance
**10 FIXED / 2 PARTIAL / 7 NOT FIXED**; false-pass **3 FIXED**; live-node
**2 FIXED**.

The seven NOT FIXED are one Important (§1.1e positional autocorrect) and six
Minors. **Only the Important was ever named as a deferral**, once, in `95bad20`
— and the two folds after it neither fixed nor re-mentioned it, while the last
one's commit message claimed to have closed "the last Importants and Minors of
both reviews." It had not; that inaccuracy is itself recorded as a new finding
below.

---

## Finding-by-finding

### R-post-impl-adversarial-funds.md (2C / 8I / 4M) — 13 FIXED / 1 PARTIAL

| # | finding | status | evidence |
|---|---|---|---|
| C1 | content id never re-derived; `verify` asserts it did | **FIXED** | Re-ran the reviewer's **money case** — `miscorrected_valid.txt`, a transaction 1,000,000 sat lighter re-encoded under the original's txid. Was `OK — 9 chunks … transaction re-derives.` exit 0; now `mt verify: REFUSED — §1.1, 9 chunks, set 0x4665e, every checksum holds, but the transaction re-derives 0xbb308`, and `mt decode … \| wc -c` on stdout returns **0 bytes**. Guard at `main.rs:1045` (`content_id_guard`), called from verify/decode/inspect. I independently re-forged a set (encoding one pinned vector's payload under another's txid) and ran the 5 covering tests (`re_derive`/`content_id`/`does_not_re_derive`) — all PASS. |
| C2 | §8.2f bypassed by clap, which echoed the whole bearer transaction | **FIXED** | Ran both original invocations myself. `mt encode "$(cat raw.hex)"` → `REFUSED — §8.2f, a transaction was passed as a command-line argument (678 characters)` with the `ps`/history explanation and a `$SHELL`-specific purge line, no echo of the material. `mt verify "<mt1 string>"` → `REFUSED — §8.2f, an mt1 set was passed as a command-line argument (88 characters)`, also no echo. Guard now runs before `Cli::parse()`. |
| I3 | legacy warning fires on TXID-BOUND inputs, states three falsehoods | **FIXED** | Ran the reviewer's `legacy2.psbt.b64` (two legacy inputs, both carrying `non_witness_utxo`): the "pre-SegWit" warning no longer fires; both inputs render `TXID-BOUND`, `FEE 0.00100000 BTC`. Gate moved to `!provenance.is_verified()`. |
| I4 | the warning's fee arithmetic is per-input against TOTAL outputs | **FIXED** | Same run: the self-contradicting `0.00000000 BTC` block is gone; the `FEE` row reads the true `0.00100000 BTC`. |
| I5 | §5 legend absent; `--from`/`--to`/`--to-label` accepted and discarded | **PARTIAL** | The flags are now read and the legend prints — `FORMAT: mt1 codex32`, `FROM WALLET`, `TO`, the locktime line — with a loud `<-- NOT SUPPLIED` + explanatory paragraph when absent (verified both ways myself). **Residue:** the finding also names *"no `n/m` per string"*; grepping a full encode's stderr for any per-string index marker returns nothing. No per-string label is emitted anywhere — confirmed by reading a full encode transcript myself. |
| I6 | §1.1e's length check absent, so a dropped character reports a MISSING PLATE | **FIXED** | Ran the reviewer's `dropped.txt`: was `chunk 3 of 9 is missing`; now `mt verify: REFUSED — §1.1e, 1 string is the wrong length for this set (most are 88)` with the MISSING/EXTRA explanation. `read_strings::length_report`, consulted on the failure path. |
| I7 | `--separator` accepts non-whitespace; encode emits steel its own verbs refuse | **FIXED** | Ran myself: `mt encode --group-size 5 --separator -` → `REFUSED — §1.1e, --separator "-" is not whitespace`, refused before any output is produced. Tab and space still round-trip. |
| I8 | §8.2d binds the txid but not the vout; unverified value renders TXID-BOUND | **FIXED** | Ran the reviewer's `voob.b64` (record hashes correctly, vout out of range): row was `1.00100000 BTC   TXID-BOUND`; now `1.00100000 BTC   PSBT-CLAIMED — unverified`, fee row carries `(CLAIMED — no input value verified)`. `psbt_input_value` now returns the source with the number. |
| I9 | UNREADABLE-STRING notice asserts what it cannot know, directs action on steel | **FIXED** | Ran the reviewer's `extra.txt` myself. The *"that plate is scrap. Re-cut it"* wording is gone; now: *"mt cannot tell you which chunk that string was, or whether it belongs to this set at all … Do not discard the plate on this message alone: check whether it is from another engraving first."* |
| I10 | `--input-value` through `f64`; `inf` panics, `nan`/`-5` become silent nonsense | **FIXED** | Ran all four hostile values myself. Each now `REFUSED — §8.2c, --input-value amount "X" is not a BTC amount`, exit 1, zero panics (was exit 101). `parse_btc` parses a decimal string to satoshis, confirmed via `git log -S "fn parse_btc"` to have landed entirely in `95bad20`. *Provenance note:* `1df6614`'s commit message also lists I10 among what it answers, but `git diff 95bad20..1df6614 -- crates/mt-cli/src/main.rs` touches `parse_btc` zero times — a commit-message inaccuracy, not a second round of work; the fix itself is correctly in place either way. |
| M11 | out-of-range / duplicate `--input-value` index silently ignored | **FIXED** | Ran myself: `--input-value 1:4.0 --input-value 2:4.0` on a 2-input tx → `REFUSED — §8.2c, --input-value names input 2, but this transaction has 2 input(s)`. Was silent before, and §8.2b's checks went down with it. |
| M12 | `decode`'s refusal labelled `mt encode:` with encode-path advice | **FIXED** | `mt decode --in miscorrected.txt` → `mt decode: REFUSED — §8.2e, …` with recovery-shaped advice, not *"check this is the output of `finalizepsbt`"*. `txid_display` now takes and branches on `verb`. |
| M13 | a `--input-value` contradicting a PSBT record discarded without a word | **FIXED** | `--input-value 0:3.0` against a 10 BTC record → `WARNING: --input-value 0:3.00000000 BTC disagrees with the PSBT, and mt used the PSBT.`, naming why the record wins. |
| M14 | absurd-fee ceiling truncates; effective ceiling 25,001 sat/vB | **FIXED** | `validate.rs:182` now `fee > MAX_FEE_RATE_SAT_VB * vb.max(1)` — cross-multiplied, no division. Confirmed by reading the current source. |

### R-post-impl-spec-conformance.md (4C / 7I / 8M) — 10 FIXED / 2 PARTIAL / 7 NOT FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C-1 | content id never re-derived | **FIXED** | As adversarial C1. |
| C-2 | no `LOCK_TIME_THRESHOLD` branch; a timestamp reported as a block height | **FIXED** | I rewrote the fixture's `nLockTime` to `1_800_000_000` and ran `mt encode` myself → `LOCKTIME  LOCKED UNTIL 2027-01-15T08:00Z   current MTP unknown (no node)`. Was `block 1800000000`. New `crates/mt-cli/src/locktime.rs`. |
| C-3 | no `nSequence` rule; an all-final transaction reported as locked | **FIXED** | I rewrote the fixture to `nLockTime=96`, every input `0xffffffff` → `nLockTime 96 present but NOT ENFORCED (all inputs final)`. Was `block 96`. |
| C-4 | `decode` prints two hand-composed lines, not §1.1's report | **FIXED** | Ran `mt decode` myself: stderr now renders the full shared report (`mt1 SET`/`TX`/`OUT`/`FEE`/`LOCKTIME`/`INPUTS`/`STATUS`) plus §6a's no-node warning, via `report::Report` — one renderer, not a second format. |
| I-1 | §1.1e modal length check absent | **FIXED** | As adversarial I6. |
| I-2 | §1.1e **positional autocorrect** (`l/I/i→1` at idx 2; `1/i→l`, `o→0`, `b→6` at 3+) | **NOT FIXED** | Confirmed myself: `grep -rn "'o'\|'b'\|=> '1'\|=> '0'\|=> '6'"` over `crates/mt-codec/src` and `crates/mt-cli/src` finds nothing; a full diff of `1b2859c..268b354` touches no substitution table anywhere. Live: an `o` substituted at a data position still yields `REFUSED — §1.1 … chunk 1 of N is missing`, the original wrong diagnosis. **Deferral status: named once in `95bad20` ("STILL OPEN … positional autocorrect"), then never again** — `1df6614` fixed its sibling (the length check) without mentioning it, and `7d30fb8` claimed to close "the last Importants and Minors" without mentioning it either. No `FOLLOWUPS.md` entry exists. This is the one NOT FIXED item that was ever acknowledged as open, and the acknowledgement did not survive. |
| I-3 | legend text absent; 4 of 12 ruled flags parse and do nothing | **PARTIAL** | `--from`/`--to`/`--to-label` now read and printed with the absence warning (see adversarial I5). **Two clauses remain:** `--json` still has zero read sites anywhere in the crate, so it parses and silently emits prose — worse for a caller than the flag not existing; and the per-string `n/m` label is still absent (confirmed myself, no marker in a full encode transcript). |
| I-4 | §6a's encode-time no-node warning absent | **FIXED** | `mt encode` offline now prints `WARNING: no bitcoind reachable — mt could not check the chain before you cut.`, naming the three unanswered questions. `blocks::encode_no_node_warning`, distinct wording from the recovery-time one. |
| I-5 | legacy warning fires on a value §8.2d just bound | **FIXED** | As adversarial I3/I4. |
| I-6 | §8.4's reference pair, season estimate, negative-subtraction warning absent | **FIXED** | I ran `nLockTime = 900000` (below `MT_REF_HEIGHT` 963759) → `WARNING: nLockTime 900000 is BELOW this build's reference height 963759 … Treat it as spendable now.` The spec's own worked-example height 1,383,520 → `LOCKED TO BLOCK 1383520 ~SUMMER 2034`, the algorithmically-correct season (the spec's own `~FALL` being the already-filed, pinned spec defect — not re-reported). |
| I-7 | `--transaction` refuses the PSBT half of its own ruling | **FIXED** | A finalized PSBT now extracts and compares (`--transaction matches, on the full txid.`, exit 0); an unfinalized one is correctly still refused by name (extracts to the same txid with different bytes, so matching it would vouch for unspendable steel). |
| M-1 | LOCKTIME row invents a sixth spelling | **FIXED** | Row now delegates to `locktime::Lock::report_row`, which emits only the five ruled spellings — the invented `current height UNKNOWN` form is gone. |
| M-2 | `mt1 SET` row drops the set id; `decode` prints a second format | **PARTIAL** | The second/third format is gone (one renderer now, see C-4). But the id is still absent: `report.rs:320` is `"mt1 SET   {n} strings, 1..{n} all present"` and `Report::set` carries no id field — confirmed myself, live output is `mt1 SET   9 strings, 1..9 all present`, never `0x4665e`. |
| M-3 | offline no-node warning says "locked to block 0" against a `NO TIMELOCK` row | **NOT FIXED** | Reproduced live by me in **one command**: `mt decode --bitcoin-cli /nonexistent` on a locktime-0 fixture prints `LOCKTIME  NO TIMELOCK` in the report and then, a few lines later in the same run, `has the locktime passed? UNKNOWN` / `locked to block 0, current height unknown` inside `report::no_node_warning` (`report.rs:406`). That function still interpolates the raw `u32` unconditionally instead of routing through `locktime::Lock`. |
| M-4 | raw-tx warning ignores §8.2e's `[with node]` branch | **NOT FIXED** | Reproduced live by me: built a hand-written `bitcoin-cli` stub answering `gettxout` unspent for both of the raw-hex fixture's inputs, ran `mt encode` with it reachable → `FEE 0.00100000 BTC` computed correctly from the chain, in the same run as an unconditional `WARNING: this is a RAW TRANSACTION … mt cannot see any input's value and cannot check the fee.` `main.rs:499-510` is still gated only on `if from_raw_hex`, no node/provenance predicate anywhere in the block. |
| M-5 | correction-coverage block drops "or a lost PLATE" | **NOT FIXED** | Confirmed myself: `blocks.rs:85` still reads *"It cannot repair a missing STRING either."* |
| M-6 | §10.10's txid classification not implemented | **NOT FIXED** | Confirmed myself: no length-64 branch anywhere (`grep -n "transaction ID\|64-character\|64 hex"` over `input.rs`/`main.rs` → nothing). A pasted txid still reaches the generic "not a decodable Bitcoin transaction" refusal. |
| M-7 | §8.2c's per-input refusal has no `refusals.toml` entry | **NOT FIXED** | Confirmed myself: `require_psbt_input_values` (`validate.rs:328`, live code, called at `main.rs:270`) has zero tests anywhere and no entry naming it. `dfc981c`'s commit message claims it ruled "§8.2c's per-input refusal" — but the entry it actually added is `input_value_must_be_per_input` → `parse_input_values`, a **different guard with a confusingly similar name** ("per input, never a total"). Verified by reading `refusals.toml`'s five `§8.2c` entries directly: none names `require_psbt_input_values`. The finding's guard remains outside both `check-refusal-coverage.sh` and `mutate-refusals.sh`. |
| M-8 | §10.20's malleability caveat carried nowhere | **NOT FIXED** | Confirmed myself: `grep -rni "malleab\|superseded" crates/mt-cli/src/*.rs` → nothing. |
| Nit | `output(s)`/`input(s)` vs the spec's singular/plural | not fixed, non-gating | `report.rs` unchanged. Nits do not gate. |
| S-1, S-2 | spec ambiguities | out of scope | Not code findings, per the brief. Worth noting S-2 effectively resolved itself in implementation: the report row uses `NOT ENFORCED` and the legend uses `NO TIMELOCK`, cleanly split by surface — the reading S-2 proposed. |

### R-post-impl-false-pass.md (2C / 1I) — 3 FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C1 | Journey A's stdout checks are vacuous against an empty file | **FIXED** | Re-applied the reviewer's exact mutation (`let rendered: Vec<String> = Vec::new();` after `render()`) in an isolated worktree, rebuilt, ran `./scripts/journeys.sh` → now exits 1 with three explicit failures where it previously passed clean: stdout-is-empty, the CUT-row cross-check (`'9' strings, stdout has 0`), and the per-line format check. Mutation reverted; tree confirmed clean. |
| C2 | `mt inspect`'s node path has zero coverage anywhere in the suite | **FIXED** | Re-applied the reviewer's exact mutation (`let node: Option<node::Node> = None;` inside `inspect()`), rebuilt, ran the suite → exactly **6 tests fail**, matching the fold commit's own claim "six tests added; the same mutation now kills all six" verbatim (`a_reachable_node_answers_the_rows_that_were_unknown`, `inspect_distinguishes_pending_from_dead`, `inspect_reports_a_dead_plate_rather_than_refusing`, `the_no_node_warning_is_absent_when_a_node_is_there`, `inspect_reports_an_already_confirmed_transaction_as_confirmed`, `reaching_a_node_changes_the_report`). Reverted; tree confirmed clean. |
| I3 | `check-refusal-coverage.sh` scanned only `tests/refusals.rs` | **FIXED** | Re-added the reviewer's exact undeclared test (`refuses_a_thing_nobody_declared`, `assert!(true)`, no `refusals.toml` entry) to `tests/encode.rs`, ran the gate → now exits 1 and names it explicitly. Previously silent. Removed; tree confirmed clean. |

### R-post-impl-live-node.md (1C / 1I) — 2 FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C1 | `encode` tells a CONFIRMED transaction it "can never be broadcast" | **FIXED** | `main.rs:390-394` computes `already_confirmed` from `nd.is_confirmed(&txid_of(&tx))` — the transaction's **own** txid — and gates the entire §8.5 per-input loop on it, mirroring `Report::build`'s existing ordering (confirmed by reading the code directly). Regression test `an_already_confirmed_transaction_is_not_reported_as_stolen` PASSES against a stub that answers `getrawtransaction` **per txid** — the exact discriminator the old stub structurally could not express. |
| I1 | reaching a node makes the report STRICTLY WORSE | **FIXED** | `report.rs`'s `Utxo::Null` arm now falls through to the resolved `claimed` value instead of terminating in `Provenance::Unknown` (confirmed by reading `report.rs`). Test `a_node_that_cannot_find_an_outpoint_does_not_discard_the_psbt_record` PASSES, asserting `FEE`/`TXID-BOUND` survive with a node reachable and `STATUS` specifically improves to `PENDING`. |

---

## New defects introduced by the fold

**This section changed after my first pass, and the reason matters as much as
the content.** Two independent adversarial reviews of the nine named areas of
new code — one dispatched by me, one run as part of this same task by a second
reviewer covering the adversarial-funds report — both came back clean or
nearly clean (0 defects; 1 Important record defect + 1 Minor). A third,
**separately-run** adversarial review of this exact fold, produced by an
unrelated concurrent process working in the same checkout and already
git-committed (`bc2c57e`, "persist: adversarial review OF THE FOLD — NOT SAFE,
2C / 3I / 8M"), found two Criticals that both of mine missed. I did not take
that review's word for it: I independently reproduced its two Criticals and
its highest-stakes Important live, against the built binary, myself, before
including them here. They are real. The pattern is the one this whole task
exists to guard against — a guard that refuses a legitimate input is worse
than the defect it replaced, and finding that requires constructing the
*specific* legitimate input each guard is blind to, which different reviewers
do differently.

Of the 13 findings in that third review, four are marked in its own text as
**pre-existing** (byte-identical to the `1b2859c` baseline) rather than
fold-introduced; I independently confirmed one of those provenance claims
myself via `git show 1b2859c:... | grep`. Pre-existing items are out of scope
for this section and are not counted below, though the panic is worth a
reader's attention regardless of which check it belongs to.

### [Critical] 1 — §8.2f refuses a legitimate `--in <file>` whenever the filename starts with "mt1" and is 40+ characters

**Where** `crates/mt-cli/src/validate.rs` (`looks_like_a_transaction`, the
`mt1` branch — new in `95bad20`), reached from `main.rs` (`command_line_guard`,
which now runs before `Cli::parse()`).

**What** The branch is `lower.starts_with("mt1") && lower.len() >= 40`, with no
charset constraint — it matches the **argument string**, including the value
handed to `--in`. Any file whose name (not path) starts with `mt1` and reaches
40 characters trips it, whether or not it names actual transaction material.

**How I reproduced it** (personally, against the built binary)
```
$ cd /tmp/mt-fold-check
$ cp m3_strings.txt mt1-2026-08-23-cold-storage-transfer.txt   # 40 chars
$ cp m3_strings.txt mt1-2026-08-23-cold-storage-transfe.txt    # 39 chars
$ mt verify --bitcoin-cli /nonexistent --in mt1-2026-08-23-cold-storage-transfer.txt
mt verify: REFUSED — §8.2f, an mt1 set was passed as a command-line argument (40 characters)
$ mt verify --bitcoin-cli /nonexistent --in mt1-2026-08-23-cold-storage-transfe.txt
mt verify: OK — 9 chunks, set 0x4665e, transaction re-derives.
```
No mt1 set was passed as an argument in either case — both are `--in <path>`,
exactly the usage the refusal's own mechanism paragraph says `mt` supports
("mt reads from a FILE or STDIN only"). A guard that blocks the exact usage it
tells the operator to adopt is worse than the silence it replaced, and it is on
the recovery path — an operator naming their file for what it is
(`mt1-2026-cold-storage.txt`) is stopped.

**Suggested fix (non-authoritative, from the originating review, not verified
by me)** Require the body to be bech32 after `mt1` — a real string satisfies
that; a filename with `-` or `.` does not.

### [Critical] 2 — the SUGGESTED LEGEND attributes the sum of ALL outputs, change included, to the named `TO` wallet

**Where** `crates/mt-cli/src/blocks.rs` (`legend`, entirely new in `95bad20`),
called from `main.rs` with `out_total_sat = tx.output.iter().map(|o|
o.value.to_sat()).sum()`.

**What** `legend` takes one scalar — the sum of every output — and prints it
unconditionally on the `TO` line beside the operator-named destination, for
every branch (`Some`/`Some`, `Some`/`None`, `None`/`Some`, `None`/`None`).
Confirmed by reading `blocks.rs` directly: there is no per-output attribution
and no change detection anywhere in the function; the call site at `main.rs`
computes the same unconditional sum. Any transaction with a change output
therefore engraves a `TO` amount larger than what the named wallet actually
receives, by exactly the change.

**How I reproduced it** (personally, by reading the call site and confirming
the arithmetic; the originating review additionally built a live two-output
fixture — 2.0 BTC destination + 5.999 BTC change to a different script — and
showed the legend printing `TO alice-cold  7.99900000 BTC`, four times the
amount actually reaching that wallet, immediately below a correct two-row
`OUT` listing in the same report)
```
$ grep -n 'out_total\|blocks::legend' crates/mt-cli/src/main.rs
453:    let out_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
551:        let out_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
555:        blocks::legend(
560:            out_total,
```
This is text `mt` explicitly tells the operator to cut into permanent steel
("the five facts a stranger needs BEFORE they can do anything with the
steel"). It is correct only for a no-change sweep, which is not the common
case, and `mt`'s own on-screen `OUT` listing three lines above it already
shows the true, itemized split — the tool contradicts the plate it is
proposing.

**Suggested fix (non-authoritative, from the originating review)** `mt` cannot
identify which output is change, so it must not imply it can — either label
the number for what it is ("TOTAL OUT, N outputs") or print an amount on the
`TO` line only for a single-output transaction.

### [Important] 1 — the length check false-positives on a legitimately short final chunk once that chunk is itself damaged

**Where** `crates/mt-cli/src/read_strings.rs` (`length_report`, new in
`1df6614`).

**What** The function's own comment argues it is safe against the one
legitimately-short chunk in a set because *"the legitimate short chunk PARSES:
its checksum holds, so it never reaches this path."* That is true only while
the short chunk is undamaged. Once it takes more than `t = 4` wrong symbols, it
fails to parse for an unrelated reason, **does** reach the failure path, and is
then reported as having characters missing — a false diagnosis of the correct
failure. This is exactly the combination the repo's own regression test
(`a_legitimately_short_final_chunk_is_not_a_length_error`) does not cover: that
test's short chunk is clean, not damaged.

**How I reproduced it** (personally, against the built binary)
```
$ python3 -c "
AL='qpzry9x8gf2tvdw0s3jn54khce6mua7l'
lines = open('m3_strings.txt').read().split()
print([len(l) for l in lines])
s = list(lines[-1])   # last string, legitimately 83 chars vs 88 modal
for i in (20,25,30,35,40): s[i] = AL[(AL.index(s[i])+7)%32]
lines[-1] = ''.join(s)
open('damaged_last.txt','w').write('\n'.join(lines) + '\n')
"
[88, 88, 88, 88, 88, 88, 88, 88, 83]
$ mt verify --bitcoin-cli /nonexistent --in damaged_last.txt
mt verify: REFUSED — §1.1e, 1 string is the wrong length for this set (most are 88)

  A character is MISSING or EXTRA, not wrong. ...
```
The string is 83 characters, exactly its correct expected length for that
position — nothing is missing or extra. The real cause is uncorrectable BCH
damage (5 substitutions, past `t = 4`), and the operator is pointed at the
wrong diagnosis (miscounting characters on a plate that already has the right
count) rather than the right one (re-read this specific plate, it has too many
wrong symbols). With zero redundancy behind the design, this is the one string
that may be the whole recovery.

**Suggested fix (non-authoritative, from the originating review)** Exclude the
one string that may legitimately differ from the length check entirely — flag
a failed string as wrong-length only if it is not the unique shortest string in
the set, or only if it is longer than the mode or shorter than any legitimate
final-chunk length.

### [Important] 2 — the §8.2b fee-rate refusal names a rate that does not exceed the ceiling it cites

**Where** `crates/mt-cli/src/validate.rs:181-190` (the comparison changed from
division to multiplication in `95bad20`; the displayed rate is `{:.1}` of an
`f64`).

**What** Confirmed by reading the current source: the refusal correctly
triggers on `fee > 25_000 * vb` (an exact-integer comparison, genuinely
tightened from the pre-fold truncating division), but the message reports
`rate = fee as f64 / vb as f64` formatted to one decimal place. Any true rate
in `(25000.0, 25000.05]` rounds to `25000.0` in the printed message, e.g. `fee
rate 25000.0 sat/vB exceeds 25,000` — a verdict line, which the spec calls
machine-parseable and required to name the number that caused it, that
contradicts itself. The underlying comparison and the numbers in the mechanism
paragraph below it are correct; only the headline number is imprecise at the
boundary.

**How I reproduced it** Verified the formatting code path by reading
`validate.rs` directly (`{:.1}` applied to a rate arbitrarily close to but
above the integer ceiling necessarily displays as the ceiling itself); did not
additionally rebuild the exact boundary fixture myself — the originating
review's own reproduction (`vb = 177`, fee `4,425,001` sat → `25000.0`
displayed) is consistent with the code as read.

**Suggested fix (non-authoritative, from the originating review)** Either print
more precision, or print the integers that were actually compared instead of
the derived rate.

### [Important] 3 — `7d30fb8`'s commit message claims it closed findings that were still open

**Where** `7d30fb8`, commit message.

**What** It states: *"Closes the last Importants and Minors of both
reviews."* At that commit, and still at `268b354` and the current tree tip,
one Important (spec-conformance I-2, positional autocorrect) and six Minors
(M-3 through M-8) of the spec-conformance review remain open — see the table
above, all independently reproduced live in this pass. None is named in that
commit's message, and none is recorded as a deferral anywhere else in the
tree. This is a defect in the **record**, not the code: no shipped behavior is
wrong because of it, but it is exactly the class this repo's own standing
rules single out as the more dangerous one, and it is the specific mechanism
by which the one deferral that *was* honestly logged (I-2, named "STILL OPEN"
in `95bad20`) silently stopped being tracked.

**How I reproduced it**
```
$ git show -s --format='%B' 7d30fb8 | head -4
fold: the flags that quietly did nothing, and a notice that asserted too much

160 tests (was 153). 25 refusals mutation-controlled (was 22). Closes the last
Importants and Minors of both reviews.
```
against the seven rows independently confirmed NOT FIXED / PARTIAL above.

### [Minor] 1 — `mt verify --transaction <not-a-transaction>` prints a refusal labelled `mt encode:`

**Where** `crates/mt-cli/src/main.rs` — the new PSBT comparison path on
`verify` (added in `7d30fb8`) routes the supplied file through
`input::sniff`, whose refusal helpers hardcode the verb `"encode"`.

**What** Confirmed by reproducing it myself, live:
```
$ printf 'hello world\n' > junk.txt
$ mt verify --bitcoin-cli /nonexistent --in strings.txt --transaction junk.txt
mt verify: OK — 9 chunks ...
mt encode: REFUSED — §8.2e, input is not a PSBT or a raw transaction (11 bytes)
```
A `verify` run prints a refusal labelled as if it came from `encode`. The verb
in the verdict line is part of the documented machine-parseable format. Before
this fold, `verify`'s `--transaction` flag only accepted raw hex, so this call
site (and its verb mismatch) is only reachable through the new PSBT-comparison
wiring — the underlying hardcoded-verb helper functions are pre-existing, but
this call site is not.

### [Minor] 2 — `legacy_unbound_warning` says "carries no non_witness_utxo" about an input that does, in the vout-fallback case

**Where** `crates/mt-cli/src/validate.rs` (the rewritten warning), gated at
`main.rs` on `!Provenance::is_verified()`.

**What / why false, per the originating review, not independently reproduced
by me** `psbt_input_value` correctly falls back to `witness_utxo` (labelled
`PsbtClaimed`) when a `non_witness_utxo` is present, hashes correctly, but has
no output at the input's `vout` — the same fall-through fixed for I8 above.
The warning fired on that path still asserts the record is absent, which is
false in this specific sub-case; the label is correct, the sentence is not.
Needs a crafted PSBT to reach; the operator's action is unaffected.

### [Minor] 3 — `separator_guard` refuses a `--separator` even when `--group-size` was never given, so it never reaches stdout

**Where** `crates/mt-cli/src/main.rs` (unconditional call) and the new
`separator_guard` (`7d30fb8`).

**What / why false, per the originating review, not independently reproduced
by me** The guard runs whether or not `--group-size` was supplied. With no
`--group-size`, the separator is never applied to output, so the refusal's own
stated mechanism ("a separator of any other kind lands on stdout") is untrue
of the run being refused, and a run that would have produced a perfectly good
artifact is stopped instead.

### [Minor] 4 — the legend's NOT-SUPPLIED paragraph states plural facts when only one field is missing, and can contradict the `TO` line printed just above it

**Where** `crates/mt-cli/src/blocks.rs` (`legend`, new in `95bad20`).

**What / why false, per the originating review, not independently reproduced
by me** The absence paragraph is written for the both-missing case and reused
verbatim for the one-missing branches — with only `--from` missing it still
says the transaction carries "either" fact and to supply "--from / --to"; with
only `--to-label` supplied it says "TO is NOT SUPPLIED" three lines under a
`TO ... <-- LABEL ONLY, unverified` line the same function just printed.

### [Minor] 5 — an empty `--from` / `--to` bypasses the NOT SUPPLIED warning entirely

**Where** `crates/mt-cli/src/blocks.rs`, `legend` — matches on
`Option<&str>` without testing `.is_empty()`.

**What** `--from ""` is `Some("")`, not `None`, so it is treated as supplied:
the legend renders `FROM WALLET ` with a trailing blank, no `<-- NOT SUPPLIED`
marker, and the entire explanatory paragraph — which §10.4 makes normative for
"absent/blank" explicitly — is suppressed. Omitting the flag behaves
correctly; only the empty-string case is missed. The realistic trigger is
`--from "$WALLET_ID"` with the variable unset, which is silent at the shell and
now silent in `mt` too.

**How I reproduced it** Confirmed by reading `legend`'s match arms directly
(no `.is_empty()` guard anywhere in the four-way match); this specific
reproduction command is from the sub-reviewer that found it, not re-run by me:
```
$ mt encode --bitcoin-cli /nonexistent --from "" --to "" --in raw.hex
    FROM WALLET 
    TO   7.99900000 BTC
                                          <-- no warning paragraph at all
```

---

## Pre-existing issues surfaced incidentally (not fold defects, not counted above)

Found while constructing legitimate inputs for the checks above; not
introduced by this fold (confirmed byte-identical to baseline `1b2859c` where
noted), and out of scope for this check's question, but worth a reader's
attention:

- **A panic (exit 101), not a refusal, on a non-ASCII character straddling
  byte 11 of the first `mt1` line.** `read_strings.rs`'s `full[..ELIDED_DROP]`
  slices a `String` by byte index. I reproduced this myself: `printf
  'mt1qqqqqqq\xc3\xa9qqqqqqqqqqqqqqq\n' | mt decode --in - ...` panics with
  `byte index 11 is not a char boundary`. I independently confirmed via `git
  show 1b2859c:crates/mt-cli/src/read_strings.rs` that this exact byte-slicing
  line already existed at the pre-review baseline — genuinely pre-existing,
  not fold-introduced.
- `--json` parses on all four verbs and is read nowhere in the crate —
  byte-identical to baseline per the originating review, not re-verified by me
  beyond confirming the current behavior is a no-op.
- `decode`/`inspect` accept `--transaction` and silently ignore it —
  byte-identical to baseline per the originating review.
- `--group-size 0` is silently accepted as ungrouped — confirmed myself via
  `git show 1b2859c:crates/mt-cli/src/main.rs`, the `Some(n) if n > 0 => …, _
  => base` match arm is byte-identical to baseline.
- `--elide-prefix`'s correction-coverage report measures lengths before
  elision is applied, so the mandatory before-you-cut character count
  describes strings that are not the ones on stdout — per the originating
  review, explicitly marked pre-existing there and not independently
  reproduced by me.

## Observations, not findings

- **`check-refusal-coverage.sh`'s widened scan produces no false positives.**
  Ran it live myself: 25 tests over 16 ruled refusals, 0 problems. It
  structurally cannot detect a refusal-shaped function with **zero** tests —
  exactly M-7's shape — but its own header documents that limit, so this is
  the gate working as scoped, not a gap the fold introduced or hid.
- **The eight areas of new code not covered by a defect above were each tried
  against their hardest legitimate case and held.** A legitimately short final
  chunk that is *undamaged* is not flagged by the length check; a clean
  multi-chunk set is never falsely refused by `content_id_guard`; a tab
  separator round-trips; ordinary amounts at both ends of the valid range
  parse; a mixed-`nSequence` transaction (only one input non-final) still
  correctly reports as locked; a stronger provenance is never overridden by a
  weaker one for the same input; `Refusal::with_verbatim` bypasses `wrap()`
  cleanly without disturbing non-verbatim refusals.

## Tree state

`crates/mt-cli/` confirmed byte-identical to `268b354` throughout this review.
All mutations applied during verification (three, for the false-pass gate
checks) were reverted in an isolated worktree and confirmed not to touch the
main checkout. This report is the only file I wrote to
`design/agent-reports/`; a duplicate produced by a collision between two of
the dispatched sub-reviewers is preserved, not deleted, at
`design/agent-reports/R-fold-check-round2-parallel-b.md`, and the
independently-committed adversarial review this report's "New defects"
section draws on and re-verifies is at `design/agent-reports/
R-fold-adversarial-round2.md` (commit `bc2c57e`, not authored by any agent
dispatched in this task).
