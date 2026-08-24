//! BCH primitives for the mt1 string layer: bech32 alphabet conversion and
//! syndrome-based error correction.
//!
//! **PORTED FROM `mk-codec`, not `md-codec`** — and the distinction is not
//! bookkeeping. This file's header claimed `md-codec` through the whole of P1
//! and cited a file path that does not exist, while `mod.rs` one directory
//! over, `PROVENANCE.md`, and the P1 commit message all correctly said
//! `mk-codec`. An independent claim check caught it.
//!
//! It matters because of the constellation's **three-way defect check**: a bug
//! found in a BCH primitive here must be checked against the repo this code
//! came from, and a reader who believes the wrong provenance checks the wrong
//! sibling, finds nothing, and closes the loop. The fork decision itself was
//! made because `mk-codec` does NOT depend on `md-codec`, so the two are
//! genuinely different upstreams.
//!
//! The BCH polynomials and field arithmetic are shared with both siblings —
//! all three reuse BIP-93's `BCH(93,80,8)` regular code and `BCH(108,93,8)`
//! long code. The only `mt1`-specific knobs are the HRP (`"mt"`) and the
//! NUMS-derived target residues ([`crate::consts::MT_REGULAR_CONST`] /
//! [`crate::consts::MT_LONG_CONST`]).
//!
//! This file exposes no top-level `encode_string` / `decode_string`: `mt1`'s
//! header lives at the 5-bit symbol layer as **one 11-symbol header on every
//! string** (`string_layer/header.rs`). An earlier version of this comment
//! described a `SingleString`/`Chunked` split with 2- and 8-symbol headers —
//! a shape `mt1` does not have and never shipped, since the operator's ruling
//! removed the `chunked` bit in favour of per-field symbol alignment.

use super::bch_decode;
use crate::consts::{HRP, MT_LONG_CONST, MT_REGULAR_CONST};

/// Which BCH code variant a string uses.
///
/// Determined by the total data-part length: regular for ≤93 chars,
/// long for 96–108 chars. Lengths 94–95 are reserved-invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BchCode {
    /// Regular code: BCH(93,80,8). 13-char checksum.
    Regular,
    /// Long code: BCH(108,93,8). 15-char checksum.
    Long,
}

/// The bech32 32-character alphabet, in 5-bit-value order.
///
/// `q=0, p=1, z=2, r=3, y=4, 9=5, x=6, 8=7, g=8, f=9, 2=10, t=11, v=12,
///  d=13, w=14, 0=15, s=16, 3=17, j=18, n=19, 5=20, 4=21, k=22, h=23,
///  c=24, e=25, 6=26, m=27, u=28, a=29, 7=30, l=31`.
pub const ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Inverse lookup: char (lowercase ASCII) -> 5-bit value, or 0xFF if not in alphabet.
const ALPHABET_INV: [u8; 128] = build_alphabet_inv();

const fn build_alphabet_inv() -> [u8; 128] {
    let mut inv = [0xFFu8; 128];
    let mut i = 0;
    while i < 32 {
        inv[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    inv
}

/// Convert a sequence of 8-bit bytes to a sequence of 5-bit values
/// (padded with zero bits at the end if the bit count is not a multiple of 5).
pub fn bytes_to_5bit(bytes: &[u8]) -> Vec<u8> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity((bytes.len() * 8).div_ceil(5));
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 0x1F) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 0x1F) as u8);
    }
    out
}

