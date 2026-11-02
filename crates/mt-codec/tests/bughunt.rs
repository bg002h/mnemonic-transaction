//! Bug-hunt triage rows (mt-codec). Each asserts the CORRECT behaviour.
use mt_codec::string_layer::bch::{decode_string, encode_5bit_to_string};
use mt_codec::{Error, decode, encode};

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

/// FIX-B R4: a mixed-count set is refused with the new `CountMismatch`
/// variant, checked right after the set-id check. Asserted through `Debug`
/// and `Display` so this row compiles before the variant exists.
#[test]
fn bughunt_mt_04_count_mismatch_variant_and_text() {
    let txid = "abcde00000000000000000000000000000000000000000000000000000000000";
    let a = encode(&[0x11u8; 70], txid).unwrap();
    let b = encode(&[0x22u8; 100], txid).unwrap();
    assert_eq!((a.len(), b.len()), (2, 3), "need count 2 vs 3");
    let e = decode(&[a[0].clone(), b[1].clone()]).err();
    assert_eq!(
        e.as_ref().map(|e| format!("{e:?}")).as_deref(),
        Some("CountMismatch { expected: 2, found: 3, index: 2 }")
    );
    assert_eq!(
        e.map(|e| e.to_string()).as_deref(),
        Some("chunk set size mismatch: expected 2 chunks, chunk 2 carries 3")
    );
}

/// FIX-B R6: a non-ASCII character is refused as `InvalidChar` before case
/// folding, at its char offset in the data part (after `mt1`).
#[test]
fn bughunt_mt_06_kelvin_is_invalid_char_at_data_offset() {
    let data: Vec<u8> = (0..80).map(|i| (i % 32) as u8).collect();
    let s = encode_5bit_to_string(&data).unwrap();
    let i = s[3..].find('k').unwrap() + 3;
    assert_eq!(
        i, 25,
        "fixture moved: first data-part 'k' is no longer at byte 25"
    );
    let bad = format!("{}\u{212A}{}", &s[..i], &s[i + 1..]);
    let e = decode_string(&bad).err();
    assert_eq!(
        e,
        Some(Error::InvalidChar {
            ch: '\u{212A}',
            position: 22
        })
    );
    assert_eq!(
        e.map(|e| e.to_string()).as_deref(),
        Some("character '\u{212A}' at data-part offset 22 is not in the bech32 alphabet")
    );
}

/// FIX-B R8: `invariant_prefix` on a string too short to hold a header and a
/// checksum returns the existing `TooShort`, with the same fields
/// `decode_chunk` uses.
#[test]
fn bughunt_mt_08_invariant_prefix_short_input_is_too_short() {
    let r = std::panic::catch_unwind(|| mt_codec::string_layer::invariant_prefix("mt1qq"));
    assert_eq!(
        r.ok(),
        Some(Err(Error::TooShort {
            len: 2,
            header: 11,
            checksum: 13,
            min: 24
        }))
    );
}

/// FIX-B R8 (review M2): the guard threshold is 24 symbols, not 8.
#[test]
fn bughunt_mt_08_ten_symbol_string_is_too_short() {
    assert_eq!(
        mt_codec::string_layer::invariant_prefix("mt1qqqqqqqqqq"),
        Err(Error::TooShort {
            len: 10,
            header: 11,
            checksum: 13,
            min: 24
        })
    );
}
