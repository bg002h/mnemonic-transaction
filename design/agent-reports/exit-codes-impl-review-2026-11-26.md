# Review: `mt` exit-code implementation (exit-code spec rev 5)

Reviewer: independent adversarial review. I did not write this code.
Diff: `git diff ded423f^..HEAD` on `fx/mnemonic-transaction` (`ded423f`, `60151a4`,
`56e6ebc`, `bb996d0`, `608c338`). Normative: mnemonic-engrave
`design/SPEC_constellation_exit_codes.md` §2-§5, §7d. Inputs I also read: `/home/user/fx/impl-mt.md`
and the survey `exit-codes-2026-11-26/mt-me.md`.

**Verdict: 0 Critical, 2 Important, 5 Minor, 5 Nit.** The mechanism is sound. Every refusal
carries a class, R8 adds 32, usage and internal never combine, and the legend is the
spec's text. Both Importants are about R9's edge.

## What I ran (binary built from HEAD, `target/debug/mt`)

| probe | got | expected |
| --- | --- | --- |
| `mt --help` (tail) | the legend on one line, word for word as R11; rc 0 | same |
| `mt --help >&-` | `cannot write the output: Bad file descriptor`, rc 32 | 32 (R8) |
| `mt decode --nosuch >&-` | rc 64 | 64 (never combines) |
| `mt decode --in set.txt --bitcoin-cli /x >&-` | rc 32 | 32 |
| `mt encode --in tx.hex --bitcoin-cli /x >&-` | rc 32 | 32 |
| `mt encode ... --out o.txt >&-` | rc 0, `o.txt` holds the strings (0600) | 0 |
| `mt decode --bitcoin-cli /x <&-` | rc 2 ("no strings found") | see N3 |
| `mt decode --in s2.txt --transaction tx.hex >> s2.txt` | **rc 8** (§8.2i) | **64** (R2), see M1 |
| `mt inspect --in s2.txt --transaction tx.hex >> s2.txt` | **rc 8** | **64**, see M1 |
| named helper `#!/bin/sh` exits 0 with empty output | **rc 0**, STATUS "the node did not answer every question … Check the node" | **32**, see I1 |
| named helper that answers `garbage` | **rc 0**, same STATUS | **32**, see I1 |
| named helper whose shebang interpreter is missing (spawn ENOENT) | rc 0, "no bitcoind reachable" | see I2 |
| named path that does not exist | rc 0 | see I2 |
| `scripts/check-refusal-coverage.sh` | OK, 93 rows | OK |
| same gate, with a row's test changed to `assert_refused(.., 2, /* INVALID */ ..)` or `.., 1, // INVALID` | **still OK** | FAIL, see M2 |
| `cargo test -p mt-cli --test exit_codes` | 14 passed, 0 failed | pass |

## Critical

None.

## Important

### I1. A named `--bitcoin-cli` that runs but gives no usable answer exits 0. R9 and mt's own new help text both say 32.

- **Where:** `crates/mt-cli/src/node.rs:121-165` (`call` only notes IO on spawn, pipe or non-zero
  exit). The help text added at `main.rs:133-134` and `main.rs:299-300` says: "A path you name that
  runs and does not answer adds 32".
- **Scenario:** `--bitcoin-cli ./wrapper` where the wrapper exits 0 and prints nothing, or prints an
  HTML error page or a proxy banner. `Node::find` treats any `getblockcount` reply as a node. Every
  later reply fails to parse, so mt prints `STATUS UNKNOWN — the node did not answer every question …
  Check the node, then run mt again`, and exits **0**. In the same situation, a stub that exits 1
  gives 32 (`node_tristate.rs` rows). Two failures with the same report get two different codes.
  A script cannot tell this run from "every check passed".
- **Spec:** R9 says that when the user named the helper, "a failure sets 32". §3 test 2 says the
  same: "something needed from the system failed". The implementer's spec question 1 deliberately
  leaves out unparseable replies ("mt already reports it as UNKNOWN"). But reporting UNKNOWN is the
  R9 warning, not a reason to exit 0. A helper the operator asked for is not optional.
- **Fix:** a named helper whose reply mt cannot use should call `helper_failed()`. That covers a
  `getblockcount` reply that does not parse as an integer (do it in `Node::find` or `block_count`), a
  `getblockchaininfo` that is not JSON, `Utxo::Unanswered` from a parse failure, and
  `ParentState::Unanswered` from a reply. Two real answers must not count: `gettxout` null or empty,
  and `getindexinfo` `{}`. Add `exit_codes.rs` rows for a silent stub and a garbage stub. If Brian
  rules the other way, the help text must stop saying "does not answer". Today the help says 32
  while the binary exits 0.

