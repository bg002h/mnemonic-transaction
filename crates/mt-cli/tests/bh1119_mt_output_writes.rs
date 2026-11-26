//! Bughunt 2026-11-19 FIX-B, output writes (design/FIX_bh1119_mt_output_writes.md).
//!
//! Keys: bh-mt-C2 (a failed stdout write exits 0), bh-mt-C3 (a failed `--out`
//! replacement empties the old file and prints two false claims), bh-mt-I4
//! (`--out` or stdout naming an input replaces it at exit 0). Rulings: T5
//! (mnemonic-io-lib 0.1.3 `write_private` and `same_file`), T6 (the failed-write
//! contract), Brian 2026-11-23 Q2 (an output that names an input is refused)
//! and the file-output convention's R5 (truthful text).
//!
//! Every run is offline (no `--bitcoin-cli`, no `bitcoin-cli` on `PATH`). Inputs are
//! the corpus vector `even` in `mt-codec/src/test_vectors/mt1_v1.json`. Unix
//! only: /dev/full, `ulimit -f`, hard links and symlinks.
//!
//! Exit codes follow the constellation exit-code spec (mnemonic-engrave
//! `SPEC_constellation_exit_codes.md`): a failed write ADDS IO (32, R8), an
//! output naming an input is REFUSED (8), usage is 64. Test names that say
//! `exits_1` / `exit_2` predate it and are kept, because the FIX document's
//! row table cites them.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

mod common;
use common::exit::{INVALID, IO, REFUSED, USAGE};

/// Offline: no `--bitcoin-cli`, and no `bitcoin-cli` on the child's `PATH`
/// (a NAMED path that is missing adds 32, exit-code spec R9).
const NO_NODE_PATH: &str = "/nonexistent";
const FULL: &str = "mt: cannot write the output: No space left on device (os error 28)";
const BADF: &str = "mt: cannot write the output: Bad file descriptor (os error 9)";
const WRITE_LINE_PREFIX: &str = "mt: cannot write the output: ";
const MECH_OUT: &str = "They are one file (the same path, a symlink or a hard link), so \
     writing the output would replace the input mt is reading.";
const MECH_STDOUT: &str = "They are one file (the same path, a symlink or a hard link), so \
     the output would go into the file mt reads its input from. A > redirect empties that \
     file before mt starts, so check that it still holds what you expect.";
const REMEDY: &str = "Write to a new name.";

fn bin() -> PathBuf {
    assert_cmd::cargo::cargo_bin("mt")
}

fn mt() -> Command {
    let mut c = Command::new(bin());
    c.env("PATH", NO_NODE_PATH);
    c
}

fn vector(label: &str) -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["label"] == label)
        .unwrap()
        .clone()
}

/// The corpus `even` transaction (222 bytes) as hex, newline-terminated.
fn even_hex() -> String {
    vector("even")["raw_hex"].as_str().unwrap().to_string() + "\n"
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

/// A fresh directory holding `name` with `body`, mode 0600.
fn dir_with(name: &str, body: &str) -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join(name);
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
    (d, p)
}

fn full() -> Stdio {
    Stdio::from(
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap(),
    )
}

fn append_to(p: &Path) -> Stdio {
    Stdio::from(std::fs::OpenOptions::new().append(true).open(p).unwrap())
}

fn read_from(p: &Path) -> Stdio {
    Stdio::from(std::fs::File::open(p).unwrap())
}

fn run(c: &mut Command) -> Output {
    c.output().unwrap()
}

