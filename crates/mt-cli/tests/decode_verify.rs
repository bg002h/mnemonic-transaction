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

// ── the drawer: duplicates, and a plate that is scrap ────────────────────────

/// Damage `n` characters of a string, at spread-out positions.
fn damage(s: &str, positions: &[usize]) -> String {
    let mut c: Vec<char> = s.chars().collect();
    for &p in positions {
        c[p] = if c[p] == 'q' { 'p' } else { 'q' };
    }
    c.into_iter().collect()
}

/// **§1.8's advice produces duplicates, so duplicates are the expected state of
/// a well-kept drawer** — not an anomaly. What matters is which copy `mt` uses
/// and whether it says so: the point of cutting a second plate is that the
/// better one gets used, and "first one wins" would report the margin of
/// whichever the operator happened to type first.
#[test]
fn a_duplicate_keeps_the_healthier_copy_and_says_which_it_dropped() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines.push(damage(&s[2], &[45])); // chunk 3, one wrong character

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "a duplicate must not fail the set");
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(err.contains("DUPLICATE RESOLVED. chunk 3"), "got: {err}");
    assert!(
        err.contains("KEPT       the copy needing 0 of 4"),
        "the CLEAN copy must win, not the first-typed one: {err}"
    );
    assert!(
        err.contains("DISCARDED  the copy needing 1 of 4"),
        "the discarded copy's margin is what tells the operator to re-cut: {err}"
    );
    // The kept copy is clean, so there is nothing to report a correction for.
    assert!(
        !err.contains("CORRECTION APPLIED"),
        "the margin report described a copy mt did not use: {err}"
    );
}

/// The reverse order. If "first one wins" were the rule this would keep the
/// damaged copy and the test above would still pass — so both orders are
/// asserted, and only the pair proves the rule.
#[test]
fn the_healthier_copy_wins_whichever_arrives_first() {
    let s = strings_of("even");
    let mut lines = vec![damage(&s[2], &[45])];
    lines.extend(s.iter().cloned());

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("KEPT       the copy needing 0 of 4"),
        "the damaged copy won because it was typed first: {err}"
    );
}

/// **A miscut plate must not kill a set that has a good copy of that chunk.**
/// §1.8's advice is to cut a second copy, and this is the drawer that followed
/// it: one string damaged past `t = 4`, one clean, both typed back. Failing on
/// the first unreadable string would refuse a completely recoverable set —
/// while holding the good copy.
#[test]
fn a_plate_damaged_past_the_budget_does_not_kill_a_recoverable_set() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines.push(damage(&s[4], &[20, 30, 40, 50, 60, 70])); // six: past t = 4

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an unreadable EXTRA string failed a set that is complete:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("UNREADABLE STRING"), "silently ignored: {err}");
    assert!(
        err.contains("string 7"),
        "the plate is not identified: {err}"
    );
    assert!(
        err.contains("that plate is scrap"),
        "no action named: {err}"
    );
}

/// The other half of the rule: tolerating an unreadable string must NOT tolerate
/// a MISSING chunk. Without this, the change above would turn a set that cannot
/// be recovered into one that reports OK.
#[test]
fn an_unreadable_string_with_no_other_copy_still_fails() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines[4] = damage(&s[4], &[20, 30, 40, 50, 60, 70]); // REPLACED, not added

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a set missing chunk 5 entirely reported success"
    );
}

/// Garbage in must still say what is wrong with the input, not report a missing
/// plate — the operator would go looking in a drawer for something that was
/// never the problem.
#[test]
fn unreadable_input_reports_the_read_error_not_a_missing_chunk() {
    let f = tmp_with("mt1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq");
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        !err.contains("missing") || err.contains("checksum") || err.contains("BCH"),
        "a garbage input was reported as a missing plate: {err}"
    );
}

/// **The margin report gives the BEFORE-value, not only the position.**
///
/// A bare position tells the operator to go and look; a before-and-after tells
/// them what to look for — and it is the only way to distinguish a MIS-CUT plate
/// from a MIS-READ one. If the steel really says the corrected character, the
/// plate is fine and the typist slipped.
#[test]
fn the_margin_report_gives_before_and_after_values() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines[0] = damage(&s[0], &[40, 55]);

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(err.contains("2 of 4 symbols"), "{err}");
    // Both corrections, each with what was read and what replaced it.
    let reads = err.matches("read ").count();
    assert_eq!(reads, 2, "expected one before-value per correction: {err}");
    assert_eq!(err.matches("corrected to ").count(), 2, "{err}");
    // The characters actually on the plate. `damage` wrote 'q' or 'p'.
    for (pos, was) in [(41usize, 'q'), (56, 'q')] {
        assert!(
            err.contains(&format!("pos {pos:>3}   read {was}, corrected to ")),
            "position {pos} is missing its before-value: {err}"
        );
    }
}
