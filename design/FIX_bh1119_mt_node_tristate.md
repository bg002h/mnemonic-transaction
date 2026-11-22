# FIX: node answers are tri-state and parsed as JSON (bughunt-1119 FIX table row 2, "FIX-A node tri-state")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Keys: bh-mt-C1 (Critical),
bh-mt-I13, bh-mt-I14, bh-mt-I15, review r1 I-1, mt-median-time-substring (I). Rulings: architect T11
(`coordinator/agent-reports/architect-bughunt-1119-questions.md:49`); architect mt-node-tristate-i1 (`coordinator/agent-reports/architect-mt-node-tristate-i1.md`). Track: full.
Class B (funds path) per the grader (`grade-mt.md:85`: "funds-path code, class B"; the
FIX table row 2 does not repeat it). Base: origin/main `4d1461c`. Branch
`fix/mt-node-tristate`. Every holder-visible text is RULED (architect mt-node-tristate-i1).

Not to be confused with `design/FIX_bughunt_mt_a.md` (an older FIX-A, §8.6).

## 1. Findings (at 4d1461c)

- **C1.** `Node::call` (`crates/mt-cli/src/node.rs:66-86`) returns `None` on any
  non-zero exit; `Node::txout` (`node.rs:99-105`) maps `None`, and a reply whose
  `value` it cannot extract, to `Utxo::Null`. Core's `bitcoin-cli` prints nothing and
  exits 0 for a null result (`src/bitcoin-cli.cpp:984-988` `ParseResult`) and exits
  `abs(code)` for an RPC error (`:1009` `ParseError`), so a FAILED call is read as
  "spent or never existed". With confirmed parents, `inspect` prints `STATUS    DEAD
  — an input was spent by someone else. The engraving is scrap` (exit 0) for a live
  plate, and `encode` refuses with §8.5 "can never be broadcast"
  (`main.rs:585-593`). The test stub pins the wrong mapping
  (`tests/common/mod.rs:69-71`: empty gettxout = `exit 1`; `:64` unknown txid =
  `exit 1`).
- **I13.** `Node::is_confirmed` (`node.rs:178-195`) decides "confirmed" by
  substring: `"confirmations":  0` (two spaces) is read as confirmed, giving DEAD.
- **I14.** `Node::chain` (`node.rs:135-148`) knows no `testnet4`, so a testnet4
  node gets mainnet `bc1q…` addresses under `(addresses shown as MAINNET — no node
  to ask)` (`report.rs:357-362`), a note that blames a node that answered.
- **I15.** `extract_value_sats` (`node.rs:203-213`) reads `"value":5e-8` as
  500000000 sat (it stops at `e`, then multiplies 5 by 10^8), labelled as fetched.
- **I-1 (review r1).** `getrawtransaction` answers from a txindex that is still building (Core `node/transaction.cpp:132-143`), so a parent below the index frontier reads confirmed while THIS transaction, mined above it, exits 5. Every input is then null with a confirmed parent: `inspect` prints DEAD and `encode` refuses §8.5 ("Build a new transaction ..."), which pays twice. **mt-median-time-substring.** `Node::median_time` (`node.rs:113-121`) reads `"mediantime": 1.7e9` as 1 and prints `current MTP 1970-01-01T00:00Z`.

Core facts (checked in a Bitcoin Core checkout at `64a7c7cbb9`):
`bitcoin-cli.cpp:984-988, 1009`; `rpc/rawtransaction.cpp:70-83` (`confirmations` is
pushed only when the transaction has a block hash: `1 + tip - height` in the
active chain, `0` when the block is not in the active chain, absent for a mempool
transaction); `rpc/rawtransaction.cpp:353-370` (a transaction `getrawtransaction`
cannot find throws `RPC_INVALID_ADDRESS_OR_KEY` = -5, `rpc/protocol.h:42`, in all
three txindex situations, so `bitcoin-cli` exits 5); `util/chaintype.cpp:15-23`
(chain names `main`, `test`, `testnet4`, `signet`, `regtest`). rust-bitcoin 0.32.102
has `Network::Testnet4` (`network.rs:82`), displayed `testnet4` (`:204`). `rpc/node.cpp:348-358, 390-392` (`getindexinfo`: `{"txindex": {"synced": b, "best_block_height": h}}`, `{}` without `-txindex`); `index/base.cpp:129, 219` (`synced` is set once and never cleared); `rpc/blockchain.cpp:1392` (`mediantime` is an integer).

## 2. Ruling T11 (binding, quoted)

