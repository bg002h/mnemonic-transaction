//! `mt1`'s 55-bit chunk header, spec §10.13(a2).
//!
//! ```text
//! version(5) | chunk_set_id(20) | count−1(15) | index(15)   = 55 bits = 11 symbols
//! ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 40 bits = 8 symbols, IDENTICAL
//!                                              across every chunk of a set
//! ```
//!
//! **Every field is a whole number of 5-bit symbols**, which is the property the
//! whole layout was chosen for: it means the invariant prefix ends on a character
//! boundary, so a hand engraver can stop repeating it and keep the payload
//! characters vertically aligned (`--elide-prefix`, §3b).
//!
//! Two things this layout does NOT have, both deliberate:
//!
//! - **no `chunked` flag.** `mt1` is always chunked, so the bit encoded nothing
//!   — and a 1-bit field at offset 5 was exactly what pushed every later field
//!   off a character boundary. `version` alone identifies the generation.
//! - **no bit packer.** Because each field is symbol-aligned, the header is
//!   built and read by pushing and taking 5-bit symbols. `mt-codec` inherits
//!   none of `md-codec`'s bitstream, padding-tolerance or rollback machinery.

use crate::consts::{HEADER_BITS, HEADER_SYMBOLS, MAX_CHUNKS, VERSION, W_COUNT, W_INDEX, W_SET_ID};
use crate::error::{Error, Result};

/// A parsed `mt1` chunk header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    /// Wire-format generation. 5 bits.
    pub version: u8,
    /// Top 20 bits of the transaction's txid, in its display form (§10.13 c).
    pub chunk_set_id: u32,
    /// How many chunks the set has. Stored on the wire as `count − 1`.
    pub count: usize,
    /// This chunk's position, 0-based on the wire. Human-facing output numbers
    /// chunks from 1 (§1.1); `index` appears in no message.
    pub index: usize,
}

impl ChunkHeader {
    /// Build a header, checking the fields fit their widths.
    pub fn new(chunk_set_id: u32, count: usize, index: usize) -> Result<Self> {
        if count == 0 || count > MAX_CHUNKS {
            return Err(Error::TooManyChunks {
                len: 0,
                needed: count,
                max: MAX_CHUNKS,
            });
        }
        if index >= count {
            return Err(Error::IndexOutOfRange { index, count });
        }
        Ok(Self {
            version: VERSION,
            chunk_set_id: chunk_set_id & ((1 << W_SET_ID) - 1),
            count,
            index,
        })
    }

    /// Write the header as [`HEADER_SYMBOLS`] 5-bit symbols, most significant
    /// first.
    ///
    /// `count` is stored as **`count − 1`**, matching the constellation's offset
    /// convention: a set of 1 stores `0`. An implementer storing it plain
    /// produces plates whose every multi-chunk set is off by one — unreadable by
    /// the other implementation, and sending a recoverer to hunt a chunk that
    /// was never cut.
    pub fn to_symbols(self) -> [u8; HEADER_SYMBOLS] {
        let bits = (u64::from(self.version) << (W_SET_ID + W_COUNT + W_INDEX))
            | (u64::from(self.chunk_set_id) << (W_COUNT + W_INDEX))
            | ((self.count as u64 - 1) << W_INDEX)
            | self.index as u64;
        let mut out = [0u8; HEADER_SYMBOLS];
        for (i, slot) in out.iter_mut().enumerate() {
            let shift = HEADER_BITS - 5 * (i as u32 + 1);
            *slot = ((bits >> shift) & 0x1F) as u8;
        }
        out
    }

