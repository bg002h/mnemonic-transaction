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

// ── F-250: `-` means stdin, which is already the default ─────────────────────

/// **`mt encode -` must WORK, not error.** `-` meaning *read stdin* is honoured
/// by `cat`, `tar`, `curl`, `gpg` and `jq`, so an operator carrying that habit
/// types it on their first try. `mt` already reads stdin by default, so the
/// intent was satisfied before the argument was parsed — the command failed for
/// asking politely.
///
/// Asserted as EQUALITY with the flagless run, not merely `success()`: a `-`
/// that were silently treated as a filename, or that suppressed the strings,
/// would pass a bare success check.
#[test]
fn a_bare_dash_means_stdin_and_changes_nothing() {
    let v = &corpus()["vectors"][0];
    let raw = v["raw_hex"].as_str().unwrap();

    let plain = mt()
        .args(["encode", "--bitcoin-cli", "/nonexistent/bitcoin-cli"])
        .write_stdin(raw)
        .output()
        .unwrap();
    let dashed = mt()
        .args(["encode", "--bitcoin-cli", "/nonexistent/bitcoin-cli", "-"])
        .write_stdin(raw)
        .output()
        .unwrap();

    assert!(dashed.status.success(), "`mt encode -` must succeed");
    assert_eq!(
        String::from_utf8_lossy(&dashed.stdout),
        String::from_utf8_lossy(&plain.stdout),
        "`-` must change NOTHING about the artifact"
    );
    assert_eq!(
        String::from_utf8_lossy(&dashed.stderr),
        String::from_utf8_lossy(&plain.stderr),
        "`-` must not add or remove a single line of the report"
    );
}

/// It composes with the flag, because an operator who learned `-` uses it
/// everywhere.
#[test]
fn a_bare_dash_composes_with_qr() {
    let v = &corpus()["vectors"][0];
    let raw = v["raw_hex"].as_str().unwrap();
    let a = mt()
        .args([
            "encode",
            "--qr",
            "--bitcoin-cli",
            "/nonexistent/bitcoin-cli",
            "-",
        ])
        .write_stdin(raw)
        .output()
        .unwrap();
    assert!(a.status.success(), "`mt encode --qr -` must succeed");
    assert_eq!(
        String::from_utf8_lossy(&a.stdout),
        format!("tx:{raw}\n"),
        "the record is unchanged by the dash"
    );
}

/// **`-` is the ONLY positional admitted.** Anything else must still be an
/// error, or the §8.2f pre-clap guard would be the only thing standing between
/// a mistyped argument and silent acceptance.
#[test]
fn a_dash_does_not_open_the_door_to_other_positionals() {
    let a = mt()
        .args(["encode", "--bitcoin-cli", "/nonexistent/bitcoin-cli", "wat"])
        .write_stdin("00")
        .output()
        .unwrap();
    assert!(
        !a.status.success(),
        "a stray positional must still be refused"
    );
}

// ── F-248: mt must recognise its OWN output ──────────────────────────────────

