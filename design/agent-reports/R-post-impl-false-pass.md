# Mechanical false-PASS hunt — post-implementation review

**Scope.** `crates/mt-cli/tests/*.rs`, `crates/mt-codec/tests/*.rs`, inline
`#[cfg(test)]` modules, `scripts/mutate-refusals.sh`,
`scripts/check-refusal-coverage.sh`, `scripts/journeys.sh`,
`.github/workflows/ci.yml`. Independent reviewer; did not author this code.

**Method.** Every finding below was produced by copying the repo to
`/tmp/fp-review`, mutating exactly one thing, rebuilding, and running the
specific test or gate. A finding is reported only where the mutated build
compiled and ran, and the broken behavior was actually exercised.

## Verdict

**2 Critical / 1 Important / 0 Minor.** Two of the three CI gates
(`journeys.sh`, and the test suite as a whole with respect to `mt inspect`'s
node integration) can report full green while the artifact or code path they
claim to validate is silently absent or dead. `check-refusal-coverage.sh`'s
exhaustiveness guarantee holds only for one file, not for the codebase, and a
real instance of the gap it leaves already exists in the repo
(`refuses_a_hex_encoded_psbt_by_name` in `encode.rs`). `mutate-refusals.sh`
itself is sound — spot-checked twice below, both went correctly red — and
`check-refusal-coverage.sh`'s bijection checks that stay within `refusals.rs`
(test exists, check resolves, signature shape, no duplicates, seeded-set
completeness) are sound as far as they reach.

## Findings

### [Critical] 1 — `scripts/journeys.sh`, Journey A's stdout checks

**Where:** `scripts/journeys.sh:98–105` (the three assertions on
`$WORK/a.out`, i.e. `mt encode`'s stdout — the mt1 strings meant for
engraving, the actual deliverable of the whole tool).

**Why it cannot fail:** all three checks are structurally vacuous against an
*empty* file:
- `lacks "$WORK/a.out" "WARNING"` — true on an empty file.
- `lacks "$WORK/a.out" "FEE"` — true on an empty file.
- `[ "$(grep -cv '^mt1[0-9a-z]*$' "$WORK/a.out")" -eq 0 ]` — counts
  non-matching lines; an empty file has zero non-matching lines, so this
  reads "every line is a valid mt1 string" for a file with **no lines at
  all**.

No assertion anywhere in Journey A checks that `a.out` is non-empty.

**The mutation I applied:** in `crates/mt-cli/src/main.rs`, immediately after
`let rendered = render(&strings, &args);`, inserted
`let rendered: Vec<String> = Vec::new();` — so `mt encode` runs every check,
prints every warning and report row to stderr as normal, and returns `Ok(())`,
but writes **zero lines to stdout**.

**Result:** passed (journeys.sh exited 0).

