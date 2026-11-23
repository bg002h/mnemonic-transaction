#!/usr/bin/env bash
# Gate for the security-sweep FIX `read-args-transaction-verify-only`
# (mt-decode-inspect-ignore-transaction). Run from the repository root on the
# builder's final head, once, to a file:
#
#   bash design/FIX_sec_read_args_transaction_verify_only/gate.sh > /scratch/cache/mt-rat-gate.log 2>&1
#
# Runs every step of the CI `test` job (.github/workflows/ci.yml) once, then the
# FIX's own checks, and fails on the first short count or missing line.
#
# WHAT THIS DOES NOT COVER: it does not run the CI `docs-only` short-circuit
# job or the release workflow; it does not check the toolkit manual (mt has no
# page in it); it does not judge clap's own wording (not mt's text). The totals
# are origin/main 88ac6d9 (459 tests) + this FIX (4 rows); a head that also
# carries another FIX must add that FIX's counts.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH="$HOME/.cargo/bin:$PATH"
# scripts/journeys.sh runs the hard-coded target/debug/mt, so every step builds
# into the worktree's own target/ (a private CARGO_TARGET_DIR would leave
# journeys with no binary, or a stale one).
unset CARGO_TARGET_DIR

fail() { echo "GATE FAIL: $*"; exit 1; }
D=design/FIX_sec_read_args_transaction_verify_only

cargo --version | grep -q '^cargo 1\.85\.0 ' || fail "cargo is not 1.85.0"

# The sidecar is installed unchanged.
cmp -s "$D/sec_read_args_transaction.rs.txt" crates/mt-cli/tests/sec_read_args_transaction.rs \
  || fail "tests/sec_read_args_transaction.rs differs from the sidecar"

cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked

out=$(cargo nextest run --locked --build-jobs 4 -j 4 --workspace --no-fail-fast 2>&1) || { echo "$out" | tail -40; fail "nextest"; }
echo "$out" | grep -E 'Summary .* 463 tests run: 463 passed' || fail "expected 463 tests run, 463 passed"

rows=$(cargo nextest run --locked --build-jobs 4 -j 4 -p mt-cli --test sec_read_args_transaction 2>&1) || fail "sec_rat rows"
echo "$rows" | grep -E '4 tests run: 4 passed' || fail "expected 4 sec_rat rows"

cargo test --locked --workspace --doc

cov=$(./scripts/check-refusal-coverage.sh) || { echo "$cov"; fail "refusal coverage"; }
echo "$cov" | grep 'check-refusal-coverage: 61 refusal tests over 18 ruled refusals' || fail "coverage count"

mut=$(./scripts/mutate-refusals.sh) || { echo "$mut"; fail "mutate-refusals"; }
echo "$mut" | grep 'mutate-refusals: all 61 refusal tests go red' || fail "mutation count"

./scripts/journeys.sh
bash ci/docs-only.test.sh
bash ci/code-docs-check.test.sh
bash ci/code-docs-check.sh

# R1: decode and inspect no longer take the verify-only argument set.
! git grep -n 'Verify(ReadArgs)' -- crates/mt-cli/src/main.rs || fail "verify still takes the shared ReadArgs"
[ "$(git grep -c 'transaction: Option<std::path::PathBuf>' -- crates/mt-cli/src/main.rs | cut -d: -f2)" = 1 ] \
  || fail "expected exactly one --transaction field"

echo "GATE PASS"
