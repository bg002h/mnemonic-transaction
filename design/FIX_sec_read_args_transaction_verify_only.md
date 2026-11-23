# FIX: `--transaction` is verify-only (security sweep unit `read-args-transaction-verify-only`)

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-23. Key:
`mt-decode-inspect-ignore-transaction` (codecs slice row 54, Important; row 63
`mt-decode-inspect-transaction-ignored` is its duplicate). Source:
`coordinator/reports/security-sweep/grade/routing.md` row
`read-args-transaction-verify-only`; grade in
`coordinator/agent-reports/grade-security-sweep-codecs.md` row 54.
Track: small-fix. No new flag, type, command, public API or mt-authored text.
Base: origin/main `88ac6d9`. Branch `fix/sec-read-args-transaction-verify-only`.

## 1. Finding (at 88ac6d9)

`--transaction <PATH>` is a field of the shared `ReadArgs`
(`crates/mt-cli/src/main.rs:101-113`, struct at `:61`), and `decode`, `verify`
and `inspect` all take `ReadArgs` (`Command`, `:52-56`). Its only reader is
inside `verify` (`:1545`). So `mt decode` and `mt inspect` accept it, compare
nothing, never mention it, and exit 0; `decode` prints broadcastable hex on
stdout. `mt decode --help` and `mt inspect --help` list it, with the text
"`verify` only.". An operator who runs `mt decode --in plates.txt
--transaction mine.hex` to check before broadcasting gets the hex and exit 0
for a set that holds a different transaction; `mt verify` with the same two
files refuses (`REFUSED — §1.1, the supplied transaction is not the one on
these strings`, exit 1). Probed in section 5 (RED).

## 2. Rule (decided)

SPEC §1.1's flag table (`design/SPEC_mt_v0_1.md:3520`) already rules
`--transaction <psbt|hex>` "on `verify`". The fix makes the CLI match it:

- **R1.** `--transaction` is declared on `verify` alone. `decode` and
  `inspect` no longer have the argument; clap rejects it as an unknown
  argument, exit 2, nothing on stdout, before any input is read. This is the
  same mechanism and the same exit code that `mt decode --out` meets today
  (`--out` is "On `encode` alone", `main.rs` `EncodeArgs`):
  `error: unexpected argument '--out' found`, exit 2.
- **R2.** `verify` is unchanged: same flag spelling, same value name `<PATH>`,
  same help text (the doc comment moves verbatim, including "`verify` only."),
  same comparison, same refusals, same match line.
- **R3.** No other `ReadArgs` field moves. `--quiet` keeps its §10.10
  refusal on `inspect`/`verify`; `--json`, `--in`, `-`, `--allow-argv-secret`
  and `--bitcoin-cli` stay shared.
- **R4.** Order is unchanged: `validate::argv_intake` and
  `validate::command_line_guard` still run on raw argv before clap, so
  `mt decode --transaction <inline hex>` still meets the §8.2f refusal first
  (exit 1), not clap's error. Only a path value (or none) reaches clap.

