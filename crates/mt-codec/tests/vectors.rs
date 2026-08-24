//! The pinned vector corpus — the one artifact `mt-codec` did not produce.
//!
//! Everything else in this crate's tests is `mt-codec` agreeing with itself.
//! These vectors came from `scripts/gen-mt1-vectors.py` in `mnemonic-engrave`,
//! which re-implements bech32, the 55-bit header and the BCH polymod from BIP-93
//! and the spec, and which validates itself against 40/40 of `mk-codec`'s
//! committed corpus before emitting anything.
//!
//! **A vector this crate generated could not falsify this crate.** That is
//! precisely how a wrong NUMS constant would launder itself into looking
//! correct: every string would be self-consistent, and unreadable by every other
//! implementation. Regenerate with that script, never here.

use mt_codec::string_layer::pipeline;
use sha2::{Digest, Sha256};

/// `mk`'s pattern, adopted: the hash covers **the file the test actually reads**.
///
/// Re-pin by re-running the generator in `mnemonic-engrave` and pasting the new
/// digest here. Re-pinning from this crate's own output would defeat the point.
///
/// ```text
/// sha256sum crates/mt-codec/src/test_vectors/mt1_v1.json
/// ```
const VECTORS_V1_SHA256: &str = "ab5b3729b62d49f00dab206e973e177eafdb711d873c3a7c7968d22304b66087";

/// Generator provenance, so whoever re-pins can find the only thing allowed to
/// regenerate these (plan R11 M7 — a rule naming a script is unfollowable by
/// someone who cannot locate it).
const GENERATOR: &str = "mnemonic-engrave: scripts/gen-mt1-vectors.py";

fn corpus() -> serde_json::Value {
    serde_json::from_str(mt_codec::VECTORS_V1_JSON).expect("vector corpus is not valid JSON")
}

#[test]
fn vector_file_sha256_matches_pin() {
    let digest = Sha256::digest(mt_codec::VECTORS_V1_JSON.as_bytes());
    assert_eq!(
        format!("{digest:x}"),
        VECTORS_V1_SHA256,
        "the vector corpus changed. If that was deliberate, regenerate with \
         {GENERATOR} and re-pin — NOT from this crate's own output"
    );
}

/// The corpus records the header geometry it was generated under. If `mt-codec`
/// and the generator ever disagree about the layout, that disagreement shows up
/// here rather than as mysterious checksum failures.
#[test]
fn corpus_agrees_with_this_builds_geometry() {
    let c = corpus();
    assert_eq!(c["format"], "mt1");
    assert_eq!(c["wire_version"], u64::from(mt_codec::VERSION));
    assert_eq!(c["hrp"], mt_codec::HRP);
    assert_eq!(c["header_bits"], u64::from(mt_codec::consts::HEADER_BITS));
    assert_eq!(c["header_symbols"], mt_codec::consts::HEADER_SYMBOLS as u64);
    assert_eq!(
        c["mt_regular_const"].as_str().unwrap(),
        format!("{:#x}", mt_codec::MT_REGULAR_CONST),
        "the corpus was generated under a different NUMS constant than this build uses"
    );
    assert_eq!(
        c["nums_domain"].as_str().unwrap().as_bytes(),
        mt_codec::consts::NUMS_DOMAIN
    );
}

/// **The load-bearing test.** Encode the vector's transaction and demand the
/// exact strings the independent generator produced — byte for byte.
///
/// A wrong HRP, a wrong `version`, a plain `count`, a filling chunker, a wrong
/// NUMS constant or a mis-packed header each produce *different bytes*, so all
/// of them fail here by construction rather than needing a bespoke test apiece.
#[test]
fn encode_reproduces_every_pinned_string() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let raw = hex::decode(v["raw_hex"].as_str().unwrap()).unwrap();
        let txid = v["txid"].as_str().unwrap();
        let want: Vec<String> = v["strings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();

        let got = pipeline::encode(&raw, txid).expect("encode failed");
        assert_eq!(got, want, "{label}: strings differ from the pinned vector");
    }
}

