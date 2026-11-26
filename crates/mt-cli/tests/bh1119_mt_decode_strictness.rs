//! FIX-I, mt-codec decode strictness, CLI rows (bughunt-1119 bh-mt-I18).
//! The `even` corpus vector re-cut 38/36/37/37/37/37 under its own set id:
//! BCH-valid strings, a set no conforming encoder writes. Each row asserts the
//! CORRECT behaviour and fails on unmodified code.

use assert_cmd::Command;
use mt_codec::string_layer::bch::{bytes_to_5bit, encode_5bit_to_string};
use mt_codec::{ChunkHeader, decode};
use std::io::Write;

const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

fn chunk(set: u32, count: usize, index: usize, payload: &[u8]) -> String {
    let mut syms: Vec<u8> = ChunkHeader::new(set, count, index)
        .unwrap()
        .to_symbols()
        .to_vec();
    syms.extend(bytes_to_5bit(payload));
    encode_5bit_to_string(&syms).unwrap()
}

/// The re-cut set, in index order.
fn recut() -> Vec<String> {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
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
    let (bytes, id) = (set.bytes, set.chunks[0].header.chunk_set_id);
    let sizes = [38usize, 36, 37, 37, 37, 37];
    let mut at = 0;
    sizes
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            let s = chunk(id, sizes.len(), i, &bytes[at..at + n]);
            at += n;
            s
        })
        .collect()
}

fn run(verb: &str, strings: &[String]) -> (Option<i32>, String, String) {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(strings.join("\n").as_bytes()).unwrap();
    f.write_all(b"\n").unwrap();
    f.flush().unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let o = mt()
        .arg(verb)
        .args(["--bitcoin-cli", OFFLINE])
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Whitespace-collapsed, so a wrapped mechanism line still matches.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

const DETAIL: &str = "chunk 1 of 6 has payload length 38; §3b's balanced split of a 222-byte payload gives count 6, bytes per chunk 37, last chunk 37";

fn assert_refused(verb: &str, strings: &[String]) {
    let (code, stdout, stderr) = run(verb, strings);
    let head = format!("mt {verb}: REFUSED — §1.1, the set does not verify");
    assert!(
        code == Some(2) // INVALID: the input is wrong for the verb
            && stdout.is_empty()
            && stderr.starts_with(&head)
            && flat(&stderr).contains(DETAIL),
        "exit {code:?}\n--- stdout\n{stdout}\n--- stderr\n{stderr}"
    );
}

/// C1: `mt decode`, strings in index order.
#[test]
fn c1_decode_refuses_recut_set_in_index_order() {
    assert_refused("decode", &recut());
}

/// C2: `mt decode`, strings reversed.
#[test]
fn c2_decode_refuses_recut_set_reversed() {
    let mut s = recut();
    s.reverse();
    assert_refused("decode", &s);
}

/// C3: `mt verify`, strings in index order.
#[test]
fn c3_verify_refuses_recut_set() {
    assert_refused("verify", &recut());
}

/// C4: `mt inspect`, strings in index order.
#[test]
fn c4_inspect_refuses_recut_set() {
    assert_refused("inspect", &recut());
}
