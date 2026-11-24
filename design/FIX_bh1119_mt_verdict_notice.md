# FIX: verdict and notice integrity on stderr (bughunt-1119 FIX table row 38, "FIX-G verdict and notice integrity")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-23. Keys: bh-mt-I10 (I),
bh-mt-I23 (I). Ruling T21 (`coordinator/agent-reports/architect-bughunt-1119-questions.md:59`),
binding. Track: small (one Opus review round). Base: origin/main `88ac6d9`.
Branch `fix/mt-verdict-notice`. **No new holder-visible text.** Every line this
FIX prints is printed today by the loud `decode` or by a passing `verify`; the
FIX changes when each is printed (section 3).

Not to be confused with the older `design/FIX_bughunt_mt_*.md` (a different hunt).

## 1. Findings (run at 88ac6d9, `target/debug/mt`)

Fixtures: the `even` vector of `crates/mt-codec/src/test_vectors/mt1_v1.json`
(6 strings, set `0x2dcf2`) and the `uneven` vector's `raw_hex`. All signet test
material. Section 6 holds the rows and the measured base output.

- **I10 (decode --quiet drops warnings and notices).** `main.rs:1395` opens
  `if !args.quiet {` and everything decode writes to stderr before the F-275
  stdout-mode warning sits inside it: the §1.1a report, the no-node warning
  (`main.rs:1473-1486`), and the three notice blocks
  `transliteration_notices` / `set_notices` / `margin_report`
  (`main.rs:1487-1489`). The node probe and the transaction parse that the
  no-node warning needs (`main.rs:1409-1411`) are inside it too. So
  `mt decode --quiet` on a set that needed a BCH repair, held a misread
  character, a duplicate or an unreadable spare, or ran with no node, exits 0
  with byte-identical stdout and an **empty stderr**. The flag's own help
  (`main.rs:274-275`: "Suppress the inspection report. **Warnings and refusals
  are never suppressed**, on any verb.") and SPEC_mt_v0_1.md:3484-3485 say the
  opposite. The `--json` arm already treats every one of these as a warning (it
  puts each into the `warnings` array, `main.rs:1424-1447`).
- **I23 (verify prints OK before its comparison can fail).** `verify` writes
  `mt verify: OK — {n} chunks, set {id}, transaction re-derives.`
  (`main.rs:1532-1536`) and the three notice blocks (`:1537-1539`) **before** it
  reads, parses or compares `--transaction` (`:1545-1614`). A mismatched, missing
  or unparseable `--transaction` therefore prints the OK line first and the
  `mt verify: REFUSED — ...` second, exit 1. The exit code is right; the first
  verdict on stderr is wrong.

## 2. Ruling (binding, quoted)

- **T21 mt-Q13, Q14 (I10, I23): warnings are never suppressed** (the flag's own
  help, grade-mt:67), **and the OK line is written only after the comparison
  passes.**

Reading applied (author; flagged for the architect in section 9, not treated as
open): the first clause answers mt-Q13 (the no-node warning is owed under
`--quiet`: yes) and the notices half of mt-Q14 (the notices still print when the
comparison refuses: yes). The second clause answers the OK-line half.

## 3. Behaviour after the fix (rules)

**Q1 decode --quiet keeps every warning and notice.** With `--quiet` (and no
`--json`, which stays refused with `--quiet`, unchanged), `decode` writes to
stderr exactly the bytes the loud `decode` writes after its report, in the same
order, and nothing else:
  (a) when no node is reachable, the no-node warning, preceded by the one blank
      line the loud path writes before it (`writeln!(stderr)` at `main.rs:1474`),
      text from `report::no_node_warning(...)` with the same arguments
      (`ReadFrom::Strings`);
  (b) `transliteration_notices`, then `set_notices`, then `margin_report`,
      unchanged functions, unchanged arguments;
  (c) then the F-275 stdout-mode warning, when it applies, as today
      (`main.rs:1497-1499`, already outside the branch).
Only the report (`r.render()`, `main.rs:1469`) is omitted. Equivalently: quiet
stderr equals loud stderr from the blank line before `WARNING: no bitcoind
reachable.` (or, with a node, from the end of the report) to the end. Row 6
asserts this for five fixtures.

**Q2 decode --quiet probes the node.** To know whether (a) applies, the quiet
path now runs the same `decode_tx(&set.bytes, "decode", Subject::Set)`,
`compute_txid()` and `node::Node::find(&args.bitcoin_cli)` the loud path runs
(`main.rs:1409-1411`), before any stderr write. `decode_tx` adds no refusal in
practice: `content_id_guard` (`main.rs:1385`) has already parsed the same bytes
with `txid_partial` and `set_end_guard`. The node is only observed (CLAUDE.md
"Node access is read-only"). `Report::build` is not called under `--quiet`.

**Q3 decode stdout and exit code are unchanged** on every path (rows 7, 8 and
the existing F-275 rows).

**V1 verify compares before it speaks.** When `--transaction` is given, `verify`
reads, sniffs, parses and compares it (all of `main.rs:1545-1614`) **before it
writes anything to stderr**.
  - Comparison passes: stderr is, as today, the OK line, then
    `transliteration_notices`, `set_notices`, `margin_report`, then
    `  --transaction matches, on the full txid.` Byte-identical to the base
    output (rows 13, 14).
  - Any refusal inside that block (cannot read, not a transaction or PSBT, PSBT
    does not parse, trailing bytes, PSBT not finalized, txid mismatch): stderr
    holds the three notice blocks (each prints only when it has content, as
    today), then the refusal as `main` prints it today. **No OK line.** Exit code
    unchanged (rows 9-12).
**V2 verify without `--transaction`** is unchanged: OK line, then the notices.
**V3** the refusal texts, refusal codes and exit codes are unchanged everywhere.

## 4. Holder-visible texts

None new; none reworded. What changes is only presence and order:
- `mt decode --quiet` stderr now carries, when they apply, `WARNING: no bitcoind
  reachable. ...`, `CHARACTERS mt READ DIFFERENTLY. ...`, `DUPLICATE RESOLVED.
  ...`, `UNREADABLE STRING. ...`, `CORRECTION APPLIED. ...` (today: none of them).
- `mt verify --transaction X` that refuses no longer prints `mt verify: OK — ...`;
  its notices precede the refusal.

`--help` for `--quiet` (`main.rs:274-275`) already states the rule; unchanged.

## 5. Spec, README, help, manual, CHANGELOG

- `design/SPEC_mt_v0_1.md` is a copy owned by mnemonic-engrave (CLAUDE.md
  "Design files are copies"). Spec delta, applied to mnemonic-engrave's copy
  first and re-synced byte-for-byte by this FIX's build; anchors as in mt's copy
  at 88ac6d9. Both sentences are PROPOSED (architect confirms):
  1. §10.10b item b (:3484-3489), after "It does **not** relax §1.1a's rule that
     stdout stays empty unless every check passes." insert:
     `Under `--quiet`, `decode` prints on stderr everything its loud form prints
     after the report, in the same order: the no-node warning when no node is
     reachable, the misread-character, duplicate, unreadable-string and
     correction notices, and the stdout-mode warning.`
  2. §1.1 `--transaction` paragraph (:510-513), after "`md verify` can only
     re-encode and diff." insert:
     `The `OK` line is printed only after that comparison passes. A failed
     comparison prints the notices and then the refusal, never the `OK` line.`
  3. §1.1a F-275 paragraph (:2675-2676): "and an anonymous pipe is 0600, so the
     documented pipeline stays silent." becomes "and an anonymous pipe is 0600,
     so the documented pipeline gets no mode warning." (the no-node warning now
     reaches a `--quiet` pipeline run with no node).
