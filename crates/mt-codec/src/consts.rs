//! `mt1`'s constants, and the derivations that keep them honest.
//!
//! Every value here is either **derived** or **pinned with a test that derives
//! it**. The spec's named hazard (§12.22) is a constant copied from a sibling
//! codec, and a copied constant is not detectably wrong: it yields chunks that
//! are *self-consistent* and unreadable by every other implementation,
//! surfacing at recovery where it looks exactly like steel damage.

/// The human-readable part. **`"mt"`, NOT `"mt1"`** — the `1` in a rendered
/// `mt1…` string is bech32's separator, not part of the HRP (spec §10.13 b).
///
/// This is one of the values §10.13(a2) flags as a guess-hazard, and it feeds
/// `hrp_expand` on both the create and verify sides — so getting it wrong
/// produces a format that verifies only against itself.
pub const HRP: &str = "mt";

/// `mt1`'s wire-format generation. Five bits wide, so 32 generations exist;
/// this is generation 1.
pub const VERSION: u8 = 1;

/// NUMS-derived target residue for the regular BCH code: the top **65 bits** of
/// `SHA-256(`[`NUMS_DOMAIN`]`)`.
///
/// Verified against its own derivation in this module's tests. The *rule* was
/// always derivable from the siblings, but the **domain string is an arbitrary
/// chosen name** no implementer could infer — which is exactly why copying a
/// sibling's constant is the tempting mistake.
pub const MT_REGULAR_CONST: u128 = 0x0001_a2fc_877f_9528_d7c1;

/// The domain string [`MT_REGULAR_CONST`] is derived from.
pub const NUMS_DOMAIN: &[u8] = b"shibbolethnumstransaction";

// ── the header, spec §10.13(a2) ───────────────────────────────────────────────
//
// 55 bits = 11 symbols, and EVERY FIELD is a whole number of 5-bit symbols.
// That is not tidiness: it is what lets a hand engraver stop repeating the
// invariant prefix and keep the payload characters vertically aligned. There is
// no `chunked` flag — `mt1` is always chunked, so the bit encoded nothing, and
// a 1-bit field at offset 5 was what pushed every later field off a character
// boundary.

/// Width of the `version` field, in bits. One symbol.
pub const W_VERSION: u32 = 5;
/// Width of the `chunk_set_id` field, in bits. Four symbols.
pub const W_SET_ID: u32 = 20;
/// Width of the `count − 1` field, in bits. Three symbols.
pub const W_COUNT: u32 = 15;
/// Width of the `index` field, in bits. Three symbols.
pub const W_INDEX: u32 = 15;

/// Total header width in bits. Exactly 11 symbols.
pub const HEADER_BITS: u32 = W_VERSION + W_SET_ID + W_COUNT + W_INDEX;
/// Header width in 5-bit symbols.
pub const HEADER_SYMBOLS: usize = (HEADER_BITS / 5) as usize;

/// Width in symbols of the part of the header that is **identical across every
/// chunk of a set** — `version + chunk_set_id + count`.
///
/// Exactly 8, which is what makes `--elide-prefix` expressible: there is a clean
/// character boundary to cut at. Under the superseded 50-bit layout this was
/// 7.6 symbols and the eighth character mixed invariant bits with `index` bits.
pub const INVARIANT_PREFIX_SYMBOLS: usize = ((W_VERSION + W_SET_ID + W_COUNT) / 5) as usize;

/// The **ceiling the chunk count is derived from** — never a chunk's size.
///
/// Conflating the two is a defect this spec records twice: `bytes_per_chunk` is
/// `ceil(len / count)` and is usually *less* than this.
pub const PAYLOAD_CEILING_BYTES: usize = 40;

/// Largest representable chunk count, from [`W_COUNT`].
pub const MAX_CHUNKS: usize = 1 << W_COUNT;

/// NUMS target residue for the **long** BCH code: top 75 bits of the same
/// domain string.
///
/// **`mt1` never selects the long code, and cannot.** Worst case is a 40-byte
/// chunk: 11 header + 64 payload + 13 checksum = **88 symbols**, against the
/// regular code's 93-symbol domain. The constant exists because the ported
/// primitive offers both codes, and is derived rather than invented so it cannot
/// be a stale literal if a future format ever does need it. `unreachable_long_code`
/// asserts the bound rather than trusting this comment.
pub const MT_LONG_CONST: u128 = 0x68bf21dfe54a35f0481;

/// Largest data part any `mt1` string can have, in symbols.
pub const MAX_DATA_SYMBOLS: usize =
    HEADER_SYMBOLS + (PAYLOAD_CEILING_BYTES * 8).div_ceil(5) + CHECKSUM_SYMBOLS;

/// The regular BCH code's domain, in symbols.
pub const REGULAR_CODE_SYMBOLS_MAX: usize = 93;

/// Number of 5-bit checksum symbols on a regular-code string.
pub const CHECKSUM_SYMBOLS: usize = 13;

// ── compile-time invariants ───────────────────────────────────────────────────
//
// These were runtime `assert!`s until clippy pointed out they compare constants
// and are therefore optimised out — proving nothing. As `const` blocks they fail
// the BUILD instead, which is what you actually want from a bound that must hold
// for every possible input rather than for the ones a test happens to try.