/// `mt <args>` under `ulimit -f 0` with SIGXFSZ ignored: every write to a
/// regular file fails with EFBIG after the open.
fn run_no_file_writes(args: &[&std::ffi::OsStr]) -> Output {
    Command::new("bash")
        .arg("-c")
        .arg(r#"trap '' XFSZ; ulimit -f 0; export PATH="$NO_NODE_PATH"; exec "$@""#)
        .env("NO_NODE_PATH", NO_NODE_PATH)
        .arg("bash")
        .arg(bin())
        .args(args)
        .output()
        .unwrap()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn last_line(o: &Output) -> String {
    err(o).lines().last().unwrap_or("").to_string()
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The same-file refusal as printed: the first line exact, the rest compared
/// with whitespace collapsed (the mechanism and remedy wrap at 68 columns).
fn assert_same_file_refusal(
    class: i32,
    o: &Output,
    verb: &str,
    output: &str,
    input: &str,
    stdout_form: bool,
) {
    let e = err(o);
    assert_eq!(o.status.code(), Some(class), "exit code; stderr:\n{e}");
    let first = e.lines().next().unwrap_or("");
    assert_eq!(
        first,
        format!("mt {verb}: REFUSED — §8.2i, {output} is the same file as {input}"),
        "first line; stderr:\n{e}"
    );
    let mech = if stdout_form { MECH_STDOUT } else { MECH_OUT };
    assert_eq!(
        norm(&e),
        norm(&format!("{first}\n{mech}\n{REMEDY}")),
        "whole refusal"
    );
}

fn assert_write_failure(o: &Output, line: &str) {
    let e = err(o);
    assert_eq!(o.status.code(), Some(IO), "exit code; stderr:\n{e}");
    assert_eq!(last_line(o), line, "last stderr line; stderr:\n{e}");
    assert_eq!(
        e.matches(WRITE_LINE_PREFIX).count(),
        1,
        "printed once; stderr:\n{e}"
    );
}

fn even_strings_file() -> (tempfile::TempDir, PathBuf) {
    dir_with("s.txt", &even_strings())
}

fn even_hex_file() -> (tempfile::TempDir, PathBuf) {
    dir_with("tx.hex", &even_hex())
}

// ---- C2: a failed stdout write (T6) ---------------------------------------

#[test]
fn bh1119_mt_c2_encode_stdout_full_exits_1() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_encode_qr_stdout_full_exits_1() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--qr", "--in"])
        .arg(&tx)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_decode_stdout_full_exits_1() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
        .arg(&s)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_decode_json_stdout_full_exits_1() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--json", "--in"])
        .arg(&s)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_inspect_strings_stdout_full_exits_1() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["inspect", "--in"])
        .arg(&s)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_inspect_strings_json_stdout_full_exits_1() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["inspect", "--json", "--in"])
        .arg(&s)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_inspect_transaction_stdout_full_exits_1() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["inspect", "--in"])
        .arg(&tx)
        .stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_help_stdout_full_exits_1() {
    let o = run(mt().arg("--help").stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_subcommand_help_stdout_full_exits_1() {
    let o = run(mt().args(["decode", "--help"]).stdout(full()));
    assert_write_failure(&o, FULL);
}

#[test]
fn bh1119_mt_c2_version_stdout_full_exits_1() {
    let o = run(mt().arg("--version").stdout(full()));
    assert_write_failure(&o, FULL);
}

/// stdout opened read-only (`1</dev/null`): the kernel answers EBADF, which
/// std's stdout reports as a successful write.
#[test]
fn bh1119_mt_c2_decode_read_only_stdout_exits_1() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
        .arg(&s)
        .stdout(read_from(Path::new("/dev/null"))));
    assert_write_failure(&o, BADF);
}

/// Guard: a reader that closed the pipe before mt wrote (EPIPE) only stops the
/// output; the exit code is the verb's own and nothing is said.
#[test]
fn bh1119_mt_c2_decode_closed_pipe_keeps_exit_0() {
    let (_d, s) = even_strings_file();
    let o = run(Command::new("python3")
        .arg("-c")
        .arg(
            "import os,subprocess,sys\n\
             r,w=os.pipe(); os.close(r)\n\
             p=subprocess.run(sys.argv[1:],stdout=w,stderr=subprocess.PIPE,\n\
             env=dict(os.environ,PATH=os.environ['NO_NODE_PATH']))\n\
             sys.stderr.buffer.write(p.stderr); sys.exit(p.returncode)",
        )
        .arg(bin())
        .env("NO_NODE_PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
        .arg(&s));
    let e = err(&o);
    assert_eq!(o.status.code(), Some(0), "stderr:\n{e}");
    assert!(!e.contains(WRITE_LINE_PREFIX), "stderr:\n{e}");
}

/// Guard: verify's verdict is on stderr and is carried by the exit code; a
/// failed stderr write does not change it (T6).
#[test]
fn bh1119_mt_c2_verify_stderr_full_keeps_exit_0() {
    let (_d, s) = even_strings_file();
    let o = run(mt().args(["verify", "--in"]).arg(&s).stderr(full()));
    assert_eq!(o.status.code(), Some(0));
}

/// Guard: decode --json's document goes to stderr; a failed stderr write
/// changes neither the exit code nor the hex on stdout.
#[test]
fn bh1119_mt_c2_decode_json_stderr_full_keeps_exit_0_and_the_hex() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--json", "--in"])
        .arg(&s)
        .stderr(full()));
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&o.stdout), even_hex());
}

