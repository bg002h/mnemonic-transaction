//! Review rows for bughunt-1119 FIX-C (review-mt20-s1, I1). A separate file
//! because `bh1119_mt_input_parsing.rs` is installed unchanged from the
//! sha256-pinned FIX sidecar.
//!
//! Every run is offline (`--bitcoin-cli /nonexistent/bitcoin-cli`).

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

/// The corpus `even` transaction, 222 bytes, as hex.
fn even_hex() -> String {
    vector("even")["raw_hex"].as_str().unwrap().to_string()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn file(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

// ── review-mt20-s1 I1: the content id is checked before the end of the bytes ─

/// A mismatched content id AND trailing bytes: the content-id refusal comes
/// first (FIX section 3 R3), not the E-SET trailing-bytes refusal.
#[test]
fn bh1119_mt_i1_set_with_wrong_id_and_trailing_byte_reports_the_id() {
    let mut bytes = hex(&even_hex());
    bytes.push(0xab);
    let other =
        bitcoin::consensus::deserialize::<bitcoin::Transaction>(&hex(&p5("raw_hex"))).unwrap();
    let strings: String =
        mt_codec::string_layer::pipeline::encode(&bytes, &other.compute_txid().to_string())
            .unwrap()
            .iter()
            .map(|s| s.clone() + "\n")
            .collect();
    let f = file(strings.as_bytes());
    let out = mt()
        .args(["decode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "the run succeeded:\n{stderr}");
    assert!(
        stderr.contains("the transaction re-derives"),
        "the content-id refusal is missing:\n{stderr}"
    );
    assert!(
        !stderr.contains("continue for"),
        "E-SET shadowed the id check:\n{stderr}"
    );
}
