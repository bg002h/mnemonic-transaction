#!/usr/bin/env bash
# Gate for design/FIX_bh1119_mt_arg_intake.md (bughunt-1119 FIX-H, mt argument
# intake). Run from the repository root on the builder's final head:
#
#   bash design/FIX_bh1119_mt_arg_intake/gate.sh > /scratch/cache/mt-arg-intake-gate.log 2>&1
#
# Runs every step of the CI `test` job (.github/workflows/ci.yml) once, then the
# FIX's own checks, and fails on the first short count or missing line.
#
# WHAT THIS DOES NOT COVER: it does not judge the PROPOSED wording (architect),
# does not run the CI `docs-only` short-circuit job or the release workflow, and
# does not check the toolkit manual (mt has no page in it). The non-UTF-8 rows
# are #[cfg(unix)]; on a non-unix host the row count below is wrong by design.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "GATE FAIL: $*"; exit 1; }

cargo --version | grep -q '^cargo 1\.85\.0 ' || fail "cargo is not 1.85.0"

# The sidecars are the rows and the toml entries, unchanged.
cmp -s design/FIX_bh1119_mt_arg_intake/arg_intake.rs.txt crates/mt-cli/tests/arg_intake.rs \
  || fail "tests/arg_intake.rs differs from the sidecar"

cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked

out=$(cargo nextest run --locked --build-jobs 6 -j 6 --no-fail-fast 2>&1) || { echo "$out" | tail -40; fail "nextest"; }
echo "$out" | grep -E 'Summary .* 424 tests run: 424 passed' || fail "expected 424 tests run, 424 passed"

rows=$(cargo nextest run --locked -p mt-cli --test arg_intake 2>&1) || fail "arg_intake rows"
echo "$rows" | grep -E '22 tests run: 22 passed' || fail "expected 22 arg_intake rows"

cargo test --locked --workspace --doc

cov=$(./scripts/check-refusal-coverage.sh) || { echo "$cov"; fail "refusal coverage"; }
echo "$cov" | grep 'check-refusal-coverage: 59 refusal tests over 18 ruled refusals' || fail "coverage count"

mut=$(./scripts/mutate-refusals.sh) || { echo "$mut"; fail "mutate-refusals"; }
echo "$mut" | grep 'mutate-refusals: all 59 refusal tests go red' || fail "mutation count"

./scripts/journeys.sh
bash ci/docs-only.test.sh
bash ci/code-docs-check.test.sh
bash ci/code-docs-check.sh

# T20's one shared predicate: defined once in read_strings.rs, used by the
# reader and by encode's separator guard.
[ "$(grep -c 'fn is_grouping_whitespace(' crates/mt-cli/src/read_strings.rs)" = 1 ] || fail "predicate not defined once"
grep -q 'is_grouping_whitespace' crates/mt-cli/src/main.rs || fail "separator_guard does not use the predicate"
[ "$(grep -c 'is_grouping_whitespace' crates/mt-cli/src/read_strings.rs)" -ge 2 ] || fail "the reader does not use the predicate"
# T22: argv is read once, as OsString.
! grep -n 'env::args()' crates/mt-cli/src/*.rs || fail "std::env::args() still read"

echo "GATE PASS"
