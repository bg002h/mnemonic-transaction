# FIX: output writes (bughunt-1119 FIX table row 12, "FIX-B output writes")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-24. Keys and final grades: bh-mt-C2 (**I**;
the hunter and the mt grader said C, the Fable review regraded it I, OQ3), bh-mt-C3 (**I**;
the consultant said C, upheld I, OQ2), bh-mt-I4 (**I**). Track: small (one Opus review
round). Not a funds-path (class B) row. Base: origin/main `3993d0c`. Branch
`fix/mt-output-writes`. Every new or changed holder-visible text is marked **PROPOSED
(architect confirms)**; everything else below is decided by the rulings in section 2.
No new flag, type visible to a holder, or command.

Not to be confused with the older `design/FIX_bughunt_mt_b.md` (a different hunt, also
called "FIX-B").

## 1. Findings (run by the author at 3993d0c; `main.rs` anchors are at 3993d0c)

Run offline (`--bitcoin-cli /nonexistent/bitcoin-cli`) on the corpus vector `even`
(`crates/mt-codec/src/test_vectors/mt1_v1.json`), debug build, uid 1000:

- **C2, stdout.** `mt encode --in tx.hex > /dev/full` exits 0; so do `decode`, `decode
  --json`, `inspect` (strings and transaction paths, text and `--json`), `mt --help`,
  `mt decode --help` and `mt --version`. Every stdout write discards its result:
  `emit` (`main.rs:912-916`), decode's hex (`:1590-1599` under `--json`, `:1659-1668`),
  inspect's report (`:1909-1919` for a transaction, `:2042-2052` for strings), and
  clap's own help/version printing (`Cli::parse_from`, `:332`). A stdout opened
  read-only (`1</dev/null`) loses the output at exit 0 too: the kernel answers EBADF
  and std's stdout reports EBADF as a successful write.
- **C3, `--out`.** mt-cli pins mnemonic-io-lib at `d8fec678` (0.1.1, `crates/mt-cli/Cargo.toml:29`),
  whose `write_private` truncates in place. Under `ulimit -f 0` (SIGXFSZ ignored) an
  existing 4-byte `--out` file is left at **0 bytes**, exit 1. stderr prints `WARNING: the
  strings went into s2.txt — and they are BEARER ...` (`main.rs:862-866`, printed BEFORE
  the write) and then `mt encode: REFUSED — §6b, cannot write s2.txt` with the mechanism
  `... File too large (os error 27). Nothing was engraved and nothing was left behind.`
  (`:871-883`). Both claims are false. An absent target under the same limit is left as
  an empty 0600 file.
- **I4, output names an input.** `cp tx.hex src.hex; mt encode --in src.hex --out
  src.hex` exits 0 and `src.hex` now begins `mt1p9h8jqq9q`: the source is gone. The same
  happens with `--out` naming stdin's file, with a hard link or symlink to `--in`, and
  with stdout appended (`>>`) to `--in` on `encode`, `decode` and `inspect`. No identity
  check exists (`main.rs:366-420`, `:1510-1512`, `:1968-1970`).
- Pre-existing, unchanged by this FIX: `--out /dev/null` fails with `Operation not
  permitted (os error 1)` for a non-root user (io-lib's DIRECT path fchmods the handle;
  FOLLOWUPS line `iolib-direct-chmod-non-regular` in mnemonic-engrave). After this FIX it
  prints the new line of rule W3 instead of the old refusal.

## 2. Rulings (binding, quoted or cited)