/// Convert a sequence of 5-bit values back to 8-bit bytes.
///
/// Returns `None` if any value in `values` is ≥ 32 (out of 5-bit range),
/// or if the trailing padding bits are non-zero.
pub fn five_bit_to_bytes(values: &[u8]) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(values.len() * 5 / 8);
    for &v in values {
        if v >= 32 {
            return None;
        }
        acc = (acc << 5) | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    // Any remaining bits must be zero (padding).
    if bits >= 5 {
        return None;
    }
    if (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}

/// The bech32 separator character between HRP and data-part (BIP 173 §3).
///
/// Re-exported by [`crate::consts::HRP`] is `"mt"`; this module's
/// BCH-checksum helpers consume the HRP through their `hrp` parameter so
/// that the same primitives can verify any single-HRP codex32-derived
/// string. Production callers MUST pass [`crate::consts::HRP`].
pub const SEPARATOR: char = '1';

/// Determine the BchCode variant from a total data-part length.
///
/// Boundaries are from BIP 93 (codex32): regular code `BCH(93,80,8)` caps at 93,
/// long code `BCH(108,93,8)` runs 96–108, and lengths 94–95 are explicitly
/// reserved-invalid to prevent ambiguity in code-variant selection. Lengths
/// below 14 or above 108 are also rejected.
pub fn bch_code_for_length(data_part_len: usize) -> Option<BchCode> {
    match data_part_len {
        14..=93 => Some(BchCode::Regular),
        94..=95 => None,
        96..=108 => Some(BchCode::Long),
        _ => None,
    }
}

/// Check whether a string is all-lowercase, all-uppercase, or mixed.
///
/// Only ASCII letters are considered; non-ASCII characters (digits, punctuation,
/// Unicode letters) are treated as neither case. This is appropriate for MD
/// strings, whose alphabet is a subset of ASCII. An empty string or one with
/// no ASCII letters returns [`CaseStatus::Lower`].
pub fn case_check(s: &str) -> CaseStatus {
    let mut has_lower = false;
    let mut has_upper = false;
    for c in s.chars() {
        if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_uppercase() {
            has_upper = true;
        }
        if has_lower && has_upper {
            break;
        }
    }
    match (has_lower, has_upper) {
        (true, true) => CaseStatus::Mixed,
        (true, false) => CaseStatus::Lower,
        (false, true) => CaseStatus::Upper,
        (false, false) => CaseStatus::Lower, // empty / no letters; treat as lower
    }
}

/// Result of a case check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStatus {
    /// All-lowercase or no letters.
    Lower,
    /// All-uppercase.
    Upper,
    /// Both lowercase and uppercase letters present (invalid).
    Mixed,
}

/// BCH polymod constants for the regular checksum (BCH(93,80,8)).
///
/// Source: BIP 93 (codex32) reference implementation, `ms32_polymod` function.
/// These five values are XORed into the running residue based on the top 5 bits
/// of the residue at each step. The polymod operation uses a 65-bit residue
/// (top 5 bits = current `b`, bottom 60 bits = masked state).
///
/// Verified against the canonical reference at
/// <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki>.
pub const GEN_REGULAR: [u128; 5] = [
    0x19dc500ce73fde210,
    0x1bfae00def77fe529,
    0x1fbd920fffe7bee52,
    0x1739640bdeee3fdad,
    0x07729a039cfc75f5a,
];

/// Constellation-internal initial residue that mt-codec's `ms32_polymod` and
/// `ms32_long_polymod` seed before processing any input — shared byte-for-byte
/// with mk1 and md1 alike — this constant is BIP-93's, not any sibling's.
///
/// This value (`0x23181b3`) **IS** codex32/BIP-93's published `ms32_polymod`
/// initial residue verbatim: the reference `ms32_polymod` seeds its accumulator
/// with exactly this constant, and [`GEN_REGULAR`] / [`GEN_LONG`] are BIP-93's
/// `ms32` / `ms32_long` generators term for term. (It is **not** bech32/BIP-173's
/// init `1`; that init belongs to a different code. An earlier note here
/// claiming this was "deliberately NOT codex32's init" and that "the BIP-93
/// reference `ms32_polymod` starts from `1`, not `0x23181b3`" was wrong.)
/// md1 and mt1 seed this init literally. `ms1` (`ms-codec`) uses the
/// mathematically **equivalent** formulation — codex32's literal `1` init with
/// an `hrp_expand("ms")` prepend — because `0x23181b3` is exactly the fold of
/// `hrp_expand("ms")` from `1`; a raw constant-diff against `ms-codec`'s
/// `POLYMOD_INIT = 0x1` is therefore NOT a discrepancy. Sharing this init with
/// md1 is harmless: each of mt1's regular + long codes is self-contained (the
/// same init seeds both checksum-create and verify), so the init's contribution
/// cancels and a valid codeword's residue equals its per-HRP target at every
/// length, for any fixed init. Domain separation is carried by the per-HRP
/// target constants (`MT_REGULAR_CONST` / `MT_LONG_CONST`) + the HRP — never by
/// this init. The reverted ms-codec
/// v0.2.1 bug was a non-codex32 init *paired with* an empirically-miscalibrated
/// target diverging from codex32 across lengths, not this value being
/// intrinsically length-variant; see
/// `mnemonic-secret/design/BUG_decode_with_correction_length_divergence.md`.
pub const POLYMOD_INIT: u128 = 0x23181b3;

