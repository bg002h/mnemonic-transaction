# FIX: mt-codec decode refuses a set that is not §3b's balanced split (bughunt-1119 FIX table row 35, "FIX-I mt-codec decode strictness")

Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-23. Keys: bh-mt-I18 (I),
bh-mt-I19 (M, accepted). Ruling T18 (`coordinator/agent-reports/architect-bughunt-1119-questions.md:56`),
binding; P7 (`:104`, library-only reach graded I). Track: full (public library
API and a new `Error` variant; up to three review rounds, no separate plan).
Not class B (no funds path shown; data integrity of a backup). Base: origin/main
`88ac6d9`. Branch `fix/mt-decode-strictness`.

Not to be confused with the older `design/FIX_bughunt_mt_*.md` (a different hunt).

**Source files this FIX changes (for build ordering against the other mt rows):**
`crates/mt-codec/src/error.rs`, `crates/mt-codec/src/string_layer/pipeline.rs`;
new tests `crates/mt-codec/tests/bh1119_mt_decode_strictness.rs`,
`crates/mt-cli/tests/bh1119_mt_decode_strictness.rs`. No `crates/mt-cli/src`
file changes. Overlap with the other mt rows: FIX_bh1119_mt_prefix_corrected
also edits `pipeline.rs` (`decode_chunk`'s `invariant_prefix`, P0), a different
function from this FIX's `decode`; a change to `explain_failure`
(`crates/mt-cli/src/main.rs:1881-2017`) would move this FIX's CLI rows (C1-C4,
section 6); the codec rows depend on no mt-cli change.

## 1. Findings (run at 88ac6d9, nextest, `-E 'binary(bh1119_mt_decode_strictness)'`)

The test text is in `design/FIX_bh1119_mt_decode_strictness/` (section 7). All
fixtures are the repo's own corpus (`crates/mt-codec/src/test_vectors/mt1_v1.json`)
or hand-built byte strings; no key material.

- **I18 (wrong bytes, order-dependent, Ok).** `pipeline::decode`
  (`pipeline.rs:249-384`) takes `bpc` from the first non-final chunk in INPUT
  order (`pipeline.rs:296-303`) and slices every non-final payload to it
  (`pipeline.rs:362-376`, `c.payload[..keep.min(c.payload.len())]`). It never
  checks that the chunk sizes are the split §3b states normatively
  (`SPEC_mt_v0_1.md:1529-1541`: `count = ceil(len/40)`, `bytes_per_chunk =
  ceil(len/count)`, "equal across the set except for the last"). The triage set
  (non-final 2 and 3 bytes, final 1) returns `Ok([1,2,3,4,6])` in three orders
  and `Ok([1,2,3,4,5,6])` in the other three. Any set whose sizes differ from the
  plan is accepted: final chunk longer than the rest (R3), an unbalanced 10+1
  split (R4), a filling chunker's 40/40/10 (R5, the hazard §3b names), one chunk
  of 41 bytes (R6), an empty trailing chunk (R7), and 32,768 chunks of 41 bytes,
  1,343,488 bytes, past the 1,310,720-byte ceiling (R8).
- **I18 at the CLI (reached; the triage only said "may").** All strings of a
  BCH-valid set parse, so `length_report` (`read_strings.rs:329`, consulted only
  on failure) never runs and the codec's result is used. The `even` vector
  (222 bytes, six chunks of 37) re-cut 38/36/37/37/37/37 under its own set id:
  - in index order, `mt decode`, `mt verify` and `mt inspect` exit 0
    (`mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.`): a set no
    conforming encoder writes is reported as a good backup;
  - reversed, `mt decode` exits 1 with a false diagnosis:
    `mt decode: REFUSED — §8.2e, input is not a decodable Bitcoin transaction`
    ... `Every checksum held, so this is not miscut steel — the strings are more
    likely from two different engravings mixed together.` (the decoder dropped
    byte 38 and the 221 bytes left do not parse).
- **I19 (M, accepted by T18).** `decode_chunk` (`pipeline.rs:218-227`) discards
  the leftover pad bits without checking them. T18: "BIP-93 L217 pads 'with
  arbitrary bits', and SPEC_mt_v0_1.md:3669-3673 asks no zero pad; no
  canonical-encoding guarantee is added." This FIX changes nothing for I19; R10
  pins the accepted behaviour. The length rule below does not depend on pad bits:
  a chunk's payload length is `floor(5n/8)` for `n` body symbols, whatever the pad
  bits hold. (The batch brief lists grade-mt.md:130 item 10 as unruled; T18 rules
  it, and this FIX does not depend on it either way.)

