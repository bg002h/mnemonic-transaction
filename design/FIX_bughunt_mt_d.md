# FIX-D: `inspect` node txindex detection (mt bug hunt #5, ruling R5)

Track: small fix. Inputs: triage `design/agent-reports/bughunt-triage-mt.md`
(row 5), grade and rulings `design/agent-reports/grade-bughunt-mt.md` (§1 row
5, §2 R5, §3 FIX-D). Branch `fix/bughunt-d`, from 3bfbc98.

**All three STOP points (§7) are ruled** (architect bughunt-fixdocs, QD1, QD2;
STOP-3 settled as (b)). Everything below is decided.

## 1. Finding

`Node::has_txindex` (`crates/mt-cli/src/node.rs:158`) is a substring test,
`s.contains("txindex") && s.contains("\"synced\": true")`, over the whole
`getindexinfo` reply. Any other synced index makes an unsynced (or absent)
txindex count as synced. `Report::build` (`report.rs:184`) uses it to decide
whether a parent that `getrawtransaction` cannot find means PENDING (with
txindex) or UNKNOWN (without), so a missing parent prints PENDING where §6a
requires UNKNOWN. The same predicate also wrongly reads compact JSON
(`"synced":true`, no space) as not synced, and trusts a truncated reply.

## 2. Ruling R5 (binding, quoted from the grade)

> `has_txindex` parses the `getindexinfo` reply as JSON. It returns true only
> when the object `txindex` exists and its `synced` is the boolean `true`.
> Anything else, including a parse failure, returns false, which gives UNKNOWN
> (the §6a fail-safe direction). No printed text changes.

Read as: the whole reply (after the existing `trim()` in `Node::call`) is one
JSON document; its top-level value is an object; that object has a member named
`txindex`; that member's value is an object; that object's own member `synced`
is the JSON literal `true`. Every other case returns false: no reply
(`call` returned `None`), a reply that is not one complete JSON document
(including trailing non-whitespace), a top-level array or scalar, no `txindex`
member, `txindex` not an object, no `synced` member, `synced` any value other
than the literal `true` (`false`, `"true"`, `1`, `null`), and a `synced: true`
nested deeper than txindex's own members. Duplicate member names take serde_json's last value; Core never emits them
(UniValue `pushKV` replaces) and a source that did could lie outright (ruled,
QD2; no rows).

The fix-05 scratch patch (`/scratch/cache/bughunt/mt/fix-05.patch`) is
rejected: it slices from `"txindex"` to the first `}` and still substring
matches, so it fails rows 05b, 05c and 05d below (executed, §4).

## 3. Spec, README and help delta

None. No printed text, flag, exit code or refusal changes; the PENDING and
UNKNOWN texts in `report.rs` are untouched (only a code comment there changes,
§6 step 2). The spec's liveness table (spec
lines 579-583) already says UNKNOWN when there is no usable `-txindex`; the
spec, README and `--help` never mention `getindexinfo`, so nothing there names
the old mechanism (grep: 0 hits in `design/SPEC_mt_v0_1.md` and `README.md`).
No toolkit-manual row (ruling O1: mt has no manual chapter). No refusal is
added, so `refusals.toml` and `scripts/check-refusal-coverage.sh` are unchanged.

## 4. RED rows

All rows are in `crates/mt-cli/tests/bughunt.rs` (its own test binary,
`mt-cli::bughunt`). Row 05 is the triage row from a5e5fd7, unchanged. Rows
05b-05g are new in this FIX's first commit; they share the helper
`inspect_with_indexinfo(reply)`, which runs `mt inspect --bitcoin-cli <stub>
--in <even vector strings>` against a stub whose `getindexinfo` prints `reply`
and whose `gettxout`/`getrawtransaction` exit 1, so every parent is NOT FOUND
and the STATUS row turns on `has_txindex` alone. Each asserts rc 0, that stdout
contains `STATUS    <want>`, and that it does not contain the other of
PENDING/UNKNOWN.

| row | `getindexinfo` reply | GREEN expects | unmodified code (3bfbc98 + rows) | fix-05 applied |
|---|---|---|---|---|
| `bughunt_mt_05_txindex_unsynced_other_index_synced_is_unknown` | `{"txindex": {"synced": false, ...}, "basic block filter index": {"synced": true, ...}}` | UNKNOWN | FAIL: "txindex is NOT synced yet a missing parent was reported PENDING" | PASS |
| `bughunt_mt_05b_txindex_compact_json_synced_is_pending` | `{"txindex":{"synced":true,"best_block_height":963832}}` | PENDING | FAIL: "a synced txindex in compact JSON was not recognised" | FAIL |
| `bughunt_mt_05c_txindex_truncated_reply_is_unknown` | `{"txindex": {"synced": true, "best_block_height": 963832}` (no closing brace) | UNKNOWN | FAIL: "a reply that is not valid JSON was trusted" | FAIL |
| `bughunt_mt_05d_txindex_nested_synced_true_is_unknown` | `{"txindex": {"synced": false, "best_block_height": 10, "detail": {"synced": true}}}` | UNKNOWN | FAIL: "a nested synced:true was read as txindex's own" | FAIL |
| `bughunt_mt_05e_txindex_absent_name_in_value_is_unknown` | `{"coinstatsindex": {"synced": true, ...}, "note": "txindex"}` | UNKNOWN | FAIL: "a reply with no txindex member was read as a synced txindex" | PASS |
| `bughunt_mt_05f_txindex_pretty_synced_true_is_pending` (guard) | `{"txindex": {"synced": true, "best_block_height": 963832}}` | PENDING | PASS (guard) | PASS |
| `bughunt_mt_05g_txindex_synced_string_is_unknown` (guard) | `{"txindex": {"synced": "true", "best_block_height": 963832}}` | UNKNOWN | PASS (guard) | PASS |