- **T5** (`coordinator/agent-reports/architect-bughunt-1119-questions.md:41`): io-lib
  `write_private` becomes temp file + rename ("on any error remove the temp and leave the
  old file byte-identical"; a non-regular target keeps the direct write); `same_file(out,
  &[inputs])` "(dev+ino on Unix; stdout by fstat of fd 1) for the alias rows ... mt-I4";
  then "pin bumps ... ms, mt (git rev d8fec67 today)".
- **T6** (same report `:42`): "a closed pipe stops output and keeps the verb's own code;
  any other stdout or `--out` write error prints `<tool>: cannot write the output:
  <error>` and exits 1; a verb already failing keeps its code. A verdict printed on
  stderr (mt verify) is carried by the exit code, so a failed stderr write does not
  change it." md's published form (toolkit manual `42-md.md:799`) applies it to "a verb
  or `--help`".
- **Brian 2026-11-23, file-output Q2** (`coordinator/memory/file-output-convention-rulings-2026-11-23.md`):
  "output names an input (same path, symlink or hard link): refuse, exit non-zero,
  naming both ("write to a new name"); io-lib `same_file` gets callers."
- **File-output convention R5** (`coordinator/agent-reports/architect-file-output-convention-1123.md`
  section 3; its unit U3 is this code): "Nothing says "wrote" or "went into" before the
  write returns. A failure names the path and the OS error and says what is on disk:
  "<path> is unchanged" when a file was there, "nothing was written" when none was."
- **Overrides 2026-11-24** (`coordinator/briefs/bh1119-batch-1124-overrides.md`): io-lib
  0.1.2 is not on crates.io (`cargo search` shows 0.1.0), so pin the git rev the way
  mt-cli already does.

## 3. Behaviour after the fix (rules)

**P1 Pin (T5).** `crates/mt-cli/Cargo.toml:29` pins
`rev = "e09d538dcd0b552cdc58fe7ff228e2eca5f7f64e"` (mnemonic-engrave master, the merge of
#41; `crates/mnemonic-io-lib` there is version 0.1.2 and byte-identical to the fold
commit `32745703`). The comment's last paragraph (`:26-28`) becomes: `The rev is
mnemonic-engrave master e09d538d, the merge of #41, which carries mnemonic-io-lib 0.1.2
(write_private replaces through a temp file and a rename; same_file). Both repositories
are public, so no deploy key is needed.` `Cargo.lock` follows (`cargo update -p
mnemonic-io-lib`), one entry.

**S1 One stdout writer (T6).** Every byte mt writes to stdout goes through one writer
that writes and then flushes, and checks both results: encode's strings and `tx:`
record, decode's hex (both arms), inspect's report (transaction and strings paths, text
and `--json`), and clap's `--help`, `<verb> --help` and `--version` output.
- **S1a** The first failed write stops every later stdout write of the run.
- **S1b** A closed pipe (`ErrorKind::BrokenPipe`) prints nothing and the exit code is
  the verb's own.
- **S1c** Any other error: the verb's remaining stderr output is unchanged, and the last
  line mt prints on stderr is `mt: cannot write the output: <error>` (`<error>` is the
  `std::io::Error` Display, e.g. `No space left on device (os error 28)`), printed once.
  The exit code becomes 1 when the verb would have exited 0; a verb that exits non-zero
  keeps its code (mt verbs write stdout only on success, so this arises only in rule).
- **S1d** A stdout whose access mode is read-only (`fcntl(1, F_GETFL) & O_ACCMODE` is
  neither `O_WRONLY` nor `O_RDWR`) is checked at the first stdout write and is error
  EBADF: `mt: cannot write the output: Bad file descriptor (os error 9)`, exit 1. A verb
  that writes nothing to stdout never checks. A closed fd 1 (`>&-`) is not detected
  (fcntl fails; md's precedent, `md-cli/src/out.rs` `read_only`).
- **S1e** stderr write failures are ignored and never change the exit code. This covers
  verify's verdict (T6) and decode `--json`'s document, which is on stderr.
- **S1f** clap usage errors keep clap's stderr text and exit 2. Help and version text is
  byte-identical to today's when the write succeeds.

**W1 `--out` writer (T5).** `encode --out` writes through io-lib 0.1.2 `write_private`:
an absent path or an existing regular file (symlinks followed) is written to a sibling
temp file and renamed over the name; any error leaves the old file's bytes as they were
and removes the temp. Anything else (a device, a FIFO, a directory, `/dev/stdout`) is
written directly, as today.

**W2 Truthful order (R5).** The `--out` warning (`blocks::redirected_output_warning(form,
Some(path))`, the `... went into <path> ...` block) is printed only after `write_private`
returns Ok. Its text is unchanged. On failure it is not printed.

**W3 Failure text (T6 + R5).** Before the write, mt notes what is at the path
(`std::fs::metadata`, which follows symlinks): a regular file, absent (`NotFound`,
including a dangling symlink), or other (any other kind, or any other metadata error).
On a write error mt prints, as the last stderr line, and exits 1:
- regular file before: `mt: cannot write the output: <path>: <error>. <path> is unchanged.`
- absent before: `mt: cannot write the output: <path>: <error>. Nothing was written.`
- other: `mt: cannot write the output: <path>: <error>`

`<path>` is `--out` as typed (`Path::display()`); `<error>` is the io error's Display.
No refusal block, no remedy. The old §6b refusal (`main.rs:871-883`) and its two
sentences "Nothing was engraved and nothing was left behind." and "Check the directory
exists and is writable, then re-run. Without --out the artifact goes to stdout." are
removed. "Unchanged" is about the file's bytes: io-lib's preflight may already have set
an existing target to 0600 before the failing step (T5/F-244), which is not a change of
content.

**Q1 The output is never one of the inputs (Q2, T5 `same_file`).** New guard
`validate::same_file_guard(verb, output, inputs) -> Result<(), Refusal>` calling
`mnemonic_io_lib::write::same_file`.
- **Q1a** Endpoints. `encode`: the output is `--out <path>` when given, else standard
  output. `decode`, `inspect`: the output is standard output. The input is `--in <path>`
  when given; else standard input when no `--allow-argv-secret` material was admitted;
  else there is no input to compare and the guard passes. `verify` writes nothing to
  stdout and is not checked. With `--out` given, `encode`'s stdout is not an output and
  is not checked.
- **Q1b** Identity is io-lib's: dev+ino of the file a path resolves to (symlinks
  followed, hard links equal); an absent path by its canonical directory plus name, so
  `--in x --out x` with `x` absent is the same file; a pipe, terminal or device never
  matches.
- **Q1c** Order. In each verb the guard runs right after the verb's argument-only
  guards (`encode`: `json_unsupported_guard`; `decode`: `json_quiet_guard`; `inspect`:
  `quiet_unsupported_guard`) and before any input is read or any warning is printed.
  The pre-clap `command_line_guard` still runs first.
- **Q1d** A match refuses with §8.2i (section 4), exit 1, and nothing is read or written.

## 4. Holder-visible texts

**T6 line (decided by T6, not PROPOSED):** `mt: cannot write the output: <error>`.

**W3 lines (PROPOSED, architect confirms; T6's prefix with R5's path and state):**

```text
mt: cannot write the output: <path>: <error>. <path> is unchanged.
mt: cannot write the output: <path>: <error>. Nothing was written.
mt: cannot write the output: <path>: <error>
```

**§8.2i refusal (PROPOSED, architect confirms).** Rendered by `Refusal`'s Display
(`refusal.rs:78-103`): first line `mt <verb>: REFUSED — §8.2i, <verdict>`, then the
mechanism and remedy wrapped at 68 columns.

```text
verdict:   <output> is the same file as <input>
           <output> = "--out <path>" | "standard output"
           <input>  = "--in <path>"  | "standard input"