/// Guard: a verb that refuses writes nothing to stdout and keeps its code.
#[test]
fn bh1119_mt_c2_refused_verb_keeps_exit_1_and_says_no_write_line() {
    let (_d, g) = dir_with("g.txt", "not a transaction\n");
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
        .arg(&g)
        .stdout(full()));
    let e = err(&o);
    assert_eq!(o.status.code(), Some(INVALID), "stderr:\n{e}");
    assert!(e.contains("REFUSED"), "stderr:\n{e}");
    assert!(!e.contains(WRITE_LINE_PREFIX), "stderr:\n{e}");
}

/// Guard: a usage error exits 64, with no write line.
#[test]
fn bh1119_mt_c2_usage_error_keeps_exit_2() {
    let o = run(mt().args(["decode", "--no-such-flag"]).stdout(full()));
    assert_eq!(o.status.code(), Some(USAGE), "stderr:\n{}", err(&o));
    assert!(!err(&o).contains(WRITE_LINE_PREFIX));
}

// ---- C3: a failed --out write (T5 pin, T6 line, R5 text) -------------------

#[test]
fn bh1119_mt_c3_failed_replacement_keeps_the_previous_file() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("strings.txt");
    std::fs::write(&out, "OLD BACKUP\n").unwrap();
    let o = run_no_file_writes(&[
        "encode".as_ref(),
        "--in".as_ref(),
        tx.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
    ]);
    assert_eq!(o.status.code(), Some(IO), "stderr:\n{}", err(&o));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "OLD BACKUP\n");
    let mut names: Vec<String> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["strings.txt", "tx.hex"], "no temp left behind");
}

#[test]
fn bh1119_mt_c3_failed_replacement_says_the_file_is_unchanged() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("strings.txt");
    std::fs::write(&out, "OLD BACKUP\n").unwrap();
    let o = run_no_file_writes(&[
        "encode".as_ref(),
        "--in".as_ref(),
        tx.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
    ]);
    let p = out.display();
    assert_write_failure(
        &o,
        &format!(
            "mt: cannot write the output: {p}: File too large (os error 27). {p} is unchanged."
        ),
    );
    let e = err(&o);
    assert!(!e.contains("left behind"), "stderr:\n{e}");
    assert!(!e.contains("went into"), "stderr:\n{e}");
    assert!(!e.contains("REFUSED"), "stderr:\n{e}");
}

#[test]
fn bh1119_mt_c3_failed_new_file_leaves_nothing() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("strings.txt");
    let o = run_no_file_writes(&[
        "encode".as_ref(),
        "--in".as_ref(),
        tx.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
    ]);
    let p = out.display();
    assert_write_failure(
        &o,
        &format!(
            "mt: cannot write the output: {p}: File too large (os error 27). Nothing was written."
        ),
    );
    let names: Vec<String> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        names,
        ["tx.hex"],
        "nothing created at the name, no temp left"
    );
    assert!(!err(&o).contains("went into"));
}

#[test]
fn bh1119_mt_c3_missing_directory_says_nothing_was_written() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("no-such-dir").join("strings.txt");
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .arg("--out")
        .arg(&out));
    let p = out.display();
    assert_write_failure(
        &o,
        &format!(
            "mt: cannot write the output: {p}: No such file or directory (os error 2). \
             Nothing was written."
        ),
    );
}

/// A target that is not a regular file (here a character device) is written
/// directly; the line names no on-disk state. The OS error varies with the
/// user (EPERM from the chmod for a non-root user, ENOSPC as root).
#[test]
fn bh1119_mt_c3_device_target_names_no_state() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .args(["--out", "/dev/full"]));
    let e = err(&o);
    assert_eq!(o.status.code(), Some(IO), "stderr:\n{e}");
    let last = last_line(&o);
    assert!(
        last.starts_with("mt: cannot write the output: /dev/full: "),
        "stderr:\n{e}"
    );
    assert!(!last.ends_with("is unchanged."), "{last}");
    assert!(!last.ends_with("Nothing was written."), "{last}");
    assert!(last.ends_with(')'), "ends with the OS error: {last}");
}

