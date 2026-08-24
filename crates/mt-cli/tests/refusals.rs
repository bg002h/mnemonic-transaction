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

/// A stand-in `bitcoin-cli` that answers from a script rather than a chain.
///
/// **Not a convenience.** §8.5 and §6a are the two refusals that cannot fire
/// without a node, so testing them against the real one would make them
/// unrunnable in CI — and untested is how a refusal that never fires looks from
/// the outside.
fn node_stub(
    gettxout: &str,
    parent_confirmations: Option<u32>,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let parent = match parent_confirmations {
        Some(n) => format!(r#"{{"txid":"x","confirmations": {n}}}"#),
        None => String::new(),
    };
    let script = format!(
        r#"#!/bin/sh
# Reads bitcoin-cli's -stdin form: one argument per line.
verb=""
while read -r line; do
  if [ -z "$verb" ]; then verb="$line"; fi
done
case "$verb" in
  getblockcount)     echo 963832 ;;
  getindexinfo)      echo '{{"txindex": {{"synced": true, "best_block_height": 963832}}}}' ;;
  gettxout)          {gettxout} ;;
  getrawtransaction) {parent_case} ;;
  *)                 exit 1 ;;
esac
"#,
        gettxout = if gettxout.is_empty() {
            "exit 1".to_string()
        } else {
            format!("echo '{gettxout}'")
        },
        parent_case = if parent.is_empty() {
            "exit 1".to_string()
        } else {
            format!("echo '{parent}'")
        },
    );
    // A DIRECTORY, not a NamedTempFile. A NamedTempFile stays open for writing
    // for its whole lifetime, and Linux refuses to exec a file that is open for
    // writing -- ETXTBSY. The failure is silent here: exec fails, Node::find
    // returns None, and every chain-derived row reads UNKNOWN, so the test
    // reports "no node reachable" rather than "the stub could not run".
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bitcoin-cli");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    (dir, path)
}

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
    let (_dir, stub) = node_stub("", Some(6));
    let (out, err, ok) = encode_with_node(&s(&v, "raw_hex"), &stub, &[]);
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
    let (_dir, stub) = node_stub("", Some(0)); // found, 0 confirmations: in the mempool
    let (_, err, ok) = encode_with_node(&s(&v, "raw_hex"), &stub, &[]);
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
    let (_dir, stub) = node_stub(r#"{"value": 3.00000000, "scriptPubKey": {}}"#, None);
    let (out, err, ok) = encode_with_node(
        &s(&v, "raw_hex"),
        &stub,
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
    let (_dir, stub) = node_stub(r#"{"value": 4.00000000, "scriptPubKey": {}}"#, None);
    let (_, err, ok) = encode_with_node(
        &s(&v, "raw_hex"),
        &stub,
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
