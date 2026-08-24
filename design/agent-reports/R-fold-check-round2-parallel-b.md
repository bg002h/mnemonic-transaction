# Mechanical fold check, round 2 — independent reviewer, did not author the folds

**Scope.** For each finding in the four persisted reports, was it actually
fixed, and did the fix introduce a new defect? Not a fresh audit. All facts
listed as already-settled in the dispatch brief (fmt/clippy/nextest 160
tests/journeys.sh/check-refusal-coverage.sh/mutate-refusals.sh all green; the
`~FALL 2034` vs `SUMMER 2034` spec defect; mainnet-address rendering) were
taken as given and used as tools, not re-verified.

**Method.** For every finding claimed FIXED: ran the regression test that now
covers it (`cargo nextest run --locked -E 'test(=NAME)'`), and for the highest-
stakes ones (all four Criticals in the funds report, the two live-node
findings, the two false-pass mutations) additionally re-ran the original
"How I checked" reproduction against the built `./target/debug/mt`, or
re-applied the reviewer's exact source mutation and confirmed it now goes red.
For findings claimed fixed with NO regression test, built a fresh fixture from
the fixture material already in `crates/mt-cli/tests/fixtures/p5_base.json`
(raw hex, legacy-parent material) and ran the CLI directly. Three source
mutations were applied and reverted during this review (journeys.sh's Journey A
empty-stdout mutation, inspect's node-suppression mutation, and
check-refusal-coverage.sh's undeclared-test probe); the working tree is
confirmed clean of all three (`git diff --stat` on the touched files is empty).

**A note on the working tree.** A concurrent, unrelated process was writing to
this same checkout throughout this review — untracked `zz_scratch_*.rs` test
files under `crates/mt-cli/tests/`, an in-progress edit to
`crates/mt-codec/src/string_layer/{bch,bch_decode}.rs`, and untracked reports
`design/agent-reports/R-bch-port-conformance.md` and
`R-journey-walk-round2.md`. None of this is mine, none of it is part of any
fold commit under review, and none of it was touched, read for content, or
relied upon here — it is noted only so a `git status --porcelain` mid-review
doesn't look like contamination from this task. `cargo nextest run` invocations
below were scoped with `-E 'not test(zz_scratch)'` or exact test-name filters
to avoid picking up that debris; the 160-test figure quoted throughout this
report is the count on the four fold commits' own tracked test files, not on
whatever the concurrent process's untracked files add.

## Verdict

**29 FIXED / 2 PARTIAL / 7 NOT FIXED / 0 REGRESSED**, across the 38 Critical/
Important/Minor findings in the four reports (2 spec-defect items, S-1 and S-2,
and one Nit are called out separately below, not counted in this tally per the
task brief). `NC / NI`: **0 Critical / 0 Important / 0 Minor** new defects
found in the nine areas of newly-written code — every legitimate input tried
against them behaved correctly; see "New defects" section.

By report: adversarial-funds **14/14 FIXED**. spec-conformance **10 FIXED / 2
PARTIAL / 7 NOT FIXED** (of 19). false-pass **3/3 FIXED**. live-node **2/2
FIXED**.

The 7 NOT FIXED are all Minor except one Important (positional autocorrect,
§1.1e), and none were declared as a deliberate deferral in any fold commit
message — they were simply not addressed, and in one case (§8.2c's per-input
refusal, M-7) the fold's own commit message describes fixing something that,
on inspection, is a different guard with a similar name.

## Finding-by-finding

### R-post-impl-adversarial-funds.md (2C / 8I / 4M) — 14/14 FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C1 | content id never re-derived; `verify` lies | FIXED | `content_id_guard` (`main.rs:1045`), called from verify/decode/inspect. Ran `mt verify`/`decode`/`inspect` on a forged set (bytes of one pinned vector under another's txid) via `cargo nextest -E 'test(a_genuine_set_re_derives_its_id)+test(decode_emits_nothing_when_the_id_does_not_re_derive)+test(verify_refuses_a_set_that_does_not_re_derive_its_id)+test(inspect_refuses_a_set_that_does_not_re_derive_its_id)'` → all PASS. Full clean round trip (encode→verify→decode→inspect) on a real fixture also confirmed the positive case still works. |
| C2 | §8.2f bypassed by clap; argument echoed | FIXED | `validate::command_line_guard` now runs before `Cli::parse()` and covers `mt1` strings too. Manually ran `mt encode --bitcoin-cli /nonexistent "<678-char hex>"` and `mt verify --bitcoin-cli /nonexistent "<mt1 string>"` directly — both now print `REFUSED — §8.2f`, name shell history/`ps`, and do **not** echo the argument. Tests `there_is_no_way_to_pass_a_transaction_as_an_argument` + `refuses_a_transaction_passed_as_a_command_line_argument` PASS. |
| I3 | legacy warning fires on TXID-BOUND | FIXED | `validate::legacy_unbound_warning` (`validate.rs:388`) now gates on resolved provenance, not `witness.is_empty()`. Test `a_txid_bound_legacy_input_produces_no_warning` builds a real legacy PSBT with `non_witness_utxo` and asserts the warning is absent and `TXID-BOUND` prints — PASS. |
| I4 | fee arithmetic per-input vs total outputs | FIXED | Same fix as I3. Test `the_warning_never_contradicts_the_fee_row` asserts the warning's quoted fee equals the `FEE` row's on both a bound and unbound legacy input — PASS. |
| I5 | §5 legend / `--from`/`--to`/`--to-label` silently discarded | FIXED | Ran `mt encode --from deadbeef --to "cold storage" --to-label "safe deposit box 12"` directly: stderr now prints `FORMAT: mt1 codex32`, `FROM WALLET deadbeef`, `TO cold storage (safe deposit box 12)  7.99900000 BTC`. Ran without the flags: prints `FROM WALLET ????????    <-- NOT SUPPLIED` / `TO ????????  ... <-- NOT SUPPLIED` plus the loud absence-warning paragraph the spec's §10.4 requires. (The `n/m` per-string sub-clause is untested by this report's own reproduction and is covered separately under spec-conformance I-3, where it's flagged PARTIAL.) |
| I6 | §1.1e length check absent | FIXED | `read_strings::length_report` (new). Ran `mt verify` on a set with one character deleted from string 3 of 9 clean strings → `REFUSED — §1.1e, ... string 3: 86 characters (expected 87) — 1 character is MISSING`, not a missing-chunk accusation. Tests `a_dropped_character_is_named_as_a_length_error_not_a_missing_plate`, `a_legitimately_short_final_chunk_is_not_a_length_error`, `an_extra_character_is_named_as_extra`, `a_correct_length_string_that_fails_keeps_its_own_message` all PASS. |
| I7 | `--separator` accepts non-whitespace, produces unreadable steel | FIXED | Ran `mt encode --group-size 5 --separator -` → now `REFUSED — §1.1e, --separator "-" is not whitespace`, refused before any output. Tests `a_non_whitespace_separator_is_refused_before_anything_is_cut` + `whitespace_separators_round_trip` PASS; manually confirmed a tab separator round-trips through `mt verify` cleanly. |
| I8 | §8.2d binds txid, not vout; unverified value renders TXID-BOUND | FIXED | `psbt_input_value` (`validate.rs:282`) now returns `(sats, ValueSource)` together, so the label can't disagree with where the number came from — a record with no output at the input's vout now falls through to `PsbtClaimed`, not `TxidBound`. Test `a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound` PASS. |
| I9 | UNREADABLE-STRING notice asserts what it cannot know | FIXED | Rebuilt the exact scenario (9-string clean set + 1 appended unreadable string, BCH-broken past `t=4`): message changed from an unconditional "that plate is scrap, re-cut it" to "mt cannot tell you which chunk that string was, or whether it belongs to this set at all ... Do not discard the plate on this message alone: check whether it is from another engraving first." |
| I10 | `--input-value` via `f64`, panics on inf/1e300, silent nonsense on nan/-5 | FIXED | `parse_btc` (`main.rs:1293`) now parses as a decimal string, no `f64` anywhere. Ran all four hostile values directly: `inf`, `1e300`, `nan`, `-5` each now print `REFUSED — §8.2c, --input-value amount "X" is not a BTC amount` — zero panics. Boundary-checked: `21000000` (max supply) parses fine, `21000000.00000001` is correctly refused. **Provenance note:** this landed entirely in fold commit 95bad20 (`parse_btc` didn't exist before it, confirmed via `git log -S`); the later fold commit 1df6614's message lists "adversarial I3, I4, I6 **and I10**" among what it answers, but `git diff 95bad20..1df6614 -- crates/mt-cli/src/main.rs` shows zero touches to `parse_btc` or amount parsing — that mention is a bookkeeping inaccuracy in the commit message, not a second round of work; the fix itself is correctly in place. |
| M11 | out-of-range/duplicate `--input-value` index silently ignored | FIXED | `check_input_value_indices` / `input_index_range_guard` (`main.rs:1243`, `1268`). Ran `mt encode --input-value 1:4.0 --input-value 2:4.0` on a 2-input tx → `REFUSED — §8.2c, --input-value names input 2, but this transaction has 2 input(s)`. Tests `an_input_value_index_that_names_no_input_is_refused` + `a_repeated_input_value_index_is_refused` PASS. |
| M12 | decode's refusal labelled `mt encode:`, gives encode-path advice | FIXED | `txid_display` (`main.rs:607`) now takes `verb` and branches its remedy. Built a throwaway integration test encoding 40 bytes of garbage (not a valid tx) under a valid header/checksum and ran `mt decode` on it: now prints `mt decode: REFUSED — §8.2e, ...` with recovery-path advice ("check every plate carries the same 8 characters after `mt1`"), not `mt encode:`/`finalizepsbt`. Test file removed after use, tree confirmed clean. |
| M13 | contradicting `--input-value` discarded without a word | FIXED | Ran `mt encode --input-value 0:3.0` against a legacy PSBT whose record says 10 BTC → `WARNING: --input-value 0:3.00000000 BTC disagrees with the PSBT, and mt used the PSBT.` with the reason named. Test `a_value_contradicting_the_psbt_is_reported_not_swallowed` PASS. |
| M14 | absurd-fee ceiling truncates (25,001 effective) | FIXED | `validate.rs:182` now compares `fee > MAX_FEE_RATE_SAT_VB * vb` (cross-multiplied) instead of dividing — confirmed by reading the current source; no more integer-division truncation possible. |

### R-post-impl-spec-conformance.md (4C / 7I / 8M + S-1/S-2) — 10 FIXED / 2 PARTIAL / 7 NOT FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C-1 | content id never re-derived | FIXED | Same as adversarial C1 above. |
| C-2 | no `LOCK_TIME_THRESHOLD` branch; timestamp reported as height | FIXED | New `crates/mt-cli/src/locktime.rs`. Rewrote the fixture's `nLockTime` to `1_800_000_000` (a timestamp) and ran `mt encode` directly → `LOCKTIME  LOCKED UNTIL 2027-01-15T08:00Z   current MTP unknown (no node)`, matching the fold commit's own claimed output exactly. Tests `a_value_below_the_threshold_is_a_height`, `a_timestamp_locktime_is_never_presented_as_a_height`, `the_threshold_decides_height_versus_timestamp` PASS. |
| C-3 | no `nSequence` rule; all-final tx reported as locked | FIXED | Rewrote the fixture to `nLockTime=96` with **every** input's sequence `0xffffffff` and ran `mt encode` → `LOCKTIME  nLockTime 96 present but NOT ENFORCED (all inputs final)`. Also tried the **mixed** case (one input final, one not) with the same locktime, which the original report didn't test: correctly reports `LOCKED TO BLOCK 96` (enforced), confirming the rule isn't just right at the two extremes. Test `a_locktime_no_input_enforces_is_reported_as_not_enforced` PASS. |
| C-4 | `decode` prints two hand-composed lines, not the report | FIXED | Ran `mt decode` directly on a clean set: stderr now prints the full shared report (`mt1 SET`/`TX`/`OUT`/`FEE`/`LOCKTIME`/`INPUTS`/`STATUS`) plus the no-node warning, not the old two-line form. Test `decode_reports_on_stderr_and_hex_on_stdout` PASS. |
| I-1 | §1.1e modal length check absent | FIXED | Same as adversarial I6. |
| I-2 | §1.1e **positional** autocorrect (`l/I/i→1` at idx 2; `1/i→l, o→0, b→6` at idx 3+) | **NOT FIXED** | `grep -rn "'l'\|=> '1'\|=> '0'\|=> '6'" crates/mt-codec/src/string_layer/pipeline.rs` and a full diff of the fold range (`git diff 1b2859c..268b354`) show no substitution table anywhere. Reproduced directly: a data-part character changed to `o` still gives `REFUSED — §1.1, ... chunk 1 of 9 is missing` — the exact original wrong diagnosis (an `mtl…`-prefix variant of the same test produced a *different* wrong diagnosis, a length-error, because the new length check now intercepts it first — but no autocorrect fired in either case). The 95bad20 fold commit message explicitly listed this as "STILL OPEN... for the next round" alongside the modal length check; the modal length check was fixed in 1df6614, this was not, and no later commit mentions it again. |
| I-3 | legend text + 4 of 12 ruled flags silently discarded | **PARTIAL** | The once-printed fields (`FORMAT: mt1 codex32`, `FROM`, `TO`, `LOCKED TO BLOCK/UNTIL`, plus the absence warning) are fixed — see adversarial I5. The **per-string `n/m`** clause the spec also rules under this same finding ("printed per string: n/m") is still absent: `grep` for any `{}/{}`-shaped per-string label in `main.rs`/`blocks.rs`/`report.rs` finds nothing, and a full `--from`/`--to` run's stderr has no `n/m` marker beside any string. |
| I-4 | §6a no-node warning at encode time absent | FIXED | `blocks::encode_no_node_warning()` (`blocks.rs:211`, new). Ran `mt encode` offline: prints `WARNING: no bitcoind reachable — mt could not check the chain before you cut.` naming all three unanswered questions (unspent/fee/locktime). Wording differs from the spec's illustrative block (no literal `§8.5`/`§6a`/`§8.4` tags, no line-item `UNKNOWN` column) but the substantive gap — no warning existed at all — is closed. |
| I-5 | legacy warning fires on bound value | FIXED | Same as adversarial I3/I4. |
| I-6 | §8.4 reference pair / season estimate / negative-subtraction warning absent | FIXED | Ran `mt encode` on `nLockTime=900000` (below `MT_REF_HEIGHT=963759`) → `WARNING: nLockTime 900000 is BELOW this build's reference height 963759. ... Treat it as spendable now.` Ran on the spec's own worked-example height `1383520` → `LOCKED TO BLOCK 1383520 ~SUMMER 2034` (the algorithmically-correct season, not the spec's own erroneous `~FALL` — consistent with the already-settled, pinned spec defect). Test `a_lock_below_the_reference_height_warns` PASS, plus `locktime::tests::the_worked_example_projects_to_summer_not_the_spec_s_fall` (pre-existing pin). |
| I-7 | `--transaction` refuses the PSBT half of its own ruling | FIXED | Tests `verify_transaction_accepts_a_psbt` + `verify_transaction_refuses_an_unfinalized_psbt` PASS — the latter confirms the fix didn't just accept any PSBT but correctly still refuses an unfinalized one (extracts to different bytes than what was engraved). |
| M-1 | LOCKTIME row invents a sixth spelling | FIXED | Report's `LOCKTIME` row now delegates to `locktime::Lock::report_row`, which only emits the ruled spellings — confirmed by the C-2/C-3 reproductions above, both of which show a ruled spelling (`LOCKED UNTIL ...` / `... NOT ENFORCED (all inputs final)`), never the old `block {n}, current height {h}` form. Test `the_row_uses_only_the_ruled_spellings` PASS. |
| M-2 | `mt1 SET` row drops the set id; `decode` prints a second format | **PARTIAL** | `decode`'s second/third wording is gone — it now uses the same `Report::render()` as `inspect` (see C-4). But the row itself still doesn't carry the set id: current code is `writeln!(s, "mt1 SET   {n} strings, 1..{n} all present")` (`report.rs:320`) — no `0x…` id slot exists on `Report`, confirmed by reading the struct and by every `mt1 SET` line produced in this review (e.g. `mt1 SET   9 strings, 1..9 all present`, never `0x4665e`). |
| M-3 | offline no-node warning says "locked to block 0" contradicting `NO TIMELOCK` | **NOT FIXED** | Ran `mt decode` on the clean 9-string, `nLockTime=0` fixture: `LOCKTIME  NO TIMELOCK  current height unknown (no node)` immediately followed a few lines later, in the same run, by `- has the locktime passed? UNKNOWN` / `locked to block 0, current height unknown` inside `report::no_node_warning` (`report.rs:406`). That function still hardcodes the locktime value rather than routing through `locktime::Lock`; direct contradiction reproduced live in a single command's output. |
| M-4 | raw-tx warning ignores §8.2e's `[with node]` branch | **NOT FIXED** | Built a stub `bitcoin-cli` answering `gettxout` unspent (4.0 BTC each input) for the raw-hex fixture and ran `mt encode --bitcoin-cli <stub>`: `FEE       0.00100000 BTC` computed correctly, yet the same run still prints, unconditionally, `WARNING: this is a RAW TRANSACTION ... mt cannot see any input's value and cannot check the fee.` (`main.rs:499-510`, still gated only on `if from_raw_hex`, no `node.is_none()` check) — directly contradicting the FEE row two lines above it. |
| M-5 | correction-coverage block drops "or a lost PLATE" | **NOT FIXED** | `blocks.rs`'s `correction_coverage` still reads "It cannot repair a missing STRING either" (unchanged wording); confirmed live in the mutated-journeys.sh output captured during the false-pass C1 repro below. Spec wants "a missing STRING **or a lost PLATE**". |
| M-6 | §10.10 txid classification absent | **NOT FIXED** | `grep -n "64.character\|64-character" crates/mt-cli/src/input.rs crates/mt-cli/src/main.rs` — no hits. A pasted 64-hex-char txid still reaches the generic "not a decodable Bitcoin transaction" refusal rather than being named as a txid. |
| M-7 | §8.2c's per-input refusal has no `refusals.toml` entry | **NOT FIXED** | The fold commit dfc981c's message claims "§3b's all-elided refusal and §8.2c's per-input refusal each had a passing test and no entry ... now ruled entries" — but the guard the original finding names, `validate::require_psbt_input_values` (`validate.rs:328`, live code, called from `main.rs:270`), has **zero** tests anywhere (`grep -rln "require_psbt_input_values\|carries no UTXO record and no supplied value" crates/mt-cli/tests/*.rs` → no hits) and no `refusals.toml` entry naming it. What the fold actually added an entry for is a **different, similarly-named** §8.2c guard — `parse_input_values`/`input_value_must_be_per_input` ("per input, never a total") — which the commit message's wording ("per-input refusal") plausibly conflates with the one the review meant. `require_psbt_input_values` remains uncovered by both `check-refusal-coverage.sh` and `mutate-refusals.sh`. |
| M-8 | §10.20 malleability caveat carried nowhere | **NOT FIXED** | `grep -rn "malleab\|superseded" crates/mt-cli/src/*.rs` → no hits. |
| Nit | `OUT`/`INPUTS` render `output(s)`/`input(s)` vs spec's singular/plural | not fixed, non-gating | `report.rs:324,356` unchanged. Noted for completeness only — Nits don't gate per the severity rules. |
| S-1, S-2 | spec ambiguities, not code defects | out of scope | Per the task brief, not findings to fix. Noted: the code's current behavior (no report from `verify`; the LOCKTIME row using `NOT ENFORCED` while the *legend* line separately uses `NO TIMELOCK` for the same all-final transaction, confirmed live during the C-3 repro) is consistent with the resolution S-2 itself proposes — the two spellings now cleanly separate by context (report row vs. legend) rather than colliding. |

### R-post-impl-false-pass.md (2C / 1I) — 3/3 FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C1 | `journeys.sh` Journey A's stdout checks are vacuous against an empty file | FIXED | Re-applied the reviewer's exact mutation (`let rendered: Vec<String> = Vec::new();` after `render()` in `main.rs`), rebuilt, ran `./scripts/journeys.sh` → now exits 1 with three explicit failures: `FAIL stdout is EMPTY — mt reported success and engraved nothing`, `FAIL CUT row says '9' strings, stdout has 0`, `FAIL 0 of 0 stdout lines are not lowercase ungrouped mt1 strings`. Previously this mutation passed clean. Mutation reverted; `git diff --stat crates/mt-cli/src/main.rs` confirmed empty afterward. |
| C2 | `mt inspect`'s node path has zero test coverage | FIXED | Re-applied the reviewer's exact mutation (`let node: Option<node::Node> = None;` after `Node::find` in `inspect()`), rebuilt, ran `cargo nextest run --locked --no-fail-fast -E 'not test(zz_scratch)'` → **exactly 6 tests fail** (`a_reachable_node_answers_the_rows_that_were_unknown`, `inspect_reports_a_dead_plate_rather_than_refusing`, `inspect_distinguishes_pending_from_dead`, `inspect_reports_an_already_confirmed_transaction_as_confirmed`, `reaching_a_node_changes_the_report`, `the_no_node_warning_is_absent_when_a_node_is_there`) — matching the fold commit's own claim "Six tests added; the same mutation now kills all six" exactly. Mutation reverted; diff confirmed empty. |
| I3 | `check-refusal-coverage.sh` only scanned `tests/refusals.rs` | FIXED | Appended the reviewer's exact undeclared test (`refuses_a_thing_nobody_declared`, `assert!(true)`, no `refusals.toml` entry) to `crates/mt-cli/tests/encode.rs`, ran `./scripts/check-refusal-coverage.sh` → now `FAILED`, naming the exact test: `` `refuses_a_thing_nobody_declared` (crates/mt-cli/tests/encode.rs) named refuses_*, but has no entry in refusals.toml ``. Previously silent. Test removed; diff confirmed empty. Also read the widened scan's 3-signal logic and its `EXEMPT` list (2 entries, each with a stated reason for a legitimate false-match) — see "New defects" §9 below; no over-flagging found. |

### R-post-impl-live-node.md (1C / 1I) — 2/2 FIXED

| # | finding | status | evidence |
|---|---|---|---|
| C1 | `encode` tells a CONFIRMED tx "can never be broadcast" | FIXED | `main.rs` now computes `already_confirmed` via `node.is_confirmed(&txid_of(&tx))` **before** the §8.5 per-input loop, mirroring `Report::build`'s ordering — confirmed by reading the code directly. Regression test `an_already_confirmed_transaction_is_not_reported_as_stolen` builds a stub that answers `getrawtransaction` **per txid** (confirms both the tx's own txid and its parents' — the exact discriminator the old single-answer stub couldn't express) and asserts `ok`, absence of "can never be broadcast" / "Build a new transaction", and presence of "ALREADY CONFIRMED" — PASS. The companion `refuses_a_spent_input_whose_parent_confirmed` (case D, genuine theft) still correctly refuses, with a self-check that the stub isn't accidentally modeling the fixture's own output as confirmed — PASS. |
| I1 | reaching a node makes the report strictly worse (fee UNKNOWN with node, known offline) | FIXED | `report.rs`'s `Utxo::Null` arm now falls through to the `claimed` (PSBT/txid-bound/asserted) value instead of terminating in `Provenance::Unknown` (confirmed by reading `report.rs:174-213`). Test `a_node_that_cannot_find_an_outpoint_does_not_discard_the_psbt_record` builds a stub with parents unconfirmed (mempool) and asserts `FEE 0.00100000 BTC` and `TXID-BOUND` hold **both** offline and with the node reachable, and that `STATUS` specifically improves from `UNKNOWN` to `PENDING` (the one row a node legitimately *can* improve) — PASS. |

## New defects introduced by the fold

**None found.** Zero Critical / zero Important / zero Minor, across all nine
named areas. Each was exercised with a legitimate input constructed to be the
hard case, not just the hostile one:

1. **`crates/mt-cli/src/locktime.rs`.** Tried: nLockTime=0 (`NO TIMELOCK`,
   correct); the spec's own worked-example height 1,383,520 (`~SUMMER 2034`,
   correct per the known/pinned spec-defect resolution); a **mixed**-sequence
   transaction (one input final, one not) at a non-zero locktime below the
   reference height — correctly reports `LOCKED TO BLOCK 96` (enforced) with
   the below-reference warning, not `NOT ENFORCED`, confirming the rule isn't
   only correct at the two all-final/none-final extremes the repo's own tests
   cover. CLEAN.
2. **`content_id_guard` / `explain_failure`.** A full clean round trip
   (encode→verify→decode→inspect) on a real fixture reproduces the original
   txid at every step with no false refusal. The legitimately-short-final-chunk
   case (uneven payload) is covered by the repo's own
   `a_legitimately_short_final_chunk_is_not_a_length_error`, which passes —
   confirming `content_id_guard` and the new length-check machinery don't
   collide on the one case explicitly designed to look like damage but isn't.
   CLEAN.
3. **`blocks::legend` / `encode_no_node_warning`.** Tried all four
   combinations of `--from`/`--to` present-vs-absent and node reachable-vs-not
   (via a hand-built `bitcoin-cli` stub). Every combination renders sensibly;
   the absence warning and the no-node warning are mutually exclusive with
   their positive counterparts as expected. CLEAN.
4. **`legacy_unbound_warning`.** Tried a TXID-bound legacy input (silent,
   correct), a genuinely unbound one (warns, every clause derived from the
   actual source rather than asserted), an operator-supplied value on top of a
   `witness_utxo` record (correctly attributes to the record, which wins), and
   a multi-input transaction (fee arithmetic no longer saturates or
   contradicts the `FEE` row). CLEAN.
5. **`parse_btc` / `separator_guard` / `input_index_range_guard` /
   `check_input_value_indices`.** Tried legitimate amounts at both ends
   (`0.00000001`, `0`, `21000000` exactly accepted; `21000000.00000001`
   correctly refused one satoshi past max supply), a legitimate tab separator
   (round-trips through `mt verify` cleanly), and legitimate existing indices
   on a 2-input transaction (both accepted, fee computed). CLEAN.
6. **`read_strings::length_report`.** The hard case the fold commit message
   itself calls out — a legitimately short final chunk from an uneven
   payload — is asserted not to trigger a false length-mismatch refusal by the
   repo's own `a_legitimately_short_final_chunk_is_not_a_length_error`, which
   passes. CLEAN.
7. **`Refusal::with_verbatim`.** Read the `Display` impl (`refusal.rs:77-99`):
   `verbatim` is a purely additive, default-`None` field; when absent,
   rendering is byte-identical to before (mechanism + optional remedy, wrapped
   at 68 columns); when present, the verbatim block is appended after,
   line-by-line, un-reflowed. Every non-verbatim refusal reproduced during this
   review (§8.2f, §8.2c amount parsing, §8.2c index guards, §1.1e length
   check without the ranked list) rendered normally. CLEAN.
8. **`Report::build`'s provenance fallback.** Read the `Utxo::Unspent` arm
   (unchanged — still `ChainFetched`, still wins) alongside the new `Null`
   fallback. Confirmed via the same stub used for area 3 that a normal fully
   chain-verified LIVE input (unspent, correct value) still renders
   `TXID-BOUND`/chain-fetched correctly — the fallback chain only engages on
   the `Null` path, not the happy path. CLEAN.
9. **`check-refusal-coverage.sh` / `mutate-refusals.sh`.** The widened
   3-signal scan (named `refuses_*`, asserts a `REFUSED — §` message, or calls
   `assert_refused`) could plausibly false-positive on an ordinary test that
   happens to contain one of those signals for an unrelated reason. Read the
   script's own `EXEMPT` list (`check-refusal-coverage.sh:53-59`): it already
   carries two such cases with stated reasons —
   `ordinary_amounts_parse` (a control asserting a *legitimate* amount is
   **not** refused, which necessarily contains a `§`-refusal-shaped string to
   check its absence) and `refuses_beyond_the_budget` (an `mt-codec` BCH limit
   test that matches the naming convention by coincidence). Both are handled
   correctly, and no other declared test in the current suite trips the three
   signals without being either a genuine refusal test or one of these two
   exemptions. **Separately, and this is not a new defect but is worth
   restating precisely:** the widened scan can only ever catch an existing,
   undeclared test — it structurally cannot detect a refusal-shaped function
   with **zero** tests at all, which is exactly the shape of the still-open
   M-7 finding above (`require_psbt_input_values`). The script's own header
   already documents this limit ("checks the coupling, not the ruling"), so
   this is the gate working exactly as designed and scoped, not a gap the fold
   introduced or hid.
