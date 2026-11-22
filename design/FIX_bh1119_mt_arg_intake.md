# FIX: argument intake (bughunt-1119 FIX table row 21, "FIX-H argument intake")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Keys: bh-mt-I9, bh-mt-I11,
bh-mt-I12, bh-mt-I24 (all Important) and sweep-1122 row 12
(`mt-decode-inspect-transaction-ignored`, the same root as I11). Rulings: architect
T12, T13, T20, T22 (`coordinator/agent-reports/architect-bughunt-1119-questions.md:50,51,58,60`),
binding. Track: small (one Opus review round). Not a funds-path (class B) row.
Base: origin/main `1f6cc6b`. Branch `fix/mt-arg-intake`. Every new holder-visible
text is marked **PROPOSED (architect confirms)**; everything else below is decided.

Not to be confused with the older `design/FIX_bughunt_mt_*.md` (a different hunt).

## 1. Findings (at 1f6cc6b, each run by the author)

- **I9.** `separator_guard` (`crates/mt-cli/src/main.rs:2213-2229`) admits any
  all-whitespace separator. The reader (`read_strings.rs:63-64`) splits input on
  `\n` first (`str::lines`), then strips whitespace inside a line. So
  `mt encode --group-size 5 --separator '\n'` exits 0 and prints strings that
  `mt decode` refuses: `mt decode: REFUSED — §3b, the full string is only 5
  characters`. Measured on the fixture `p5_base.json` `raw_hex`: `"\n"`, `"\r\n"`
  and `" \n"` all encode at exit 0 and fail to decode; `" "`, `"\t"`, `"\r"`,
  U+000B, U+000C, U+0085 and U+2028 encode and decode back to the same hex.
- **I11 (= sweep row 12).** `--transaction` lives in the shared `ReadArgs`
  (`main.rs:101-113`) but only `verify` reads it (`main.rs:1446`). `mt decode
  --transaction /nonexistent/tx.hex` and `mt inspect --transaction ...` exit 0
  and never open the file; `mt verify` with the same arguments exits 1.
- **I12.** `--in <path>` and the positional `-` (`ReadArgs.stdin_dash`
  `main.rs:67-82`, `EncodeArgs.stdin_dash` `main.rs:140-152`) are both accepted;
  the file wins and stdin is silently ignored. Exit 0 on all four verbs
  (`decode`, `verify`, `inspect`, `encode`).
- **I24.** `main` reads argv with `std::env::args()` (`main.rs:293`), which
  panics on a non-UTF-8 argument: `mt verify --in plate-\xff.txt` exits 101,
  `thread 'main' panicked at library/std/src/env.rs:871:51`. The same panic
  fires for a non-UTF-8 program path (argv[0]).

## 2. Rulings (binding, quoted)

- **T12 mt-Q4 = sweep MT-Q1 (I11, row 12):** refuse `--transaction` on
  decode/inspect. main.rs:2061: "A flag that cannot work must REFUSE, not sit
  there inert."
- **T13 mt-Q5 (I12):** refuse `--in X` together with `-`, same rule (:2061);
  applies to EncodeArgs too.
- **T20 mt-Q12 (I9):** encode admits exactly the separators decode admits (one
  shared predicate).
- **T22 mt-Q15 (I24):** read argv with `args_os` and refuse a non-UTF-8 argument
  with a named error (exit 64) before `argv_intake`, which keeps `&[String]`
  unchanged.

Spec check (the spec is a copy owned by mnemonic-engrave): §10's flag table
(`design/SPEC_mt_v0_1.md:3520`) already lists `--transaction` as "on `verify`";
§1.1e (`SPEC:987-991`) splits on newlines and strips whitespace within a line,
which is the T20 predicate; nothing in the spec mentions `-`, argv encoding or
exit 64. No spec text is affected; no erratum.

## 3. Behaviour after the fix (rules)

Refusals use the existing `Refusal` type and print through it (`refusal.rs`:
first line `mt <verb>: REFUSED — <section>, <verdict>`, then the mechanism and
remedy wrapped at 68 columns, indented two spaces). Exit 1 for every refusal
except H4, which exits 64 (T22). stdout stays empty on every refusal.

