# Final pre-merge review (Fable): bg002h/mnemonic-transaction PR #32 (T1 BIP-68 relative locks, T2 ms1 scan)

Reviewer: Fable 5.2, 2026-11-25. Diff: `git diff origin/main...claude/project-thread-utm1aa` (base `9b094e0`, head `63a83e7`, 4 commits). Nothing tracked was edited, committed, pushed or posted. Probes ran against `target/debug/mt` built at head, with scratch files outside the repo.

**Verdict: 0 Critical, 0 Important, 5 Minor, 6 Nit (all new; none blocks).** T1 is correct against BIP-68, BIP-112 and Core's `CalculateSequenceLocks`. T2 closes the reported case and I found no path that prints secret material. The r1/r2 gradings stand; I would not raise any logged item.

## Gates run

- `CARGO_INCREMENTAL=0 cargo test --locked -j 2 -p mt-cli --no-fail-fast` at head: every binary passes except `history_purge` (6 failures, the expected zsh/fish gap). Unit 45, `tests/locktime.rs` 7, `tests/refusals.rs` 67.
- I did not re-run fmt or clippy (r1 ran both clean at `a611567`; the two later commits touch only Markdown).

## What I checked and found sound

- **T1 semantics** (`crates/mt-cli/src/locktime.rs:132-155`): version gate read unsigned, disable bit 31, type bit 22, 16-bit mask, zero value is no lock, 512-second units with no overflow.
- **No false "unlocked" state.** A relative-locked input has `nSequence != 0xffffffff`, so `NotEnforced` cannot coexist with a relative lock, and the two differing conditions in `report_rows` (:314) and `legend` (:332) agree on every reachable state.
- **Probes** (v2, no node, `encode` then `inspect`/`decode`, text and `--json`):
  - Relative only: `RELATIVE LOCK 32768 BLOCKS ON INPUT 0` on the report row, the legend and the no-node block (`UNKNOWN`); no `NO TIMELOCK` anywhere.
  - Mixed inputs (`8000`, `ffffffff`, `400064`): inputs 0 and 2 reported, input 1 skipped.
  - `nLockTime` 900000 plus a relative lock: the warning says "still binds", not "spendable now".
  - Timestamp lock plus a relative time lock: both lines print.
  - `--json` parses, and `legend` carries an escaped `\n`.
- **T2, binary-PSBT exemption** (`validate.rs:1775`): a `psbt\xff…\n<ms1>` file is refused by `encode` and `inspect` under §8.2e and by `decode` and `verify` under §1.1e ("not text"). No window of the ms1 body appeared in any output.
- **Do the tests fail without the fix?** I agree with r1 §3: the four integration tests and five unit tests discriminate. The gaps are F3 and F4 below.

## Findings

### F1 (Minor): with a node, mt still does not answer "has the relative lock passed?", and two printed texts say it will
`locktime.rs:312-325` (`report_rows` ignores `chain` for relative rows); `report.rs:709-733`; `blocks.rs` `encode_no_node_warning`.

- **Scenario.** A recoverer inspects a relative-locked plate with no node. mt prints `has the locktime passed? UNKNOWN`, then "TO RESOLVE ALL FOUR AT ONCE, either: run mt inspect again with a bitcoind reachable, or look this txid up in any block explorer". They re-run with a node and get the identical `RELATIVE LOCK 32768 BLOCKS ON INPUT 0   counted from…` row. An explorer has never seen an unbroadcast txid.
- **Same promise at encode time.** The warning lists "how far away is the locktime, in real blocks?" and says a node "answers all three".
- **Regression in one detail.** A relative-only transaction with a node used to print `NO TIMELOCK   current height N`. It now prints no current height at all, so the operator cannot even do the arithmetic by hand.
- **Why Minor.** Nothing false reaches steel, and a premature broadcast is refused `non-BIP68-final`. But this is the question T1 exists to answer, and the two texts are now untrue for this class.
- **Fix direction.** mt already calls `gettxout` per input, whose reply carries `confirmations`. For block locks, print `coin has N confirmations` beside the row and keep the current height. Until then, qualify the two texts for relative locks.

### F2 (Minor): one legend line per input repeats the same lock N times
`locktime.rs:329-337`, `blocks.rs:241`.

- **Scenario.** A recovery sweep of five UTXOs from one `older(32768)` wallet. Probed: the suggested legend is five lines, `RELATIVE LOCK 32768 BLOCKS ON INPUT 0` through `… INPUT 4`, about 185 characters to cut by hand for one fact. The report row repeats the same way.
- **Relation to logged items.** `mt-t1-legend-width` (r1 M3) covers the 35-character field. This is the line count, which grows with inputs and is the larger cost for the common multi-UTXO case.
- **Fix direction.** Group equal locks: `RELATIVE LOCK 32768 BLOCKS ON ALL INPUTS`, or `ON INPUTS 0-4`. Decide it together with `mt-t1-legend-counts-from`, since both change the engraved spelling. It is better settled before plates are cut with the per-input form.

