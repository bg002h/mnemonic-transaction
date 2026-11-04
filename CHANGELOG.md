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
