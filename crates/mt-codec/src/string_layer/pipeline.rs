//! Bytes ↔ `mt1` strings: the layer that joins header, chunking and BCH.
//!
//! This is the whole of `mt-codec`'s public behaviour. Everything a human reads
//! — reports, warnings, refusals, positions counted from 1 — belongs to
//! `mt-cli`; this module deals in bytes and symbols only.

use crate::consts::{
    CHECKSUM_SYMBOLS, HEADER_SYMBOLS, HRP, INVARIANT_PREFIX_SYMBOLS, MT_REGULAR_CONST,
};
use crate::error::{Error, Result};
use crate::string_layer::bch::{
    ALPHABET, bch_create_checksum_regular, bch_verify_regular, bytes_to_5bit,
};
use crate::string_layer::chunk::{self, Chunking};
use crate::string_layer::header::ChunkHeader;

/// Top 20 bits of a txid **in its display form** — the content id (§10.13 c).
///
/// The display form is the byte-reversed one a user reads, and "which 20 bits,
/// from which end" is exactly where two implementations diverge silently. So
/// this takes the display string rather than raw bytes, and takes it as the
/// caller already has it.
pub fn content_id_from_txid_display(txid_hex: &str) -> Result<u32> {
    let head = txid_hex
        .get(..5)
        .ok_or(Error::InvalidStringLength(txid_hex.len()))?;
    u32::from_str_radix(head, 16).map_err(|_| Error::InvalidHrp(txid_hex.to_string()))
}

/// Render one chunk as an `mt1…` string.
fn encode_chunk(header: ChunkHeader, payload: &[u8]) -> String {
    let mut data = Vec::with_capacity(HEADER_SYMBOLS + 64);
    data.extend_from_slice(&header.to_symbols());
    data.extend_from_slice(&bytes_to_5bit(payload));
    let checksum = bch_create_checksum_regular(HRP, &data);
    data.extend_from_slice(&checksum);

    let mut s = String::with_capacity(HRP.len() + 1 + data.len());
    s.push_str(HRP);
    s.push('1');
    for sym in &data {
        s.push(ALPHABET[*sym as usize] as char);
    }
    s
}

/// Encode a transaction into the full `mt1` string set.
///
/// `txid_display` is the transaction's txid as a user sees it. The caller
/// supplies it rather than this crate computing it, because §10.13(c) settles
/// *which* transaction's txid is meant — the **extracted** one — and that is a
/// PSBT question `mt-cli` owns.
pub fn encode(payload: &[u8], txid_display: &str) -> Result<Vec<String>> {
    let set_id = content_id_from_txid_display(txid_display)?;
    let plan = chunk::plan(payload.len())?;
    (0..plan.count)
        .map(|index| {
            let header = ChunkHeader::new(set_id, plan.count, index)?;
            Ok(encode_chunk(header, &payload[chunk::range(plan, index)]))
        })
        .collect()
}

/// Strip `mt1`, lowercase, and return the data-part symbols.
fn to_symbols(s: &str) -> Result<Vec<u8>> {
    let lower = s.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix(HRP)
        .and_then(|r| r.strip_prefix('1'))
        .ok_or_else(|| Error::InvalidHrp(s.to_string()))?;
    rest.chars()
        .enumerate()
        .map(|(i, c)| {
            ALPHABET
                .iter()
                .position(|&a| a as char == c)
                .map(|v| v as u8)
                .ok_or(Error::InvalidChar { ch: c, position: i })
        })
        .collect()
}

/// One decoded chunk, before reassembly.
#[derive(Debug, Clone)]
pub struct DecodedChunk {
    /// Its header.
    pub header: ChunkHeader,
    /// Its payload bytes, trimmed to the length the chunking rule dictates.
    pub payload: Vec<u8>,
    /// How many symbols BCH had to repair. **Zero is the ordinary case**, and
    /// anything approaching `t = 4` is what §1.1 requires `verify` to report:
    /// a chunk at its limit is one scratch from unrecoverable.
    pub corrected: usize,
    /// **Where** the repairs were, as 0-based offsets into the data part.
    ///
    /// Carried because §1.1 requires the margin report to LOCALISE corrections,
    /// not just count them: `pos 29 read v corrected to d` is a claim an operator
    /// settles against the steel in seconds, and counts alone leave them nothing
    /// to compare. `mt-cli` converts these to 1-based whole-string positions.
    pub corrected_positions: Vec<usize>,
    /// The symbol that was **read** at each corrected position, before repair,
    /// in the same order as `corrected_positions`.
    ///
    /// Without it the margin report can say *where* it repaired but not *what
    /// it repaired away* — and `pos 41 read v, corrected to d` is a claim an
    /// operator settles against the steel in seconds, while a bare position
    /// leaves them nothing to compare. **It is also the only way to tell a
    /// mis-cut from a mis-READ:** if the steel really says `d`, the plate is
    /// fine and the typist slipped.
    pub corrected_from: Vec<u8>,
    /// The symbol BCH put there instead, in the same order.
    ///
    /// Stored beside `corrected_from` rather than re-derived from the payload,
    /// so the pair is atomic: a re-derivation is a second implementation of the
    /// symbol layout, free to disagree with the first.
    pub corrected_to: Vec<u8>,
}

