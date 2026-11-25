//! FIX-G (bughunt-1119 FIX table row 38): verdict and notice integrity on
//! stderr. Keys bh-mt-I10 (decode --quiet drops warnings and notices) and
//! bh-mt-I23 (verify prints OK before its --transaction comparison can fail).
//! Ruling T21 (architect-bughunt-1119-questions.md:59).
//!
//! Each test asserts the FIXED behaviour. Rows marked CONTROL pass on the base
//! and pin what the fix must not change. Spec: design/FIX_bh1119_mt_verdict_notice.md.

mod common;

use assert_cmd::Command;
use std::io::Write;

const OFFLINE: &str = "/nonexistent/bitcoin-cli";
const NO_NODE_HEAD: &str = "\nWARNING: no bitcoind reachable. mt read this transaction from the\n";
const OK_EVEN: &str = "mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.";
const MATCH_LINE: &str = "  --transaction matches, on the full txid.";

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

fn vector(label: &str) -> serde_json::Value {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()
        .clone()
}

fn strings_of(label: &str) -> Vec<String> {
    vector(label)["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

fn raw_of(label: &str) -> String {
    vector(label)["raw_hex"].as_str().unwrap().to_string()
}

fn tmp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Swap one payload character for another symbol (`q` <-> `p`).
fn damage(st: &str, positions: &[usize]) -> String {
    let mut c: Vec<char> = st.chars().collect();
    for &p in positions {
        c[p] = if c[p] == 'q' { 'p' } else { 'q' };
    }
    c.into_iter().collect()
}

/// even, string 1 payload offset 40 damaged: one in-budget BCH correction.
fn corrected_even() -> String {
    let mut st = strings_of("even");
    st[0] = damage(&st[0], &[40]);
    st.join("\n") + "\n"
}

/// even, the first two genuine `6`s of string 1 typed as `b` (from offset 3).
fn transliterated_even() -> String {
    let mut st = strings_of("even");
    let mut c: Vec<char> = st[0].chars().collect();
    let mut n = 0;
    for ch in c.iter_mut().skip(3) {
        if n == 2 {
            break;
        }
        if *ch == '6' {
            *ch = 'b';
            n += 1;
        }
    }
    assert_eq!(n, 2, "fixture: string 1 holds fewer than two `6`s");
    st[0] = c.into_iter().collect();
    st.join("\n") + "\n"
}

/// even plus a second copy of chunk 3 with one damaged character.
fn duplicate_even() -> String {
    let mut st = strings_of("even");
    let extra = damage(&st[2], &[45]);
    st.push(extra);
    st.join("\n") + "\n"
}

/// even plus a seventh string damaged past the budget (six characters).
fn unreadable_spare_even() -> String {
    let mut st = strings_of("even");
    let extra = damage(&st[4], &[20, 30, 40, 50, 60, 70]);
    st.push(extra);
    st.join("\n") + "\n"
}

fn decode(extra: &[&str], text: &str, cli: &std::path::Path) -> std::process::Output {
    let f = tmp(text);
    mt().arg("decode")
        .arg("--bitcoin-cli")
        .arg(cli)
        .args(extra)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap()
}

fn decode_offline(extra: &[&str], text: &str) -> std::process::Output {
    decode(extra, text, std::path::Path::new(OFFLINE))
}

fn verify(text: &str, transaction: &std::path::Path) -> std::process::Output {
    let f = tmp(text);
    mt().arg("verify")
        .arg("--in")
        .arg(f.path())
        .arg("--transaction")
        .arg(transaction)
        .output()
        .unwrap()
}

/// The quiet run's stderr must be the loud run's stderr from the blank line
/// before the no-node warning to the end: the same bytes, the report removed.
fn assert_quiet_is_loud_after_report(text: &str) {
    let loud = decode_offline(&[], text);
    let quiet = decode_offline(&["--quiet"], text);
    assert_eq!(loud.status.code(), Some(0), "{}", s(&loud.stderr));
    assert_eq!(quiet.status.code(), Some(0), "{}", s(&quiet.stderr));
    assert_eq!(loud.stdout, quiet.stdout, "--quiet changed stdout");
    let l = s(&loud.stderr);
    let at = l
        .find(NO_NODE_HEAD)
        .unwrap_or_else(|| panic!("loud stderr carries no no-node warning:\n{l}"));
    assert_eq!(
        s(&quiet.stderr),
        &l[at..],
        "--quiet stderr is not the loud stderr minus the report"
    );
}

// ---- bh-mt-I10: decode --quiet keeps every warning and notice ----

/// Row 1.
#[test]
fn bh1119_mt_g_quiet_keeps_the_correction_notice() {
    let o = decode_offline(&["--quiet"], &corrected_even());
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let e = s(&o.stderr);
    assert!(
        e.contains("\nCORRECTION APPLIED. 1 chunk needed repair:\n"),
        "--quiet dropped the correction notice:\n[{e}]"
    );
}

/// Row 2.
#[test]
fn bh1119_mt_g_quiet_keeps_the_no_node_warning() {
    let o = decode_offline(&["--quiet"], &(strings_of("even").join("\n") + "\n"));
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let e = s(&o.stderr);
    assert!(
        e.starts_with(NO_NODE_HEAD),
        "--quiet dropped the no-node warning:\n[{e}]"
    );
}

/// Row 3.
#[test]
fn bh1119_mt_g_quiet_keeps_the_transliteration_notice() {
    let o = decode_offline(&["--quiet"], &transliterated_even());
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let e = s(&o.stderr);
    assert!(
        e.contains("\nCHARACTERS mt READ DIFFERENTLY.")
            && e.contains("you typed b, mt read it as 6"),
        "--quiet dropped the transliteration notice:\n[{e}]"
    );
}

/// Row 4.
#[test]
fn bh1119_mt_g_quiet_keeps_the_duplicate_notice() {
    let o = decode_offline(&["--quiet"], &duplicate_even());
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let e = s(&o.stderr);
    assert!(
        e.contains("\nDUPLICATE RESOLVED. chunk 3 was present twice.\n"),
        "--quiet dropped the duplicate notice:\n[{e}]"
    );
}

/// Row 5.
#[test]
fn bh1119_mt_g_quiet_keeps_the_unreadable_string_notice() {
    let o = decode_offline(&["--quiet"], &unreadable_spare_even());
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let e = s(&o.stderr);
    assert!(
        e.contains("\nUNREADABLE STRING. string 7 of the input could not be read:\n"),
        "--quiet dropped the unreadable-string notice:\n[{e}]"
    );
}

/// Row 6: no new text, no reordering, and no report row leaks.
#[test]
fn bh1119_mt_g_quiet_stderr_is_the_loud_stderr_without_the_report() {
    let clean = strings_of("even").join("\n") + "\n";
    for text in [
        clean,
        corrected_even(),
        transliterated_even(),
        duplicate_even(),
        unreadable_spare_even(),
    ] {
        assert_quiet_is_loud_after_report(&text);
    }
}

/// Row 7, CONTROL: with a node reachable there is no no-node warning, and a
/// clean set under --quiet still writes nothing to stderr.
#[test]
fn bh1119_mt_g_quiet_with_a_node_stays_silent_on_a_clean_set() {
    let stub = common::node_stub(r#"{"value": 50.00000000, "scriptPubKey": {}}"#, &[]);
    let o = decode(
        &["--quiet"],
        &(strings_of("even").join("\n") + "\n"),
        stub.path(),
    );
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    assert_eq!(
        s(&o.stderr),
        "",
        "a clean set with a node reachable must stay silent"
    );
    assert_eq!(s(&o.stdout).trim_end(), raw_of("even"));
}

/// Row 8, CONTROL: stdout is the transaction hex, unchanged by --quiet.
#[test]
fn bh1119_mt_g_quiet_stdout_is_the_hex() {
    let o = decode_offline(&["--quiet"], &corrected_even());
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    assert_eq!(s(&o.stdout), format!("{}\n", raw_of("even")));
}

// ---- bh-mt-I23: verify prints OK only after --transaction compares ----

/// Row 9.
#[test]
fn bh1119_mt_g_verify_mismatch_prints_no_ok() {
    let other = tmp(&raw_of("uneven"));
    let o = verify(&(strings_of("even").join("\n") + "\n"), other.path());
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(
        !e.contains("mt verify: OK"),
        "OK printed before REFUSED:\n{e}"
    );
    assert_eq!(
        e.lines().next().unwrap_or(""),
        "mt verify: REFUSED — §1.1, the supplied transaction is not the one on these strings"
    );
}

/// Row 10.
#[test]
fn bh1119_mt_g_verify_missing_transaction_prints_no_ok() {
    let o = verify(
        &(strings_of("even").join("\n") + "\n"),
        std::path::Path::new("/nonexistent/x.hex"),
    );
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(
        !e.contains("mt verify: OK"),
        "OK printed before REFUSED:\n{e}"
    );
    assert_eq!(
        e.lines().next().unwrap_or(""),
        "mt verify: REFUSED — §1.1, cannot read /nonexistent/x.hex"
    );
}

/// Row 11.
#[test]
fn bh1119_mt_g_verify_unreadable_transaction_prints_no_ok() {
    let junk = tmp("this is not a transaction\n");
    let o = verify(&(strings_of("even").join("\n") + "\n"), junk.path());
    let e = s(&o.stderr);
    assert_ne!(o.status.code(), Some(0), "{e}");
    assert!(
        !e.contains("mt verify: OK"),
        "OK printed before REFUSED:\n{e}"
    );
    assert!(
        e.lines()
            .next()
            .unwrap_or("")
            .starts_with("mt verify: REFUSED — "),
        "the first stderr line must be the refusal:\n{e}"
    );
}

/// Row 12: a refused comparison still prints the notices (warnings are never
/// suppressed), before the refusal, and never the OK line.
#[test]
fn bh1119_mt_g_verify_refusal_keeps_the_notices() {
    let other = tmp(&raw_of("uneven"));
    let o = verify(&corrected_even(), other.path());
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(
        !e.contains("mt verify: OK"),
        "OK printed before REFUSED:\n{e}"
    );
    let c = e
        .find("\nCORRECTION APPLIED. 1 chunk needed repair:\n")
        .unwrap_or_else(|| panic!("the refused run dropped the correction notice:\n{e}"));
    let r = e
        .find("mt verify: REFUSED — §1.1, the supplied transaction is not the one on these strings")
        .unwrap_or_else(|| panic!("no refusal:\n{e}"));
    assert!(c < r, "the notices must come before the refusal:\n{e}");
}

/// Row 13, CONTROL: a matching transaction prints exactly one OK, first, and
/// the match line last.
#[test]
fn bh1119_mt_g_verify_match_prints_one_ok_then_the_match_line() {
    let right = tmp(&raw_of("even"));
    let o = verify(&(strings_of("even").join("\n") + "\n"), right.path());
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{e}");
    assert_eq!(e, format!("{OK_EVEN}\n{MATCH_LINE}\n"));
}

/// Row 14, CONTROL: on success the order stays OK, notices, match line.
#[test]
fn bh1119_mt_g_verify_match_keeps_ok_notices_match_order() {
    let right = tmp(&raw_of("even"));
    let o = verify(&corrected_even(), right.path());
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{e}");
    assert_eq!(e.matches("mt verify: OK").count(), 1, "{e}");
    assert!(e.starts_with(&format!("{OK_EVEN}\n")), "{e}");
    let c = e
        .find("\nCORRECTION APPLIED. 1 chunk needed repair:\n")
        .expect("notice");
    let m = e.find(MATCH_LINE).expect("match line");
    assert!(c < m, "{e}");
    assert!(e.ends_with(&format!("{MATCH_LINE}\n")), "{e}");
}

/// Row 15: with a node reachable, --quiet still prints the notices, and no
/// no-node warning.
#[test]
fn bh1119_mt_g_quiet_with_a_node_keeps_the_notices() {
    let stub = common::node_stub(r#"{"value": 50.00000000, "scriptPubKey": {}}"#, &[]);
    let o = decode(&["--quiet"], &corrected_even(), stub.path());
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let e = s(&o.stderr);
    assert!(
        e.starts_with("\nCORRECTION APPLIED. 1 chunk needed repair:\n"),
        "--quiet with a node dropped the correction notice:\n[{e}]"
    );
    assert!(!e.contains("WARNING: no bitcoind reachable."), "{e}");
}

#[test]
fn bh1119_mt_g_verify_refusal_keeps_the_set_notice() {
    let other = tmp(&raw_of("uneven"));
    let o = verify(&duplicate_even(), other.path());
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{e}");
    let d = e
        .find("DUPLICATE RESOLVED.")
        .unwrap_or_else(|| panic!("dropped the duplicate notice:\n{e}"));
    let r = e
        .find("mt verify: REFUSED")
        .unwrap_or_else(|| panic!("no refusal:\n{e}"));
    assert!(d < r, "{e}");
}

#[test]
fn bh1119_mt_g_verify_without_transaction_prints_no_match_line() {
    let f = tmp(&(strings_of("even").join("\n") + "\n"));
    let o = mt()
        .arg("verify")
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    let e = s(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{e}");
    assert!(e.starts_with(OK_EVEN), "{e}");
    assert!(
        !e.contains(MATCH_LINE),
        "claims a comparison that never ran:\n{e}"
    );
}
