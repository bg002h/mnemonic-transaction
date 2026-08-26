//! `mt encode --record` — P2 of `SPEC_engrave_transaction`.
//!
//! **`mt` owns transactions, so `mt` manufactures the `tx:` record.** `me` has
//! no verb that manufactures any other constellation string — `md1`, `mk1` and
//! `ms1` all come from their own tools and `me` only consumes them — so the
//! record it consumes must come from the tool that owns the material. This file
//! is the producer's half.
//!
//! **THE RECORD IS CONCATENATION, and that is the whole wire format.** `tx:`
//! followed by the transaction's canonical serialization in lowercase hex,
//! nothing else — `me`'s `sysw::record::TX_PREFIX` + `hex_lower`. An earlier
//! parallel implementation framed it (magic, version, form byte, txid, wtxid,
//! flags); that format is RETIRED, and the acceptance sheet records why: with
//! the txid derived and the wtxid superseded by the signature predicate,
//! nothing survives for a frame to carry.
//!
//! The other tests here are about the CHANNEL: which stream carries what, and
//! that a failing run contributes NOTHING to stdout — the invariant the whole
//! `mt … | me sysw pack` pipeline rests on, because `fish` reports a
//! pipeline's status as the LAST command's, so an upstream failure is
//! otherwise invisible.

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

/// The honest 222-byte 1-in/2-out P2WPKH spend, txid from the node that made it.
fn even() -> serde_json::Value {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()
        .clone()
}