/// A chunk the set carried **twice**, and what `mt` did about it.
///
/// §1.8's advice is to cut a second copy, so duplicates are the *expected*
/// state of a well-kept drawer rather than an anomaly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicate {
    /// 0-based chunk index. `mt-cli` renders it 1-based.
    pub index: usize,
    /// Corrections the copy `mt` KEPT needed.
    pub kept_corrections: usize,
    /// Corrections the copy `mt` DISCARDED needed.
    pub discarded_corrections: usize,
}

/// A string `mt` could not read at all, and which copy saved the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// 1-based position in the input, which is what the operator can point at.
    pub input_position: usize,
    /// Why it could not be read.
    pub reason: String,
}

/// Everything `decode` learnt, not merely the bytes.
///
/// **A tuple would have hidden the last two fields**, and they are the two an
/// operator holding steel most needs: which plate is closest to unrecoverable,
/// and which one `mt` stopped trusting.
#[derive(Debug, Clone)]
pub struct DecodedSet {
    /// The reassembled transaction bytes.
    pub bytes: Vec<u8>,
    /// The chunk `mt` used for each index, in index order.
    pub chunks: Vec<DecodedChunk>,
    /// Duplicates resolved, in index order.
    pub duplicates: Vec<Duplicate>,
    /// Strings that could not be read, whose chunks came from another copy.
    pub unreadable: Vec<Unreadable>,
}

/// Parse and checksum-verify one `mt1` string, correcting up to `t = 4`.
pub fn decode_chunk(s: &str, plan: Option<Chunking>) -> Result<DecodedChunk> {
    let symbols = to_symbols(s)?;
    if symbols.len() < HEADER_SYMBOLS + CHECKSUM_SYMBOLS {
        return Err(Error::TooShort {
            len: symbols.len(),
            header: HEADER_SYMBOLS,
            checksum: CHECKSUM_SYMBOLS,
            min: HEADER_SYMBOLS + CHECKSUM_SYMBOLS,
        });
    }
    // Try the string AS WRITTEN first (§1.1e): correction is a repair attempted
    // on failure, never a preprocessing pass.
    let (symbols, corrected, corrected_positions, (corrected_from, corrected_to)) =
        if bch_verify_regular(HRP, &symbols) {
            (symbols, 0, Vec::new(), (Vec::new(), Vec::new()))
        } else {
            let r = crate::string_layer::bch::bch_correct_regular(HRP, &symbols)?;
            // `corrected_positions` index into the same array that was handed
            // in, so the pre-repair symbol is read from the ORIGINAL here —
            // after `r.data` shadows it there is nothing left to compare with.
            let from: Vec<u8> = r
                .corrected_positions
                .iter()
                .filter_map(|&i| symbols.get(i).copied())
                .collect();
            let to: Vec<u8> = r
                .corrected_positions
                .iter()
                .filter_map(|&i| r.data.get(i).copied())
                .collect();
            (
                r.data,
                r.corrections_applied,
                r.corrected_positions,
                (from, to),
            )
        };

    let header = ChunkHeader::from_symbols(&symbols)?;
    let body = &symbols[HEADER_SYMBOLS..symbols.len() - CHECKSUM_SYMBOLS];

    // The payload's byte length is known from the chunking rule, never inferred
    // from the symbol count — inferring it over-reads the final chunk's padding.
    let want = match plan {
        Some(p) if header.index == header.count - 1 => p.last_chunk_bytes,
        Some(p) => p.bytes_per_chunk,
        None => body.len() * 5 / 8,
    };
    let mut payload = Vec::with_capacity(want);
    let (mut acc, mut nbits) = (0u32, 0u32);
    for &sym in body {
        acc = (acc << 5) | u32::from(sym);
        nbits += 5;
        while nbits >= 8 {
            nbits -= 8;
            payload.push(((acc >> nbits) & 0xFF) as u8);
        }
    }
    payload.truncate(want);
    Ok(DecodedChunk {
        header,
        payload,
        corrected,
        corrected_positions,
        corrected_from,
        corrected_to,
    })
}