> node.rs@4d1461c:101-104 maps both a failed call and an unparseable answer to
> `Utxo::Null`; add an indeterminate variant for both arms and parse JSON, not
> substrings. An indeterminate answer takes the path mt already has for "no node"
> (main.rs:670 prints `encode_no_node_warning` and proceeds), so `encode` warns and
> proceeds, and the status verdict reads indeterminate, never DEAD. The `node_stub`
> "exit 1 = not found" convention (tests/common/mod.rs:64-70) moves to Core's real
> mapping. Text via WORDING_REVIEW.

## 3. Behaviour after the fix (rules)

Terms. A call **answers** when `bitcoin-cli -stdin` runs and exits 0; its reply is
stdout, lossily decoded as UTF-8 and trimmed (as today). A call **fails** when it
cannot be spawned, its stdin cannot be written, it cannot be waited on, it exits
non-zero, or it is killed by a signal. The fail exit code is kept when there is one.
"JSON" means one complete `serde_json::Value` document parsed from the whole
trimmed reply (trailing non-whitespace is a parse failure; serde_json's recursion
limit of 128 applies). **Unanswered** is the new third outcome: no usable answer.

- **N1 `gettxout` (`Node::txout`).** Fails -> `Unanswered`. Answers with an empty
  reply -> `Null` (Core's null). Answers with the JSON literal `null` -> `Null`.
  Answers with a JSON object whose member `value` is an **amount** -> `Unspent(sats)`.
  Anything else -> `Unanswered`: not JSON, not an object, no `value`, `value` not a
  JSON number (a string `"3.0"` included), or a number that is not an amount.
- **N2 amount.** A JSON number is an amount when it is not negative (a leading minus
  sign, `-0` included, is never an amount), has at most 8 decimal places, and is at
  most 21,000,000 BTC (`bitcoin::Amount::MAX_MONEY`). It converts to satoshis
  exactly: `5e-8` is 5, `3.00000000` is 300000000, `0.000000051` and
  `21000000.00000001` are not amounts. (Author's check: serde_json's default float
  parse is exact for every amount, since an amount has at most 16 significant
  digits and a decimal exponent of at least -8; `bitcoin::Amount::from_btc` then
  refuses a negative value and a value with more than 8 decimals. The builder may
  use any exact method; the rows decide.)
- **N3 `getrawtransaction <txid> true` (`Node::is_confirmed`).** Fails with exit
  code 5 -> `NotFound` (Core's not-found). Fails any other way -> `Unanswered`.
  Answers with a JSON object: no member `confirmations` -> `InMempool` (Core's
  mempool shape); `confirmations` a JSON non-negative integer: 0 -> `InMempool`
  (Core: block not in the active chain; found is not confirmed), 1 or more ->
  `Confirmed`; `confirmations` anything else (string, float, negative, null,
  boolean, array, object) -> `Unanswered`. Any other answer (empty, not JSON, not an
  object) -> `Unanswered`.
- **N4 `getblockchaininfo` chain (`Node::chain`).** Answers with a JSON object
  whose member `chain` is a string: `main` -> Bitcoin, `test` -> Testnet,
  `testnet4` -> Testnet4, `signet` -> Signet, `regtest` -> Regtest, any other string
  -> **unrecognised chain**. Fails, not JSON, not an object, no `chain`, or `chain`
  not a string -> **chain not said**. The match is exact and case-sensitive
  (Core prints lower case).
- **N5 unchanged calls.** `Node::find` (a node exists when `getblockcount` answers),
  `block_count` and `has_txindex` (FIX-D R5) keep their behaviour; `has_txindex` is asked once, BEFORE this transaction's own `getrawtransaction` (N6, N9).
- **N5b `median_time`.** Answers with a JSON object whose member `mediantime` is a JSON non-negative integer -> that value. Anything else (failed call, not JSON, not an object, no member, a float such as `1.7e9`, negative, string) -> no MTP (LOCKTIME prints its existing `current MTP unknown (no node)`).
- **N6 report status (`Report::build`, used by `inspect`, `decode` and `encode`'s
  report).** Evaluated in this order; the first that holds wins: `txindex` means `has_txindex` was true when asked, and it is asked before item 1's question (Core's `synced` never goes back to false, so a read before the own question covers it; a read after does not).
  1. this transaction's own `is_confirmed` is `Confirmed` -> `SPENT — ALREADY
     CONFIRMED` (unchanged);
  2. no node -> `UNKNOWN — no node reachable` (unchanged);
  3. this transaction's own `is_confirmed` is `Unanswered` -> **unanswered**
     (new): without the question asked first, DEAD's evidence is exactly the
     success case's shape;
  4. any input `Null` with parent `Confirmed`, and `txindex` -> `DEAD` (that input alone proves it: a synced index answered "not found" for this transaction);
  5. any input `Unanswered`, or any input `Null` with parent `Unanswered` ->
     **unanswered** (new);
  5a. any input `Null` with parent `Confirmed`, without `txindex` -> **index-not-synced** (new, W5): own `NotFound` from an index that is not synced does not rule out "already mined";
  6. any input `Null` with parent `NotFound` and no synced txindex -> the existing
     txindex `UNKNOWN` text (unchanged);
  7. any input `Null` with parent `InMempool`, or `NotFound` with a synced txindex
     -> `PENDING` (unchanged);
  8. all values known -> `LIVE`; otherwise the existing txindex `UNKNOWN` text
     (unchanged).
  The parent of an `Unanswered` input is not asked. An `Unanswered` input's value
  is taken as on the offline path: the caller's claimed record (`claimed`) with its
  own provenance if there is one, else `UNKNOWN`. Order 5 before 5a before 6 is RULED (the failed call is the one a re-run in seconds fixes; 5a's parent is confirmed, 6's is not found).
- **N7 JSON.** `inspect --json` (and every `--json` that renders a `Report`) prints
  `"status": "Indeterminate"` for **unanswered** and for **index-not-synced** (T11: "reads indeterminate"). The
  other status strings are unchanged (`AlreadyConfirmed`, `Live`, `Dead`,
  `Pending`, `Unknown`, `Indeterminate`); the new variants' Rust names must not leak
  through `{:?}`. An `Unanswered` input with no claimed record has `"sats": null,
  "provenance": "Unknown", "verified": false` (as offline).
- **N8 OUT row.** With a node: a known chain renders as today (`(testnet4)` for
  Testnet4, through `Network`'s Display); an unrecognised chain and a chain not
  said get the two new texts of §4 (addresses still rendered for mainnet, as with
  no node). With no node: unchanged `(addresses shown as MAINNET — no node to ask)`.
- **N9 `encode` (`main.rs:563-596`, `669-671`).** 
  - Before the own question: `has_txindex`, once (N5).
  - Own `is_confirmed` `Confirmed`: unchanged (no per-input node check).
  - Own `is_confirmed` `Unanswered`: no per-input node check at all (no chain
    value, no §6a value comparison, no §8.5), exactly as with no node; the run is
    **node-unanswered**.
  - Per input: `Unspent` unchanged (chain value, §6a mismatch refusal). `Null`:
    parent `Confirmed` with `txindex` refuses §8.5 (unchanged text and exit 1); parent `Confirmed` without `txindex` proceeds and the run is **index-not-synced**; parent `InMempool`
    or `NotFound` proceeds silently (unchanged); parent `Unanswered` proceeds and the
    run is node-unanswered. `Unanswered`: the value stays as claimed (or unknown),
    no guard runs, the run is node-unanswered.
  - At the place the no-node warning prints (`main.rs:669-671`): no node -> the
    existing no-node warning (unchanged); a node and a node-unanswered run -> the
    new warning of §4, once, on stderr; otherwise nothing. Like the no-node
    warning, it is printed with or without `--quiet`. An index-not-synced run then prints W6 once on stderr (after W2 when both apply), with or without `--quiet`.
  - The exit code is unchanged by an unanswered answer: encode proceeds to its
    normal outcome (0 on success; any other refusal it reaches still refuses).
  - A §8.5 refusal on one input still refuses even if another input is
    `Unanswered` (the loop order is unchanged; the first refusal wins).
- **N10 exit codes.** No exit code changes anywhere: `inspect` and `decode` exit 0
  on every status, as today; `encode` per N9.

## 4. Holder-visible texts

All new texts are **RULED** (architect mt-node-tristate-i1); each goes to the master
wording list (`mnemonic-refugium` `design/WORDING_REVIEW.md`) as a row after merge
(listed in the author's report, not appended here).

- **W1 STATUS row, unanswered** (`report.rs` `Status::render`; same layout as the
  existing txindex text, continuation lines indented by 27 spaces). Exactly, as
  `inspect` prints it:

      STATUS    UNKNOWN — the node did not answer every question: a call failed or
                                 its reply could not be read. mt will not guess DEAD from a missing
                                 answer. Check the node, then run mt again

  (Rust source text: `"UNKNOWN — the node did not answer every question: a call
  failed or\n` + 27 spaces + `its reply could not be read. mt will not guess DEAD
  from a missing\n` + 27 spaces + `answer. Check the node, then run mt again"`.)
- **W2 encode warning** (new `blocks.rs` function beside `encode_no_node_warning`,
  rendered by `refusal::Warning`'s Display, body wrapped at 68). Headline:
  `the node did not answer every question — mt could not check every input before you cut.`
  Body source text (`\n` are line breaks, the two list items begin with a space as
  in `encode_no_node_warning`):
  `A bitcoind was reachable, but at least one call failed or returned a reply mt
  could not read, so for at least one input mt has NOT answered:\n\n - is it still
  unspent, or did something else take it?\n - what is its value, and so what fee
  does this actually pay?\n\nEngraving takes about 21 minutes per plate and is
  permanent. Check the node (one that is still starting up refuses calls), then run
  mt again; that takes seconds. If an input turns out to be spent, the plate is
  scrap the moment it leaves the machine.`
  Rendered (author's scratch build):

      WARNING: the node did not answer every question — mt could not check every input before you cut.

        A bitcoind was reachable, but at least one call failed or returned a
        reply mt could not read, so for at least one input mt has NOT
        answered:
        
        - is it still unspent, or did something else take it?
        - what is its value, and so what fee does this actually pay?
        
        Engraving takes about 21 minutes per plate and is permanent. Check
        the node (one that is still starting up refuses calls), then run mt
        again; that takes seconds. If an input turns out to be spent, the
        plate is scrap the moment it leaves the machine.

- **W3 OUT row, unrecognised chain:**
  `OUT       {n} output(s)   (addresses shown as MAINNET — the node named a chain mt does not know)`
- **W4 OUT row, chain not said:**
  `OUT       {n} output(s)   (addresses shown as MAINNET — the node did not say which chain it is on)`
- **W5 STATUS row, index not synced** (`report.rs` `Status::render`; same layout
  as W1, continuation lines indented by 27 spaces). Exactly, as `inspect` prints
  it:

      STATUS    UNKNOWN — an input is not in the UTXO set, its parent confirmed, and this
                                 node has not reported its -txindex synced: mt cannot tell whether this
                                 transaction is already mined or someone else spent the input. mt will
                                 not guess DEAD. Run mt again once getindexinfo shows the txindex synced

  (Rust source text: `"UNKNOWN — an input is not in the UTXO set, its parent
  confirmed, and this\n` + 27 spaces + `node has not reported its -txindex synced:
  mt cannot tell whether this\n` + 27 spaces + `transaction is already mined or
  someone else spent the input. mt will\n` + 27 spaces + `not guess DEAD. Run mt
  again once getindexinfo shows the txindex synced"`. True in all four non-synced
  states (it says "not reported", not "building"); names an RPC, prints no command
  line (B8).)
- **W6 encode warning** (new `blocks.rs` function beside `encode_no_node_warning`,
  rendered by `refusal::Warning`'s Display, body wrapped at 68). Headline:
  `the node has not reported its -txindex synced — mt cannot tell whether this transaction is already mined.`
  Body source text (`\n` are line breaks):
  `An input is not in the UTXO set and its parent is confirmed. Either this transaction is already in a block (it spent the input itself, and there is nothing left to engrave), or someone else spent the input (the plate would be scrap). Only a synced -txindex tells the two apart, and this node has not reported one.\n\nDo not build a replacement transaction on this answer: if this one is already mined, a replacement pays twice. Engraving takes about 21 minutes per plate and is permanent. Run mt again once getindexinfo shows the txindex synced.`
  Rendered (architect's scratch build; stub: index building, own exit 5, parents
  confirmed):

      WARNING: the node has not reported its -txindex synced — mt cannot tell whether this transaction is already mined.

        An input is not in the UTXO set and its parent is confirmed. Either
        this transaction is already in a block (it spent the input itself,
        and there is nothing left to engrave), or someone else spent the
        input (the plate would be scrap). Only a synced -txindex tells the
        two apart, and this node has not reported one.
        
        Do not build a replacement transaction on this answer: if this one
        is already mined, a replacement pays twice. Engraving takes about 21
        minutes per plate and is permanent. Run mt again once getindexinfo
        shows the txindex synced.
- Not new: `(testnet4)` on the OUT row comes from the existing
  `OUT {n} output(s)   ({network})` form; JSON `"Indeterminate"` is an existing
  value.

Unchanged and asserted unchanged: the DEAD, PENDING, LIVE, ALREADY CONFIRMED, no
node and txindex UNKNOWN texts; the §8.5 and §6a refusals; the no-node warnings.

## 5. Spec, README, help and manual delta

- **Spec (`design/SPEC_mt_v0_1.md`, the mt copy; precedent Q0 of
  `FIX_bughunt_mt_a.md` §7 STOP 2: edit the mt copy, the engrave sync is owed by
  engrave F-321; no PROVENANCE edit).** In §6a's liveness table, after the row whose first cell is `**UNKNOWN**` and
  second cell is "`null`, and no `-txindex`" (line 580), add this row (verbatim,
  one line, indented three spaces like its neighbours):

         | **UNKNOWN** | the node gave **no usable answer**: a `gettxout` call failed or its reply was neither null nor an object whose `value` is an amount; or a `getrawtransaction` call failed with anything but Core's not-found (exit 5), or its reply is not an object whose `confirmations`, if present, is a non-negative integer — for an input's parent, or for the transaction itself, which is asked first | `mt` has no answer and says so, and never prints DEAD on it. A DEAD proved by another input still stands. `encode` warns and proceeds, as with no node |

  and directly after that row add this row (verbatim, one line, indented three
  spaces like its neighbours):

         | **UNKNOWN** | `null`, the parent is **CONFIRMED**, and the node has **not reported a synced `-txindex`** (building, absent, or `getindexinfo` gave no answer) | `mt` cannot tell "already mined" (a building index answers "not found" for a transaction above its frontier) from DEAD, and says so; never DEAD, never §8.5's "build a new transaction". `encode` warns and proceeds |

  and after the paragraph ending "never printing DEAD on evidence that cannot
  distinguish it from PENDING." (lines 582-587) add this paragraph (verbatim, one
  line, indented three spaces like its neighbours):

         **A failed call is not a null (bughunt-1119 C1).** `bitcoin-cli` prints nothing and exits 0 for a null result, and exits with the error code's absolute value for an RPC error, so "empty, exit 0" is `gettxout`'s "not in the UTXO set" and a non-zero exit is no answer at all. Replies are parsed as JSON, never by substring; a value is an amount only when it is non-negative, has at most 8 decimals and is at most 21,000,000 BTC. `getindexinfo` is asked before this transaction's own lookup; a `mediantime` is read only as a JSON integer.
- **README:** none (grep of `README.md` for UNKNOWN, MAINNET, testnet, gettxout,
  "no node": 0 hits on the texts that change).
- **`--help`:** none (no flag, no help text names these states).
- **Toolkit manual:** none. mt has no chapter in
  `mnemonic-toolkit` `docs/manual/src/40-cli-reference/` (checked at origin/master:
  no mt file; ruling O1 of FIX-D).
- **refusals.toml / check-refusal-coverage:** no refusal added or changed.

## 6. RED rows

All 34 rows are in a new test binary `crates/mt-cli/tests/node_tristate.rs`
(`mt-cli::node_tristate`). Its exact text is the sidecar
`design/FIX_bh1119_mt_node_tristate/node_tristate.rs.txt` (sha256
`1d6c6bdfab0f9db85496decceb83e2c8ebe03a30dc590f2614f2bce374583786`); the builder's
RED commit copies it to that path unchanged. Every row drives `mt inspect` or
`mt encode` on `tests/fixtures/p5_base.json`'s `raw_hex` (two inputs, one output,
regtest) against a scripted `bitcoin-cli` whose defaults are: `getblockcount`
963832, a synced txindex, `{"chain": "regtest", ...}`, `getrawtransaction` exit 5,
anything else exit 1; each row overrides verbs. `STATUS_UNANSWERED` is W1 and
`ENCODE_UNANSWERED_HEADLINE` is `WARNING: ` + W2's headline, so a wording change
updates those two constants.

Command (run by the author on 4d1461c with the rows and §7's test changes; log
`/scratch/cache/mt-node-tristate-red-full.log`):

    cargo nextest run --locked --build-jobs 6 -j 6 --workspace --no-fail-fast

Recorded: `374 tests run: 348 passed, 26 failed`; the 26 failures are exactly the
RED rows below, each at the assertion named. The 8 guards pass.

| row | stub (overrides) | GREEN expects | at 4d1461c |
|---|---|---|---|
| `bh1119_mt_c1_null_gettxout_confirmed_parent_still_dead` (guard) | gettxout exit 0; parents `confirmations: 6` | DEAD | PASS |
| `bh1119_mt_c1_failed_gettxout_inspect_is_not_dead` | gettxout exit 28 (stderr `error code: -28`); parents confirmed | rc 0; no DEAD, no "scrap"; stdout ends with W1 | FAIL: "reported DEAD" |
| `bh1119_mt_c1_failed_gettxout_inspect_json_is_indeterminate` | gettxout exit 1; parents confirmed; `--json` | `"status": "Indeterminate"` | FAIL: `"Dead"` |
| `bh1119_mt_c1_failed_gettxout_encode_warns_and_proceeds` | gettxout exit 1; parents confirmed; `encode` | rc 0, strings on stdout, no REFUSED, W2 headline, no "no bitcoind reachable" | FAIL: rc 1, §8.5 REFUSED |
| `bh1119_mt_c1_unreadable_gettxout_reply_is_not_dead` | gettxout `{"bestblock": "00", "confirmations": 3}`; parents confirmed | W1 | FAIL: "reported DEAD" |
| `bh1119_mt_c1_unreadable_parent_reply_is_not_dead` | gettxout exit 0; parents `{"txid": "00", "confirmations": "6"}` | W1 | FAIL: "reported DEAD" |
| `bh1119_mt_c1_unreadable_own_tx_reply_is_not_dead` | gettxout exit 0; own tx `not json`; parents confirmed | W1 | FAIL: "reported DEAD" |
| `bh1119_mt_c1_unreadable_own_tx_reply_encode_warns_and_proceeds` | as above; `encode` | rc 0, no "can never be broadcast", strings, W2 headline | FAIL: rc 1, §8.5 REFUSED |
| `bh1119_mt_c1_failed_parent_lookup_is_unanswered_not_pending` | gettxout exit 0; getrawtransaction exit 1 | not PENDING; W1 | FAIL: "a failed lookup read as PENDING" |
| `bh1119_mt_c1_not_found_parent_with_txindex_still_pending` (guard) | gettxout exit 0 (parents exit 5) | PENDING | PASS |
| `bh1119_mt_c1_dead_input_outranks_unanswered_input` (guard, N6 order) | gettxout exit 0 for input 0's txid, exit 1 for input 1; parents confirmed | DEAD | PASS |
| `bh1119_mt_i13_two_space_zero_confirmations_is_pending` | gettxout exit 0; parents `"confirmations":  0` | PENDING | FAIL: DEAD |
| `bh1119_mt_i13_encode_two_space_zero_is_not_refused` | as above; `encode` | rc 0, no REFUSED | FAIL: rc 1, §8.5 REFUSED |
| `bh1119_mt_i13_absent_confirmations_is_pending` (guard) | parents `{"txid": "00"}` | PENDING | PASS |
| `bh1119_mt_i13_compact_confirmed_is_dead` (guard) | parents `{"txid":"00","confirmations":6}` | DEAD | PASS |
| `bh1119_mt_i14_testnet4_node_prints_testnet4_addresses` | chain `testnet4` | `OUT       1 output(s)   (testnet4)`, `tb1q`, no `bc1q` | FAIL: "no node to ask", bc1q |
| `bh1119_mt_i14_unknown_chain_is_named_as_such` | chain `futurenet` | W3 with n = 1 | FAIL: "no node to ask" |
| `bh1119_mt_i14_chainless_reply_is_named_as_such` | getblockchaininfo exit 1 | W4 with n = 1 | FAIL: "no node to ask" |
| `bh1119_mt_i15_scientific_value_is_five_sats` | gettxout `{"value":5e-8,...}`; `--json` | input sats `[5, 5]` | FAIL: `[500000000, 500000000]` |
| `bh1119_mt_i15_overprecise_value_is_unanswered` | `{"value":0.000000051,...}`; `--json` | `"Indeterminate"`, sats `[null, null]` | FAIL: `"Live"` |
| `bh1119_mt_i15_negative_value_is_unanswered` | `{"value":-1,...}` then `-0` | W1 (both) | FAIL: PENDING |
| `bh1119_mt_i15_value_above_money_range_is_unanswered` | `{"value":21000000.00000001,...}` | W1 | FAIL: LIVE |
| `bh1119_mt_i15_json_null_reply_is_null` (guard) | gettxout prints `null`; parents confirmed | DEAD | PASS |
| `bh1119_mt_i15_core_decimal_value_is_exact` (guard) | `{"value": 3.00000000, ...}`; `--json` | sats `[300000000, 300000000]` | PASS |
| `bh1119_mt_c1_unsynced_txindex_own_not_found_is_not_dead` | getindexinfo synced false; gettxout exit 0; parents `confirmations: 6`; own exit 5 | rc 0, no DEAD/scrap, ends with W5 | FAIL: "txindex building: reported DEAD" |
| `bh1119_mt_c1_unsynced_txindex_own_not_found_json_is_indeterminate` | same, `--json` | `"Indeterminate"` | FAIL: `"Dead"` |
| `bh1119_mt_c1_unsynced_txindex_own_not_found_encode_is_not_never_broadcast` | same, `encode` | rc 0, no REFUSED, strings, W6 headline + "Do not build a replacement transaction" | FAIL: "encode refused: ... REFUSED — §8.5" |
| `bh1119_mt_c1_unreported_txindex_own_not_found_is_not_dead` | getindexinfo exit 89, then `{}`; `inspect` + `encode` | W5; encode as above | FAIL: "getindexinfo failed: reported DEAD" |
| `bh1119_mt_c1_txindex_state_read_before_own_question_inspect` | getindexinfo synced only after own txid asked | W5 | FAIL: "reported DEAD" |
| `bh1119_mt_c1_txindex_state_read_before_own_question_encode` | same, `encode` | as encode above | FAIL: "REFUSED — §8.5" |
| `bh1119_mt_c1_unanswered_outranks_txindex_unknown` | building; input 0 gettxout exit 1; parent of input 1 exit 5 | W1 | FAIL: "STATUS row is not the unanswered text" |
| `bh1119_mt_c1_unanswered_outranks_index_not_synced` | building; input 0 exit 1; input 1 null, parents confirmed | W1 | FAIL: "reported DEAD" |
| `bh1119_mt_median_time_exponent_reply_is_not_1970` | getblockchaininfo `mediantime` 1.7e9; nLockTime 1800000000 | `LOCKED UNTIL 2027-`, no 1970 | FAIL: "current MTP 1970-01-01T00:00Z" |
| `bh1119_mt_median_time_integer_reply_is_read` (guard) | `mediantime` 1700000000 | `current MTP 2023-11-14T22:13Z` | PASS |

`cargo nextest list --locked -p mt-cli --test node_tristate` lists 34.

## 7. Tests that change

Exact diff: sidecar `design/FIX_bh1119_mt_node_tristate/test-changes.diff` (sha256
`41274f12f97408dc4a69a42114be9a8700ae1692ebf5b495a7d0c78a549da9d8`), applied in
the RED commit. It moves every stub to Core's real mapping (T11):

- `tests/common/mod.rs` `node_stub`: an empty `gettxout` argument becomes `exit 0`
  (no output: Core's null) instead of `exit 1`; an unlisted `getrawtransaction`
  txid exits 5 instead of 1; the doc comment says so.
- `tests/bughunt.rs` row 05's script and `inspect_with_indexinfo`'s script: add
  `gettxout) exit 0` and `getrawtransaction) exit 5` arms before `*) exit 1`; the
  section comment (lines 597-600) says so.

Measured: with the change alone on 4d1461c, every existing test still passes (the
mapping is behaviour-neutral there: both old and new stubs read as Null/NotFound).
With the author's scratch fix and WITHOUT the change, these 9 existing tests fail,
which is why it is required: `inspect::reaching_a_node_changes_the_report`,
`inspect::a_reachable_node_answers_the_rows_that_were_unknown`,
`inspect::inspect_reports_a_dead_plate_rather_than_refusing`,
`inspect::inspect_distinguishes_pending_from_dead`,
`refusals::refuses_a_value_that_disagrees_with_the_chain`,
`refusals::refuses_a_spent_input_whose_parent_confirmed`,
`refusals::a_node_that_cannot_find_an_outpoint_does_not_discard_the_psbt_record`,
`bughunt::bughunt_mt_05b_txindex_compact_json_synced_is_pending`,
`bughunt::bughunt_mt_05f_txindex_pretty_synced_true_is_pending`.

`node.rs`'s unit tests `parses_a_value_into_satoshis`, `parses_a_whole_btc_value`
and `absent_value_is_none` keep their inputs and expected values; only their call
shape follows whatever signature the builder gives the value parser.
`design/FIX_bughunt_mt_d.md:60-63` describes the old stub; it is a closed
historical document and is not edited.

## 8. Build order

1. **RED commit:** this document and its sidecars (already on this branch), plus
   `crates/mt-cli/tests/node_tristate.rs` copied from the sidecar and
   `test-changes.diff` applied. Expect §6's `348 passed, 26 failed`.
2. **GREEN commit:** `crates/mt-cli/src/node.rs` (N1-N4: a call outcome that keeps
   the exit code; `Utxo` and `ParentState` gain the unanswered variant; a chain
   reply with known / unrecognised / not said; JSON parsing per N1-N4; doc comments
   updated, including `Status`'s "SIX variants" count comment in `report.rs`, which
   becomes seven), `report.rs` (N6-N8, W1, W3, W4), `main.rs` (N9),
   `blocks.rs` (W2), and §5's spec text. No `Cargo.toml` change (serde_json is
   already a dependency; bitcoin 0.32.102 has `Testnet4`). Expect 0 failures.
3. The builder lists the §12 lines in its report; nothing is appended to
   `design/FOLLOWUPS.md` in the PR.

Author's scratch evidence (not normative; production code on this branch is
untouched): a scratch implementation of N1-N10 with the test changes gave
`364 tests run: 364 passed` and a clean `cargo clippy --all-targets --locked -- -D
warnings` (logs `/scratch/cache/mt-node-tristate-green-full.log`,
`/scratch/cache/mt-nt-clippy-green.log`; diff
`/scratch/cache/mt-node-tristate-scratch-fix.diff`).

## 9. Coverage map (rule -> rows; the `bh1119_mt_` prefix is omitted)

| rule | rows |
|---|---|
| N1 failed gettxout | c1_failed_gettxout_inspect_is_not_dead, c1_failed_gettxout_inspect_json_is_indeterminate, c1_failed_gettxout_encode_warns_and_proceeds |
| N1 empty / `null` = Null | c1_null_gettxout_confirmed_parent_still_dead, i15_json_null_reply_is_null |
| N1 unreadable reply | c1_unreadable_gettxout_reply_is_not_dead |
| N2 amount exact / not an amount | i15_scientific_value_is_five_sats, i15_core_decimal_value_is_exact, i15_overprecise_value_is_unanswered, i15_negative_value_is_unanswered, i15_value_above_money_range_is_unanswered |
| N3 exit 5 / other failure | c1_not_found_parent_with_txindex_still_pending, c1_failed_parent_lookup_is_unanswered_not_pending |
| N3 confirmations parse | i13_two_space_zero_confirmations_is_pending, i13_absent_confirmations_is_pending, i13_compact_confirmed_is_dead, c1_unreadable_parent_reply_is_not_dead |
| N4 chain | i14_testnet4_node_prints_testnet4_addresses, i14_unknown_chain_is_named_as_such, i14_chainless_reply_is_named_as_such |
| N6 order 3 (own unanswered) | c1_unreadable_own_tx_reply_is_not_dead |
| N6 order 4 before 5 | c1_dead_input_outranks_unanswered_input |
| N6 order 5 before 6, 5 before 5a | c1_unanswered_outranks_txindex_unknown, c1_unanswered_outranks_index_not_synced |
| N6 item 4/5a, N9 (I-1) | c1_unsynced_txindex_own_not_found_is_not_dead, c1_unsynced_txindex_own_not_found_json_is_indeterminate, c1_unsynced_txindex_own_not_found_encode_is_not_never_broadcast, c1_unreported_txindex_own_not_found_is_not_dead |
| N5/N9 getindexinfo before own question | c1_txindex_state_read_before_own_question_inspect, c1_txindex_state_read_before_own_question_encode |
| N5b median_time | median_time_exponent_reply_is_not_1970, median_time_integer_reply_is_read |
| N7 JSON | c1_failed_gettxout_inspect_json_is_indeterminate, i15_overprecise_value_is_unanswered |
| N8 OUT row | the three i14 rows |
| N9 encode | c1_failed_gettxout_encode_warns_and_proceeds, c1_unreadable_own_tx_reply_encode_warns_and_proceeds, i13_encode_two_space_zero_is_not_refused; existing refusals rows keep §8.5 and §6a |
| N10 exit codes | every row asserts rc |
| W1-W6 | the rows above assert each text exactly (W2 by headline) |

## 10. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
Same list as `FIX_bughunt_mt_d.md` §8:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expect 374 run, 0 failed)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh`
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh`, `bash ci/code-docs-check.sh` and `bash ci/code-docs-check.test.sh` (M-2, release.yml:119)
10. Rows by name: `cargo nextest run --locked -p mt-cli --test node_tristate`
    -> `34 tests run: 34 passed`. Fewer than 34 is a FAIL.

Review: class B (funds path) per the grader; up to three rounds (full track).

## 11. Open points for the architect (no STOP)

None: W1-W6, N7 and the N6 order are RULED (architect mt-node-tristate-i1); class B stands.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

One line owed to this build (closed by its GREEN commit), covering C1, I13, I14, I15, review r1 I-1 and mt-median-time-substring; and the Nit/Minor lines of the architect report.
