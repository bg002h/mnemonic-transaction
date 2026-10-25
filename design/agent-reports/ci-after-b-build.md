
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

## L2 (docs head 6c685cf04a8b1e8392084b4f59b57aa5cc499d93, previous head cb5d335b8b167fbdcea5fa347785637bff0236cd)

- run 40236583357 job 121299356568 `ci/test (rust)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'test (rust)' passed on it; skipping the guarded steps`
- run 40236583355 job 121299356454 `release/release test (ubuntu-latest)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'release test (ubuntu-latest)' passed on it; skipping the guarded steps`
- run 40236583355 job 121299356610 `release/build mt (linux-x86_64)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'build mt (linux-x86_64)' passed on it; skipping the guarded steps`
- run 40236583355 job 121299356633 `release/build mt (linux-aarch64)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'build mt (linux-aarch64)' passed on it; skipping the guarded steps`
- run 40236583355 job 121299356680 `release/build mt (macos-aarch64)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'build mt (macos-aarch64)' passed on it; skipping the guarded steps`
- run 40236583355 job 121299356697 `release/build mt (macos-x86_64)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'build mt (macos-x86_64)' passed on it; skipping the guarded steps`
- run 40236583355 job 121299356706 `release/build mt (windows-x86_64)`: `docs-only: all 1 files changed since the previous PR head cb5d335b8b167fbdcea5fa347785637bff0236cd are documentation, the merged base 402af07b6267906a8d43d496b23fced60b567e79 is an ancestor of it, and 'build mt (windows-x86_64)' passed on it; skipping the guarded steps`