- README.md: no mention of `--quiet` or of verify's line order; unchanged.
- Toolkit manual: `mnemonic-toolkit` `docs/manual/src/40-cli-reference/` has no
  `mt` page (41-46 cover mnemonic, md, ms, mk-cli, mr); no lockstep text.
- CHANGELOG.md: unchanged (the merged bughunt-1119 GREEN commits 3923ad6 and
  9bd361c add no entry; a release entry is written at release).

## 6. RED rows

Sidecar: `design/FIX_bh1119_mt_verdict_notice/verdict_notice.rs.txt`
(sha256 `cada033e36f7aeeabf54817500bc41f6057eea03259554ba6ba04638de3a580a`, rustfmt
edition 2024 clean, clippy `-D warnings` clean when installed). Install as `crates/mt-cli/tests/verdict_notice.rs`, then:

    cargo nextest run --locked --build-jobs 2 -j 2 -p mt-cli --test verdict_notice --no-fail-fast

At 88ac6d9 (this branch's base; the FIX commit is docs-only):
`14 tests run: 4 passed, 10 failed` (the 4 are CONTROL rows 7, 8, 13, 14). Each
failure is at the row's first assertion after the exit-status check, for the
defect (log: `/scratch/cache/wt-mt-verdict-notice-red2.log`).

GREEN feasibility (author, scratch, reverted; not part of this FIX): a 117-line
diff to `decode`, `verify` and the section 7 test
(`/scratch/cache/wt-mt-verdict-notice-scratch-green.diff`) turned all 14 rows
GREEN and kept the whole `mt-cli` suite green: `421 tests run: 421 passed`
(407 base + 14 rows).

| # | row (`bh1119_mt_g_` omitted) | input | base (RED) | after fix |
|---|---|---|---|---|
| 1 | quiet_keeps_the_correction_notice | even, s1 offset 40 q<->p; decode --quiet, offline | stderr `[]` | contains `\nCORRECTION APPLIED. 1 chunk needed repair:\n` |
| 2 | quiet_keeps_the_no_node_warning | even; decode --quiet, offline | stderr `[]` | starts `\nWARNING: no bitcoind reachable. mt read this transaction from the\n` |
| 3 | quiet_keeps_the_transliteration_notice | even, first two `6` of s1 typed `b`; decode --quiet | stderr `[]` | contains `\nCHARACTERS mt READ DIFFERENTLY.` and `you typed b, mt read it as 6` |
| 4 | quiet_keeps_the_duplicate_notice | even + chunk 3 copy with 1 error; decode --quiet | stderr `[]` | contains `\nDUPLICATE RESOLVED. chunk 3 was present twice.\n` |
| 5 | quiet_keeps_the_unreadable_string_notice | even + string 7 with 6 errors; decode --quiet | stderr `[]` | contains `\nUNREADABLE STRING. string 7 of the input could not be read:\n` |
| 6 | quiet_stderr_is_the_loud_stderr_without_the_report | fixtures of rows 1-5 (clean, corrected, transliterated, duplicate, unreadable); decode vs decode --quiet, offline | `left: ""`, right: the loud suffix from `\nWARNING: no bitcoind reachable.` | quiet stderr == loud stderr from that blank line on; stdout equal |
| 7 | quiet_with_a_node_stays_silent_on_a_clean_set (CONTROL) | even; decode --quiet, `node_stub` | PASS | stderr `""`, stdout the `even` raw hex |
| 8 | quiet_stdout_is_the_hex (CONTROL) | row 1 fixture; decode --quiet | PASS | stdout `<even raw_hex>\n`, exit 0 |
| 9 | verify_mismatch_prints_no_ok | even; verify --transaction <uneven raw_hex file> | `OK printed before REFUSED:` (`mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.` then the refusal) | exit 1; first line `mt verify: REFUSED — §1.1, the supplied transaction is not the one on these strings`; no `mt verify: OK` |
| 10 | verify_missing_transaction_prints_no_ok | even; verify --transaction /nonexistent/x.hex | same, refusal `cannot read /nonexistent/x.hex` | exit 1; first line `mt verify: REFUSED — §1.1, cannot read /nonexistent/x.hex`; no OK |
| 11 | verify_unreadable_transaction_prints_no_ok | even; --transaction file `this is not a transaction\n` | same, refusal `mt verify: REFUSED — §8.2e, input is not a PSBT or a raw transaction (25 bytes)` | non-zero exit; first line starts `mt verify: REFUSED — `; no OK |
| 12 | verify_refusal_keeps_the_notices | row 1 fixture; --transaction <uneven raw_hex> | `OK printed before REFUSED:` | exit 1; no OK; `CORRECTION APPLIED. 1 chunk needed repair:` before the §1.1 refusal line |
| 13 | verify_match_prints_one_ok_then_the_match_line (CONTROL) | even; --transaction <even raw_hex> | PASS | stderr exactly `mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.\n  --transaction matches, on the full txid.\n` |
| 14 | verify_match_keeps_ok_notices_match_order (CONTROL) | row 1 fixture; --transaction <even raw_hex> | PASS | one OK, first; CORRECTION block; match line last |

## 7. Tests that change

- `crates/mt-cli/tests/decode_verify.rs`
  `decode_says_nothing_about_an_owner_only_stdout` (:1541-1571): it runs
  `decode --quiet --bitcoin-cli /nonexistent/bitcoin-cli` and asserts stderr is
  `""` twice (file at 0600, and the pipe). Under Q1 that stderr is the no-node
  warning. Change both `assert_eq!(err, "")` checks to: stderr does not contain
  `stdout is a file of mode` (the F-275 headline, `validate.rs:1176`) **and**
  stderr starts with `\nWARNING: no bitcoind reachable.`; the pipe message
  becomes "an anonymous pipe is mode 0600; the documented pipeline gets no mode
  warning". The test's name and its subject (no F-275 warning at 0600) stay.
  Verified by the scratch run in section 6 (the whole `decode_verify` file
  passes with this change and the fix). No other existing test changes: the
  other `--quiet` rows (`decode_verify.rs:1504-1610`) assert with `contains`
  and pass unchanged.
- Added: `crates/mt-cli/tests/verdict_notice.rs` (14 rows, verbatim from the
  sidecar).
- `refusals.toml`: unchanged (no refusal added, removed or reworded).

Coverage map (rule -> rows): Q1 (a) -> 2, 6; Q1 (b) -> 1, 3, 4, 5, 6; Q1 (c) ->
existing `the_decode_warning_is_never_suppressed_and_is_data_under_json`; Q1
"nothing else" -> 6; Q2 -> 2, 7; Q3 -> 7, 8; V1 refusal -> 9, 10, 11, 12; V1
pass -> 13, 14; V2 -> existing `verify_reports_ok_on_a_clean_set`,
`verify_reports_its_margin_not_just_a_verdict`; V3 -> 9, 10 (exact refusal
first line), 12. Spec delta 1 -> 1-6; 2 -> 9-14; 3 -> the changed test above.

## 8. Build order and files

Source files this FIX changes (for build ordering against the other mt rows):
`crates/mt-cli/src/main.rs` (functions `decode` and `verify` only). Tests:
`crates/mt-cli/tests/verdict_notice.rs` (new),
`crates/mt-cli/tests/decode_verify.rs` (one test). Spec copy:
`design/SPEC_mt_v0_1.md` (re-sync). Neighbours touching the same functions:
`fix/sec-read-args-transaction-verify-only` (moves `--transaction` onto `verify`;
same `verify` body) and `fix/mt-arg-intake` (argv; `main.rs` near :291) — rebase
on whichever lands first; the rules here are independent of both.

1. Install the sidecar; run the row command; see `14 tests run: 4 passed, 10 failed`.
2. Q1 + Q2 (decode): rows 1-6 GREEN, 7-8 stay GREEN; apply the section 7 test
   change in the same commit.
3. V1 (verify): rows 9-12 GREEN, 13-14 stay GREEN.
4. Re-sync the spec copy once mnemonic-engrave carries section 5's delta.
5. Gate (section 10) once to a file.

## 9. Open points for the architect (no STOP)

- **O1 reading of T21 for mt-Q14.** T21 rules the OK line explicitly; that the
  notices still print on a refused comparison is read from its first clause
  ("warnings are never suppressed") and SPEC :3484-3485 ("Warnings and refusals
  are NEVER suppressed, on any verb"). The stage-T scratch fix dropped them
  (triage-mt-t6.md:16). If the architect reads T21 otherwise, V1's refusal
  branch prints only the refusal, and row 12 inverts to assert no
  `CORRECTION APPLIED`.
- **O2 node probe under `--quiet`** (Q2). Printing the no-node warning needs
  the probe; the alternative (print it without probing) would claim "no bitcoind
  reachable" while one is. Recorded so a reviewer sees the new `bitcoin-cli`
  call on the quiet path is deliberate.

## 10. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
The CI `test` job (`.github/workflows/ci.yml`) plus the rows:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expect base count + 14, 0
   failed; if another mt FIX lands first, base count moves by its own additions)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh` (count unchanged from base)
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh` and `bash ci/code-docs-check.sh`
10. Rows by name: `cargo nextest run --locked -p mt-cli --test verdict_notice`
    -> `14 tests run: 14 passed`. Fewer than 14 is a FAIL.

## 11. FOLLOWUPS (listed in the author's report, not appended here)

No `design/FOLLOWUPS.md` line names I10 or I23 at 88ac6d9 (grep for `quiet`,
`verify: OK`, `I10`, `I23`: no match); none closed, none filed.
