# FIX-D doc fold: architect rulings QD1, QD2

Doc: design/FIX_bughunt_mt_d.md. No test rows added or changed (the ruling names none; 05h/05i are dropped, never added).

- QD1 (STOP-1): option S. Serde_json moves to runtime deps (Cargo.lock unchanged). Predicate spelled out in section 6 step 2; report.rs:424-425 comment rewrite added to the GREEN commit; option H and its unit-test clause dropped. Section 6 now lists node.rs, Cargo.toml and that comment; section 3 notes the comment change.
- QD2 (STOP-2): duplicates take serde_json's last value; section 2 sentence replaced; no rows.
- QD2 (STOP-3): base is master after A-C, gate expects 0 failures; the ten-failure list in section 8 dropped.
- Section 7 rewritten as "all ruled", citing the sections.

RED run on unmodified production code (log /scratch/cache/bughunt/mt/fixd-fold-red.log), filter `^bughunt_mt_05(|b|c|d|e|f|g)_`: 7 tests run: 2 passed, 5 failed. Failures are 05, 05b, 05c, 05d, 05e; guards 05f and 05g pass. Matches the doc's recorded result.

STOPs: none. The `report.rs` comment re-wrap is mechanical (the ruling gives the text, not line breaks).

Counts: 0 Critical, 0 Important.
