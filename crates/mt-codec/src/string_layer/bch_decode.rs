//! Syndrome-based BCH decoder for the MK regular and long codes.
//!
//! **PORTED FROM `mk-codec`, not `md-codec`.** See `bch.rs` for why the
//! distinction is load-bearing: the constellation's three-way defect check
//! sends a reader to the upstream this code came from, and the wrong name
//! sends them to the wrong repository.

#[cfg(test)]
use super::bch::{GEN_LONG, GEN_REGULAR};

// ---------------------------------------------------------------------------
// GF(32) — same field as `crate::string_layer::bch::ALPHABET` codes
// ---------------------------------------------------------------------------

/// One element of `GF(32) = GF(2)[α] / (α⁵ + α³ + 1)`, encoded as a
/// 5-bit integer `0..32` whose binary digits are the polynomial
/// coefficients (low bit = constant term).
type Gf32 = u8;

/// Primitive polynomial reduction mask for `GF(32)`: when a `GF(32)`
/// multiplication overflows into bit 5, XOR with `0b00_1001 = 9` to fold
/// `α⁵ ≡ α³ + 1` back into the residue.
const GF32_REDUCE: u8 = 0b0_1001;

/// Multiply two `GF(32)` elements (carryless multiply with reduction).
const fn gf32_mul(a: Gf32, b: Gf32) -> Gf32 {
    let mut result: u8 = 0;
    let mut a = a;
    let mut i = 0;
    while i < 5 {
        if (b >> i) & 1 != 0 {
            result ^= a;
        }
        // Multiply a by α; reduce if it leaves the 5-bit window.
        let carry = (a >> 4) & 1;
        a = (a << 1) & 0x1F;
        if carry != 0 {
            a ^= GF32_REDUCE;
        }
        i += 1;
    }
    result
}

// ---------------------------------------------------------------------------
// GF(1024) — built as GF(32²) via ζ² = ζ + 1
// ---------------------------------------------------------------------------

/// One element of `GF(1024)` as a pair `(lo, hi)` of `GF(32)` elements
/// representing `lo + hi·ζ` where `ζ² = ζ + 1` (i.e., `ζ` is a
/// primitive cube root of unity in `GF(1024)*`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Gf1024 {
    lo: Gf32,
    hi: Gf32,
}

impl Gf1024 {
    const ZERO: Gf1024 = Gf1024 { lo: 0, hi: 0 };
    const ONE: Gf1024 = Gf1024 { lo: 1, hi: 0 };

    /// Embed a `GF(32)` element as the constant term.
    const fn from_gf32(v: Gf32) -> Self {
        Gf1024 { lo: v, hi: 0 }
    }

    fn add(self, other: Self) -> Self {
        Gf1024 {
            lo: self.lo ^ other.lo,
            hi: self.hi ^ other.hi,
        }
    }

    fn is_zero(self) -> bool {
        self.lo == 0 && self.hi == 0
    }

    /// Multiply two `GF(1024)` elements using the field relation
    /// `ζ² = ζ + 1`. Concretely:
    ///
    /// ```text
    /// (lo + hi·ζ) · (lo' + hi'·ζ)
    ///   = lo·lo' + (lo·hi' + hi·lo')·ζ + hi·hi'·ζ²
    ///   = lo·lo' + (lo·hi' + hi·lo')·ζ + hi·hi'·(ζ + 1)
    ///   = (lo·lo' + hi·hi') + (lo·hi' + hi·lo' + hi·hi')·ζ
    /// ```
    fn mul(self, other: Self) -> Self {
        let ll = gf32_mul(self.lo, other.lo);
        let lh = gf32_mul(self.lo, other.hi);
        let hl = gf32_mul(self.hi, other.lo);
        let hh = gf32_mul(self.hi, other.hi);
        Gf1024 {
            lo: ll ^ hh,
            hi: lh ^ hl ^ hh,
        }
    }

    fn pow(self, mut exp: u32) -> Self {
        let mut base = self;
        let mut result = Gf1024::ONE;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul(base);
            }
            base = base.mul(base);
            exp >>= 1;
        }
        result
    }

    fn inv(self) -> Self {
        // Fermat: a^(2^10 - 2) = a^1022 = a^-1 in GF(1024)*.
        debug_assert!(!self.is_zero(), "inv of zero in GF(1024)");
        self.pow(1022)
    }
}