/// The SAME transaction with every witness stripped — 113 bytes, and its txid
/// is byte-identical to the 222-byte original because stripping the witness is
/// precisely the operation the txid is defined to ignore.
///
/// **This is the artifact §8.3 exists for.** It parses, it round-trips, and its
/// txid matches what an operator would compare against — and not one input
/// carries a signature, so a plate cut from it can never be broadcast. Derived
/// here rather than pasted, so it cannot drift from the honest vector it is the
/// stripped form of.
fn even_stripped_hex() -> String {
    let raw = hex(even()["raw_hex"].as_str().unwrap());
    let tx: bitcoin::Transaction = bitcoin::consensus::deserialize(&raw).unwrap();
    let legacy = bitcoin::Transaction {
        version: tx.version,
        lock_time: tx.lock_time,
        input: tx
            .input
            .iter()
            .map(|i| bitcoin::TxIn {
                witness: bitcoin::Witness::new(),
                ..i.clone()
            })
            .collect(),
        output: tx.output.clone(),
    };
    assert_eq!(
        legacy.compute_txid(),
        tx.compute_txid(),
        "stripping the witness must not change the txid, or this fixture is \
         not the artifact the guard exists for"
    );
    let bytes = bitcoin::consensus::serialize(&legacy);
    assert_eq!(bytes.len(), 113, "the pinned size of the stripped form");
    use core::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn tmp_with(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

/// Every run here is OFFLINE, so the assertions are about the record and not
/// about what a reachable node would add.
fn encode(args: &[&str], hexstr: &str) -> assert_cmd::assert::Assert {
    let f = tmp_with(hexstr.as_bytes());
    let mut c = mt();
    c.arg("encode");
    c.args(args.iter());
    c.args(["--bitcoin-cli", "/nonexistent/bitcoin-cli"]);
    c.arg("--in").arg(f.path()).assert()
}

// ── R3: no default, and the refusal teaches ──────────────────────────────────

/// R3 (spec §5, §2.2) — `--record` with neither form. **The refusal TEACHES**,
/// because a bare blocking refusal is what gets aliased away.
#[test]
fn record_without_a_form_is_refused_and_the_refusal_teaches() {
    let a = encode(&["--record"], even()["raw_hex"].as_str().unwrap()).failure();
    let err = String::from_utf8_lossy(&a.get_output().stderr).to_string();
    assert!(err.contains("--record needs a form"), "{err}");
    // The citation NAMES its document. `mt`'s own spec has a §2.2-shaped
    // section about `mt-codec`, so a bare `§2.2` resolves to the wrong file.
    assert!(err.contains("SPEC_engrave §2.2"), "{err}");
    assert!(err.contains("--raw"), "it must NAME both forms: {err}");
    assert!(err.contains("--chunks"), "{err}");
    assert!(
        err.contains("QR plates") && err.contains("Text plates"),
        "and say what each one PRODUCES — the choice is not reversible once \
         the steel is cut: {err}"
    );
    assert!(
        a.get_output().stdout.is_empty(),
        "nothing on stdout on a refusal"
    );
}

/// It runs BEFORE the transaction is read, so a refusal costs no work and can
/// never leave a partial artifact. Asserted with a path that does not exist: if
/// the form guard ran second, the message would be about the missing file.
#[test]
fn the_form_refusal_runs_before_anything_is_read() {
    let a = mt()
        .args(["encode", "--record", "--in", "/nonexistent/tx.hex"])
        .assert()
        .failure();
    let err = String::from_utf8_lossy(&a.get_output().stderr).to_string();
    assert!(err.contains("--record needs a form"), "{err}");
    assert!(!err.contains("cannot read"), "{err}");
}

/// The other two halves of the same rule are STRUCTURAL — clap enforces them,
/// so `record_form_guard` only ever handles the one case clap cannot express.
/// If either `requires`/`conflicts_with` is dropped, this goes red.
#[test]
fn a_form_without_record_and_both_forms_at_once_are_refused_by_clap() {
    for args in [
        vec!["--raw"],
        vec!["--chunks"],
        vec!["--record", "--raw", "--chunks"],
    ] {
        let a = encode(&args, even()["raw_hex"].as_str().unwrap()).failure();
        let err = String::from_utf8_lossy(&a.get_output().stderr).to_string();
        assert!(
            err.contains("error:") && (err.contains("--record") || err.contains("--raw")),
            "{args:?} must be refused by the parser, not silently accepted: {err}"
        );
        assert!(a.get_output().stdout.is_empty(), "{args:?}: stdout");
    }
}

// ── The two forms ────────────────────────────────────────────────────────────

/// **The RAW form is `tx:` + the transaction's hex and NOTHING else.** Asserted
/// as one string equality rather than a set of `contains` checks: a framed
/// record would satisfy every `contains` here and still be the retired format.
#[test]
fn the_raw_form_is_the_prefix_and_the_transaction_hex_and_nothing_else() {
    let v = even();
    let raw_hex = v["raw_hex"].as_str().unwrap();
    let a = encode(&["--record", "--raw"], raw_hex).success();
    let out = String::from_utf8_lossy(&a.get_output().stdout).to_string();
    assert_eq!(out, format!("tx:{raw_hex}\n"), "the record IS concatenation");
    assert_eq!(out.lines().count(), 1, "ONE record, one line");
}

/// The CHUNKS form is exactly what `mt encode` already emits, byte for byte.
///
/// **This is a finding, not an implementation choice.** Under the shipped
/// design a chunk set rides as BARE `mt1` records — no prefix, no hex, the
/// container's own LF between them, the same route `md1`/`mk1` already take —
/// and the `tx:` metadata record that an earlier draft put beside them was
/// dropped because nothing survives for it to carry. So `--chunks` selects a
/// form; it does not transform the artifact. Wrapping the strings in anything
/// here would be inventing a container the consumer does not parse.
#[test]
fn the_chunks_form_is_exactly_what_bare_encode_emits() {
    let v = even();
    let raw_hex = v["raw_hex"].as_str().unwrap();
    let with = encode(&["--record", "--chunks"], raw_hex).success();
    let without = encode(&[], raw_hex).success();
    assert_eq!(
        with.get_output().stdout,
        without.get_output().stdout,
        "`--record --chunks` must not transform the strings"
    );
    let out = String::from_utf8_lossy(&with.get_output().stdout).to_string();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 6, "the even vector's six chunks");
    for l in &lines {
        assert!(l.starts_with("mt1"), "a BARE record, no prefix and no hex");
        assert!(!l.contains(' '), "no interior whitespace: {l}");
        assert_eq!(*l, l.to_ascii_lowercase());
    }
}

/// A record is engraved VERBATIM and `EPD` §6.4 forbids interior whitespace and
/// requires the canonical unbroken string, so the two flags that make stdout
/// non-canonical cannot be combined with `--record`.
///
/// Structural, via clap, rather than a guard: the alternative is silently
/// ignoring a flag the operator typed, which is how a grouped `mt1` string
/// reaches `me sysw pack` and comes back as "record 3 unrecognised" naming the
/// wrong tool.
#[test]
fn the_record_forms_cannot_be_grouped_or_elided() {
    for extra in [vec!["--group-size", "4"], vec!["--elide-prefix"]] {
        let mut args = vec!["--record", "--chunks"];
        args.extend(extra.iter().copied());
        let a = encode(&args, even()["raw_hex"].as_str().unwrap()).failure();
        assert!(
            a.get_output().stdout.is_empty(),
            "{extra:?}: a non-canonical record must never reach stdout"
        );
    }
}

// ── §8.3 on the new path ─────────────────────────────────────────────────────

/// **`--record --raw` inherits §8.3, and this is what the move BUYS.**
///
/// Before it, the producer (`me tx`) emitted a record for a witness-stripped
/// transaction at exit 0 and `me sysw pack` refused the same bytes at exit 4
/// one step later — two commands, two answers, with the operator shown a
/// success first. `mt` already refuses an input carrying neither scriptSig nor
/// witness, per input, so moving the verb here makes the disagreement
/// unconstructible: the producer never emits what the consumer will refuse.
#[test]
fn the_raw_form_inherits_the_signature_guard() {
    let stripped = even_stripped_hex();
    let a = encode(&["--record", "--raw"], &stripped).failure();
    let err = String::from_utf8_lossy(&a.get_output().stderr).to_string();
    assert!(err.contains("§8.3"), "the guard must name its section: {err}");
    assert!(
        err.contains("carry no signature") && err.contains("input 0"),
        "and name WHICH inputs: {err}"
    );
    assert!(
        a.get_output().stdout.is_empty(),
        "no record for an unbroadcastable transaction"
    );
    // The control: the SAME transaction with its witnesses intact passes, so
    // the refusal is about the signatures and not about the fixture.
    let ok = encode(&["--record", "--raw"], even()["raw_hex"].as_str().unwrap()).success();
    assert!(
        String::from_utf8_lossy(&ok.get_output().stdout).starts_with("tx:"),
        "the honest form still produces a record"
    );
}

/// The chunks form reaches the same guard by the same route — one call site,
/// not two — so neither form can emit a set for a transaction nothing satisfies.
#[test]
fn the_chunks_form_inherits_the_signature_guard_too() {
    let a = encode(&["--record", "--chunks"], &even_stripped_hex()).failure();
    let err = String::from_utf8_lossy(&a.get_output().stderr).to_string();
    assert!(err.contains("§8.3"), "{err}");
    assert!(a.get_output().stdout.is_empty());
}

// ── The pipeline invariant ───────────────────────────────────────────────────

/// **THE PIPELINE INVARIANT** (spec §1.1, §7): `mt` contributes NOTHING to
/// stdout on any failure path.
///
/// `fish` reports a pipeline's status as the LAST command's — `false | true`
/// gives `status=0` — so a failed `mt encode` is invisible to the operator
/// unless its stdout is empty and `me sysw pack` therefore refuses.
#[test]
fn a_failing_run_contributes_nothing_to_stdout() {
    let cases: Vec<(&str, Vec<String>)> = vec![
        (
            "--record with no form",
            vec!["encode".into(), "--record".into()],
        ),
        (
            "a file that does not exist",
            vec![
                "encode".into(),
                "--record".into(),
                "--raw".into(),
                "--in".into(),
                "/nonexistent/tx.hex".into(),
            ],
        ),
        (
            "input that is not a transaction",
            vec!["encode".into(), "--record".into(), "--raw".into()],
        ),
        (
            "a transaction on argv (§8.2f)",
            vec![
                "encode".into(),
                even()["raw_hex"].as_str().unwrap().to_string(),
            ],
        ),
    ];
    for (name, args) in cases {
        let mut c = mt();
        c.args(args.iter());
        c.args(["--bitcoin-cli", "/nonexistent/bitcoin-cli"]);
        let a = c.write_stdin("not a transaction at all\n").assert().failure();
        assert!(
            a.get_output().stdout.is_empty(),
            "{name}: {} bytes reached stdout on a failing run — `fish` reports \
             only the LAST command's status, so this is how a failed encode \
             becomes an engraved plate",
            a.get_output().stdout.len()
        );
    }
}

// ── What the RAW form says it made ───────────────────────────────────────────

/// **THE RAW FORM MUST NOT DESCRIBE THE ARTIFACT IT DID NOT MAKE.**
///
/// Found by regenerating `mnemonic-engrave`'s host journey against the new
/// verb and reading the transcript, which is the only place it is visible:
/// every unit test passed while `--record --raw` printed the whole `mt1`
/// apparatus. It emits ONE `tx:` record for a QR plate, and stderr said
///
/// - *"mt corrects up to 4 wrong CHARACTERS per string … strings 1-6 are 87"*
///   — BCH per-string correction, on an artifact carrying no `mt1` string;
/// - *"Type the strings back from the steel and run `mt verify`"* — you cannot
///   type a QR symbol back, and there are no strings on the plate;
/// - `CUT 6 strings, 522 characters` and `PREFIX all 6 strings begin mt1p9h8jqq9`;
/// - a legend reading `FORMAT: mt1 codex32` and *"on EACH plate, its number:
///   1/6, 2/6, … 6/6"* — for a job that produces one plate of a different kind.
///
/// Every one of those is an instruction for a **21-minute-per-plate,
/// irreversible** operation, and each names a different artifact than the one
/// on stdout. The strings are still computed (the pipeline builds them either
/// way); they are simply not what this run emitted.
#[test]
fn the_raw_form_does_not_describe_the_mt1_strings_it_did_not_emit() {
    let v = even();
    let raw_hex = v["raw_hex"].as_str().unwrap();
    let raw = encode(&["--record", "--raw"], raw_hex).success();
    let err = String::from_utf8_lossy(&raw.get_output().stderr).to_string();

    // Each of these is a claim about `mt1` strings. NONE may appear.
    for lie in [
        "corrects up to 4 wrong CHARACTERS",
        "typed-from-steel",
        "mt verify",
        "mt1 codex32",
        "1/6",
        "CUT       6 strings",
        "PREFIX    all 6 strings",
        "strings 1-6 are 87",
    ] {
        assert!(
            !err.contains(lie),
            "the raw form printed {lie:?}, which describes the mt1 strings it \
             did NOT emit:\n{err}"
        );
    }

    // …and it must say what it DID make, and how to check it. `mt inspect`
    // over a scanned QR is the command the device's own post-cut screen names.
    assert!(err.contains("RECORD"), "say what was produced: {err}");
    assert!(
        err.contains("mt inspect"),
        "the post-cut check for a QR plate is `mt inspect` over what the \
         scanner hands back — `mt verify` reads mt1 strings: {err}"
    );

    // THE CONTROL, on the same fixture: the CHUNKS form still says all of it,
    // so this is about the FORM and not about a block that was deleted.
    let chunks = encode(&["--record", "--chunks"], raw_hex).success();
    let cerr = String::from_utf8_lossy(&chunks.get_output().stderr).to_string();
    for kept in [
        "corrects up to 4 wrong CHARACTERS",
        "typed-from-steel",
        "mt verify",
        "mt1 codex32",
        "1/6",
        "CUT       6 strings",
        "PREFIX    all 6 strings",
    ] {
        assert!(
            cerr.contains(kept),
            "the chunks form must keep {kept:?} — it DOES cut six strings:\n{cerr}"
        );
    }
    assert!(!cerr.contains("mt inspect"), "chunks are verified by typing back");
}

/// The bearer warning about a redirected stdout names the right NOUN. It is
/// the same hazard either way — the artifact copies itself — but "the strings
/// just left this terminal" after emitting one record is the wording that
/// makes an operator look for six of something.
#[test]
fn the_redirected_warning_names_what_actually_left() {
    let v = even();
    let raw_hex = v["raw_hex"].as_str().unwrap();
    let raw = encode(&["--record", "--raw"], raw_hex).success();
    let err = String::from_utf8_lossy(&raw.get_output().stderr).to_string();
    assert!(err.contains("the record just left this terminal"), "{err}");
    // …and the BODY agrees with the subject. The first substitution left
    // "so the record went somewhere that keeps them".
    assert!(err.contains("the record went somewhere that keeps it"), "{err}");
    assert!(!err.contains("keeps them"), "singular subject, singular body: {err}");
    let chunks = encode(&["--record", "--chunks"], raw_hex).success();
    let cerr = String::from_utf8_lossy(&chunks.get_output().stderr).to_string();
    assert!(cerr.contains("the strings just left this terminal"), "{cerr}");
}

/// The §8.2h refusal names the artifact THAT RUN produced.
///
/// It said *"These **strings** ARE the engraving"* and *"stdout IS the
/// strings"* on a run that emitted one `tx:` record — the same defect class as
/// the block above, in the one message an operator meets while their file
/// already exists. Reached by the commonest divergence there is: `mt encode
/// --record --raw > rec.txt`, where `>` creates the file 0644 under the usual
/// umask.
#[test]
fn the_world_readable_refusal_names_the_artifact_this_run_made() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let f = tmp_with(even()["raw_hex"].as_str().unwrap().as_bytes());

    // `std::process::Command`, not `assert_cmd`: the refusal under test is
    // about the MODE OF FD 1, so stdout has to be a real 0644 file.
    let out = |form: &str| -> String {
        let sink = dir.path().join(format!("out{form}.txt"));
        let handle = std::fs::File::create(&sink).unwrap();
        std::fs::set_permissions(&sink, std::fs::Permissions::from_mode(0o644)).unwrap();
        let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
            .args(["encode", "--record", form])
            .args(["--bitcoin-cli", "/nonexistent/bitcoin-cli"])
            .arg("--in")
            .arg(f.path())
            .stdout(std::process::Stdio::from(handle))
            .stderr(std::process::Stdio::piped())
            .output()
            .unwrap();
        assert!(!o.status.success(), "{form}: §8.2h must refuse a 0644 stdout");
        assert_eq!(
            std::fs::metadata(&sink).unwrap().len(),
            0,
            "{form}: a refusal must leave no artifact"
        );
        String::from_utf8_lossy(&o.stderr).into_owned()
    };

    let raw = out("--raw");
    assert!(raw.contains("§8.2h"), "{raw}");
    assert!(raw.contains("This record IS the engraving"), "{raw}");
    assert!(raw.contains("stdout IS the record"), "{raw}");
    assert!(
        !raw.contains("These strings ARE") && !raw.contains("stdout IS the strings"),
        "the raw form emitted ONE record, not strings: {raw}"
    );

    // THE CONTROL: the chunks form keeps the plural, because it is true there.
    let chunks = out("--chunks");
    assert!(chunks.contains("These strings ARE the engraving"), "{chunks}");
    assert!(chunks.contains("stdout IS the strings"), "{chunks}");
}
