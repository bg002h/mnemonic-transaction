# Changelog

All notable changes to `mt-cli` and `mt-codec` are documented in this file.
Each release entry is prefixed with the crate name (`## mt-cli [0.2.0]`,
`## mt-codec [0.2.0]`).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [SemVer](https://semver.org/spec/v2.0.0.html) with the
pre-1.0 convention that the second component (`0.X`) is the breaking-change
axis.

Releases 0.1.0 and 0.1.1 are described by their tags (`mt-cli-v0.1.0`,
`mt-cli-v0.1.1`); this file starts at 0.2.0.

## mt-cli [Unreleased]

### Changed
- **Exit codes follow the constellation exit-code spec**
  (mnemonic-engrave `design/SPEC_constellation_exit_codes.md`, revision 5, all
  eleven decisions as recommended). BREAKING for scripts that branch on the
  code. Every refusal carries its class (`Refusal::new` takes it), and
  `tests/refusals.toml` gains a `class` per row that
  `scripts/check-refusal-coverage.sh` checks against the row's test (R13).
  - Usage is 64: clap errors (2 before), the §10.10 flag-combination refusals
    and a bad `--separator` (1 before; the separator is now checked before any
    input is read, R2). Non-UTF-8 argv stays 64.
  - Invalid input is 2 (1 before): garbage, an invalid or uncorrectable set,
    an ms1, a pasted txid, empty input, the §8.1/§8.2b/c/d/e, §8.3, §8.6 and
    §8.7b correctness refusals.
  - Refused is 8 (1 before): bearer material on argv (§8.2f), a world-readable
    stdout (§8.2h), the same file in and out (§8.2i).
  - No is 1 (unchanged): `verify --transaction` mismatch, a node-proved spent
    input (§8.5), a value that disagrees with the chain (§6a).
  - IO is 32: an input that cannot be read (1 before), and a failed stdout or
    `--out` write, which now ADDS 32 to the verb's code instead of turning 0
    into 1 (R8). A `--bitcoin-cli` the operator named that runs and fails adds
    32 (0 before, R9); an absent path is still the offline path, exit 0.
  - Repaired is 16: `decode`, `verify` and `inspect` that used a BCH-corrected
    chunk now set 16 (0 before, decision 5), with the same output. Bits sum: a
    `verify --transaction` mismatch on a corrected set is 17.
  - A codec failure that the checks before it make unreachable is 101.
  - The root `--help` ends with the exit-status legend (R11).
- **FIX-J: printed remedies (bh-mt-I22, I25).** The §8.1 and
  §10.10 remedies tell the operator to pass the `hex` field of `finalizepsbt`'s
  answer, not the whole answer. `scripts/live-smoke-test.sh` sets `umask 077`
  and prints a failing step's stderr.

## mt-cli [0.2.0] — 2026-11-04

Fixes from the mt bug hunt (FIX-A to FIX-D).

### Changed
- **FIX-A: signature shape in `mt encode` (SPEC §8.6).** A signature-shaped
  element is judged by where it sits (Taproot or ECDSA), not by its length. A
  two-element witness is a Taproot script path (BIP-341: two or more elements).
  The scriptSig walk counts every push opcode (`OP_0`, `OP_PUSHDATA4`,
  `OP_1NEGATE`, `OP_1` to `OP_16`).
- **FIX-C: refusals.** `--json` together with `--quiet` on `decode` is refused,
  and `--quiet` on `inspect` and `verify` is refused.
- **FIX-C: JSON output.** `decode` and `inspect` JSON warnings carry the
  duplicate and correction notices; `render_json` escapes control characters
  U+0000 to U+001F.
- **FIX-D: `getindexinfo`.** `has_txindex` parses the node's `getindexinfo`
  reply as JSON; only txindex's own `synced: true` counts, and any parse
  failure, other index or other value reads as unknown. `serde_json` is now a
  runtime dependency.

### Fixed
- **A closed stdout (`mt decode ... >&-`) no longer exits 0 with the output lost.** std's runtime silently reopens a closed fd 0, 1 or 2 on `/dev/null` before `main`, so a write to it "succeeded". `mt` now notes a closed fd 1 before that and plugs each closed standard fd itself (so a file `mt` opens later, such as an `--out` file, cannot become stdout); a verb that writes to stdout (and `--help`/`--version`) prints `mt: cannot write the output: Bad file descriptor (os error 9)` and exits 1, as for a read-only stdout. `encode --out` with stdout closed is unaffected (exit-code survey 2026-11-26, Defect 1).
- **FIX-B: chunk-set checks.** Chunks of one set that disagree about the set's
  chunk count are refused (`verify` checks the same count in every chunk), and
  `invariant_prefix` returns `TooShort` for input under 24 symbols.
- **FIX-B: non-ASCII input.** `decode_string` rejects non-ASCII in the data part
  before lowercasing.
- **FIX-C:** `restore_elided` no longer panics on a non-ASCII cut point (it
  refuses the input as unreadable), and the `mtl`/`mti` prefix repair is kept
  when autocorrect then decodes.

### Dependencies
- `mt-codec` 0.2.0 (see below).

## mt-codec [0.2.0] — 2026-11-04

### Changed — BREAKING
- **`Error::CountMismatch { expected, found, index }` is a new variant of the
  public `Error` enum, which is not `#[non_exhaustive]`;** an exhaustive
  `match` on `mt_codec::Error` no longer compiles. `decode` returns it when the
  chunks of one set disagree about how many chunks the set has.

### Fixed
- `decode_string` rejects non-ASCII in the data part before lowercasing, and
  `invariant_prefix` returns `TooShort` under 24 symbols (FIX-B).

mt-codec is not published to crates.io; `mnemonic-engrave` takes it by git rev.
