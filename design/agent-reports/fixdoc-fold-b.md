# FIX-B doc fold of architect rulings (Q0, QB1)

Fold commit: 2babc10. No STOP, no Critical or Important.

- Q0 (S2 resolved): spec delta edits the mt copy (section 1.1); engrave sync owed by F-321; commit-message section added carrying `Spec copy: mt is authoritative; engrave sync owed by F-321.` Header text and build step 2.1 no longer say a ruling is pending.
- QB1 (S1 resolved): keep `InvalidHrp` for non-ASCII HRP or missing separator; no new text, no production or RED row change.
- RED rows: none added or changed. The 7 FIX-B rows were run once on unmodified code, filtered by exact name: 7 matched, 0 passed, 7 failed (reasons as already stated in the doc, section 3).

Counts: 0 Critical, 0 Important.