## 2. Rulings (binding, quoted)

- **T18 mt-Q10 (I18, I19).** "I18 (uneven middle chunks, wrong bytes with Ok)
  refuses with a new Error variant. I19 stays M and accepted: BIP-93 L217 pads
  'with arbitrary bits', and SPEC_mt_v0_1.md:3669-3673 asks no zero pad; no
  canonical-encoding guarantee is added. Unblocks FIX-I."
- **P7.** A public function of a published crate returning Ok with wrong data,
  with no CLI path, is I. (Section 1 shows a CLI path as well.)

## 3. Spec delta (rules)

**D1. The size rule.** `pipeline::decode`, once every slot `0..count` holds a
chunk (after every existing check, section D2), computes
`total` = the sum of the chosen chunks' payload lengths and
`p = chunk::plan(total)` (`chunk.rs`, the §3b rule), then walks the chunks in
INDEX order `i = 0..count`. Chunk `i` conforms when `i < p.count` and its payload
length equals `chunk::range(p, i).len()`. The first chunk (lowest index) that
does not conform ends the decode with
`Error::ChunkLengthMismatch { index: i + 1, count, found, total, planned_count:
p.count, bytes_per_chunk: p.bytes_per_chunk, last_chunk_bytes:
p.last_chunk_bytes }`. A conforming set reassembles by concatenating every
chunk's whole payload in index order (each now equals its plan range), so the
result no longer depends on input order. The rule judges the whole set against
the plan, so a set with a single odd chunk is reported at the lowest index whose
length differs from the plan for that total, which need not be the odd chunk
(R3: a 38-byte final chunk is reported at chunk 1). The rule is the spec's own
split, so every set `encode` writes passes (R9, and every corpus vector).

- **D1a. Total past the header's reach.** If `chunk::plan(total)` returns
  `Err(TooManyChunks { .. })` (possible only when hand-built chunks over 40 bytes
  push `total` past `32,768 × 40`), `decode` returns that error unchanged (R8).
- **D1b. No other new case.** When every chunk `i < count` conforms,
  `p.count == count` follows (if `p.count > count`, the first `count` plan ranges
  sum to less than `total`; if `p.count < count`, chunk `p.count` is out of plan),
  so no separate count comparison exists. A zero-byte chunk is judged by the same
  rule (R7); a single-chunk set is judged by the same rule (R6: 41 bytes plans as
  two chunks).

**D2. Precedence (unchanged errors keep their order).** In `decode`, these stay
exactly where they are and fire before D1: nothing readable (first error),
the "no non-final chunk read" `MissingChunk { missing: 1, count }` early return
(today's `bpc` block, `pipeline.rs:296-303`; the builder keeps that guard as a
plain condition, `count > 1` and no read chunk has `index < count - 1`, since
`bpc` itself is removed), `SetIdMismatch`, `CountMismatch`, the `idx >= count`
`MissingChunk`, `AmbiguousChunk`, then `MissingChunk` for the first empty slot.
All slots are checked for presence before D1 runs, so a set that is both
incomplete and uneven reports the missing chunk. `duplicates` and `unreadable`
are computed as today.

**D3. The variant (new, public).** In `crates/mt-codec/src/error.rs`, before
`MixedCase`:

- Name: `ChunkLengthMismatch` (PROPOSED, architect confirms; named after
  `SetIdMismatch`, `CountMismatch`, `ContentIdMismatch`).