/// `ζ`, a primitive cube root of unity. With our `Y² = Y + 1` quadratic,
/// `ζ` is `Y` itself, encoded as `(0, 1)` in `(lo, hi)` form. Used in
/// tests to verify the field relation `ζ² = ζ + 1`; the runtime
/// arithmetic uses [`BETA`] and [`GAMMA`] directly.
#[cfg(test)]
/// Retained deliberately: `ZETA` and the two `generator_polynomial_*` functions
/// below are the DERIVATION of the packed `GEN_*` constants in `bch.rs`. Nothing
/// calls them now that `mk`'s constant-specific tests are gone, but they are the
/// only record of where those u128s come from — and a future reader cannot
/// reconstruct that from the packed values.
#[allow(dead_code)]
const ZETA: Gf1024 = Gf1024 { lo: 0, hi: 1 };

/// `β = G·ζ = 8·ζ`, the primitive element for the **regular code**'s
/// BCH-defining group. `β` has order 93. (BIP 93 §"Generation of valid
/// checksum".)
const BETA: Gf1024 = Gf1024 { lo: 0, hi: 8 };

/// `γ = E + X·ζ = 25 + 6·ζ`, the primitive element for the **long
/// code**'s BCH-defining group. `γ` has order 1023.
const GAMMA: Gf1024 = Gf1024 { lo: 25, hi: 6 };

/// Smallest exponent in the 8-consecutive-roots window of the regular
/// code's generator polynomial: `g_regular(β^j) = 0` for `j = 77, …, 84`.
const REGULAR_J_START: u32 = 77;

/// Smallest exponent in the 8-consecutive-roots window of the long
/// code's generator polynomial: `g_long(γ^j) = 0` for
/// `j = 1019, 1020, …, 1026`.
const LONG_J_START: u32 = 1019;

// `β` and `γ` orders (93 and 1023, respectively) are tested directly
// in the unit-test module via inline integer constants; we don't need
// run-time symbols for them.

// ---------------------------------------------------------------------------
// Generator polynomial reconstruction (used only for self-test)
// ---------------------------------------------------------------------------

/// Reconstruct `g_regular(x)`, the degree-13 BCH generator polynomial,
/// from `GEN_REGULAR[0]`. Returns coefficients with `result[i]` being
/// the coefficient of `x^i`. The leading coefficient (`result[13]`) is 1.
///
/// **Why this works**: `polymod_step` computes
/// `(residue · x + symbol) mod g(x)` in `GF(32)[x] / g(x)`. The constant
/// `GEN_REGULAR[0]` is, by construction,
/// `1 · x^13 mod g(x) = x^13 mod g(x)`, packed as 13 5-bit GF(32)
/// coefficients (high coeff = `x^12`, low coeff = `x^0`). Since
/// `g(x) = x^13 + (x^13 - g(x))`, and reduction in characteristic 2
/// gives `g_low = x^13 mod g(x)`, we have
/// `g(x) = x^13 + GEN_REGULAR[0]_packed_as_polynomial`.
#[cfg(test)]
#[allow(dead_code)]
fn generator_polynomial_regular() -> [Gf32; 14] {
    let mut g = [0u8; 14];
    g[13] = 1;
    for (i, slot) in g.iter_mut().enumerate().take(13) {
        *slot = ((GEN_REGULAR[0] >> (5 * i)) & 0x1F) as u8;
    }
    g
}

/// Reconstruct `g_long(x)`, the degree-15 BCH generator polynomial,
/// from `GEN_LONG[0]`. Same packing convention as
/// [`generator_polynomial_regular`].
#[cfg(test)]
#[allow(dead_code)]
fn generator_polynomial_long() -> [Gf32; 16] {
    let mut g = [0u8; 16];
    g[15] = 1;
    for (i, slot) in g.iter_mut().enumerate().take(15) {
        *slot = ((GEN_LONG[0] >> (5 * i)) & 0x1F) as u8;
    }
    g
}

/// Horner-form polynomial evaluation: GF(32)-coefficient polynomial at
/// a GF(1024) point. `coeffs[i]` is the coefficient of `x^i`.
fn horner(coeffs: &[Gf32], x: Gf1024) -> Gf1024 {
    let mut acc = Gf1024::ZERO;
    for &c in coeffs.iter().rev() {
        acc = acc.mul(x).add(Gf1024::from_gf32(c));
    }
    acc
}

