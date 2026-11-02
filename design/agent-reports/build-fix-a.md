# Build report: FIX-A (mt bughunt #1-#3)

Branch `fix/bughunt-a`, built from 68c4d9c. Local only, nothing pushed.

## Commits
- 1db2391 spec delta: §8.6 "Script path and pushes, exactly" (Q0), with the F-321 line.
- 5fa0873 §3.1: `SigCtx` replaces the length arms of `sig_shape` (QA1, R2 replaced).
- 0b50b95 §3.2: `is_taproot_script_path` takes `>= 2` elements (R1).
- 6c06f1c §3.3: scriptSig walk counts every push opcode (R3); message quotes 18 matched / 18 passed.
- b660e03 FOLLOWUPS: review M1, N1-N3 (`deferred-minor`) and the v0 one-element-witness residual (`open`).
- 7286dc6 EXEMPT the nine bughunt §8.6 shape rows from refusal-coverage (coordinator ruling).

## Counts
- RED (`red-a.log`): `18 tests run: 4 passed, 14 failed`, as the doc states.
- GREEN (`green-a.log`, `10-rows.log`): `18 tests run: 18 passed`.

## Gate (logs in /scratch/cache/bughunt/mt-build-a/)
1. fmt `1-fmt.log`: pass. 2. clippy `2-clippy.log`: pass. 3. build `3-build.log`: pass.
4. workspace nextest `4-nextest.log`: 302 run, 294 passed, 8 failed. All 8 are other FIXes' rows: mt-codec 04, 06, 08; mt-cli 05, 07, 10, 11, 12. No FIX-A row or older test fails.
5. doc tests `5-doc.log`: pass.
6. check-refusal-coverage: first run `6-coverage.log` FAILED (nine rows); after 7286dc6 `6b-coverage.log` passes (34 refusal tests, 18 ruled refusals).
7. mutate-refusals (scratch copy under /scratch/cache): `7-mutate.log` and, after the EXEMPT change, `7b-mutate.log`: all 34 go red.
8. journeys `8-journeys.log`, `8b-journeys.log`: pass.
9. docs-only.test, code-docs-check.test, code-docs-check (`9-*.log`): pass.
10. FIX-A rows by name (`10-rows.log`): 18 matched, 18 passed.
11. Coordinator mutation gate: not run (coordinator's).

## Deviations
- Gate step 6 failed on nine committed rows (01b, 01c, 01e, 01f, 01i, 01j, 02b, 02c, 03b), which the doc did not decide. I stopped; the coordinator ruled option (a), following the FIX-B precedent (aab091f): EXEMPT entries in `scripts/check-refusal-coverage.sh` with the reasons the coordinator gave, `refusals.toml` untouched. The STOP file was deleted uncommitted.
- The v0 one-element residual FOLLOWUPS entry has status `open`; the doc gave none.
- The doc's §4.1 "Tests that change: None" holds; the only non-doc file changed beyond §3 is the coverage script.
