//! bughunt-1119 FIX-H (mt): argument intake, keys bh-mt-I9, I11, I12, I24
//! (architect rulings T20, T12, T13, T22). Spec: design/FIX_bh1119_mt_arg_intake.md.
//!
//! I9: `encode --separator` admits exactly the separators the reading verbs
//! admit (one shared predicate), so a separator containing a newline is refused.
//! I11: `--transaction` is refused on `decode` and `inspect`; only `verify`
//! compares. I12: `--in <path>` together with the positional `-` is refused on
//! every verb. I24: a non-UTF-8 argument is refused by name, exit 64, before
//! anything else reads argv. Every refusal here is a USAGE error (64): the
//! command line alone is wrong (exit-code spec decision 2).

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::exit::USAGE;

/// The texts this FIX adds (PROPOSED, architect confirms). A wording change
/// updates these constants and nothing else.
const T_PHRASE: &str = "only `verify` makes that comparison";
const D_VERDICT: &str = "--in and `-` name two different inputs";
const D_PHRASE: &str = "mt reads exactly one input";
const N_PHRASE: &str = "a newline ends a string";
const U_PHRASE: &str = "does not print the argument back";

fn t_verdict(verb: &str) -> String {
    format!("mt {verb}: REFUSED — §10.10, --transaction has no meaning for `{verb}`")
}

fn d_verdict(verb: &str) -> String {
    format!("mt {verb}: REFUSED — §10.10, {D_VERDICT}")
}

fn n_verdict(sep: &str) -> String {
    format!("mt encode: REFUSED — §1.1e, --separator {sep:?} contains a newline")
}

fn u_verdict(verb: &str, n: usize) -> String {
    format!("mt {verb}: REFUSED — §10.10, argument {n} is not valid UTF-8")
}

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

fn raw_hex() -> String {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    v["raw_hex"].as_str().unwrap().to_string()
}

fn file_with(text: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(text.as_bytes()).unwrap();
    f.flush().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    f
}

/// `mt encode` of the fixture, with any extra flags; returns (rc, stdout, stderr).
fn encode_with(extra: &[&str]) -> (Option<i32>, String, String) {
    let tx = file_with(&raw_hex());
    let out = mt()
        .env("PATH", "/nonexistent")
        .args(["encode"])
        .args(extra)
        .arg("--in")
        .arg(tx.path())
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A file holding the fixture's mt1 strings, one per line.
fn strings_file() -> tempfile::NamedTempFile {
    let (rc, out, err) = encode_with(&[]);
    assert_eq!(rc, Some(0), "fixture encode failed:\n{err}");
    file_with(&out)
}

/// stderr with the refusal's indentation and wrapping undone, so a phrase that
/// the 68-column wrap split across lines still matches.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_refusal(rc: Option<i32>, out: &str, err: &str, code: i32, first: &str, phrase: &str) {
    assert_eq!(rc, Some(code), "exit code; stderr:\n{err}");
    assert!(out.is_empty(), "a refusal wrote to stdout:\n{out}");
    assert_eq!(
        err.lines().next(),
        Some(first),
        "first stderr line; stderr:\n{err}"
    );
    assert!(flat(err).contains(phrase), "missing {phrase:?} in:\n{err}");
}

// ---- I9 (T20): encode admits exactly the separators decode admits ----------

fn assert_newline_separator_refused(sep: &str, class: i32) {
    let (rc, out, err) = encode_with(&["--group-size", "5", "--separator", sep]);
    assert_refusal(rc, &out, &err, class, &n_verdict(sep), N_PHRASE);
}

#[test]
fn bh1119_mt_i9_encode_refuses_lf_separator() {
    assert_newline_separator_refused("\n", USAGE);
}

#[test]
fn bh1119_mt_i9_encode_refuses_crlf_separator() {
    assert_newline_separator_refused("\r\n", USAGE);
}

#[test]
fn bh1119_mt_i9_encode_refuses_separator_ending_in_lf() {
    assert_newline_separator_refused(" \n", USAGE);
}

/// Guard: a separator that is not all whitespace keeps the existing refusal,
/// even when it also holds a newline.
#[test]
fn bh1119_mt_i9_non_whitespace_wins_over_newline() {
    let (rc, out, err) = encode_with(&["--group-size", "5", "--separator", ".\n"]);
    assert_eq!(rc, Some(USAGE), "stderr:\n{err}");
    assert!(out.is_empty());
    assert_eq!(
        err.lines().next(),
        Some("mt encode: REFUSED — §1.1e, --separator \".\\n\" is not whitespace"),
        "stderr:\n{err}"
    );
}