/// A standard-stream name is written through the open stream by io-lib 0.1.3,
/// at the stream's own offset, even when it resolves to a regular file (stdout
/// appended to `s.txt`). With s.txt at 1023 bytes and `ulimit -f 1`, one byte
/// is appended and the next write fails, so the file is no longer as it was and
/// the line must name no on-disk state. The second look in W3 (architect io-lib
/// 1124 F2) is what keeps this true.
#[test]
fn bh1119_mt_c3_stdio_alias_onto_a_file_names_no_state() {
    let (d, tx) = even_hex_file();
    let s = d.path().join("s.txt");
    let old = format!("{}\n", "O".repeat(1022));
    std::fs::write(&s, &old).unwrap();
    let o = run(Command::new("bash")
        .arg("-c")
        .arg(r#"trap '' XFSZ; ulimit -f 1; export PATH="$NO_NODE_PATH"; exec "$@""#)
        .arg("bash")
        .arg(bin())
        .env("NO_NODE_PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .args(["--out", "/dev/stdout"])
        .stdout(append_to(&s)));
    let e = err(&o);
    assert_eq!(o.status.code(), Some(IO), "stderr:\n{e}");
    let last = last_line(&o);
    assert!(
        last.starts_with("mt: cannot write the output: /dev/stdout: "),
        "stderr:\n{e}"
    );
    assert!(!last.ends_with("is unchanged."), "{last}");
    assert!(!last.ends_with("Nothing was written."), "{last}");
    assert!(last.ends_with(')'), "ends with the OS error: {last}");
    assert!(
        std::fs::read_to_string(&s).unwrap().starts_with(&old),
        "s.txt keeps its 1023 old bytes"
    );
}

/// Guard: a successful replacement writes the strings 0600 and only then says
/// where they went.
#[test]
fn bh1119_mt_c3_successful_replacement_writes_0600_and_says_so() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("strings.txt");
    std::fs::write(&out, "OLD BACKUP\n").unwrap();
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o644)).unwrap();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .arg("--out")
        .arg(&out));
    let e = err(&o);
    assert_eq!(o.status.code(), Some(0), "stderr:\n{e}");
    assert!(e.contains("went into"), "stderr:\n{e}");
    let piped = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx));
    assert_eq!(std::fs::read(&out).unwrap(), piped.stdout);
    let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

// ---- I4: an output that names an input (Q2, T5 same_file) -------------------

#[test]
fn bh1119_mt_i4_encode_in_equals_out_refused() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .arg("--out")
        .arg(&tx));
    let p = tx.display();
    assert_same_file_refusal(
        REFUSED,
        &o,
        "encode",
        &format!("--out {p}"),
        &format!("--in {p}"),
        false,
    );
    assert_eq!(std::fs::read_to_string(&tx).unwrap(), even_hex());
}

#[test]
fn bh1119_mt_i4_encode_out_hard_link_to_in_refused() {
    let (d, tx) = even_hex_file();
    let link = d.path().join("hard.txt");
    std::fs::hard_link(&tx, &link).unwrap();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .arg("--out")
        .arg(&link));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "encode",
        &format!("--out {}", link.display()),
        &format!("--in {}", tx.display()),
        false,
    );
    assert_eq!(std::fs::read_to_string(&tx).unwrap(), even_hex());
}

#[test]
fn bh1119_mt_i4_encode_out_symlink_to_in_refused() {
    let (d, tx) = even_hex_file();
    let link = d.path().join("sym.txt");
    std::os::unix::fs::symlink(&tx, &link).unwrap();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .arg("--out")
        .arg(&link));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "encode",
        &format!("--out {}", link.display()),
        &format!("--in {}", tx.display()),
        false,
    );
    assert_eq!(std::fs::read_to_string(&tx).unwrap(), even_hex());
}

#[test]
fn bh1119_mt_i4_encode_out_equals_stdin_refused() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--out"])
        .arg(&tx)
        .stdin(read_from(&tx)));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "encode",
        &format!("--out {}", tx.display()),
        "standard input",
        false,
    );
    assert_eq!(std::fs::read_to_string(&tx).unwrap(), even_hex());
}

#[test]
fn bh1119_mt_i4_encode_stdout_appends_to_in_refused() {
    let (_d, tx) = even_hex_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .stdout(append_to(&tx)));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "encode",
        "standard output",
        &format!("--in {}", tx.display()),
        true,
    );
    assert_eq!(std::fs::read_to_string(&tx).unwrap(), even_hex());
}

#[test]
fn bh1119_mt_i4_decode_stdout_appends_to_in_refused() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode", "--in"])
        .arg(&s)
        .stdout(append_to(&s)));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "decode",
        "standard output",
        &format!("--in {}", s.display()),
        true,
    );
    assert_eq!(std::fs::read_to_string(&s).unwrap(), even_strings());
}