mechanism (output is --out):
           They are one file (the same path, a symlink or a hard link), so writing the output would replace the input mt is reading.
mechanism (output is standard output):
           They are one file (the same path, a symlink or a hard link), so the output would go into the file mt reads its input from. A > redirect empties that file before mt starts, so check that it still holds what you expect.
remedy:    Write to a new name.
```

The stdout mechanism says what a `>` did because the shell truncates the file before mt
runs; mt cannot undo that and should not let the holder believe the file is intact.

## 5. Spec, README, help, manual, CHANGELOG

- **Spec.** Apply directly to mt's `design/SPEC_mt_v0_1.md` in the GREEN commit
  (precedent: `FIX_bh1119_mt_verdict_notice.md` section 5; mnemonic-engrave's copy is
  behind and has no `--out` paragraph). List the deltas in the build report for porting
  to mnemonic-engrave. PROPOSED text:
  1. §8.2h, after the paragraph ending "re-running a command is the case an operator
     actually hits (F-244)." (`:2610-2614`), insert: `Since mnemonic-io-lib 0.1.2 that
     write goes to a temp file beside the target and is renamed over it, so a failed
     write leaves an existing file's bytes as they were and creates nothing at a new
     name. The `--out` warning is printed only after the write succeeded. A failed
     write prints `mt: cannot write the output: <path>: <error>`, followed by `. <path>
     is unchanged.` when a file was there or `. Nothing was written.` when none was, and
     exits 1.`
  2. A new item between 2h and 3 (before `:2687`):
     `2i. **The output is one of the inputs** → **refuse**, before anything is read.
     The output is `encode`'s `--out` (else stdout) and the stdout of `decode` and
     `inspect`; the input is `--in` (else stdin). Same file means the same path, a
     symlink or a hard link (device and inode), or two absent paths naming one place.
     `verify` writes no stdout and is not checked. Refusal text: FIX_bh1119_mt_output_writes
     section 4.`
  3. After 2i: `**A failed stdout write.** A reader that closes the pipe early only
     stops the output; the exit code is the verb's own. Any other stdout write error,
     from a verb or from `--help`/`--version` (a full disk, a read-only stdout), prints
     `mt: cannot write the output: <error>` on stderr and exits 1. A failed stderr write
     changes nothing: `verify`'s verdict is its exit code.`
