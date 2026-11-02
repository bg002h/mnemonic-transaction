**⚠ 1 new Important (pre-existing, outside R1-R3): mt encode falsely refuses a legacy P2PKH input with an uncompressed pubkey, naming a wrong sighash. Routed as STOP 3.**

# FIX-doc report: mt bughunt FIX-A (encode §8.6 shape)

Branch `fix/bughunt-a` (local, not pushed), commit 6917ffe on base 3bfbc98.
Deliverable: `design/FIX_bughunt_mt_a.md`, plus 8 new rows in
`crates/mt-cli/tests/bughunt.rs`. No production code committed.

## RED evidence (unmodified code)

`cargo nextest run --locked -p mt-cli -E 'test(/^bughunt_mt_0[123]/)'` ran 11:
9 failed, 2 passed.
- 01, 01b, 01c: encode accepts a keyless script-path witness (rc 0). R1.
- 02, 02b: encode accepts a 64-byte `30 3d` SIGHASH_NONE sig (rc 0). R2.
- 03, 03c, 03d: refused "carries no signature" (OP_0, OP_1, PUSHDATA4 end the
  walk). R3.
- 03b: wrong first line ("carries no signature", not "sighash 0x02"). R3 extra
  row.
- 01d (p2wpkh `[sig, pk33]` is not a script path) and 02c (73-byte DER NONE
  still refused) are guards and pass, as intended.

## GREEN evidence (scratch fix, reverted)

With the doc's exact §3 edits, `cargo nextest run --locked -p mt-cli` ran 248
tests: 243 passed. The 5 failures were other FIXes' bughunt rows (05, 07, 10,
11, 12). All 11 FIX-A rows passed, and no existing test regressed.

## Findings

- **Important (new; STOP 3):** a P2PKH scriptSig `<sig ALL> <04.. 65-byte
  pubkey>` is refused with `mt encode: REFUSED — §8.6, input 0 is signed with
  sighash 0x.., not SIGHASH_ALL`, rc 1, because `sig_shape` reads a 65-byte
  non-`0x30` element as an explicit-sighash Schnorr signature. This was
  confirmed by probe on base. No row was committed, because the fix needs a
  design choice. The likelihood test is not met, since the refusal names the
  wrong problem.
- **Note (STOP 1):** R2's 2-byte `30 3d` test refuses about 2^-16 of honest
  64-byte Schnorr signatures with a false sighash text. The existing 65-byte
  `0x30` arm does the same at about 2^-8. Options are in the doc.
- **Note (STOP 2):** the spec is an engrave-owned copy, and the mt CLAUDE.md
  says to change it there. The grade says the work is in the mt repo only.
  The architect picks the repo.
- **Below threshold, noted only:** a 64- or 65-byte P2SH redeem script in a
  scriptSig counts as a signature. This is the spec's own documented heuristic
  limit ("a signature-shaped element the script never checks would pass").

Counts: 0 C, 1 I (new, routed), 0 M, 0 N; 3 STOPs; 8 new rows (6 RED, 2 guards); 11 FIX-A rows, expected matched count 11.
