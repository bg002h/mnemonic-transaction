# Bug report: bg002h/mnemonic-transaction @ 56886b8

## Summary
- **Scope:** all 18 Rust source files, about 9,600 lines, in `mt-codec` and `mt-cli`.
- **Agents:** 8 slices, each read by a reader and a second reviewer. All finished.
- **Result:** 12 suspicions were rated 3 or higher, and **all 12 were confirmed**: 9 demonstrated with failing tests and 3 by reading.
- **How they were tested:** the crates don't build offline because of a missing git dependency, so each test is a standalone `rustc` copy of the code in question. For #4, the prover compiled a copy of the whole `mt-codec` crate and called the public `encode`/`decode`. Tests are under `/tmp/hunt-mt/prover-N/` in the sandbox.

Reviewers checked these and found them correct: the BCH generators (against BIP-93), the masks and length ranges, the NUMS constants (by SHA-256 in Python), the header and chunk layout, and the decoder.

## Confirmed bugs (worst first)

### Medium

**1. Signature check: a keyless taproot script-path spend passes as "signed"** (demonstrated)
- Where: `crates/mt-cli/src/validate.rs` `is_taproot_script_path`, line 1229. It requires `w.len() >= 3`.
- Input: a taproot input whose leaf needs no stack items, e.g. `OP_TRUE`, so the witness is `[script, 65-byte control block]`.
- Should: refuse with "carries no signature". The docstring says control blocks are never counted as signatures.
- Does: the control block is counted as a Schnorr signature, and its last byte as the sighash. If that byte is `0x01`, an input that commits to nothing is accepted.
- Test: `prover-16/t.rs`.
- Fix: change the check to `w.len() >= 2`.

**2. Signature check: a 64-byte ECDSA signature is never sighash-checked** (demonstrated)
- Where: `validate.rs` `sig_shape`, line 1175. The 64-byte arm comes before the DER arm.
- Input: a valid 64-byte DER ECDSA signature ending in `0x02` (SIGHASH_NONE).
- Should: be classed as ECDSA with sighash 2, which the rule that every signature must be SIGHASH_ALL (§8.6) would then refuse.
- Does: it is classed as a default Schnorr signature with no sighash, so the check is skipped. Any 64-byte non-signature element, such as a preimage, also counts as a signature.
- Test: `prover-11/t.rs`.
- Fix: only treat a 64-byte element as Schnorr if it doesn't start with `30 3d`.

**3. Legacy P2SH multisig inputs are refused as unsigned** (demonstrated)
- Where: `validate.rs` `script_sig_pushes`, line 1241. There is no arm for `OP_0`, so parsing stops at the first byte.
- Input: a finalized 2-of-3 P2SH scriptSig, `00 <sig> <sig> <redeem>`.
- Should: 4 elements, and the signatures are then counted and their sighash checked.
- Does: 0 elements, so `has_signature_guard` refuses a validly signed transaction.
- Test: `prover-6/t.rs`.
- Fix: add `0x00 => (1, 0)`.

**4. Chunks from two different transactions can be silently joined** (demonstrated)
- Where: `crates/mt-codec/src/string_layer/pipeline.rs` `decode`, line 291. Only `chunk_set_id` is compared; each chunk's `count` is never checked.
- Input: chunk 0 of a 2-chunk set plus chunk 1 of a 3-chunk set with the same 20-bit set id.
- Should: an error.
- Does: Ok, with 35 bytes of one transaction followed by 34 bytes of the other.
- Test: `prover-13/codec/tests/count_mismatch.rs`.
- Fix: refuse when `c.header.count != count`.

**5. `has_txindex` reports txindex as ready while it is still syncing** (demonstrated)
- Where: `crates/mt-cli/src/node.rs`, line 160. It runs two separate substring tests over the whole `getindexinfo` reply.
- Input: txindex `synced:false` while a block filter index is `synced:true`.
- Does: returns true, so a lookup that finds nothing in the unfinished index is read as proof the transaction isn't on chain, giving a wrong PENDING or DEAD status.
- Test: `prover-10/t.rs`.
- Fix: check `synced` only inside the txindex object, or parse the JSON.

### Low

