# Whole-diff review: mt FIX-D (e532b7f8 vs 9b961bed), single slice

Reviewer: Sonnet, effort medium. Re-review: not applicable (first whole-diff slice; FIX-review fold commits out of scope).

**Verdict: ruling R5 is implemented as the FIX doc states. No Critical or Important.**

## Checks
- Predicate (node.rs:163-170): `from_str::<Value>(s.trim())` then `get("txindex").and_then(|t| t.get("synced")) == Some(&Bool(true))`. Walked each R5 input:
  - `call` None (spawn fail, nonzero exit, RPC failure): `is_some_and` gives false, UNKNOWN.
  - Empty or truncated reply, trailing non-whitespace: `from_str` errors, false.
  - Top-level array or scalar: `Value::get("txindex")` returns None, false.
  - `txindex` not an object (string, array, null): `.get("synced")` returns None, false.
  - `synced` of `false`, `"true"`, `1`, `null`: unequal to `Bool(true)`, false.
  - Nested deeper `synced: true`: only the direct member is read, false.
  - Compact and pretty `synced:true`: true.
  - Real Core reply (`{"txindex": {"synced": true, "best_block_height": N}}`, `{}` when no index): true / false.
  No input maps to PENDING that R5 says is UNKNOWN. Fail-safe direction holds.
- Plant-back: copied the worktree to /scratch/cache/bughunt/wd-d/p2, restored the old substring body, ran `cargo nextest run --locked -p mt-cli -E 'test(/bughunt_mt_05/)'`: 7 run, 5 failed (05, 05b, 05c, 05d, 05e), 2 passed (05f, 05g guards). Each failure message names the stated reason (compact JSON unrecognised, truncated reply trusted, nested synced read as own, name-in-value read as index, other index synced). Rows 05b-05d fail as the FIX doc says.
- Cargo.toml: serde_json moved from dev-deps to deps, `--locked` build fine. report.rs change is comment only and accurate.
- Row helper `inspect_with_indexinfo` makes `gettxout`/`getrawtransaction` fail, so STATUS depends on `has_txindex` alone; `assert_status` checks both the wanted and the other status, so a row cannot pass vacuously.

## Mutation reach (3 mutants generated, all caught)
Not reached by any mutant but each fails closed or is already filed:
- RPC-failure branch (`call` None -> false): no row has a failing `getindexinfo`; an inverted `is_some_and` would not be generated as a mutant here. Covered by `fixd-r5-unrowed-cases` (M2).
- JSON shapes with no row: trailing garbage, top-level array/scalar, `txindex` non-object, `synced` of 1/null/false-literal-only-via-05. Same M2.
- `trim()` redundancy: N1 (filed).
None is a wrong-result path today; all are only guarded by serde_json semantics.

## Findings
None new. (Minor: rows for the RPC-failure and top-level-array cases are the cheapest additions under M2; no new entry needed.)

C / I / M / N: 0 / 0 / 0 / 0