Implementation shape (builder's choice of names): a verify-only args struct
that flattens `ReadArgs` and adds the `transaction` field; `Command::Verify`
takes it; `verify()` reads `transaction` from it. No production behaviour
other than R1 changes.

## 3. What the holder sees after the fix

`mt decode --bitcoin-cli /nonexistent/bitcoin-cli --in <strings> --transaction <file>`:
exit 2, stdout empty, stderr exactly:

    error: unexpected argument '--transaction' found

      tip: to pass '--transaction' as a value, use '-- --transaction'

    Usage: mt decode --bitcoin-cli <PATH> --in <PATH>

    For more information, try '--help'.

`mt inspect …` the same with `mt inspect` in the Usage line. (The Usage line
is clap's and echoes the flags given, never their values.) `mt decode --help`
and `mt inspect --help` no longer list `--transaction`; `mt verify --help`
still lists `      --transaction <PATH>`.

All of this text is clap 4's (pinned by `Cargo.lock`), not mt's. **No new
mt-authored text, so no WORDING_REVIEW row.** It was observed by building a
throwaway implementation of R1 on 88ac6d9 (not committed) and running the
command above.

## 4. Spec, manual, CHANGELOG

- **Spec: no sentence changes.** §1.1 (`SPEC_mt_v0_1.md:510`, "Optionally,
  `--transaction <psbt|hex>`" in the `verify` item) and the flag table (`:3520`,
  "on `verify`") already say this. Nothing in the spec says decode or inspect
  takes it (`grep -n -- '--transaction' design/SPEC_mt_v0_1.md`: lines 510,
  528, 3339, 3347, 3520, 3525, all verify or flag-list context). No re-sync
  from mnemonic-engrave is needed.
- **Toolkit manual: no lockstep.** `git grep -n -- '--transaction' origin/master
  -- docs` in mnemonic-toolkit returns nothing; mt has no page in it.
- **CHANGELOG:** none in this FIX (entries are written at release, commit
  78f03c9; same as the bh1119 FIXes).

## 5. RED rows (run on unmodified 88ac6d9)

Test file: `crates/mt-cli/tests/sec_read_args_transaction.rs`, installed
verbatim from the sidecar `design/FIX_sec_read_args_transaction_verify_only/sec_read_args_transaction.rs.txt`
(the gate `cmp`s them). Command:
`cargo nextest run --locked --build-jobs 4 -j 4 -p mt-cli --test sec_read_args_transaction --no-fail-fast`.
Output at 88ac6d9: `4 tests run: 1 passed, 3 failed`.

| row | at 88ac6d9 (pasted) | after the fix |
| --- | --- | --- |
| `sec_rat_decode_with_transaction_is_a_usage_error` | FAIL `sec_read_args_transaction.rs:80:5` `` assertion `left == right` failed: mt decode: mt1 SET   6 strings, 1..6 all present, all begin mt1p9h8jqq9 `` … `left: Some(0)` `right: Some(2)` | PASS: exit 2, stdout empty, stderr equals section 3's text |
| `sec_rat_inspect_with_transaction_is_a_usage_error` | FAIL `:80:5` `` assertion `left == right` failed: mt inspect: `` `WARNING: no bitcoind reachable. …` `left: Some(0)` `right: Some(2)` | PASS, same with `mt inspect` |
| `sec_rat_decode_and_inspect_help_omit_transaction` | FAIL `:105:9` `mt decode --help still lists --transaction:` | PASS |
| `sec_rat_verify_keeps_transaction` (control) | PASS | PASS: help lists `      --transaction <PATH>`; matching transaction exits 0 with `  --transaction matches, on the full txid.` |

The hand probe the grader ran, re-run here at 88ac6d9 with the corpus vectors
`even` (strings) and `uneven` (raw hex, a different transaction):
`mt decode … --transaction uneven.hex` rc 0, 445 bytes on stdout;
`mt inspect …` rc 0, 616 bytes on stdout; `mt verify …` rc 1, `REFUSED — §1.1,
the supplied transaction is not the one on these strings`. Neither decode nor
inspect stderr names the flag.

GREEN proof: the throwaway R1 implementation with the sidecar installed gives
`463 tests run: 463 passed` over the workspace (459 at base + 4).

## 6. Tests that change

None. Every existing `--transaction` use is on `verify`
(`decode_verify.rs:320,335`, `inspect.rs:805`, `refusals.rs:1248,1280`,
`bh1119_mt_input_parsing.rs:271,288,315,422`); `scripts/` and `ci/` never pass
it. `refusals.toml` gains no entry (a clap usage error is not a §8 refusal;
`check-refusal-coverage.sh` stays at 61 tests over 18 refusals with the new file
present). One new test file (section 5).

## 7. Gate

`bash design/FIX_sec_read_args_transaction_verify_only/gate.sh > <log> 2>&1` on
the builder's head: every CI `test` job step once (fmt, clippy `-D warnings`,
build, nextest 463/463, doc tests, refusal coverage 61/18, mutate-refusals 61,
journeys, docs-only and code-docs-check tests), the sidecar `cmp`, the 4 rows,
and two greps (`Verify(ReadArgs)` gone; exactly one `transaction:
Option<std::path::PathBuf>` field). Its header states what it does not cover.
It unsets `CARGO_TARGET_DIR`, because `scripts/journeys.sh` runs the hard-coded
`target/debug/mt`. Run by the author on the uncommitted throwaway
implementation with the sidecar installed: `GATE PASS` (463/463, 4/4, coverage
61/18, mutation 61, journeys, code-docs check ok).

## 8. Decided here (flag if wrong, no STOP)

- **D1. Clap usage error (exit 2) rather than an mt refusal naming
  `mt verify` (exit 1).** The routing row allows either. Chosen because the
  spec already rules the flag "on `verify`" (so no spec sentence and no new
  wording are needed), it matches the `--out`-on-`decode` precedent, and it
  removes the flag from `decode --help`/`inspect --help`, which is where the
  operator was told about it. The alternative (keep the field shared, add a
  guard like `quiet_unsupported_guard` with a new text naming `mt verify`)
  needs a new mt text, a WORDING_REVIEW row, a refusals.toml entry and a spec
  sentence; it would be the only refusal for a flag the spec never gave the
  verb. Neither path has funds risk after the fix (both stop before stdout).
- **D2.** The moved help text keeps "`verify` only." verbatim (now redundant
  but true); editing help text is out of scope.

## 9. Files this FIX touches

- `crates/mt-cli/src/main.rs` (`Command::Verify`, `ReadArgs`, a verify-only
  args struct, `verify()` signature and its `:1545` read)
- `crates/mt-cli/tests/sec_read_args_transaction.rs` (new, from the sidecar)

## 10. FOLLOWUPS

None closed (no line exists for this key at 88ac6d9). Proposed lines are in the
author's report, not appended here.
