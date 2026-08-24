//! `mt decode` and `mt verify`.
//!
//! The recovery path. Most of these assert on things that only matter years
//! later, to someone holding steel and no context — which is why they are worth
//! more than they look.

use assert_cmd::Command;
use std::io::Write;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

fn corpus() -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn tmp_with(s: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(s.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn strings_of(label: &str) -> Vec<String> {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn raw_of(label: &str) -> String {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["raw_hex"]
        .as_str()
        .unwrap()
        .to_string()
}

/// P3's gate: every pinned vector round-trips through `decode`.
#[test]
fn decode_round_trips_every_vector() {
    for label in ["even", "uneven"] {
        let f = tmp_with(&strings_of(label).join("\n"));
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{label}: decode failed");
        assert_eq!(
            String::from_utf8(out.stdout).unwrap().trim(),
            raw_of(label),
            "{label}: decode did not reproduce the transaction"
        );
    }
}

/// §1.1e's split-then-strip. An operator copying several strings out of a
/// terminal produces one line, and "strip whitespace before doing anything else"
/// taken literally makes that unparseable.
#[test]
fn decode_accepts_the_shapes_an_operator_actually_produces() {
    let s = strings_of("even");
    let grouped: Vec<String> = s
        .iter()
        .map(|x| {
            x.chars()
                .collect::<Vec<_>>()
                .chunks(8)
                .map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();

    for (name, body) in [
        ("one per line", s.join("\n")),
        ("single-line blob", s.concat()),
        ("grouped", grouped.join("\n")),
        ("uppercase", s.join("\n").to_uppercase()),
        ("blank lines between", s.join("\n\n")),
        ("trailing whitespace", format!("{}\n  \n", s.join("\n"))),
    ] {
        let f = tmp_with(&body);
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "decode rejected {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap().trim(),
            raw_of("even"),
            "{name}: wrong bytes"
        );
    }
}

/// §1.1a takes strings "in any order".
#[test]
fn decode_is_order_independent() {
    let mut s = strings_of("even");
    s.reverse();
    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

/// The elided form round-trips with no flag on the reading side: detection is
/// unambiguous, because a line beginning `mt1` is full and anything else is not.
#[test]
fn decode_restores_elided_input_without_a_flag() {
    let s = strings_of("even");
    let drop = 3 + 8;
    let mut lines = vec![s[0].clone()];
    lines.extend(s[1..].iter().map(|x| x[drop..].to_string()));

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

/// Mixed full/elided is legal — an operator who elides "after a while" produces
/// exactly that.
#[test]
fn decode_accepts_mixed_full_and_elided() {
    let s = strings_of("even");
    let drop = 3 + 8;
    let mut lines = vec![s[0].clone(), s[1].clone()];
    lines.extend(s[2..].iter().map(|x| x[drop..].to_string()));

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

/// All-elided is refused, naming the shape of what is missing rather than
/// guessing.
#[test]
fn decode_refuses_all_elided_input() {
    let drop = 3 + 8;
    let lines: Vec<String> = strings_of("even")
        .iter()
        .map(|x| x[drop..].to_string())
        .collect();
    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();

    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("REFUSED — §3b"), "got: {err}");
    assert!(
        err.contains("8 characters"),
        "must name what is needed: {err}"
    );
    assert!(out.stdout.is_empty(), "stdout must stay empty on a refusal");
}

/// **stdout stays empty unless every check passes.** The documented pipeline
/// pipes stdout onward; a failure path that still printed hex would let it
/// broadcast a transaction that failed `mt`'s own checks.
#[test]
fn decode_writes_nothing_to_stdout_on_failure() {
    for body in ["not strings at all", "mt1qqqq", ""] {
        let f = tmp_with(body);
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(!out.status.success(), "{body:?} should not succeed");
        assert!(
            out.stdout.is_empty(),
            "{body:?}: stdout carried bytes on a failed decode"
        );
    }
}

/// `decode` prints the report on **stderr**, because it is the verb a recoverer
/// reaches for first — `inspect` is the one designed for them and they have no
/// way to know that. It must not pollute the pipe.
#[test]
fn decode_reports_on_stderr_and_hex_on_stdout() {
    let f = tmp_with(&strings_of("even").join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();

    assert!(stderr.contains("TX  "), "the report did not reach stderr");
    assert!(stderr.contains("mt1 SET"), "the set line is missing");
    assert_eq!(stdout.lines().count(), 1, "stdout must be exactly the hex");
    assert!(stdout.trim().chars().all(|c| c.is_ascii_hexdigit()));
}

// ── verify ───────────────────────────────────────────────────────────────────

#[test]
fn verify_reports_ok_on_a_clean_set() {
    let f = tmp_with(&strings_of("even").join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("mt verify: OK"), "got: {err}");
    assert!(err.contains("6 chunks"), "got: {err}");
}

/// **The margin report** — the Critical the journey walk found. A chunk repaired
/// four times passes as OK while sitting one scratch from unrecoverable, and a
/// verdict that hides that tells the operator the opposite of what they need.
#[test]
fn verify_reports_its_margin_not_just_a_verdict() {
    let mut s = strings_of("even");
    // damage one character in the first string's payload
    let mut chars: Vec<char> = s[0].chars().collect();
    let at = 40;
    chars[at] = if chars[at] == 'q' { 'p' } else { 'q' };
    s[0] = chars.into_iter().collect();

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "one damaged symbol must still verify OK"
    );

    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("CORRECTION APPLIED"),
        "no margin report: {err}"
    );
    assert!(
        err.contains("1 of 4 symbols"),
        "margin not quantified: {err}"
    );
    assert!(err.contains("pos "), "corrections not LOCALISED: {err}");
    assert!(
        err.contains("chunk   1"),
        "chunk numbering must be 1-based in human output: {err}"
    );
}

/// `--transaction` compares the FULL txid. A 20-bit compare would report a match
/// for any transaction sharing those bits.
#[test]
fn verify_transaction_matches_and_rejects() {
    let f = tmp_with(&strings_of("even").join("\n"));

    let right = tmp_with(&raw_of("even"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .arg("--transaction")
        .arg(right.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8(out.stderr).unwrap().contains("full txid"));

    let wrong = tmp_with(&raw_of("uneven"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .arg("--transaction")
        .arg(wrong.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a different transaction must be rejected"
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("not the one on these strings"), "got: {err}");
    assert!(
        err.contains("FULL txid"),
        "the refusal must say the comparison was on the full txid: {err}"
    );
}

/// §1.1: `verify` never asks a node, so it runs on an air-gapped machine. Forcing
/// the offline path must change nothing.
#[test]
fn verify_never_needs_a_node() {
    let f = tmp_with(&strings_of("even").join("\n"));
    let with = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let without = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert_eq!(
        with.status.success(),
        without.status.success(),
        "verify behaved differently with no bitcoin-cli on PATH"
    );
    assert_eq!(
        with.stderr, without.stderr,
        "verify's output depended on a node"
    );
}