**Evidence:**
```
$ cargo build   # 1 warning (unused `rendered`), compiles clean
$ ./scripts/journeys.sh
...
  stdout (the artifact):
  assertions:
    ok   bearer warning present
    ...
    ok   stdout carries no prose
    ok   stdout carries no report
    ok   every stdout line is a lowercase, ungrouped mt1 string
...
journeys: A, B (both forms) and C all pass on what the operator SEES
$ echo $?
0
```
`stdout (the artifact):` prints nothing (confirmed empty), and the run still
reports success — the exact failure mode the script's own header comment
warns against ("a journey that checks only `$?` passes against a tool that
succeeds in silence... the output IS the deliverable").

---

### [Critical] 2 — `mt inspect`'s node integration has zero test coverage anywhere in the suite

**Where:** `crates/mt-cli/tests/inspect.rs:1–6` (module doc, the claim);
`crates/mt-cli/src/main.rs:765` (`inspect()`'s `node::Node::find` call).

**Why it cannot fail:** `inspect.rs`'s own module doc states: *"Run both
offline and with a node, because offline-only passes vacuously... the gate
proves nothing about the rows that matter."* But every one of the 14 tests in
`inspect.rs` uses `OFFLINE = "/nonexistent/bitcoin-cli"` — none constructs a
node stub. `node_stub(...)` is defined and used only in `refusals.rs`, and
only to drive `mt encode` (the §8.5/§6a refusal tests) — never `mt inspect`.
`report.rs`'s inline unit tests construct `Report` structs directly, bypassing
`Report::build`'s node-consultation logic entirely. The result: `inspect()`'s
own call site of `Report::build(&tx, &txid, node.as_ref(), &[])` is never
exercised with `node` = `Some(_)` by any test in the repository.

**The mutation I applied:** in `crates/mt-cli/src/main.rs`, inside `inspect()`,
after `let node = node::Node::find(&args.bitcoin_cli);`, inserted
`let node: Option<node::Node> = None;` — so `mt inspect` behaves as if no node
is ever reachable, regardless of `--bitcoin-cli`.

**Result:** passed — the **entire** test suite (117/117) and `journeys.sh`
both passed.

**Evidence:**
```
$ cargo build   # 1 warning (unused `node`), compiles clean
$ cargo nextest run --locked
     Summary [   0.140s] 117 tests run: 117 passed, 0 skipped
$ ./scripts/journeys.sh; echo $?
...
journeys: A, B (both forms) and C all pass on what the operator SEES
0
```
`check-refusal-coverage.sh` is a static-text gate and does not run the binary,
so it is unaffected either way — noted for completeness, not claimed as a
control here.

This is corroborated by `design/agent-reports/R-post-impl-live-node.md`
(already on disk, produced independently by the controller running the real
binary against regtest `bitcoind`): its I1 finding — `inspect` and `encode`
disagreeing, and `inspect` getting `STATUS` right where `encode` got it wrong
— was found by **hand-running the binary against a live node**, not by any
automated test, which is exactly what this mutation proves structurally:
nothing in CI would have caught it either way, because nothing in CI reaches
`inspect`'s node path at all.

---

### [Important] 3 — `check-refusal-coverage.sh`'s "test added with no entry" check only sees `tests/refusals.rs`

**Where:** `scripts/check-refusal-coverage.sh:27` (`TESTS=crates/mt-cli/tests/refusals.rs`)
and the loop at lines 106–111 that flags a `refuses_*`-named test with no
`refusals.toml` entry.

**Why it cannot fail:** the script's own header claims *"A refusal cannot be
added and silently go untested, and a refusal test cannot exist without an
entry saying which rule it proves."* The "other direction" of that bijection
is implemented by scanning **one file** for functions named `refuses_*`. A
refusal-shaped test placed in any other `tests/*.rs` file is invisible to it
— and one already exists in the repo:
`crates/mt-cli/tests/encode.rs:271 fn refuses_a_hex_encoded_psbt_by_name()`,
which asserts `err.starts_with("mt encode: REFUSED — §8.2e,")` — a genuine
refusal assertion, with no entry of its own in `refusals.toml` (it happens to
share `recognised_guard` with an entry that does exist, so it is not currently
an *uncovered* check — but the gate does not know that; it simply never looks).

**The mutation I applied:** appended a new test to `crates/mt-cli/tests/encode.rs`:
```rust
#[test]
fn refuses_a_thing_nobody_declared() {
    assert!(true, "never checked by check-refusal-coverage.sh");
}
```
No corresponding `refusals.toml` entry — the exact condition the gate exists
to catch, placed one file away from where it looks.

**Result:** passed (gate exited 0, printed no problem).

**Evidence:**
```
$ ./scripts/check-refusal-coverage.sh; echo $?
check-refusal-coverage: 15 refusal tests over 12 ruled refusals, each with a test that exists and a check that resolves
  §8.1    refuses_an_unfinalized_psbt
  ...
  §6a     refuses_a_value_that_disagrees_with_the_chain
0
```
No mention of `refuses_a_thing_nobody_declared` anywhere in the output.

The gate's own docstring (lines 19–22) states its scope limit as "whether the
LIST is complete against the spec" — it does not disclose that its *test*-side
bijection is scoped to a single file, which is the actual mechanism of this
gap.

## Tests and gates I broke and which correctly went RED

- **`refuses_an_unfinalized_psbt`** (`crates/mt-cli/tests/refusals.rs`) —
  mutated `finalized_guard_psbt` in `src/validate.rs` to `return Ok(());`
  immediately (neutering §8.1). Result: **FAILED**, correctly — the fixture
  then trips §8.6 instead (no signature shape on an unfinalized input), and
  `assert_refused`'s exact-section check (`"REFUSED — §8.1,"`) caught the
  mismatch rather than accepting the wrong refusal.
  ```
  thread 'refuses_an_unfinalized_psbt' panicked ...
  expected a §8.1 refusal, got:
  mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature
  ```

- **`unreadable_input_reports_the_read_error_not_a_missing_chunk`**
  (`crates/mt-cli/tests/decode_verify.rs`) — mutated
  `crates/mt-codec/src/string_layer/pipeline.rs`'s "nothing readable at all"
  branch to always return `Error::MissingChunk{missing:1,count:0}`, discarding
  `first_error` (simulating the exact regression the test's docstring names:
  "reporting 'missing chunk 1 of 0' for a file of garbage"). Result:
  **FAILED**, correctly:
  ```
  a garbage input was reported as a missing plate: mt verify: REFUSED — §1.1,
  the set does not verify

    chunk 1 of 0 is missing
  ```
  This test's `!err.contains("missing") || err.contains("checksum") ||
  err.contains("BCH")` assertion looked structurally suspicious (a
  three-way OR is trivially true whenever the first clause holds) but is
  **not** vacuous against the regression class it exists for.

All mutations were reverted (`git checkout --`) after each test; `/tmp/fp-review`
ended clean.
