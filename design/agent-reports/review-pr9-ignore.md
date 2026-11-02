# Review: PR mt#9 ignore commit 82d47e9 (ba718eb..82d47e9)

Verdict: yes. The commit makes CI green without hiding or weakening any FIX-B row.

- Diff is 8 `#[ignore = "RED row owed to FIX-x; its build removes this"]` lines in `crates/mt-cli/tests/bughunt.rs`, nothing else (diffstat: 1 file, 8 insertions). No test body or assertion changed.
- Each attribute sits directly above its fn: 01, 02, 03 -> FIX-A; 05 -> FIX-D; 07, 10, 11, 12 -> FIX-C. Reasons match the settled row->FIX map.
- No FIX-B row is ignored: cli `bughunt_mt_04_cli_count_mismatch_refusal` and codec 04 (x2), 06 (x2), 08 (x3) all show PASS in nextest.txt (grep). No `#[ignore]` exists in `crates/mt-codec/tests/bughunt.rs` or elsewhere in the cli file beyond the 8.
- No ignored row is one FIX-B made GREEN: ignored.txt shows all 8 FAIL when run with ignored-only, so none is secretly passing. Full run 284 passed, 8 skipped.

Findings: none.

C/I/M/N: 0/0/0/0
