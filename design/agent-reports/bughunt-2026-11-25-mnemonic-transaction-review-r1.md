# Review r1: bg002h/mnemonic-transaction PR #32 (T1 BIP-68 relative locks, T2 ms1 scan)

Reviewer: independent adversarial review (Opus 5.5), 2026-11-25.
Diff: `git diff origin/main...claude/project-thread-utm1aa` (base `9b094e0`, head `a611567`, 2 commits, 7 files, +679/-71). Nothing was edited, committed or posted. Probes ran against the built `target/debug/mt` with scratch files outside the repo.

**Verdict: 0 Critical, 0 Important, 4 Minor, 7 Nit.** T1 is correct against BIP-68 and Core. T2 closes the reported case (ms1 after an mt1 set or after a long first line) and does not falsely refuse legitimate input in practice. It is narrower than its name and doc comment claim (M1). No probe found any secret bytes reaching output.

## Gates run

- `cargo test --locked -j 2 -p mt-cli --no-fail-fast`: every binary passes except `history_purge` (6 failures, all "`/usr/bin/fish` / zsh is required", the expected environment gap). The new tests pass: unit 45 (5 new `bh1125_t1_*`), `tests/locktime.rs` 7 (2 new), `tests/refusals.rs` 67 (2 new).
- `cargo fmt --check`: clean. `cargo clippy --locked -p mt-cli --all-targets -- -D warnings`: clean.

## 1. T1 semantics (BIP-68 and Core, checked against source)

Sources fetched: `bitcoin/bips` `bip-0068.mediawiki` (master) and `bitcoin/bitcoin` `src/consensus/tx_verify.cpp` and `src/primitives/transaction.h` (master).

| Point | Source | PR (`crates/mt-cli/src/locktime.rs`) | OK? |
|---|---|---|---|
| Version gate | Core master: `fEnforceBIP68 = tx.version >= 2 && …` with `const uint32_t version` (transaction.h:299). Older releases: `static_cast<uint32_t>(tx.nVersion) >= 2` | `:135` `(tx.version.0 as u32) < 2` returns nothing. Version -1 (0xffffffff) is enforced, which matches Core. A unit test covers it | yes |
| Disable flag, bit 31 | BIP-68 l.30/32; Core :71 | `:146` skips the input | yes |
| Type flag, bit 22, 512 s | BIP-68 l.36; Core :79/94 (`<< 9`) | `time` flag, `value*512` | yes |
| Mask 0x0000ffff | BIP-68 l.40 "MUST be applied" | `(seq & 0xffff) as u16`, with bits 16-21 and 23-30 ignored. A unit test uses `0x0041_0064` | yes |
| Zero value | BIP-68 l.46/51: "lock-time of zero indicates an input which can be included in any block". Core computes `coinHeight+0-1`, which is always satisfied | value 0 is not reported (`then_some`) | yes |
| Coinbase | BIP-68 l.53; Core ConnectBlock skips coinbase | `:141` filters inputs whose prevout `is_null()` | yes (see N1) |
| nSequence 0xffffffff / 0xfffffffe / 0xfffffffd | bit 31 set, so no relative lock | none reported; a unit test covers it | yes |
| Interaction with nLockTime | a relative-locked input has nSequence != 0xffffffff, so nLockTime is enforced | `read_absolute` is unchanged. `NotEnforced` together with a non-empty `relative` is impossible, so the different conditions in `report_rows` (:314, `!= Lock::None`) and `legend` (:332, excludes `NotEnforced`) agree | yes |
| Overflow | 65535*512 = 33,553,920 < u32::MAX | `u32::from(u16)*512` | yes |