### I2. Spec question 1: "named but absent = offline, 0". This is a deviation from R9 as written and needs a ruling.

- **Where:** `node.rs:131-132`: `ErrorKind::NotFound` returns `Failed(None)` without noting IO.
  `main.rs:378-387` sets "named" from `ValueSource::CommandLine`.
- **The case for the reading:** mt's normative spec and every gate and journey use
  `--bitcoin-cli /nonexistent/bitcoin-cli` as the offline switch, so making it 32 breaks that
  contract.
- **The case against it, from the letter of R9 and §3:** R9 says "When the user named the helper
  (`mt --bitcoin-cli PATH`) … a failure sets 32". Test 2 says "failed **or gone missing**". §7d says
  only "no node at all stays 0", and does not say that a named-but-missing path counts as no node.
- **Failure scenario:** a typo such as `--bitcoin-cli /usr/local/bin/bitcoin-cIi`, or a wrapper
  script whose interpreter is missing. ENOENT from `execve` also reports `NotFound`, which I measured:
  rc 0. The operator believes the node checks ran (spent inputs, a contradicted value) and gets
  exit 0, with only a warning paragraph saying they did not. For `encode` before cutting steel, this
  is the case R9 exists for.
- **Fix:** ask Brian before merge. Option (a) is cheap: keep the carve-out, and have the exit-code
  spec §7d and `SPEC_mt_v0_1.md` §6a say it explicitly ("a named path that does not exist is the
  offline switch, 0"). Option (b) is better aligned with R9: add an explicit offline switch (for
  example `--offline` or `--bitcoin-cli none`), make a named-but-missing path 32, and move the gates
  and journeys to the switch. That is a flag change, so the toolkit manual, GUI and refusals.toml
  move with it. Until the ruling, the CHANGELOG and README should call this a provisional reading.
- **On the reading itself:** it is defensible, but it is not the one closest to the §3 tests. Test 2
  read literally gives 32.

## Minor

### M1. R2: a usage error can lose to a refusal. `same_file_guard` (8) runs before two usage guards (64).

- **Where:** `main.rs` `decode` (about :1657-1665): `json_quiet` → **`same_file_guard`** →
  `transaction_unsupported_guard` → `in_with_dash_guard`. `inspect` (about :2122-2130) has the same
  order. `encode` (:438-455) runs `json_unsupported` → `separator` → **`same_file_guard`** →
  `in_with_dash_guard`.
- **Scenario, measured:** `mt decode --in s2.txt --transaction tx.hex >> s2.txt` gives 8. It should
  give 64. The same holds for `mt encode --in a.hex - --out a.hex`. `same_file_guard` stats files,
  and test 1 is judged "without reading any input, file or environment", and it is asked first. So
  the command-line error has to win. The diff moved `separator_guard` for this exact reason but left
  these two guards behind.
- **Fix:** in all three verbs, run every `Exit::USAGE` guard (`transaction_unsupported`,
  `in_with_dash`, `quiet/json_unsupported`, `json_quiet`, `separator`) before `same_file_guard`. Add
  an `exit_codes.rs` row for usage plus same-file. In `refusals.toml`, the §8.2i Q1c note ("before
  any read") stays true.

### M2. The R13 gate (`check-refusal-coverage.sh` check 5) passes a row whose test does not assert its class.

- **Where:** `scripts/check-refusal-coverage.sh:221-243`. The positive check is
  `\bCLASS\b` anywhere in the test body, after string literals are stripped. Comments are not
  stripped, and the check does not require the name to be the asserted value. The negative check
  only catches `Some(X)`, `, X,` and `(X, `, so numeric literals get through.
- **Demonstrated** on a scratch copy: I changed `refuses_a_duplicate_outpoint` to
  `assert_refused(&out, &err, rc, 2, /* INVALID */ "§8.2b")` and then to
  `assert_refused(&out, &err, rc, 1, // INVALID` + newline. **Both pass the gate.** The second
  asserts class NO for an INVALID row. `assert_ne!(rc, Some(INVALID))` and `let _ = INVALID;` would
  pass too.
- Today's rows really do assert their class (each goes through `assert_refused` /
  `assert_refused_with` / `assert_same_file_refusal`, which `assert_eq!` the code). So this is a gate
  weakness, not a current false PASS.
