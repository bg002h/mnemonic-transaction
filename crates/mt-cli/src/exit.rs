//! The constellation exit codes (mnemonic-engrave
//! `design/SPEC_constellation_exit_codes.md` §3).
//!
//! **A local copy.** It mirrors the `exit` module of `mnemonic-io-lib` (spec
//! decision 4) until that crate is published with it; `mt` takes the library
//! from crates.io, and 0.1.3 does not carry it. When it does, this module is
//! replaced by the library's and the pinning test below moves with it.
//!
//! Codes 1 to 63 are sums of six bits, so any combination is one unique
//! number. 64 (usage) and 101 (internal) never combine.

use core::ops::{BitOr, BitOrAssign};

/// Everything requested was done and every check passed.
pub const OK: u8 = 0;
/// The input is well-formed and a comparison against something else came out
/// no (a `--transaction` that is not the one on the strings, an input the node
/// proves spent).
pub const NO: u8 = 1;
/// An input is wrong for this verb whatever the flags.
pub const INVALID: u8 = 2;
/// The tool checked part of the result and could not confirm it.
pub const UNVERIFIED: u8 = 4;
/// The tool declined under a safety or policy rule that a flag, channel or
/// different destination would waive.
pub const REFUSED: u8 = 8;
/// A checksum correction was applied to an input and the corrected value used.
pub const REPAIRED: u8 = 16;
/// Something the tool needed from the system failed or is missing.
pub const IO: u8 = 32;
/// The command line alone is wrong; nothing ran. Never combines.
pub const USAGE: u8 = 64;
/// A bug. Never combines.
pub const INTERNAL: u8 = 101;

/// The root `--help` legend (spec R11), word for word.
pub const LEGEND: &str = "Exit status: 0 = done, every check passed. 64 = bad command line, nothing \
ran. 101 = internal error. 130 = stopped by the operator. Other codes from 128 up = killed by a \
signal. Otherwise the sum (1 to 63) of: 1 a check said no; 2 an input is invalid; 4 verify the \
result before trusting it; 8 refused (network mismatch, secret on the command line, unsafe \
output); 16 a correction was applied; 32 a file, device or helper failed.";

/// An exit status: a set of the six combinable bits, or one of the codes that
/// never combine (usage, internal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Exit(u8);

impl Exit {
    /// 0.
    pub const OK: Self = Self(OK);
    /// 1.
    pub const NO: Self = Self(NO);
    /// 2.
    pub const INVALID: Self = Self(INVALID);
    /// 4.
    pub const UNVERIFIED: Self = Self(UNVERIFIED);
    /// 8.
    pub const REFUSED: Self = Self(REFUSED);
    /// 16.
    pub const REPAIRED: Self = Self(REPAIRED);
    /// 32.
    pub const IO: Self = Self(IO);
    /// 64; never combines.
    pub const USAGE: Self = Self(USAGE);
    /// 101; never combines.
    pub const INTERNAL: Self = Self(INTERNAL);

    /// Is every bit of `other` set here? For a code that never combines
    /// (usage, internal) this is equality.
    #[must_use]
    #[allow(dead_code)] // part of the mirrored API; mt's own code only sums
    pub fn contains(self, other: Self) -> bool {
        if self.is_alone() || other.is_alone() {
            return self == other;
        }
        self.0 & other.0 == other.0
    }

    /// The process exit status.
    #[must_use]
    pub fn to_exit_code(self) -> u8 {
        self.0
    }

    fn is_alone(self) -> bool {
        self.0 > 63
    }
}

/// Sums two outcomes. A code that never combines wins over the bits: usage
/// (nothing ran) over everything, then internal.
impl BitOr for Exit {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        match (self.0, rhs.0) {
            (USAGE, _) | (_, USAGE) => Self::USAGE,
            (a, b) if a > 63 || b > 63 => Self::INTERNAL,
            (a, b) => Self(a | b),
        }
    }
}

impl BitOrAssign for Exit {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers are the spec's (§3), not this crate's to choose.
    #[test]
    fn the_numbers_are_the_specs() {
        assert_eq!(
            [
                OK, NO, INVALID, UNVERIFIED, REFUSED, REPAIRED, IO, USAGE, INTERNAL
            ],
            [0, 1, 2, 4, 8, 16, 32, 64, 101]
        );
        for (c, n) in [
            (Exit::OK, 0),
            (Exit::NO, 1),
            (Exit::INVALID, 2),
            (Exit::UNVERIFIED, 4),
            (Exit::REFUSED, 8),
            (Exit::REPAIRED, 16),
            (Exit::IO, 32),
            (Exit::USAGE, 64),
            (Exit::INTERNAL, 101),
        ] {
            assert_eq!(c.to_exit_code(), n);
        }
    }

    #[test]
    fn every_sum_of_the_six_bits_is_unique_and_below_64() {
        let bits = [NO, INVALID, UNVERIFIED, REFUSED, REPAIRED, IO];
        let mut seen = std::collections::BTreeSet::new();
        for mask in 0u32..64 {
            let mut e = Exit::OK;
            for (i, b) in bits.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    e |= Exit(*b);
                }
            }
            assert!(e.to_exit_code() < 64);
            assert!(seen.insert(e.to_exit_code()));
        }
        assert_eq!(seen.len(), 64);
    }

    #[test]
    fn bits_combine_and_are_found() {
        let e = Exit::REPAIRED | Exit::INVALID;
        assert_eq!(e.to_exit_code(), 18);
        assert!(e.contains(Exit::REPAIRED));
        assert!(e.contains(Exit::INVALID));
        assert!(!e.contains(Exit::NO));
        assert!(e.contains(Exit::OK));
    }

    #[test]
    fn usage_and_internal_never_combine() {
        assert_eq!(Exit::USAGE | Exit::IO, Exit::USAGE);
        assert_eq!(Exit::IO | Exit::USAGE, Exit::USAGE);
        assert_eq!(Exit::INTERNAL | Exit::REPAIRED, Exit::INTERNAL);
        assert!(!Exit::USAGE.contains(Exit::OK));
        assert!(!Exit::INTERNAL.contains(Exit::NO));
        assert!(Exit::USAGE.contains(Exit::USAGE));
    }

    /// R11's legend, as the spec words it.
    #[test]
    fn the_legend_is_the_specs() {
        for part in [
            "0 = done, every check passed.",
            "64 = bad command line, nothing ran.",
            "101 = internal error.",
            "130 = stopped by the operator.",
            "Other codes from 128 up = killed by a signal.",
            "Otherwise the sum (1 to 63) of: 1 a check said no; 2 an input is invalid; 4 verify \
             the result before trusting it; 8 refused (network mismatch, secret on the command \
             line, unsafe output); 16 a correction was applied; 32 a file, device or helper \
             failed.",
        ] {
            assert!(LEGEND.contains(part), "missing: {part}");
        }
    }
}
