Full catalog of the constellation's scripts: https://github.com/bg002h/mnemonic-refugium/blob/coordinator/coordinator/scripts/README.md

| script | path | purpose | usage |
| --- | --- | --- | --- |
| `code-docs-check.sh` | `ci/code-docs-check.sh` | Fails if any scanned code file names a file that ci/docs-only.sh treats as documentation (which would let a docs-only change skip the job that reads it). | `bash ci/code-docs-check.sh` |
| `code-docs-check.test.sh` | `ci/code-docs-check.test.sh` | Scenario tests for code-docs-check.sh, one throwaway git repo per row; needs only git and bash. | `bash ci/code-docs-check.test.sh` |
| `docs-only.sh` | `ci/docs-only.sh` | CI guard: writes docs_only=true to $GITHUB_OUTPUT only when every file changed since the reference commit is documentation per ci/docs-only-paths.txt; any doubt writes false. Reads event shape from CI env (PR_BASE_SHA, PUSH_BEFORE, DEFAULT_BRANCH, CHECK_NAME). | `bash ci/docs-only.sh` |
| `docs-only.test.sh` | `ci/docs-only.test.sh` | Scenario tests for docs-only.sh against a scratch file:// origin and a stand-in gh; never touches the real remote. | `bash ci/docs-only.test.sh` |
| `check-provenance.sh` | `scripts/check-provenance.sh` | Compares each file design/PROVENANCE.md lists as copied from mnemonic-engrave against its source; prints SKIP and exits 0 if the source repo is absent. Local only, not run in CI. | `MT_SPEC_SOURCE=/path/to/mnemonic-engrave scripts/check-provenance.sh` |
| `check-refusal-coverage.sh` | `scripts/check-refusal-coverage.sh` | Exhaustiveness gate: crates/mt-cli/tests/refusals.toml entries and the refusal tests must be a bijection. | `scripts/check-refusal-coverage.sh` |
| `journeys.sh` | `scripts/journeys.sh` | Runs the three operator journeys (encode, recover, miscut) end to end against target/debug/mt, printing transcripts and asserting on what the operator sees, not exit codes. | `scripts/journeys.sh` |
| `live-smoke-test.sh` | `scripts/live-smoke-test.sh` | End-to-end against a funded regtest bitcoind: encode a finalized PSBT, verify, inspect with the node, decode, testmempoolaccept. SKIPs unless MT_RT names a bitcoin-cli wrapper; not run in CI. | `MT_RT=/path/to/bitcoin-cli-wrapper scripts/live-smoke-test.sh` |
| `mutate-refusals.sh` | `scripts/mutate-refusals.sh` | For each refusals.toml entry, neuters the named check, runs only that refusal's test and asserts it goes RED, then restores crates/mt-cli/src from a byte copy. | `scripts/mutate-refusals.sh` |