Command (run on unmodified production code, log
`/scratch/cache/bughunt/mt/fixd-red.log`):

    cargo nextest run --locked -p mt-cli --test bughunt --no-fail-fast -E 'test(/^bughunt_mt_05(|b|c|d|e|f|g)_/)'

Recorded result: `7 tests run: 2 passed, 5 failed`; the five failures are rows
05-05e, each at its STATUS assertion with the message in the table (not at the
rc assertion). The fix-05 column is from the same command with fix-05 applied
as a scratch change (log `fixd-fix05.log`): 4 passed, 3 failed.

No rows for duplicate members: ruled QD2, last-wins, none added (the former 05h and 05i are dropped).

## 5. Tests that change

None. Existing node-backed tests use `tests/common/mod.rs`'s stub, whose reply
is Core's pretty form `{"txindex": {"synced": true, "best_block_height":
963832}}` (row 05f's shape), and every one of them passes on the baseline
(§6). The new rows only add.

## 6. Build order

1. Commit 1 (RED, already on this branch): rows 05b-05g and their helper in
   `crates/mt-cli/tests/bughunt.rs`, with this document.
2. Commit 2 (GREEN): `crates/mt-cli/src/node.rs`, `crates/mt-cli/Cargo.toml`
   and the `report.rs` comment below (STOP-1 ruled option S, QD1):
   - `Cargo.toml`: move `serde_json = "1"` from `[dev-dependencies]` to
     `[dependencies]`. `Cargo.lock` is unchanged (serde_json 1.0.151 locked).
   - `report.rs:424-425` comment only: "Hand-rolled rather than derived:
     `serde` is not a dependency of this crate," becomes "Hand-rolled rather
     than derived: mt-cli takes `serde_json` only to read `getindexinfo`
     (node.rs), not serde's derive," (re-wrapped as `///` lines).
   - Replace the body of `has_txindex` with the R5 predicate of §2, built on
     `serde_json`: `from_str::<Value>(reply.trim())`, then
     `get("txindex").and_then(|t| t.get("synced")) == Some(&Value::Bool(true))`;
     a parse error is false. serde_json's recursion limit of 128 makes deeper
     nesting a parse failure. `call` stays as is.
   - Its doc comment gains one paragraph: the reply is parsed as JSON and only
     txindex's own boolean `synced: true` counts; anything else, a parse
     failure included, is "no index", which reads UNKNOWN (§6a's fail-safe
     direction), because another synced index once made an unsynced txindex
     read as synced (bug hunt #5).
   - No other file changes. `report.rs` changes only the comment above.
3. Run the gate (§8) once to a file, grep it, and quote the counts in the
   GREEN commit message.

FIX-D touches `node.rs`, `mt-cli/Cargo.toml` and one comment in `report.rs`.
Its base is the merged master after A-C (STOP-3 (b)); it expects 0 failures.

## 7. STOP points: all ruled

- **STOP-1 (dependency): ruled option S** (architect bughunt-fixdocs, QD1):
  promote `serde_json = "1"` to a runtime dependency, parse into
  `serde_json::Value`. Probe on 1.0.151: pretty or compact `synced:true` is
  true; trailing ` x`, a top-level array and `"true"` are false; 200-deep
  nesting fails to parse (limit 128). Option H (hand-rolled parser) is dropped.
  Rows 05-05g are the same under either option.
- **STOP-2 (duplicate member names): ruled** (QD2): serde_json's last value
  wins; no rows. Core cannot emit duplicates (UniValue `pushKV` replaces; one
  `SummaryToJSON` per index, each with a distinct name).
- **STOP-3 (base): ruled (b)** (QD2): D goes on master after A-C and expects
  0 failures; the (a) list of ten expected failures is dropped.

## 8. Gate

Toolchain: `$HOME/.cargo/bin` first in PATH; `cargo --version` must print
1.85.0 (`rust-toolchain.toml`). mt's phase-close list is its CI `test (rust)`
job (`.github/workflows/ci.yml`) plus the doc tests and the docs-only self-test
that `release.yml` runs. `trace-quote-check.py` and `vendor-freshness.sh` are
mnemonic-refugium scripts and do not exist in this repo. Run each once, output
to one file, then grep:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (needs `/usr/bin/zsh`,
   `/usr/bin/fish`, `/usr/bin/script`, `/usr/bin/timeout`; all present here)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh`
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh`
10. FIX-D rows by exact name:
    `cargo nextest run --locked -p mt-cli --test bughunt -E 'test(/^bughunt_mt_05(|b|c|d|e|f|g)_/)'`
    Expected: **7 tests run: 7 passed**. The filter's matched count was
    checked with `cargo nextest list` (7). Fewer than 7 is a FAIL.

Expected step-4 totals. Baseline (3bfbc98 + RED rows, log
`/scratch/cache/bughunt/mt/fixd-baseline-full.log`): `293 tests run: 278
passed, 15 failed` (the 5 FIX-D RED rows plus 10 owned by A, B and C). On the
merged master after A-C (STOP-3 (b)) the gate expects 0 failed; the total is
293 plus the rows A, B and C add.

Review: class B is not required (no PSBT or transaction code); one whole-diff
adversarial review of the GREEN diff, persisted per the repo rules.
