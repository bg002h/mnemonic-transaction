**⚠ 1 Important: QA1's Taproot context and R1's two-element script path have no row that fails if the builder gets them wrong.**

# FIX review r1: FIX-A (`design/FIX_bughunt_mt_a.md`, head 1ce9cfc)

Reviewer: Opus, effort medium. Scratch copy: `/scratch/cache/bughunt/mt-fix-review-a/src` (branch from 1ce9cfc). Applied §3 exactly as written; the result is saved as `validate.fixed.rs`.

## Executed evidence

- RED on base 3bfbc98 plus the rows: `14 tests run: 2 passed, 12 failed`. This matches §4.2 exactly.
- GREEN after §3: `14 tests run: 14 passed`. The exact-name 01b command runs 1 test.
- Full workspace after §3, `cargo nextest run --locked --no-fail-fast`: `298 run, 290 passed, 8 failed`. All 8 failures are rows owned by other FIXes:
  - mt-codec: 04, 06, 08.
  - mt-cli: 05, 07, 10, 11, 12.
  - No FIX-A row and no pre-existing test fails.
- Plants into the fixed code, mt-cli no-fail-fast (the baseline is the 5 other-FIX failures):

  | Plant | What it changes | Caught by the doc's rows? |
  |---|---|---|
  | P1 | witness ctx is always Taproot | yes, 78 failures |
  | P2 | scriptSig ctx is Taproot | yes (03, 03b–03e) |
  | P7 | no `0x00` arm | yes (03, 03b) |
  | P8 | no `0x4e` arm | yes (03d) |
  | P9 | no `OP_1`–`OP_16` arm | yes (03c) |
  | P10 | Ecdsa ctx also accepts a 64-byte element | yes (02, 02b, 01e) |
  | P3 | drop `w.len() == 1`, so key paths become Ecdsa | **no** |
  | P5 | a `30 3d` test is added inside Taproot | **no** |
  | P6 | `>= 2` reverted to `>= 3` | **no** |
  | P11 | ctx computed after the script-path pops | **no** |

  See I1 for the four that are not caught.
- Controller's question on 01f (0x51) against the ruling's example (0x87): on base, both are refused with rc 1, giving "sighash 0x51" and "sighash 0x87" respectively. Both are wrong-text refusals that **fail closed**, and they are equivalent for RED. The fold report's phrase "same fail-open class" is wrong (N3). The real 65-byte fail-open is a two-element v0 witnessScript whose last byte is `0x01`: base gives rc 0, and after §3 it is refused. No row covers it (I1 adds 01i).
- Controller's question on an honest 64-byte Schnorr key-path signature beginning `30 3d`:
  - After §3 it encodes, with or without an annex. A 65-byte ALL key path also encodes, and so does a two-sig script path. P2SH-P2WPKH encodes.
  - However, no row pins any of these: P3 refuses every honest taproot key-path spend, and the whole mt-cli suite stays green. **Yes, the doc leaves that wrong result untested** (I1).

## Fold check (a)

- Q0 is carried: §2 targets the mt copy, the F-321 sync is noted, and §8 has the commit line.
- QA1 is carried exactly: SigCtx, the ctx rule placed after the annex strip and before the pops, the `9..=73` DER arm, the old 64/65 arms removed, and rows 01e and 01f.
- QA3 is carried: row 03e is the scriptSig-side guard.
- The fold has no carry defect. Its gap is in coverage (I1).

## Important

### I1. QA1's Taproot context and R1 are unpinned: four mis-transcriptions pass every row

**Location.** The fault sits in two spans of the FIX doc: the rule text and the row table.

- The rule text is FIX doc §3.1, lines 98–99 (`let ctx = if w.len() == 1 || is_taproot_script_path(&w) ...`, computed "before the script-path pops"), and §3.2, line 128 (`>= 2`).
- The row table is §4.2, lines 172–208, plus the count in §6 step 10, line 243.

**Failure scenarios.** Each was run with the plant in the fixed scratch code and `cargo nextest run --locked --no-fail-fast -p mt-cli`. Every plant leaves the suite green apart from the 5 other-FIX rows.