/// Horner-form polynomial evaluation: GF(1024)-coefficient polynomial
/// at a GF(1024) point. `coeffs[i]` is the coefficient of `x^i`.
fn horner_ext(coeffs: &[Gf1024], x: Gf1024) -> Gf1024 {
    let mut acc = Gf1024::ZERO;
    for &c in coeffs.iter().rev() {
        acc = acc.mul(x).add(c);
    }
    acc
}

// ---------------------------------------------------------------------------
// Syndromes
// ---------------------------------------------------------------------------

/// Compute the eight syndromes
/// `S_m = E(α^{j_start + m - 1})` for `m = 1, …, 8`, where `E(x)` is the
/// error polynomial (recoverable as the polymod residue minus the MD
/// target constant). The remainder is already congruent to `E(x)`
/// modulo `g(x)`, so evaluating it at the generator's roots is
/// equivalent to evaluating `E(x)` itself.
fn compute_syndromes(
    residue_xor_const: u128,
    checksum_len: u32,
    alpha: Gf1024,
    j_start: u32,
) -> [Gf1024; 8] {
    // Unpack the remainder: `checksum_len` GF(32) coefficients packed
    // with the highest-order coefficient (x^{checksum_len-1}) at bit
    // 5*(checksum_len-1) and the constant term (x^0) at bits 0..5.
    // Stack-allocate at the maximum (Long code = 15); the active slice
    // is `&coeffs[..checksum_len]`.
    let mut coeffs = [0u8; 15];
    let len = checksum_len as usize;
    for i in 0..checksum_len {
        coeffs[i as usize] = ((residue_xor_const >> (5 * i)) & 0x1F) as u8;
    }
    let coeffs = &coeffs[..len];

    let mut syndromes = [Gf1024::ZERO; 8];
    let alpha_j_start = alpha.pow(j_start);
    let mut alpha_j = alpha_j_start;
    for s in &mut syndromes {
        *s = horner(coeffs, alpha_j);
        alpha_j = alpha_j.mul(alpha);
    }
    syndromes
}

// ---------------------------------------------------------------------------
// Berlekamp–Massey
// ---------------------------------------------------------------------------

/// Berlekamp–Massey for BCH over `GF(1024)`. Returns the error-locator
/// polynomial `Λ(x)` with `Λ(0) = 1`. `Λ` has degree equal to the
/// number of errors when the received word is correctable.
fn berlekamp_massey(syndromes: &[Gf1024; 8]) -> Vec<Gf1024> {
    // Standard formulation (Massey 1969 / Lin & Costello §6.3, adapted
    // for 0-indexed syndromes where syndromes[k] = S_{j_start + k}).
    let n = syndromes.len();
    let mut lam: Vec<Gf1024> = vec![Gf1024::ONE]; // current connection poly
    let mut prev: Vec<Gf1024> = vec![Gf1024::ONE]; // last-updated connection poly
    let mut l: usize = 0; // current LFSR length
    let mut m: usize = 1; // shift since last update
    let mut b = Gf1024::ONE; // discrepancy from last update

    for k in 0..n {
        // Discrepancy: d = syndromes[k] + sum_{i=1..L} lam[i] * syndromes[k-i]
        let mut d = syndromes[k];
        for i in 1..=l {
            // i > k means k - i would underflow; skip rather than wrap.
            // i >= lam.len() means lam[i] doesn't exist yet; same skip.
            if i <= k && i < lam.len() {
                d = d.add(lam[i].mul(syndromes[k - i]));
            }
        }

        if d.is_zero() {
            m += 1;
        } else if 2 * l <= k {
            // Length increases. New lam = lam - (d/b) * x^m * prev.
            let t = lam.clone();
            let scale = d.mul(b.inv());
            let new_len = (lam.len()).max(prev.len() + m);
            lam.resize(new_len, Gf1024::ZERO);
            for (i, &p) in prev.iter().enumerate() {
                let idx = i + m;
                lam[idx] = lam[idx].add(scale.mul(p));
            }
            l = k + 1 - l;
            prev = t;
            b = d;
            m = 1;
        } else {
            // Length stays the same. lam = lam - (d/b) * x^m * prev.
            let scale = d.mul(b.inv());
            let new_len = (lam.len()).max(prev.len() + m);
            lam.resize(new_len, Gf1024::ZERO);
            for (i, &p) in prev.iter().enumerate() {
                let idx = i + m;
                lam[idx] = lam[idx].add(scale.mul(p));
            }
            m += 1;
        }
    }

    while lam.len() > 1 && lam.last().is_some_and(|x| x.is_zero()) {
        lam.pop();
    }
    lam
}

