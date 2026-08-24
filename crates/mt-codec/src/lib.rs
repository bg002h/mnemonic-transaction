//! `mt-codec` — the `mt1` wire format.
//!
//! `mt1` carries an **already-signed** Bitcoin transaction as chunked codex32
//! strings for hand engraving. This crate is the wire format only: no I/O, no
//! node, no CLI, and no refusals — those are `mt-cli`'s (spec §8).
//!
//! # What is pinned, and why it is pinned that way
//!
//! The vectors in `src/test_vectors/mt1_v1.json` were **not produced by this
//! crate**. They come from `scripts/gen-mt1-vectors.py` in `mnemonic-engrave`,
//! which re-implements bech32, the header and the BCH polymod from BIP-93 and the
//! spec. A vector this crate generated could not falsify this crate — that is
//! precisely how a wrong NUMS constant would launder itself into looking
//! correct. Regenerate with that script, never here.

pub mod consts;
pub mod error;
pub mod string_layer;

pub use consts::{HRP, MT_REGULAR_CONST, VERSION};
pub use error::{Error, Result};
pub use string_layer::pipeline;
pub use string_layer::{ChunkHeader, Chunking, DecodedChunk, decode, encode};

/// The pinned vector corpus, `include_str!`-baked so the SHA-256 pin covers the
/// bytes a test actually reads (`mk`'s pattern — see `tests/vectors.rs`).
pub const VECTORS_V1_JSON: &str = include_str!("test_vectors/mt1_v1.json");
