# PR 9: ignore foreign RED rows

Commit: 82d47e9 (on fix/bughunt-b, on top of ba718eb). Only change: 8 `#[ignore = "RED row owed to FIX-X; ..."]` lines in crates/mt-cli/tests/bughunt.rs (A: mt_01,02,03; D: mt_05; C: mt_07,10,11,12).

- fmt --check and clippy -D warnings: clean (cargo 1.85.0).
- Full run (/scratch/cache/bughunt/pr9-ignore/nextest.txt): `284 tests run: 284 passed, 8 skipped`
- Ignored-only run (/scratch/cache/bughunt/pr9-ignore/ignored.txt): `8 tests run: 0 passed, 8 failed, 284 skipped` (exactly the 8 rows, still RED).
- Push of 82d47e9 to origin fix/bughunt-b: succeeded.
- Note: I did not measure the pre-change skipped count; I assumed 0 before.
