**⚠ Untriaged candidate Important outside mt: mnemonic-key decode_string has #6's Kelvin pattern (to_lowercase before the non-ASCII check). Not confirmed by a test.**

# FIX-B document report: mt bug hunt, codec robustness

FIX doc: `design/FIX_bughunt_mt_b.md`, commit `aab091f` on `fix/bughunt-b`
(base `3bfbc98`, not pushed). It covers R4, R6 and R8, with the spec delta, the
exact code, the RED rows, the tests that change, build order (B first; FIX-C's
base is B's merged head) and the gate.

## RED evidence

- Four new rows: `bughunt_mt_04_count_mismatch_variant_and_text`,
  `bughunt_mt_04_cli_count_mismatch_refusal`,
  `bughunt_mt_06_kelvin_is_invalid_char_at_data_offset` and
  `bughunt_mt_08_invariant_prefix_short_input_is_too_short`. Three rows are
  reused from a5e5fd7.
- On unmodified code the 7 rows ran by exact name: 7 matched, 0 passed,
  7 failed. Each failed for the stated reason (FIX doc §3).
- GREEN is reachable: with the doc's code applied as a scratch change, all 7
  pass. fmt, clippy, check-refusal-coverage, mutate-refusals and journeys also
  pass. The only full-suite failures are the 8 bughunt rows of FIX-A, C and D.
  The scratch change was reverted and is not committed.

## Disclosures

- Test infrastructure: one `EXEMPT` entry was added to
  `scripts/check-refusal-coverage.sh` for the CLI row. Without it the gate
  fails.
- The mt repo has no CLAUDE.md phase-close list. The gate is taken from CI's
  job plus the release workflow's doc tests.

## STOPs (for the architect)

- S1: a non-ASCII character in the HRP, or a string with no separator, has no
  data-part offset. The doc keeps today's `InvalidHrp`, which fails closed.
  This needs confirming.
- S2: which spec copy the delta edits. Repo rules say to edit
  mnemonic-engrave and then re-sync. The grade says "mt repo only". The mt
  copy has drifted from engrave by 152 diff lines. The spec edit waits on
  this ruling; the code steps do not.

## Cross-repo (BCH defect rule; read only)

- mk `275a994`, `crates/mk-codec/src/string_layer/bch.rs:673` / `690`: the
  string is lowercased before the `is_ascii` check. This is the same pattern
  as mt #6, which was graded Important. It is a candidate for mk triage.
- md `07d85472`, `crates/md-codec/src/chunk.rs:528` and `codex32.rs:73`: the
  code does `c as u8` on a char. That truncates non-ASCII, so U+0171 becomes
  `q`. This is a candidate for md triage and is unverified.

Counts: 0 Critical, 0 Important in FIX-B scope; 2 untriaged cross-repo candidates (1 likely Important, mk); 2 STOPs.
