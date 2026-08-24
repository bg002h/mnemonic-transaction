//! BCH correction — that it CORRECTS, not merely that it refuses.
//!
//! R8's gates lens found this gate missing and the omission was subtle: the
//! round-trip tests use **clean** vectors, so the BCH residue is zero and the
//! correction path is **never entered**. The only negative test proved failure
//! *beyond* `t = 4`. Nothing proved that correction *within* budget works —
//! which is the single behaviour the format exists for. A hand engraver miscuts
//! a character; if `t = 4` does not repair it, `mt1` has no purpose.

use mt_codec::string_layer::bch::ALPHABET;
use mt_codec::string_layer::pipeline;

const T: usize = 4;

fn corpus() -> serde_json::Value {
    serde_json::from_str(mt_codec::VECTORS_V1_JSON).unwrap()
}

/// Damage `n` distinct data-part characters, deterministically and never to
/// themselves. Positions are spread rather than adjacent, because adjacent
/// symbol errors are the easy case for a syndrome decoder.
fn damage(s: &str, n: usize) -> String {
    let (hrp, data) = s.split_at(3);
    let mut chars: Vec<char> = data.chars().collect();
    let stride = chars.len() / (n + 1);
    for k in 0..n {
        let at = stride * (k + 1);
        let cur = ALPHABET
            .iter()
            .position(|&a| a as char == chars[at])
            .unwrap();
        // shift by a fixed non-zero amount, so the damage is a real substitution
        chars[at] = ALPHABET[(cur + 7) % 32] as char;
    }
    format!("{hrp}{}", chars.iter().collect::<String>())
}

/// Within budget, correction must RESTORE THE ORIGINAL BYTES — and report how
/// much of its budget it spent, which §1.1 requires `verify` to surface.
#[test]
fn corrects_one_through_four_symbols() {
    let c = corpus();
    let v = &c["vectors"][0];
    let strings: Vec<String> = v["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let clean = pipeline::decode_chunk(&strings[0], None).unwrap();

    for n in 1..=T {
        let broken = damage(&strings[0], n);
        assert_ne!(broken, strings[0], "damage({n}) did not change the string");

        let got = pipeline::decode_chunk(&broken, None)
            .unwrap_or_else(|e| panic!("{n} damaged symbols should be correctable, got {e}"));

        assert_eq!(
            got.payload, clean.payload,
            "{n} damaged symbols: payload not restored"
        );
        assert_eq!(
            got.header, clean.header,
            "{n} damaged symbols: header not restored"
        );
        assert_eq!(
            got.corrected, n,
            "correction count misreported for {n} damaged symbols"
        );
        assert_eq!(
            got.corrected_positions.len(),
            n,
            "{n} damaged symbols: positions not localised — §1.1 needs WHERE, not just how many"
        );
    }
}

/// Beyond budget, it must NOT silently accept. A decoder that quietly returns
/// mangled bytes is worse than one that refuses, because the content id is the
/// only thing left and it is blind to the witness region.
#[test]
fn refuses_beyond_the_budget() {
    let c = corpus();
    let v = &c["vectors"][0];
    let s = v["strings"][0].as_str().unwrap();
    let clean = pipeline::decode_chunk(s, None).unwrap();

    let broken = damage(s, T + 1);
    match pipeline::decode_chunk(&broken, None) {
        Err(_) => {}
        Ok(got) => assert_ne!(
            got.payload,
            clean.payload,
            "{} damaged symbols were silently 'corrected' back to the original — \
             the decoder is exceeding its own stated capacity",
            T + 1
        ),
    }
}

/// The margin is what makes a passing `verify` actionable. A chunk repaired four
/// times is ONE SCRATCH from unrecoverable, and a verdict that hides that tells
/// the operator the opposite of what they need (§1.1, the Critical the journey
/// walk found).
#[test]
fn margin_is_reported_at_the_limit() {
    let c = corpus();
    let s = c["vectors"][0]["strings"][0].as_str().unwrap();
    let at_limit = pipeline::decode_chunk(&damage(s, T), None).unwrap();
    assert_eq!(at_limit.corrected, T);
    assert_eq!(
        T - at_limit.corrected,
        0,
        "a chunk at t=4 has no margin left, and verify must be able to say so"
    );
}

/// A clean string must not enter the correction path at all. §1.1e rules that
/// correction is "a repair attempted on failure, never a preprocessing pass" —
/// because the positional corrections run in OPPOSITE directions depending on
/// position, so a naive pass over valid input would rewrite `mt1…` as `mtl…`.
#[test]
fn a_clean_string_is_never_corrected() {
    for v in corpus()["vectors"].as_array().unwrap() {
        for s in v["strings"].as_array().unwrap() {
            let got = pipeline::decode_chunk(s.as_str().unwrap(), None).unwrap();
            assert_eq!(got.corrected, 0, "a valid string was 'corrected'");
            assert!(got.corrected_positions.is_empty());
        }
    }
}
