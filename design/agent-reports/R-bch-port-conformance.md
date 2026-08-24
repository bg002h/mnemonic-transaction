# R-bch-port-conformance — mt-codec's BCH layer vs. mk-codec upstream

**Reviewer:** independent sonnet agent, wrote none of this code.
**Scope:** `crates/mt-codec/src/string_layer/bch.rs` and `bch_decode.rs` only
(header code out of scope per brief). Question: does mt-codec's BCH layer
behave identically to the mk-codec code it was ported from, except where
`mt1` deliberately differs?

## Verdict

**0 Critical / 0 Important / 2 Minor.** The port behaves identically to its
upstream. Every shared table/constant is byte-identical, every algorithmic
function body (bech32 alphabet handling, polymod, syndrome computation,
Berlekamp–Massey, Chien search, Forney correction) is textually identical
between the two crates apart from the deliberate `MK_*_CONST` →
`MT_*_CONST` identifier rename, and a cross-run harness against real
`mk-codec` v0.5.0 and `mt-codec` confirms identical correction behavior
(positions, magnitudes, correction counts) for t = 0..5 on both the regular
and long codes, including agreement at the t = 5 refusal boundary.

## Upstream located at

`/scratch/code/shibboleth/mnemonic-key/crates/mk-codec` (package `mk-codec`
v0.5.0). Confirmed as the correct, current primary repo:

- `origin` = `git@github.com:bg002h/mnemonic-key.git`, branch `main`, clean
  working tree.
- `Cargo.toml` version is `0.5.0`, which **exactly matches** the provenance
  pin recorded in `mt-codec`'s `design/PROVENANCE.md` and in
  `string_layer/mod.rs`'s module doc ("Provenance pin: `mk-codec` 0.5.0").
- Ruled out two decoys under the same GitHub repo: `mk-v010-cross-update/`
  is a second worktree of the same `bg002h/mnemonic-key.git` remote, checked
  out on branch `feature/md-codec-v0.10-shipped-cross-update` at
  `mk-codec` **0.1.1** — an older snapshot, not the pinned version, and not
  `main`. `mnemonic-toolkit/vendor/mk-codec` is a vendored copy inside an
  unrelated sibling repo, not the primary.

## Substantive differences

| what | mt | mk | deliberate or DEFECT |
| --- | --- | --- | --- |
| HRP | `"mt"` | `"mk"` | deliberate (spec) |
| Target residues | `MT_REGULAR_CONST` / `MT_LONG_CONST` | `MK_REGULAR_CONST` / `MK_LONG_CONST` | deliberate (per-format NUMS constants) |
| `GEN_REGULAR`, `GEN_LONG`, `POLYMOD_INIT`, `REGULAR_SHIFT/MASK`, `LONG_SHIFT/MASK`, `ALPHABET` | identical values | identical values | N/A — shared BIP-93 constants, verified element-by-element |
| `BETA`, `GAMMA`, `ZETA` (`Gf1024`) | identical values | identical values | N/A — shared field constants, verified |
| All production function bodies (`polymod_run`, `polymod_step`, `hrp_expand`, `bytes_to_5bit`/`five_bit_to_bytes`, `case_check`, `bch_code_for_length`, `bch_create_checksum_*`, `bch_verify_*`, `bch_correct_*`, `encode_5bit_to_string`, `decode_string`, syndrome computation, Berlekamp–Massey, Chien search, Forney) | textually identical | textually identical | N/A — confirmed by diff, 0 code hunks beyond the const rename |
| Module-level algorithm derivation doc (syndrome formulas, BM/Chien/Forney math, `GF(1024)` root-set tables, position-indexing derivation) in `bch_decode.rs` | deleted, replaced with a pointer to `bch.rs`'s provenance note | ~80-line derivation retained | port-time doc loss (Minor 1) |
| Unit-test modules in `bch.rs` (34 tests) and `bch_decode.rs` (13 tests) | deleted wholesale, no direct successor at unit granularity | present | port-time test-coverage loss (Minor 2) |

## Findings

### Minor 1 — BCH decoder algorithm derivation doc deleted, not carried forward

**Where:** `bch_decode.rs` module doc, both repos (hunk `@@ -1,85 +1,9 @@`).

**What:** mk-codec's `bch_decode.rs` opens with an ~80-line derivation:
syndrome formula `S_m = E(α^{j_start-1+m})`, why `j_start = 77` (regular) /
`1019` (long), the full root sets for `g_regular`/`g_long`, the Forney
shifted-form justification (`X_k^{1-j_start}` factor, citing Lin & Costello
§6.3), and the position-indexing convention (`k = (L-1) - d`). mt-codec's
version deletes all of it, replacing it with a short pointer to `bch.rs`'s
provenance note (which explains *where the code came from*, not *why the
math is correct*).

**Is it in mk too?** N/A — this is not a defect present in mk-codec; it is
something mt-codec's port *removed* that mk-codec still has. Nothing to fix
upstream.

**How I checked:** read both module docs in full (diff hunk `@@ -1,85 +1,9
@@` in `bch_decode.rs`); the deleted text is reproduced verbatim in mk's
file today.

**Impact:** none on behavior — the code the doc describes is byte-identical
in both crates (see next section). This is a maintainability gap: a future
mt-codec maintainer debugging the decoder has no in-repo derivation of
`j_start`, the root sets, or the Forney shift factor, and must consult
mk-codec (or its own ancestor, md-codec) to reconstruct the reasoning.

### Minor 2 — mk's BCH unit tests were deleted during the port with no unit-level successor

**Where:** `bch.rs` (`@@ -710,637 +722,3 @@`, 34 tests removed) and
`bch_decode.rs` (`@@ -601,249 +533,3 @@`, 13 tests removed), both repos.

