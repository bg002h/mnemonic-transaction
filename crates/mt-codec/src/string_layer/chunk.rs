//! Chunking, spec §3b — **balanced, not filled**.
//!
//! ```text
//! count           = ceil(payload_len / 40)      // 40 is the CEILING
//! bytes_per_chunk = ceil(payload_len / count)   // usually LESS than 40
//! last chunk      = whatever remains
//! ```
//!
//! **The two constants do different jobs and conflating them is the defect this
//! module exists to prevent.** `40` is the ceiling the *count* derives from; it
//! never describes a chunk's size. A 535-byte payload gives `count = 14` and
//! `bytes_per_chunk = 39` — not thirteen 41-byte chunks, and not fourteen
//! chunks of 40/40/…/15.
//!
//! Getting this wrong is not a cosmetic difference: two implementations would
//! choose different chunk boundaries, and §1.1e's mandatory pre-decode length
//! check would then read the other's byte-perfect strings as **damaged steel**.

use crate::consts::{MAX_CHUNKS, PAYLOAD_CEILING_BYTES};
use crate::error::{Error, Result};

/// How a payload divides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunking {
    /// Number of chunks.
    pub count: usize,
    /// Bytes in every chunk but the last.
    pub bytes_per_chunk: usize,
    /// Bytes in the last chunk. Equal to [`Self::bytes_per_chunk`] only when the
    /// payload divides evenly.
    pub last_chunk_bytes: usize,
}

/// Divide `payload_len` bytes into balanced chunks.
pub fn plan(payload_len: usize) -> Result<Chunking> {
    let count = payload_len.div_ceil(PAYLOAD_CEILING_BYTES).max(1);
    if count > MAX_CHUNKS {
        return Err(Error::TooManyChunks {
            len: payload_len,
            needed: count,
            max: MAX_CHUNKS,
        });
    }
    let bytes_per_chunk = payload_len.div_ceil(count).max(1);
    let last_chunk_bytes = payload_len - (count - 1) * bytes_per_chunk;
    Ok(Chunking {
        count,
        bytes_per_chunk,
        last_chunk_bytes,
    })
}

/// The byte range of chunk `index`.
pub fn range(c: Chunking, index: usize) -> core::ops::Range<usize> {
    let start = index * c.bytes_per_chunk;
    let len = if index == c.count - 1 {
        c.last_chunk_bytes
    } else {
        c.bytes_per_chunk
    };
    start..start + len
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The spec's own measured artifacts. These are the numbers §3b's table
    /// carries, and they were stale in the spec for a whole day after the header
    /// widened — so they are pinned here against the rule rather than the table.
    #[test]
    fn matches_the_specs_measured_artifacts() {
        for (len, count, bpc, last) in [
            (162usize, 5usize, 33usize, 30usize),
            (405, 11, 37, 35),
            (535, 14, 39, 28),
            (742, 19, 40, 22),
            (560, 14, 40, 40),
            (2_498, 63, 40, 18),
            (222, 6, 37, 37), // the `even` vector
            (284, 8, 36, 32), // the `uneven` vector
        ] {
            let c = plan(len).unwrap();
            assert_eq!(
                (c.count, c.bytes_per_chunk, c.last_chunk_bytes),
                (count, bpc, last),
                "chunking wrong for {len} bytes"
            );
        }
    }

    /// BALANCED, not filled — the property that distinguishes this from a
    /// chunker that packs 40 bytes until it runs out. A filler also round-trips
    /// and also stays under the ceiling, so a round-trip test cannot tell them
    /// apart; only the boundaries can.
    #[test]
    fn is_balanced_and_never_filled() {
        for len in 1usize..=4_000 {
            let c = plan(len).unwrap();
            assert!(
                c.bytes_per_chunk <= PAYLOAD_CEILING_BYTES,
                "{len}: chunk exceeds the ceiling"
            );
            assert_eq!(
                c.bytes_per_chunk,
                len.div_ceil(c.count),
                "{len}: not balanced — this is a FILLING chunker"
            );
            assert!(
                c.last_chunk_bytes >= 1 && c.last_chunk_bytes <= c.bytes_per_chunk,
                "{len}: last chunk {} out of range",
                c.last_chunk_bytes
            );
            // the ranges must tile the payload exactly, with no gap or overlap
            let mut covered = 0usize;
            for i in 0..c.count {
                let r = range(c, i);
                assert_eq!(
                    r.start, covered,
                    "{len}: chunk {i} does not abut its predecessor"
                );
                covered = r.end;
            }
            assert_eq!(covered, len, "{len}: chunks do not tile the payload");
        }
    }

    /// A filling chunker would pass a round-trip test. Show it fails THIS test,
    /// so the test is known to discriminate rather than merely to pass.
    #[test]
    fn a_filling_chunker_would_fail_this() {
        // 535 bytes: balanced gives 14 x 39 (last 28); filling gives 13 x 40 + 15.
        let c = plan(535).unwrap();
        assert_eq!((c.count, c.bytes_per_chunk), (14, 39));
        let filled_count = 535_usize.div_ceil(PAYLOAD_CEILING_BYTES);
        assert_eq!(filled_count, 14, "count is the same either way");
        assert_ne!(
            c.bytes_per_chunk, PAYLOAD_CEILING_BYTES,
            "if bytes_per_chunk were 40 this chunker would be FILLING, and \
             §1.1e's length check would read a conforming set as damaged"
        );
    }

    #[test]
    fn refuses_a_payload_beyond_the_count_field() {
        let too_big = (MAX_CHUNKS + 1) * PAYLOAD_CEILING_BYTES;
        assert!(matches!(plan(too_big), Err(Error::TooManyChunks { .. })));
    }
}
