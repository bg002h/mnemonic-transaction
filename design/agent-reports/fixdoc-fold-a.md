# FIX-A doc fold: architect rulings Q0, QA1, QA3

Fold commit: 1676bf8 (doc + 3 test rows).

## What changed
- Q0: spec delta lands in the mt copy; section 8 commit-message line added.
- QA1: R2 replaced by the SigCtx rule. Rewrote section 2 sentence, 3.1
  (enum, sig_shape(e, ctx), satisfaction_elements tuples, both guards),
  section 1 table, 4.2 rows, counts (11 -> 14), section 5, section 6 step 10.
  Residual (one-element v0 P2WSH reads as key path) recorded in 3.1; the
  FOLLOWUPS.md line is NOT yet filed (owner mt, ownerless residue).
- QA3: row 03e; STOP 3 marked ruled. STOPs 1-3 all marked ruled.

## RED run (unmodified production code, one capture)
filter ^bughunt_mt_0[123]: 14 tests run: 2 passed, 12 failed.
Failures: 01 01b 01c 01e 01f 02 02b 03 03b 03c 03d 03e. Guards 01d, 02c pass.
Reasons: 01e rc 0 (encoded); 01f wrong line "sighash 0x51" (my test script ends
0x51; the ruling's 0x87 is just a different final byte, same fail-open class);
03e "sighash 0x07". Scratch GREEN with the probe's validate.rs: all 14 pass,
only rows 05/07/10/11/12 fail (fail-fast stopped at 34/251).

## STOP
None. Note: ruling names no guard row for the honest 64-byte 30 3d Schnorr
key path; none added.