#[test]
fn bh1119_mt_i4_inspect_stdout_appends_to_in_refused() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["inspect", "--in"])
        .arg(&s)
        .stdout(append_to(&s)));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "inspect",
        "standard output",
        &format!("--in {}", s.display()),
        true,
    );
    assert_eq!(std::fs::read_to_string(&s).unwrap(), even_strings());
}

#[test]
fn bh1119_mt_i4_decode_stdin_and_stdout_one_file_refused() {
    let (_d, s) = even_strings_file();
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["decode"])
        .stdin(read_from(&s))
        .stdout(append_to(&s)));
    assert_same_file_refusal(
        REFUSED,
        &o,
        "decode",
        "standard output",
        "standard input",
        true,
    );
    assert_eq!(std::fs::read_to_string(&s).unwrap(), even_strings());
}

/// The check runs before the input is read: a file that is not a transaction
/// gets the same-file refusal, not the parse refusal.
#[test]
fn bh1119_mt_i4_refused_before_the_input_is_read() {
    let (_d, g) = dir_with("g.txt", "not a transaction\n");
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&g)
        .arg("--out")
        .arg(&g));
    let p = g.display();
    assert_same_file_refusal(
        REFUSED,
        &o,
        "encode",
        &format!("--out {p}"),
        &format!("--in {p}"),
        false,
    );
    assert_eq!(std::fs::read_to_string(&g).unwrap(), "not a transaction\n");
}

/// Guard: an `--out` beside the input, under another name, is written.
#[test]
fn bh1119_mt_i4_distinct_out_is_written() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("strings.txt");
    let o = run(mt()
        .env("PATH", NO_NODE_PATH)
        .args(["encode", "--in"])
        .arg(&tx)
        .arg("--out")
        .arg(&out));
    assert_eq!(o.status.code(), Some(0), "stderr:\n{}", err(&o));
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
    assert_eq!(std::fs::read_to_string(&tx).unwrap(), even_hex());
}

/// Guard: verify writes nothing to stdout, so a stdout that names its input is
/// not checked and verify still answers.
#[test]
fn bh1119_mt_i4_verify_is_not_checked() {
    let (_d, s) = even_strings_file();
    let o = run(mt().args(["verify", "--in"]).arg(&s).stdout(append_to(&s)));
    assert_eq!(o.status.code(), Some(0), "stderr:\n{}", err(&o));
    assert_eq!(std::fs::read_to_string(&s).unwrap(), even_strings());
}

// ---- Defect 1 (exit-code survey 2026-11-26): stdout closed at exec --------

/// `mt <args>` with fd 1 really closed at exec (`>&-`). std's runtime plugs a
/// closed standard fd with `/dev/null` before `main`, so the output was lost
/// at exit 0.
fn run_stdout_closed(args: &[&std::ffi::OsStr]) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(r#"export PATH="$NO_NODE_PATH"; exec "$0" "$@" >&-"#)
        .env("NO_NODE_PATH", NO_NODE_PATH)
        .arg(bin())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn closed_stdout_adds_io_with_the_line() {
    let (_d, s) = even_strings_file();
    let (_e, tx) = even_hex_file();
    let s = s.as_os_str();
    let tx = tx.as_os_str();
    let a = |v: &str| std::ffi::OsStr::new(v).to_owned();
    for args in [
        vec![a("decode"), a("--in"), s.into()],
        vec![a("decode"), a("--json"), a("--in"), s.into()],
        vec![a("encode"), a("--in"), tx.into()],
        vec![a("inspect"), a("--in"), s.into()],
        vec![a("--help")],
        vec![a("--version")],
    ] {
        let refs: Vec<&std::ffi::OsStr> = args.iter().map(|x| x.as_os_str()).collect();
        let o = run_stdout_closed(&refs);
        assert!(!err(&o).contains("panicked"), "{args:?}: {}", err(&o));
        assert_write_failure(&o, BADF);
    }
}

/// With stdout closed, `encode --out` succeeds and the file holds exactly the
/// strings: no stdout write lands in it.
#[test]
fn closed_stdout_with_out_file_succeeds_and_nothing_goes_astray() {
    let (d, tx) = even_hex_file();
    let out = d.path().join("strings.txt");
    let o = run_stdout_closed(&[
        "encode".as_ref(),
        "--in".as_ref(),
        tx.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
    ]);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(0), "stderr:\n{e}");
    assert!(!e.contains(WRITE_LINE_PREFIX), "stderr:\n{e}");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), even_strings());
}
