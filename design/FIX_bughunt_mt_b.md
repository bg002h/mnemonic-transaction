# FIX-B: mt bug hunt, codec robustness (#4, #6, #8; rulings R4, R6, R8)

Small-fix track. Repo: mnemonic-transaction only. Branch `fix/bughunt-b`, base
`3bfbc98`. Inputs: `design/agent-reports/grade-bughunt-mt.md` (R4, R6, R8 bind;
§3 row FIX-B), `design/agent-reports/bughunt-triage-mt.md` (#4, #6, #8
CONFIRMED). No new flag, type or command; no new printed text (the one new
string is a codec `Display` that reaches the CLI through an existing refusal).

Two STOP sections (S1, S2) are open at the end. S2 blocks step 1 (the spec
edit) only; steps 2-5 do not depend on it.

## Build order

1. FIX-B builds first. FIX-C (reader, JSON, flags) touches the same reader
   error mapping; FIX-C's base is FIX-B's merged head on master, not `3bfbc98`.
2. Within FIX-B, in this order:
   1. Spec delta (after S2 is ruled).
   2. RED rows: already committed on this branch (this document's commit).
      Run the RED command below and confirm 7 matched, 7 failed.
   3. R8: `invariant_prefix` guard.
   4. R4: `Error::CountMismatch` and its check.
   5. R6: early non-ASCII scan in `decode_string`.
   6. Gate (below), captured once to a file.

## 1. Spec delta (`design/SPEC_mt_v0_1.md`, §1.1 `mt verify` checklist)

Where the edit lands is S2. The text is decided. At the current mt copy, lines
221-223, replace:

```
   (**chunks 1 through `count` present**, i.e. wire indices `0..count-1`), every
   chunk carries the same
   `chunk_set_id`, and the reassembled transaction re-derives that id.
```

with:

```
   (**chunks 1 through `count` present**, i.e. wire indices `0..count-1`), every
   chunk carries the same
   `chunk_set_id` and the same `count`, and the reassembled transaction
   re-derives that id.
```

This is the only spec change. §1.1a's decode table (row "know when the set is
complete | `count` in every header") is unchanged. No refusal text, exit code
or JSON warning is added to the spec: the refusal is the existing §1.1 one.

## 2. Code changes (production)

### R8, `crates/mt-codec/src/string_layer/pipeline.rs`, `invariant_prefix`

Immediately after `let symbols = to_symbols(s)?;` insert:

```rust
    let min = HEADER_SYMBOLS + CHECKSUM_SYMBOLS;
    if symbols.len() < min {
        return Err(Error::TooShort {
            len: symbols.len(),
            header: HEADER_SYMBOLS,
            checksum: CHECKSUM_SYMBOLS,
            min,
        });
    }
```

These are the same constants and fields `decode_chunk` uses (11, 13, 24). No
new text.

### R4, `crates/mt-codec/src/error.rs`

Insert a variant directly after `SetIdMismatch { .. },` and before
`MissingChunk`:

```rust
    /// Chunks in one set disagree about how many chunks the set has.
    #[error("chunk set size mismatch: expected {expected} chunks, chunk {index} carries {found}")]
    CountMismatch {
        /// The count the first readable chunk carries.
        expected: usize,
        /// The count this chunk carries.
        found: usize,
        /// 1-based chunk number, as printed to a human.
        index: usize,
    },
```

### R4, `crates/mt-codec/src/string_layer/pipeline.rs`, `decode`

In the `for c in read` loop, immediately after the `if cid != set_id { ... }`
block and before `if idx >= count {`, insert:

```rust
        if c.header.count != count {
            return Err(Error::CountMismatch {
                expected: count,
                found: c.header.count,
                index: idx + 1,
            });
        }
```

`expected` is the count of the first readable chunk (`read.first()`), `index`
is the offending chunk's `header.index + 1`, as `SetIdMismatch` does.

Ordering, decided (fail-closed either way, no further change): the pre-loop
`bpc` lookup (`read.iter().find(|c| c.header.index < count - 1)`) runs before
the loop, so a set whose first chunk claims `count = 2` and holds no chunk with
index 0 still refuses with `MissingChunk { missing: 1, count: 2 }`, not
`CountMismatch`. The check is not moved ahead of `bpc`.

mt-cli: no change. `decode`, `verify` and `inspect` all reach
`explain_failure`'s generic fallback (`crates/mt-cli/src/main.rs`), which
prints `mt {verb}: REFUSED — §1.1, the set does not verify` and the variant
text; exit 1; stdout empty. This is pinned by
`bughunt_mt_04_cli_count_mismatch_refusal`.

### R6, `crates/mt-codec/src/string_layer/bch.rs`, `decode_string`

As the first statements after `use crate::Error;`, before the `case_check`
call, insert:

```rust
    if let Some(sep) = s.rfind(SEPARATOR) {
        if let Some((i, ch)) = s[sep + 1..]
            .chars()
            .enumerate()
            .find(|(_, c)| !c.is_ascii())
        {
            return Err(Error::InvalidChar { ch, position: i });
        }
    }
```

`position` is the char offset in the data part of the original string. The
`s.to_lowercase()` line is not changed. The existing `if !c.is_ascii()` branch
inside the data loop stays as written: it is now unreachable (lowercasing an
all-ASCII data part stays ASCII) but it guards `ALPHABET_INV[c as usize]`
(`[u8; 128]`) against a future refactor. A mutation survivor on that branch is
an equivalent mutant; the reviewer brief records it as such.

## 3. RED rows

All in this branch's commit; none changes production code. Reused rows are from
a5e5fd7 (CONFIRMED by the triage); new rows were added for the exact texts the
rulings fix.

| row | file | kind | asserts |
|---|---|---|---|
| `bughunt_mt_04_decode_rejects_mixed_count_chunks` | `crates/mt-codec/tests/bughunt.rs` | reused | mixed-count set is refused |
| `bughunt_mt_04_count_mismatch_variant_and_text` | `crates/mt-codec/tests/bughunt.rs` | new (R4) | `decode([a[0], b[1]])` errs with Debug `CountMismatch { expected: 2, found: 3, index: 2 }` and Display `chunk set size mismatch: expected 2 chunks, chunk 2 carries 3` (asserted via strings so the row compiles before the variant exists) |
| `bughunt_mt_04_cli_count_mismatch_refusal` | `crates/mt-cli/tests/bughunt.rs` | new (R4, CLI) | for `decode`, `verify`, `inspect` on stdin `a[0]\nb[1]\n`: exit 1, stdout empty, first stderr line `mt {verb}: REFUSED — §1.1, the set does not verify`, stderr contains the CountMismatch Display |
| `bughunt_mt_06_kelvin_sign_is_not_k` | `crates/mt-codec/tests/bughunt.rs` | reused | a U+212A in place of `k` is not accepted |
| `bughunt_mt_06_kelvin_is_invalid_char_at_data_offset` | `crates/mt-codec/tests/bughunt.rs` | new (R6) | fixture's first data-part `k` is at byte 25; error is `InvalidChar { ch: '\u{212A}', position: 22 }`, Display `character '\u{212A}' at data-part offset 22 is not in the bech32 alphabet` |
| `bughunt_mt_08_invariant_prefix_short_input_does_not_panic` | `crates/mt-codec/tests/bughunt.rs` | reused | `invariant_prefix` on short input does not panic |
| `bughunt_mt_08_invariant_prefix_short_input_is_too_short` | `crates/mt-codec/tests/bughunt.rs` | new (R8) | `invariant_prefix("mt1qq")` returns `Err(TooShort { len: 2, header: 11, checksum: 13, min: 24 })` |

Command (RED before step 2.3, GREEN after step 2.5):

```
PATH=$HOME/.cargo/bin:$PATH cargo nextest run --locked --no-fail-fast -E 'test(/(^|::)(bughunt_mt_04_decode_rejects_mixed_count_chunks|bughunt_mt_04_count_mismatch_variant_and_text|bughunt_mt_04_cli_count_mismatch_refusal|bughunt_mt_06_kelvin_sign_is_not_k|bughunt_mt_06_kelvin_is_invalid_char_at_data_offset|bughunt_mt_08_invariant_prefix_short_input_does_not_panic|bughunt_mt_08_invariant_prefix_short_input_is_too_short)$/)'
```

Expected matched count: **7** (6 in `mt-codec::bughunt`, 1 in `mt-cli::bughunt`).
A short or empty count is a FAIL.

RED, observed on unmodified code (`3bfbc98` plus this commit's tests), summary
`7 tests run: 0 passed, 7 failed, 284 skipped`, each for the stated reason:

- `..._04_count_mismatch_variant_and_text`: `left: None`, `right:
  Some("CountMismatch { expected: 2, found: 3, index: 2 }")` (the set is
  accepted).
- `..._04_cli_count_mismatch_refusal`: `left: Some("mt decode: REFUSED — §8.2e,
  input is not a decodable Bitcoin transaction")`, `right: Some("mt decode:
  REFUSED — §1.1, the set does not verify")` (the codec accepts the set; the
  spliced payload then fails transaction parsing).
- `..._06_kelvin_is_invalid_char_at_data_offset`: `left: None`, `right:
  Some(InvalidChar { ch: 'K', position: 22 })` (U+212A, printed by Debug; the
  string is accepted).
- `..._08_invariant_prefix_short_input_is_too_short`: panics at
  `crates/mt-codec/src/string_layer/pipeline.rs:367:15` (slice out of range),
  then `left: None`, `right: Some(Err(TooShort { len: 2, header: 11, checksum:
  13, min: 24 }))`.
- Reused rows: `_04_decode_rejects_mixed_count_chunks` (bughunt.rs:13),
  `_06_kelvin_sign_is_not_k` (bughunt.rs:29),
  `_08_..._does_not_panic` (panic at pipeline.rs:367, bughunt.rs:40).

GREEN was proven reachable with the code in §2 applied as a scratch change
(reverted; not committed): all 7 rows pass, and `fmt`, `clippy`,
`check-refusal-coverage.sh`, `mutate-refusals.sh` and `journeys.sh` pass. The
only failing tests in the full suite were the 8 bughunt rows owned by FIX-A,
FIX-C and FIX-D (#01, 02, 03, 05, 07, 10, 11, 12).

## 4. Tests that change

- `crates/mt-codec/tests/bughunt.rs`: import becomes `use mt_codec::{Error,
  decode, encode};`; three new rows (above). Committed.
- `crates/mt-cli/tests/bughunt.rs`: one new row (above). Committed.
- `scripts/check-refusal-coverage.sh`: one `EXEMPT` entry,
  `bughunt_mt_04_cli_count_mismatch_refusal`, with reason: the codec's
  chunk-count check reached through `explain_failure`'s generic §1.1 fallback
  like `SetIdMismatch`; there is no CLI guard fn for `mutate-refusals.sh` to
  neuter; the codec row holds the check. Without it the gate fails (the row
  "asserts on a §-refusal message" with no `refusals.toml` entry). Committed.
  This is a test-infrastructure change; the PR description says so.
- `crates/mt-cli/tests/refusals.toml`: unchanged.
- No existing test changes expectation (the scratch GREEN run showed no other
  regressions).
- README, CHANGELOG, `--help`: no change (O1 lockstep target is mt's own
  `--help`, README and spec; no flag or printed CLI text changes here).

## 5. Gate (run once to a file, then grep)

The repo has no CLAUDE.md phase-close list; its gate is CI's job
(`.github/workflows/ci.yml`) plus the release workflow's doc tests. With
`$HOME/.cargo/bin` first in PATH and `cargo --version` = 1.85.0
(`rust-toolchain.toml`):

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked --no-fail-fast` (expected: only the 8 non-B
   bughunt rows fail until FIX-A, C, D land; no other failure)
5. `cargo test --locked --workspace --doc`
6. `./scripts/check-refusal-coverage.sh`
7. `./scripts/mutate-refusals.sh`
8. `./scripts/journeys.sh`
9. The RED command of §3: 7 matched, 7 passed. Quote the count in the GREEN
   commit message.

Then `coordinator/scripts/mutation-gate.sh <worktree> 3bfbc98` before review
(production Rust changed). Expected equivalent mutant: the now-unreachable
`!c.is_ascii()` branch in `decode_string`'s data loop.

## 6. Cross-repo record (BCH defect rule)

Checked the sibling `decode_string` paths for #6's pattern (Unicode case
folding before the non-ASCII check). Read only; no test run in those repos.

- mnemonic-key `275a994`, `crates/mk-codec/src/string_layer/bch.rs:673`:
  `s.to_lowercase()` runs before the `!c.is_ascii()` check at line 690. Same
  pattern as #6 (mt's string layer is ported from it): a U+212A in the data
  part would lowercase to `k` and be accepted. Not fixed here; for the
  coordinator to file against mk.
- descriptor-mnemonic `07d85472`: uses `to_ascii_lowercase`, so no Kelvin
  folding. A different pattern: `crates/md-codec/src/chunk.rs:528` and
  `codex32.rs:73` map a char with `c as u8`, which truncates a non-ASCII char
  (for example U+0171 becomes 0x71, `q`) and can land in the alphabet. Not
  verified by a test; for the coordinator to triage against md.

## STOP S1: non-ASCII in the HRP, or no separator

R6 says "any non-ASCII character" and gives `position` as the data-part
offset. A non-ASCII character in the HRP, or a string with no `1` separator,
has no data-part offset. As written (§2), the early scan covers only the data
part; such strings keep today's result, `InvalidHrp`, which still refuses
(no non-ASCII character lowercases to `m`, `t` or `1`, so it cannot forge the
HRP). Recommended: keep `InvalidHrp`. Needs a ruling to confirm.

## STOP S2: which copy of the spec the delta edits

mt's `CLAUDE.md` and `design/PROVENANCE.md` say the spec is a copy owned by
mnemonic-engrave: change it there, then re-sync. The grade (§3) says every
FIX is "mt repo only". The mt copy has already drifted from mnemonic-engrave
(`diff` of the two files: 152 lines at engrave `049f1340`), so a re-sync would
also carry or overwrite unrelated changes. Options: (a) edit the mt copy only,
as earlier phases did, and file a follow-up to carry it to engrave; (b) edit
engrave first and re-sync. Needs a ruling; step 2.1 waits on it.