- **P3** (key-path ctx lost, for example `||` becomes `&&` or the `w.len() == 1` term is dropped): every honest taproot key-path spend is refused. A 64-byte signature becomes "carries no signature". A `30 3d .. 02` signature becomes "signed with sighash 0x02, not SIGHASH_ALL". This is a wrong refusal on the most common spend type, and it is exactly the misfire QA1 was written to remove.
- **P5** (a builder keeps the old `30 3d` test inside Taproot): an honest Schnorr signature beginning `30 3d` is refused "sighash 0x02".
- **P11** (ctx computed after the pops, where `w.len()` is the remaining signature count): a script path with two Schnorr signatures, `[sig64, sig64, script, cb]`, is refused "carries no signature".
- **P6** (R1 not applied, so `>= 3` stays): rows 01, 01b and 01c, labelled R1, still pass, because the Ecdsa context alone refuses `[OP_TRUE, cb]`. A two-element keyless script path whose script is DER-shaped then **encodes**, rc 0. That is a fail-open. The example is the 49-byte script `30 2e` followed by 47 bytes of `0x01`. So rows labelled R1 cannot fail for R1.

**Fix text.**

1. §4.2: add these four rows to `crates/mt-cli/tests/bughunt.rs`, placed immediately after `bughunt_mt_01f_p2wsh_keyless_65_byte_script_refused` and before the `// ---- #5` divider, exactly as follows. The code is fmt-clean, checked with `cargo fmt --check`.