- **README** (PROPOSED): after the paragraph ending "to parse prose out of its own
  input." (`README.md:21-24`) add: `If stdout cannot be written (a full disk), mt prints
  \`mt: cannot write the output: <error>\` and exits 1; a reader that closes the pipe
  early only stops the output. mt refuses to write its output into one of its own
  input files.`
- **Help:** no change (the `--out` doc comment `main.rs:140-158` stays true).
- **Toolkit manual:** none. `mnemonic-toolkit` master `docs/manual/src/40-cli-reference/`
  has no `mt` page (41-mnemonic, 42-md, 43-ms, 44-mk-cli, 45-mr, 46-mr-sitting).
- **CHANGELOG:** none in this FIX (entries are written at release).

## 6. RED rows

All 33 rows are one new test binary `crates/mt-cli/tests/bh1119_mt_output_writes.rs`
(`mt-cli::bh1119_mt_output_writes`); its exact text is the sidecar
`design/FIX_bh1119_mt_output_writes/output_writes.rs.txt`. Run by the author at
`3993d0c` with the sidecar copied into `tests/`:

`cargo nextest run --locked --build-jobs 6 -j 6 -p mt-cli --test bh1119_mt_output_writes --no-fail-fast`
→ `33 tests run: 8 passed, 25 failed` (the 25 RED rows fail, the 8 guards pass).

Shell forms: `F` is `/dev/full`; `L0` is `bash -c 'trap "" XFSZ; ulimit -f 0; exec "$@"'
bash mt ...`; every encode/decode/inspect run has `--bitcoin-cli /nonexistent/bitcoin-cli`.

