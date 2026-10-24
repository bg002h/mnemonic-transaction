r1: 0C/0I/0M/1N

# L3 delta review: mnemonic-transaction PR #4 (feat/ci-docs-skip), head 48b92962, round r1

Cursory delta review per brief `ci-skip-constellation/briefs/l3-mnemonic-transaction-r1.md`.
Shared files, SPEC, gate, byte identity and L1/L2 taken as settled.

## Deltas checked

1. `ci/docs-only-paths.txt`: unchanged in `ba8ab912..HEAD` (SPEC §3 step 3: the append is
   empty). `ci/code-docs-exempt.txt` exempts only `^scripts/check-provenance\.sh$`. Grep of
   `.github/`, `ci/`, `scripts/`, `crates/`: the script is named only in README.md and in
   itself. No workflow step runs it (ci.yml runs `check-refusal-coverage.sh`,
   `mutate-refusals.sh`, `journeys.sh`; release.yml runs none). It matches SPEC §3's table. OK.
2. Insertion site: release.yml `test`, directly after `Self-test the docs-only guard`, with
   `if: runner.os == 'Linux' && github.event.inputs.tag == ''` as its second line, and before
   `Docs-only change?`. Matches SPEC §2.3. The only site for this repo. OK.
3. release.yml `test`: `name:`, checkout `ref:` and `CHECK_NAME` match SPEC §2.1 exactly.
   Old name `test (<os>)`: no other reference in the repo. `assemble` has `needs: [test, build]`
   (job id, unchanged). ci.yml `test (rust)` is a different job. The Cargo.toml comment
   `test (rust + go)` refers to mnemonic-engrave. `main` is unprotected (filed `cs-r2-m1`). OK.
4. PR description: accurate on rule A′, the code-docs check, the checkout change, the rename and
   the ` (tag)` suffix; states the `.github/` change; links the SPEC. Still a draft. OK.

## Minors and nits

- N1: The PR description says "The changes are byte-identical to its gate's applied copy". The PR
  also adds two `design/agent-reports/` files, which the gate copy does not have. Scope it to
  "the CI changes".