```rust
/// QA1 guard (GREEN before and after): a taproot key-path witness has one
/// element, so its context is Taproot; an honest 64-byte Schnorr signature
/// that happens to begin `30 3d` and end `0x02` is SIGHASH_DEFAULT and encodes.
#[test]
fn bughunt_mt_01g_taproot_keypath_303d_schnorr_encodes() {
    let mut sig = vec![0x30u8, 0x3d];
    sig.extend([0x5au8; 61]);
    sig.push(0x02);
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[sig])),
    ));
    assert!(
        out.status.success(),
        "an honest key-path Schnorr signature was refused:\n{}",
        s(&out.stderr)
    );
}

/// QA1 guard (GREEN before and after): the context is computed before the
/// script-path pops, so a two-signature script path `[sig64, sig64, script,
/// control block]` is Taproot and both 64-byte signatures count.
#[test]
fn bughunt_mt_01h_taproot_scriptpath_two_sigs_encodes() {
    let w = bitcoin::Witness::from_slice(&[
        vec![0x5au8; 64],
        vec![0x5bu8; 64],
        vec![0x51u8],
        control_block_65(),
    ]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert!(
        out.status.success(),
        "a signed taproot script path was refused:\n{}",
        s(&out.stderr)
    );
}

/// QA1: a 65-byte witnessScript whose last byte is 0x01 (here the data push
/// `01 01`) is not a Schnorr signature with SIGHASH_ALL; the keyless input is
/// refused with the "no signature" text.
#[test]
fn bughunt_mt_01i_p2wsh_keyless_65_byte_script_ending_01_refused() {
    let mut ws = vec![0xa8u8, 0x20];
    ws.extend([0x66u8; 32]);
    ws.push(0x88);
    while ws.len() < 63 {
        ws.push(0x61);
    }
    ws.extend([0x01, 0x01]);
    assert_eq!(ws.len(), 65);
    let mut spk = vec![0x00, 0x20];
    spk.extend([0x44u8; 32]);
    let w = bitcoin::Witness::from_slice(&[vec![0x77u8; 32], ws]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// R1: a two-element script path `[script, control block]` is a script path,
/// so its script is popped and never read as a signature, even when the
/// script (here a 49-byte push whose bytes are DER-shaped and end 0x01) looks
/// like an ECDSA signature with SIGHASH_ALL.
#[test]
fn bughunt_mt_01j_two_element_scriptpath_der_shaped_script_refused() {
    let mut script = vec![0x30u8, 0x2e];
    script.extend([0x01u8; 47]);
    assert_eq!(script.len(), 49);
    let w = bitcoin::Witness::from_slice(&[script, control_block_65()]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}
```

2. §4.2 table: append these rows after the `03e` row.

   | row | ruling | asserts | unmodified result |
   |---|---|---|---|
   | `bughunt_mt_01g_taproot_keypath_303d_schnorr_encodes` | QA1 guard | p2tr key path `[64-byte 30 3d .. 02]` encodes, rc 0 | PASS (guard) |
   | `bughunt_mt_01h_taproot_scriptpath_two_sigs_encodes` | QA1 guard (ctx before the pops) | p2tr `[sig64, sig64, OP_TRUE, cb65]` encodes, rc 0 | PASS (guard) |
   | `bughunt_mt_01i_p2wsh_keyless_65_byte_script_ending_01_refused` | QA1 | v0 P2WSH `[32 bytes, 65-byte script ending 01 01]`: rc 1, first line `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature` | FAIL: rc 0 (encode accepted) |
   | `bughunt_mt_01j_two_element_scriptpath_der_shaped_script_refused` | R1 | p2tr `[49-byte DER-shaped script, cb65]`: same line, rc 1 | FAIL: rc 0 (encode accepted) |

3. In the ruling column of rows 01, 01b and 01c, replace `R1` / `R1 (annex)` with `R1 + QA1` / `R1 + QA1 (annex)`. Then add this sentence after line 197: "Rows 01, 01b and 01c are refused by the SigCtx rule even without R1; R1 itself is pinned by `01j`. Rows `01g` and `01h` guard the Taproot side of the rule: a key path is Taproot, and the context is fixed before the pops."

4. Line 172: replace "plus two guards" with "plus four guards".

5. Lines 205–208: change the expected counts.
   - Before §3: replace with `18 tests run: 4 passed, 14 failed`. Failed: 01, 01b, 01c, 01e, 01f, 01i, 01j, 02, 02b, 03, 03b, 03c, 03d, 03e. Passed: the guards 01d, 01g, 01h, 02c.
   - After §3: replace with `18 tests run: 18 passed`.
   - Replace "matches all 14 rows" with "matches all 18 rows".

6. Line 243: replace `expected matched count 14, 14 passed` with `expected matched count 18, 18 passed`.

**Verification.** This was run as printed in the scratch copy with the four rows added.

- After §3: `18 tests run: 18 passed`.
- On base 3bfbc98's `validate.rs`: `18 tests run: 4 passed, 14 failed`. The passes are 01d, 01g, 01h and 02c. 01i and 01j fail at the rc assertion.
- Each plant fails exactly one new row:

  | Plant | Fails | Count |
  |---|---|---|
  | P3 | 01g | 17 passed, 1 failed |
  | P5 | 01g | 17 passed, 1 failed |
  | P6 | 01j | 17 passed, 1 failed |
  | P11 | 01h | 17 passed, 1 failed |

- Full mt-cli after §3 with the rows: `256 run, 251 passed, 5 failed`. The 5 are the other-FIX rows 05, 07, 10, 11 and 12.

## Minor

- M1. §2 spec delta, the sentence "a 64- or 65-byte witnessScript or public key is not a signature" overclaims. A v0 witnessScript of 64 or 65 bytes shaped `30 <n-3> .. 01` (a crafted push-48 script) reads as an ECDSA ALL signature, so a keyless spend encodes.
  - Unusual conditions: (1) a keyless v0 P2WSH; (2) a witnessScript crafted to be DER-shaped. It sits inside §8.6's declared heuristic.
  - Fix: replace that clause with "a 64- or 65-byte witnessScript or public key is not read as a Schnorr signature".

## Nit

- N1. §3.3 says "add these arms before `0x01..=0x4b`" but lists `0x4e`, `0x4f` and `0x51..=0x60`, which the order line then places later. This is harmless, because the patterns are disjoint (pre-fold text).
- N2. §2 spec delta: "`30 <len>`" never says that len is the element length minus 3, which is what the code checks (pre-fold text).
- N3. `fixdoc-fold-a.md` line 18: "same fail-open class" is wrong. On base, both 0x51 and 0x87 fail closed with the wrong text, as the ruling says ("refused with the wrong text"). The doc's 01f row text is correct.

## Provenance

Fold 1676bf8 was authored by Claude Sonnet 5.5 (effort unknown). It accounts for I1 (its SigCtx text and rows), M1 (it rewrote the sentence) and N3 (its report). N1 and N2 pre-date it.

Design concern (ungraded): the witness-shape context remains a guess wherever the PSBT carries `witness_utxo`. Classifying by the spent script would make the context exact and close the one-element v0 residual. That residual is already filed by the controller.

r1: 0C/1I/1M/3N
