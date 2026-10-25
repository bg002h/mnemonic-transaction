
# Re-run after: ci: guard the release build jobs with ci/docs-only.sh and ci.yml test (rust)

PR #5. Copies ci-skip-constellation reference/after-b/B-tx (SPEC-after-b.md).

## Change

```
 .github/workflows/ci.yml      | 23 +++++++++++++++++++++++
 .github/workflows/release.yml | 30 ++++++++++++++++++++++++------
 2 files changed, 47 insertions(+), 6 deletions(-)
```

## Gate

`make-after-b-reference --check: PASS (12 files)`

## Local checks

```
ci/docs-only.sh: OK
ci/docs-only.test.sh: OK
ci/code-docs-check.sh: OK
ci/code-docs-check.test.sh: OK
docs-only.test.sh: 73 passed, 0 failed
code-docs-check.test.sh: 41 passed, 0 failed
code-docs check: ok (41 files scanned, 20 docs files)
PASS B-tx .github/workflows/ci.yml: exactly the §4.2 edits to test
PASS B-tx .github/workflows/release.yml: exactly the §4.2 edits to build
after-b-check B-tx: PASS
```

## L1 (code head cb5d335b8b167fbdcea5fa347785637bff0236cd)

```
run 40236022051    release                        pull_request   success    ok
run 40236022011    ci                             pull_request   success    ok
  release/release test (ubuntu-latest)             success    ok
  release/build mt (linux-aarch64)                 success    ok
  release/build mt (macos-x86_64)                  success    ok
  release/build mt (macos-aarch64)                 success    ok
  release/build mt (windows-x86_64)                success    ok
  release/build mt (linux-x86_64)                  success    ok
  release/assemble + release                       skipped    ok (allowed skip)
  ci/test (rust)                                   success    ok
pr-checks: PASS (mnemonic-transaction cb5d335b8b167fbdcea5fa347785637bff0236cd)
```