- **Fix:** strip `//…` and `/*…*/` before matching. Require the class in an asserting position:
  `Some(CLASS)` inside `assert_eq!`, or as the `class` argument of a helper listed in the script
  that is known to `assert_eq!` it. Also fail any numeric `Some(<digit>)` compared to the exit code
  in a row's test.

### M3. The CHANGELOG entry from `ded423f` sits in the released 0.2.0 section and says "exits 1".

- **Where:** `CHANGELOG.md:69`, under `## mt-cli [0.2.0] — 2026-11-04` → `### Fixed`.
  `mt-cli-v0.2.0` is `eec7910`, which is before this work.
- That release did not contain this fix, and the code it states is now 32. The commit subject
  (`ded423f`, "exits 1") is history and can stay as it is.
- **Fix:** move the entry under `## mt-cli [Unreleased]` and say "adds 32 (R8)".

### M4. Weak or stale test assertions.

- `tests/exit_codes.rs` `a_named_bitcoin_cli_that_fails_is_io_and_an_absent_one_is_offline` asserts
  `!out.is_empty() || verb == "inspect"`, which exempts `inspect` from "the report still prints".
  `inspect` prints its report on stdout (measured), so the exemption makes R9's "keeps its report"
  half of the claim untested for that verb. Assert non-empty stdout for `inspect` too.
- `tests/bh1119_mt_output_writes.rs` `closed_stdout_exits_1_with_the_line` asserts IO (32), but its
  name still says 1. The file header keeps old names for the FIX table, but this test is new in
  `ded423f` and has no FIX row citing it. Rename it `closed_stdout_adds_io_with_the_line`.
- Same file: the `inspect` case in that test runs with the default `bitcoin-cli`, not `OFFLINE`. On
  a CI host with a real `bitcoin-cli` it talks to a node. That does not change the code (the
  default is not "named"), but it makes the test depend on the environment. Pass `OFFLINE`.

### M5. The normative mt spec still states the old codes. This is owed upstream, and the implementer gave approximate line numbers.

- `design/SPEC_mt_v0_1.md` (the copy here; the source is in mnemonic-engrave) has four stale
  statements:
  - `:2620`: an `--out` write failure "exits 1"
  - `:2708`: a stdout write failure "exits 1"
  - `:3532` and `:3534`: §10.10 "exit 1" (now 64)
  - `:3375-3386`: the exit-status paragraph
- The implementer said they could not find the write-failure lines. They are `:2620` and `:2708` in
  this copy. Following the repo hazard, edit mnemonic-engrave first and re-sync. File one
  `owed`-type FOLLOWUP so the drift is tracked, not just listed in a report.

## Nit

- **N1.** `Exit::INTERNAL` refusals (`main.rs` §3b "transaction cannot be chunked", and §1.1 "cannot
  derive the content id") print in the operator refusal shape: `mt encode: REFUSED — §3b, …`. Exit
  101 means "report it", so add a line saying this is a bug to report. Otherwise an operator reads it
  as a policy refusal.
- **N2.** `node.rs:159-161` exempts exit 5 and 89 for every call. Only `getrawtransaction` (5) and
  `getindexinfo` (89) can return them as answers. A named helper that exits 5 on `getblockcount` is
  treated as an absent node and gives 0. Scope the exemption to the call that can answer with it.
- **N3.** A closed stdin (`<&-`) is now plugged by the probe and reads as empty input, so 2. That
  matches decision 3 ("empty input is 2") and is not a regression (std plugged it before). Still, a
  missing fd 0 is arguably §3 test 2 ("missing from the system"). Mention it in the spec's §7d, or
  leave it.
- **N4.** R11 says "Per-verb help states only the bits that verb can set". Per-verb help states only
  the `--bitcoin-cli` 32 line, and does not say that `decode`, `verify` and `inspect` can set 16 or
  that `encode` can set 8. Add one short line per verb.
- **N5.** Spec question 4 (a discarded damaged duplicate gives 0). This follows R6's "used"
  literally, and I agree with it. But mt prints "DISCARDED the copy needing 1 of 4 … it is the one to
  re-cut", and code 16's operator action is "re-engrave". Journey C's `drawer-scrap` names an
  unreadable plate and exits 0. Raise it as a one-line spec question so that a code of 0 with
  "re-cut this plate" is a choice Brian makes, not a side effect.

## The implementer's spec questions

| # | question | judgment |
| --- | --- | --- |
| 1 | an absent named path is offline (0), and a reply that does not parse is not counted | **Absent: defensible but not closest to §3 test 2; needs a ruling (I2). Unparseable reply: disagree (I1).** Not getting an answer from a helper the operator named is the failure R9 describes, and mt's own new help text says so. |
| 2 | R1 sums: 17 (mismatch after correction), 18 (content id fails after correction), 34; an uncorrectable set stays 2 | **Agree.** §3 itself expects a refusal after a correction to keep 16 ("a network refusal found before or after a correction … assert the bits"). The content-id refusal text names the corrected chunks (`main.rs:2440-2470`), so the reported outcome contains the correction. R1's atomic clause (`md repair` → 2, not 18) is about an *uncorrectable* input, which mt also gives 2. |
| 3 | transliteration (`l`/`i` → `1` in the HRP) is not a correction | **Agree.** No checksum correction is applied. 16's action ("the plate is damaged") does not fit a glyph-reading ambiguity. |
| 4 | a duplicate plate where the clean copy is kept gives 0 | **Agree with the letter of R6** ("used"). Raise it as a spec note (N5). |
| 5 | `--separator` is a setting (64) | **Agree.** Decision 2 names "output form" as a setting, and moving the guard before input (R2) is right. M1 is the same fix, not yet applied to two other guards. |
| 6 | the two unreachable codec failures are 101 | **Agree** (§3 test 7, R13). Wording nit N1. |

## Checked and found correct

- **Legend (R11).** `exit::LEGEND` matches the spec's text character for character. The rendered
  root `--help` ends with it on one line (clap does not wrap it). `the_legend_is_the_specs` and
  `the_root_help_ends_with_the_legend` pin it.
- **R8.** `out::exit_code` ORs in IO and keeps the verb's bits (`decode` on a corrected set to
  `/dev/full` gives 48, tested). EPIPE keeps the verb's own code. USAGE and INTERNAL absorb every
  other bit (`BitOr`, unit-tested). `--help >&-` gives 32.
