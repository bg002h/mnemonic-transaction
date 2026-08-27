//! §8 in full — one test per refusal, and the list they are checked against.
//!
//! **Every fixture here is CLEAN IN ALL OTHER RESPECTS**, so it trips exactly
//! one refusal. Several would otherwise trip two — an oversized transaction is
//! also value-blind, an unsigned one is also unfinalized — and a fixture
//! tripping the *wrong* refusal still passes a naive test that only checks the
//! run failed.
//!
//! The base material in `fixtures/p5_base.json` comes from a real regtest node;
//! the **defect** each refusal needs is introduced here, one field at a time, so
//! "it trips exactly one" is visible in the source rather than buried in a blob.
//!
//! **These tests being green proves nothing on its own.** A refusal test that
//! passes against code with the check deleted is testing nothing, and this
//! constellation has paid for that twice. `scripts/mutate-refusals.sh` neuters
//! each named check and asserts its test goes red; `scripts/check-refusal-coverage.sh`
//! asserts this file and `refusals.toml` are a bijection.

use assert_cmd::Command;
use bitcoin::consensus::{deserialize, serialize};
use std::io::Write;

mod common;
use common::{fixture_txids, node_stub};

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

/// The offline mechanism. Never `PATH`: that is process-global and would
/// silently change neighbouring tests in the same run.
const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn base() -> serde_json::Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v[k].as_str()
        .unwrap_or_else(|| panic!("fixture {k} missing"))
        .to_string()
}

/// Write a fixture to a file **mode 0600**, so §8.2g's warning does not fire and
/// pollute every assertion in this file with output it did not ask for.
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

fn b64_decode(s: &str) -> Vec<u8> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let (mut acc, mut bits, mut out) = (0u32, 0u32, Vec::new());
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace()) {
        if c == b'=' {
            break;
        }
        acc = (acc << 6) | T.iter().position(|&t| t == c).unwrap() as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    out
}