/// **`mt encode` fed its own `mt1` strings must say so.**
///
/// The walk reached this by the likeliest route there is: run `mt encode`, see
/// 22 strings, decide to use the SeedHammer, re-run, and paste back the last
/// thing on screen. The refusal was correct and useless — *"input is not a PSBT
/// or a raw transaction (1978 bytes)"* — while `mt` was staring at 22 strings
/// of its own manufacture and holds the recogniser for them.
///
/// `1` is not in the bech32 charset, so it appears only as the HRP separator:
/// counting `mt1` in whitespace-stripped text is an exact count of strings, and
/// this asserts the real number rather than a "looks like" hedge.
#[test]
fn pasting_mt1_strings_back_into_encode_names_them_and_the_right_verb() {
    let v = &corpus()["vectors"][0];
    let strings: Vec<String> = v["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let n = strings.len();
    let pasted = strings.join("\n");

    let a = mt()
        .args(["encode", "--bitcoin-cli", "/nonexistent/bitcoin-cli"])
        .write_stdin(pasted)
        .output()
        .unwrap();
    assert!(!a.status.success(), "still a refusal");
    let err = String::from_utf8_lossy(&a.stderr).to_string();

    assert!(
        err.contains("mt1"),
        "it must NAME what it is looking at: {err}"
    );
    assert!(
        err.contains(&n.to_string()),
        "and count them -- it saw {n} strings: {err}"
    );
    assert!(
        err.contains("mt decode"),
        "and name the verb that turns them back into a transaction: {err}"
    );
    assert!(
        !err.contains("is not a PSBT or a raw transaction"),
        "the generic sniff failure is what made this useless; it must not be \
         the headline when mt can identify the input exactly: {err}"
    );
    assert!(a.stdout.is_empty(), "nothing on stdout");
}

/// The generic refusal must SURVIVE for input that really is unidentifiable —
/// otherwise the fix above would have replaced one blind message with another.
#[test]
fn genuinely_unrecognisable_input_still_gets_the_generic_refusal() {
    let a = mt()
        .args(["encode", "--bitcoin-cli", "/nonexistent/bitcoin-cli"])
        .write_stdin("zzzz not anything at all zzzz")
        .output()
        .unwrap();
    assert!(!a.status.success());
    let err = String::from_utf8_lossy(&a.stderr).to_string();
    assert!(
        err.contains("is not a PSBT or a raw transaction"),
        "unidentifiable input keeps the generic message: {err}"
    );
    assert!(!err.contains("mt decode"), "and must not misdirect: {err}");
}

/// The SAME defect one form over. Since `--qr` exists, `mt encode`'s output can
/// be a `tx:` record, and pasting that back is exactly as likely as pasting the
/// strings. It must not fall through to the generic sniff failure either.
#[test]
fn pasting_a_tx_record_back_into_encode_is_recognised_too() {
    let v = &corpus()["vectors"][0];
    let raw = v["raw_hex"].as_str().unwrap();
    let a = mt()
        .args(["encode", "--bitcoin-cli", "/nonexistent/bitcoin-cli"])
        .write_stdin(format!("tx:{raw}"))
        .output()
        .unwrap();
    assert!(!a.status.success(), "still a refusal");
    let err = String::from_utf8_lossy(&a.stderr).to_string();
    assert!(err.contains("tx:"), "name what it is: {err}");
    assert!(
        !err.contains("is not a PSBT or a raw transaction"),
        "not the blind message: {err}"
    );
    // NEVER echoed: the record body is the bearer transaction itself.
    assert!(
        !err.contains(&raw[..40]),
        "the refusal must not print the transaction back: {err}"
    );
    assert!(a.stdout.is_empty(), "nothing on stdout");
}

// ── P1 row 10 — the `--out` channel (§6b) ────────────────────────────────────
//
// §6b rules `--out FILE`: the artifact goes to a file **created 0600 by the
// shared crate's `write_private`**, never `std::fs::write`. It exists for
// F-244: a shell redirect cannot create a file owner-only, which is why mt's
// world-readable remedies were `umask 077` and `chmod 600` -- remedies that
// existed BECAUSE there was no `--out`.
//
// The ruling also settles a contradiction the spec fold caught: mt's refusal
// said it has no `--out` because "stdout IS the strings, by design (§3b)", and
// §3b does not say that. §3b rules WHICH STREAM carries the artifact, not
// whether a file channel exists.
//
// `--out` is on `encode` ALONE. §6b's reasoning is entirely about the refusal
// mt prints, and that refusal fires from encode; adding the channel to `decode`
// would half-close a hazard while reading as a whole fix.

/// The offline mechanism, as everywhere else: never `PATH`, which is
/// process-global and would silently change neighbouring tests.
#[cfg(unix)]
const OUT_OFFLINE: &[&str] = &["--bitcoin-cli", "/nonexistent/bitcoin-cli"];

#[cfg(unix)]
fn mode_of(p: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

/// **THE GATE, and the PRE-EXISTING TARGET is the whole of it — F-244.**
///
/// `OpenOptions::mode()` binds on CREATE only. An implementation that passes
/// `0o600` to the open and stops leaves an existing 0644 target at 0644 and
/// reports success, and re-running a command is the case an operator actually
/// hits. The crate's `write_private` sets the mode a second time on the OPEN
/// FILE — on the handle rather than on the path, because between two calls that
/// name a file the name can be made to point somewhere else.
///
/// It pins the CONTENTS too: a function that tightened the mode and wrote
/// nothing would satisfy a permissions-only assertion.
#[test]
#[cfg(unix)]
fn out_tightens_a_pre_existing_world_readable_target_to_0600() {
    use std::os::unix::fs::PermissionsExt;
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("strings.txt");

    std::fs::write(&dest, b"stale").unwrap();
    std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        mode_of(&dest),
        0o644,
        "the CONTROL: the target really is 0644 before the run"
    );

    let out = mt()
        .args(["encode"])
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .arg("--out")
        .arg(&dest)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success(), "--out must write the artifact: {err}");
    assert_eq!(
        mode_of(&dest),
        0o600,
        "F-244: `0o600` binds on CREATE, so an implementation that only passes it \
         to OpenOptions leaves this at 0644 and reports success"
    );

    let written = std::fs::read_to_string(&dest).unwrap();
    assert!(
        !written.contains("stale"),
        "a shrinking overwrite must leave no tail of the previous file: {written}"
    );
    let want: Vec<&str> = v["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(
        written.lines().collect::<Vec<_>>(),
        want,
        "the file must hold the artifact, not just a tightened empty file"
    );
    assert!(
        out.stdout.is_empty(),
        "with --out the artifact goes to the FILE and nothing goes to stdout"
    );
}