**6. Kelvin sign (U+212A) is accepted as `k`** (demonstrated)
- Where: `mt-codec/src/string_layer/bch.rs` `decode_string`, line 685. Full Unicode lowercasing runs before the ASCII check.
- Input: a valid mt1 string with one data `k` replaced by U+212A.
- Should: InvalidChar.
- Does: decodes with 0 corrections.
- Test: `prover-12/sa/lib.rs`.
- Fix: reject non-ASCII first, then use `to_ascii_lowercase`.

**7. `read_strings` panics on a non-ASCII character near the start of a line** (demonstrated)
- Where: `crates/mt-cli/src/read_strings.rs` `restore_elided`, line 599. It slices `full[..11]` at a byte offset.
- Input: `MT1aaaaaaaéqqqqqqqqqq`.
- Should: refuse.
- Does: panics because byte 11 falls inside `é`.
- Test: `prover-5/t.rs`.
- Fix: also require `full.is_char_boundary(11)` before slicing.

**8. `invariant_prefix` panics on short input** (demonstrated)
- Where: `pipeline.rs`, line 367. It slices `symbols[..8]` with no length check, and the CLI calls it in three places.
- Input: `invariant_prefix("mt1qq")`.
- Should: return a too-short error.
- Does: panics with "range end index 8 out of range".
- Test: `prover-7/standalone.rs`.

**9. The offline warning says the fee is UNKNOWN even when the report shows one** (confirmed by reading)
- Where: `crates/mt-cli/src/report.rs` `no_node_warning`, line 592. The function is never given the fee.
- Does: an offline inspect of a PSBT with txid-bound values prints a FEE row with an amount, then a warning saying the fee is unknown.
- Fix: pass in whether the fee is known.

**10. `mt inspect --json` leaves out the duplicate-chunk and correction-margin warnings** (confirmed by reading)
- Where: `crates/mt-cli/src/main.rs`, around lines 1641-1672. `decode --json` also leaves out the margin report, including "NO MARGIN LEFT".
- Does: these notices are printed nowhere, though the tool promises warnings are never suppressed.
- Fix: add them to the JSON `warnings` array.

**11. `mt decode --json --quiet` silently ignores `--json`** (confirmed by reading)
- Where: `main.rs`, line 1274. The JSON branch sits inside `if !args.quiet`.
- Does: exits 0 with only the hex and no JSON.
- Fix: add `conflicts_with = "quiet"` to the `json` flag.

**12. A misread separator plus one confusable character is refused, though autocorrect could fix both** (confirmed by reading)
- Where: `read_strings.rs`, around line 113. The `mtl`→`mt1` repair is kept only if the string decodes before transliteration.
- Input: a plate with `mt1` read as `mtl` and one `6` read as `b`.
- Does: refused as having a "SECOND defect".
- Fix: also keep the repair when any `positional_autocorrect` candidate decodes.

## Rejected
None.

## Not checked
About 35 suspicions were rated 2 or lower and were not sent to provers. The notable ones:
- **`validate.rs`:**
  - Input and output totals are added without overflow checks (`:151`, and `report.rs:277`), so the fee checks could be bypassed.
  - The BIP-341 annex is stripped from every witness, not just taproot ones (`:1208`).
  - Taproot leaf versions other than 0xc0/0xc1 aren't recognised.
- **`main.rs`:**
  - `verify --transaction` compares only txids.
  - Bytes after a valid transaction are not rejected (`:2035`).
- **`node.rs`:**
  - `testnet4` is not mapped to a network.
  - The confirmation check uses fragile substring tests.
- **`pipeline.rs`:** non-zero padding bits, mixed case, and middle chunks of the wrong length are all accepted, and `from_str_radix` accepts `+`.
- **`report.rs`:**
  - The JSON escaping doesn't handle control characters.
  - A transaction already in the mempool may be reported as theft.
- **`input.rs`:**
  - Trimming can drop trailing bytes of a binary PSBT.
  - The base64 decoder is lenient.

## What was read
Every non-test line of all 18 files was read by both a reader and a reviewer. The only exception: the reviewer of `main.rs` skipped lines 1-279, which hold the clap structs; the reader read the whole file.