/// Guard: every whitespace separator without a newline is still admitted, and
/// `mt decode` reads the result back to the same transaction.
#[test]
fn bh1119_mt_i9_encode_admits_every_separator_decode_admits() {
    let raw = raw_hex();
    for sep in [
        "", " ", "\t", "  ", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{2028}",
    ] {
        let (rc, out, err) = encode_with(&["--group-size", "5", "--separator", sep]);
        assert_eq!(rc, Some(0), "separator {sep:?} refused:\n{err}");
        let f = file_with(&out);
        let back = mt()
            .env("PATH", "/nonexistent")
            .args(["decode", "--quiet", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert_eq!(
            back.status.code(),
            Some(0),
            "separator {sep:?}: decode refused:\n{}",
            String::from_utf8_lossy(&back.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&back.stdout).trim(),
            raw,
            "separator {sep:?}"
        );
    }
}

// ---- I11 (T12): --transaction refused on decode and inspect ----------------

fn run_read(
    verb: &str,
    input: &std::path::Path,
    extra: &[&std::ffi::OsStr],
) -> (Option<i32>, String, String) {
    let out = mt()
        .env("PATH", "/nonexistent")
        .args([verb, "--in"])
        .arg(input)
        .args(extra)
        .write_stdin("not read\n")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn bh1119_mt_i11_decode_refuses_transaction() {
    let set = strings_file();
    let missing = std::ffi::OsStr::new("/nonexistent/tx.hex");
    let (rc, out, err) = run_read("decode", set.path(), &["--transaction".as_ref(), missing]);
    assert_refusal(rc, &out, &err, USAGE, &t_verdict("decode"), T_PHRASE);
}

#[test]
fn bh1119_mt_i11_inspect_refuses_transaction() {
    let set = strings_file();
    let missing = std::ffi::OsStr::new("/nonexistent/tx.hex");
    let (rc, out, err) = run_read("inspect", set.path(), &["--transaction".as_ref(), missing]);
    assert_refusal(rc, &out, &err, USAGE, &t_verdict("inspect"), T_PHRASE);
}

/// The raw-transaction subject of inspect, with a file that exists and matches:
/// still refused, because nothing would compare it.
#[test]
fn bh1119_mt_i11_inspect_raw_tx_refuses_existing_transaction() {
    let tx = file_with(&raw_hex());
    let (rc, out, err) = run_read(
        "inspect",
        tx.path(),
        &["--transaction".as_ref(), tx.path().as_os_str()],
    );
    assert_refusal(rc, &out, &err, USAGE, &t_verdict("inspect"), T_PHRASE);
}

/// Guard: verify still compares.
#[test]
fn bh1119_mt_i11_verify_still_compares() {
    let set = strings_file();
    let tx = file_with(&raw_hex());
    let (rc, _out, err) = run_read(
        "verify",
        set.path(),
        &["--transaction".as_ref(), tx.path().as_os_str()],
    );
    assert_eq!(rc, Some(0), "stderr:\n{err}");
    assert!(
        err.contains("--transaction matches, on the full txid."),
        "stderr:\n{err}"
    );
}

/// Order: --transaction is refused before --in with `-`.
#[test]
fn bh1119_mt_i11_transaction_refusal_precedes_dash_refusal() {
    let set = strings_file();
    let missing = std::ffi::OsStr::new("/nonexistent/tx.hex");
    let (rc, out, err) = run_read(
        "decode",
        set.path(),
        &["--transaction".as_ref(), missing, "-".as_ref()],
    );
    assert_refusal(rc, &out, &err, USAGE, &t_verdict("decode"), T_PHRASE);
}

/// Guard: the existing flag refusals keep precedence over both new ones.
#[test]
fn bh1119_mt_i11_existing_flag_refusals_keep_precedence() {
    let set = strings_file();
    let missing = std::ffi::OsStr::new("/nonexistent/tx.hex");
    let (rc, _out, err) = run_read(
        "decode",
        set.path(),
        &[
            "--json".as_ref(),
            "--quiet".as_ref(),
            "--transaction".as_ref(),
            missing,
            "-".as_ref(),
        ],
    );
    assert_eq!(rc, Some(USAGE));
    assert_eq!(
        err.lines().next(),
        Some("mt decode: REFUSED — §10.10, --json and --quiet contradict each other"),
        "stderr:\n{err}"
    );
    let (rc, _out, err) = run_read(
        "inspect",
        set.path(),
        &["--quiet".as_ref(), "--transaction".as_ref(), missing],
    );
    assert_eq!(rc, Some(USAGE));
    assert_eq!(
        err.lines().next(),
        Some("mt inspect: REFUSED — §10.10, --quiet has no meaning for `inspect`"),
        "stderr:\n{err}"
    );
}

// ---- I12 (T13): --in together with `-` refused on every verb ---------------

#[test]
fn bh1119_mt_i12_decode_refuses_in_with_dash() {
    let set = strings_file();
    let (rc, out, err) = run_read("decode", set.path(), &["-".as_ref()]);
    assert_refusal(rc, &out, &err, USAGE, &d_verdict("decode"), D_PHRASE);
}

#[test]
fn bh1119_mt_i12_verify_refuses_in_with_dash() {
    let set = strings_file();
    let (rc, out, err) = run_read("verify", set.path(), &["-".as_ref()]);
    assert_refusal(rc, &out, &err, USAGE, &d_verdict("verify"), D_PHRASE);
}

#[test]
fn bh1119_mt_i12_inspect_refuses_in_with_dash() {
    let set = strings_file();
    let (rc, out, err) = run_read("inspect", set.path(), &["-".as_ref()]);
    assert_refusal(rc, &out, &err, USAGE, &d_verdict("inspect"), D_PHRASE);
}

#[test]
fn bh1119_mt_i12_encode_refuses_in_with_dash() {
    let (rc, out, err) = encode_with(&["-"]);
    assert_refusal(rc, &out, &err, USAGE, &d_verdict("encode"), D_PHRASE);
}

/// Guard: `--in` alone and `-` alone still read their one input.
#[test]
fn bh1119_mt_i12_in_alone_and_dash_alone_still_work() {
    let set = strings_file();
    let raw = raw_hex();
    let (rc, out, err) = run_read("decode", set.path(), &["--quiet".as_ref()]);
    assert_eq!(rc, Some(0), "stderr:\n{err}");
    assert_eq!(out.trim(), raw);
    let body = std::fs::read_to_string(set.path()).unwrap();
    let dashed = mt()
        .env("PATH", "/nonexistent")
        .args(["decode", "--quiet", "-"])
        .write_stdin(body)
        .output()
        .unwrap();
    assert_eq!(dashed.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&dashed.stdout).trim(), raw);
}

// ---- I24 (T22): a non-UTF-8 argument is refused, exit 64 -------------------

#[cfg(unix)]
fn run_os(args: &[&[u8]]) -> (Option<i32>, String, String) {
    use std::os::unix::ffi::OsStrExt;
    let out = mt()
        .args(args.iter().map(|a| std::ffi::OsStr::from_bytes(a)))
        .write_stdin("")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[cfg(unix)]
#[test]
fn bh1119_mt_i24_non_utf8_argument_is_refused_exit_64() {
    let (rc, out, err) = run_os(&[b"verify", b"--in", b"plate-\xff.txt"]);
    assert_refusal(rc, &out, &err, USAGE, &u_verdict("verify", 3), U_PHRASE);
    assert!(!err.contains("panicked"), "stderr:\n{err}");
    assert!(!err.contains("plate-"), "the argument was echoed:\n{err}");
}

#[cfg(unix)]
#[test]
fn bh1119_mt_i24_names_the_first_non_utf8_argument() {
    let (rc, out, err) = run_os(&[b"decode", b"--in", b"\xfe", b"--transaction", b"\xff"]);
    assert_refusal(rc, &out, &err, USAGE, &u_verdict("decode", 3), U_PHRASE);
}

/// Before `argv_intake`: the override's admitted material is never echoed and
/// never read.
#[cfg(unix)]
#[test]
fn bh1119_mt_i24_refused_before_allow_argv_secret() {
    let raw = raw_hex();
    let (rc, out, err) = run_os(&[
        b"encode",
        b"--allow-argv-secret",
        raw.as_bytes(),
        b"--in",
        b"\xff",
    ]);
    assert_refusal(rc, &out, &err, USAGE, &u_verdict("encode", 5), U_PHRASE);
    assert!(!err.contains(&raw[..16]), "material echoed:\n{err}");
}

/// No verb typed: the verb falls back exactly as the §8.2f pre-clap guard's does.
#[cfg(unix)]
#[test]
fn bh1119_mt_i24_untyped_verb_falls_back_to_encode() {
    let (rc, out, err) = run_os(&[b"\xff"]);
    assert_refusal(rc, &out, &err, USAGE, &u_verdict("encode", 1), U_PHRASE);
}

/// Guard: a UTF-8 path outside ASCII still works.
#[test]
fn bh1119_mt_i24_utf8_non_ascii_path_still_works() {
    let set = strings_file();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("plate-é.txt");
    std::fs::copy(set.path(), &p).unwrap();
    let out = mt()
        .env("PATH", "/nonexistent")
        .args(["verify", "--in"])
        .arg(&p)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The program path (argv[0]) is not an argument: a non-UTF-8 install path is
/// read lossily and never refused.
#[cfg(unix)]
#[test]
fn bh1119_mt_i24_non_utf8_program_path_is_not_refused() {
    use std::os::unix::ffi::OsStrExt;
    let set = strings_file();
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join(std::ffi::OsStr::from_bytes(b"mt-\xff"));
    std::os::unix::fs::symlink(assert_cmd::cargo::cargo_bin("mt"), &link).unwrap();
    let out = std::process::Command::new(&link)
        .env("PATH", "/nonexistent")
        .args(["verify", "--in"])
        .arg(set.path())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
