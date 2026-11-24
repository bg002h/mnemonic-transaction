# FIX: printed remedies and the documented live script (bughunt-1119 FIX table row 39, "FIX-J printed remedies")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-23. Keys: bh-mt-I22 (I),
bh-mt-I25 (I). Rulings: T15 (`coordinator/agent-reports/architect-bughunt-1119-questions.md:53`,
"reword the remedy so it works as printed (take `hex` from Core's JSON once
`complete` is true); do not teach `encode` a new input form") and Brian B8
(`coordinator/memory/product-rulings-sweep-bughunt-2026-11-22.md:29`, README and
remedy code count as printed), both binding. Track: small (one Opus review
round). Base: origin/main `88ac6d9`. Branch `fix/mt-printed-remedies`.
Two remedy texts change and the live script gains one failure-line template;
every new text is PROPOSED (architect confirms). No flag, exit code, refusal,
input form or spec rule changes.

Not to be confused with the older `design/FIX_bughunt_mt_*.md` (a different hunt).

## 1. Findings (RED run at 88ac6d9; section 6 has the commands and output)

- **I22 (the §8.1 remedy fails as printed).** `mt encode` of a PSBT with
  unfinalized inputs refuses (§8.1) with the remedy "Run `bitcoin-cli
  finalizepsbt <psbt>` and pass the result." (`crates/mt-cli/src/validate.rs:77`).
  Core's `finalizepsbt` answers with a JSON object, `{"hex": ..., "complete":
  true}` when it extracts, `{"psbt": ..., "complete": false}` when signatures
  are missing (Bitcoin Core v25.0 `src/rpc/rawtransaction.cpp:1416-1435`:
  `psbt` optional "if not extracted", `hex` optional "if extracted", `complete`
  always; `extract` defaults to true). `encode` refuses that object, so the
  remedy, followed as printed, ends in a second refusal. The `hex` value alone
  encodes.
- **I22 sibling (the §10.10 remedy, same defect).** A txid given where a
  transaction belongs refuses (§10.10) with the remedy "Fetch the transaction:
  `bitcoin-cli getrawtransaction <txid>`, then pass that. Or `bitcoin-cli
  finalizepsbt <psbt>` if you have not broadcast yet." (`crates/mt-cli/src/main.rs:2163-2166`).
  The `getrawtransaction` branch works as printed (non-verbose, Core returns the
  hex string); the `finalizepsbt` branch inherits "pass that" and fails the same
  way. Under B8 this is the same printed-remedy defect, so this FIX rewords it
  too (open point O1 lets the architect drop it back to a FOLLOWUPS line).
- **I25 (the README's live script stops at stage 1 and hides why).** README.md:52
  prints `MT_RT=./rt ./scripts/live-smoke-test.sh`. Under the default umask 022,
  the shell creates `$WORK/plates.txt` (script line 66, `>"$WORK/plates.txt"`)
  as mode 0644 before the `chmod 600` on line 67 runs, and `mt encode` refuses a
  stdout file that others can read (§8.2h, `validate.rs:1076-1095`). `set -e`
  ends the script at line 66 with exit 1; the EXIT trap (line 36) deletes
  `$WORK`, and with it `enc.err`, the only copy of the refusal. The operator sees
  `== 1. encode ==` and nothing else. Steps 3 and 4 (lines 79, 84) send mt's
  stderr to `/dev/null`, so any failure there is silent too.

## 2. Rulings (binding, quoted)

- T15: "reword the remedy so it works as printed (take `hex` from Core's JSON
  once `complete` is true); do not teach `encode` a new input form (smaller; a
  new form is full track)."
- B8: "\"printed command\" includes README and remedy code."

## 3. Behaviour after the fix (rules)

- **R1 (§8.1 remedy).** The remedy of the §8.1 refusal (`finalized_guard_psbt`,
  `validate.rs:77`) is the text in section 4, T1. Verdict, mechanism, section and
  exit code (1) stay.
- **R2 (§10.10 remedy).** The remedy of the txid refusal (`main.rs:2163-2166`)
  is the text in section 4, T2. Verdict, mechanism, section and exit code stay.

**R2b (§10.10 remedy from `decode` and `verify`).** When `verb` is `decode` or `verify`, the remedy is T2b
instead of T2 (`encode` and `inspect` keep T2; both accept a raw transaction). Shape:
`.with_remedy(if verb == "decode" || verb == "verify" { format!(T2b) } else { T2.to_string() })`.

- **R3 (no new input form, T15).** `encode` still refuses Core's whole
  `finalizepsbt` answer; its refusal text is unchanged. Its `hex` field, passed
  alone, encodes (already true at base; row 3 pins both).
- **R4 (umask).** `scripts/live-smoke-test.sh` sets `umask 077` immediately after
  its `cd` (line 25), before `mktemp -d` (line 35), so every file it creates is
  mode 0600 from creation whatever the caller's umask. The two `chmod 600`
  lines (63, 67) are deleted: they are redundant under R4, and line 67 runs too
  late to matter, which is the defect.
- **R5 (a failing mt step shows its stderr).** Each of the four mt steps writes
  its stderr to a file in `$WORK` (`enc.err`, `ver.err` as today; `ins.err` and
  `dec.err` replace the two `2>/dev/null`). When a step exits non-zero, the
  script prints the T3 line, then that file with every line prefixed by three
  spaces (the script's existing `sed 's/^/   /'` indent), then exits 1. This
  happens before the EXIT trap removes `$WORK`. Implementation shape (builder's
  choice of form): a `fail_step <verb> <status> <errfile>` shell function called
  as `"$MT" <verb> ... 2>"$WORK/<x>.err" || fail_step <verb> $? "$WORK/<x>.err"`;
  `$?` there is the mt step's status. The existing content checks (lines 69, 70,
  75, 80, 85) and their texts stay.
- **R6 (README).** README.md:52 stays as printed; R4 and R5 make it work as
  printed. `scripts/README.md:12` stays.

## 4. Holder-visible texts (all PROPOSED, architect confirms)

Both remedies print through `Refusal`'s Display, wrapped at 68 columns and
indented two spaces (`refusal.rs:89-93`); the strings below are the
`with_remedy` arguments.

- **T1 (R1), replaces "Run `bitcoin-cli finalizepsbt <psbt>` and pass the
  result.":**
  ``Run `bitcoin-cli finalizepsbt <psbt>`. If its answer shows `"complete": true`, pass the value of its `hex` field (the signed transaction), not the whole answer. If it shows `"complete": false`, the PSBT still lacks signatures: sign it first (`walletprocesspsbt`), then run `finalizepsbt` again.``
- **T2 (R2), replaces "Fetch the transaction: `bitcoin-cli getrawtransaction
  <txid>`, then pass that. Or `bitcoin-cli finalizepsbt <psbt>` if you have not
  broadcast yet.":**
  ``Fetch the transaction: `bitcoin-cli getrawtransaction <txid>`, then pass that. If you have not broadcast yet, run `bitcoin-cli finalizepsbt <psbt>` instead and pass the value of its `hex` field (present once it shows `"complete": true`).``
- **T2b (R2b):** `` `mt <verb>` reads the mt1 strings from the plates, not a transaction. To look at the transaction this txid names, fetch it with `bitcoin-cli getrawtransaction <txid>` and pass that to `mt inspect`. ``
- **T3 (R5), new script line on stdout:** `FAIL: mt <verb> exited <status>; its stderr follows:`
  where `<verb>` is `encode`, `verify`, `inspect` or `decode` and `<status>` is
  mt's exit status in decimal. Example: `FAIL: mt encode exited 1; its stderr follows:`.

The row file pins T1 and T2 as constants (`REMEDY_UNFINALIZED`, `REMEDY_TXID`)
and T3 by its encode instance; a wording change at confirmation edits those
constants and this section only.

## 5. Spec, README, help, manual, CHANGELOG

- Spec (`design/SPEC_mt_v0_1.md`, a copy owned by mnemonic-engrave): no
  change. It does not quote either remedy (`grep -n "pass the result\|then pass
  that" design/SPEC_mt_v0_1.md` -> no output at base); its `finalizepsbt`
  passages (:2419, :3266) describe Core's `extract` default, which this FIX
  relies on and does not change.
