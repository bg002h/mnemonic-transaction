//! FIX-I, mt-codec decode strictness (bughunt-1119 bh-mt-I18, bh-mt-I19).
//! Rows R1-R8 assert the CORRECT behaviour and fail on unmodified code; R9 and
//! R10 are controls that pass before and after.
//!
//! Each RED row compares `decode(..)`'s result, with the error rendered by its
//! `Display`, against the expected refusal text. That form compiles on the
//! unmodified crate (no new variant is named), so the RED run shows the defect
//! itself: `left: Ok(..)` where the right side is an `Err`.

use mt_codec::string_layer::bch::{bytes_to_5bit, decode_string, encode_5bit_to_string};
use mt_codec::{ChunkHeader, decode, encode};

/// One BCH-valid string carrying `payload` under a hand-built header.
fn chunk(set: u32, count: usize, index: usize, payload: &[u8]) -> String {
    let mut syms: Vec<u8> = ChunkHeader::new(set, count, index)
        .unwrap()
        .to_symbols()
        .to_vec();
    syms.extend(bytes_to_5bit(payload));
    encode_5bit_to_string(&syms).unwrap()
}

/// A set cut from `bytes` at the given sizes, all under `set`.
fn cut(set: u32, bytes: &[u8], sizes: &[usize]) -> Vec<String> {
    assert_eq!(sizes.iter().sum::<usize>(), bytes.len(), "sizes must tile");
    let mut at = 0;
    sizes
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            let s = chunk(set, sizes.len(), i, &bytes[at..at + n]);
            at += n;
            s
        })
        .collect()
}

/// `decode`'s bytes, or its error as printed.
type Outcome = Result<Vec<u8>, String>;

fn result(strings: &[String]) -> Outcome {
    decode(strings).map(|d| d.bytes).map_err(|e| e.to_string())
}

/// The `even` corpus vector: 222 bytes in 6 chunks of 37, and its set id.
fn even() -> (Vec<u8>, u32) {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/src/test_vectors/mt1_v1.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let strings: Vec<String> = v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["label"] == "even")
        .unwrap()["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    let set = decode(&strings).unwrap();
    (set.bytes, set.chunks[0].header.chunk_set_id)
}

fn every_order_refuses(strings: &[String], want: &str) {
    let n = strings.len();
    let mut orders: Vec<Vec<usize>> = vec![(0..n).collect(), (0..n).rev().collect()];
    if n == 3 {
        orders = vec![
            vec![0, 1, 2],
            vec![1, 0, 2],
            vec![0, 2, 1],
            vec![2, 1, 0],
            vec![1, 2, 0],
            vec![2, 0, 1],
        ];
    }
    let got: Vec<(Vec<usize>, Outcome)> = orders
        .into_iter()
        .map(|o| {
            let s: Vec<String> = o.iter().map(|&i| strings[i].clone()).collect();
            let r = result(&s);
            (o, r)
        })
        .collect();
    for (order, r) in &got {
        assert_eq!(
            r,
            &Err(want.to_string()),
            "order {order:?}; every order: {got:?}"
        );
    }
}

/// R1 (I18, the triage set): non-final chunks of 2 and 3 bytes, final of 1.
#[test]
fn r1_uneven_middle_chunks_are_refused_in_every_order() {
    let c = [
        chunk(0x12345, 3, 0, &[1, 2]),
        chunk(0x12345, 3, 1, &[3, 4, 5]),
        chunk(0x12345, 3, 2, &[6]),
    ];
    every_order_refuses(
        &c,
        "chunk 1 of 3 has payload length 2; §3b's balanced split of a 6-byte payload gives count 1, bytes per chunk 6, last chunk 6",
    );
}

/// R2 (I18, a real transaction): the `even` vector re-cut 38/36/37/37/37/37
/// under its own set id. In index order the old decoder returns the true bytes
/// from a set no conforming encoder writes; reversed it returns 221 wrong bytes.
#[test]
fn r2_recut_even_vector_is_refused_in_both_orders() {
    let (bytes, set) = even();
    let c = cut(set, &bytes, &[38, 36, 37, 37, 37, 37]);
    every_order_refuses(
        &c,
        "chunk 1 of 6 has payload length 38; §3b's balanced split of a 222-byte payload gives count 6, bytes per chunk 37, last chunk 37",
    );
}

