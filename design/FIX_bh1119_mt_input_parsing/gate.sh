#!/usr/bin/env bash
# Gate for design/FIX_bh1119_mt_input_parsing.md (bughunt-1119 FIX-C, mt input
# parsing: bh-mt-I1, I2, I20). Run from the repository root on the builder's
# final head, once, to a file:
#
#   bash design/FIX_bh1119_mt_input_parsing/gate.sh > /scratch/cache/mt-input-parsing-gate.log 2>&1
#
# Runs every step of the CI `test` job (.github/workflows/ci.yml) once, then the
# FIX's own checks, and fails on the first short count or missing line.
#
# WHAT THIS DOES NOT COVER: it does not judge the PROPOSED wording (architect);
# does not run the CI `docs-only` short-circuit job or the release workflow;
# does not check the spec delta in mnemonic-engrave or run
# scripts/check-provenance.sh (needs a sibling checkout; run it by hand after
# the re-sync); does not check the toolkit manual (mt has no page in it). The
# totals are 14ba6f7 (row 23 merged) + this FIX; a head that also carries row 21
# must add its counts (FIX doc section 10).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "GATE FAIL: $*"; exit 1; }
D=design/FIX_bh1119_mt_input_parsing

cargo --version | grep -q '^cargo 1\.85\.0 ' || fail "cargo is not 1.85.0"

# The sidecars are installed unchanged.
cmp -s "$D/input_parsing.rs.txt" crates/mt-cli/tests/bh1119_mt_input_parsing.rs \
  || fail "tests/bh1119_mt_input_parsing.rs differs from the sidecar"
n=$(wc -l < "$D/refusals-toml-append.txt")
tail -n "$n" crates/mt-cli/tests/refusals.toml | cmp -s - "$D/refusals-toml-append.txt" \
  || fail "refusals.toml does not end with the sidecar append"

cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked

out=$(cargo nextest run --locked --build-jobs 4 -j 4 --workspace --no-fail-fast 2>&1) || { echo "$out" | tail -40; fail "nextest"; }
echo "$out" | grep -E 'Summary .* 458 tests run: 458 passed' || fail "expected 458 tests run, 458 passed"

rows=$(cargo nextest run --locked --build-jobs 4 -j 4 -p mt-cli --test bh1119_mt_input_parsing 2>&1) || fail "input_parsing rows"
echo "$rows" | grep -E '26 tests run: 26 passed' || fail "expected 26 input_parsing rows"

cargo test --locked --workspace --doc

cov=$(./scripts/check-refusal-coverage.sh) || { echo "$cov"; fail "refusal coverage"; }
echo "$cov" | grep 'check-refusal-coverage: 61 refusal tests over 18 ruled refusals' || fail "coverage count"

mut=$(./scripts/mutate-refusals.sh) || { echo "$mut"; fail "mutate-refusals"; }
echo "$mut" | grep 'mutate-refusals: all 61 refusal tests go red' || fail "mutation count"

./scripts/journeys.sh
bash ci/docs-only.test.sh
bash ci/code-docs-check.test.sh
bash ci/code-docs-check.sh

# T23: base64 is recognised by decoding; the 7-character prefix is gone.
! git grep -n 'cHNidP8"' -- crates/mt-cli/src || fail "a cHNidP8 string constant is still in src"
[ "$(git grep -c 'PSBT_BASE64_STEM' -- crates/mt-cli/src/validate.rs | cut -d: -f2)" -ge 1 ] || fail "the argv classifier does not use PSBT_BASE64_STEM"
# I1/I2: no unchecked single-object parse is left.
! git grep -n 'Psbt::deserialize(' -- crates/mt-cli/src || fail "Psbt::deserialize (no end check) still called"
! git grep -n 'Transaction::consensus_decode(' -- crates/mt-cli/src || fail "Transaction::consensus_decode (no end check) still called"

echo "GATE PASS"