- README.md: no change (R6). scripts/README.md: no change.
- `--help`: no change (neither text is in help).
- Toolkit manual (`mnemonic-toolkit` `docs/manual/src/40-cli-reference/`): no
  lockstep. `grep -rn "finalizepsbt\|pass the result\|then pass that"` there ->
  no output; `grep -rln "mt encode\|not finalized\|transaction ID"` -> no output.
- CHANGELOG.md: one bullet under the next unreleased `mt-cli` heading
  (`### Changed`): "**FIX-J: printed remedies (bh-mt-I22, I25).** The §8.1 and
  §10.10 remedies tell the operator to pass the `hex` field of `finalizepsbt`'s
  answer, not the whole answer. `scripts/live-smoke-test.sh` sets `umask 077`
  and prints a failing step's stderr." If the file has no unreleased heading
  when the builder lands, add `## mt-cli [Unreleased]` above the newest entry.

## 6. RED rows

Row file: `crates/mt-cli/tests/printed_remedies.rs`, installed verbatim from
`design/FIX_bh1119_mt_printed_remedies/printed_remedies.rs.txt` (6 rows; uses
`tests/common/mod.rs` `fixture_txids` and `tests/fixtures/p5_base.json`, signet
and regtest test material only; rows 4-5 need `bash`, `python3` and `/bin/sh`,
which the CI `ubuntu-latest` runner has).