/// **`--out` SUPPRESSES §8.2h ENTIRELY**, because mt creates the file
/// owner-only and there is no longer a destination it did not choose.
///
/// stdout is a real 0644 file here — the exact destination §8.2h refuses — and
/// the run must still succeed, because nothing is going there.
#[test]
#[cfg(unix)]
fn out_suppresses_the_world_readable_stdout_gate() {
    use std::os::unix::fs::PermissionsExt;
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("artifact.txt");
    let sink = dir.path().join("stdout.txt");
    let handle = std::fs::File::create(&sink).unwrap();
    std::fs::set_permissions(&sink, std::fs::Permissions::from_mode(0o644)).unwrap();

    // `std::process::Command`, not `assert_cmd`: the gate under test is about
    // the MODE OF FD 1, so stdout has to be a real 0644 file.
    let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
        .arg("encode")
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .arg("--out")
        .arg(&dest)
        .stdout(std::process::Stdio::from(handle))
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(
        o.status.success(),
        "stdout's mode is nobody's business once --out is given: {err}"
    );
    assert!(
        !err.contains("8.2h"),
        "§8.2h must not fire with --out: {err}"
    );
    assert_eq!(
        std::fs::metadata(&sink).unwrap().len(),
        0,
        "nothing may reach stdout when --out was given"
    );
    assert_eq!(mode_of(&dest), 0o600, "and the artifact is owner-only");
}

/// `--out` and a shell redirect must produce the **same bytes**.
///
/// Without this, an `--out` that dropped the trailing newline, or joined the
/// strings differently, would satisfy every assertion above — and the operator
/// engraving from the file would be reading a different artifact from the one
/// the pipeline sees.
#[test]
#[cfg(unix)]
fn out_writes_the_same_bytes_the_pipeline_gets() {
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("strings.txt");

    let piped = mt()
        .args(["encode"])
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    assert!(piped.status.success());

    let out = mt()
        .args(["encode"])
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .arg("--out")
        .arg(&dest)
        .output()
        .unwrap();
    assert!(out.status.success());

    assert_eq!(
        std::fs::read(&dest).unwrap(),
        piped.stdout,
        "--out must write byte-for-byte what the pipeline receives"
    );
}

/// The `--qr` form goes through the same channel, and the record is one line.
#[test]
#[cfg(unix)]
fn out_carries_the_qr_record_too() {
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("record.txt");

    let out = mt()
        .args(["encode", "--qr"])
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .arg("--out")
        .arg(&dest)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_to_string(&dest).unwrap();
    assert_eq!(mode_of(&dest), 0o600);
    assert_eq!(written.lines().count(), 1, "one tx: record: {written}");
    assert!(written.starts_with("tx:"), "{written}");
}

/// **The operator is told WHERE the artifact went, and that mt made it 0600.**
///
/// The `stdout is not a terminal` warning cannot say this: with `--out` the
/// artifact goes to a file whether or not stdout is a terminal, so the sentence
/// that block prints would be false about the run that just happened.
#[test]
#[cfg(unix)]
fn out_names_the_file_it_wrote_and_the_mode_it_made_it() {
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("strings.txt");

    let out = mt()
        .args(["encode"])
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .arg("--out")
        .arg(&dest)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success(), "{err}");
    assert!(
        err.contains(&dest.display().to_string()),
        "the artifact left in a file mt named on the command line, and the \
         operator must be told which: {err}"
    );
    assert!(
        err.contains("0600"),
        "and that mt created it owner-only, which is the whole reason --out \
         exists: {err}"
    );
    assert!(
        !err.contains("stdout is not a terminal"),
        "that sentence is about a REDIRECT, and this run did not have one: {err}"
    );
    assert!(
        err.contains("shred -u"),
        "the destroy-it-afterwards advice is the same either way, and must not \
         have been lost with the sentence that no longer applies: {err}"
    );
}

/// **The §8.2h remedy names `--out` first.** The whole reason the ruling was
/// made is that a shell redirect cannot create a file 0600; a refusal that
/// offers only `umask` and `chmod` is a refusal written for a tool without this
/// channel.
#[test]
#[cfg(unix)]
fn the_world_readable_remedy_offers_out_before_the_shells_workarounds() {
    use std::os::unix::fs::PermissionsExt;
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let f = tmp_with(v["raw_hex"].as_str().unwrap().as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let sink = dir.path().join("out.txt");
    let handle = std::fs::File::create(&sink).unwrap();
    std::fs::set_permissions(&sink, std::fs::Permissions::from_mode(0o644)).unwrap();

    let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
        .arg("encode")
        .args(OUT_OFFLINE)
        .arg("--in")
        .arg(f.path())
        .stdout(std::process::Stdio::from(handle))
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    assert!(!o.status.success(), "§8.2h still refuses a 0644 stdout");
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(err.contains("--out"), "the remedy must name --out: {err}");
    assert!(
        !err.contains("mt has no --out"),
        "the sentence that said mt has no --out is retired with the ruling that \
         gave it one: {err}"
    );
    assert!(
        !err.contains("by design (§3b)"),
        "and so is the §3b citation it leaned on -- §3b rules WHICH STREAM \
         carries the artifact, not whether a file channel exists. **Only that \
         PHRASE is forbidden, not the reference**: the legend block on the same \
         stderr cites §3b legitimately, for the plate layout, and a bare \
         `!contains(\"§3b\")` fails on it -- measured, this assertion did: {err}"
    );
    // The workarounds stay: an operator who cannot change the command line
    // still needs them.
    assert!(err.contains("umask 077"), "{err}");
    assert!(err.contains("--allow-world-readable"), "{err}");
}