- **R2 and argv guard.** Non-UTF-8 argv gives 64 before anything else runs. The §8.2f argv guard
  runs before clap and gives 8. Clap errors give 64. Help and version give 0. The separator check
  now runs before input is read. The exception is M1.
- **Decision 2.** `--input-value` (an input value) gives 2. `--group-size x` and `--separator` (both
  settings) give 64. No clap value parser rejects an input value.
- **Classes against the §3 table and §7d.** Unsigned tx to mt: 2 (§8.1). ms1: 2 (§8.9). Pasted txid:
  2. Empty input: 2. A world-readable stdout: 8 (§8.2h). Same file: 8 (§8.2i). §8.5 spent input: 1.
  §6a: 1. `verify --transaction` mismatch: 1. A file that cannot be read: 32. §8.2b fee cap: 2. That
  last one is right because mt has no flag that waives the cap, so §3 test 5 does not apply ("which
  a flag … would waive"; compare `derive-child --application rsa` → 2).
- **R13 blanket mappings removed.** No `ExitCode::FAILURE` and no catch-all `Err → 1` are left. All
  67 `Refusal::new` sites carry a class, and the compiler enforces it.
- **R5.** `inspect`'s STATUS DEAD stays 0.
- **The closed-stdout probe (`out.rs` `closed_fds`).**
  - It runs from `.init_array` / `__mod_init_func`, before `lang_start` and so before std's
    `sanitize_standard_fds`. That is single-threaded, and the only calls are `fcntl` and `open`.
  - The constants `F_GETFD=1`, `O_RDWR=2` and `EBADF=9` are right on every listed OS.
  - Plugging the lowest fd first means each `open` lands on the closed fd, so an `--out` file can
    never become fd 1 (measured: `--out` with `>&-` gives 0 and a correct file).
  - Unix targets that are not listed (illumos, AIX and others) fall back to std's plug, as before.
    That is not a regression, but it is worth a line in the CHANGELOG.
  - Windows is untouched.
  - The probe also links into the bin crate's unit-test harness. That is harmless.
- **Repo hazards.** Binary-PSBT sniffing is untouched (`input::sniff` still sees raw bytes; the only
  change is the class argument). Node access is still read-only: no new RPC verbs were added.
- **Test stubs.** The stubs now answer `getblockchaininfo` as a regtest node. The coverage this
  removed from `inspect.rs` (the MAINNET fallback) is still covered by the `node_tristate.rs` i14
  rows.

## Targeted test run

`cargo test --locked -p mt-cli --test exit_codes` (CARGO_PROFILE_*_DEBUG=0, CARGO_INCREMENTAL=0):
14 passed, 0 failed. The I1 and M1 cases have no row in this file, which is why it passes.