Command (run by the author at 88ac6d9 with the row file copied in, then removed):
`cargo nextest run --locked -p mt-cli --test printed_remedies --no-fail-fast`

| # | row | rule | at base | expected after |
|---|---|---|---|---|
| 1 | `i22_unfinalized_psbt_remedy_names_hex_field` | R1, T1 | FAIL | PASS |
| 2 | `i22_txid_remedy_names_hex_field` | R2, T2 | FAIL | PASS |
| 2b | `i22_txid_remedy_reading_verbs_point_to_inspect` | R2b, T2b | FAIL | PASS |
| 3 | `i22_finalizepsbt_answer_refused_hex_field_encodes` | R3 | PASS (guard) | PASS |
| 4 | `i25_live_smoke_test_under_umask_022` | R4 | FAIL | PASS |
| 5 | `i25_live_smoke_test_failure_shows_diagnostic` | R5, T3 | FAIL | PASS |

Output at 88ac6d9 (`Summary [...] 5 tests run: 1 passed, 4 failed, 0 skipped`), the
failing assertions:

- Row 1: ``left: "Run `bitcoin-cli finalizepsbt <psbt>` and pass the result."`` (the T1 constant on the right).
- Row 2: ``left: "Fetch the transaction: `bitcoin-cli getrawtransaction <txid>`, then pass that. Or `bitcoin-cli finalizepsbt <psbt>` if you have not broadcast yet."`` (T2 on the right).
- Row 4: `status Some(1)`; stdout ends `== 1. encode ==`; stderr empty.
- Row 5: `left: Some(64)`, `right: Some(1)`; stdout ends `== 1. encode ==` (set -e
  exits with mt's status and no diagnostic).

Cause checks run by hand at 88ac6d9, `target/debug/mt`:
- Row 4's cause is the umask and nothing else: the base script, a copy in a temp
  root with the row-4 scripted node, ends `live-smoke-test: the bytes recovered
  from the engraving are a transaction` from `umask 077` and stops at
  `== 1. encode ==` from `umask 022`. `mt encode --bitcoin-cli
  /nonexistent/bitcoin-cli --in <raw hex file> > <new file>` from umask 022
  exits 1 with `mt encode: REFUSED — §8.2h, stdout is a file of mode 0644 — its
  permissions grant read to group and others.`
- Row 3's whole-answer refusal at base (RED_JSON for O3), exit 1:
  `mt encode: REFUSED — §8.2e, input is not a PSBT or a raw transaction (704 bytes)`,
  mechanism "mt accepts a binary PSBT (magic `psbt\xff`), a base64 PSBT
  (beginning `cHNidP`), or a raw signed transaction as hex. This input begins 7b
  22 68 65 78 22 3a 22 and matches none of them.", remedy "Check the file is the
  one you meant, and pass it with --in."

GREEN check (author, scratch, reverted): T1 and T2 applied as the `with_remedy`
strings, the script changed per R4-R5 with a `fail_step` function; the same
command printed `5 tests run: 5 passed, 0 skipped`, and `bash -n` passed. The
scratch diff is not committed (builder authors the change).

Coverage map: R1 -> 1; R2 -> 2; R2b -> 2b; R3 -> 3; R4 -> 4; R5 -> 5 (encode step; the
verify, inspect and decode steps share the function and are checked by gate
row 9); R6 -> 4 (runs the script as README.md:52 prints it, under umask 022);
T1 -> 1; T2 -> 2; T2b -> 2b; T3 -> 5.

## 7. Tests that change

None at base pin either remedy text or the script (`grep -rn "pass the
result\|then pass that\|live-smoke" crates/` -> no output). `refusals.toml` and
`scripts/check-refusal-coverage.sh` key on sections, not remedies: no change.
One new test binary (`printed_remedies`, 6 rows).

## 8. Build order and files

Source files this FIX changes (for the coordinator's mt build order):
- `crates/mt-cli/src/validate.rs` (one `with_remedy` string, line 77)
- `crates/mt-cli/src/main.rs` (one `with_remedy` string, lines 2163-2166)
- `scripts/live-smoke-test.sh`
- `crates/mt-cli/tests/printed_remedies.rs` (new)
- `CHANGELOG.md`

Neighbours touching the same files: any mt FIX editing `validate.rs` or
`main.rs` near these lines rebases trivially (string-only change). No
dependency on another row.

## 9. Open points for the architect (no STOP)

- **O1 (scope of R2).** The §10.10 remedy is outside key I22's citation but has
  the same defect under B8. Included; if the architect prefers I22 strictly,
  drop R2, T2 and row 2 and file the line in section 11 instead.
- **O2 (a third remedy, not fixed).** The §8.3 remedy (`validate.rs:112`, raw
  transaction with unsigned inputs) reads "Sign it first — `walletprocesspsbt`,
  then `finalizepsbt`." `walletprocesspsbt` takes a PSBT, not the raw hex the
  operator holds (Core offers `converttopsbt` or `signrawtransactionwithwallet`
  for that). It prints no runnable command line, so this FIX leaves it; listed
  as a proposed FOLLOWUPS line (section 11).
- **O3 (the refusal of Core's whole answer).** R3 keeps that refusal's text.
  Section 6 records what it prints at base (RED_JSON): a §8.2e refusal whose remedy, "Check the file is the one you meant", does not name the `hex` field. A remedy there that names
  the `hex` field would need `encode` to recognise Core's answer, which T15
  rules out on this track.

O1: keep R2 and add R2b/T2b (review r1). O2: file the author's FOLLOWUPS line. O3: keep the refusal text; M-3 filed as a FOLLOWUPS line.

## 10. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
The CI `test` job (`.github/workflows/ci.yml`) plus the rows:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (base count + 6; if another mt
   FIX lands first, the base count moves by its own additions)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh` (count unchanged from base)
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. `bash -n scripts/live-smoke-test.sh`, then
   `grep -c 'fail_step' scripts/live-smoke-test.sh` -> 5 (one definition, four
   calls) and `grep -n '2>/dev/null\|chmod 600' scripts/live-smoke-test.sh` -> no
   output. If the builder names the function differently, the review checks the
   four call sites by hand.
10. `bash ci/docs-only.test.sh` and `bash ci/code-docs-check.sh`
11. Rows by name: `cargo nextest run --locked -p mt-cli --test printed_remedies`
    -> `6 tests run: 6 passed`. Fewer than 6 is a FAIL.
12. `grep -n 'and pass the result' crates/mt-cli/src/validate.rs` -> no output.
13. Optional, not CI: a real regtest node, `MT_RT=./rt ./scripts/live-smoke-test.sh`
    from umask 022 -> ends with `allowed: true` (grade-mt.md:118 asks for it;
    the rows use a scripted node).

## 11. FOLLOWUPS (listed in the author's report, not appended here)

No existing FOLLOWUPS line names I22 or I25 (`grep -n "I22\|I25\|live-smoke\|finalizepsbt" design/FOLLOWUPS.md` -> no output at base). Proposed new line for O2 (and for O1 if R2 is dropped) is in the author's report.