- Fields, all `usize`, in this order: `index` (1-based chunk number, as printed
  to a human), `count` (the set size from the headers), `found` (this chunk's
  payload length in bytes), `total` (the set's total payload length),
  `planned_count`, `bytes_per_chunk`, `last_chunk_bytes` (§3b's plan for
  `total`).
- Display (PROPOSED, architect confirms; holder-visible through the CLI, D4),
  one line:

      chunk {index} of {count} has payload length {found}, which §3b's balanced split does not produce for a total payload length of {total} (count {planned_count}, bytes per chunk {bytes_per_chunk}, last chunk {last_chunk_bytes})

  The text puts no noun after a number that can be 1 or 0, so it needs no plural
  forms (the codec's other texts do not pluralise either).
- Rustdoc on the variant (wording free): it is raised only for a complete set
  whose BCH-valid chunks are not §3b's split; such a set is never cut from a
  conforming encoder's output, so it is hand-built or from another
  implementation; it carries the plan so the refusal names the numbers (the
  `error.rs:1-5` rule).
- The `decode` rustdoc (`pipeline.rs:244-248`) gains one sentence (wording free):
  a complete set whose chunk sizes are not §3b's split is refused with
  `ChunkLengthMismatch`, so the result never depends on input order.

**D4. CLI (no code change).** `decode`, `verify` and `inspect` pass the codec
error to `explain_failure` (`main.rs:1382`, `:1523`, `:1785`). With every string
readable and no index typed twice, it reaches the fall-through
(`main.rs:2011-2017`), so each verb prints, exit 1, stdout empty:

    mt <verb>: REFUSED — §1.1, the set does not verify

      chunk 1 of 6 has payload length 38, which §3b's balanced split does
      not produce for a total payload length of 222 (count 6, bytes per
      chunk 37, last chunk 37)

(measured, section 6). When an index was also typed twice, the existing
"Chunk N arrived TWICE" stack hint follows, as today. No new CLI text, refusal
entry or remedy: the codec's other set-level errors (`SetIdMismatch`,
`AmbiguousChunk`, `ContentIdMismatch`) take the same path and have no entry in
`crates/mt-cli/tests/refusals.toml` (its `check` field names a
`fn(..) -> Result<(), Refusal>` in mt-cli, which `explain_failure` is not), so
`check-refusal-coverage.sh` and `mutate-refusals.sh` counts do not change.

**D5. Unchanged.** `decode_chunk`, `encode`, `chunk::plan`, `chunk::range`,
`invariant_prefix`, every existing `Error` variant and text, every mt-cli source
file. I19: pad bits stay unchecked (T18).

**D6. Spec, README, manual, CHANGELOG.** No spec delta: §3b already states the
split normatively and §1.1e says the final chunk's length "cannot be checked
until the set is complete, and is not checked before then"
(`SPEC_mt_v0_1.md:1036-1038`), which D1 is. The spec here is a copy owned by
mnemonic-engrave (`CLAUDE.md` "Design files are copies"), so no copy edit either.
README: none. Toolkit manual: none (`docs/manual/src/40-cli-reference/` has no
mt page, and the CLI text is unchanged). CHANGELOG: none in this FIX (entries are
written at release, as the sibling FIX docs say).

**D7. Semver.** `Error` is a public exhaustive enum (no `#[non_exhaustive]`), so
the variant is a breaking change on the pre-1.0 `0.X` axis (CHANGELOG preamble).
mt-codec is not published (git pin in mnemonic-engrave,
`me-cli/Cargo.toml:104`). No mt-cli or me-cli source matches on `Error`
variants (grep, section 9), so nothing fails to build. The version bump is the
release's; filed as a FOLLOWUPS line (section 10).

## 4. Holder-visible texts

| where | text | status |
| --- | --- | --- |
| `Error::ChunkLengthMismatch` Display, printed by `mt decode|verify|inspect` as the refusal's mechanism (D3, D4) | `chunk {index} of {count} has payload length {found}, which §3b's balanced split does not produce for a total payload length of {total} (count {planned_count}, bytes per chunk {bytes_per_chunk}, last chunk {last_chunk_bytes})` | PROPOSED (architect confirms) |
| the refusal head | `mt <verb>: REFUSED — §1.1, the set does not verify` | existing, unchanged |

Exit codes: unchanged mechanism (a `Refusal` exits 1); the re-cut set moves from
exit 0 (C1, C3, C4) and from a wrong `§8.2e` refusal (C2) to the refusal above.

## 5. Codec RED rows (`crates/mt-codec/tests/bh1119_mt_decode_strictness.rs`)

Each row runs `decode` and compares `Result<bytes, error Display>`; the
"every order" rows run index order and reversed (all six orders for the
three-chunk R1). Command:
`cargo nextest run --locked -p mt-codec -E 'binary(bh1119_mt_decode_strictness)'`.

| row | set (payload bytes per chunk, set id) | base 88ac6d9 (seen) | after (expected) |
| --- | --- | --- | --- |
| R1 `r1_uneven_middle_chunks_are_refused_in_every_order` | 2/3/1, `0x12345` | FAIL `left: Ok([1, 2, 3, 4, 6])` | `Err("chunk 1 of 3 has payload length 2, which §3b's balanced split does not produce for a total payload length of 6 (count 1, bytes per chunk 6, last chunk 6)")`, all six orders |
| R2 `r2_recut_even_vector_is_refused_in_both_orders` | `even` 222 bytes re-cut 38/36/37/37/37/37, its own id | FAIL `left: Ok([2, 0, 0, 0, 0, 1, 1, 124, ...` | `Err("chunk 1 of 6 has payload length 38, ... total payload length of 222 (count 6, bytes per chunk 37, last chunk 37)")` |
| R3 `r3_final_chunk_longer_than_the_rest_is_refused` | 37×5 then 38 | FAIL `left: Ok([0, 1, ...` | `Err("chunk 1 of 6 has payload length 37, ... of 223 (count 6, bytes per chunk 38, last chunk 33)")` |
| R4 `r4_unbalanced_two_chunk_set_is_refused` | 10/1 | FAIL `left: Ok([0, 1, ..., 10])` | `Err("chunk 1 of 2 has payload length 10, ... of 11 (count 1, bytes per chunk 11, last chunk 11)")` |
| R5 `r5_filled_set_is_refused` | 40/40/10 | FAIL `left: Ok([0, 1, ..., 89])` | `Err("chunk 1 of 3 has payload length 40, ... of 90 (count 3, bytes per chunk 30, last chunk 30)")` |
| R6 `r6_single_chunk_over_the_ceiling_is_refused` | one chunk of 41 | FAIL `left: Ok([0, 1, ..., 40])` | `Err("chunk 1 of 1 has payload length 41, ... of 41 (count 2, bytes per chunk 21, last chunk 20)")` |
| R7 `r7_empty_trailing_chunk_is_refused` | 30/0 | FAIL `left: Ok([0, 1, ..., 29])` | `Err("chunk 2 of 2 has payload length 0, ... of 30 (count 1, bytes per chunk 30, last chunk 30)")` |
| R8 `r8_total_beyond_the_header_ceiling_is_refused` | 32,768 chunks of 41 | FAIL `left: Ok(1343488)` | `Err("payload of 1343488 bytes needs 33588 chunks; the header addresses at most 32768")` |
| R9 `r9_conforming_sets_of_every_length_still_decode` (control) | `encode` of every length 1..=600, index order and reversed | PASS | PASS |
| R10 `r10_i19_nonzero_padding_stays_accepted` (control, T18) | one 1-byte chunk, a pad bit flipped, checksum recomputed | PASS `Ok([165])` | PASS `Ok([165])` |

## 6. CLI RED rows (`crates/mt-cli/tests/bh1119_mt_decode_strictness.rs`)

The R2 set written one string per line to a 0600 temp file, run as
`mt <verb> --bitcoin-cli /nonexistent/bitcoin-cli --in <file>`. Each row asserts
exit 1, empty stdout, stderr starting `mt <verb>: REFUSED — §1.1, the set does
not verify`, and the whitespace-collapsed stderr containing the R2 Display text.
Command: `cargo nextest run --locked -p mt-cli -E 'binary(bh1119_mt_decode_strictness)'`.

| row | verb, order | base 88ac6d9 (seen) | after |
| --- | --- | --- | --- |
| C1 `c1_decode_refuses_recut_set_in_index_order` | decode, index order | FAIL exit 0, the transaction hex on stdout, `TX 2dcf2b97...f630` | exit 1, D4 text |
| C2 `c2_decode_refuses_recut_set_reversed` | decode, reversed | FAIL exit 1 `mt decode: REFUSED — §8.2e, input is not a decodable Bitcoin transaction` ... `two different engravings mixed together` | exit 1, D4 text |
| C3 `c3_verify_refuses_recut_set` | verify, index order | FAIL exit 0 `mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.` | exit 1, D4 text |
| C4 `c4_inspect_refuses_recut_set` | inspect, index order | FAIL exit 0, the full inspect report on stdout | exit 1, D4 text |

The after column was measured on the scratch GREEN (section 8).

## 7. Tests that change

- New: the two files above; their full text is
  `design/FIX_bh1119_mt_decode_strictness/codec.rs.txt` and `.../cli.rs.txt`
  (the builder copies them to the two test paths unchanged; test names are the
  row names).
- Changed: none. No existing test asserts the old behaviour (section 8's
  scratch GREEN ran the whole workspace suite with D1-D3 applied).

## 8. Scratch GREEN (author, not committed)

The author applied D1-D3 to a scratch copy of the base (not committed; reverted
with `git checkout` before the FIX commit) to check that the expected texts are
exact and that no existing test moves:

- `cargo nextest run --locked --no-fail-fast` (whole workspace, both new files in
  place): `473 tests run: 473 passed` (base 459 plus the 14 rows).
- `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`:
  clean (the D2 guard written as a plain condition; `bpc` removed).
- The rows alone: `14 tests run: 14 passed`. Then, with the scratch reverted, the
  same command on base: `14 tests run: 2 passed, 12 failed` (R9, R10 pass).
- `target/debug/mt decode|verify|inspect --bitcoin-cli /nonexistent/bitcoin-cli
  --in <the R2 set>`: each printed the D4 block (wrapped as shown there) and
  exit 1.

The scratch is a check of this document, not a prescribed implementation; the
builder writes the code to D1-D3.

## 9. Coverage map (rule -> rows)

| rule | rows |
| --- | --- |
| D1 non-final chunks unequal | R1, R2, C1-C4 |
| D1 final chunk over the plan | R3 |
| D1 balanced split, not just equal middles | R4, R5 |
| D1 single chunk judged by the plan | R6 |
| D1 chunk past the plan's count (D1b) | R7 |
| D1 order independence | R1 (six orders), R2, R3, R4, R5, R7 (two orders), C1 vs C2 |
| D1a plan error passes through | R8 |
| D1 every conforming set still decodes | R9, existing `vectors.rs` and the corpus |
| D2 precedence | existing `bughunt.rs` (`bughunt_mt_04_*` count mismatch), `correction.rs`, `decode_is_order_independent`; unchanged and run in the gate |
| D3 variant and Display | R1-R7 (exact Display) |
| D4 CLI path, exit 1, text | C1-C4 |
| D5 I19 unchanged | R10 |
| D7 no consumer matches on `Error` | gate row 9 |

## 10. FOLLOWUPS (listed in the author's report, not appended here)

- `mt-codec-decode-plan-check`: owed-to-build (I18), closed by the GREEN commit.
- `mt-codec-error-variant-semver`: the release bumps mt-codec on the `0.X` axis
  for the new variant; mnemonic-engrave's mt-codec pin gains the refusal at its
  next bump (`me-cli/src/sysw/mt.rs:163`, `:240` use `decode(..).ok()`).

## 11. Gate (builder, final head; run once to a file, then grep)

Toolchain `$HOME/.cargo/bin` first in PATH; `cargo --version` must print 1.85.0.
The CI `test` job's list (`.github/workflows/ci.yml`) plus the rows:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expect 0 failed; 473 run on
   base 88ac6d9 plus this FIX, more if other mt rows land first)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh` (count unchanged from base)
7. `./scripts/mutate-refusals.sh` (count unchanged from base)
8. `./scripts/journeys.sh`
9. `grep -rnE 'Error::[A-Z][A-Za-z]* *(\{|\(|=>)' crates/mt-cli/src` -> no output
   (no mt-cli match on codec variants; D7)
10. `bash ci/docs-only.test.sh` and `bash ci/code-docs-check.sh`
11. Rows by name:
    `cargo nextest run --locked -p mt-codec -p mt-cli -E 'binary(bh1119_mt_decode_strictness)'`
    -> `14 tests run: 14 passed`. Fewer than 14 is a FAIL.
12. `grep -n 'bpc' crates/mt-codec/src/string_layer/pipeline.rs` -> no output
    (the first-read size is gone, D1).

Review: full track, up to three rounds; not class B.

## 12. Open points for the architect (no STOP)

- **O1.** The variant name and Display text (D3), PROPOSED.
- **O2.** D1 names the lowest index that differs from the plan for the set's
  total, not the "odd one out" (R3 names chunk 1 for a long final chunk). A
  majority-based diagnosis would need its own rules for ties; these sets are
  BCH-valid and so hand-built or from another implementation, never steel damage
  (a dropped or added character fails BCH and reaches `length_report` instead).
- **O3.** No CLI remedy line is added (D4), matching the codec's other set-level
  errors. A remedy would be new holder text.