### F3 (Minor): the `--json` `relative` array and the multi-line `legend` have no test
`report.rs:584-604`. Nothing in `crates/mt-cli/tests/` reads `locktime.relative` or a two-line `legend`. The object is hand-built with `format!`. A dropped comma or a swapped `blocks`/`seconds` key ships green. This is new machine-readable surface (r1 N2 logged its shape, not its lack of a test).
- **Fix direction.** One `inspect --json` test on a mixed blocks/seconds transaction that parses the output and asserts both elements.

### F4 (Minor): two conditions the code documents as load-bearing are unpinned
- `validate.rs:1745-1748`: the bech32-charset test is what "keeps a line of wrapped base64 from tripping it". No test feeds a token that starts `ms1` and holds a non-bech32 character, or a wrapped base64 PSBT. Removing the `.all(..)` clause, which would refuse about 1 in 65,000 base64 lines, fails nothing.
- `locktime.rs:141`: the coinbase skip. No test has a null prevout, so deleting the filter fails nothing.
- **Fix direction.** A unit test for `ms1_token_anywhere` with negative controls (`ms1`, `ms1b…`, an `mt1…` string, a base64 line starting `Ms1+`), and one row in `bh1125_t1_what_is_not_a_relative_lock`.

### F5 (Minor): no CHANGELOG entry
`CHANGELOG.md` `## mt-cli [Unreleased]` records operator-visible fixes (FIX-J is there). This PR adds a printed spelling, an engraved legend line, a `--json` key and a wider refusal, and adds no entry. The engraved-text change is the kind a release note must carry: two mt versions now suggest different plates for the same transaction.

### F6 (Nit): `Lock`'s single-lock methods stay `pub`
`locktime.rs:219`, `:267`, `:284`. `Lock::legend()`, `Lock::report_row()` and `Lock::below_reference_warning()` are now only correct when called through `Locks`. No caller outside the module uses them. A later `read_absolute(tx).legend()` (`read_absolute` is legitimately used at `report.rs:404`) would compile and reintroduce T1. Make the three private to the module.

### F7 (Nit): the report row is 106 columns
`locktime.rs:318-321`. `LOCKTIME  RELATIVE LOCK 32768 BLOCKS ON INPUT 0   counted from the block that confirms the coin it spends`. Every other report row wraps near 72 (see the FEE row). It wraps mid-phrase on an 80-column terminal, and again in the indented no-node block.

### F8 (Nit): duplicated unit arithmetic
`report.rs:594-598` recomputes `value * 512`, which `Relative::amount` (`locktime.rs:111`) already owns. Expose one accessor so the plate and the JSON cannot disagree.

### F9 (Nit): `ms1_token_anywhere` doc comment and the leftover head check
`validate.rs:1732-1733` says "the 64-byte head above"; the head check is below, in `secret_guard`. The head check is now a second, looser rule that applies only to the first 64 bytes (it refuses `ms1 <body>` at the start but not later, as r2 found). Say in the comment why both exist, or fold them into one when `mt-t2-ms1-scan-forms` is done.

### F10 (Nit): `From<Lock> for Locks` exists only for tests
`locktime.rs:169-176`. It is used only by `report.rs` unit tests, and it builds a `Locks` that silently claims "no relative lock". Gate it `#[cfg(test)]`.

### F11 (Nit): spec leaves "five spellings" claims and says nothing of the T2 rule
`design/SPEC_mt_v0_1.md`. The "THESE FIVE ARE THE stderr REPORT'S SPELLINGS" note after the amended list still reads as exhaustive. §8.9 (line 3250) is one line and does not record the whole-input token scan or the binary-PSBT exemption, so the T2 behaviour is specified only in code comments. Fold into the owed re-sync (`mt-spec-copy-drift-engrave`).

## Logged items I challenged

- **`mt-t2-ms1-scan-forms` (r1 M1): Minor stands.** It is a refusal-policy gap with no print, engrave or store path. My binary-PSBT probes add four verbs with zero echo.
- **`mt-t1-legend-counts-from` (r1 M2): Minor stands,** but it is the logged item I would fix first, together with F2, because both change what is cut into steel.
- **`mt-verify-transaction-no-secret-guard` (r2 M1): Minor stands** (pre-existing, 8-byte echo cannot reach payload).

## Cross-repo note (not a finding against this PR)

No other repository prints these spellings: I found no `NO TIMELOCK` / `RELATIVE LOCK` text in mnemonic-toolkit's manual, mnemonic-engrave's crates or mnemonic-gui. The seedhammer fork's matches for "timelock" are in the md composer (`gui/composer_lock.go`, `md/`), not a transaction view; I did not check whether `gui/transaction.go` shows `nLockTime` under another wording.

## Summary for the fold

- Nothing blocks. All findings are Minor or Nit and can be logged under the project's rule.
- Cheapest to fix before merge if the owner wants: F3 and F4 (tests only), F5 (one CHANGELOG paragraph), F6 (three visibility keywords).
- Needs an owner decision, best taken before plates are cut with the new line: F2 with `mt-t1-legend-counts-from`, and F1.