/// Reassemble a full set into the original bytes.
///
/// Ordering comes from each chunk's **header index**, never from the order the
/// strings arrived in — §1.1a takes them "in any order", and sorting them
/// lexicographically almost works, which is the worst kind of nearly-right.
pub fn decode(strings: &[String]) -> Result<DecodedSet> {
    // Read EVERY string first, keeping the failures rather than propagating the
    // first one.
    //
    // **A miscut plate must not kill a set that has a good copy of that chunk.**
    // §1.8's advice is to cut a second copy, and journey C is precisely the
    // drawer that followed it: one string damaged past t = 4, one clean, both
    // typed back. Failing on the first unreadable string would refuse a set that
    // is completely recoverable — and would do it while holding the good copy.
    let mut read: Vec<DecodedChunk> = Vec::new();
    let mut unreadable: Vec<Unreadable> = Vec::new();
    let mut first_error: Option<Error> = None;
    for (n, s) in strings.iter().enumerate() {
        match decode_chunk(s, None) {
            Ok(c) => read.push(c),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e.clone());
                }
                unreadable.push(Unreadable {
                    input_position: n + 1,
                    reason: e.to_string(),
                });
            }
        }
    }

    // Nothing readable at all: the FIRST error is the honest answer. Reporting
    // "missing chunk 1 of 0" for a file of garbage would send the operator
    // looking for a plate rather than at what they typed.
    let Some(first) = read.first() else {
        return Err(first_error.unwrap_or(Error::MissingChunk {
            missing: 1,
            count: 0,
        }));
    };
    let count = first.header.count;
    let set_id = first.header.chunk_set_id;

    // The full-chunk size must come from a chunk that is NOT the last one — the
    // last is shorter whenever the payload does not divide evenly. Taking it
    // from `strings[0]` works only while the strings happen to arrive in order,
    // which §1.1a explicitly does not promise; `decode_is_order_independent`
    // caught exactly that.
    let bpc = if count == 1 {
        0 // a single chunk is its own last: its length is taken directly below
    } else {
        read.iter()
            .find(|c| c.header.index < count - 1)
            .map(|c| c.payload.len())
            .ok_or(Error::MissingChunk { missing: 1, count })?
    };

    let mut slots: Vec<Option<DecodedChunk>> = vec![None; count];
    let mut duplicates: Vec<Duplicate> = Vec::new();
    for c in read {
        let (idx, cid) = (c.header.index, c.header.chunk_set_id);
        if cid != set_id {
            return Err(Error::SetIdMismatch {
                expected: set_id,
                found: cid,
                index: idx + 1,
            });
        }
        if idx >= count {
            return Err(Error::MissingChunk {
                missing: count,
                count,
            });
        }
        match slots[idx].take() {
            None => slots[idx] = Some(c),
            Some(existing) => {
                // §1.1's duplicate rule: byte-identical copies are §1.8's own
                // advice being followed, so accept. Distinct valid payloads are
                // the only ambiguous case.
                if existing.payload != c.payload {
                    return Err(Error::AmbiguousChunk {
                        index: idx + 1,
                        candidates: 2,
                    });
                }
                // KEEP THE HEALTHIER COPY, and say which one was dropped. Two
                // copies of one chunk agree on the payload but not on how much
                // of the t = 4 budget each spent getting there, and "first one
                // wins" would report the margin of whichever the operator
                // happened to type first. The point of a second plate is that
                // the better one is used.
                let (keep, drop) = if c.corrected < existing.corrected {
                    (c, existing)
                } else {
                    (existing, c)
                };
                duplicates.push(Duplicate {
                    index: idx,
                    kept_corrections: keep.corrected,
                    discarded_corrections: drop.corrected,
                });
                slots[idx] = Some(keep);
            }
        }
    }

    let mut chunks = Vec::with_capacity(count);
    let mut bytes = Vec::new();
    for (i, slot) in slots.into_iter().enumerate() {
        let c = slot.ok_or(Error::MissingChunk {
            missing: i + 1,
            count,
        })?;
        let keep = if i == count - 1 || count == 1 {
            c.payload.len()
        } else {
            bpc
        };
        bytes.extend_from_slice(&c.payload[..keep.min(c.payload.len())]);
        chunks.push(c);
    }
    duplicates.sort_by_key(|d| d.index);
    Ok(DecodedSet {
        bytes,
        chunks,
        duplicates,
        unreadable,
    })
}

/// The 8 characters every string of a set shares, after `mt1`.
///
/// What `--elide-prefix` drops and what `mt encode` prints as its `PREFIX` row,
/// so an operator can group engravings **by eye, without decoding**.
pub fn invariant_prefix(s: &str) -> Result<String> {
    let symbols = to_symbols(s)?;
    Ok(symbols[..INVARIANT_PREFIX_SYMBOLS]
        .iter()
        .map(|&v| ALPHABET[v as usize] as char)
        .collect())
}

/// The target residue this build checksums against. Exposed so a test can
/// assert it is `mt1`'s and not a sibling's.
pub const fn target_const() -> u128 {
    MT_REGULAR_CONST
}
