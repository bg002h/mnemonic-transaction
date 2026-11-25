#!/usr/bin/env bash
# Gate for design/FIX_bh1119_mt_output_writes.md (bughunt-1119 FIX-B, mt output
# writes: bh-mt-C2, C3, I4). Run from the repository root on the builder's
# final head, once, to a file:
#
#   bash design/FIX_bh1119_mt_output_writes/gate.sh > /scratch/cache/mt-output-writes-gate.log 2>&1
#
# Runs every step of the CI `test` job (.github/workflows/ci.yml) once, then the
# FIX's own checks, and fails on the first short count or missing line.
#
# WHAT THIS DOES NOT COVER: it does not judge the PROPOSED wording (architect);
# does not run the CI `docs-only` short-circuit job or the release workflow;
# does not port the spec delta to mnemonic-engrave's copy (listed in the build
# report); does not run scripts/check-provenance.sh (needs a sibling checkout);
# does not check the toolkit manual (mt has no page in it); does not test
# Windows or macOS (the rows are #![cfg(unix)]; the read-only-stdout row is
# Linux/Unix fcntl behaviour). The totals are base 3993d0c (523 tests, 67
# refusal tests over 18 ruled refusals) + this FIX; a head that also carries
# another row must add its counts (FIX doc section 10).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=6

fail() { echo "GATE FAIL: $*"; exit 1; }
D=design/FIX_bh1119_mt_output_writes

cargo --version | grep -q '^cargo 1\.85\.0 ' || fail "cargo is not 1.85.0"

# T5 + Brian 2026-11-24: mt-cli pins mnemonic-io-lib 0.1.3 from crates.io.
grep -qx 'mnemonic-io-lib = "0.1.3"' crates/mt-cli/Cargo.toml || fail 'mt-cli does not pin mnemonic-io-lib = "0.1.3"'
[ "$(grep -A2 -x 'name = "mnemonic-io-lib"' Cargo.lock | tr '\n' '|')" = 'name = "mnemonic-io-lib"|version = "0.1.3"|source = "registry+https://github.com/rust-lang/crates.io-index"|' ] || fail "Cargo.lock does not resolve io-lib 0.1.3 from crates.io"
! grep -q 'git+https://github.com/bg002h/mnemonic-engrave' Cargo.lock || fail "Cargo.lock still has an io-lib git source"

# The sidecars are installed unchanged.
cmp -s "$D/output_writes.rs.txt" crates/mt-cli/tests/bh1119_mt_output_writes.rs \
  || fail "tests/bh1119_mt_output_writes.rs differs from the sidecar"
n=$(wc -l < "$D/refusals-toml-append.txt")
tail -n "$n" crates/mt-cli/tests/refusals.toml | cmp -s - "$D/refusals-toml-append.txt" \
  || fail "refusals.toml does not end with the sidecar append"

cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked

out=$(cargo nextest run --locked --build-jobs 6 -j 6 --workspace --no-fail-fast 2>&1) || { echo "$out" | tail -40; fail "nextest"; }
echo "$out" | grep -E 'Summary .* 557 tests run: 557 passed' || fail "expected 557 tests run, 557 passed"

rows=$(cargo nextest run --locked --build-jobs 6 -j 6 -p mt-cli --test bh1119_mt_output_writes 2>&1) || fail "output_writes rows"
echo "$rows" | grep -E '34 tests run: 34 passed' || fail "expected 34 output_writes rows"

cargo test --locked --workspace --doc

cov=$(./scripts/check-refusal-coverage.sh) || { echo "$cov"; fail "refusal coverage"; }
echo "$cov" | grep 'check-refusal-coverage: 76 refusal tests over 19 ruled refusals' || fail "coverage count"

mut=$(./scripts/mutate-refusals.sh) || { echo "$mut"; fail "mutate-refusals"; }
echo "$mut" | grep 'mutate-refusals: all 76 refusal tests go red' || fail "mutation count"

./scripts/journeys.sh
bash ci/docs-only.test.sh
bash ci/code-docs-check.test.sh
bash ci/code-docs-check.sh

# C3 / R5: the false sentence is gone; I4: io-lib same_file has a caller.
# (Every stdout site is held by a C2 row, not by a grep.)
! git grep -n 'left behind' -- crates/mt-cli/src || fail "\"left behind\" is still in src"
[ "$(git grep -c 'same_file(' -- crates/mt-cli/src | awk -F: '{s+=$NF} END {print s+0}')" -ge 1 ] || fail "io-lib same_file has no caller"

echo "GATE PASS"
