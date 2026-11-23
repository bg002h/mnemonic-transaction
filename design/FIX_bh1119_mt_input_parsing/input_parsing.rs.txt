//! Bughunt 2026-11-19 FIX-C, input parsing (design/FIX_bh1119_mt_input_parsing.md).
//!
//! Keys: bh-mt-I1 (a transaction read from a cursor without an end check),
//! bh-mt-I2 (a PSBT read without an end check, and a base64 decoder that stops
//! at the first `=`), bh-mt-I20 (a base64 PSBT recognised by a 7-character
//! prefix that depends on the first key's length). Ruling T23: sniff by
//! decoding.
//!
//! Every run is offline (`--bitcoin-cli /nonexistent/bitcoin-cli`). Inputs are
//! the public fixtures `tests/fixtures/p5_base.json` and the corpus vectors in
//! `mt-codec/src/test_vectors/mt1_v1.json`.

use assert_cmd::Command;
use std::io::Write;

const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn p5(key: &str) -> String {
    let v = json(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/p5_base.json"
    ));
    v[key].as_str().unwrap().to_string()
}

fn vector(label: &str) -> serde_json::Value {
    let v = json(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    ));
    v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["label"] == label)
        .unwrap()
        .clone()
}

/// p5's finalized 2-in/1-out transaction, 339 bytes, as hex.
fn raw_hex() -> String {
    p5("raw_hex")
}

/// The corpus `even` transaction, 222 bytes, as hex.
fn even_hex() -> String {
    vector("even")["raw_hex"].as_str().unwrap().to_string()
}

/// The corpus `uneven` transaction, as hex: a second well-formed transaction.
fn uneven_hex() -> String {
    vector("uneven")["raw_hex"].as_str().unwrap().to_string()
}