/// The long BCH code is unreachable: a 40-byte chunk is 88 symbols against the
/// regular code's 93-symbol domain. If this ever fails, `mt1` has grown past the
/// regular code and that is a wire-format decision, not an implementation one.
const _: () = assert!(MAX_DATA_SYMBOLS <= REGULAR_CODE_SYMBOLS_MAX);

/// `count` must address a maximum-size standard transaction: ~100,000 vbytes at
/// 40 bytes per chunk is 2,500 chunks. A 10-bit field (1,024) would NOT clear it,
/// which is why per-field alignment settled on 15 rather than 10.
const _: () = assert!(MAX_CHUNKS > 100_000 / PAYLOAD_CEILING_BYTES);

/// The header and every field in it are whole numbers of 5-bit symbols — the
/// property `--elide-prefix` rests on.
const _: () = assert!(HEADER_BITS % 5 == 0);
const _: () = assert!(W_VERSION % 5 == 0 && W_SET_ID % 5 == 0);
const _: () = assert!(W_COUNT % 5 == 0 && W_INDEX % 5 == 0);
const _: () = assert!(INVARIANT_PREFIX_SYMBOLS == 8);

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// The drift test. Both siblings carry one; `md-codec`'s cites `mk`'s as its
    /// model. It catches the constant and its domain string drifting apart.
    #[test]
    fn regular_const_reproduces_from_domain() {
        let digest = Sha256::digest(NUMS_DOMAIN);
        let hi = u128::from_be_bytes(digest[0..16].try_into().unwrap());
        assert_eq!(
            hi >> 63,
            MT_REGULAR_CONST,
            "MT_REGULAR_CONST drift from SHA-256(NUMS_DOMAIN) top-65-bits"
        );
    }

    /// What the drift test CANNOT catch: a constant copied together with its
    /// domain string. Both would reproduce each other and the drift test would
    /// pass.
    ///
    /// **Hardcoded literals, deliberately — never imports.** Importing the
    /// siblings' constants would let a future refactor move both sides of the
    /// comparison together, which is the one thing these assertions exist to
    /// prevent. The mistake that actually gets made is against the crate you
    /// have open, and `mt-codec`'s string layer is ported from `mk`.
    #[test]
    fn const_differs_from_every_sibling() {
        const MD_REGULAR: u128 = 0x0000_815c_0774_7a33_92e7;
        const MK_REGULAR: u128 = 0x0001_0624_35f9_1072_fa5c;
        assert_ne!(MT_REGULAR_CONST, MD_REGULAR, "mt1 is using md1's constant");
        assert_ne!(MT_REGULAR_CONST, MK_REGULAR, "mt1 is using mk1's constant");

        const MD_DOMAIN: &[u8] = b"shibbolethnums";
        const MK_DOMAIN: &[u8] = b"shibbolethnumskey";
        assert_ne!(NUMS_DOMAIN, MD_DOMAIN, "mt1 is using md1's domain string");
        assert_ne!(NUMS_DOMAIN, MK_DOMAIN, "mt1 is using mk1's domain string");
    }

    /// The header arithmetic that `--elide-prefix` and every string length rest
    /// on. Asserted rather than trusted because three header layouts were ruled
    /// in one day and every length figure in the spec was stale after the third.
    #[test]
    fn header_is_symbol_aligned_per_field() {
        assert_eq!(HEADER_BITS, 55);
        assert_eq!(HEADER_SYMBOLS, 11);
        assert_eq!(HEADER_BITS % 5, 0, "header total is not symbol-aligned");
        for (name, w) in [
            ("version", W_VERSION),
            ("chunk_set_id", W_SET_ID),
            ("count", W_COUNT),
            ("index", W_INDEX),
        ] {
            assert_eq!(w % 5, 0, "{name} is not a whole number of symbols");
        }
        assert_eq!(
            INVARIANT_PREFIX_SYMBOLS, 8,
            "the elidable prefix must be exactly 8 symbols"
        );
    }

    /// The BOUND is enforced at compile time above; this pins the VALUE, so a
    /// change to the header or the ceiling shows up as a specific number rather
    /// than only as a build failure.
    #[test]
    fn worst_case_data_part_is_88_symbols() {
        assert_eq!(MAX_DATA_SYMBOLS, 88);
    }

    /// Derived, not pasted — the same discipline as the regular constant.
    #[test]
    fn long_const_reproduces_from_domain() {
        let digest = Sha256::digest(NUMS_DOMAIN);
        let hi = u128::from_be_bytes(digest[0..16].try_into().unwrap());
        assert_eq!(hi >> 53, MT_LONG_CONST, "MT_LONG_CONST drift");
    }

    /// The bound is enforced at compile time above; this pins the headroom so a
    /// reader can see what the width actually buys.
    #[test]
    fn count_headroom_over_the_standardness_bound() {
        const STANDARDNESS_CHUNKS: usize = 100_000 / PAYLOAD_CEILING_BYTES;
        assert_eq!(STANDARDNESS_CHUNKS, 2_500);
        assert_eq!(MAX_CHUNKS, 32_768);
        assert_eq!(MAX_CHUNKS / STANDARDNESS_CHUNKS, 13);
    }
}
