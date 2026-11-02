**⚠ 0 Critical / 3 Important: a keyless P2WSH with a DER-shaped witnessScript still gets a card (I1), and two mutation survivors remove FIX behaviour (I2, I3).**

# Whole-diff review: mt FIX-A, slice s1 (design and threat, executed probes)

Reviewer: s1 (Opus, effort medium). Target `fix/bughunt-a`, diff
`3bfbc98..1b0f7e4`. Scratch copy `/scratch/cache/bughunt/wd-a-s1/src`
(`git archive 1b0f7e4`), own `CARGO_TARGET_DIR`. Probes are PSBTs built in a
scratch `tests/probe_s1.rs` copy of the `bughunt.rs` builders, run through
`mt encode`. First review of this diff (not a re-review).

Fold accounting: this is the first review of the diff, so 0 of the findings
come from a previous fix commit. `build-fix-a.md` does not record the
build's author model and effort.

## Findings (appended as found)

### I1 (Important): a keyless v0 P2WSH whose witnessScript is DER-shaped encodes (wrong card)

- **Where:** `crates/mt-cli/src/validate.rs:1232-1236` (`satisfaction_elements`). In the
  Ecdsa context the last witness element stays in the candidate list.
- **Failure:** the witness is `[32-byte preimage, ws]` against a P2WSH spk.
  - `ws` is `30 3d`, then a 48-byte push, OP_2DROP, OP_NOPs and a trailing `01 01`.
  - It is a 64-byte witnessScript that is valid as a script.
  - `sig_shape` reads it as a DER ECDSA signature with sighash 0x01.
  - `mt encode` exits 0 and prints a card for an input with no signature.
- **Probed lengths:** n = 52, 64, 65 and 73 all exit with rc 0 on 1b0f7e4.
- **Contradictions:** this breaks
  - the new spec sentence (SPEC_mt_v0_1.md §8.6, ~3061-3063): "a 64- or 65-byte witnessScript or public key is not a signature";
  - the FIX table row (FIX_bughunt_mt_a.md:25), which says a P2WSH with a 64-byte witnessScript and no signature is refused with "carries no signature".
- **Why the shape rule misses it:** the 9..=73 `30 <n-3>` rule cannot tell this script from a signature.
- **BIP-141:**
  - line 166: P2WPKH is `<signature> <pubkey>`;
  - line 206: "The last item in the witness (the "witnessScript") is popped off".
  - So the last item of a v0 witness with 2 or more items is never a signature.
- **History:** this was pre-existing on the base for 64/65, but the diff claims to close it.
- **Fix text (code)**, in `satisfaction_elements`, directly after the script-path
  block's `w.pop(); // leaf script`:
  ```rust
          w.pop(); // leaf script
      } else if ctx == SigCtx::Ecdsa {
          // BIP-141: the last item of a v0 witness with two or more items
          // is its witnessScript (P2WSH) or public key (P2WPKH), never a
          // signature.
          w.pop();
      }
  ```
  Checks run on the fix:
  - it compiles (`SigCtx` derives `PartialEq`);
  - `rustfmt --check` and `cargo clippy --locked -p mt-cli --bins -- -D warnings` are clean.
- **Fix text (spec)**, in §8.6, after "In any other witness and in a `scriptSig`
  only a DER ECDSA element `30 <len>` of 9 to 73 bytes including its trailing
  sighash byte counts, …", add:
  "The last element of any other witness is its witnessScript or public key
  (BIP-141) and is never counted."
- **RED row (added to `tests/bughunt.rs`, run as printed):**
  `bughunt_mt_01k_p2wsh_keyless_der_shaped_64_byte_script_refused`.
  - The witness is `[32×0x77, ws64]` against spk `00 20 44×32`.
  - It calls `assert_refused_with(&out, "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature")`.
  - Command: `cargo nextest run --locked -p mt-cli -E 'test(/(^|::)bughunt_mt_0(1k|1l|1m|3f|3g)_/)'`
  - Observed on unmodified 1b0f7e4:
    `FAIL … bughunt_mt_01k_p2wsh_keyless_der_shaped_64_byte_script_refused` /
    `expected exit 1 with \`mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature\`, got Some(0)` /
    `left: Some(0) right: Some(1)`.
    The other four rows pass: `5 tests run: 4 passed, 1 failed`.
- **GREEN with the code fix applied as a scratch change:**
  - The whole mt-cli suite gives `261 tests run: 256 passed, 5 failed`.
  - The 5 failures are 05, 07, 10, 11 and 12, all other FIXes' rows (settled).
  - 01k passes.
- **Probe diff (40 probes, fixed against original):** only
  `p2wsh-keyless-der-shaped-ws-{52,64,65,73}` change, from rc 0 to rc 1
  "carries no signature". Every honest probe still encodes, including P2WPKH, native
  and wrapped multisig P2WSH, and the existing 01d.
- **Product point (for the architect, not a reviewer choice):** fix it in code and spec
  (recommended; the spec already promises it), or narrow the spec sentence to
  "only by shape" and file the residual. Rewording only the spec leaves a card for a
  keyless input, so the code fix is recommended.

### I2 (Important): survivor validate.rs:1186:13 (delete `sig_shape` Taproot arm 65) is fail-open

- **Where:** `validate.rs:1186`, `65 => Some(SigShape::SchnorrExplicit(e[64]))`.
- **Why it matters:** it removes FIX behaviour, namely the "Schnorr explicit" non-ALL refusal and the
  honest 65-byte key path.