#[test]
fn decode_round_trips_every_pinned_vector() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let raw = hex::decode(v["raw_hex"].as_str().unwrap()).unwrap();
        let strings: Vec<String> = v["strings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();

        let set = pipeline::decode(&strings).expect("decode failed");
        let (bytes, chunks) = (set.bytes, set.chunks);
        assert_eq!(bytes, raw, "{label}: round trip lost bytes");
        assert!(
            chunks.iter().all(|c| c.corrected == 0),
            "{label}: a clean vector needed correction"
        );
    }
}

/// §1.1a takes strings "in any order". In-order success proves nothing about
/// that, and a `sorted()` bug in the generator passed the in-order test — so
/// order-independence is asserted directly, reversed and rotated.
#[test]
fn decode_is_order_independent() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let raw = hex::decode(v["raw_hex"].as_str().unwrap()).unwrap();
        let strings: Vec<String> = v["strings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();

        let mut reversed = strings.clone();
        reversed.reverse();
        assert_eq!(
            pipeline::decode(&reversed).unwrap().bytes,
            raw,
            "{label}: decode depends on arrival order (reversed)"
        );

        let mut rotated = strings[2..].to_vec();
        rotated.extend_from_slice(&strings[..2]);
        assert_eq!(
            pipeline::decode(&rotated).unwrap().bytes,
            raw,
            "{label}: decode depends on arrival order (rotated)"
        );
    }
}

/// The 8-symbol invariant prefix is what `--elide-prefix` drops and what the
/// `PREFIX` row prints. The corpus records it, so a layout change that broke the
/// elision boundary would surface here.
#[test]
fn invariant_prefix_matches_the_corpus() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let want = v["invariant_prefix"].as_str().unwrap();
        let strings: Vec<&str> = v["strings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();

        for s in &strings {
            assert_eq!(
                pipeline::invariant_prefix(s).unwrap(),
                want,
                "{label}: invariant prefix differs on {s}"
            );
        }
        assert_eq!(
            want.len(),
            8,
            "{label}: the elidable prefix is not 8 characters"
        );
    }
}

/// The corpus records `txid` and `wtxid` separately, and they differ for every
/// segwit transaction. This pins §1.1's `TX` row by BYTES: double-SHA-256 of the
/// engraved serialisation is the **wtxid**, so an implementation wiring `TX` to
/// the hash of what `decode` emits fails here rather than shipping the defect.
#[test]
fn txid_is_not_the_hash_of_the_engraved_bytes() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let raw = hex::decode(v["raw_hex"].as_str().unwrap()).unwrap();
        let txid = v["txid"].as_str().unwrap();
        let wtxid = v["wtxid"].as_str().unwrap();
        assert_ne!(
            txid, wtxid,
            "{label}: vector does not exercise txid != wtxid"
        );

        let once = Sha256::digest(&raw);
        let twice = Sha256::digest(once);
        let mut display = twice.to_vec();
        display.reverse();
        assert_eq!(
            hex::encode(display),
            wtxid,
            "{label}: double-SHA-256 of the engraved bytes is not the wtxid"
        );
    }
}

/// The content id is the top 20 bits of the txid's DISPLAY form. "Which 20 bits,
/// from which end" is where two implementations diverge silently, so it is
/// checked against the corpus rather than derived twice the same way.
#[test]
fn content_id_matches_the_corpus() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let txid = v["txid"].as_str().unwrap();
        let want = u32::from_str_radix(v["set_id"].as_str().unwrap().trim_start_matches("0x"), 16)
            .unwrap();
        assert_eq!(
            pipeline::content_id_from_txid_display(txid).unwrap(),
            want,
            "{label}: content id derived differently from the generator"
        );
    }
}
