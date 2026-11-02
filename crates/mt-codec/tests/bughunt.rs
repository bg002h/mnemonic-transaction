//! Bug-hunt triage rows (mt-codec). Each asserts the CORRECT behaviour.
use mt_codec::string_layer::bch::{decode_string, encode_5bit_to_string};
use mt_codec::{decode, encode};

/// #4: chunk 0 of a 2-chunk set + chunk 1 of a 3-chunk set (same set id).
#[test]
fn bughunt_mt_04_decode_rejects_mixed_count_chunks() {
    let txid = "abcde00000000000000000000000000000000000000000000000000000000000";
    let a = encode(&[0x11u8; 70], txid).unwrap();
    let b = encode(&[0x22u8; 100], txid).unwrap();
    assert_eq!((a.len(), b.len()), (2, 3), "need count 2 vs 3");
    let r = decode(&[a[0].clone(), b[1].clone()]);
    assert!(
        r.is_err(),
        "mixed-count set accepted: {:?}",
        r.map(|d| d.bytes.len())
    );
}

/// #6: U+212A (Kelvin sign) in place of a data-part `k` must be InvalidChar.
#[test]
fn bughunt_mt_06_kelvin_sign_is_not_k() {
    let data: Vec<u8> = (0..80).map(|i| (i % 32) as u8).collect();
    let s = encode_5bit_to_string(&data).unwrap();
    assert!(decode_string(&s).is_ok());
    let i = s[3..].find('k').unwrap() + 3;
    let bad = format!("{}\u{212A}{}", &s[..i], &s[i + 1..]);
    let r = decode_string(&bad);
    assert!(
        r.is_err(),
        "accepted Kelvin sign in {bad:?}: corrections={:?}",
        r.map(|d| d.corrections_applied)
    );
}

/// #8: invariant_prefix on a too-short string must return Err, not panic.
#[test]
fn bughunt_mt_08_invariant_prefix_short_input_does_not_panic() {
    let r = std::panic::catch_unwind(|| mt_codec::string_layer::invariant_prefix("mt1qq"));
    assert!(r.is_ok(), "invariant_prefix panicked on short input");
    assert!(r.unwrap().is_err());
}