// ---------------------------------------------------------------------------
// Chien search + Forney
// ---------------------------------------------------------------------------

/// Search for the roots of `Λ(x)` among `α⁰, α⁻¹, …, α⁻⁽ᴸ⁻¹⁾`, where
/// `L = data_with_checksum_len` (we restrict the search to legitimate
/// error positions; HRP-prefix positions are not transmitted).
///
/// Returns the list of polynomial degrees `d ∈ [0, L)` such that
/// `Λ(α⁻ᵈ) = 0`. Each such `d` is the polynomial degree of an error.
/// Returns `None` if the number of distinct roots found does not equal
/// `deg(Λ)`.
fn chien_search(
    lambda: &[Gf1024],
    data_with_checksum_len: usize,
    alpha: Gf1024,
) -> Option<Vec<usize>> {
    let deg = lambda.len() - 1;
    if deg == 0 {
        return Some(Vec::new());
    }

    let mut error_degrees = Vec::with_capacity(deg);
    let alpha_inv = alpha.inv();
    let mut current = Gf1024::ONE; // α^0
    for d in 0..data_with_checksum_len {
        if horner_ext(lambda, current).is_zero() {
            error_degrees.push(d);
        }
        current = current.mul(alpha_inv);
    }

    if error_degrees.len() != deg {
        return None;
    }
    Some(error_degrees)
}