- **H1 (T20, I9).** One predicate, `read_strings::is_grouping_whitespace(c: char)
  -> bool`, defined as `c != '\n' && c.is_whitespace()`, `pub(crate)`.
  - The reader strips, within a line, exactly the characters it accepts
    (`read_strings.rs:64`: `line.split_whitespace()` becomes a split on
    `is_grouping_whitespace`; behaviour unchanged, because `str::lines` has
    already removed every `\n` and a CRLF's `\r`).
  - `separator_guard` admits a separator iff every character satisfies the
    predicate (the empty separator stays admitted). Otherwise it refuses, in this
    order: (a) any character that is not whitespace gives the EXISTING refusal,
    unchanged (`--separator ".\n" is not whitespace`); (b) else (the separator is
    whitespace and holds a `\n`) the new refusal N1.
  - Unchanged: `--separator` is still checked where it is now (`main.rs:418`,
    after the input is read and §8.9/§8.2g have run).
- **H2 (T12, I11).** New `transaction_unsupported_guard(transaction:
  Option<&std::path::Path>, verb: &str) -> Result<(), Refusal>` in `main.rs`.
  It refuses with T1 whenever `--transaction` is present, whatever the path
  (missing, unreadable or a matching transaction file). Called in `decode` and
  `inspect` only, after their existing flag guards and before any input is read:
  - `decode`: `json_quiet_guard` (unchanged, first), then H2, then H3, then
    `read_input`.
  - `inspect`: `quiet_unsupported_guard` (unchanged, first), then H2, then H3,
    then `read_input`. Both inspect subjects (mt1 strings and a raw
    transaction/PSBT) are covered because the guard runs before routing.
  - `verify` is unchanged and still compares.
- **H3 (T13, I12).** New `in_with_dash_guard(has_in: bool, dash: bool, verb:
  &str) -> Result<(), Refusal>` in `main.rs`. It refuses with D1 when `--in` is
  given (any value, including `--in -`) and the positional `-` is present. Called:
  - `decode`, `inspect`: after H2 (above).
  - `verify`: after `quiet_unsupported_guard` and `json_unsupported_guard`
    (unchanged), before `read_input`.
  - `encode`: after `json_unsupported_guard` (unchanged), before the intake
    `match` (`main.rs:370`), so nothing is read.
  - Unchanged: `-` alone and `--in` alone; `--in` plus `--allow-argv-secret`
    material keeps its existing warning (file wins). When `--in`, `-` and admitted
    argv material are all present, H3 refuses first.
- **H4 (T22, I24).** New `pub fn utf8_argv_guard(args: Vec<std::ffi::OsString>)
  -> Result<Vec<String>, Refusal>` in `validate.rs`. `main` becomes: collect
  `std::env::args_os()`, call the guard, and on `Err` print the refusal with
  `eprint!` and return `std::process::ExitCode::from(64)`; on `Ok` the
  `Vec<String>` goes to `validate::argv_intake` exactly as today (its `&[String]`
  signature and everything after it are unchanged). `std::env::args()` no longer
  appears in `crates/mt-cli/src`.
  - argv[0] (the program path) is converted with `to_string_lossy` and never
    refused (decided: it is not an argument the holder typed, and clap's name is
    fixed to `mt` by `#[command(name = "mt")]`).
  - Arguments 1.. are converted with `OsString::into_string`; the FIRST that fails
    is refused with U1, naming its position `n` (its argv index: argument 1 is the
    first after the program name). Later arguments are not inspected.
  - The verb in the refusal's first line is chosen as `command_line_guard` chooses
    it (`validate.rs:661-665`): argv[1] if it is valid UTF-8 and one of `encode`,
    `decode`, `verify`, `inspect`; otherwise `encode`.
  - The refusal never prints the argument, lossy or otherwise.
  - It runs before `argv_intake` and before `command_line_guard`, so a non-UTF-8
    argument next to bearer material on argv gets U1 and nothing is echoed.

## 4. Holder-visible texts (all PROPOSED, architect confirms)

Each text below is the exact Rust string literal content (between the quotes,
one logical line each; `\n\n` is a paragraph break). `{sep:?}` is the
separator's Debug form (so `"\n"` prints as `"\n"` with a backslash), `{verb}`
the verb, `{n}` the argv index. The fenced blocks are verbatim; nothing in them
is Markdown.

**N1** — encode, section `§1.1e`, exit 1:

```text
verdict:   --separator {sep:?} contains a newline
mechanism: mt reads strings back one per line: a newline ends a string. A separator containing one cuts every string into pieces on stdout — the stream you engrave — and mt's own verbs then refuse the result.\n\nmt refuses this now rather than after the plates are cut.
remedy:    Use a space, a tab, or omit --separator (a space is the default).
```

(The remedy is the existing guard's, unchanged.)

**T1** — decode and inspect, section `§10.10`, exit 1:

```text
verdict:   --transaction has no meaning for `{verb}`
mechanism: `--transaction` compares the set against a transaction by its full txid, and only `verify` makes that comparison. `{verb}` would accept the flag and never open the file, so a caller would take the transaction as checked when nothing compared it.\n\nmt refuses rather than accepting the flag and ignoring it.
remedy:    Run `mt verify --transaction <path>` for the comparison, or drop `--transaction`.
```

**D1** — encode, decode, verify, inspect, section `§10.10`, exit 1:

```text
verdict:   --in and `-` name two different inputs
mechanism: `--in` names a file and `-` names standard input, and mt reads exactly one input. Picking either one silently could give you the answer for the input you did not mean, at exit 0.\n\nmt refuses rather than choosing for you.
remedy:    Drop `-` to read the file, or drop `--in` to read standard input.
```

**U1** — verb per H4, section `§10.10`, exit 64:

```text
verdict:   argument {n} is not valid UTF-8
mechanism: mt reads its command line as text, and argument {n} holds bytes that are not UTF-8. mt reads nothing until every argument is text, and it does not print the argument back.
remedy:    If the argument is a file name, rename the file or make a link to it under a UTF-8 name, and pass that name instead.
```

**Help (`mt <verb> --help`)**, doc-comment lines, verbatim before -> after:

```text
main.rs:196  /// Separator to use with `--group-size`. **Whitespace only.**
          -> /// Separator to use with `--group-size`. **Whitespace only, never a newline.**
main.rs:103  /// `verify` only. Comparing against the 20-bit set id would report a match
          -> /// `verify` only; `decode` and `inspect` refuse it. Comparing against the
             /// 20-bit set id would report a match
```

(the rest of the `--transaction` paragraph is unchanged; the builder rewraps it).
The hidden `stdin_dash` doc comments (`main.rs:67-82`, `140-152`; `hide = true`,
not shown by `--help`) replace "Accepted and IGNORED" with "Accepted alone;
refused together with `--in` (T13)". Not holder-visible.

The rows assert the first line exactly and one phrase of each mechanism after
undoing the wrap (constants at the top of the row file), so a wording change
edits only those constants.

## 5. Spec, README, help and manual delta

- **Spec:** none (§2).
- **README:** none. `README.md:26` ("a file or stdin, never an argument") stays
  true.
- **Help:** §4 "Help".
- **Toolkit manual:** none. `mnemonic-toolkit` master
  `docs/manual/src/40-cli-reference/` has no `mt` page (41-mnemonic, 42-md,
  43-ms, 44-mk-cli, 45-mr, 46-mr-sitting); its `--separator` rows describe the
  toolkit's own flags.
- **CHANGELOG:** none in this FIX (the file has no Unreleased section; release
  prep writes the entry).

## 6. RED rows

All 22 rows are in a new test binary `crates/mt-cli/tests/arg_intake.rs`
(`mt-cli::arg_intake`). Its exact text is the sidecar
`design/FIX_bh1119_mt_arg_intake/arg_intake.rs.txt` (sha256
`dfbf9c063ce52499a3709dd6da3ef610c75d20c55216baffe89814d94b15d816`); the builder's
RED commit copies it to that path unchanged. Rows drive the real `mt` binary
offline (`--bitcoin-cli /nonexistent/bitcoin-cli`) on `tests/fixtures/p5_base.json`
`raw_hex` and on the mt1 strings `mt encode` makes from it. The five I24 rows that
build non-UTF-8 bytes are `#[cfg(unix)]` (CI runs ubuntu-latest).

Commands run by the author on 1f6cc6b with the row file in place:

    cargo nextest run --locked --build-jobs 6 -j 6 --workspace --no-fail-fast
    cargo nextest run --locked --build-jobs 6 -j 6 -p mt-cli --test arg_intake --no-fail-fast

Recorded: the full suite `424 tests run: 408 passed, 16 failed` (via the gate,
`/scratch/cache/mt-arg-intake-gate-red.log`); the 16 are the RED rows below. The
row binary alone: `22 tests run: 6 passed, 16 failed`
(`/scratch/cache/mt-arg-intake-red.log`). The 402 existing tests all pass.

| row | input | GREEN expects | at 1f6cc6b |
|---|---|---|---|
| `bh1119_mt_i9_encode_refuses_lf_separator` | `--group-size 5 --separator "\n"` | rc 1, N1 | FAIL: rc 0 |
| `bh1119_mt_i9_encode_refuses_crlf_separator` | separator `"\r\n"` | rc 1, N1 | FAIL: rc 0 |
| `bh1119_mt_i9_encode_refuses_separator_ending_in_lf` | separator `" \n"` | rc 1, N1 | FAIL: rc 0 |
| `bh1119_mt_i9_non_whitespace_wins_over_newline` (guard) | separator `".\n"` | rc 1, existing `--separator ".\n" is not whitespace` | PASS |
| `bh1119_mt_i9_encode_admits_every_separator_decode_admits` (guard) | `""`, `" "`, `"\t"`, `"  "`, `"\r"`, U+000B, U+000C, U+0085, U+2028 | each: encode rc 0, `decode --quiet` prints `raw_hex` | PASS |
| `bh1119_mt_i11_decode_refuses_transaction` | `decode --in <set> --transaction /nonexistent/tx.hex` | rc 1, T1(decode) | FAIL: rc 0 |
| `bh1119_mt_i11_inspect_refuses_transaction` | `inspect --in <set> --transaction /nonexistent/tx.hex` | rc 1, T1(inspect) | FAIL: rc 0 |
| `bh1119_mt_i11_inspect_raw_tx_refuses_existing_transaction` | `inspect --in <tx.hex> --transaction <tx.hex>` | rc 1, T1(inspect) | FAIL: rc 0 |
| `bh1119_mt_i11_transaction_refusal_precedes_dash_refusal` | decode with `--transaction` and `-` | rc 1, T1 (not D1) | FAIL: rc 0 |
| `bh1119_mt_i11_existing_flag_refusals_keep_precedence` (guard) | decode `--json --quiet --transaction X -`; inspect `--quiet --transaction X` | existing `--json and --quiet contradict each other`; existing ``--quiet has no meaning for `inspect` `` | PASS |
| `bh1119_mt_i11_verify_still_compares` (guard) | `verify --in <set> --transaction <tx.hex>` | rc 0, `--transaction matches, on the full txid.` | PASS |
| `bh1119_mt_i12_decode_refuses_in_with_dash` | `decode --in <set> -`, stdin `not read` | rc 1, D1(decode) | FAIL: rc 0 |
| `bh1119_mt_i12_verify_refuses_in_with_dash` | same, verify | rc 1, D1(verify) | FAIL: rc 0 |
| `bh1119_mt_i12_inspect_refuses_in_with_dash` | same, inspect | rc 1, D1(inspect) | FAIL: rc 0 |
| `bh1119_mt_i12_encode_refuses_in_with_dash` | `encode - --in <tx.hex>` | rc 1, D1(encode) | FAIL: rc 0 |
| `bh1119_mt_i12_in_alone_and_dash_alone_still_work` (guard) | `decode --quiet --in <set>`; `decode --quiet -` with the set on stdin | rc 0, `raw_hex` both | PASS |
| `bh1119_mt_i24_non_utf8_argument_is_refused_exit_64` | `verify --in plate-\xff.txt` | rc 64, U1(verify, 3); no `panicked`, no `plate-` | FAIL: rc 101 |
| `bh1119_mt_i24_names_the_first_non_utf8_argument` | `decode --in \xfe --transaction \xff` | rc 64, U1(decode, 3) | FAIL: rc 101 |
| `bh1119_mt_i24_refused_before_allow_argv_secret` | `encode --allow-argv-secret <raw_hex> --in \xff` | rc 64, U1(encode, 5); `raw_hex[..16]` not on stderr | FAIL: rc 101 |
| `bh1119_mt_i24_untyped_verb_falls_back_to_encode` | `mt \xff` | rc 64, U1(encode, 1) | FAIL: rc 101 |
| `bh1119_mt_i24_non_utf8_program_path_is_not_refused` | symlink `mt-\xff` -> mt; `verify --in <set>` | rc 0 | FAIL: rc 101 (`env.rs:871`) |
| `bh1119_mt_i24_utf8_non_ascii_path_still_works` (guard) | `verify --in <dir>/plate-é.txt` | rc 0 | PASS |

"N1/T1/D1/U1" in the GREEN column means: stdout empty, stderr's first line is
exactly that refusal's first line, and the flattened stderr contains its phrase
constant.

## 7. Tests that change

- **No existing test changes.** Checked: the only `--transaction` users
  (`inspect.rs:805`, `refusals.rs:1248`, `refusals.rs:1280`,
  `decode_verify.rs:320`, `decode_verify.rs:335`) all run `verify`; the
  separator tests (`refusals.rs:1141-1180`) use `-`, `.`, `|`, `0`, `" "`,
  `"\t"`, `"  "`; the dash tests (`encode.rs:355-400`, `decode_verify.rs:1395-1420`)
  pass `-` without `--in`. The 402 existing tests pass at 1f6cc6b with the rows
  added, and none asserts a behaviour this FIX changes.
- **`crates/mt-cli/tests/refusals.toml`:** 17 new entries, appended at the end,
  exact diff in the sidecar `design/FIX_bh1119_mt_arg_intake/test-changes.diff`
  (sha256 `858a1ce6db25efe31043dc9de8be216a3cc97f97d34f679ad6d603881c2e179a`),
  applied in the RED commit with `git apply`. One entry per refusal row (the 15
  RED refusal rows plus the two guards whose bodies assert on `REFUSED — §`, which
  `check-refusal-coverage.sh` would otherwise flag): checks `src/main.rs::separator_guard`
  (4), `src/main.rs::transaction_unsupported_guard` (4),
  `src/main.rs::json_quiet_guard` (1), `src/main.rs::in_with_dash_guard` (4),
  `src/validate.rs::utf8_argv_guard` (4). All cite `§1.1e` or `§10.10`, both
  already in the gate's seeded set, so `scripts/check-refusal-coverage.sh` needs
  no edit. Coverage goes from `42 refusal tests over 18 ruled refusals` to
  `59 refusal tests over 18 ruled refusals`. Run at 1f6cc6b with the rows and
  the diff applied, the coverage gate fails naming exactly the three guard
  functions that do not exist yet (`transaction_unsupported_guard`,
  `in_with_dash_guard`, `utf8_argv_guard`); it passes once they exist.
- The mutation of `utf8_argv_guard` (`return Ok(Default::default())`) hands an
  empty argv to clap, which exits 2, so its four rows (expecting 64) go red.

## 8. Build order

1. RED commit: copy `arg_intake.rs.txt` to `crates/mt-cli/tests/arg_intake.rs`,
   `git apply design/FIX_bh1119_mt_arg_intake/test-changes.diff`. Run the row
   binary: `22 tests run: 6 passed, 16 failed`.
2. H4 (`utf8_argv_guard`, `main` reads `args_os`): the five I24 rows go green.
3. H1 (predicate, reader, `separator_guard`, `--separator` help): the three I9
   RED rows go green.
4. H2 and H3 (two guards, their call sites, `--transaction` help, hidden doc
   comments): the eight I11/I12 RED rows go green.
5. Gate (§10).

## 9. Coverage map (rule -> rows; `bh1119_mt_` omitted)

| rule | rows |
|---|---|
| H1 newline refused (`\n`, `\r\n`, trailing) | i9_encode_refuses_lf_separator, i9_encode_refuses_crlf_separator, i9_encode_refuses_separator_ending_in_lf |
| H1 order (a) before (b) | i9_non_whitespace_wins_over_newline |
| H1 admitted set = decode's set (incl. `""`, `\r`, U+2028) | i9_encode_admits_every_separator_decode_admits |
| H1 one shared predicate | gate.sh grep lines (not a row) |
| H2 decode / inspect (strings) / inspect (raw tx, existing file) | i11_decode_refuses_transaction, i11_inspect_refuses_transaction, i11_inspect_raw_tx_refuses_existing_transaction |
| H2 before H3; existing guards first | i11_transaction_refusal_precedes_dash_refusal, i11_existing_flag_refusals_keep_precedence |
| H2 verify unchanged | i11_verify_still_compares |
| H3 on each of the four verbs | i12_decode_/verify_/inspect_/encode_refuses_in_with_dash |
| H3 single source unchanged | i12_in_alone_and_dash_alone_still_work (+ existing encode.rs / decode_verify.rs dash tests) |
| H4 exit 64, position named, no echo, no panic | i24_non_utf8_argument_is_refused_exit_64 |
| H4 first offending argument only | i24_names_the_first_non_utf8_argument |
| H4 before argv_intake | i24_refused_before_allow_argv_secret |
| H4 verb fallback | i24_untyped_verb_falls_back_to_encode |
| H4 argv[0] lossy | i24_non_utf8_program_path_is_not_refused |
| H4 UTF-8 non-ASCII unaffected | i24_utf8_non_ascii_path_still_works |

Not covered by a row: the help-text edits (§4 "Help") and the `--in -` spelling
of H3 (same code path as any `--in` value).

## 10. Gate (builder, final head; run once to a file, then grep)

    bash design/FIX_bh1119_mt_arg_intake/gate.sh > /scratch/cache/mt-arg-intake-gate.log 2>&1

The script (committed with this document) runs the CI `test` job's steps
(`cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
`cargo build --locked`, `cargo nextest run --locked` expecting
`424 tests run: 424 passed`, doc tests, `check-refusal-coverage.sh` expecting
`59 refusal tests over 18 ruled refusals`, `mutate-refusals.sh` expecting
`all 59 refusal tests go red`, `journeys.sh`, the `ci/` doc checks), then the
row binary (`22 tests run: 22 passed`), the sidecar identity, the shared-predicate
greps and the absence of `std::env::args()`. It ends `GATE PASS`. What it does
not cover is stated in its header (wording, the docs-only CI job, release
workflow, manual). Shown RED at 1f6cc6b with the rows and toml diff applied:
`424 tests run: 408 passed, 16 failed`, `GATE FAIL: nextest`.

## 11. Open points for the architect (no STOP)

- **O1.** N1, T1, D1, U1 and the two help lines (PROPOSED), the `§10.10`
  section for T1/D1/U1, and N1's `§1.1e`.
- **O2.** H4's argv[0] rule (lossy, never refused) and the verb fallback to
  `encode` for an untyped verb (mirrors `command_line_guard`). U1 is mt's first
  exit 64; every other mt refusal exits 1 (T22 names 64 explicitly).
- **O3.** Not folded, a sibling of I12: `mt encode --allow-argv-secret <hex> -`
  with `garbage` on stdin exits 0, encodes the argv material and ignores stdin
  with no notice (run at 1f6cc6b). T13 covers `--in` with `-` only. Filed in the
  author's report as a proposed FOLLOWUPS line.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

One line owed to this build (closed by its GREEN commit) and one new line (O3).

## 13. Files this FIX touches (for build ordering against row 23, FIX-F)

`crates/mt-cli/src/main.rs` (main, encode/decode/verify/inspect openings,
`separator_guard`, two new guards, two help comments),
`crates/mt-cli/src/read_strings.rs` (`read` line 64, the new predicate),
`crates/mt-cli/src/validate.rs` (new `utf8_argv_guard`),
`crates/mt-cli/tests/refusals.toml` (append at end), new
`crates/mt-cli/tests/arg_intake.rs`.
