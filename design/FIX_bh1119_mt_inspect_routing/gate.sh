#!/usr/bin/env bash
# Gate for design/FIX_bh1119_mt_inspect_routing.md (bughunt-1119 FIX-D, mt
# inspect routing: bh-mt-I3, I5, I6). Run from the repository root on the
# builder's final head, once, to a file:
#
#   bash design/FIX_bh1119_mt_inspect_routing/gate.sh > /scratch/cache/mt-inspect-routing-gate.log 2>&1
#
# Runs every step of the CI `test` job (.github/workflows/ci.yml) once, then the
# FIX's own checks, and fails on the first short count or missing line.
#
# WHAT THIS DOES NOT COVER: it does not judge the PROPOSED wording (rows pin the
# PROPOSED texts of FIX doc section 4 exactly; Q1-Q4 follow Brian's rulings of
# 2026-11-23 and 2026-11-24); does not
# run inspect against a live node (every row is offline); does not run the CI
# `docs-only` short-circuit job or the release workflow; does not check the spec
# delta in mnemonic-engrave or run scripts/check-provenance.sh (needs a sibling
# checkout; run it by hand after the re-sync); does not check the toolkit manual
# (mt has no page in it). Rows 15 and 23 are controls of inline refusals with no
# guard fn, so no mutation control covers them. Rows 28 and 29 (encode and
# encode --qr refuse a vout-less record) are controls: encode already refuses, so
# they pass before the fix. The totals are 88ac6d9 + this
# FIX; a head that also carries rows 35, 38, 39 or another fix/mt-* branch must
# add their counts (FIX doc section 10). Build head (main 0753a5f, which carries
# row 38 and decode-strictness, + this FIX): 459 + 31 + 33 = 523 measured.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "GATE FAIL: $*"; exit 1; }
D=design/FIX_bh1119_mt_inspect_routing

cargo --version | grep -q '^cargo 1\.85\.0 ' || fail "cargo is not 1.85.0"

# The sidecars are installed unchanged.
cmp -s "$D/inspect_routing.rs.txt" crates/mt-cli/tests/bh1119_mt_inspect_routing.rs \
  || fail "tests/bh1119_mt_inspect_routing.rs differs from the sidecar"
n=$(wc -l < "$D/refusals-toml-append.txt")
tail -n "$n" crates/mt-cli/tests/refusals.toml | cmp -s - "$D/refusals-toml-append.txt" \
  || fail "refusals.toml does not end with the sidecar append"

cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked

out=$(cargo nextest run --locked --build-jobs 2 -j 2 --workspace --no-fail-fast 2>&1) || { echo "$out" | tail -40; fail "nextest"; }
echo "$out" | grep -E 'Summary .* 523 tests run: 523 passed' || fail "expected 523 tests run, 523 passed"

rows=$(cargo nextest run --locked --build-jobs 2 -j 2 -p mt-cli --test bh1119_mt_inspect_routing 2>&1) || fail "inspect_routing rows"
echo "$rows" | grep -E '33 tests run: 33 passed' || fail "expected 33 inspect_routing rows"

cargo test --locked --workspace --doc

cov=$(./scripts/check-refusal-coverage.sh) || { echo "$cov"; fail "refusal coverage"; }
echo "$cov" | grep 'check-refusal-coverage: 67 refusal tests over 18 ruled refusals' || fail "coverage count"

mut=$(./scripts/mutate-refusals.sh) || { echo "$mut"; fail "mutate-refusals"; }
echo "$mut" | grep 'mutate-refusals: all 67 refusal tests go red' || fail "mutation count"

./scripts/journeys.sh
bash ci/docs-only.test.sh
bash ci/code-docs-check.test.sh
bash ci/code-docs-check.sh

# R2: routing is by sniff first, then strings_subject (line-start rule); the
# whole-text start test is gone.
! git grep -n 'looks_like_an_mt1_set' -- crates/mt-cli/src || fail "looks_like_an_mt1_set is still in src"
git grep -q 'fn strings_subject' -- crates/mt-cli/src/main.rs || fail "no strings_subject in main.rs"
# R3: inspect no longer reports a PSBT's unsigned template.
! git grep -n 'psbt.unsigned_tx.clone()' -- crates/mt-cli/src/main.rs || fail "inspect still clones unsigned_tx"
[ "$(git grep -c 'extract_tx_unchecked_fee_rate' -- crates/mt-cli/src/main.rs | cut -d: -f2)" -ge 3 ] || fail "fewer than 3 extractions (encode, verify, inspect)"

echo "GATE PASS"