/// Shifted Forney's algorithm: given `Λ(x)`, the syndromes (at
/// `α^{j_start}, …, α^{j_start + 7}`), and the error degrees `d_k` such
/// that `α^{-d_k}` are the roots of `Λ`, compute the GF(32) error
/// magnitudes at each position.
///
/// Formula (with `j_start` shift):
///
/// ```text
/// e_k = X_k^{1 - j_start} · Ω(X_k^{-1}) / Λ'(X_k^{-1})
/// ```
///
/// where `X_k = α^{d_k}`, `Ω(x) ≡ S(x)·Λ(x) mod x^8`, and `Λ'(x)` is
/// the formal derivative.
///
/// Returns `None` if any computed magnitude does not lie in the symbol
/// field `GF(32)`.
fn forney(
    syndromes: &[Gf1024; 8],
    lambda: &[Gf1024],
    error_degrees: &[usize],
    alpha: Gf1024,
    j_start: u32,
) -> Option<Vec<Gf32>> {
    // Ω(x) = S(x) * Λ(x) mod x^8, where S(x) = sum_{m=0..7} S_{j_start + m} * x^m.
    let s_poly: Vec<Gf1024> = syndromes.to_vec();
    let mut omega = vec![Gf1024::ZERO; 8];
    for i in 0..s_poly.len().min(8) {
        for j in 0..lambda.len() {
            if i + j < 8 {
                omega[i + j] = omega[i + j].add(s_poly[i].mul(lambda[j]));
            }
        }
    }

    // Λ'(x) = formal derivative. In characteristic 2 only odd-power
    // terms survive: Λ'(x) = sum_{i odd} lambda[i] * x^{i-1}.
    let mut lambda_prime = vec![Gf1024::ZERO; lambda.len().saturating_sub(1)];
    for i in 1..lambda.len() {
        if i % 2 == 1 {
            lambda_prime[i - 1] = lambda[i];
        }
    }

    let mut magnitudes = Vec::with_capacity(error_degrees.len());
    for &d in error_degrees {
        // X_k = α^d.
        let x_k = alpha.pow(d as u32);
        let x_k_inv = x_k.inv();
        let omega_val = horner_ext(&omega, x_k_inv);
        let lam_p_val = horner_ext(&lambda_prime, x_k_inv);
        if lam_p_val.is_zero() {
            return None;
        }

        // Compute X_k^{1 - j_start}. Note `1 - j_start` is negative;
        // since X_k has order ord(α) (93 or 1023), we use
        // X_k^{1 - j_start} = X_k^{(ord - j_start + 1) mod ord}.
        // But we handle this generically via x_k_inv^{j_start - 1}.
        let shift = j_start.saturating_sub(1);
        let x_k_shift = x_k_inv.pow(shift); // = X_k^{-(j_start - 1)} = X_k^{1 - j_start}

        let mag = x_k_shift.mul(omega_val.mul(lam_p_val.inv()));

        // Magnitude must lie in GF(32) (the high coefficient must be zero).
        if mag.hi != 0 {
            return None;
        }
        if mag.lo == 0 {
            // Zero magnitude is not a real error — typically signals
            // more than 4 actual errors that fooled BM.
            return None;
        }
        magnitudes.push(mag.lo);
    }
    Some(magnitudes)
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Decode a regular-code BCH error pattern. Inputs:
///
/// - `residue_xor_const`: the value
///   `polymod(hrp_expand(hrp) || data_with_checksum) ⊕ MT_REGULAR_CONST`.
///   By the BCH syndrome property, this is congruent to the error
///   polynomial `E(x)` modulo `g_regular(x)`.
/// - `data_with_checksum_len`: the total symbol count of
///   `data_with_checksum` (in the `0..=93` range for the regular code).
///
/// Returns `Some((positions, magnitudes))` if the algorithm finds a
/// consistent error pattern of weight `≤ 4`. Each `positions[k]` is an
/// index into `data_with_checksum` (post-HRP-prefix); each
/// `magnitudes[k]` is a `GF(32)` symbol that must be XORed into
/// `data_with_checksum[positions[k]]` to repair the codeword. Returns
/// `None` if the pattern is uncorrectable.
// v0.3.1: promoted from `pub(super)` so downstream consumers (toolkit
// `repair` feature) can compute corrections for ms / md HRPs using their
// own target-residue constants (all 3 share the BIP-93 BCH(93,80,8) code).
pub fn decode_regular_errors(
    residue_xor_const: u128,
    data_with_checksum_len: usize,
) -> Option<(Vec<usize>, Vec<Gf32>)> {
    decode_errors(
        residue_xor_const,
        data_with_checksum_len,
        13,
        BETA,
        REGULAR_J_START,
    )
}

/// Long-code analog of [`decode_regular_errors`].
///
/// v0.3.1: promoted from `pub(super)` for downstream-consumer access (see
/// `decode_regular_errors` docs).
pub fn decode_long_errors(
    residue_xor_const: u128,
    data_with_checksum_len: usize,
) -> Option<(Vec<usize>, Vec<Gf32>)> {
    decode_errors(
        residue_xor_const,
        data_with_checksum_len,
        15,
        GAMMA,
        LONG_J_START,
    )
}

fn decode_errors(
    residue_xor_const: u128,
    data_with_checksum_len: usize,
    checksum_len: u32,
    alpha: Gf1024,
    j_start: u32,
) -> Option<(Vec<usize>, Vec<Gf32>)> {
    let syndromes = compute_syndromes(residue_xor_const, checksum_len, alpha, j_start);

    // All-zero syndromes ⇒ no errors (caller usually detects earlier).
    if syndromes.iter().all(|s| s.is_zero()) {
        return Some((Vec::new(), Vec::new()));
    }

    let lambda = berlekamp_massey(&syndromes);
    let deg = lambda.len() - 1;
    if deg == 0 || deg > 4 {
        // > 4 errors is above the BCH(•, •, 8) / t = 4 capacity.
        return None;
    }

    let error_degrees = chien_search(&lambda, data_with_checksum_len, alpha)?;
    if error_degrees.len() != deg {
        return None;
    }

    let magnitudes = forney(&syndromes, &lambda, &error_degrees, alpha, j_start)?;

    // Translate polynomial degrees back to data_with_checksum indices.
    // For data_with_checksum[k] (k = 0..L-1), polynomial degree d = L - 1 - k.
    // So k = L - 1 - d.
    let mut positions = Vec::with_capacity(error_degrees.len());
    for &d in &error_degrees {
        if d >= data_with_checksum_len {
            // Should not happen since chien_search bounds d to [0, L).
            return None;
        }
        let k = data_with_checksum_len - 1 - d;
        positions.push(k);
    }

    // Sort ascending by position for deterministic output. Magnitudes
    // need to be reordered along with the positions.
    let mut paired: Vec<(usize, Gf32)> = positions.into_iter().zip(magnitudes).collect();
    paired.sort_by_key(|p| p.0);
    let positions: Vec<usize> = paired.iter().map(|p| p.0).collect();
    let magnitudes: Vec<Gf32> = paired.iter().map(|p| p.1).collect();

    Some((positions, magnitudes))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
