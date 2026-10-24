
# Re-run after: ci: skip a docs-only push to a PR that already passed (rule A')

PR #4. Applies ci-skip-constellation SPEC.md (GREEN at e8f831d), Part A.

## Change

```
 .github/workflows/release.yml |  13 +++-
 ci/code-docs-check.sh         | 165 ++++++++++++++++++++++++++++++++++++++++++
 ci/code-docs-check.test.sh    | 128 ++++++++++++++++++++++++++++++++
 ci/code-docs-exempt.txt       |   7 ++
 ci/docs-only.sh               | 158 +++++++++++++++++++++++++++++-----------
 ci/docs-only.test.sh          | 103 ++++++++++++++++++++++++--
 ci/shared-ci.sha256           |   4 +
 7 files changed, 526 insertions(+), 52 deletions(-)
```

## Gate

`gate-public: 0 failure(s)`

## Local checks

```
ci/docs-only.sh: OK
ci/docs-only.test.sh: OK
ci/code-docs-check.sh: OK
ci/code-docs-check.test.sh: OK
docs-only.test.sh: 73 passed, 0 failed
code-docs-check.test.sh: 41 passed, 0 failed
code-docs check: ok (41 files scanned, 18 docs files)
```

## L1 (code head e9f952dca7d40a53502bc34562d5f51b189d137c)

```
run 40043011646    release                        pull_request   success    ok
run 40043011654    ci                             pull_request   success    ok
  release/release test (ubuntu-latest)             success    ok
  release/build mt (macos-x86_64)                  success    ok
  release/build mt (windows-x86_64)                success    ok
  release/build mt (linux-x86_64)                  success    ok
  release/build mt (linux-aarch64)                 success    ok
  release/build mt (macos-aarch64)                 success    ok
  release/assemble + release                       skipped    ok (allowed skip)
  ci/test (rust)                                   success    ok
pr-checks: PASS (mnemonic-transaction e9f952dca7d40a53502bc34562d5f51b189d137c)
```
