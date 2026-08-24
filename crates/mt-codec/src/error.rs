//! `mt-codec`'s error type.
//!
//! Every variant carries **the value that caused it**, because §8 promises each
//! refusal names its own number and `mt-cli` cannot name what the codec did not
//! hand it.

use thiserror::Error;

/// What can go wrong reading or writing `mt1`.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// A character outside bech32's alphabet. `1`, `b`, `i` and `o` are absent
    /// from the charset precisely because they are confusable when engraved.
    ///
    /// **`position` is a 0-based offset into the DATA PART**, which is the
    /// codec's natural coordinate. `mt-cli` converts it to the 1-based
    /// whole-string position §1.1 requires for anything a human reads — the
    /// codec does not know about `mt1` prefixes or grouping, and a report that
    /// mixed the two coordinate systems is how an operator gets sent to the
    /// wrong character.
    #[error("character {ch:?} at data-part offset {position} is not in the bech32 alphabet")]
    InvalidChar {
        /// The offending character.
        ch: char,
        /// 0-based offset into the data part.
        position: usize,
    },

    /// Too short to contain a header and a checksum.
    #[error(
        "string is {len} symbols; a header ({header}) plus a checksum ({checksum}) needs at least {min}"
    )]
    TooShort {
        /// Symbols present in the data part.
        len: usize,
        /// Header width in symbols.
        header: usize,
        /// Checksum width in symbols.
        checksum: usize,
        /// The minimum that could parse.
        min: usize,
    },

    /// The BCH checksum does not hold and correction could not repair it.
    ///
    /// Carries how many symbols correction *did* fix before giving up, because
    /// §1.1 requires `verify` to report its margin rather than only a verdict:
    /// a chunk repaired four times is one scratch from unrecoverable.
    #[error("checksum invalid; correction repaired {corrected} of at most {budget} symbols")]
    ChecksumFailed {
        /// Symbols corrected before the attempt failed.
        corrected: usize,
        /// The per-chunk correction budget, `t`.
        budget: usize,
    },

    /// A wire `version` this build does not know.
    #[error("unknown wire version {found}; this build understands {known}")]
    UnknownVersion {
        /// The version read from the header.
        found: u8,
        /// The version this build writes.
        known: u8,
    },

    /// `index` is not less than `count`.
    #[error("chunk index {index} is out of range for a set of {count}")]
    IndexOutOfRange {
        /// The offending index, as it appears on the wire (0-based).
        index: usize,
        /// The set size read from the header.
        count: usize,
    },

    /// Chunks in one set disagree about which set they belong to.
    #[error("chunk set id mismatch: expected {expected:#07x}, chunk {index} carries {found:#07x}")]
    SetIdMismatch {
        /// The id the rest of the set carries.
        expected: u32,
        /// The id this chunk carries.
        found: u32,
        /// 1-based chunk number, as printed to a human.
        index: usize,
    },

    /// A chunk is missing from the set.
    #[error("chunk {missing} of {count} is missing")]
    MissingChunk {
        /// 1-based chunk number.
        missing: usize,
        /// The set size.
        count: usize,
    },

    /// Two chunks claim the same index and cannot be reconciled.
    ///
    /// Only raised for the genuinely ambiguous case: several candidates whose
    /// checksums all hold and whose payloads differ. A single valid candidate
    /// wins, and byte-identical duplicates are accepted silently — §1.8 tells
    /// operators to cut spare copies, so refusing them would refuse the one
    /// mitigation the spec offers.
    #[error("chunk {index} has {candidates} distinct valid candidates; cannot choose")]
    AmbiguousChunk {
        /// 1-based chunk number.
        index: usize,
        /// How many distinct valid byte strings claim it.
        candidates: usize,
    },

    /// The reassembled transaction does not re-derive the set's content id.
    ///
    /// **This identifies the transaction; it does not prove every byte.** The
    /// content id is the txid, which is blind to the witness region — so damage
    /// there re-derives the expected id and passes. Error correction is BCH's
    /// job, per chunk.
    #[error(
        "reassembled transaction does not match the set's content id {expected:#07x} (derived {derived:#07x})"
    )]
    ContentIdMismatch {
        /// The id every chunk header carries.
        expected: u32,
        /// The id derived from the reassembled bytes.
        derived: u32,
    },

    /// A payload too large for the count field to address.
    #[error("payload of {len} bytes needs {needed} chunks; the header addresses at most {max}")]
    TooManyChunks {
        /// Payload length in bytes.
        len: usize,
        /// Chunks required.
        needed: usize,
        /// The header's ceiling.
        max: usize,
    },

    /// BCH correction could not repair the string within `t = 4`.
    ///
    /// Carries the decoder's own account, because §1.1 requires `verify` to
    /// report **how much of its budget it spent**, not merely a verdict: a chunk
    /// repaired four times passes while sitting one scratch from unrecoverable.
    #[error("BCH correction failed: {0}")]
    BchUncorrectable(String),

    /// The data part is not a length any `mt1` string can have.
    #[error("data part of {0} symbols is not a valid mt1 length")]
    InvalidStringLength(usize),

    /// Mixed case. Normalisation is free, so `mt` lowercases input before
    /// anything else — but a *string* mixing cases is not a string this codec
    /// will silently reinterpret.
    #[error("string mixes upper and lower case")]
    MixedCase,

    /// Missing or wrong human-readable part. Carries the string as seen.
    #[error("not an mt1 string: {0:?}")]
    InvalidHrp(String),
}

/// `mt-codec`'s result alias.
pub type Result<T> = core::result::Result<T, Error>;