/// Right-shift amount to extract the top 5 bits from a 65-bit regular-code residue.
///
/// Usage: `b = residue >> REGULAR_SHIFT` gives the 5-bit feedback selector
/// for the polymod algorithm.
pub const REGULAR_SHIFT: u32 = 60;

/// Mask preserving the low 60 bits of a 65-bit regular-code residue.
pub const REGULAR_MASK: u128 = 0x0fffffffffffffff;

/// BCH polymod constants for the long checksum (BCH(108,93,8)).
///
/// Source: BIP 93 (codex32) reference implementation, `ms32_long_polymod` function.
/// The long polymod uses a 75-bit residue (top 5 bits = `b`, bottom 70 bits = masked state).
///
/// Verified against the canonical reference at
/// <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki>.
pub const GEN_LONG: [u128; 5] = [
    0x3d59d273535ea62d897,
    0x7a9becb6361c6c51507,
    0x543f9b7e6c38d8a2a0e,
    0x0c577eaeccf1990d13c,
    0x1887f74f8dc71b10651,
];

/// Right-shift amount to extract the top 5 bits from a 75-bit long-code residue.
///
/// Usage: `b = residue >> LONG_SHIFT` gives the 5-bit feedback selector
/// for the polymod algorithm.
pub const LONG_SHIFT: u32 = 70;

/// Mask preserving the low 70 bits of a 75-bit long-code residue.
pub const LONG_MASK: u128 = 0x3fffffffffffffffff;

/// One step of the BCH polymod algorithm from BIP 93.
///
/// Updates the running `residue` to incorporate the next 5-bit input `value`
/// using the polynomial defined by `gen`, shift width `shift`, and mask `mask`.
/// The same function is used for both the regular and long codes; pass
/// `(GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK)` for the regular code and
/// `(GEN_LONG, LONG_SHIFT, LONG_MASK)` for the long code.
///
/// Returns the updated residue after incorporating `value`. The top 5 bits of
/// the returned residue feed the next iteration's `b` selector.
///
/// This is a direct port of BIP 93's `ms32_polymod` / `ms32_long_polymod` inner
/// loop. See <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki> .
fn polymod_step(residue: u128, value: u128, r#gen: &[u128; 5], shift: u32, mask: u128) -> u128 {
    let b = residue >> shift;
    let mut new_residue = ((residue & mask) << 5) ^ value;
    for (i, &g) in r#gen.iter().enumerate() {
        if (b >> i) & 1 != 0 {
            new_residue ^= g;
        }
    }
    new_residue
}

/// BIP 173-style HRP-expansion: produces the 5-bit-symbol prelude that gets
/// prepended to the data part before running the BCH polymod.
///
/// For each HRP character `c`, emits `c >> 5` (high 3 bits); then emits a
/// single 0 separator; then emits each character's `c & 31` (low 5 bits).
/// The result has length `2 * hrp.len() + 1` for ASCII HRPs.
///
/// For `hrp_expand("md")` this returns `[3, 3, 0, 13, 4]`.
pub fn hrp_expand(hrp: &str) -> Vec<u8> {
    let bytes = hrp.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 2 + 1);
    for &c in bytes {
        out.push(c >> 5);
    }
    out.push(0);
    for &c in bytes {
        out.push(c & 31);
    }
    out
}