| row (`bh1119_mt_` omitted) | command | today (3993d0c, seen) | after |
|---|---|---|---|
| c2_encode_stdout_full_exits_1 | `mt encode --in tx.hex >F` | exit 0 | exit 1, last line `mt: cannot write the output: No space left on device (os error 28)` |
| c2_encode_qr_stdout_full_exits_1 | `mt encode --qr --in tx.hex >F` | exit 0 | as above |
| c2_decode_stdout_full_exits_1 | `mt decode --in s.txt >F` | exit 0 | as above |
| c2_decode_json_stdout_full_exits_1 | `mt decode --json --in s.txt >F` | exit 0 | as above |
| c2_inspect_strings_stdout_full_exits_1 | `mt inspect --in s.txt >F` | exit 0 | as above |
| c2_inspect_strings_json_stdout_full_exits_1 | `mt inspect --json --in s.txt >F` | exit 0 | as above |
| c2_inspect_transaction_stdout_full_exits_1 | `mt inspect --in tx.hex >F` | exit 0 | as above |
| c2_help_stdout_full_exits_1 | `mt --help >F` | exit 0 | as above |
| c2_subcommand_help_stdout_full_exits_1 | `mt decode --help >F` | exit 0 | as above |
| c2_version_stdout_full_exits_1 | `mt --version >F` | exit 0 | as above |
| c2_decode_read_only_stdout_exits_1 | `mt decode --in s.txt 1</dev/null` | exit 0 | exit 1, last line `mt: cannot write the output: Bad file descriptor (os error 9)` |
| c2_decode_closed_pipe_keeps_exit_0 (guard) | decode into a pipe whose read end is closed (python3 harness) | exit 0, no line | same |
| c2_verify_stderr_full_keeps_exit_0 (guard) | `mt verify --in s.txt 2>F` | exit 0 | same |
| c2_decode_json_stderr_full_keeps_exit_0_and_the_hex (guard) | `mt decode --json --in s.txt 2>F` | exit 0, hex on stdout | same |
| c2_refused_verb_keeps_exit_1_and_says_no_write_line (guard) | `mt decode --in g.txt >F` (g = "not a transaction") | exit 1, REFUSED, no line | same |
| c2_usage_error_keeps_exit_2 (guard) | `mt decode --no-such-flag >F` | exit 2 | same |
| c3_failed_replacement_keeps_the_previous_file | `L0 mt encode --in tx.hex --out strings.txt` (holds `OLD BACKUP\n`) | exit 1, strings.txt `""` | exit 1, strings.txt `OLD BACKUP\n`, dir holds only strings.txt and tx.hex |
| c3_failed_replacement_says_the_file_is_unchanged | same | last line `--out the artifact goes to stdout.` (old refusal) | last line `mt: cannot write the output: <p>: File too large (os error 27). <p> is unchanged.`; no `left behind`, `went into`, `REFUSED` |
| c3_failed_new_file_leaves_nothing | `L0 mt encode --in tx.hex --out strings.txt` (absent) | old refusal; empty 0600 file left (seen by hand) | last line `... <p>: File too large (os error 27). Nothing was written.`; dir holds only tx.hex |
| c3_missing_directory_says_nothing_was_written | `mt encode --in tx.hex --out <dir>/no-such-dir/strings.txt` | old refusal | last line `mt: cannot write the output: <p>: No such file or directory (os error 2). Nothing was written.` |
| c3_device_target_names_no_state | `mt encode --in tx.hex --out /dev/full` | old refusal (EPERM) | exit 1, last line starts `mt: cannot write the output: /dev/full: `, ends `)`, no state sentence |
| c3_successful_replacement_writes_0600_and_says_so (guard) | `mt encode --in tx.hex --out strings.txt` over a 0644 file | exit 0, `went into`, 0600, bytes = piped stdout | same |
| i4_encode_in_equals_out_refused | `mt encode --in tx.hex --out tx.hex` | exit 0, tx.hex replaced | §8.2i `--out <p> is the same file as --in <p>`, tx.hex unchanged |
| i4_encode_out_hard_link_to_in_refused | `--out hard.txt` (hard link to tx.hex) | exit 0 | §8.2i `--out <link> ... --in <tx>` |
| i4_encode_out_symlink_to_in_refused | `--out sym.txt` (symlink to tx.hex) | exit 0 | §8.2i `--out <link> ... --in <tx>` |
| i4_encode_out_equals_stdin_refused | `mt encode --out tx.hex < tx.hex` | exit 0 | §8.2i `--out <p> is the same file as standard input` |
| i4_encode_stdout_appends_to_in_refused | `mt encode --in tx.hex >> tx.hex` | exit 0 | §8.2i `standard output is the same file as --in <p>` (stdout mechanism) |
| i4_decode_stdout_appends_to_in_refused | `mt decode --in s.txt >> s.txt` | exit 0 | §8.2i, `decode` |
| i4_inspect_stdout_appends_to_in_refused | `mt inspect --in s.txt >> s.txt` | exit 0 | §8.2i, `inspect` |
| i4_decode_stdin_and_stdout_one_file_refused | `mt decode < s.txt >> s.txt` | exit 0 | §8.2i `standard output is the same file as standard input` |
| i4_refused_before_the_input_is_read | `mt encode --in g.txt --out g.txt` (g = "not a transaction") | first line `mt encode: REFUSED — §8.2e, input is not a PSBT or a raw transaction (17 bytes)` | §8.2i first line |
| i4_distinct_out_is_written (guard) | `mt encode --in tx.hex --out strings.txt` | exit 0 | same |
| i4_verify_is_not_checked (guard) | `mt verify --in s.txt >> s.txt` | exit 0, s.txt unchanged | same |

