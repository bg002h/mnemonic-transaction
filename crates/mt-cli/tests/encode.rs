//! `mt encode`, end to end.
//!
//! The split under test throughout: **stdout carries the artifact, stderr
//! carries everything a human must see.** Several of these assert on which
//! stream something landed in, because that boundary is what makes the output
//! pipeable — and the first downstream consumer that has to parse prose out of
//! its input is the one that engraves a warning label as a chunk.

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

fn tmp_with(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

/// P2's gate: `mt encode` reproduces every pinned string exactly.
///
/// The vectors came from an independent generator, so this is the CLI checked
/// against bytes neither it nor `mt-codec` produced.
#[test]
fn reproduces_every_pinned_vector() {
    for v in corpus()["vectors"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
        let out = mt()
            .args(["encode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{label}: encode failed");

        let got: Vec<String> = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        let want: Vec<String> = v["strings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert_eq!(got, want, "{label}: stdout differs from the pinned vector");
    }
}

/// §0a: stdout is the strings and NOTHING else. No banner, no legend, no blank
/// separator — the moment prose shares that stream, every consumer has to strip it.
#[test]
fn stdout_carries_only_strings() {
    let v = &corpus()["vectors"][0];
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let out = mt()
        .args(["encode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    for line in stdout.lines() {
        assert!(
            line.starts_with("mt1"),
            "stdout carried something that is not a string: {line:?}"
        );
        assert_eq!(
            line,
            line.to_lowercase(),
            "stdout must be lowercase (§1.1e); uppercase is engraving advice, \
             not the byte stream"
        );
        assert!(!line.contains(' '), "stdout must be ungrouped by default");
    }
    assert!(!stdout.contains("WARNING"), "a warning reached stdout");
}

/// The three mandatory blocks, all on stderr, all present without being asked for.
#[test]
fn the_three_mandatory_blocks_are_printed() {
    let v = &corpus()["vectors"][0];
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let out = mt()
        .args(["encode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(
        err.contains("can broadcast this transaction"),
        "BEARER block missing"
    );
    // The rendered text is wrapped, so match on a phrase that cannot straddle
    // a break rather than on one that can.
    assert!(
        err.contains("WITNESS SHAPE"),
        "BEARER block does not qualify the check"
    );
    assert!(
        err.contains("CAN defeat it"),
        "BEARER block omits that the check is defeatable"
    );
    assert!(
        err.contains("4 wrong CHARACTERS per string"),
        "correction block missing"
    );
    assert!(
        err.contains("Count each string"),
        "correction block omits the count check"
    );
    assert!(
        err.contains("verify the ENGRAVING"),
        "verify-the-steel block missing"
    );
}

/// `--quiet` suppresses the inspection report ONLY. Warnings and refusals are
/// never suppressed, on any verb — a `--quiet` that silenced §8's warnings would
/// let a script engrave a plate whose hazards nobody saw.
#[test]
fn quiet_suppresses_the_report_but_never_the_warnings() {
    let v = &corpus()["vectors"][0];
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let out = mt()
        .args(["encode", "--quiet", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(
        !err.contains("\nCUT "),
        "--quiet did not suppress the report"
    );
    assert!(
        !err.contains("\nPREFIX "),
        "--quiet did not suppress the report"
    );
    assert!(
        err.matches("WARNING:").count() >= 3,
        "--quiet suppressed warnings, which it must never do"
    );
}

/// `--elide-prefix`: first string full, the rest carrying index + payload only.
/// The output stays self-describing, so `decode` needs no flag of its own.
#[test]
fn elide_prefix_drops_exactly_eleven_characters_after_the_first() {
    let v = &corpus()["vectors"][0];
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let out = mt()
        .args(["encode", "--elide-prefix", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let lines: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    let full: Vec<usize> = v["string_lengths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap() as usize)
        .collect();

    assert!(
        lines[0].starts_with("mt1"),
        "the first string must stay full"
    );
    assert_eq!(lines[0].chars().count(), full[0]);
    for (i, line) in lines.iter().enumerate().skip(1) {
        assert!(!line.starts_with("mt1"), "string {} was not elided", i + 1);
        assert_eq!(
            line.chars().count(),
            full[i] - 11,
            "elision must drop `mt1` (3) plus the invariant prefix (8) = 11"
        );
    }
}

#[test]
fn group_size_only_affects_stdout_rendering() {
    let v = &corpus()["vectors"][0];
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let out = mt()
        .args(["encode", "--group-size", "8", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let first = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert!(first.contains(' '), "grouping did not apply");
    assert_eq!(
        first.replace(' ', ""),
        v["strings"][0].as_str().unwrap(),
        "grouping changed the string rather than only its rendering"
    );
}

// ── §8.2e's sniffing procedure, over the fixture corpus ──────────────────────
//
// P2 asserts the SNIFFING fixtures only. The refusal fixtures belong to P5,
// because at P2 the refusals do not exist yet — a gate asserting "accepted or
// refused" over the whole corpus would pass because everything is accepted, and
// then start failing when P5 lands.

fn raw_hex() -> String {
    corpus()["vectors"][0]["raw_hex"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn accepts_hex_in_every_plausible_shape() {
    let h = raw_hex();
    for (name, body) in [
        ("plain", h.clone()),
        ("uppercase", h.to_uppercase()),
        ("0x-prefixed", format!("0x{h}")),
        ("trailing newline", format!("{h}\n")),
        ("CRLF", format!("{h}\r\n")),
        ("leading whitespace", format!("  \n{h}")),
        ("line-wrapped", {
            let mut w = String::new();
            for c in h.as_bytes().chunks(64) {
                w.push_str(std::str::from_utf8(c).unwrap());
                w.push('\n');
            }
            w
        }),
    ] {
        let f = tmp_with(body.as_bytes());
        let out = mt()
            .args(["encode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "sniffing rejected {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// The one genuinely ambiguous input: valid hex AND a PSBT. The refusal must
/// name the real problem, because "invalid transaction" sends the operator to
/// look at the wrong thing.
#[test]
fn refuses_a_hex_encoded_psbt_by_name() {
    let hex_psbt = "70736274ff0100";
    let f = tmp_with(hex_psbt.as_bytes());
    let out = mt()
        .args(["encode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.starts_with("mt encode: REFUSED — §8.2e,"), "got: {err}");
    assert!(
        err.contains("hex-encoded PSBT"),
        "refusal does not name the real problem: {err}"
    );
}

/// Unrecognised input names what was seen — never a bare "invalid input", which
/// tells an operator nothing they can act on.
#[test]
fn unrecognised_input_shows_the_bytes() {
    let f = tmp_with(b"this is not a transaction at all");
    let out = mt()
        .args(["encode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("REFUSED — §8.2e"), "got: {err}");
    assert!(
        err.contains("74 68 69 73"),
        "refusal does not show the bytes: {err}"
    );
}

/// `--bitcoin-cli /nonexistent` is the offline mechanism every gate and journey
/// needing an air-gapped run uses. It must not crash — the alternative an
/// implementer reaches for is editing `PATH`, which is process-global and would
/// silently change neighbouring tests in the same run.
#[test]
fn a_missing_bitcoin_cli_does_not_crash() {
    let v = &corpus()["vectors"][0];
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let out = mt()
        .args([
            "encode",
            "--bitcoin-cli",
            "/nonexistent/bitcoin-cli",
            "--in",
        ])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an absent bitcoin-cli must be a warning, not a failure: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// §8.2f: a transaction passed as a command-line argument lands in shell history
/// and in `ps` for every user on the machine. `mt` reads from a file or stdin.
#[test]
fn there_is_no_way_to_pass_a_transaction_as_an_argument() {
    let out = mt().args(["encode", &raw_hex()]).output().unwrap();
    assert!(
        !out.status.success(),
        "a positional transaction argument was accepted — §8.2f refuses this"
    );
}