**Locked but printed as unlocked, or the reverse.** I found neither. Checked by probe (`mt inspect --in <x>.hex --bitcoin-cli /nonexistent [--json]`, built from the `even` vector):
- v2, seq 0x00008000, lt 0: `LOCKTIME  RELATIVE LOCK 32768 BLOCKS ON INPUT 0   counted from…`. No `NO TIMELOCK` anywhere. The no-node block says `has the locktime passed? UNKNOWN`.
- v2, seq 0x00400064: `RELATIVE LOCK 51200 SECONDS ON INPUT 0`.
- version 0xffffffff, seq 0x10: `RELATIVE LOCK 16 BLOCKS ON INPUT 0`, matching Core.
- v2, seq 0x90, lt 96: the `LOCKED TO BLOCK 96` row, then an indented `RELATIVE LOCK 144 BLOCKS ON INPUT 0` row. Both appear in the no-node block.

**"UNKNOWN" and the removed "spendable now".** Both are correct. `is_unlocked()` returns false whenever a relative lock exists, so the answer is UNKNOWN. With a relative lock present, `Locks::below_reference_warning` (:343) replaces "Treat it as spendable now" with "…also carries a RELATIVE LOCK…, which still binds". No other "spendable" text remains (grep).

**`--json` `relative` array: not a breaking change.** It is an additive key. I searched `/home/user/mnemonic-engrave`, `/home/user/mnemonic-gui`, `/home/user/mnemonic-toolkit` and `/home/user/seedhammer` and found no parser of `mt --json`. (Engrave's `gen-mt1-vectors.py` `locktime` is Core's `decoderawtransaction` field. Toolkit hits are its own `inspect`.) The spec defines no JSON schema for this object. Two shape changes are noted as N2.

## 2. T2 (`crates/mt-cli/src/validate.rs:1741-1790`)

**Binary-PSBT sniff order: preserved.** `secret_guard` only reads the bytes. `sniff` (input.rs:38) is unchanged and still runs after it. `is_binary_psbt` (:1757) uses the same outer trim as `trim_outer` (`is_ascii_whitespace`) and the same magic. The early return only makes the guard more permissive for binary input. Before the PR, a binary PSBT holding `\nms1` in its first 64 bytes was falsely refused, and the new test fixes that. A valid binary PSBT with an ms1 appended is refused by §8.2e ("continues for 55 bytes after the end of the PSBT") and nothing is echoed (probed: 0 secret substrings).

**False refusals of legitimate input: negligible.** Hex has no `m`/`s`, and mt1 strings start `mt1`. A wrapped base64 PSBT line would trip the scan only if it began `[mM][sS]1` (p ≈ 1.5e-5) and every remaining character were case-folded bech32 (55 of 64 base64 symbols, so about 1e-4 for a 64-column line, or higher for a short unpadded last line). That is about 1e-6 per PSBT or less, and the failure is fail-closed (N3).

**Can an ms1 still slip through? Yes (M1).** No secret was echoed in any case.

## Findings

