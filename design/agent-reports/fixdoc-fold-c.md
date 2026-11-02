# FIX-C doc fold of architect rulings (X = C)

Outcome: all rulings folded into design/FIX_bughunt_mt_c.md; 15 RED rows, 15 failed on unmodified production code, each for the stated reason. No STOP.

## What each ruling changed
- Q0: S1 resolved to the mt copy; commit-message line `Spec copy: mt is authoritative; engrave sync owed by F-321.` added to the doc. C M1: not folded (Minor, deferred; coordinator files FOLLOWUPS).
- QC2: section 2.1 rewritten (first boundary-safe mt1 line, else refusal with 1.1 text and n = 1-based position); row 07c now pins the exact first line; new row 07d; refusals.toml gains a 1.1 entry for restore_elided (4 entries now).
- QC3: section 2.4 order stated; rows 10b and 10d (inspect) assert the whole array; 10d decode checks relative order (second input adds a duplicate of clean string 3 so both kinds appear).
- QC4: section 2.2 last sentence; R12 amended; row 12b asserts no `you typed l` / `you typed i`.
- QC5: section 2.3 cell lowercase; row 10e asserts `\u0000`, `\u0001`, `\u001f`.
- Count changes: 14 -> 15 in RED command, build order, gate step 12.

## RED run (base 3bfbc98 production code, log /scratch/cache/fixc-red.log)
`cargo nextest run --locked --no-fail-fast -E 'test(/^(report::tests::)?bughunt_mt_(07|07b|07c|07d|10|10b|10c|10d|10e|11|11b|r11_.*|12|12b)_/)'`
15 run, 0 passed, 15 failed. Reasons: 07/07b/07c/07d panic at read_strings.rs:599:22 (rc 101); 10b, 10c, 10d inspect array lacks the entries; 10e `\n` not escaped; 11b/11/r11 rc 0; 12/12b rc 1 refusal. No guard rows.

## Not machine-proved
- 10d's decode ordering branch and 10e's lowercase assertions are unreachable on base (earlier assertions fail first); they are proved at GREEN.
- Gate steps 7-8 (refusal coverage, mutate-refusals) not run; they need the GREEN code.

STOP: none.
Counts: 0 Critical, 0 Important.
