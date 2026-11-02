# FIX review r1, D: design/FIX_bughunt_mt_d.md at 8c9e830

Reviewer: Opus, effort medium. Brief: coordinator `briefs/bughunt-mt-fix-review-r1.md`.
This is the first review round. The fold under review (ae41cfb) was authored by
Sonnet 5.5; its effort is not recorded in the commit. M1 comes from that fold, because
it introduced the "base after A-C" sentence. M2 and N1 predate it.

Verdict: a Sonnet builder working from the doc alone can apply every fix, and all 7
rows turn GREEN for the stated reason. Each row fails under at least one planted
fault. No correct result changes, and every non-`true` path fails closed to UNKNOWN.

## (a) Fold check (QD1, QD2)

- QD1: option S is carried in §6 step 2 (Cargo.toml move, Cargo.lock unchanged),
  §7 STOP-1 and §3. The `report.rs:424-425` comment text matches the architect's
  wording word for word. Option H and its §8 unit-test clause are gone, and no
  stale H text remains anywhere in the doc.
- QD2: the duplicate-member sentence in §2 matches the ruling text. Rows 05h and 05i
  are dropped (§4, §7). STOP-3 (b) and "0 failures" are in §6, §7 and §8, and the
  ten-failure list is gone.

## (b) Anchors, all resolved on 8c9e830

- `node.rs:158` is `has_txindex`.
- `report.rs:184` is its only caller.
- `report.rs:424` holds the "Hand-rolled" comment.
- `serde_json` is in `[dev-dependencies]`, and Cargo.lock has 1.0.151 at line 611.
- Spec 579-583 is the UNKNOWN row plus the `-txindex` prose.
- `getindexinfo` gets 0 hits in the spec and README.

## (c) Tests can fail

Scratch: `/scratch/cache/bughunt/mt-fix-review-d/repo`. I applied §6 step 2 literally
(Cargo.toml move, comment, predicate, doc paragraph). The same §8 step 10 command,
`cargo nextest run --locked -p mt-cli --test bughunt -E 'test(/^bughunt_mt_05(|b|c|d|e|f|g)_/)'`,
gave the results below. I added `--no-fail-fast` for the plant runs.

| build | result | failing rows |
|---|---|---|
| GREEN as written | 7 run, 7 passed | none |
| P1: original substring | 2 passed, 5 failed | 05, 05b, 05c, 05d, 05e |
| P2: also accepts the string "true" | 6 passed, 1 failed | 05g |
| P3: any index synced | 5 passed, 2 failed | 05, 05e |
| P4: searches deeper than txindex's own `synced` | 6 passed, 1 failed | 05d |
| P5: parse failure falls back to substring | 6 passed, 1 failed | 05c |
| P6: always false | 5 passed, 2 failed | 05b, 05f |

Every row is killed by at least one plant. The fold added no rows. `Cargo.lock` is
unchanged after the move, as the doc claims (git diff is empty).

Unrowed R5 cases, run against the GREEN `mt inspect` with a stub node:

| `getindexinfo` reply | STATUS |
|---|---|
| `{"txindex": {"synced": true}} x` | UNKNOWN |
| a top-level array | UNKNOWN |
| `synced: null` | UNKNOWN |
| `synced: 1` | UNKNOWN |
| `{"txindex": true, "synced": true}` | UNKNOWN |
| `{}` | UNKNOWN |
| pretty synced | PENDING |

## Gate on the scratch copy (A-C absent)

I ran §8 steps 1-9 once to `/scratch/cache/bughunt/mt-fix-review-d/gate.log`:

- fmt rc 0
- clippy clean
- build ok
- nextest: `293 tests run: 283 passed, 10 failed`. The 10 are exactly A/B/C-owned
  rows (01, 02, 03, 04, 06, 07, 08, 10, 11, 12), and all 5 FIX-D failures are gone.
  This matches the baseline 278+15.
- doc tests ok
- check-refusal-coverage ok
- `mutate-refusals: all 34 refusal tests go red`
- journeys pass
- `docs-only.test.sh: 73 passed, 0 failed`

Base simulation: I merged A and B heads onto 3bfbc98. B and C conflict with each
other in `mt-cli/tests/bughunt.rs`; that is C's base question, not D's. I then
cherry-picked D's RED commit, the fold and the GREEN:

- clean on A+B, and separately clean on C's head (including the GREEN `report.rs`
  comment beside C's `esc` change at :432);
- on A+B+D, the 7 rows pass 7/7, and `cargo nextest list` with the filter matches 7.

**Not checkable here:** §6's and §8's "expects 0 failures" on master after A-C, and
the "293 plus the rows A, B and C add" total. A-C GREEN code does not exist, and the
B/C merge needs C's rebase.

## Findings

No Critical, no Important.

### Minor

- M1 (fold ae41cfb). The header and §6 step 1 still say "Branch `fix/bughunt-d`, from
  3bfbc98" and "RED, already on this branch". §6 now says the base is master after
  A-C, but no step says how to move the branch. Suggested text, added to §6 step 1:
  "Before commit 2, rebase `fix/bughunt-d` onto master after A-C merge." I graded
  this Minor rather than Important for two reasons: rebase and merge give the same
  tree, and the cherry-picks above were clean on both A+B and C.
- M2. R5's trailing-text, top-level-array, non-object-txindex and `null`/`1` cases
  have no rows. They hold only because the doc prescribes `from_str`, and the probes
  above give UNKNOWN for each. A wrong PENDING needs two unusual conditions: a
  builder who deviates to a streaming or first-document parse, and a node that
  appends text after valid JSON.

### Nit

- N1. §6 step 2 has `from_str::<Value>(reply.trim())`, but `Node::call` already
  trims, so the second `trim()` is redundant. It is harmless.

Design note (ungraded): none.

r1: 0C/0I/2M/1N