/// R3: a final chunk longer than the others.
#[test]
fn r3_final_chunk_longer_than_the_rest_is_refused() {
    let bytes: Vec<u8> = (0..223u32).map(|i| i as u8).collect();
    let c = cut(0x0abcd, &bytes, &[37, 37, 37, 37, 37, 38]);
    every_order_refuses(
        &c,
        "chunk 1 of 6 has payload length 37; §3b's balanced split of a 223-byte payload gives count 6, bytes per chunk 38, last chunk 33",
    );
}

/// R4: equal non-final chunks, but the split is not balanced (11 bytes is one
/// chunk under §3b, not 10 + 1).
#[test]
fn r4_unbalanced_two_chunk_set_is_refused() {
    let bytes: Vec<u8> = (0..11u8).collect();
    let c = cut(0x0abcd, &bytes, &[10, 1]);
    every_order_refuses(
        &c,
        "chunk 1 of 2 has payload length 10; §3b's balanced split of a 11-byte payload gives count 1, bytes per chunk 11, last chunk 11",
    );
}

/// R5: a FILLING chunker's set (40/40/10), the spec's named hazard (§3b).
#[test]
fn r5_filled_set_is_refused() {
    let bytes: Vec<u8> = (0..90u8).collect();
    let c = cut(0x0abcd, &bytes, &[40, 40, 10]);
    every_order_refuses(
        &c,
        "chunk 1 of 3 has payload length 40; §3b's balanced split of a 90-byte payload gives count 3, bytes per chunk 30, last chunk 30",
    );
}

/// R6: a single chunk over the 40-byte ceiling.
#[test]
fn r6_single_chunk_over_the_ceiling_is_refused() {
    let bytes: Vec<u8> = (0..41u8).collect();
    let c = cut(0x0abcd, &bytes, &[41]);
    assert_eq!(
        result(&c),
        Err(
            "chunk 1 of 1 has payload length 41; §3b's balanced split of a 41-byte payload gives count 2, bytes per chunk 21, last chunk 20"
                .to_string()
        )
    );
}

/// R7: an empty trailing chunk the plan has no place for.
#[test]
fn r7_empty_trailing_chunk_is_refused() {
    let bytes: Vec<u8> = (0..30u8).collect();
    let c = cut(0x0abcd, &bytes, &[30, 0]);
    every_order_refuses(
        &c,
        "chunk 2 of 2 has payload length 0; §3b's balanced split of a 30-byte payload gives count 1, bytes per chunk 30, last chunk 30",
    );
}

/// R8: 32,768 chunks of 41 bytes hold more than the header can address under
/// §3b's 40-byte ceiling; the existing `TooManyChunks` text names it.
#[test]
fn r8_total_beyond_the_header_ceiling_is_refused() {
    let count = 32_768usize;
    let c: Vec<String> = (0..count)
        .map(|i| chunk(0x0abcd, count, i, &[(i % 251) as u8; 41]))
        .collect();
    assert_eq!(
        result(&c).map(|b| b.len()),
        Err(
            "payload of 1343488 bytes needs 33588 chunks; the header addresses at most 32768"
                .to_string()
        )
    );
}

/// R9 (control, passes before and after): every conforming set decodes, in
/// index order and reversed, for every payload length 1..=600.
#[test]
fn r9_conforming_sets_of_every_length_still_decode() {
    let txid = "abcde00000000000000000000000000000000000000000000000000000000000";
    for len in 1usize..=600 {
        let bytes: Vec<u8> = (0..len).map(|i| (i * 7 + len) as u8).collect();
        let strings = encode(&bytes, txid).unwrap();
        let mut rev = strings.clone();
        rev.reverse();
        assert_eq!(result(&strings), Ok(bytes.clone()), "len {len}");
        assert_eq!(result(&rev), Ok(bytes), "len {len} reversed");
    }
}

/// R10 (control, T18): non-zero pad bits stay accepted; no canonical-encoding
/// guarantee is added (BIP-93 pads "with arbitrary bits").
#[test]
fn r10_i19_nonzero_padding_stays_accepted() {
    let good = chunk(0x00abc, 1, 0, &[0xA5]);
    let d = decode_string(&good).unwrap();
    let mut syms = d.data().to_vec();
    *syms.last_mut().unwrap() ^= 1;
    let tampered = encode_5bit_to_string(&syms).unwrap();
    assert_ne!(tampered, good);
    assert_eq!(result(&[tampered]), Ok(vec![0xA5]));
}
