//! `mt1`'s string layer: bytes ↔ `mt1…` strings.
//!
//! **Ported from `mk-codec/src/string_layer/`, not from `md-codec`.** `mk` is the
//! sibling that already faced the generalise-or-fork decision for a *second*
//! format; `md` has only ever been first. The constellation forks these
//! primitives per format rather than sharing a crate — `md1` and `mk1` use
//! HRP-mixed BCH with per-format target residues that are **not upstreamable**,
//! and the shared-crate plan was **retired 2026-05-03**, not deferred. So this
//! is the third instance of a pattern, never the third tenant of a future crate.
//!
//! Provenance pin: `mk-codec` 0.5.0. See `design/PROVENANCE.md`; a defect found
//! in any of the three BCH implementations triggers checking the other two.
//!
//! - [`bch`] — polymod, checksum, correction, bech32 alphabet helpers
//! - [`bch_decode`] — the syndrome/BM/Forney decoder. Format-agnostic: it takes
//!   the target residue as a parameter, which is why nothing in it is `mt`-specific
//! - [`header`] — the 55-bit, per-field symbol-aligned chunk header
//! - [`chunk`] — balanced chunking (§3b)

pub mod bch;
pub mod bch_decode;
pub mod chunk;
pub mod header;
pub mod pipeline;

pub use chunk::{Chunking, plan, range};
pub use header::ChunkHeader;
pub use pipeline::{DecodedChunk, decode, decode_chunk, encode, invariant_prefix};
