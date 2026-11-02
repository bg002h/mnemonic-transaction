# Fold report: FIX-C doc, review r1 I1-I4

Fold commit: 6739f6e (on fix/bughunt-c, parent 2d18f81). Only the four
Important fix texts were applied. No Minor or Nit was applied, and no rebase
was done.

## Anchors applied (each matched exactly once)

- I1: `crates/mt-cli/tests/bughunt.rs`, row 10d `is_needed` filter:
  `" corrections"` replaced by `" needed "`.
- I2: `design/FIX_bughunt_mt_c.md`, build order step 1: the "expected to be
  clean" sentence replaced by the conflict-resolution text (same words,
  re-wrapped to the doc's line width).
- I3: `bughunt.rs`, row 07d: the rustfmt-form `flat.contains("read cleanly
  and 1 did not")` assertion added after the first-line assert. §3 07d
  expected-output cell: the whitespace-flattened stderr clause appended.
- I4: §2.5: the `EXEMPT` entry text and the "Row 07d is not given a
  `refusals.toml` entry" paragraph added after the four-entry block. §4: the
  `scripts/check-refusal-coverage.sh` bullet added after the refusals.toml
  bullet.

## RED command (doc §3, run once on unfixed code)

Command: `cargo nextest run --locked --no-fail-fast -E 'test(/^(report::tests::)?bughunt_mt_(07|07b|07c|07d|10|10b|10c|10d|10e|11|11b|r11_.*|12|12b)_/)'`

Output: `/scratch/cache/bughunt/mt-fold-c/red.txt`

- Observed: 15 tests run, 0 passed, 15 failed.
- Stated in doc §3 and build step 2: 15 matched, 15 failed.
- Match: yes.

## Format check

`cargo fmt --check`: exit 0 (output in `/scratch/cache/bughunt/mt-fold-c/fmt.txt`).

## Not run

Only the RED command and fmt check were run, as the brief asks. The GREEN
gate and the other build-order steps were not run.
