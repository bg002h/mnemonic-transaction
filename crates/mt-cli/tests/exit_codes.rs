//! The constellation exit codes (mnemonic-engrave
//! `design/SPEC_constellation_exit_codes.md`), one row per class `mt` can
//! return. The per-refusal rows are in `refusals.toml` (`class`), checked by
//! `scripts/check-refusal-coverage.sh`; this file pins the classes that are
//! not a §8 refusal row, and the bits that combine.

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::exit::{INVALID, IO, NO, OK, REFUSED, REPAIRED, USAGE};

/// `mt`, offline by default: no `bitcoin-cli` on the child's `PATH`, so a
/// machine with one never sends it fixture txids. A test that wants a node
/// names a stub with `--bitcoin-cli`.
fn mt() -> Command {
    let mut c = Command::cargo_bin("mt").unwrap();
    c.env("PATH", "/nonexistent");
    c
}

/// The offline posture: no `--bitcoin-cli` on the command line and no
/// `bitcoin-cli` on `PATH` (set on the child only). A NAMED path that is
/// missing is a helper failing (R9), not a way to go offline.
const NO_NODE_PATH: &str = "/nonexistent";

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
    run_with_path(NO_NODE_PATH, args, stdin)
}

/// As `run`, with the child's `PATH` set to `path`.
fn run_with_path(path: &str, args: &[&str], stdin: &str) -> (Option<i32>, String, String) {
    let o = mt()
        .env("PATH", path)
        .args(args)
        .write_stdin(stdin)
        .output()
        .unwrap();
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

/// R2 (review M1): every command-line check is judged before the §8.2i
/// same-file refusal, which stats files. A command line that is wrong AND
/// writes onto its input is 64, not 8.
#[test]
fn a_usage_error_outranks_a_same_file_refusal() {
    let f = tmp(&strings("even").join("\n"));
    let tx = tmp(vector("even", "raw_hex").as_str().unwrap());
    for verb in ["decode", "inspect"] {
        let same = std::fs::OpenOptions::new()
            .append(true)
            .open(f.path())
            .unwrap();
        let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
            .env("PATH", NO_NODE_PATH)
            .args([verb, "--in"])
            .arg(f.path())
            .arg("--transaction")
            .arg(tx.path())
            .stdout(same)
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(USAGE),
            "{verb}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let p = tx.path().to_str().unwrap();
    let (code, _, err) = run(&["encode", "--in", p, "-", "--out", p], "");
    assert_eq!(code, Some(USAGE), "encode: {err}");
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
            let (code, out, err) = run(&[verb], input);
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
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
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
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
        .arg(f.path())
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(IO | REPAIRED));
}

/// A `bitcoin-cli` stub with this shell body, in its own directory (a file
/// open for writing cannot be exec'd).
fn helper(body: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let cli = dir.path().join("bitcoin-cli");
    std::fs::write(&cli, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let p = cli.to_str().unwrap().to_string();
    (dir, p)
}

/// Run `decode`, `inspect` and `encode` against a named `--bitcoin-cli` that
/// fails, and assert each exits 32, still prints its output (R9: the run keeps
/// its report and strings), and names the helper on ONE stderr line (review
/// M3): `mt: --bitcoin-cli <path>: <reason>`.
fn assert_named_helper_fails(cli: &str, reason: &str, why: &str) {
    let set = strings("even").join("\n");
    let raw = vector("even", "raw_hex");
    let f = tmp(raw.as_str().unwrap());
    let fp = f.path().to_str().unwrap();
    for argv in [
        &["decode", "--bitcoin-cli", cli][..],
        &["inspect", "--bitcoin-cli", cli],
        &["encode", "--bitcoin-cli", cli, "--in", fp],
    ] {
        let verb = argv[0];
        let (code, out, err) = run(argv, if verb == "encode" { "" } else { &set });
        assert_eq!(code, Some(IO), "{why}, {verb}: {err}");
        assert!(!out.is_empty(), "{why}: {verb} lost its output");
        if verb == "encode" {
            assert!(out.starts_with("mt1"), "{why}: encode lost its strings");
        }
        let prefix = format!("mt: --bitcoin-cli {cli}: ");
        let named: Vec<&str> = err.lines().filter(|l| l.starts_with(&prefix)).collect();
        assert_eq!(
            named.len(),
            1,
            "{why}, {verb}: want one line naming the helper:\n{err}"
        );
        assert!(
            named[0].contains(reason),
            "{why}, {verb}: {:?} does not say {reason:?}",
            named[0]
        );
    }
}

/// R9: a `bitcoin-cli` the operator NAMED, that runs and fails, sets 32 and
/// the output still prints.
#[test]
fn a_named_bitcoin_cli_that_fails_is_io() {
    let (_d, cli) = helper("#!/bin/sh\nwhile read -r _; do :; done\nexit 1\n");
    assert_named_helper_fails(&cli, "exited 1", "exits 1");
}

/// Review M1: Core's not-found (5) and an old Core's method-not-found (89) are
/// answers only from the calls that give them (`getrawtransaction`,
/// `getindexinfo`). A named helper that exits 5 or 89 on EVERY call fails.
#[test]
fn a_named_bitcoin_cli_exiting_5_or_89_on_every_call_is_io() {
    for n in [5, 89] {
        let (_d, cli) = helper(&format!(
            "#!/bin/sh\nwhile read -r _; do :; done\nexit {n}\n"
        ));
        assert_named_helper_fails(&cli, &format!("exited {n}"), &format!("exit {n}"));
    }
}

/// R9 (review I1): a named helper that exits 0 but gives no reply mt can use
/// (nothing at all, or garbage) has failed just the same.
#[test]
fn a_named_bitcoin_cli_with_no_usable_reply_is_io() {
    let (_d, cli) = helper("#!/bin/sh\nwhile read -r _; do :; done\nexit 0\n");
    assert_named_helper_fails(&cli, "gave a reply mt could not read", "silent");
    let (_d, cli) = helper("#!/bin/sh\nwhile read -r _; do :; done\necho garbage\n");
    assert_named_helper_fails(&cli, "gave a reply mt could not read", "garbage");
}

/// R9 and §3 test 2, "failed or gone missing" (coordinator ruling on review
/// I2): a NAMED path that does not exist, or cannot be started, is 32.
#[test]
fn a_named_bitcoin_cli_that_is_missing_is_io() {
    assert_named_helper_fails(
        "/nonexistent/bitcoin-cli",
        "could not be started: ",
        "missing path",
    );
    let (_d, cli) = helper("#!/nonexistent/sh\nexit 0\n");
    assert_named_helper_fails(&cli, "could not be started: ", "missing interpreter");
}

/// Review M5: naming the default, `--bitcoin-cli bitcoin-cli`, is naming it:
/// the node is then required, and its absence is 32.
#[test]
fn naming_the_default_bitcoin_cli_makes_the_node_required() {
    assert_named_helper_fails("bitcoin-cli", "could not be started: ", "named default");
}

/// Review M5: the DEFAULT `bitcoin-cli`, found on `PATH`, that fails or gives
/// garbage is not a helper the operator asked for: 0, with the warning.
#[test]
fn the_default_bitcoin_cli_on_path_that_fails_is_a_warning() {
    let set = strings("even").join("\n");
    for body in [
        "#!/bin/sh\nwhile read -r _; do :; done\nexit 1\n",
        "#!/bin/sh\nwhile read -r _; do :; done\necho garbage\n",
    ] {
        let (d, _) = helper(body);
        let path = d.path().to_str().unwrap();
        for verb in ["decode", "inspect"] {
            let (code, out, err) = run_with_path(path, &[verb], &set);
            assert_eq!(code, Some(OK), "{verb} {body:?}: {err}");
            assert!(!out.is_empty(), "{verb} lost its output");
            assert!(
                format!("{out}{err}").contains("WARNING"),
                "{verb} {body:?}: no warning:\n{out}{err}"
            );
            assert!(!err.contains("mt: --bitcoin-cli"), "{err}");
        }
    }
}

/// Review M2: `verify` never asks a node, so a named `--bitcoin-cli` there is
/// a command-line error (64), like `--transaction` on `decode`.
#[test]
fn verify_refuses_a_named_bitcoin_cli() {
    let set = strings("even").join("\n");
    let s = common::node_stub("", &[]);
    for cli in ["/nonexistent/bitcoin-cli", s.path().to_str().unwrap()] {
        let (code, out, err) = run(&["verify", "--bitcoin-cli", cli], &set);
        assert_eq!(code, Some(USAGE), "{cli}: {err}");
        assert!(out.is_empty());
    }
    let (code, _, err) = run(&["verify"], &set);
    assert_eq!(code, Some(OK), "{err}");
}

/// R11: each verb's help states the bits that verb can set, and no others.
#[test]
fn each_verb_help_states_its_bits() {
    for (verb, line) in [
        (
            "encode",
            "Exit status: 0, 64 or 101, or the sum of: 1 the node proves an input spent or \
             contradicts a value; 2 the input is invalid; 8 refused; 32 a file, device or \
             helper failed.",
        ),
        (
            "decode",
            "Exit status: 0, 64 or 101, or the sum of: 2 the input is invalid; 8 refused; \
             16 a correction was applied; 32 a file, device or helper failed.",
        ),
        (
            "verify",
            "Exit status: 0, 64 or 101, or the sum of: 1 --transaction names another \
             transaction; 2 the input is invalid; 8 refused; 16 a correction was applied; \
             32 a file or device failed.",
        ),
        (
            "inspect",
            "Exit status: 0, 64 or 101, or the sum of: 2 the input is invalid; 8 refused; \
             16 a correction was applied; 32 a file, device or helper failed.",
        ),
    ] {
        let (code, out, _) = run(&[verb, "--help"], "");
        assert_eq!(code, Some(OK));
        let flat = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat(&out).ends_with(&flat(line)),
            "{verb} --help does not end with its bits:\n{out}"
        );
    }
}

/// Only the DEFAULT `bitcoin-cli` being absent is the offline path: 0, with
/// its warning.
#[test]
fn the_default_bitcoin_cli_absent_is_offline() {
    let set = strings("even").join("\n");
    for verb in ["decode", "inspect"] {
        let (code, out, err) = run(&[verb], &set);
        assert_eq!(code, Some(OK), "{verb}: {err}");
        assert!(!out.is_empty(), "{verb} lost its output");
        assert!(
            format!("{out}{err}").contains("no bitcoind reachable"),
            "{verb}: the offline warning is missing:\n{out}{err}"
        );
    }
}

/// Core's real answers are not a failing helper: `gettxout` null (empty
/// output) and `getrawtransaction` not found (exit 5). (`getindexinfo` `{}`
/// and exit 89 are pinned in `node_tristate.rs`.)
#[test]
fn a_named_node_giving_real_null_answers_is_ok() {
    let s = common::node_stub("", &[]);
    let raw = vector("even", "raw_hex");
    let f = tmp(raw.as_str().unwrap());
    let (code, out, err) = run(
        &[
            "inspect",
            "--bitcoin-cli",
            s.path().to_str().unwrap(),
            "--in",
            f.path().to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, Some(OK), "{err}");
    assert!(!out.is_empty());
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
        let (code, out, err) = run(&[verb], &set);
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
    let (code, _, _) = run(&["decode", "--quiet"], &set);
    assert_eq!(code, Some(REPAIRED));
    let (code, _, _) = run(&["inspect", "--json"], &set);
    assert_eq!(code, Some(REPAIRED));
}

#[test]
fn a_clean_set_stays_ok() {
    let set = strings("even").join("\n");
    for verb in ["decode", "verify", "inspect"] {
        let (code, _, err) = run(&[verb], &set);
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
