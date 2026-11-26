//! The constellation exit codes (mnemonic-engrave
//! `design/SPEC_constellation_exit_codes.md`), one row per class `mt` can
//! return. The per-refusal rows are in `refusals.toml` (`class`), checked by
//! `scripts/check-refusal-coverage.sh`; this file pins the classes that are
//! not a §8 refusal row, and the bits that combine.

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::exit::{INVALID, IO, NO, OK, REFUSED, REPAIRED, USAGE};

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn corpus() -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn vector(label: &str, field: &str) -> serde_json::Value {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()[field]
        .clone()
}

fn strings(label: &str) -> Vec<String> {
    vector(label, "strings")
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn tmp(body: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(body.as_bytes()).unwrap();
    f.flush().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    f
}

/// One substituted character in the first string: BCH corrects it.
fn one_error_set() -> String {
    let mut s = strings("even");
    let mut c: Vec<char> = s[0].chars().collect();
    let p = 20;
    c[p] = if c[p] == 'q' { 'p' } else { 'q' };
    s[0] = c.into_iter().collect();
    s.join("\n")
}

fn run(args: &[&str], stdin: &str) -> (Option<i32>, String, String) {
    let o = mt().args(args).write_stdin(stdin).output().unwrap();
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

// ── 64: the command line alone ──────────────────────────────────────────────

#[test]
fn a_parser_error_is_usage() {
    for argv in [
        &["--nosuch"][..],
        &["nosuchverb"],
        &[],
        &["encode", "--group-size", "x"],
    ] {
        let (code, out, _) = run(argv, "");
        assert_eq!(code, Some(USAGE), "mt {argv:?}");
        assert!(out.is_empty());
    }
}

#[test]
fn a_flag_combination_mt_checks_itself_is_usage() {
    let f = tmp(&strings("even").join("\n"));
    let p = f.path().to_str().unwrap();
    for argv in [
        &["decode", "--json", "--quiet", "--in", p][..],
        &["verify", "--quiet", "--in", p],
        &["verify", "--json", "--in", p],
        &["inspect", "--transaction", p, "--in", p],
        &["decode", "--in", p, "-"],
        &["encode", "--json", "--in", p],
    ] {
        let (code, _, err) = run(argv, "");
        assert_eq!(code, Some(USAGE), "mt {argv:?}: {err}");
    }
}

/// A bad `--separator` is a setting (output form), so 64, and judged before
/// any input is read (R2): an unreadable `--in` does not outrank it.
#[test]
fn a_bad_separator_is_usage_before_any_input_is_read() {
    let (code, _, err) = run(
        &["encode", "--separator", "x", "--in", "/nonexistent/tx.hex"],
        "",
    );
    assert_eq!(code, Some(USAGE), "{err}");
    assert!(err.contains("§1.1e"), "{err}");
}

// ── help (R11) ───────────────────────────────────────────────────────────────

#[test]
fn the_root_help_ends_with_the_legend() {
    let (code, out, _) = run(&["--help"], "");
    assert_eq!(code, Some(OK));
    let legend = "Exit status: 0 = done, every check passed. 64 = bad command line, nothing \
ran. 101 = internal error. 130 = stopped by the operator. Other codes from 128 up = killed by a \
signal. Otherwise the sum (1 to 63) of: 1 a check said no; 2 an input is invalid; 4 verify the \
result before trusting it; 8 refused (network mismatch, secret on the command line, unsafe \
output); 16 a correction was applied; 32 a file, device or helper failed.";
    let flat = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat(&out).ends_with(&flat(legend)),
        "the root help does not end with the legend:\n{out}"
    );
}

// ── 2: the input ─────────────────────────────────────────────────────────────

#[test]
fn an_invalid_or_empty_input_is_invalid_on_every_verb() {
    for verb in ["encode", "decode", "verify", "inspect"] {
        for input in ["garbage\n", ""] {
            let (code, out, err) = run(&[verb, "--bitcoin-cli", OFFLINE], input);
            assert_eq!(code, Some(INVALID), "{verb} on {input:?}: {err}");
            assert!(out.is_empty());
        }
    }
}

#[test]
fn an_uncorrectable_set_is_invalid() {
    let mut s = strings("even");
    let mut c: Vec<char> = s[0].chars().collect();
    for ch in &mut c[10..30] {
        *ch = if *ch == 'q' { 'p' } else { 'q' };
    }
    s[0] = c.into_iter().collect();
    let (code, _, err) = run(&["verify"], &s.join("\n"));
    assert_eq!(code, Some(INVALID), "{err}");
}

// ── 32: the system ───────────────────────────────────────────────────────────

#[test]
fn an_unreadable_input_file_is_io() {
    for verb in ["encode", "decode", "verify", "inspect"] {
        let (code, _, err) = run(&[verb, "--in", "/nonexistent/x"], "");
        assert_eq!(code, Some(IO), "{verb}: {err}");
    }
    let f = tmp(&strings("even").join("\n"));
    let (code, _, err) = run(
        &[
            "verify",
            "--in",
            f.path().to_str().unwrap(),
            "--transaction",
            "/nonexistent/tx.hex",
        ],
        "",
    );
    assert_eq!(code, Some(IO), "{err}");
}

#[test]
fn a_failed_stdout_write_adds_io() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let f = tmp(&strings("even").join("\n"));
    let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
        .args(["decode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(IO));

    // ...ADDED to the verb's own bits, not replacing them (R8).
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let f = tmp(&one_error_set());
    let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
        .args(["decode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(IO | REPAIRED));
}

/// R9: a `bitcoin-cli` the operator NAMED, that runs and fails, sets 32 and
/// the output still prints. An absent one is the documented offline mechanism
/// and stays 0; so does the default `bitcoin-cli` failing.
#[test]
fn a_named_bitcoin_cli_that_fails_is_io_and_an_absent_one_is_offline() {
    let dir = tempfile::tempdir().unwrap();
    let cli = dir.path().join("bitcoin-cli");
    std::fs::write(&cli, "#!/bin/sh\ncat >/dev/null\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let set = strings("even").join("\n");
    for verb in ["decode", "inspect"] {
        let (code, out, err) = run(&[verb, "--bitcoin-cli", cli.to_str().unwrap()], &set);
        assert_eq!(code, Some(IO), "{verb}: {err}");
        assert!(
            !out.is_empty() || verb == "inspect",
            "{verb} lost its output"
        );
        let (code, _, err) = run(&[verb, "--bitcoin-cli", OFFLINE], &set);
        assert_eq!(code, Some(OK), "{verb}: {err}");
    }
    let raw = vector("even", "raw_hex");
    let f = tmp(raw.as_str().unwrap());
    let (code, out, err) = run(
        &[
            "encode",
            "--bitcoin-cli",
            cli.to_str().unwrap(),
            "--in",
            f.path().to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, Some(IO), "{err}");
    assert!(out.starts_with("mt1"), "encode lost its output");
}

// ── 8: refused ───────────────────────────────────────────────────────────────

#[test]
fn bearer_material_on_argv_is_refused() {
    let s = strings("even");
    for verb in ["encode", "decode", "verify", "inspect"] {
        let (code, _, err) = run(&[verb, &s[0]], "");
        assert_eq!(code, Some(REFUSED), "{verb}: {err}");
    }
}

// ── 1: a comparison came out no ──────────────────────────────────────────────

#[test]
fn a_verify_mismatch_is_no() {
    let f = tmp(&strings("even").join("\n"));
    let other = tmp(vector("uneven", "raw_hex").as_str().unwrap());
    let (code, _, err) = run(
        &[
            "verify",
            "--in",
            f.path().to_str().unwrap(),
            "--transaction",
            other.path().to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, Some(NO), "{err}");
}

// ── 16: a correction was used (decision 5, R6) ──────────────────────────────

#[test]
fn a_correction_sets_repaired_on_every_reading_verb() {
    let set = one_error_set();
    for verb in ["decode", "verify", "inspect"] {
        let (code, out, err) = run(&[verb, "--bitcoin-cli", OFFLINE], &set);
        assert_eq!(code, Some(REPAIRED), "{verb}: {err}");
        if verb == "decode" {
            assert_eq!(
                out.trim(),
                vector("even", "raw_hex").as_str().unwrap(),
                "decode prints the same hex"
            );
        }
    }
    // And --quiet / --json do not hide it.
    let (code, _, _) = run(&["decode", "--quiet", "--bitcoin-cli", OFFLINE], &set);
    assert_eq!(code, Some(REPAIRED));
    let (code, _, _) = run(&["inspect", "--json", "--bitcoin-cli", OFFLINE], &set);
    assert_eq!(code, Some(REPAIRED));
}

#[test]
fn a_clean_set_stays_ok() {
    let set = strings("even").join("\n");
    for verb in ["decode", "verify", "inspect"] {
        let (code, _, err) = run(&[verb, "--bitcoin-cli", OFFLINE], &set);
        assert_eq!(code, Some(OK), "{verb}: {err}");
    }
}

/// R1: a mismatch found with a corrected value reports both.
#[test]
fn a_mismatch_on_a_corrected_set_is_no_plus_repaired() {
    let f = tmp(&one_error_set());
    let other = tmp(vector("uneven", "raw_hex").as_str().unwrap());
    let (code, _, err) = run(
        &[
            "verify",
            "--in",
            f.path().to_str().unwrap(),
            "--transaction",
            other.path().to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, Some(NO | REPAIRED), "{err}");
}