Every §8.2i row checks the whole refusal (first line exact, the rest with whitespace
collapsed) and that the input file is byte-identical afterwards.

## 7. Tests that change

- None of the existing tests change. No test pins the old §6b write refusal or its text
  (`git grep -n "left behind\|cannot write" crates/` at 3993d0c: only `main.rs`).
  `encode.rs::out_suppresses_the_world_readable_stdout_gate` (`:625`) gives `--out` and a
  0644 stdout file that is not an input: unaffected by Q1a.
- New: `crates/mt-cli/tests/bh1119_mt_output_writes.rs` (the sidecar, 33 rows).
- `crates/mt-cli/tests/refusals.toml` ends with `refusals-toml-append.txt` (9 §8.2i
  entries, check `src/validate.rs::same_file_guard`).
- `scripts/check-refusal-coverage.sh` `REQUIRED` gains `"§8.2i",  # the output is one
  of the inputs (FIX_bh1119_mt_output_writes)` after `"§8.2h"`. The script's purpose is
  unchanged, so `scripts/README.md` and the catalog row do not change.

## 8. Build order

1. RED commit: the sidecar as `tests/bh1119_mt_output_writes.rs`, the `refusals.toml`
   append, the `REQUIRED` line. Run the row binary: expect `33 tests run: 8 passed, 25
   failed` (as section 6).
2. GREEN commit(s): P1 pin and lock; S1 writer (all stdout sites and clap help/version);
   W1-W3 in encode's `--out` arm; Q1 guard in encode, decode, inspect; spec deltas and
   README (section 5). Run the row binary: `33 tests run: 33 passed`.
3. The gate (section 10) once on the final head.

## 9. Coverage map (rule -> rows; `bh1119_mt_` omitted)

| rule | rows |
|---|---|
| P1 pin | gate pin lines; c3_failed_replacement_keeps_the_previous_file, c3_failed_new_file_leaves_nothing (fail on 0.1.1) |
| S1 every stdout site | c2_encode_stdout_full, c2_encode_qr_stdout_full, c2_decode_stdout_full, c2_decode_json_stdout_full, c2_inspect_strings_stdout_full, c2_inspect_strings_json_stdout_full, c2_inspect_transaction_stdout_full, c2_help, c2_subcommand_help, c2_version |
| S1a once, S1c line and exit 1 | every C2 RED row (`assert_write_failure`: exit 1, last line, printed once) |
| S1b closed pipe | c2_decode_closed_pipe_keeps_exit_0 |
| S1c failing verb keeps code | c2_refused_verb_keeps_exit_1_and_says_no_write_line |
| S1d read-only stdout | c2_decode_read_only_stdout_exits_1 |
| S1e stderr ignored | c2_verify_stderr_full_keeps_exit_0, c2_decode_json_stderr_full_keeps_exit_0_and_the_hex |
| S1f usage error | c2_usage_error_keeps_exit_2 |
| W1 atomic replace, no temp left | c3_failed_replacement_keeps_the_previous_file, c3_failed_new_file_leaves_nothing |
| W2 warning only after success | c3_failed_replacement_says_the_file_is_unchanged, c3_failed_new_file_leaves_nothing, c3_successful_replacement_writes_0600_and_says_so |
| W3 regular / absent / other | c3_failed_replacement_says_the_file_is_unchanged / c3_failed_new_file_leaves_nothing, c3_missing_directory_says_nothing_was_written / c3_device_target_names_no_state |
| Q1a endpoints (encode --out vs --in, vs stdin; stdout vs --in; stdout vs stdin; decode; inspect; verify not checked) | i4_encode_in_equals_out, i4_encode_out_equals_stdin, i4_encode_stdout_appends_to_in, i4_decode_stdin_and_stdout_one_file, i4_decode_stdout_appends_to_in, i4_inspect_stdout_appends_to_in, i4_verify_is_not_checked |
| Q1b identity (hard link, symlink, distinct name) | i4_encode_out_hard_link_to_in, i4_encode_out_symlink_to_in, i4_distinct_out_is_written |
| Q1c before any read | i4_refused_before_the_input_is_read |
| Q1d text and exit | every i4 RED row (`assert_same_file_refusal`) |