/// Run polymod over a sequence of 5-bit values using the parameters for
/// either the regular or long BCH code, starting from POLYMOD_INIT.
///
/// v0.3.1: promoted from `pub(in crate::string_layer)` to `pub` so
/// downstream consumers (toolkit `repair` feature) can compute polymod
/// residues against ms / md / mk target constants (all 3 share the
/// BIP-93 BCH(93,80,8) generator). Test-helper-drift concern remains
/// resolved by the sibling `bch_decode` module using THIS function
/// directly rather than re-implementing.
pub fn polymod_run(values: &[u8], r#gen: &[u128; 5], shift: u32, mask: u128) -> u128 {
    let mut residue = POLYMOD_INIT;
    for &v in values {
        residue = polymod_step(residue, v as u128, r#gen, shift, mask);
    }
    residue
}

/// Compute the 13-character BCH checksum for the regular code over the
/// HRP-expanded preamble plus the data part.
///
/// `data` is the sequence of 5-bit values for the data part (header + payload),
/// not including the checksum. Returns the 13-element checksum array, ready
/// to append to `data` to form the full data-part-plus-checksum.
///
/// The algorithm runs polymod over `hrp_expand(hrp) || data || [0; 13]`,
/// then XORs the result with [`MT_REGULAR_CONST`] to extract the checksum.
pub fn bch_create_checksum_regular(hrp: &str, data: &[u8]) -> [u8; 13] {
    // Regular code: 13-symbol checksum (0..=12), pad/array/extraction all use 13.
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data);
    input.extend(std::iter::repeat_n(0, 13));
    let polymod = polymod_run(&input, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK) ^ MT_REGULAR_CONST;
    let mut out = [0u8; 13];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ((polymod >> (5 * (12 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a regular-code BCH checksum.
///
/// `data_with_checksum` is the full data part including the trailing 13
/// checksum characters. Returns `true` iff the polymod over
/// `hrp_expand(hrp) || data_with_checksum` equals [`MT_REGULAR_CONST`].
pub fn bch_verify_regular(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 13 {
        return false;
    }
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    polymod_run(&input, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK) == MT_REGULAR_CONST
}

/// Compute the 15-character BCH checksum for the long code.
///
/// Same algorithm as [`bch_create_checksum_regular`] but uses the long-code
/// polymod parameters (`GEN_LONG`, `LONG_SHIFT`, `LONG_MASK`) and target
/// constant ([`MT_LONG_CONST`]). Produces a 15-element checksum array.
pub fn bch_create_checksum_long(hrp: &str, data: &[u8]) -> [u8; 15] {
    // Long code: 15-symbol checksum (0..=14), pad/array/extraction all use 15.
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data);
    input.extend(std::iter::repeat_n(0, 15));
    let polymod = polymod_run(&input, &GEN_LONG, LONG_SHIFT, LONG_MASK) ^ MT_LONG_CONST;
    let mut out = [0u8; 15];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ((polymod >> (5 * (14 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a long-code BCH checksum.
///
/// Same algorithm as [`bch_verify_regular`] with long-code parameters.
/// Returns false if `data_with_checksum` is shorter than 15 symbols.
pub fn bch_verify_long(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 15 {
        return false;
    }
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    polymod_run(&input, &GEN_LONG, LONG_SHIFT, LONG_MASK) == MT_LONG_CONST
}

/// Result of a successful BCH decode + correct attempt.
///
/// Returned by [`bch_correct_regular`] / [`bch_correct_long`] when correction
/// succeeds. `corrections_applied == 0` means the input was already valid;
/// `> 0` means substitutions were applied at the indicated positions.
///
/// Marked `#[non_exhaustive]` to allow future fields (e.g., confidence
/// score, syndrome metadata) without breaking downstream struct-literal
/// construction. Construct via the [`bch_correct_regular`] /
/// [`bch_correct_long`] APIs.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionResult {
    /// The corrected `data_with_checksum` slice (input may have been modified).
    pub data: Vec<u8>,
    /// Number of substitutions applied (0 = clean input).
    pub corrections_applied: usize,
    /// Indices into `data` of the substituted positions.
    pub corrected_positions: Vec<usize>,
}

/// Attempt to correct a regular-code BCH-checksummed string with up to four
/// substitutions, the full t = 4 capacity of the BCH(93, 80, 8) code.
///
/// Implements the standard syndrome-based BCH decoder pipeline: syndrome
/// computation in `GF(1024) = GF(32²)`, Berlekamp–Massey for the
/// error-locator polynomial, Chien search for error positions, Forney's
/// algorithm for error magnitudes. After applying the proposed corrections,
/// the result is re-verified via [`bch_verify_regular`]; the decoder rejects
/// any output that does not produce a valid codeword (defensive guard
/// against pathological 5+-error inputs whose syndromes happen to factor as
/// a degree-≤ 4 locator).
///
/// Returns `Ok(CorrectionResult)` if the input is clean or up to four
/// substitutions repair it. Returns `Err(Error::BchUncorrectable)` otherwise.
///
/// # Algorithm details
///
/// See the private `bch_decode` submodule for the algorithm and the
/// `GF(1024)` field representation.
pub fn bch_correct_regular(
    hrp: &str,
    data_with_checksum: &[u8],
) -> Result<CorrectionResult, crate::Error> {
    if bch_verify_regular(hrp, data_with_checksum) {
        return Ok(CorrectionResult {
            data: data_with_checksum.to_vec(),
            corrections_applied: 0,
            corrected_positions: vec![],
        });
    }
    // Compute polymod over hrp_expand(hrp) || data_with_checksum, XOR with
    // the MD target constant. The result is congruent to the error
    // polynomial E(x) modulo g_regular(x).
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    let residue = polymod_run(&input, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK) ^ MT_REGULAR_CONST;

    if let Some((positions, magnitudes)) =
        bch_decode::decode_regular_errors(residue, data_with_checksum.len())
    {
        if positions.is_empty() {
            // Should be unreachable (caller already verified); guard anyway.
            return Ok(CorrectionResult {
                data: data_with_checksum.to_vec(),
                corrections_applied: 0,
                corrected_positions: vec![],
            });
        }
        let mut corrected = data_with_checksum.to_vec();
        for (&p, &m) in positions.iter().zip(&magnitudes) {
            if p >= corrected.len() {
                return Err(crate::Error::BchUncorrectable(format!(
                    "decoder reported error position {p} outside data ({} symbols)",
                    corrected.len()
                )));
            }
            corrected[p] ^= m;
        }
        // Defensive: re-verify. Catches the 5+-error edge case.
        if bch_verify_regular(hrp, &corrected) {
            return Ok(CorrectionResult {
                corrections_applied: positions.len(),
                corrected_positions: positions,
                data: corrected,
            });
        }
    }
    Err(crate::Error::BchUncorrectable(
        "regular code: more than 4 substitutions or pathological pattern".into(),
    ))
}

/// Long-code analog of [`bch_correct_regular`].
///
/// Implements the same BM/Chien/Forney pipeline against the long-code
/// generator polynomial, reaching the full t = 4 capacity of
/// `BCH(108, 93, 8)`.
pub fn bch_correct_long(
    hrp: &str,
    data_with_checksum: &[u8],
) -> Result<CorrectionResult, crate::Error> {
    if bch_verify_long(hrp, data_with_checksum) {
        return Ok(CorrectionResult {
            data: data_with_checksum.to_vec(),
            corrections_applied: 0,
            corrected_positions: vec![],
        });
    }
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    let residue = polymod_run(&input, &GEN_LONG, LONG_SHIFT, LONG_MASK) ^ MT_LONG_CONST;

    if let Some((positions, magnitudes)) =
        bch_decode::decode_long_errors(residue, data_with_checksum.len())
    {
        if positions.is_empty() {
            return Ok(CorrectionResult {
                data: data_with_checksum.to_vec(),
                corrections_applied: 0,
                corrected_positions: vec![],
            });
        }
        let mut corrected = data_with_checksum.to_vec();
        for (&p, &m) in positions.iter().zip(&magnitudes) {
            if p >= corrected.len() {
                return Err(crate::Error::BchUncorrectable(format!(
                    "decoder reported error position {p} outside data ({} symbols)",
                    corrected.len()
                )));
            }
            corrected[p] ^= m;
        }
        if bch_verify_long(hrp, &corrected) {
            return Ok(CorrectionResult {
                corrections_applied: positions.len(),
                corrected_positions: positions,
                data: corrected,
            });
        }
    }
    Err(crate::Error::BchUncorrectable(
        "long code: more than 4 substitutions or pathological pattern".into(),
    ))
}

/// Encode a 5-bit-symbol data stream as a complete mt1 string.
///
/// The data stream is the concatenation `header_symbols || bytes_to_5bit(payload_bytes)`
/// where `header_symbols` is the 2-symbol single-string header or the
/// 8-symbol chunked header (closure Q-5). The BCH code variant (regular or
/// long) is auto-selected from the resulting data-part length per BIP 93:
/// regular for ≤93-symbol data parts, long for 96–108-symbol data parts.
/// Lengths in the reserved-invalid 94–95 gap or outside the BIP 93 valid
/// range return [`Error::InvalidStringLength`].
///
/// Per the v0.1 emit policy described in `design/IMPLEMENTATION_PLAN_mk_v0_1.md`
/// §5.4, callers control fragment sizing so that each chunked fragment lands
/// within long-code territory. Single-string mt1 may pick regular or long
/// based on bytecode size.
///
/// Returns the full string starting with [`crate::consts::HRP`] and the
/// BIP 173 separator (`"mt1"`).
pub fn encode_5bit_to_string(data_5bit: &[u8]) -> Result<String, crate::Error> {
    use crate::Error;

    // Auto-determine code from the eventual data-part length (data_5bit + checksum).
    let regular_total = data_5bit.len() + 13;
    let long_total = data_5bit.len() + 15;
    let code = match (
        bch_code_for_length(regular_total),
        bch_code_for_length(long_total),
    ) {
        (Some(BchCode::Regular), _) => BchCode::Regular,
        (_, Some(BchCode::Long)) => BchCode::Long,
        // Neither code variant accepts this data-part length: too short, in
        // the 94–95 reserved-invalid gap, or too long for v0.1.
        _ => {
            // Pick the closest length to report — long_total is always larger,
            // so report that as the "actual length you tried to produce".
            return Err(Error::InvalidStringLength(long_total));
        }
    };

    let checksum: Vec<u8> = match code {
        BchCode::Regular => bch_create_checksum_regular(HRP, data_5bit).to_vec(),
        BchCode::Long => bch_create_checksum_long(HRP, data_5bit).to_vec(),
    };

    let mut full = String::with_capacity(HRP.len() + 1 + data_5bit.len() + checksum.len());
    full.push_str(HRP);
    full.push(SEPARATOR);
    for &v in data_5bit {
        full.push(ALPHABET[v as usize] as char);
    }
    for v in checksum {
        full.push(ALPHABET[v as usize] as char);
    }
    Ok(full)
}

/// Result of a successful mt1 string decode at the BCH layer.
///
/// Use [`Self::data`] to access the data part as 5-bit values (header
/// symbols + payload, checksum stripped); the string-layer reassembler
/// in `crate::string_layer` splits header symbols off and feeds the
/// remaining payload through [`five_bit_to_bytes`] to recover the original
/// fragment bytes.
///
/// The full post-correction 5-bit symbol sequence (data **plus** the trailing
/// 13- or 15-char checksum) is retained internally as [`Self::data_with_checksum`]
/// and can be queried by [`Self::corrected_char_at`] for any position in
/// the data part — including positions that fall inside the checksum region.
/// The decoder-report layer uses this to surface the real corrected
/// character when BCH ECC repairs a substitution inside the checksum
/// (parallels `mk-codec`'s `Correction.corrected` field — mk is this file's
/// upstream; see the module header).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedString {
    /// Detected BCH code variant.
    pub code: BchCode,
    /// Number of substitution errors corrected (0 = clean input, 1 = recovered).
    pub corrections_applied: usize,
    /// Indices into the data-part (chars after `"md1"`) of any corrected positions.
    pub corrected_positions: Vec<usize>,
    /// Full post-correction 5-bit symbol sequence (data part + checksum), in
    /// the same coordinate system as [`Self::corrected_positions`].
    ///
    /// Length is `data().len() + 13` (regular code) or `data().len() + 15`
    /// (long code). Indices `0..data().len()` mirror [`Self::data`] symbol-for-symbol;
    /// indices `data().len()..` are the corrected checksum symbols. Use
    /// [`Self::corrected_char_at`] for the human-readable bech32 character at
    /// any position.
    pub data_with_checksum: Vec<u8>,
}

impl DecodedString {
    /// Data part as 5-bit values, with the trailing checksum stripped.
    ///
    /// Returns a slice into [`Self::data_with_checksum`] — the data part is
    /// `data_with_checksum[..len - checksum_len]`, where `checksum_len` is 13
    /// for [`BchCode::Regular`] and 15 for [`BchCode::Long`].
    pub fn data(&self) -> &[u8] {
        let checksum_len = match self.code {
            BchCode::Regular => 13,
            BchCode::Long => 15,
        };
        &self.data_with_checksum[..self.data_with_checksum.len() - checksum_len]
    }

    /// Look up the corrected bech32 character at the given position in the
    /// data part (chars after the `"md1"` HRP+separator).
    ///
    /// `char_position` is 0-indexed. Positions `0..data().len()` are in the
    /// data region; positions `data().len()..data().len() + checksum_len` are
    /// inside the BCH checksum (13 chars for [`BchCode::Regular`], 15 for
    /// [`BchCode::Long`]). All positions return the post-correction
    /// character — i.e., what the symbol *should* be after BCH repair, which
    /// is exactly what [`Correction.corrected`][crate::Correction::corrected]
    /// is documented to report.
    ///
    /// # Panics
    ///
    /// Panics if `char_position >= data_with_checksum.len()`. Callers are
    /// responsible for clamping the position to a valid range; in the decode
    /// pipeline this is guaranteed by the BCH layer (it never reports a
    /// `corrected_position` outside `data_with_checksum`). Note that
    /// `data_with_checksum` includes the checksum region; "outside the data
    /// part" elsewhere in this crate excludes the checksum and is a tighter
    /// bound than what this method requires.
    pub fn corrected_char_at(&self, char_position: usize) -> char {
        let v = self.data_with_checksum[char_position];
        ALPHABET[v as usize] as char
    }
}

/// Decode an mt1 string, validating HRP, case, length, and checksum.
///
/// Performs full BCH error correction up to four substitutions
/// (`t = 4` capacity of the BCH(93, 80, 8) regular code and the
/// BCH(108, 93, 8) long code), via syndrome-based Berlekamp–Massey +
/// Forney decoding (implemented in the sibling `bch_decode` module).
///
/// Errors:
/// - [`Error::MixedCase`] if the string mixes upper and lower case.
/// - [`Error::InvalidHrp`] if the HRP is missing or not [`crate::consts::HRP`].
/// - [`Error::InvalidStringLength`] if the data-part length isn't a valid mt1 length.
/// - [`Error::InvalidChar`] if the data part contains a non-bech32 character.
/// - [`Error::BchUncorrectable`] if the checksum can't be repaired within
///   the BCH `t = 4` correction radius.
///
/// [`Error::MixedCase`]: crate::Error::MixedCase
/// [`Error::InvalidHrp`]: crate::Error::InvalidHrp
/// [`Error::InvalidStringLength`]: crate::Error::InvalidStringLength
/// [`Error::InvalidChar`]: crate::Error::InvalidChar
/// [`Error::BchUncorrectable`]: crate::Error::BchUncorrectable
pub fn decode_string(s: &str) -> Result<DecodedString, crate::Error> {
    use crate::Error;

    if matches!(case_check(s), CaseStatus::Mixed) {
        return Err(Error::MixedCase);
    }
    let s_lower = s.to_lowercase();

    let sep_pos = s_lower
        .rfind(SEPARATOR)
        .ok_or_else(|| Error::InvalidHrp(s_lower.clone()))?;
    let (hrp, rest) = s_lower.split_at(sep_pos);
    let data_part = &rest[1..]; // skip the '1' separator

    if hrp != HRP {
        return Err(Error::InvalidHrp(hrp.to_string()));
    }

    let code =
        bch_code_for_length(data_part.len()).ok_or(Error::InvalidStringLength(data_part.len()))?;

    let mut values: Vec<u8> = Vec::with_capacity(data_part.len());
    for (i, c) in data_part.chars().enumerate() {
        if !c.is_ascii() {
            return Err(Error::InvalidChar { ch: c, position: i });
        }
        let v = ALPHABET_INV[c as usize];
        if v == 0xFF {
            return Err(Error::InvalidChar { ch: c, position: i });
        }
        values.push(v);
    }

    let correction = match code {
        BchCode::Regular => bch_correct_regular(hrp, &values),
        BchCode::Long => bch_correct_long(hrp, &values),
    };
    let result = correction?;

    Ok(DecodedString {
        code,
        corrections_applied: result.corrections_applied,
        corrected_positions: result.corrected_positions,
        data_with_checksum: result.data,
    })
}
