**⚠ Important findings (2 Important, 1 survivor-grade batch)**

# Whole-diff FIX-A, slice s2 (tests-can-fail), Sonnet 5.5, medium

Method: scratch copy of `1b0f7e4` at `/scratch/cache/bughunt/wd-a-s2/w` (own
`CARGO_TARGET_DIR`), 18 FIX-A rows green at baseline. Script
`/scratch/cache/bughunt/wd-a-s2/tools/mut.py` plants 43 single-site defects in
`validate.rs` and runs the 18 rows (logs `mut.log`, `mut2.log`). Then 11 killing
rows (`s2_*`, appended to the scratch `bughunt.rs`) were written and the
survivors re-run. All 11 pass on unmodified code. Re-review note: this is the
first s2 pass; nothing here comes from a previous fix round.

## Planted defects the existing rows kill (stated reason confirmed by the failing row names)
- ctx always Ecdsa: 01g, 01h. Always Taproot: 01d, 01e, 01f, 01i, 02, 02b, 02c.
- ctx without `len()==1`: 01g. Without script-path: 01h. Computed after the pops: 01h.
- `is_taproot_script_path` `>= 3`: 01j. Control-block shape dropped: 01d, 02, 02b, 02c.
- Ecdsa range upper 72: 02c. `0x30` check dropped: 03c. scriptSig ctx Taproot: 03, 03b-03e.
- Push arms: `OP_0` removed or skip wrong: 03, 03b. `0x4e` removed, BE length, skip 4: 03d. `0x51..=0x60` removed or start 0x52: 03c.
- Guards given a fixed ctx: killed by 01d-01i, 02*, 03*.

## Important

### I1. The three gate survivors, graded (each removes a FIX behaviour)
All three are killed by the rows below (run, all pass unmodified; each mutant fails them).

1. `validate.rs:1186:13` (the `65` Taproot arm; planted as `schnorr65_none`): no row
   exercises a 65-byte Schnorr element with explicit sighash. Not equivalent:
   the element becomes `None`, so a SIGHASH_NONE key-path signature is refused with
   the wrong text ("no signature"), and an honest SIGHASH_ALL 65-byte one is refused.
   Fix text, add to `bughunt.rs` (helper `schnorr65(first, flag)` builds
   `first, 63 x 0x5a, flag`):
   `s2_a`: `encode_psbt(&finalized(p2tr_spk(), None, Some(Witness::from_slice(&[schnorr65(0x5a,0x02)]))))`
   then `assert_refused_with(&out, "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL")`;
   `s2_a2`: the same with flag `0x01` must `status.success()`. Observed: both PASS
   on 1b0f7e4; mutant fails both.
2. `validate.rs:1265:13` (`OP_1NEGATE` arm; `no_4f`): not equivalent. Row
   `s2_b`: P2PKH scriptSig `4f 47 <sig ALL 71B> 21 <pk33>` must encode. Observed
   PASS; mutant FAILS (refused "no signature"). The pushed value (`0x81` vs empty,
   `4f_pushes_empty`) is equivalent: neither is signature-shaped in the Ecdsa ctx.
   Note `OP_16` (0x60, the range's top) is also unpinned: `51_60_to_5f`
   survived the 18 rows; `s2_b2` (`60 47 <sig> ...` encodes) kills it. The
   spec's "any other opcode ends the walk" is unpinned too (`61_incl` survived:
   0x61 read as a push); `s2_b3` (`61 47 <sig ALL>` refused with the no-signature
   text) kills it.
3. `validate.rs:1264:21` (`b.len() >= 5` guard to `true`): not equivalent. With a
   truncated `OP_PUSHDATA4` the code indexes `b[1..5]` and PANICS (exit 101,
   not a refusal). Same hole for the pre-existing `0x4c` and `0x4d` guards
   (`4c_guard_true`, `4d_guard_true` also survived). Row `s2_c`: for each scriptSig
   in `[4e 01]`, `[4d 01]`, `[4c]` on a P2PKH input, `assert_refused_with(&out,
   "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature")`.
   Observed PASS; each of the three mutants FAILS it.

### I2. Row 01c cannot detect annex handling; the annex and leaf-pop paths are unpinned
`bughunt_mt_01c_keyless_scriptpath_with_annex_refused` passes with the annex
strip deleted (`no_annex`): `[OP_TRUE, cb, 50 aa]` is then read as a non-script
witness with nothing signature-shaped, which refuses with the same text. So the
FIX table's annex row and the spec's "after removing an annex" sentence are
checked by nothing that can fail. Further survivors of the 18 rows:
`annex_ge1` (strip with one element: an honest 64-byte Schnorr key-path signature
beginning `0x50` is stripped to nothing and refused), `annex_ge3`, `nopop_leaf`
(leaf script left in the list: a keyless script path whose leaf script is 64 or 65
bytes reads the script as a Schnorr signature and ENCODES, which is exactly bughunt
#1's hole) and `scriptpath_ge1` (a one-element 65-byte key-path signature
starting `0xc0`/`0xc1` is read as a control block and refused).
Fix text, add (all PASS on 1b0f7e4, each named mutant FAILS its row):
- `s2_d`: witness `[64x0x5a, 0x51, control_block_65(), 50 aa]` on `p2tr_spk()` encodes (kills `no_annex`).
- `s2_d2`: `[64x0x5a, 50 aa]` encodes (kills `annex_ge3`, `no_annex`).
- `s2_d3`: `[64x0x50]` encodes (kills `annex_ge1`).
- `s2_e`: `[schnorr65(0xc0, 0x01)]` encodes (kills `scriptpath_ge1`, `schnorr65_none`).
- `s2_f`: for n in 64, 65, witness `[leaf, control_block_65()]` with `leaf = n x 0x61` ending `0x51` is refused with the no-signature text (kills `nopop_leaf`).

## Minor
- M1. `scriptpath_no_mod` (`(len-33)%32==0` removed) survives: only a v0 witness whose last element starts `0xc0`/`0xc1` with a non-`33+32m` length is affected; fails closed in the other direction. No row; add one only if wanted.
- M2. `ecdsa_range_9` (lower bound 9 to 10) and `ecdsa_nolen` (`e[1]==n-3` dropped) survive; pre-existing lines kept by the FIX, no row pins a short or length-mismatched DER element.

## Nit
- N1. `sigshape_old_arms` (restoring `65 if e[0]==0x30 => Ecdsa` in the Taproot arm) survives and is EQUIVALENT: `Ecdsa(b)` and `SchnorrExplicit(b)` share `sighash()` and both count as "a signature".
- N2. `4f_pushes_empty`: equivalent, see I1.2. `ctx_scriptpath_len1only`: equivalent for every shape the rows can build.

The `s2_*` rows are in the scratch file `/scratch/cache/bughunt/wd-a-s2/w/crates/mt-cli/tests/bughunt.rs` (after the line `// ---- s2 killing rows`); the worktree was not touched.

2 Important / 0 Critical: C/I/M/N = 0 / 2 / 2 / 2