fn b64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[((n >> (18 - 6 * i)) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn hex_of(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Run `mt encode` on a fixture file, offline, and return `(stdout, stderr, ok)`.
fn encode(body: &str, extra: &[&str]) -> (String, String, bool) {
    let f = tmp(body);
    let mut c = mt();
    c.args(["encode", "--bitcoin-cli", OFFLINE]);
    c.args(extra);
    let out = c.arg("--in").arg(f.path()).output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// Assert a refusal fired, named its section, and produced NO artifact.
///
/// **The stdout clause is not incidental.** §1.1's documented pipeline pipes
/// `mt`'s stdout onward, so a refusal that still printed strings would let a
/// transaction that failed `mt`'s own checks reach the steel anyway.
fn assert_refused(stdout: &str, stderr: &str, ok: bool, section: &str) {
    assert!(
        !ok,
        "the run SUCCEEDED; expected a {section} refusal.\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("REFUSED — {section},")),
        "expected a {section} refusal, got:\n{stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "a refusal still wrote to stdout — the pipeline would engrave it:\n{stdout}"
    );
}

// ── §8.1 — not fully finalized, by the PSBT's own vocabulary ─────────────────

#[test]
fn refuses_an_unfinalized_psbt() {
    let v = base();
    let (out, err, ok) = encode(&s(&v, "unfinalized_psbt_b64"), &[]);
    assert_refused(&out, &err, ok, "§8.1");
    assert!(
        err.contains("PSBT_IN_FINAL_SCRIPTSIG") || err.contains("PSBT_IN_FINAL_SCRIPTWITNESS"),
        "§8.1 must refuse in the PSBT's OWN vocabulary: {err}"
    );
    // §8's closing line: every refusal names the number that caused it.
    assert!(err.contains("2 of 2 inputs"), "the count is missing: {err}");
}

/// The control that makes the one above mean something: the SAME transaction,
/// finalized, encodes cleanly. Without it, a fixture broken in some unrelated
/// way would still pass.
#[test]
fn the_same_transaction_finalized_encodes() {
    let v = base();
    let (out, err, ok) = encode(&s(&v, "finalized_psbt_b64"), &[]);
    assert!(ok, "the clean base fixture does not encode: {err}");
    assert!(out.lines().count() > 1, "no strings on stdout");
    assert!(out.lines().all(|l| l.starts_with("mt1")));
}

// ── §8.2b — value-blind acceptance ──────────────────────────────────────────

#[test]
fn refuses_an_absurd_fee_rate() {
    // The RAW form plus an operator-asserted value: a huge input against the
    // real outputs. Nothing else about it is wrong -- it is the same signed
    // transaction the control encodes.
    let v = base();
    let (out, err, ok) = encode(
        &s(&v, "raw_hex"),
        &["--input-value", "0:100.0", "--input-value", "1:5.0"],
    );
    assert_refused(&out, &err, ok, "§8.2b");
    assert!(err.contains("25,000"), "the ceiling is not named: {err}");
    assert!(
        err.contains("sat/vB"),
        "§8.2b's verdict must carry the RATE that caused it: {err}"
    );
}

#[test]
fn refuses_outputs_that_exceed_inputs() {
    let v = base();
    let (out, err, ok) = encode(
        &s(&v, "raw_hex"),
        &["--input-value", "0:0.001", "--input-value", "1:0.001"],
    );
    assert_refused(&out, &err, ok, "§8.2b");
    assert!(
        err.contains("spends more than it takes in"),
        "expected SendingTooMuch, got: {err}"
    );
}

/// Two inputs naming one outpoint. Only one can ever be spent, so the
/// transaction is dead on arrival however well-formed it looks — and every
/// other refusal in §8 passes it.
#[test]
fn refuses_a_duplicate_outpoint() {
    let v = base();
    let mut tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    tx.input[1].previous_output = tx.input[0].previous_output;
    let (out, err, ok) = encode(&hex_of(&serialize(&tx)), &[]);
    assert_refused(&out, &err, ok, "§8.2b");
    assert!(err.contains("repeats outpoint"), "got: {err}");
}

/// An empty `vin` spends nothing. `rust-bitcoin`'s `verify_transaction` is a
/// per-input loop, so it iterates zero times and returns success.
#[test]
fn refuses_an_empty_vin() {
    let v = base();
    let mut tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    tx.input.clear();
    let (out, err, ok) = encode(&hex_of(&serialize(&tx)), &[]);
    assert_refused(&out, &err, ok, "§8.2b");
    assert!(err.contains("0 inputs"), "got: {err}");
}

/// **No minimum fee — a WARNING below 10 sat/vB.** A refusal floor would
/// hardcode today's relay policy into an artifact meant to be broadcast in 2040,
/// the same mistake as engraving a dollar figure.
#[test]
fn a_low_fee_warns_and_does_not_refuse() {
    let v = base();
    let tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    let out_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
    // A fee of ~1 sat/vB: just above the outputs.
    let vb = tx.weight().to_vbytes_ceil();
    let claimed = out_total + vb;
    // Split across BOTH inputs: §8.2b's arithmetic is over the TOTAL, and a
    // partially-supplied set leaves it unknown rather than low -- which would
    // make this test pass for the wrong reason.
    let half = sats_to_btc(claimed / 2);
    let rest = sats_to_btc(claimed - claimed / 2);
    let (_, err, ok) = encode(
        &s(&v, "raw_hex"),
        &[
            "--input-value",
            &format!("0:{half}"),
            "--input-value",
            &format!("1:{rest}"),
        ],
    );
    assert!(ok, "a low fee must NOT refuse: {err}");
    assert!(
        err.contains("WARNING: fee rate is"),
        "no low-fee warning: {err}"
    );
    assert!(
        err.contains("CPFP"),
        "the warning must name the way out: {err}"
    );
    assert!(
        !err.contains("REFUSED"),
        "a low fee was refused; §8.2b rules a warning: {err}"
    );
}

// ── §8.2d — non_witness_utxo must hash to the input's txid ──────────────────

#[test]
fn refuses_a_non_witness_utxo_that_does_not_hash_to_its_input() {
    let v = base();
    let mut psbt = bitcoin::Psbt::deserialize(&b64_decode(&s(&v, "finalized_psbt_b64"))).unwrap();
    // ONE field, and one that keeps the record a well-formed transaction: bump
    // the previous transaction's locktime. Its txid changes; nothing else does.
    let prev = psbt.inputs[0]
        .non_witness_utxo
        .as_mut()
        .expect("fixture has no non_witness_utxo");
    prev.lock_time = bitcoin::absolute::LockTime::from_consensus(
        prev.lock_time.to_consensus_u32().wrapping_add(1),
    );

    let (out, err, ok) = encode(&b64_encode(&psbt.serialize()), &[]);
    assert_refused(&out, &err, ok, "§8.2d");
    // "A mismatch is a refusal naming BOTH txids."
    let want = psbt.unsigned_tx.input[0].previous_output.txid.to_string();
    let got = psbt.inputs[0]
        .non_witness_utxo
        .as_ref()
        .unwrap()
        .compute_txid()
        .to_string();
    assert!(err.contains(&want), "the input's txid is missing: {err}");
    assert!(
        err.contains(&got),
        "the record's own hash is missing: {err}"
    );
}

/// The binding is what §8.6 leans on when it accepts legacy inputs, so the
/// UNMUTATED record must read as **verified** — not merely "not refused".
#[test]
fn a_matching_non_witness_utxo_is_reported_as_txid_bound() {
    let v = base();
    let (_, err, ok) = encode(&s(&v, "finalized_psbt_b64"), &[]);
    assert!(ok, "{err}");
    assert!(
        err.contains("TXID-BOUND"),
        "a hash-verified value must not render as claimed: {err}"
    );
    assert!(
        !err.contains("CLAIMED — no input value verified"),
        "the FEE was marked claimed although every input is txid-bound: {err}"
    );
}

/// **The middle column.** A PSBT carrying only `witness_utxo`, air-gapped: §6a
/// cannot compare it, §8.2d cannot bind it, and nothing else looks. It must not
/// render in the verified column — R6 adversarial I-5.
#[test]
fn a_witness_utxo_alone_is_reported_as_claimed() {
    let v = base();
    let mut psbt = bitcoin::Psbt::deserialize(&b64_decode(&s(&v, "finalized_psbt_b64"))).unwrap();
    for i in &mut psbt.inputs {
        i.non_witness_utxo = None;
    }
    let (_, err, ok) = encode(&b64_encode(&psbt.serialize()), &[]);
    assert!(ok, "a witness_utxo-only PSBT must still encode: {err}");
    assert!(
        err.contains("PSBT-CLAIMED — unverified"),
        "an unchecked value rendered as verified: {err}"
    );
    assert!(
        err.contains("CLAIMED — no input value verified"),
        "the FEE inherits the weakest provenance of any input: {err}"
    );
}

// ── §8.2e — nothing matched ─────────────────────────────────────────────────

#[test]
fn refuses_unrecognised_input_naming_what_was_seen() {
    let (out, err, ok) = encode("this is not a transaction at all", &[]);
    assert_refused(&out, &err, ok, "§8.2e");
    // "naming what was seen (first 8 bytes as hex, and the detected length),
    // never a bare 'invalid input'."
    assert!(
        err.contains("74 68 69 73"),
        "the bytes are not shown: {err}"
    );
    assert!(err.contains("32 bytes"), "the length is not shown: {err}");
}

// ── §8.2f — a bearer artifact on the command line ───────────────────────────

#[test]
fn refuses_a_transaction_passed_as_a_command_line_argument() {
    let v = base();
    let f = tmp(&s(&v, "finalized_psbt_b64"));
    let out = mt()
        .args([
            "encode",
            "--bitcoin-cli",
            OFFLINE,
            "--to-label",
            &s(&v, "raw_hex"),
        ])
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_refused(
        &String::from_utf8_lossy(&out.stdout),
        &err,
        out.status.success(),
        "§8.2f",
    );
    // THE REFUSAL MUST NOT ECHO IT. Printing the argument back would put the
    // bearer material in a second place -- the defect the refusal exists to name.
    let raw = s(&v, "raw_hex");
    assert!(
        !err.contains(&raw[..64]),
        "the refusal echoed the transaction it was refusing:\n{err}"
    );
    assert!(
        err.contains("shell history"),
        "the leak is not named: {err}"
    );
    assert!(err.contains("ps"), "the other leak is not named: {err}");
}

/// The narrowness is load-bearing in the other direction: an ordinary label must
/// not be mistaken for a transaction, or §8.2f refuses valid invocations.
#[test]
fn an_ordinary_label_is_not_mistaken_for_a_transaction() {
    let v = base();
    let (_, err, ok) = encode(
        &s(&v, "finalized_psbt_b64"),
        &["--to-label", "cold storage, safe deposit box 12"],
    );
    assert!(ok, "a plain label tripped §8.2f: {err}");
}

/// **F-274 — the guard must NORMALISE before it CLASSIFIES.**
///
/// `looks_like_a_transaction` lowercases for its `mt1` arm and never trims. A
/// bearer artifact pasted with a stray space is therefore unrecognised, falls
/// through to clap, and **clap echoes it verbatim to stderr** — the exact leak
/// §8.2f exists to prevent, reached by the likeliest route there is. Copying a
/// string off a terminal, out of a note, or from a chat window brings the
/// whitespace with it.
///
/// **A GENERATED CROSS-PRODUCT, not a hand list:** 4 verbs × 2 carrier classes
/// (an `mt1` string, a raw transaction) × 4 spellings (canonical,
/// leading-space, trailing-space, UPPERCASE) = **32 rows**. Measured before the
/// fix, exit codes read directly: **16 leaked** — both whitespace spellings, on
/// every verb, for both classes, each at exit 2 with the material in clap's
/// error.
///
/// The canonical and UPPERCASE rows are the **positive control**. They were
/// already caught, so without them a "fix" that refused every argument would be
/// indistinguishable from one that recognises the carrier — and the narrowness
/// is what `a_filename_beginning_mt1_is_not_mistaken_for_a_bearer_string`
/// guards from the other side.
#[test]
fn no_spelling_of_a_bearer_argument_reaches_stderr() {
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mt1 = corpus["vectors"][0]["strings"][0].as_str().unwrap();
    let raw = corpus["vectors"][0]["raw_hex"].as_str().unwrap();

    let mut rows = 0;
    for verb in ["encode", "decode", "verify", "inspect"] {
        for (class, carrier) in [("an mt1 string", mt1), ("a raw transaction", raw)] {
            for (spelling, arg) in [
                ("canonical", carrier.to_string()),
                ("leading-space", format!(" {carrier}")),
                ("trailing-space", format!("{carrier} ")),
                ("UPPERCASE", carrier.to_uppercase()),
            ] {
                rows += 1;
                let out = mt()
                    .args([verb, "--bitcoin-cli", OFFLINE])
                    .arg(&arg)
                    .output()
                    .unwrap();
                let err = String::from_utf8_lossy(&out.stderr).into_owned();
                let where_ = format!("{verb} / {class} / {spelling}");

                // The material never reaches stderr, by ANY route -- the
                // refusal's own text, or clap's echo of an argument the guard
                // failed to recognise.
                assert!(
                    !err.contains(arg.trim()),
                    "{where_}: the bearer material reached stderr\n{err}"
                );
                // ...and the guard is what stopped it. Absence alone would also
                // be satisfied by a tool that accepted the argument in silence.
                assert!(
                    err.contains("§8.2f"),
                    "{where_}: no §8.2f refusal; exit {:?}\n{err}",
                    out.status.code()
                );
                assert_eq!(
                    out.status.code(),
                    Some(1),
                    "{where_}: §8.2f is an mt refusal, so exit 1 and not clap's 2"
                );
            }
        }
    }
    assert_eq!(
        rows, 32,
        "the cross-product is 4 verbs x 2 classes x 4 spellings"
    );
}

// ── §6d: `--allow-argv-secret` is a CHANNEL, not a flag ─────────────────────
//
// The override's own parse runs on RAW argv, and so does the ROUTING of what it
// admits. Wiring it as an ordinary clap flag moves the decision after clap and
// reinstates the leak §8.2f exists to stop -- and on `mt` it is worse than on
// `me`, because NO mt verb takes material positionally: `me sysw pack` has a
// `records` positional to hand the admitted token to, and mt has nothing but a
// hidden `[-]` whose value_parser rejects everything else. So an override that
// leaves the token in argv converts a clean exit-1 refusal into clap's
// `error: invalid value '<the whole transaction>' for '[-]'` at exit 2.

fn corpus_vector() -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap()["vectors"][0]
        .clone()
}

/// **The admitted material takes the `--in` path, and the proof is byte
/// equality with `--in` itself.**
///
/// §6d: *"admitted material is passed to the tool through the same internal
/// path as `--in` content, and never re-presented to clap as a positional"*.
/// Byte equality on stdout is what makes "the same path" checkable — a success
/// check would pass for an override that admitted the material and then read an
/// empty stdin.
///
/// All four verbs, because `encode` and the reading verbs reach the bytes
/// through different functions: `encode`'s own `--in` arm, and `read_input`,
/// which `decode`, `verify` and `inspect` share.
#[test]
fn the_argv_override_routes_material_through_the_private_path() {
    let v = corpus_vector();
    let raw = v["raw_hex"].as_str().unwrap().to_string();
    let strings: Vec<String> = v["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();

    // `encode` takes ONE token: a raw transaction.
    // The reading verbs take SIX: the strings of a set, as separate argv words.
    let cases: Vec<(&str, Vec<String>, String)> = vec![
        ("encode", vec![raw.clone()], raw.clone()),
        ("decode", strings.clone(), strings.join("\n")),
        ("verify", strings.clone(), strings.join("\n")),
        ("inspect", strings.clone(), strings.join("\n")),
    ];

    for (verb, tokens, file_body) in cases {
        let f = tmp(&file_body);
        let via_in = mt()
            .args([verb, "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            via_in.status.success(),
            "{verb}: the --in control must succeed, or the comparison is \
             between two failures\n{}",
            String::from_utf8_lossy(&via_in.stderr)
        );

        let via_argv = mt()
            .args([verb, "--bitcoin-cli", OFFLINE, "--allow-argv-secret"])
            .args(&tokens)
            .output()
            .unwrap();

        assert_eq!(
            via_argv.status.code(),
            Some(0),
            "`mt {verb} --allow-argv-secret …` must PROCEED\n{}",
            String::from_utf8_lossy(&via_argv.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&via_argv.stdout),
            String::from_utf8_lossy(&via_in.stdout),
            "{verb}: the admitted material did not take the --in path"
        );
    }
}

/// **The material must be GONE from the argv clap sees, not merely permitted.**
///
/// The discriminating case, and the ORDER of the tokens is the whole test: the
/// unknown flag comes AFTER the material, so an implementation that leaves the
/// admitted token in argv makes clap reach the `[-]` positional FIRST and print
/// `error: invalid value '<the whole transaction>' for '[-]'`. Put the unknown
/// flag first and clap errors on it before it ever reaches the value, so the
/// naive implementation passes too — and a test that both worlds satisfy is not
/// a test.
#[test]
fn the_argv_override_strips_the_material_from_the_argv_clap_sees() {
    let raw = corpus_vector()["raw_hex"].as_str().unwrap().to_string();
    let out = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--allow-argv-secret"])
        .arg(&raw)
        .arg("--nosuchflag")
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        !err.contains(&raw),
        "the admitted material was handed back to clap and echoed:\n{err}"
    );
    assert!(
        err.contains("--nosuchflag"),
        "clap must name the flag it could not parse:\n{err}"
    );
}

/// **The control: the override on its own changes nothing.**
///
/// `mt encode --allow-argv-secret` with no material must behave exactly as
/// `mt encode` — both streams, and the exit code. Without this, an
/// implementation that swallowed argv wholesale would still pass the two tests
/// above.
#[test]
fn the_argv_override_alone_is_the_bare_invocation() {
    let raw = corpus_vector()["raw_hex"].as_str().unwrap().to_string();
    let plain = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE])
        .write_stdin(raw.clone())
        .output()
        .unwrap();
    let flagged = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--allow-argv-secret"])
        .write_stdin(raw)
        .output()
        .unwrap();
    assert_eq!(flagged.status.code(), plain.status.code());
    assert_eq!(
        String::from_utf8_lossy(&flagged.stdout),
        String::from_utf8_lossy(&plain.stdout)
    );
    assert_eq!(
        String::from_utf8_lossy(&flagged.stderr),
        String::from_utf8_lossy(&plain.stderr)
    );
}

/// **Two sources for one channel is WARNED about, never silent.**
///
/// `--in FILE` and admitted argv material both offer the bytes. The file wins —
/// it is the private channel and the explicit one — but an operator whose typed
/// argument was discarded in silence would have no way to know which of the two
/// mt engraved. The warning names the length and the file, and never the
/// material.
#[test]
fn material_on_argv_beside_an_in_file_is_warned_about_not_dropped() {
    let v = corpus_vector();
    let raw = v["raw_hex"].as_str().unwrap().to_string();
    let other = v["raw_hex"].as_str().unwrap().to_string();
    let f = tmp(&other);

    let out = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--allow-argv-secret"])
        .arg(&raw)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{err}");
    assert!(
        err.contains("--in was given too"),
        "the discarded argv material was not mentioned:\n{err}"
    );
    assert!(
        !err.contains(&raw),
        "the warning echoed the material:\n{err}"
    );
    assert!(
        err.contains(&f.path().display().to_string()),
        "the warning does not say WHICH source was read:\n{err}"
    );
}

/// **It is documented on every verb**, because a flag an operator cannot find
/// is one they cannot decide about. §6d makes it greppable in a script so a
/// reviewer can find it; `--help` is where the operator finds it.
#[test]
fn the_argv_override_is_documented_on_every_verb() {
    for verb in ["encode", "decode", "verify", "inspect"] {
        let out = mt().args([verb, "--help"]).output().unwrap();
        let help = String::from_utf8_lossy(&out.stdout);
        assert!(
            help.contains("--allow-argv-secret"),
            "`mt {verb} --help` does not document the override:\n{help}"
        );
    }
}

// ── §8.3 — unsigned ─────────────────────────────────────────────────────────

#[test]
fn refuses_an_unsigned_raw_transaction() {
    let v = base();
    let (out, err, ok) = encode(&s(&v, "unsigned_raw_hex"), &[]);
    assert_refused(&out, &err, ok, "§8.3");
    assert!(
        err.contains("empty scriptSig") && err.contains("empty witness"),
        "§8.3 must refuse in the RAW transaction's own vocabulary: {err}"
    );
}

// ── §8.5 / §6a — what a node says ───────────────────────────────────────────

fn encode_with_node(body: &str, node: &std::path::Path, extra: &[&str]) -> (String, String, bool) {
    let f = tmp(body);
    let mut c = mt();
    c.args(["encode", "--bitcoin-cli"]).arg(node);
    c.args(extra);
    let out = c.arg("--in").arg(f.path()).output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

#[test]
fn refuses_a_spent_input_whose_parent_confirmed() {
    let v = base();
    // gettxout null AND the parent confirmed: the output was spent or never
    // existed. BOTH facts are required.
    // The THEFT case: the parents are confirmed, and THIS transaction is NOT
    // on chain -- so the outputs were taken by somebody else.
    let (own, parents) = fixture_txids(&v);
    let conf: Vec<(&str, u32)> = parents.iter().map(|p| (&p[..16], 6u32)).collect();
    assert!(
        !conf.iter().any(|(t, _)| own.starts_with(t)),
        "the fixture spends its own output; this stub could not model theft"
    );
    let stub = node_stub("", &conf);
    let (out, err, ok) = encode_with_node(&s(&v, "raw_hex"), stub.path(), &[]);
    assert_refused(&out, &err, ok, "§8.5");
    assert!(
        err.contains("not in the UTXO set"),
        "§8.5 must say what it found: {err}"
    );
}

/// **The clause that separates a true refusal from a false one.** With the
/// parent unconfirmed, `null` is the EXPECTED answer — `include_mempool` is
/// false by ruling — so refusing would state something untrue inside a refusal.
#[test]
fn an_unconfirmed_parent_is_not_a_spent_input() {
    let v = base();
    let (_own, parents) = fixture_txids(&v);
    // found, 0 confirmations: the parent is in the mempool
    let conf: Vec<(&str, u32)> = parents.iter().map(|p| (&p[..16], 0u32)).collect();
    let stub = node_stub("", &conf);
    let (_, err, ok) = encode_with_node(&s(&v, "raw_hex"), stub.path(), &[]);
    assert!(
        ok,
        "a mempool-only parent was refused; §8.5 requires the parent CONFIRMED:\n{err}"
    );
    assert!(!err.contains("REFUSED"), "{err}");
}

#[test]
fn refuses_a_value_that_disagrees_with_the_chain() {
    let v = base();
    // The chain says 3.0 BTC; the operator asserts 4.0. mt cannot tell which is
    // wrong, so it refuses -- naming BOTH numbers.
    let stub = node_stub(r#"{"value": 3.00000000, "scriptPubKey": {}}"#, &[]);
    let (out, err, ok) = encode_with_node(
        &s(&v, "raw_hex"),
        stub.path(),
        &["--input-value", "0:4.0", "--input-value", "1:4.0"],
    );
    assert_refused(&out, &err, ok, "§6a");
    assert!(
        err.contains("4.00000000 BTC"),
        "the claim is missing: {err}"
    );
    assert!(
        err.contains("3.00000000 BTC"),
        "the chain's answer is missing: {err}"
    );
}

/// The same shape, agreeing. Without this the test above would pass against
/// code that refuses unconditionally.
#[test]
fn a_value_that_agrees_with_the_chain_is_accepted() {
    let v = base();
    let stub = node_stub(r#"{"value": 4.00000000, "scriptPubKey": {}}"#, &[]);
    let (_, err, ok) = encode_with_node(
        &s(&v, "raw_hex"),
        stub.path(),
        &["--input-value", "0:4.0", "--input-value", "1:4.0"],
    );
    assert!(ok, "an agreeing value was refused: {err}");
}

// ── §8.6 — the satisfaction must bind the outputs ───────────────────────────

#[test]
fn refuses_a_signature_that_is_not_sighash_all() {
    let v = base();
    let (out, err, ok) = encode(&s(&v, "sighash_none_psbt_b64"), &[]);
    assert_refused(&out, &err, ok, "§8.6");
    assert!(err.contains("0x02"), "the flag is not named: {err}");
    assert!(
        err.contains("SIGHASH_NONE"),
        "the flag is not spelled: {err}"
    );
    assert!(
        err.contains("UNBOUND"),
        "the consequence is not stated: {err}"
    );
}

#[test]
fn refuses_a_satisfaction_carrying_no_signature() {
    let v = base();
    let mut tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    // A keyless taproot leaf spent at depth 1: preimage, leaf script, control
    // block. NOTHING here commits to an output, so any holder can rewrite them
    // all and satisfy it again.
    tx.input[0].witness = bitcoin::Witness::from_slice(&[
        vec![0xab; 32],  // the preimage
        vec![0xcd; 143], // the leaf script
        control_block(1),
    ]);
    let (out, err, ok) = encode(&hex_of(&serialize(&tx)), &[]);
    assert_refused(&out, &err, ok, "§8.6");
    assert!(
        err.contains("no signature"),
        "expected §8.6(b), got what looks like (a): {err}"
    );
}

/// **The recognizer is grindable, and this is the grind.** A BIP-341 control
/// block at depth 1 is `33 + 32` = **65 bytes**, and so is a Schnorr signature
/// carrying an explicit sighash byte — they are indistinguishable by length. A
/// length-based recognizer counts the control block as the signature it is
/// looking for, and §8.6(b) passes an input that commits to nothing.
#[test]
fn a_control_block_is_not_counted_as_a_signature() {
    assert_eq!(
        control_block(1).len(),
        65,
        "the collision this test is about"
    );
    let v = base();
    let mut tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    tx.input[0].witness =
        bitcoin::Witness::from_slice(&[vec![0xab; 32], vec![0xcd; 143], control_block(1)]);
    let (_, err, ok) = encode(&hex_of(&serialize(&tx)), &[]);
    assert!(
        !ok,
        "the 65-byte control block was counted as a signature:\n{err}"
    );
}

/// A BIP-341 control block: leaf version byte, 32-byte internal key, and `m`
/// 32-byte merkle branch entries.
fn control_block(m: usize) -> Vec<u8> {
    let mut v = vec![0xc0u8];
    v.extend(std::iter::repeat_n(0x11u8, 32 + 32 * m));
    v
}

// ── §8.7b — the chunk ceiling ───────────────────────────────────────────────

/// **SYNTHESISED AT TEST TIME, not committed.** Exceeding 32,768 chunks needs
/// more than 1,310,720 payload bytes, so the fixture is one signed input and
/// many outputs — built here rather than carried as a ~1.3 MB blob in git.
#[test]
fn refuses_over_the_chunk_ceiling() {
    let v = base();
    let mut tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    let out0 = tx.output[0].clone();
    tx.output = std::iter::repeat_n(out0, 45_000).collect();
    let bytes = serialize(&tx);
    assert!(
        bytes.len() > 32_768 * 40,
        "the synthesised fixture is too small to trip the ceiling: {} bytes",
        bytes.len()
    );

    let (out, err, ok) = encode(&hex_of(&bytes), &[]);
    assert_refused(&out, &err, ok, "§8.7b");
    assert!(err.contains("32,768"), "the ceiling is not named: {err}");
    assert!(
        err.contains(&format!("{}", bytes.len().div_ceil(40)).replace_thousands()),
        "the chunk count is not named: {err}"
    );
}

/// Thousands separators, to match the refusal's own rendering.
trait Thousands {
    fn replace_thousands(&self) -> String;
}
impl Thousands for String {
    fn replace_thousands(&self) -> String {
        let mut out = String::new();
        for (i, c) in self.chars().enumerate() {
            if i > 0 && (self.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out
    }
}

// ── §8.9 — secrets ──────────────────────────────────────────────────────────

/// The `ms1` string here is a **valid-looking codex32 secret share**, because
/// the hazard is about what `mt` PRINTS, not about whether the string decodes.
const MS1: &str = "ms12wxvvw3vc7g9pm7f6h20pjm9pjm9pjm9pjm9pjm9pjm9p6hmun7";

#[test]
fn refuses_ms1_without_echoing_any_of_it() {
    for verb in ["encode", "decode", "verify", "inspect"] {
        let f = tmp(MS1);
        let out = mt().args([verb, "--in"]).arg(f.path()).output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(!out.status.success(), "{verb} accepted ms1");
        assert!(
            err.contains("REFUSED — §8.9,"),
            "{verb}: expected §8.9, got:\n{err}"
        );
        // THE WHOLE POINT. §8.2e's step-4 refusal names the first eight bytes so
        // an operator can see what mt thought it received -- and for ms1 those
        // bytes are SECRET SEED ENTROPY. §8.9 must run FIRST, and its own
        // message must not echo either.
        assert!(
            !err.contains(&MS1[3..11]),
            "{verb}: the refusal echoed the secret's body:\n{err}"
        );
        assert!(
            !err.contains("6d 73 31"),
            "{verb}: §8.2e's byte-naming ran on a secret:\n{err}"
        );
        assert!(
            out.stdout.is_empty(),
            "{verb}: a secret refusal wrote to stdout"
        );
    }
}

// ── shared ──────────────────────────────────────────────────────────────────

fn hex_to_bytes(s: &str) -> Vec<u8> {
    s.trim()
        .as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

fn sats_to_btc(sats: u64) -> String {
    format!("{}.{:08}", sats / 100_000_000, sats % 100_000_000)
}

/// **The success case must not be reported as the theft case.**
///
/// Every input of a confirmed transaction is spent — *by itself* — and every
/// parent is confirmed, which is bit-for-bit §8.5's condition. Without asking
/// whether THIS transaction is on chain, `encode` refused a transaction that had
/// already paid, told the operator it *"can never be broadcast"*, and advised
/// them to *"build a new transaction"* — which is advice to pay twice.
///
/// Found by running against a real regtest node. No offline or stubbed test
/// could see it: all three §8.5 situations share `gettxout -> null` and differ
/// only in what `getrawtransaction` says about a txid the old stub was never
/// asked about.
#[test]
fn an_already_confirmed_transaction_is_not_reported_as_stolen() {
    let v = base();
    let (own, parents) = fixture_txids(&v);
    // The transaction itself IS on chain, and so are its parents — exactly what
    // a node reports the moment a payment confirms.
    let mut conf: Vec<(&str, u32)> = vec![(&own[..16], 3)];
    conf.extend(parents.iter().map(|p| (&p[..16], 9u32)));
    let stub = node_stub("", &conf);

    let (out, err, ok) = encode_with_node(&s(&v, "finalized_psbt_b64"), stub.path(), &[]);
    assert!(
        ok,
        "a CONFIRMED transaction was refused as though someone had stolen its inputs:\n{err}"
    );
    assert!(
        !err.contains("can never be broadcast"),
        "mt said a transaction in a block can never be broadcast:\n{err}"
    );
    assert!(
        !err.contains("Build a new transaction"),
        "mt advised rebuilding a payment that already went through — pay twice:\n{err}"
    );
    assert!(
        err.contains("ALREADY CONFIRMED"),
        "the report must SAY the transaction confirmed, not stay silent:\n{err}"
    );
    assert!(!out.trim().is_empty(), "the strings were still produced");
}

/// **Reaching a node must never make the report WORSE.**
///
/// `include_mempool` is `false` by ruling, so `gettxout -> null` is the EXPECTED
/// answer for an unconfirmed parent — a lookup that did not find the outpoint,
/// not evidence that its value is unknown. Discarding the PSBT's txid-bound
/// record there meant the same file showed its fee offline and `UNKNOWN` with a
/// node, and §1.1's row table makes `FEE` present *"when a node is reachable
/// **or** the input was a PSBT carrying values"* — an OR the code had
/// implemented as an either/or.
#[test]
fn a_node_that_cannot_find_an_outpoint_does_not_discard_the_psbt_record() {
    let v = base();
    let (_own, parents) = fixture_txids(&v);
    // Parents in the mempool: gettxout returns null, and nothing is spent.
    let conf: Vec<(&str, u32)> = parents.iter().map(|p| (&p[..16], 0u32)).collect();
    let stub = node_stub("", &conf);

    let (_, with_node, ok) = encode_with_node(&s(&v, "finalized_psbt_b64"), stub.path(), &[]);
    assert!(ok, "a mempool parent must not refuse: {with_node}");
    let (_, offline, _) = encode(&s(&v, "finalized_psbt_b64"), &[]);

    for (label, err) in [("offline", &offline), ("with a node", &with_node)] {
        assert!(
            err.contains("FEE       0.00100000 BTC"),
            "{label}: the fee is missing, though the PSBT carries every input value:\n{err}"
        );
        assert!(
            err.contains("TXID-BOUND"),
            "{label}: a record §8.2d verified was thrown away:\n{err}"
        );
    }
    // ...and the row a node CAN improve is the one that improved.
    assert!(
        with_node.contains("STATUS    PENDING"),
        "the node's own contribution is missing:\n{with_node}"
    );
    assert!(offline.contains("STATUS    UNKNOWN"), "{offline}");
}

// ── §8.2c's amount parsing ──────────────────────────────────────────────────

/// **Typing what a person mistypes.** `--input-value 0:inf` PANICKED, and
/// `0:1e30` panicked with it — the amount went through `f64` and then a
/// saturating cast. `-5` and `NaN` did not panic, which was worse: they produced
/// a silent nonsense value that tripped §8.2b, so the operator got a refusal
/// about their OUTPUTS when the fault was in what they had just typed.
#[test]
fn a_hostile_input_value_is_refused_by_name_and_never_panics() {
    let v = base();
    for amount in [
        "inf",
        "-5",
        "NaN",
        "1e30",
        "1.234567891",
        "21000001",
        "",
        "0x5",
    ] {
        let (out, err, ok) = encode(
            &s(&v, "raw_hex"),
            &[
                "--input-value",
                &format!("0:{amount}"),
                "--input-value",
                "1:5.0",
            ],
        );
        assert!(!ok, "{amount:?} was accepted");
        assert!(
            !err.contains("panicked"),
            "{amount:?} PANICKED instead of refusing:\n{err}"
        );
        assert!(
            err.contains("REFUSED — §8.2c,"),
            "{amount:?} was refused by the wrong rule — the fault is the amount, \
             not the transaction:\n{err}"
        );
        assert!(out.trim().is_empty());
    }
}

/// The control: amounts a person actually types must pass the parser and reach
/// §8.2b's arithmetic. Without this, refusing everything would pass the test
/// above.
#[test]
fn ordinary_amounts_parse() {
    let v = base();
    for amount in ["5", "5.0", "0.05000000", "3.00000001", "21000000"] {
        let (_, err, _) = encode(
            &s(&v, "raw_hex"),
            &[
                "--input-value",
                &format!("0:{amount}"),
                "--input-value",
                "1:5.0",
            ],
        );
        assert!(
            !err.contains("REFUSED — §8.2c,"),
            "{amount:?} is a legitimate amount and was refused:\n{err}"
        );
    }
}

/// **§8.2d hashes the record; it did not check the record describes THIS
/// outpoint.** A `non_witness_utxo` whose hash matches but which has no output
/// at the input's `vout` does not describe the output being spent — the value
/// then comes from `witness_utxo`, which nothing has checked, while the row said
/// `TXID-BOUND`. An unverified number under a verified heading.
#[test]
fn a_record_with_no_output_at_the_inputs_vout_is_not_txid_bound() {
    let v = base();
    let mut psbt = bitcoin::Psbt::deserialize(&b64_decode(&s(&v, "finalized_psbt_b64"))).unwrap();
    // Point the input at a vout the previous transaction does not have, and
    // rebuild the record so its hash still matches the new outpoint.
    let prev = psbt.inputs[0].non_witness_utxo.clone().expect("no record");
    let high = prev.output.len() as u32 + 7;
    psbt.unsigned_tx.input[0].previous_output = bitcoin::OutPoint {
        txid: prev.compute_txid(),
        vout: high,
    };
    // Give it a witness_utxo so a value is still available at all.
    psbt.inputs[0].witness_utxo = Some(prev.output[0].clone());

    let (_, err, ok) = encode(&b64_encode(&psbt.serialize()), &[]);
    if ok {
        assert!(
            !err.contains("TXID-BOUND"),
            "a value read from witness_utxo was labelled TXID-BOUND:\n{err}"
        );
        assert!(
            err.contains("PSBT-CLAIMED — unverified"),
            "the fallback value must render as unverified:\n{err}"
        );
    }
}

// ── the flags that quietly did nothing ──────────────────────────────────────

/// **`--separator -` produced steel `mt`'s own verbs refuse.** `read_strings`
/// strips whitespace and nothing else, so a hyphen lands on stdout — the stream
/// the operator engraves — and the codec then sees it as a data character
/// outside the bech32 alphabet. The sequence that makes it expensive: choose it,
/// cut nine plates over several hours, type them back, and discover `mt` cannot
/// read what `mt` produced.
#[test]
fn a_non_whitespace_separator_is_refused_before_anything_is_cut() {
    let v = base();
    for sep in ["-", ".", "|", "0"] {
        let (out, err, ok) = encode(
            &s(&v, "raw_hex"),
            &["--group-size", "5", "--separator", sep],
        );
        assert_refused(&out, &err, ok, "§1.1e");
        assert!(err.contains("is not whitespace"), "got: {err}");
    }
}

/// The control: whitespace separators must still work, and must round-trip.
#[test]
fn whitespace_separators_round_trip() {
    let v = base();
    for sep in [" ", "\t", "  "] {
        let (out, err, ok) = encode(
            &s(&v, "raw_hex"),
            &["--group-size", "5", "--separator", sep],
        );
        assert!(ok, "separator {sep:?} was refused: {err}");
        assert!(out.contains(sep), "the separator did not reach stdout");

        // ...and mt must be able to read back what mt just produced.
        let f = tmp(&out);
        let back = mt()
            .args(["verify", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            back.status.success(),
            "mt refused its own output with separator {sep:?}:\n{}",
            String::from_utf8_lossy(&back.stderr)
        );
    }
}

/// An index naming no input meant the input the operator MEANT to supply still
/// had no value — so §8.2b's arithmetic silently did not run, and `mt` printed
/// `FEE UNKNOWN` while they believed they had supplied it.
#[test]
fn an_input_value_index_that_names_no_input_is_refused() {
    let v = base();
    let (out, err, ok) = encode(&s(&v, "raw_hex"), &["--input-value", "9:1.0"]);
    assert_refused(&out, &err, ok, "§8.2c");
    assert!(
        err.contains("this transaction has 2 input(s)"),
        "got: {err}"
    );
}

#[test]
fn a_repeated_input_value_index_is_refused() {
    let v = base();
    let (out, err, ok) = encode(
        &s(&v, "raw_hex"),
        &["--input-value", "0:1.0", "--input-value", "0:2.0"],
    );
    assert_refused(&out, &err, ok, "§8.2c");
    assert!(err.contains("more than once"), "got: {err}");
}

/// A supplied value that contradicts the PSBT was discarded **without a word**.
/// The record winning is correct; the silence is not — the number the operator
/// typed is the one they will check the fee against.
#[test]
fn a_value_contradicting_the_psbt_is_reported_not_swallowed() {
    let v = base();
    let (_, err, ok) = encode(&s(&v, "finalized_psbt_b64"), &["--input-value", "0:9.5"]);
    assert!(ok, "{err}");
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("disagrees with the PSBT, and mt used the PSBT"),
        "the contradiction was swallowed:\n{err}"
    );
    assert!(
        flat.contains("3.00000000 BTC"),
        "the value mt actually used must be named:\n{err}"
    );
    assert!(
        flat.contains("would only change what mt PRINTS"),
        "the operator must be told why raising the number does not help:\n{err}"
    );
}

/// §1.1 rules `--transaction <psbt|hex>`; the flag accepted the hex half and
/// refused the other. A PSBT is the form a wallet exports, and it is what an
/// operator checking steel against what they built actually has.
#[test]
fn verify_transaction_accepts_a_psbt() {
    let v = base();
    let strings = {
        let f = tmp(&s(&v, "finalized_psbt_b64"));
        let out = mt()
            .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    let sf = tmp(&strings);
    let pf = tmp(&s(&v, "finalized_psbt_b64"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(sf.path())
        .arg("--transaction")
        .arg(pf.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success(), "a finalized PSBT was refused: {err}");
    assert!(
        err.contains("--transaction matches, on the full txid."),
        "{err}"
    );
}

/// An UNFINALIZED PSBT extracts to a transaction with the same txid and
/// different BYTES — one that cannot be broadcast. Matching against it would
/// vouch for steel that does not carry a spendable transaction.
#[test]
fn verify_transaction_refuses_an_unfinalized_psbt() {
    let v = base();
    let strings = {
        let f = tmp(&s(&v, "finalized_psbt_b64"));
        let out = mt()
            .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    let sf = tmp(&strings);
    let pf = tmp(&s(&v, "unfinalized_psbt_b64"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(sf.path())
        .arg("--transaction")
        .arg(pf.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("not finalized"), "got: {err}");
}

/// **My own §8.2f fix blocked a legitimate recovery path.** Widening the
/// recogniser to catch `mt verify mt1…` — the siblings' spelling — I tested
/// prefix and length and no CHARSET. A sensible filename is then refused as a
/// bearer leak, with a verdict stating something false about what the operator
/// did, at the moment they are trying to recover money.
///
/// **An over-correction that blocks a valid input is worse than the silence it
/// replaced.**
#[test]
fn a_filename_beginning_mt1_is_not_mistaken_for_a_bearer_string() {
    let v = base();
    let strings = {
        let f = tmp(&s(&v, "raw_hex"));
        let out = mt()
            .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    let dir = tempfile::tempdir().unwrap();
    // 40 characters, beginning `mt1` — the boundary the reviewer pinned.
    let name = "mt1-2026-08-23-cold-storage-transfer.txt";
    assert_eq!(name.len(), 40);
    let path = dir.path().join(name);
    std::fs::write(&path, &strings).unwrap();

    let out = mt()
        .args(["verify", "--in"])
        .arg(&path)
        .current_dir(dir.path())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("§8.2f"),
        "a FILENAME was refused as a bearer transaction:\n{err}"
    );
    assert!(out.status.success(), "{err}");

    // ...and the same with the bare relative name, which is what an operator
    // actually types.
    let out = mt()
        .args(["verify", "--in", name])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&out.stderr).contains("§8.2f"),
        "the relative filename was refused"
    );
}

/// The control, in the other direction: a REAL `mt1` string on the command line
/// must still be refused. Only the pair proves the charset test narrowed the
/// recogniser rather than disabling it.
#[test]
fn a_real_mt1_string_on_the_command_line_is_still_refused() {
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let string = corpus["vectors"][0]["strings"][0].as_str().unwrap();
    let out = mt().args(["verify", string]).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(err.contains("REFUSED — §8.2f,"), "got: {err}");
    assert!(err.contains("an mt1 set"), "got: {err}");
}

/// **The legend would have engraved an amount nobody is sent.** It printed the
/// sum of ALL outputs beside the named `TO` wallet, so a transaction with change
/// showed a figure that is neither output — suggested for permanent steel. `mt`
/// cannot identify change: that needs the sending wallet's descriptor, which §6
/// rules it never sees.
#[test]
fn the_legend_prints_no_amount_when_it_cannot_know_one() {
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    // The `even` vector has TWO outputs: 0.05 and 49.94998590.
    let hex = corpus["vectors"][0]["raw_hex"].as_str().unwrap();
    let (_, err, ok) = encode(hex, &["--to", "alice-cold"]);
    assert!(ok, "{err}");

    assert!(
        err.contains("    TO alice-cold\n"),
        "the TO line must carry the wallet and NO amount:\n{err}"
    );
    for wrong in ["49.99998590", "49.94998590 BTC\n", "0.05000000 BTC   <--"] {
        assert!(
            !err.contains(&format!("TO alice-cold  {wrong}")),
            "the legend put {wrong} beside the destination:\n{err}"
        );
    }
    assert!(
        err.contains("NO AMOUNT on the TO line"),
        "mt must say WHY there is no amount, or the operator writes one in:\n{err}"
    );
    assert!(err.contains("CHANGE"), "the reason is not named:\n{err}");
}

/// The control: with ONE output there is nothing to confuse, and withholding the
/// amount would make the legend worse.
#[test]
fn the_legend_prints_the_amount_when_there_is_only_one_output() {
    let v = base();
    let (_, err, ok) = encode(&s(&v, "raw_hex"), &["--to", "alice-cold"]);
    assert!(ok, "{err}");
    assert!(
        err.contains("TO alice-cold  7.99900000 BTC"),
        "a single-output transaction must carry its amount:\n{err}"
    );
    assert!(!err.contains("NO AMOUNT on the TO line"), "{err}");
}

/// **§8.2c's ACTUAL per-input requirement**, which has had no test since it was
/// written. A fold check found that a `refusals.toml` entry I added claiming to
/// cover it named `parse_input_values` — a **different guard with a confusingly
/// similar name** ("per input, never a total"). `require_psbt_input_values` is
/// live code, called on every PSBT, and sat outside both gates.
///
/// The rule: where a UTXO record is absent **from a PSBT**, `mt` requires the
/// operator to supply that input's value, because §8.2b cannot check the
/// balance or the fee without it.
#[test]
fn refuses_a_psbt_input_with_no_utxo_record_and_no_supplied_value() {
    let v = base();
    let mut psbt = bitcoin::Psbt::deserialize(&b64_decode(&s(&v, "finalized_psbt_b64"))).unwrap();
    // Strip BOTH records from input 0, leaving its value bound by nothing.
    psbt.inputs[0].non_witness_utxo = None;
    psbt.inputs[0].witness_utxo = None;

    let (out, err, ok) = encode(&b64_encode(&psbt.serialize()), &[]);
    assert_refused(&out, &err, ok, "§8.2c");
    assert!(
        err.contains("carries no UTXO record and no supplied value"),
        "got: {err}"
    );
    assert!(
        err.contains("--input-value"),
        "the refusal must name the way out: {err}"
    );
}

/// The control: supplying the value makes the same PSBT encode. Without it, a
/// guard that refused unconditionally would pass the test above.
#[test]
fn supplying_the_missing_value_lets_that_psbt_encode() {
    let v = base();
    let mut psbt = bitcoin::Psbt::deserialize(&b64_decode(&s(&v, "finalized_psbt_b64"))).unwrap();
    psbt.inputs[0].non_witness_utxo = None;
    psbt.inputs[0].witness_utxo = None;

    let (out, err, ok) = encode(&b64_encode(&psbt.serialize()), &["--input-value", "0:3.0"]);
    assert!(ok, "the supplied value was not accepted: {err}");
    assert!(
        out.lines().all(|l| l.starts_with("mt1")),
        "no strings produced"
    );
    // ...and it renders as the operator's word, not as anything verified.
    assert!(err.contains("OPERATOR-ASSERTED"), "{err}");
}

/// §10.10: a pasted **txid** is a thing an operator reaches for, because it is
/// what an explorer shows and what `mt`'s own `TX` row prints. It used to fall
/// through to "valid hex but does not parse as a transaction" — true, and it
/// sends them to look at the wrong thing.
#[test]
fn refuses_a_pasted_txid_by_name() {
    let v = base();
    let tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    let txid = tx.compute_txid().to_string();
    assert_eq!(txid.len(), 64);

    let (out, err, ok) = encode(&txid, &[]);
    assert_refused(&out, &err, ok, "§10.10");
    assert!(err.contains("transaction ID"), "got: {err}");
    assert!(
        err.contains("getrawtransaction"),
        "the refusal must name how to get the actual transaction: {err}"
    );
}

/// **`--json` was wired into `inspect` only** — the precise defect
/// `render_json`'s own doc comment condemns, one commit after that comment was
/// written. A caller who asks for machine output and gets prose with exit 0 will
/// parse *something* out of the prose.
///
/// It works where there is a report to serialise, and REFUSES where there is
/// not. Doing nothing quietly is the defect; the absent feature is not.
#[test]
fn json_works_where_there_is_a_report_and_refuses_where_there_is_not() {
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let strings: Vec<String> = corpus["vectors"][0]["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let f = tmp(&strings.join("\n"));

    // decode: a report exists, so --json must produce JSON on stderr.
    //
    // **PARSE THE WHOLE STREAM, NOT A SLICE OF IT.** The first version of this
    // test found the first `{` and the last `}` and parsed between them — which
    // is what an implementation does, not what a CALLER does. It therefore
    // passed while `decode --json` was writing the PROSE report and the JSON to
    // the same stream, in the tool's core offline scenario, so the thing a
    // caller actually receives did not parse at all. A test shaped to the
    // implementation instead of to the consumer.
    let out = mt()
        .args(["decode", "--json", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    let v: serde_json::Value = serde_json::from_str(&err)
        .unwrap_or_else(|e| panic!("decode --json's stderr is not one JSON document: {e}\n{err}"));
    // ...and the prose that used to be interleaved is now DATA inside it.
    assert!(v["warnings"].is_array(), "warnings are not carried: {err}");
    assert!(
        v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("no bitcoind reachable")),
        "the offline warning was dropped rather than carried: {err}"
    );
    // stdout stays the artifact.
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.trim().chars().all(|c| c.is_ascii_hexdigit()));

    // verify and encode: no report, so the flag must REFUSE rather than sit inert.
    for verb in ["verify", "encode"] {
        let out = mt()
            .args([verb, "--json", "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success(), "{verb} accepted --json silently");
        assert!(err.contains("--json has no meaning"), "{verb}: {err}");
    }
}

/// A pasted txid reaches the READING verbs as easily as `encode` — more easily,
/// since `inspect` is the verb a recoverer is pointed at and a txid is what an
/// explorer shows them.
#[test]
fn a_pasted_txid_is_named_on_every_verb() {
    let v = base();
    let tx: bitcoin::Transaction = deserialize(&hex_to_bytes(&s(&v, "raw_hex"))).unwrap();
    let txid = tx.compute_txid().to_string();
    for verb in ["decode", "verify", "inspect"] {
        let f = tmp(&txid);
        let out = mt()
            .args([verb, "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success(), "{verb} accepted a txid");
        assert!(
            err.contains("REFUSED — §10.10,") && err.contains("transaction ID"),
            "{verb}: {err}"
        );
    }
}

/// **`b` has TWO in-alphabet originals and mt does not choose between them.**
/// mt's own refusal remedy lists both `b/6` and `8/b`, so a `b` on the page is a
/// misread `6` OR a misread `8` — and `b` is not in the bech32 alphabet at all,
/// so leaving it alone is not an option either: the string would not convert to
/// symbols and BCH would never see it.
///
/// mt tries every candidate and keeps **the one that costs the checksum least**,
/// then says what it did — in a notice of its own, because this is not damage
/// and must not be charged to the 4-symbol budget.
#[test]
fn b_is_resolved_by_the_checksum_not_by_a_guess() {
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut strings: Vec<String> = corpus["vectors"][0]["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();

    // A REALISTIC misreading: replace genuine `6`s, which is what an operator
    // writing `b` for `6` produces. (Substituting over some other character is
    // damage, not a misreading, and mt reports it as such — the branch below.)
    let mut c: Vec<char> = strings[0].chars().collect();
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
    assert_eq!(n, 2, "the fixture contains no `6` to misread");
    strings[0] = c.into_iter().collect();

    let f = tmp(&strings.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        out.status.success(),
        "two misread `6`s were not resolved:\n{err}"
    );

    assert!(
        err.contains("CHARACTERS mt READ DIFFERENTLY"),
        "the transliteration was not reported:\n{err}"
    );
    assert!(
        err.contains("you typed b, mt read it as 6"),
        "the notice must name what the OPERATOR typed, not only mt's reading:\n{err}"
    );
    // It cost nothing, and mt may say so only because the reading checksums.
    assert!(
        err.contains("This cost NONE of the 4-symbol repair budget"),
        "{err}"
    );
    assert!(
        !err.contains("CORRECTION APPLIED"),
        "a correct reading should need no BCH repair at all:\n{err}"
    );
}

/// The other branch, and the one the claim used to lie about: when the character
/// was DAMAGED rather than misread, neither candidate checksums on its own, BCH
/// repairs whichever mt chose, and **those repairs come out of the operator's
/// budget**. mt must not call that free.
#[test]
fn a_damaged_character_reported_as_b_is_not_called_free() {
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mt-codec/src/test_vectors/mt1_v1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut strings: Vec<String> = corpus["vectors"][0]["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    // Overwrite characters that are NOT `6` or `8`, so neither reading is right.
    let mut c: Vec<char> = strings[0].chars().collect();
    let mut n = 0;
    for ch in c.iter_mut().skip(20) {
        if n == 2 {
            break;
        }
        if *ch != '6' && *ch != '8' && *ch != 'b' {
            *ch = 'b';
            n += 1;
        }
    }
    assert_eq!(n, 2);
    strings[0] = c.into_iter().collect();

    let f = tmp(&strings.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success(), "{err}");
    assert!(
        err.contains("NEITHER reading checksummed on its own"),
        "mt did not say the repair came from the budget:\n{err}"
    );
    assert!(
        !err.contains("This cost NONE"),
        "mt claimed a reading was free while BCH was repairing it:\n{err}"
    );
}

/// **F-237: a sibling's material is named, not described as bad bech32.** The
/// operator is holding the RIGHT material for the WRONG TOOL, and `mt` knows
/// which tool — keyed on the literal prefix, so nothing is imported from the
/// sibling and the fork-per-codec ruling stands.
#[test]
fn a_sibling_format_is_named_rather_than_called_malformed() {
    for (prefix, tool, what) in [
        ("md1", "md", "descriptor material"),
        ("mk1", "mk", "key material"),
    ] {
        let body = format!(
            "{prefix}qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq\n{prefix}qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"
        );
        let f = tmp(&body);
        let out = mt()
            .args(["decode", "--bitcoin-cli", OFFLINE, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(!out.status.success());
        let err = String::from_utf8(out.stderr).unwrap();
        let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains(&format!("begin `{prefix}`")) && flat.contains(what),
            "{prefix}: the sibling is not named:\n{err}"
        );
        assert!(
            flat.contains(&format!("`{tool}` reads")),
            "{prefix}: the right tool is not named:\n{err}"
        );
        // ...and mt must not echo the material back, sibling or not.
        assert!(
            !err.contains("qqqqqqqqqqqqqqqqq"),
            "{prefix}: mt echoed the input:\n{err}"
        );
    }
}

// ── §8.2h — stdout is a world-readable file ──────────────────────────────────
//
// §8.2g's other half, ruled 2026-08-24 from the Goal 1 journey walk (F-244).
// `mt` warned in detail that the INPUT file was readable by others and then
// wrote the strings -- the engraving itself -- to a file it never mentioned
// again. The old warning fired on ANY redirection and never read the mode.
//
// WARN ON INPUT, REFUSE ON OUTPUT: an input's exposure has already happened, so
// refusing prevents nothing; an output's has not, so declining to create it
// badly is the whole remedy.

/// Run `mt encode` with **stdout redirected to a file of the given mode**, which
/// the shared `encode()` helper cannot do -- it pipes, and a pipe is precisely
/// the case that must NOT be refused.
#[cfg(unix)]
fn encode_to_file(body: &str, extra: &[&str], mode: u32) -> (u64, String, bool) {
    use std::os::unix::fs::PermissionsExt;
    let f = tmp(body);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("strings.txt");
    let sink = std::fs::File::create(&dest).unwrap();
    std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode)).unwrap();

    let mut c = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"));
    c.args(["encode", "--bitcoin-cli", OFFLINE]);
    c.args(extra);
    let out = c
        .arg("--in")
        .arg(f.path())
        .stdout(std::process::Stdio::from(sink))
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    (
        std::fs::metadata(&dest).unwrap().len(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

#[test]
#[cfg(unix)]
fn refuses_a_world_readable_stdout() {
    let v = base();
    let (written, err, ok) = encode_to_file(&s(&v, "finalized_psbt_b64"), &[], 0o644);
    assert_refused("", &err, ok, "§8.2h");
    assert_eq!(
        written, 0,
        "a refusal must produce NO artifact -- the strings are the engraving"
    );
    assert!(
        err.contains("--allow-world-readable"),
        "the refusal must name the override: {err}"
    );
}

/// The override, so the refusal is a guard and not a wall.
#[test]
#[cfg(unix)]
fn the_override_permits_a_world_readable_stdout() {
    let v = base();
    let (written, err, ok) = encode_to_file(
        &s(&v, "finalized_psbt_b64"),
        &["--allow-world-readable"],
        0o644,
    );
    assert!(ok, "the override must permit the write: {err}");
    assert!(written > 0, "the strings must actually be written");
}

// ── The NEAR MISSES. Both MUST pass. ─────────────────────────────────────────
// Every guard added during the `mt` cycle broke on the input that merely
// RESEMBLES the one the finding named. A finding hands you a hostile X and never
// the legitimate near-X.

/// `mt encode … > private.txt` with a tight umask is already safe.
#[test]
#[cfg(unix)]
fn does_not_refuse_an_owner_only_stdout() {
    let v = base();
    let (written, err, ok) = encode_to_file(&s(&v, "finalized_psbt_b64"), &[], 0o600);
    assert!(ok, "an owner-only file is exactly what we want: {err}");
    assert!(written > 0, "the strings must actually be written");
}

/// `mt encode … | anything` has no file mode at all -- `S_ISFIFO`, not
/// `S_ISREG`. Refusing here would break every pipeline `mt` exists to serve,
/// including the `stdout is the artifact` contract itself.
#[test]
fn does_not_refuse_a_pipe() {
    let v = base();
    let (out, err, ok) = encode(&s(&v, "finalized_psbt_b64"), &[]);
    assert!(ok, "a pipe is not a world-readable file: {err}");
    assert!(
        out.contains("mt1"),
        "the strings must still reach the pipe: {out}"
    );
}

// ── R0 round 0, finding I3 ───────────────────────────────────────────────────
// The first §8.2h guard keyed on `is_file()`, and §8.2h's own text claimed a
// FIFO "is not a file whose mode means anything". MEASURED FALSE: a NAMED fifo
// carries a mode (0666 from mkfifo) and a third party reading it really does
// receive the bytes. Only the ANONYMOUS pipe behind `|` is 0600.

#[cfg(unix)]
fn encode_to_sink(body: &str, extra: &[&str], sink: std::fs::File) -> (String, bool) {
    let f = tmp(body);
    let mut c = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"));
    c.args(["encode", "--bitcoin-cli", OFFLINE]);
    c.args(extra);
    let out = c
        .arg("--in")
        .arg(f.path())
        .stdout(std::process::Stdio::from(sink))
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

#[test]
#[cfg(unix)]
fn refuses_a_world_readable_named_fifo() {
    use std::os::unix::fs::PermissionsExt;
    let v = base();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("leak");
    std::process::Command::new("mkfifo")
        .arg(&p)
        .status()
        .unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o666)).unwrap();
    // O_RDWR: opening a FIFO write-only blocks until a reader arrives.
    let sink = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&p)
        .unwrap();

    let (err, ok) = encode_to_sink(&s(&v, "finalized_psbt_b64"), &[], sink);
    assert!(
        !ok,
        "a 0666 named FIFO is readable by others and really leaks: {err}"
    );
}

/// NEAR MISS, and the sharpest of them: `/dev/null` is mode **0666**. A guard
/// reading only permission bits refuses `mt encode … > /dev/null`. Character
/// devices persist nothing, so they are exempt.
#[test]
#[cfg(unix)]
fn does_not_refuse_dev_null() {
    let v = base();
    let sink = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap();
    let (err, ok) = encode_to_sink(&s(&v, "finalized_psbt_b64"), &[], sink);
    assert!(ok, "/dev/null is 0666 but persists nothing: {err}");
}