/// The `even` set's mt1 strings, one per line.
fn even_strings() -> String {
    vector("even")["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string() + "\n")
        .collect()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn to_hex(b: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    b.iter()
        .flat_map(|x| [H[(x >> 4) as usize] as char, H[(x & 15) as usize] as char])
        .collect()
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((*c.get(1).unwrap_or(&0) as u32) << 8)
            | (*c.get(2).unwrap_or(&0) as u32);
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// p5's finalized PSBT, 927 bytes. Its base64 has no padding.
fn final_psbt() -> Vec<u8> {
    let s = p5("finalized_psbt_b64");
    let b = b64_decode_for_fixture(&s);
    assert_eq!(b64(&b), s, "fixture base64 round-trips");
    b
}

fn b64_decode_for_fixture(s: &str) -> Vec<u8> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes().filter(|&c| c != b'=') {
        acc = (acc << 6) | T.iter().position(|&t| t == c).unwrap() as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// The same PSBT with one unknown global record (type 0xfa, value 0x2a) placed
/// first: 931 bytes, so its base64 ends `==`. The first key length is 1, so it
/// still begins `cHNidP8`.
fn padded_psbt() -> Vec<u8> {
    let f = final_psbt();
    let mut p = f[..5].to_vec();
    p.extend_from_slice(&[0x01, 0xfa, 0x01, 0x2a]);
    p.extend_from_slice(&f[5..]);
    p
}

/// bh-mt-I20's PSBT: a first global key 64 bytes long (type 0xfa, 63 zero
/// bytes), value 0x2a. Its compact-size length byte 0x40 sets the high bits
/// that the 7th base64 character carries, so the text begins `cHNidP9`.
fn long_key_psbt() -> Vec<u8> {
    let f = final_psbt();
    let mut p = f[..5].to_vec();
    p.push(64);
    p.push(0xfa);
    p.extend_from_slice(&[0u8; 63]);
    p.extend_from_slice(&[0x01, 0x2a]);
    p.extend_from_slice(&f[5..]);
    p
}

fn file(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

struct Run {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn run(verb: &str, extra: &[&str], input: &[u8]) -> Run {
    let f = file(input);
    let out = mt()
        .arg(verb)
        .args(extra)
        .args(["--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    Run {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Exit 1, the exact first line, and nothing on stdout.
fn refused(r: &Run, line: &str) {
    assert!(
        !r.ok,
        "the run succeeded; expected:\n{line}\nstderr:\n{}",
        r.stderr
    );
    assert!(
        r.stderr.lines().any(|l| l == line),
        "expected the line\n{line}\nstderr:\n{}",
        r.stderr
    );
    assert!(
        r.stdout.is_empty(),
        "a refusal wrote to stdout:\n{}",
        r.stdout
    );
}

fn accepted(r: &Run) {
    assert!(r.ok, "the run was refused:\n{}", r.stderr);
    assert!(!r.stderr.contains("REFUSED"), "stderr:\n{}", r.stderr);
}

/// `--qr` output: one `tx:` record, the canonical transaction hex.
fn qr(input: &[u8]) -> Run {
    run("encode", &["--qr"], input)
}

fn record_of(raw: &str) -> String {
    format!("tx:{raw}\n")
}

/// Strings for `even` + one trailing 0xab byte, carrying `even`'s own txid,
/// built through the public codec exactly as bh-mt-I1's recovery probe did.
fn even_plus_ab_strings() -> String {
    let mut bytes = hex(&even_hex());
    let tx: bitcoin::Transaction = bitcoin::consensus::deserialize(&bytes).unwrap();
    bytes.push(0xab);
    mt_codec::string_layer::pipeline::encode(&bytes, &tx.compute_txid().to_string())
        .unwrap()
        .iter()
        .map(|s| s.clone() + "\n")
        .collect()
}

// ── bh-mt-I1: a transaction is read to the end of its bytes ──────────────────

#[test]
fn bh1119_mt_i1_encode_refuses_raw_then_even_hex() {
    let r = qr(format!("{}\n{}\n", raw_hex(), even_hex()).as_bytes());
    refused(
        &r,
        "mt encode: REFUSED — §8.2e, the input continues for 222 bytes after the end of the transaction",
    );
}

#[test]
fn bh1119_mt_i1_encode_refuses_even_then_raw_hex() {
    let r = qr(format!("{}\n{}\n", even_hex(), raw_hex()).as_bytes());
    refused(
        &r,
        "mt encode: REFUSED — §8.2e, the input continues for 339 bytes after the end of the transaction",
    );
}

#[test]
fn bh1119_mt_i1_encode_refuses_one_trailing_byte_on_hex() {
    let r = qr(format!("{}ab\n", even_hex()).as_bytes());
    refused(
        &r,
        "mt encode: REFUSED — §8.2e, the input continues for 1 byte after the end of the transaction",
    );
}

#[test]
fn bh1119_mt_i1_inspect_refuses_one_trailing_byte_on_hex() {
    let r = run("inspect", &[], format!("{}ab\n", even_hex()).as_bytes());
    refused(
        &r,
        "mt inspect: REFUSED — §8.2e, the input continues for 1 byte after the end of the transaction",
    );
}

#[test]
fn bh1119_mt_i1_verify_refuses_trailing_byte_on_transaction_file() {
    let t = file(format!("{}ab\n", even_hex()).as_bytes());
    let r = run(
        "verify",
        &["--transaction", t.path().to_str().unwrap()],
        even_strings().as_bytes(),
    );
    refused(
        &r,
        "mt verify: REFUSED — §8.2e, the input continues for 1 byte after the end of the transaction",
    );
    assert!(!r.stderr.contains("--transaction matches"), "{}", r.stderr);
}

/// Control: two independently well-formed transactions are still compared on
/// the full txid, and the mismatch refusal is unchanged.
#[test]
fn bh1119_mt_i1_verify_still_compares_two_well_formed_transactions() {
    let t = file(format!("{}\n", uneven_hex()).as_bytes());
    let r = run(
        "verify",
        &["--transaction", t.path().to_str().unwrap()],
        even_strings().as_bytes(),
    );
    assert!(!r.ok, "{}", r.stderr);
    assert!(
        r.stderr
            .contains("the supplied transaction is not the one on these strings"),
        "{}",
        r.stderr
    );
    assert!(r.stdout.is_empty());
}

#[test]
fn bh1119_mt_i1_decode_refuses_set_with_trailing_byte() {
    let r = run("decode", &[], even_plus_ab_strings().as_bytes());
    refused(
        &r,
        "mt decode: REFUSED — §1.1, the reassembled bytes continue for 1 byte after the end of the transaction",
    );
}

#[test]
fn bh1119_mt_i1_verify_refuses_set_with_trailing_byte() {
    let t = file(format!("{}\n", even_hex()).as_bytes());
    let r = run(
        "verify",
        &["--transaction", t.path().to_str().unwrap()],
        even_plus_ab_strings().as_bytes(),
    );
    refused(
        &r,
        "mt verify: REFUSED — §1.1, the reassembled bytes continue for 1 byte after the end of the transaction",
    );
    assert!(!r.stderr.contains("mt verify: OK"), "{}", r.stderr);
}

#[test]
fn bh1119_mt_i1_inspect_refuses_set_with_trailing_byte() {
    let r = run("inspect", &[], even_plus_ab_strings().as_bytes());
    refused(
        &r,
        "mt inspect: REFUSED — §1.1, the reassembled bytes continue for 1 byte after the end of the transaction",
    );
}

// ── bh-mt-I2: a PSBT is read to the end; base64 is decoded whole ─────────────

#[test]
fn bh1119_mt_i2_encode_refuses_two_binary_psbts() {
    let mut two = final_psbt();
    two.extend_from_slice(&final_psbt());
    refused(
        &qr(&two),
        "mt encode: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT",
    );
}

#[test]
fn bh1119_mt_i2_encode_refuses_two_unpadded_base64_psbts() {
    let one = p5("finalized_psbt_b64");
    refused(
        &qr(format!("{one}\n{one}\n").as_bytes()),
        "mt encode: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT",
    );
}

const B64_DAMAGED: &str = "REFUSED — §8.2e, input looks like a base64 PSBT but does not decode";

#[test]
fn bh1119_mt_i2_encode_refuses_padded_base64_then_second_record() {
    let first = b64(&padded_psbt());
    assert!(first.ends_with("=="));
    let r = qr(format!("{first}\n{}\n", p5("finalized_psbt_b64")).as_bytes());
    refused(&r, &format!("mt encode: {B64_DAMAGED}"));
}

#[test]
fn bh1119_mt_i2_encode_refuses_text_after_padding() {
    let r = qr(format!("{}oops\n", b64(&padded_psbt())).as_bytes());
    refused(&r, &format!("mt encode: {B64_DAMAGED}"));
}

#[test]
fn bh1119_mt_i2_encode_refuses_equals_then_junk() {
    let r = qr(format!("{}=oops\n", p5("finalized_psbt_b64")).as_bytes());
    refused(&r, &format!("mt encode: {B64_DAMAGED}"));
}

#[test]
fn bh1119_mt_i2_encode_refuses_nonzero_padding_bits() {
    let mut s = b64(&padded_psbt()).into_bytes();
    // The character before `==` carries 2 data bits and 4 padding bits; the
    // next alphabet letter sets a padding bit and leaves the bytes unchanged.
    let i = s.len() - 3;
    assert_eq!(s[i], b'A', "the fixture's last data character");
    s[i] = b'B';
    let r = qr(&s);
    refused(&r, &format!("mt encode: {B64_DAMAGED}"));
}

#[test]
fn bh1119_mt_i2_encode_refuses_dangling_base64_character() {
    let r = qr(format!("{}A\n", p5("finalized_psbt_b64")).as_bytes());
    refused(&r, &format!("mt encode: {B64_DAMAGED}"));
    let flat = r.stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("one character left over"), "{}", r.stderr);
}

/// Text input: `inspect` reads a binary PSBT as text first (bh-mt-I5, FIX-D),
/// so this row uses the base64 form.
#[test]
fn bh1119_mt_i2_inspect_refuses_two_base64_psbts() {
    let one = p5("finalized_psbt_b64");
    refused(
        &run("inspect", &[], format!("{one}\n{one}\n").as_bytes()),
        "mt inspect: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT",
    );
}

#[test]
fn bh1119_mt_i2_verify_refuses_two_psbts_as_transaction_file() {
    let mut two = final_psbt();
    two.extend_from_slice(&final_psbt());
    let t = file(&two);
    // p5's transaction as a set, so only the supplied file is at fault.
    let strings = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE])
        .write_stdin(raw_hex())
        .output()
        .unwrap();
    assert!(strings.status.success());
    let r = run(
        "verify",
        &["--transaction", t.path().to_str().unwrap()],
        &strings.stdout,
    );
    refused(
        &r,
        "mt verify: REFUSED — §8.2e, the input continues for 927 bytes after the end of the PSBT",
    );
}

/// Control: a base64 export wrapped at 64 columns still reads as the PSBT.
#[test]
fn bh1119_mt_i2_encode_reads_line_wrapped_base64() {
    let one = p5("finalized_psbt_b64");
    let wrapped: String = one
        .as_bytes()
        .chunks(64)
        .map(|c| std::str::from_utf8(c).unwrap().to_string() + "\n")
        .collect();
    let r = qr(wrapped.as_bytes());
    accepted(&r);
    assert_eq!(r.stdout, record_of(&raw_hex()));
}

/// Control: base64 without its padding still decodes (decided: unchanged).
#[test]
fn bh1119_mt_i2_encode_reads_unpadded_base64() {
    let padded = b64(&padded_psbt());
    let r = qr(padded.trim_end_matches('=').as_bytes());
    accepted(&r);
    assert_eq!(r.stdout, record_of(&raw_hex()));
    let r = qr(padded.as_bytes());
    accepted(&r);
    assert_eq!(r.stdout, record_of(&raw_hex()));
}

// ── bh-mt-I20: base64 is recognised by decoding, not by `cHNidP8` ────────────

#[test]
fn bh1119_mt_i20_encode_reads_base64_with_long_first_key() {
    let text = b64(&long_key_psbt());
    assert!(text.starts_with("cHNidP9"), "{}", &text[..8]);
    let bin = run("encode", &[], &long_key_psbt());
    accepted(&bin);
    let r = run("encode", &[], text.as_bytes());
    accepted(&r);
    assert_eq!(r.stdout, bin.stdout, "base64 and binary of one PSBT differ");
    assert_eq!(qr(text.as_bytes()).stdout, record_of(&raw_hex()));
}

/// Compared with the ordinary base64 export of the same transaction: `inspect`
/// reads a binary PSBT as text first (bh-mt-I5, FIX-D).
#[test]
fn bh1119_mt_i20_inspect_reads_base64_with_long_first_key() {
    let r = run("inspect", &[], b64(&long_key_psbt()).as_bytes());
    accepted(&r);
    let plain = run("inspect", &[], p5("finalized_psbt_b64").as_bytes());
    accepted(&plain);
    assert_eq!(r.stdout, plain.stdout);
}

#[test]
fn bh1119_mt_i20_encode_names_a_damaged_long_key_base64_psbt() {
    let mut s = b64(&long_key_psbt()).into_bytes();
    s[200] = b'*';
    refused(&qr(&s), &format!("mt encode: {B64_DAMAGED}"));
}

#[test]
fn bh1119_mt_i20_argv_classifier_refuses_long_key_base64() {
    let text = b64(&long_key_psbt());
    let out = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, &text])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("mt encode: REFUSED — §8.2f,"),
        "expected the §8.2f argv refusal:\n{stderr}"
    );
    assert!(
        !stderr.contains(&text[..40]),
        "the PSBT was echoed:\n{stderr}"
    );
    assert!(out.stdout.is_empty());
}

/// The trailing-byte set really carries `even`'s txid, so only the end check
/// can catch it (guards the fixture, not the fix).
#[test]
fn bh1119_mt_i1_fixture_set_carries_even_txid() {
    let s = even_plus_ab_strings();
    let strings: Vec<String> = s.lines().map(str::to_string).collect();
    let set = mt_codec::string_layer::pipeline::decode(&strings).unwrap();
    assert_eq!(to_hex(&set.bytes), format!("{}ab", even_hex()));
}

/// The step-4 refusal names the base64 stem every PSBT spelling shares, not
/// the 7-character prefix that only some carry.
#[test]
fn bh1119_mt_i20_unrecognised_names_the_stem() {
    let r = qr(b"hello\n");
    assert!(!r.ok, "{}", r.stderr);
    assert!(r.stderr.contains("`cHNidP`),"), "{}", r.stderr);
    assert!(!r.stderr.contains("`cHNidP8`"), "{}", r.stderr);
    assert!(r.stdout.is_empty());
}
