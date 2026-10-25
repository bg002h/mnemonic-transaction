r1: 0C/0I/1M/1N

# L3 review r1: mnemonic-transaction PR #5 (B-tx), head 787bc07b7f62cc6b3d79f9e8585bf125aca45fee

Scope: `git diff origin/main...787bc07` (ci.yml `test`, release.yml `build`,
plus the build report), SPEC-after-b §4, §6, §7. Settled items not re-derived.

**The one question:** can the change skip `build` or `test (rust)` steps on an
event whose output is used, on a non-documentation change, or break the tag
path? **No.** Evidence per brief item below.

## 1. Steps that must still run

- release.yml `build`: the only output any later job uses is `Upload artifact`
  (`mt-<target>`), downloaded by `assemble`. `assemble` runs only when
  `startsWith(github.ref, 'refs/tags/') || inputs.tag != '' || inputs.sign_dry_run == 'true'`.
  On each of those events the build legs run in full:
  - tag push: guard runs (`inputs.tag` is null, `null == ''` is true), and
    `ci/docs-only.sh` takes the `push` branch; ref `refs/tags/*` matches
    neither `refs/heads/$DEFAULT_BRANCH` nor `refs/heads/?*`, so
    `full "push to refs/tags/..."`.
  - dispatch with `tag`: guard step `if:` false, `steps.docs.outputs.docs_only`
    is empty, every later `!= 'true'` holds.
  - dispatch with `sign_dry_run` and no tag (or dispatch from a tag ref):
    guard runs, `GITHUB_EVENT_NAME=workflow_dispatch` hits `*) full "event ..."`.
  No cache save, attestation or signing step exists in `build` (attestation,
  signing and upload are all in `assemble`, unchanged).
- ci.yml `test (rust)`: no step produces anything another job consumes
  (ci.yml has one job). Every step after the guard carries the condition
  (diff checked: toolchain, fmt, clippy, build, shells, nextest install,
  test, refusal coverage, refusal mutation, journeys).
- Docs rules are `^design/` and `^[^/]+\.md$`. grep of `scripts/` and
  `crates/` for `design/`, `.md`, `README`, `include_str`: the only reader of
  `design/` is `scripts/check-provenance.sh`, which no workflow runs (no
  caller under `.github`; it is in `ci/code-docs-exempt.txt` for that
  reason). `refusals.toml` lives in `crates/mt-cli/tests/`, not docs.

## 2. CHECK_NAME vs real check-run names

Ran against head cb5d335 (L1 code head), `check-runs?check_name=...&filter=latest`:

```
build mt (linux-x86_64)        => build mt (linux-x86_64):completed/success
build mt (linux                => (none)
build mt (linux-x86_64) (tag)  => (none)
BUILD MT (linux-x86_64)        => (none)
test (rust)                    => test (rust):completed/success
```

The filter is an exact, case-sensitive match. Each leg's CHECK_NAME embeds
its own `matrix.name` (five distinct values), so no other leg, no
`release test (...)` run and no ` (tag)` run can answer for it. A leg
skipped at job level concludes `skipped`, which `passed()` rejects; a leg
docs-skipped green is accepted by design (§4, settled induction).

## 3. Permissions

`build` steps: checkout (contents: read), guard (contents: read for the
compare, checks: read for check runs), toolchain/cross install, builds,
upload-artifact (runtime token, no GITHUB_TOKEN scope). Nothing needs
id-token, attestations or contents: write; those stay at workflow level for
`assemble`, which has no job-level block. ci.yml: the repo's default
workflow permission is `read` (`actions/permissions/workflow`), so the new
block only narrows it; `taiki-e/install-action` reads public releases. L1
(full run on the code head, all five legs and `test (rust)` green)
exercised every step under the new grants.

## 4. ` (tag)` suffix and checkout ref, per event

| event | checked out | check-run name | guard | can stand in? |
| --- | --- | --- | --- | --- |
| pull_request | `github.sha` = test-merge commit (was `refs/pull/N/merge`: same commit, now race-free, and it is the two-parent HEAD A′ needs) | `build mt (X)` | runs | matches CHECK_NAME; correct |
| push main / `ci/**` | `github.sha` (was the branch ref, which can move after the event; now consistent with `PUSH_BEFORE..HEAD`) | `build mt (X)` | runs | correct |
| tag push | `github.ref` (the tag) | `build mt (X) (tag)` | runs, full | never matches CHECK_NAME |
| dispatch, `tag` set | `inputs.tag` | `build mt (X) (tag)`, attached to the dispatch branch's sha (a different tree) | skipped, full | never matches: the case the suffix exists for |
| dispatch, no tag | `github.sha` | `build mt (X)` | runs, full | its tree is its sha's tree; correct |
| dispatch from a tag ref | `github.ref` (the tag) | `build mt (X) (tag)` | runs, full | never matches |

A ` (tag)` run cannot stand in for an untagged one (exact-name filter,
above), and the reverse cannot matter: every ` (tag)` run is a full run
that consults no reference. Legacy unsuffixed dispatch-with-tag runs exist
only on 135a716 and e8fd044 (`gh run list --event workflow_dispatch`),
neither of which is main's tip (402af07); an older base used as a
reference would diff in this PR's workflow edits, so that run goes full.

## 5. ci.yml `test (rust)`

Events: `pull_request` and push to `main`/`ci/**`; no tag, no dispatch.
Every step after the guard is conditioned only on `docs_only != 'true'`, so
it skips only when the guard proved a docs-only diff. `main` is not
protected (`branches/main/protection` 404) and rulesets are `[]`: no
required check. ci.yml has no `needs:`, and no other workflow names
`test (rust)`.

## 6. PR description

Accurate: names `.github/workflows/`, the three skip cases, the
permissions, the checkout and suffix change, and ends with the footer. See N1.

## Minors

- M1: ci.yml comment above `install nextest` still says "a conditional step prints a green tick when it is SKIPPED ... an unconditional step is the only kind whose green means anything", but those steps are now conditional on the docs guard; reword to say they skip only on a proven docs-only change.

## Nits

- N1: PR description says `build` "checks out `github.sha` except for a tag"; a dispatch with `tag` checks out that input tag (and also gets ` (tag)`), worth saying explicitly.