    /// Read a header from the first [`HEADER_SYMBOLS`] symbols of a data part.
    pub fn from_symbols(symbols: &[u8]) -> Result<Self> {
        if symbols.len() < HEADER_SYMBOLS {
            return Err(Error::TooShort {
                len: symbols.len(),
                header: HEADER_SYMBOLS,
                checksum: crate::consts::CHECKSUM_SYMBOLS,
                min: HEADER_SYMBOLS + crate::consts::CHECKSUM_SYMBOLS,
            });
        }
        let mut bits: u64 = 0;
        for &s in &symbols[..HEADER_SYMBOLS] {
            bits = (bits << 5) | u64::from(s);
        }
        let index = (bits & ((1 << W_INDEX) - 1)) as usize;
        let count = ((bits >> W_INDEX) & ((1 << W_COUNT) - 1)) as usize + 1;
        let chunk_set_id = ((bits >> (W_INDEX + W_COUNT)) & ((1 << W_SET_ID) - 1)) as u32;
        let version = (bits >> (W_INDEX + W_COUNT + W_SET_ID)) as u8;

        if version != VERSION {
            return Err(Error::UnknownVersion {
                found: version,
                known: VERSION,
            });
        }
        if index >= count {
            return Err(Error::IndexOutOfRange { index, count });
        }
        Ok(Self {
            version,
            chunk_set_id,
            count,
            index,
        })
    }

    /// The 8 symbols that are identical across every chunk of this set —
    /// `version + chunk_set_id + count`.
    ///
    /// This is what `--elide-prefix` drops from every string after the first,
    /// and what `mt encode` prints as the `PREFIX` row so an operator can tell
    /// which engravings belong together **by eye, without decoding**.
    pub fn invariant_prefix(self) -> [u8; crate::consts::INVARIANT_PREFIX_SYMBOLS] {
        let all = self.to_symbols();
        let mut out = [0u8; crate::consts::INVARIANT_PREFIX_SYMBOLS];
        out.copy_from_slice(&all[..crate::consts::INVARIANT_PREFIX_SYMBOLS]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::INVARIANT_PREFIX_SYMBOLS;

    #[test]
    fn round_trips_every_field() {
        for (set_id, count, index) in [
            (0x0_0000u32, 1usize, 0usize),
            (0xf_ffff, MAX_CHUNKS, MAX_CHUNKS - 1),
            (0x2_dcf2, 6, 5),
            (0x3_b426, 8, 3),
        ] {
            let h = ChunkHeader::new(set_id, count, index).unwrap();
            let back = ChunkHeader::from_symbols(&h.to_symbols()).unwrap();
            assert_eq!(back, h, "header round trip failed for {set_id:#x}");
        }
    }

    /// `count` is stored offset by one. A plain store is silently wrong on every
    /// multi-chunk set, so the encoding is asserted directly rather than only
    /// via a round trip — a round trip passes even if both sides are wrong the
    /// same way.
    #[test]
    fn count_is_stored_minus_one() {
        let h = ChunkHeader::new(0, 1, 0).unwrap();
        let syms = h.to_symbols();
        // version=1 occupies the whole first symbol; set_id is 0; so count−1=0
        // and index=0 means every later symbol is zero.
        assert_eq!(syms[0], VERSION, "version is not the first symbol");
        assert!(
            syms[1..].iter().all(|&s| s == 0),
            "a set of 1 must store count−1 = 0, got {syms:?}"
        );
    }

    #[test]
    fn invariant_prefix_is_identical_across_a_set() {
        let count = 6;
        let prefixes: std::collections::BTreeSet<_> = (0..count)
            .map(|i| {
                ChunkHeader::new(0x2_dcf2, count, i)
                    .unwrap()
                    .invariant_prefix()
            })
            .collect();
        assert_eq!(
            prefixes.len(),
            1,
            "the elidable prefix differs between chunks of one set"
        );
        assert_eq!(INVARIANT_PREFIX_SYMBOLS, 8);
    }

    #[test]
    fn rejects_index_at_or_past_count() {
        assert!(matches!(
            ChunkHeader::new(0, 4, 4),
            Err(Error::IndexOutOfRange { index: 4, count: 4 })
        ));
    }

    #[test]
    fn rejects_an_unknown_version() {
        let mut syms = ChunkHeader::new(0, 1, 0).unwrap().to_symbols();
        syms[0] = 31; // a generation this build does not write
        assert!(matches!(
            ChunkHeader::from_symbols(&syms),
            Err(Error::UnknownVersion { found: 31, .. })
        ));
    }
}