**What:** mk-codec's two files carry 47 unit tests total, including
BIP-93-canonical-value pins (`gen_regular_matches_bip93_canonical_values`,
`gen_long_matches_bip93_canonical_values`, `polymod_init_matches_bip93`),
`GF(1024)` field self-checks (`zeta_is_primitive_cube_root_of_unity`,
`beta_has_order_93_regular`, `gamma_has_order_1023_long`,
`generator_polynomial_evaluates_to_zero_at_specified_roots`), and granular
decode tests that pin exact positions/magnitudes
(`one_error_decodes_correctly_regular`, `two_errors_decode_correctly_regular`,
`four_errors_decode_correctly_long`,
`five_errors_either_rejects_or_returns_bogus_recovery`). mt-codec deletes
all 47 and does not add unit-level replacements: `cargo test -p mt-codec
--lib -- --list` shows zero tests under `string_layer::bch` or
`string_layer::bch_decode` (only `consts`, `chunk`, `header` tests remain).

**Is it in mk too?** N/A — same as Minor 1, this is a port-time subtraction,
not an upstream defect.

**How I checked:** ran `cargo test -p mt-codec --lib -- --list` (15 tests
listed, none in the `bch`/`bch_decode` modules), then confirmed
`tests/correction.rs` (4 tests: `corrects_one_through_four_symbols`,
`refuses_beyond_the_budget`, `margin_is_reported_at_the_limit`,
`a_clean_string_is_never_corrected`) exercises correction behavior only at
the `pipeline::decode_chunk` integration level, against the pinned vector
corpus — not via direct calls into `bch::bch_correct_regular` /
`bch_decode::decode_regular_errors` with hand-chosen error patterns the way
mk's removed tests did.

**Impact:** none currently — this review's cross-run harness (below)
independently confirms the underlying tables and arithmetic are unchanged,
and `tests/correction.rs` does functionally exercise the same code paths
end-to-end. But a future accidental edit to a shared constant or table
(`GEN_REGULAR`, `BETA`, etc.) would no longer be self-caught within
mt-codec's own test suite at the granularity mk-codec's suite would catch
it — it would have to manifest as a pipeline-level correction failure
against the pinned corpus to be noticed at all.

## What I compared and found IDENTICAL

- **Tables/constants, element-by-element:** `GEN_REGULAR[0..5]`,
  `GEN_LONG[0..5]`, `POLYMOD_INIT`, `REGULAR_SHIFT`/`REGULAR_MASK`,
  `LONG_SHIFT`/`LONG_MASK`, `ALPHABET` (`bch.rs`); `BETA`, `GAMMA`, `ZETA`
  (`Gf1024 { lo, hi }` field values, `bch_decode.rs`). Extracted via `sed`
  from both files and diffed programmatically — 0 differences.
- **Loop bounds / structure:** every `for`/`while` in `bch_decode.rs`'s
  production code (syndrome loop, Berlekamp–Massey `for k in 0..n` /
  `for i in 1..=l`, the `s_poly.len().min(8)` syndrome-count cap that
  bounds `t = 4`, Chien search, Forney) — line-for-line identical between
  the two files (offset only by doc-comment length).
- **Function bodies:** every non-doc line in `bch.rs` (`hrp_expand`,
  `polymod_run`, `polymod_step`, `case_check`, `bch_code_for_length`,
  `bytes_to_5bit`/`five_bit_to_bytes`, `bch_create_checksum_regular/long`,
  `bch_verify_regular/long`, `bch_correct_regular/long`,
  `encode_5bit_to_string`, `decode_string`) and `bch_decode.rs`
  (`Gf1024`/`Gf32` arithmetic, `decode_regular_errors`,
  `decode_long_errors`) — 16 diff hunks in `bch.rs` and 6 in `bch_decode.rs`
  total, every one accounted for as either the `MK_*_CONST`→`MT_*_CONST`
  rename, an `mk1`/`mk-codec`→`mt1`/`mt-codec` doc-comment edit, an
  `#[allow(dead_code)]` addition on now-test-only helper functions, or the
  two test-module deletions (Minor 1/2 above). No hunk touches a table
  value, a loop bound, a generator-polynomial coefficient, or a field
  constant.
- **Live cross-run** (throwaway harness, `/tmp/claude-1000/…/scratchpad/
  bch-crossrun`, path-dependency on both real crates, not committed to
  either repo): adapted mk's own removed test vectors (t=1..4 regular, t=1
  and t=4 long, a checksum-region error, and mk's `five_errors_...`
  5-error case) to mt's HRP/consts and ran `bch_create_checksum_*` +
  `bch_correct_*` through both crates side by side on identical data and
  identical error positions/magnitudes:
  - regular t=0/1/2/3/4 and the full `BCH(93,80,8)` t=4 capacity: mk and mt
    report identical `corrected_positions`, identical magnitudes, identical
    `corrections_applied`, and both recover their own original data.
  - long t=1 and the full `BCH(108,93,8)` t=4 capacity: same agreement.
  - error inside the checksum region: same agreement.
  - **t=5 boundary** (5 simultaneous errors, exceeding `t=4`): mk and mt
    **agree** — both return `Err(BchUncorrectable)` (the defensive
    re-verify guard in `bch_correct_long`, byte-identical code in both
    crates, catches the pathological pattern in both).
  - All 9 cases: `cargo run --release` — every assertion passed, 0 panics.

Facts already settled (per brief, not re-reported): fmt/clippy/tests green
in `mnemonic-transaction`; the fork-not-depend decision; the provenance
comment fix; naming/formatting/doc-comment differences beyond the two
Minors above.