- **Failure with the arm deleted (planted, observed):**
  - A script path `[sig64, sig65‖0x02, OP_1, cb65]` encodes with rc 0: a SIGHASH_NONE card. Its 64-byte sig still counts and the 0x02 element is ignored.
  - An honest key path `[sig64‖0x01]` is refused with "carries no signature".
- **Fix text (killing rows, run as printed):** add to `tests/bughunt.rs`
  - `bughunt_mt_01l_taproot_keypath_65_byte_schnorr_all_encodes`: the witness
    is `[64×0x5a ‖ 0x01]` on `p2tr_spk()`, and it asserts `out.status.success()`;
  - `bughunt_mt_01m_taproot_scriptpath_65_byte_none_refused`: the witness is
    `[64×0x5a, 64×0x5b ‖ 0x02, [0x51], control_block_65()]`, and it asserts
    `assert_refused_with(&out, "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL")`.
- **Observed:**
  - On real 1b0f7e4 both rows PASS.
  - With the mutant planted, both FAIL:
    - 01l panics at bughunt.rs:885 (refused);
    - 01m gives `expected exit 1 with \`mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL\`, got Some(0)`.
  - Same command as I1.

### I3 (Important): survivor validate.rs:1265:13 (delete `0x4f` OP_1NEGATE arm) removes FIX rule R3's OP_1NEGATE

- **Where:** `validate.rs:1265-1269`, in `script_sig_pushes`.
- **Failure with the arm deleted:** a scriptSig `OP_1NEGATE <sig> <redeem>` stops at
  0x4f, so its signature is never seen and an honest input is refused. This fails closed, but it
  removes a FIX R3 rule (FIX §3.3), which makes it at least Important under the brief.
- **Why it survived:** no existing row has OP_1NEGATE before a signature. 03, 03b, 03c and
  03d all PASS with the mutant planted.
- **Fix text (killing row, run as printed):** add
  `bughunt_mt_03f_op1negate_before_sig_counts`.
  - The spk is P2SH `a9 14 44×20 87`.
  - The scriptSig is `4f <71-byte DER 30 44 … 01> <redeem>`, with redeem
    `21 02 30×32 ad 4f 87` (pk CHECKSIGVERIFY OP_1NEGATE OP_EQUAL, valid and
    honest).
  - It asserts `out.status.success()`.
- **Observed:**
  - PASS on real 1b0f7e4.
  - FAIL with the arm deleted: it panics at bughunt.rs:930 (refused). The other R3 rows pass.

### M1 (Minor): survivor validate.rs:1264:21 (`b.len() >= 5` guard → true)

- **Failure:** the mutant panics with `thread 'main' panicked at crates/mt-cli/src/validate.rs:1264:59`,
  rc 101, and prints no card, so it fails closed.
- **Unusual conditions:** (1) a scriptSig that ends in OP_PUSHDATA4 with fewer than 4 length
  bytes; (2) such a transaction is consensus-invalid (Core `GetOp` returns false).
- **Suggested killing row:**
  - `bughunt_mt_03g_truncated_pushdata4_refused`: a P2SH input with scriptSig `4e 00`, which asserts the
    "carries no signature" refusal.
  - It PASSES on real code and FAILS under the mutant (`got Some(101)`).

### M2 (Minor): DER-shaped P2SH redeemScript (declared heuristic residual)

The last scriptSig push cannot be dropped the way the I1 fix drops a witness's last element: P2PK and bare multisig end in
a signature. So a keyless P2SH whose redeemScript is `30 <n-3>`-shaped still counts as
signed.
- The base already accepted this variant.
- The tool prints a card only when (1) the redeemScript is deliberately DER-shaped and (2) the
  scriptSig carries no real signature.
- File it as a heuristic residual with the v0 one-element one (b660e03).

### N1 (Nit): validate.rs:1264 `skip + len`

The u32 PUSHDATA4 length is cast to usize, so `skip + len` can overflow on a 32-bit target. Use
`checked_add` or compare against `b.len()` before adding.

## Probe summary (all executed via `mt encode` on 1b0f7e4)

- **Honest inputs, all rc 0:**
  - Taproot:
    - key path with 64-byte and 65-byte (0x01) signatures, each with and without an annex (0x50, BIP-341);
    - script path with a 64-byte signature;
    - script path with a 65-byte 0x01 signature and an annex;
    - a 97-byte control block starting c1.
  - P2SH with an OP_0 dummy (BIP-147), OP_1NEGATE, OP_1 or OP_16 before two signatures.
  - Bare multisig, P2PK, and P2PKH via PUSHDATA4.
  - Wrapped and native multisig P2WSH.
  - SIGHASH_ALL at every DER length 9..=73.
- **Non-ALL inputs, all refused:**
  - flags 0x02, 0x81 and 0x03 at every DER length, both in the witness and in the scriptSig;
  - Taproot 65-byte 0x02 on the key path, on the key path with an annex, and on a script path.
- **Keyless inputs, all refused:**
  - two-element script paths, with and without an annex;
  - a DER-71 element used as the key path or as a script-path argument.
- **The one gap:** I1.
- **Sources checked:** BIP-141 (lines 166 and 206), BIP-341 (lines 63-81: annex, ≥2 elements,
  33+32m control block, `c[0]&0xfe`), BIP-147 (an empty dummy), and Core `script.cpp`
  (`IsPushOnly` accepts up to OP_16; `GetOp` fails on a truncated PUSHDATA4).

C / I / M / N: 0 / 3 / 2 / 1
