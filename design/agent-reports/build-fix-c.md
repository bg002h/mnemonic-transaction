# Build report: mt bug-hunt FIX-C

Outcome: all gate steps pass; 15/15 FIX-C rows GREEN (RED beforehand: 15 matched, 15 failed). No STOPs.

## Commits (on origin/main 4af58aa, local only)
```
61e7c4b followups: FIX-C review r1 Minors and Nits as deferred-minor
a586932 fix: refuse --json with --quiet on decode and --quiet on inspect/verify; refusals.toml and coverage exempt for 07d (FIX-C §2.5; R10, R11, QC2)
c38ab29 fix: JSON warnings carry duplicate and correction notices on decode and inspect (FIX-C §2.4; R9, QC3)
2219f14 fix: render_json escapes control characters U+0000-U+001F (FIX-C §2.3; R9, QC5)
959c311 fix: keep the mtl/mti prefix repair when autocorrect then decodes (FIX-C §2.2; R12, QC4)
73200e8 fix: restore_elided no longer panics on a non-ASCII cut point; refuses as unreadable (FIX-C §2.1; R7, QC2)
d27ba16 spec: §10.10b JSON notices, control-char escaping, --json/--quiet and --quiet on inspect/verify refused (FIX-C §1; R9, R10, R11)
6a8c76a tests: remove FIX-C ignores on rows 07, 10, 11, 12 (RED rows live again)
0eb2a49 report: FIX-C doc fold r1 I1-I4 (fold-c-r1)
29639d3 FIX-C doc: fold review r1 I1-I4
fe724b5 report: mt bughunt FIX-C review r1 (0C/4I/2M/7N)
dff16fd report: FIX-C doc fold of architect rulings
00faf16 FIX-C doc: fold architect rulings Q0, QC2-QC5 (+ rows 07c, 07d, 10b, 10d, 10e, 12b)
bd13662 report: FIX-C doc author report (mt bughunt)
502a1aa FIX-C doc + RED rows: mt bughunt #7, #10, #11, #12, R11
```
Doc-side commits (FIX doc, RED rows, author/fold/review reports) were rebased unchanged. Build commits: ignore removal, spec 10.10b, R7 restore_elided, R12 mtl/mti repair, R9 JSON escaping, R9 JSON warnings, R10/R11 guards + refusals.toml + coverage EXEMPT, FOLLOWUPS.

## Ignore removal
Removed exactly four `#[ignore = "RED row owed to FIX-C; its build removes this"]` lines in crates/mt-cli/tests/bughunt.rs (rows 07, 10, 11, 12). FIX-A and FIX-D ignores left.

## Gate (logs in /scratch/cache/bughunt/mt-build-c/; red.log = RED run)
- Steps 1-4 docs-only, fmt, clippy, build: rc 0.
- 5 nextest --locked: 299 run, 299 passed, 4 skipped (A and D ignores).
- 6 doc tests: rc 0. 7 refusal coverage: rc 0.
- 8 mutate-refusals: all 38 refusal tests go red when their check is removed.
- 9 journeys: rc 0. 10 code-docs-check and its tests: rc 0.
- 11 provenance: rc 0 but SKIP (../mnemonic-engrave not found); the expected mismatch was not observed (review M2).
- 12 doc section 3 command: 15 run, 15 passed.

## Deviations
- Base is origin/main, not master (coordinator correction); rebase conflict in bughunt.rs (both branches appended at EOF) resolved by keeping both blocks.
- Provenance step skipped, see above; engrave sync owed by F-321.
- Last GREEN commit message quotes the RED command count (rewritten after the gate; no content change).

## FOLLOWUPS
9 deferred-minor entries filed from fix-review-r1-c.md (slugs prefixed fixc-).

## Owed by the coordinator
mutation-gate.sh, the class A whole-diff review, and filing "C M1".