### M1 (Minor): the T2 scan misses ms1 in common written forms, while the test name and doc comment claim "anywhere"
`validate.rs:1741-1750`. The token rule is "split on ASCII whitespace; the token starts `ms1`, has more than 3 bytes, and every byte after `ms1` is bech32". Probe: each variant below followed a valid 6-string mt1 set (`even`). In every case `verify`, `decode` and `inspect` **exit 0** (`verify` prints `OK`) and do not refuse by §8.9. `encode` refuses only by §8.2e.
- `ms1 2wxvvw3…` (a space after the HRP, a natural way to group a codex32 string for engraving or reading back)
- `"ms12wxv…"` (quoted, or JSON `{"seed":"ms1…"}`)
- `seed:ms12wxv…` (a label with no space; the PR's own `mid_line` test uses `seed: ` with a space, refusals.rs:953)
- `ms12w-xvvw-3vc7-…` (hyphen grouping)
- a UTF-8 BOM or NBSP (U+00A0) before `ms1`
- a single non-bech32 typo in the body (`…m9b6…`), which is still almost all secret entropy

This is the same outcome T2 reported ("verify printed OK and exited 0"), one trivial step away from the tested shapes. Mitigation, verified: in every probe nothing past the codex32 header was printed. The UNREADABLE STRING notice names one offending character and an offset. The §8.2e 8-byte echo covers at most `"ms12wxv` (quote, HRP, threshold and 3 id characters), and the payload starts at offset 9 after `ms1`. So this is a gap in the refusal policy, not a leak, and Minor fits the original Low.
- **Fix direction:** match `ms1` case-insensitively at any non-alphanumeric boundary (start of input, or a byte that is not `[0-9A-Za-z]`). Then require at least a small run of bech32 characters, allowing interleaved ASCII space and `-` separators (for example 16 or more data characters with whitespace and hyphens removed). Drop the "all-bech32 to the end of the token" condition, or apply it only to the characters before the first separator. The base64 false-positive rate stays small because of the boundary and run-length conditions. At minimum, rename the test and comment from "anywhere" to what is actually scanned.

### M2 (Minor): the engraved relative-lock line does not say what the lock is relative to
`locktime.rs:120` (`Relative::legend`), spec §8.4 amendment. The plate reads `RELATIVE LOCK 32768 BLOCKS ON INPUT 0`. The anchor ("counted from the block that confirms the coin it spends") appears only in the disposable stderr row. Failure scenario: a stranger who has the plate and not the report reads "32768 blocks" as "after this plate was cut" or "after block 0" and cannot work out when the spend becomes valid. That is the question T1 exists to answer. The spec chose this deliberately, so this is not a defect against spec. Consider `RELATIVE LOCK 32768 BLOCKS AFTER INPUT 0 CONFIRMS` (also see M3 on length).

### M3 (Minor): the legend line exceeds the field budget the spec still states
Spec §5 legend table: the lock field is budgeted at **35** characters. `RELATIVE LOCK 32768 BLOCKS ON INPUT 0` is 37. The worst seconds case, `RELATIVE LOCK 33553920 SECONDS ON INPUT 0`, is 41, plus more digits for a larger input index, and there is one line per locked input. The amendment admits the budget was "not re-measured". The table row's "35" was left as it was, while the 152-character/6-line total above it no longer holds for these transactions. Either re-measure or change the stated budget to "35 per nLockTime line; relative lines extra".

### M4 (Minor, pre-existing, context for the spec-copy check): the two SPEC copies already differ outside this hunk
The **hunks are identical**: the `+`/`-` lines of `mnemonic-transaction` `origin/main...HEAD` and `mnemonic-engrave` `origin/master...origin/claude/project-thread-utm1aa` (`67c3f1c`) for `design/SPEC_mt_v0_1.md` diff clean, 200 lines each. The **files** are not identical, before or after: `origin/main` here and engrave `origin/master` already differ by 263 diff lines. The mt copy carries text the owner lacks, for example the DEAD/UNKNOWN `-txindex` liveness rows, "A failed call is not a null (bughunt-1119 C1)", and FIX-E rulings T16/T17. CLAUDE.md says the design files are copies and the engrave spec is normative, so the copy is ahead of its owner. This PR does not cause the drift and keeps it the same size. It should be raised so the re-sync does not regress either side.

### N1 (Nit): coinbase is filtered per input, not per transaction
`locktime.rs:141`. Any input with a null prevout is skipped, including in a multi-input transaction (which `CheckTransaction` rejects anyway). `tx.is_coinbase()` would state the BIP-68 exemption exactly. No reachable effect.

### N2 (Nit): JSON shape quirks
`report.rs:586-601`. (a) `legend` is now `lines.join("\n")`, so a consumer that put `legend` on one line now gets an embedded `\n` (correctly escaped) when both kinds of lock are present. (b) Each `relative` element uses a key that changes with the unit (`"blocks"` or `"seconds"`). `{ "input": 0, "unit": "blocks", "value": 32768 }` or always-present `blocks`/`seconds` keys with null would be easier to parse. No consumer exists today.

### N3 (Nit): rare false §8.9 refusal of a wrapped base64 PSBT
`validate.rs:1743-1749`. See §2: about 1e-6 per PSBT, mostly on a short unpadded last line that starts `ms1`/`MS1`. It fails closed, but with a misleading message ("the input is ms1"). Requiring a minimum token length (an ms1 is 48 characters or more) would remove the short-last-line case.

### N4 (Nit): stale doc comment
`crates/mt-cli/tests/inspect.rs:88-91` still says "a BIP-68 relative timelock lives in `OP_CSV` inside the witness script, and reading it means evaluating the sending wallet's script". That is the claim this PR corrects.

### N5 (Nit): the time-lock anchor wording is approximate
The report row and spec say "counted from the block that confirms the coin it spends". For bit-22 locks, BIP-68 l.36/47 counts from the **median-time-past of the block before** the one that mined the coin, and ends at the MTP of the previous block. This is fine for a human-scale reading. "SECONDS" with no conversion (51200 rather than about 14.2 h) is deliberate per the spec.

### N6 (Nit): one added assertion does not discriminate
`tests/locktime.rs:139` adds a relative-lock row to `the_row_uses_only_the_ruled_spellings`. On the pre-fix code that row reads `NO TIMELOCK`, which also passes. The other T1 tests do discriminate (see §3).

### N7 (Nit): the binary-PSBT test does not check for non-echo
`tests/refusals.rs:989`. This test puts a full ms1 inside a would-be binary PSBT and only asserts that the refusal is not §8.9. It should also assert that `MS1[3..11]` is absent from stderr, as its sibling does, so a later §8.2e message that quotes bytes cannot leak through the route this PR opens on purpose. It does not leak today (probed).

## 3. Do the tests fail without the fix?

- Unit `bh1125_t1_*` (locktime.rs:530-620) use the new API (`read` → `Locks`, `Relative`), so they cannot compile on base. Their assertions (value, unit, index, version gate, disable flag, zero, negative version, both locks, warning wording) would each fail against a behavior regression.
- `bh1125_t1_a_bip68_relative_lock_is_never_engraved_no_timelock`: base prints `LOCKTIME  NO TIMELOCK` and legend `NO TIMELOCK`, so the test fails. The disable-flag control is good.
- `bh1125_t1_absolute_and_relative_rows_both_print`: base prints no RELATIVE row, so the test fails.
- `bh1125_t2_ms1_anywhere_in_the_input_is_refused`: on base, after-set (ms1 past byte 64, set lines 87 chars), after-comment (`\nms1` at byte 70) and mid-line are all unrefused, and `verify` exits 0, so the test fails. The `verify` control on the bare set is good.
- `bh1125_t2_a_binary_psbt_is_not_scanned_for_ms1`: on base, `\nms1` at byte 9 is within the 64-byte head, so the input is refused by §8.9 and the test fails. It is discriminating; see N7 for what it misses.

## 4. Spec copy

The hunk is identical to engrave's PR branch (see M4). The content was checked against BIP-68 source text. The quoted phrases ("transactions with an nVersion greater than or equal to 2", bit 31 and 22 wording, "a mask of 0x0000ffff MUST be applied", "the mining date of the output it is spending", the zero-lock sentence with a fair ellipsis, and "not applied to the nSequence field of the input of the coinbase transaction") match l.26-53. The Core statement about unsigned version matches master and the older `static_cast<uint32_t>`.

## Summary for the fold

- No Critical or Important finding, so nothing blocks.
- Recommended before merge (cheap): M1 (widen the T2 matcher or at least rename "anywhere"), N4 (stale comment) and N7 (non-echo assert).
- Log to FOLLOWUPS: M2, M3, M4 (re-sync the spec copies in both directions), N1, N2, N3, N5, N6.