Not held by a row: S1d's "closed fd 1 is not detected" (a decision to do nothing);
Q1b's "two absent paths" case (io-lib's own test S-rows hold `same_file` on Planned
paths); W3's "unchanged" when io-lib's preflight tightened the mode.

## 10. Gate (builder, final head; run once to a file, then grep)

`bash design/FIX_bh1119_mt_output_writes/gate.sh > /scratch/cache/mt-output-writes-gate.log 2>&1; grep -E "GATE (PASS|FAIL)" /scratch/cache/mt-output-writes-gate.log`

The gate checks the pin, the sidecars, fmt, clippy, build, `556 tests run: 556 passed`
(523 at 3993d0c + 33), the 33 rows, doc tests, `check-refusal-coverage: 76 refusal
tests over 19 ruled refusals` (67 over 18 at 3993d0c + 9 over §8.2i), `mutate-refusals:
all 76 refusal tests go red`, journeys, the ci self-tests, no `left behind` in src, and a
`same_file(` caller. A head that also carries another row (for example row 21) adds that
row's counts. Its "does not cover" list is in the script header.

## 11. Open points for the architect (no STOP)

1. **Pin rev.** `e09d538d` (the #41 merge on master) rather than the version-bump commit
   `30b0f758`, which predates the fold `32745703` (ENOTSUP fsync fallback). The io-lib
   tree is identical at `e09d538d`, its second parent `512f8e6c` and `32745703`.
   Rows 9-14 (md, mk, ms, tk, me) should pin the same rev; ms also pins by git rev.
2. **W3 wording.** T6 gives `<tool>: cannot write the output: <error>`; R5 asks for the
   path and the on-disk state. The proposal keeps T6's prefix and puts `<path>: ` before
   the error and the R5 sentence after it. The old refusal's remedy is dropped.
3. **S1e.** decode `--json`'s document is on stderr, so a failed write of it changes
   nothing (read from T6's scope: stdout and `--out`). The mt grader asked that "the
   JSON channel" be covered; inspect `--json` (stdout) is covered.
4. **Q1a scope.** Q2 is applied to stdout as an output (T5 names "stdout by fstat of
   fd 1"), so `decode`/`inspect ... >> input` are refused, beyond I4's `encode` case.
5. **Q3 replace notice** (Brian 11-23: one stderr line naming a replaced `--out` file)
   is not in this row: it needs io-lib 0.1.3's `Written` outcome (convention U1/U3).
   Proposed FOLLOWUPS line in the author's report.

## 12. FOLLOWUPS (listed in the author's report, not appended here)

## 13. Files this FIX touches (for build ordering against other mt rows)

`crates/mt-cli/Cargo.toml`, `Cargo.lock`, `crates/mt-cli/src/main.rs` (stdout sites,
clap parse, encode `--out` arm, the guard calls), `crates/mt-cli/src/validate.rs`
(`same_file_guard`), one new module for the stdout writer if the builder wants one,
`crates/mt-cli/tests/bh1119_mt_output_writes.rs` (new), `crates/mt-cli/tests/refusals.toml`,
`scripts/check-refusal-coverage.sh`, `design/SPEC_mt_v0_1.md`, `README.md`.
