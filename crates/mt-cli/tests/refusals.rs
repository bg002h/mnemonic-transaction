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
