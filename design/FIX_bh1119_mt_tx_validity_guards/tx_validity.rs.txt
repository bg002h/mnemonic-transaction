//! bughunt-1119 FIX-E (mt): transaction validity guards, keys bh-mt-I16 and
//! bh-mt-I17 (architect rulings T16, T17). Spec: design/FIX_bh1119_mt_tx_validity_guards.md.
//!
//! I16: an output-less transaction, an output above 21,000,000 BTC, or outputs
//! totalling more, is REFUSED by `encode` and WARNED about by `inspect`; no
//! output sum may panic. I17: a PSBT input whose txid-bound non_witness_utxo has
//! no output at the input's vout is refused by `encode`, whatever else supplies
//! a value.

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::hex_to_bytes;

const OFFLINE: &str = "/nonexistent/bitcoin-cli";

/// The texts this FIX adds (PROPOSED, architect confirms). A wording change
/// updates these constants and nothing else.
const ZERO_OUT: &str = "the transaction has 0 outputs";
const SUFFIX: &str = ", so no node will ever accept this transaction.";
const BODY_PHRASE: &str = "These bytes can never be broadcast";

fn refused(verdict: &str, section: &str) -> String {
    format!("mt encode: REFUSED — {section}, {verdict}")
}

fn warned(verdict: &str) -> String {
    format!("WARNING: {verdict}{SUFFIX}")
}

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

fn base() -> serde_json::Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
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

fn to_hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}

/// The P5 fixture transaction (two signed segwit inputs, one output), changed by `f`.
fn mutated_tx(f: impl FnOnce(&mut bitcoin::Transaction)) -> bitcoin::Transaction {
    let v = base();
    let mut tx: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&hex_to_bytes(v["raw_hex"].as_str().unwrap())).unwrap();
    f(&mut tx);
    tx
}

fn mutated_raw_hex(f: impl FnOnce(&mut bitcoin::Transaction)) -> String {
    to_hex(&bitcoin::consensus::serialize(&mutated_tx(f)))
}

struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

fn run(args: &[&str], input: &str) -> Run {
    let f = file_with(input);
    let o = mt().args(args).arg("--in").arg(f.path()).output().unwrap();
    Run {
        code: o.status.code(),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn encode_offline(input: &str, extra: &[&str]) -> Run {
    let mut a = vec!["encode", "--bitcoin-cli", OFFLINE];
    a.extend_from_slice(extra);
    run(&a, input)
}

fn assert_refused_with(r: &Run, line: &str) {
    assert_ne!(r.code, Some(101), "mt panicked: {}", r.err);
    assert_eq!(r.code, Some(1), "encode did not refuse; stderr:\n{}", r.err);
    assert!(
        r.err.lines().any(|l| l == line),
        "missing refusal line {line:?}; stderr:\n{}",
        r.err
    );
    assert!(r.out.is_empty(), "a refused encode wrote stdout: {}", r.out);
}

fn assert_warned_with(r: &Run, verdict: &str) {
    assert_ne!(r.code, Some(101), "mt panicked: {}", r.err);
    assert_eq!(
        r.code,
        Some(0),
        "inspect must not refuse; stderr:\n{}",
        r.err
    );
    let w = warned(verdict);
    assert!(
        r.err.lines().any(|l| l == w),
        "missing warning {w:?}; stderr:\n{}",
        r.err
    );
    assert!(
        r.err.contains(BODY_PHRASE),
        "warning body missing; stderr:\n{}",
        r.err
    );
}

const MAX_MONEY: u64 = 2_100_000_000_000_000;

// ── I16, encode refuses ─────────────────────────────────────────────────────

#[test]
fn bh1119_mt_i16_encode_refuses_zero_outputs() {
    let hex = mutated_raw_hex(|tx| tx.output.clear());
    let r = encode_offline(&hex, &[]);
    assert_refused_with(&r, &refused(ZERO_OUT, "§8.2b"));
}

#[test]
fn bh1119_mt_i16_encode_refuses_output_above_money_range() {
    let hex = mutated_raw_hex(|tx| tx.output[0].value = bitcoin::Amount::from_sat(MAX_MONEY + 1));
    let r = encode_offline(&hex, &[]);
    assert_refused_with(
        &r,
        &refused(
            "output 0 pays 2100000000000001 sat, more than 21,000,000 BTC",
            "§8.2b",
        ),
    );
}

#[test]
fn bh1119_mt_i16_encode_refuses_output_total_above_money_range() {
    let hex = mutated_raw_hex(|tx| {
        let mut o = tx.output[0].clone();
        o.value = bitcoin::Amount::from_sat(1_100_000_000_000_000);
        tx.output = vec![o.clone(), o];
    });
    let r = encode_offline(&hex, &[]);
    assert_refused_with(
        &r,
        &refused(
            "outputs 0 to 1 total 2200000000000000 sat, more than 21,000,000 BTC",
            "§8.2b",
        ),
    );
}

#[test]
fn bh1119_mt_i16_encode_output_sum_overflow_refuses_not_panics() {
    let hex = mutated_raw_hex(|tx| {
        let mut o = tx.output[0].clone();
        o.value = bitcoin::Amount::from_sat(1u64 << 63);
        tx.output = vec![o.clone(), o];
    });
    let r = encode_offline(&hex, &[]);
    assert_refused_with(
        &r,
        &refused(
            "output 0 pays 9223372036854775808 sat, more than 21,000,000 BTC",
            "§8.2b",
        ),
    );
}

/// Order (Core's CheckTransaction): the output checks run before the
/// duplicate-input check.
#[test]
fn bh1119_mt_i16_encode_output_range_checked_before_duplicate_inputs() {
    let hex = mutated_raw_hex(|tx| {
        tx.input[1].previous_output = tx.input[0].previous_output;
        tx.output[0].value = bitcoin::Amount::from_sat(MAX_MONEY + 1);
    });
    let r = encode_offline(&hex, &[]);
    assert_refused_with(
        &r,
        &refused(
            "output 0 pays 2100000000000001 sat, more than 21,000,000 BTC",
            "§8.2b",
        ),
    );
}

/// Guard: exactly 21,000,000 BTC is inside the range and is not refused.
#[test]
fn bh1119_mt_i16_encode_output_at_money_range_is_accepted() {
    let hex = mutated_raw_hex(|tx| tx.output[0].value = bitcoin::Amount::from_sat(MAX_MONEY));
    let r = encode_offline(&hex, &[]);
    assert_eq!(r.code, Some(0), "stderr:\n{}", r.err);
    assert!(r.out.starts_with("mt1"), "no strings on stdout: {}", r.out);
    assert!(!r.err.contains("REFUSED"), "stderr:\n{}", r.err);
}

// ── I16, inspect warns ──────────────────────────────────────────────────────

fn inspect_offline(input: &str, extra: &[&str]) -> Run {
    let mut a = vec!["inspect", "--bitcoin-cli", OFFLINE];
    a.extend_from_slice(extra);
    run(&a, input)
}

#[test]
fn bh1119_mt_i16_inspect_warns_zero_outputs() {
    let hex = mutated_raw_hex(|tx| tx.output.clear());
    let r = inspect_offline(&hex, &[]);
    assert_warned_with(&r, ZERO_OUT);
    assert!(
        r.out.contains("OUT       0 output(s)"),
        "stdout:\n{}",
        r.out
    );
}

#[test]
fn bh1119_mt_i16_inspect_warns_output_above_money_range() {
    let hex = mutated_raw_hex(|tx| tx.output[0].value = bitcoin::Amount::from_sat(MAX_MONEY + 1));
    let r = inspect_offline(&hex, &[]);
    assert_warned_with(
        &r,
        "output 0 pays 2100000000000001 sat, more than 21,000,000 BTC",
    );
}

#[test]
fn bh1119_mt_i16_inspect_output_sum_overflow_warns_not_panics() {
    let hex = mutated_raw_hex(|tx| {
        let mut o = tx.output[0].clone();
        o.value = bitcoin::Amount::from_sat(1u64 << 63);
        tx.output = vec![o.clone(), o];
    });
    let r = inspect_offline(&hex, &[]);
    assert_warned_with(
        &r,
        "output 0 pays 9223372036854775808 sat, more than 21,000,000 BTC",
    );
}

/// With a node answering every input's value, the overflowing output sum
/// leaves FEE unknown rather than panicking or wrapping.
#[test]
fn bh1119_mt_i16_inspect_with_node_overflow_fee_unknown() {
    let hex = mutated_raw_hex(|tx| {
        let mut o = tx.output[0].clone();
        o.value = bitcoin::Amount::from_sat(1u64 << 63);
        tx.output = vec![o.clone(), o];
    });
    let stub = common::node_stub(r#"{"value": 1.00000000, "confirmations": 3}"#, &[]);
    let r = run(
        &["inspect", "--bitcoin-cli", stub.path().to_str().unwrap()],
        &hex,
    );
    assert_warned_with(
        &r,
        "output 0 pays 9223372036854775808 sat, more than 21,000,000 BTC",
    );
    assert!(
        r.out.lines().any(|l| l.starts_with("FEE       UNKNOWN")),
        "stdout:\n{}",
        r.out
    );
}

#[test]
fn bh1119_mt_i16_inspect_json_carries_the_warning_first() {
    let hex = mutated_raw_hex(|tx| tx.output.clear());
    let r = inspect_offline(&hex, &["--json"]);
    assert_eq!(r.code, Some(0), "stderr:\n{}", r.err);
    let v: serde_json::Value = serde_json::from_str(&r.out).expect("stdout is one JSON document");
    let w = v["warnings"].as_array().expect("a warnings array");
    assert_eq!(
        w.first().and_then(|x| x.as_str()),
        Some(format!("{ZERO_OUT}{SUFFIX}").as_str()),
        "warnings: {w:?}"
    );
}

/// The strings path of inspect (a set of mt1 strings carrying an output-less
/// transaction, as an mt before this FIX could have cut).
#[test]
fn bh1119_mt_i16_inspect_strings_warns_zero_outputs() {
    let tx = mutated_tx(|tx| tx.output.clear());
    let bytes = bitcoin::consensus::serialize(&tx);
    let strings =
        mt_codec::string_layer::pipeline::encode(&bytes, &tx.compute_txid().to_string()).unwrap();
    let r = inspect_offline(&strings.join("\n"), &[]);
    assert_warned_with(&r, ZERO_OUT);
}

/// Guard: a valid transaction carries no validity warning.
#[test]
fn bh1119_mt_i16_inspect_valid_tx_has_no_validity_warning() {
    let v = base();
    let r = inspect_offline(v["raw_hex"].as_str().unwrap(), &[]);
    assert_eq!(r.code, Some(0), "stderr:\n{}", r.err);
    assert!(
        !r.err.contains("no node will ever accept"),
        "stderr:\n{}",
        r.err
    );
}

/// decode shares Report::build, so the checked sum reaches it too: no panic,
/// the hex still printed (decode's warnings are an open point, not ruled).
#[test]
fn bh1119_mt_i16_decode_output_sum_overflow_does_not_panic() {
    let tx = mutated_tx(|tx| {
        let mut o = tx.output[0].clone();
        o.value = bitcoin::Amount::from_sat(1u64 << 63);
        tx.output = vec![o.clone(), o];
    });
    let bytes = bitcoin::consensus::serialize(&tx);
    let strings =
        mt_codec::string_layer::pipeline::encode(&bytes, &tx.compute_txid().to_string()).unwrap();
    let r = run(&["decode", "--bitcoin-cli", OFFLINE], &strings.join("\n"));
    assert_ne!(r.code, Some(101), "decode panicked: {}", r.err);
    assert_eq!(r.code, Some(0), "stderr:\n{}", r.err);
    assert_eq!(r.out.trim(), to_hex(&bytes));
}

// ── I17, a provably nonexistent outpoint ────────────────────────────────────

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn legacy_parent() -> (bitcoin::Transaction, u32) {
    let v = base();
    let prev: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&hex_to_bytes(v["legacy_parent_hex"].as_str().unwrap()))
            .unwrap();
    (prev, v["legacy_parent_vout"].as_u64().unwrap() as u32)
}

/// A finalized legacy (P2PKH-shaped scriptSig) PSBT spending `vouts` of the
/// fixture's legacy parent, every input carrying that parent as its
/// non_witness_utxo. `txid` overrides the outpoints' txid (for a hash
/// mismatch); `witness_utxo` adds a witness_utxo copied from the parent's real
/// output. One output pays the inputs' real total less 100,000 sat.
fn legacy_psbt(vouts: &[u32], txid: Option<bitcoin::Txid>, witness_utxo: bool) -> String {
    let (prev, real_vout) = legacy_parent();
    let real = prev.output[real_vout as usize].clone();
    let total = real.value.to_sat() * vouts.len() as u64 - 100_000;
    let unsigned = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vouts
            .iter()
            .map(|&vout| bitcoin::TxIn {
                previous_output: bitcoin::OutPoint {
                    txid: txid.unwrap_or_else(|| prev.compute_txid()),
                    vout,
                },
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence(0xFFFF_FFFD),
                witness: bitcoin::Witness::new(),
            })
            .collect(),
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(total),
            script_pubkey: real.script_pubkey.clone(),
        }],
    };
    let mut sig = vec![0x30, 0x44, 0x02, 0x20];
    sig.extend(std::iter::repeat_n(0x11u8, 32));
    sig.extend_from_slice(&[0x02, 0x20]);
    sig.extend(std::iter::repeat_n(0x22u8, 32));
    sig.push(0x01);
    let mut pk = vec![0x02u8];
    pk.extend(std::iter::repeat_n(0x33u8, 32));
    let script_sig = bitcoin::script::Builder::new()
        .push_slice::<&bitcoin::script::PushBytes>((&sig[..]).try_into().unwrap())
        .push_slice::<&bitcoin::script::PushBytes>((&pk[..]).try_into().unwrap())
        .into_script();
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(unsigned).unwrap();
    for inp in &mut psbt.inputs {
        inp.final_script_sig = Some(script_sig.clone());
        inp.non_witness_utxo = Some(prev.clone());
        if witness_utxo {
            inp.witness_utxo = Some(real.clone());
        }
    }
    b64(&psbt.serialize())
}

fn i17_verdict(n: usize, vout: u32) -> String {
    let (prev, _) = legacy_parent();
    format!(
        "input {n} spends output {vout} of {}, which has {} output(s)",
        prev.compute_txid(),
        prev.output.len()
    )
}

#[test]
fn bh1119_mt_i17_encode_refuses_missing_vout_with_input_value() {
    let r = encode_offline(&legacy_psbt(&[99], None, false), &["--input-value", "0:10"]);
    assert_refused_with(&r, &refused(&i17_verdict(0, 99), "§8.2d"));
}

#[test]
fn bh1119_mt_i17_encode_refuses_missing_vout_with_witness_utxo() {
    let r = encode_offline(&legacy_psbt(&[99], None, true), &[]);
    assert_refused_with(&r, &refused(&i17_verdict(0, 99), "§8.2d"));
}

#[test]
fn bh1119_mt_i17_encode_refuses_missing_vout_without_any_value() {
    let r = encode_offline(&legacy_psbt(&[99], None, false), &[]);
    assert_refused_with(&r, &refused(&i17_verdict(0, 99), "§8.2d"));
}

#[test]
fn bh1119_mt_i17_encode_names_the_first_offending_input() {
    let (_, real_vout) = legacy_parent();
    let r = encode_offline(
        &legacy_psbt(&[real_vout, 99], None, false),
        &["--input-value", "1:10"],
    );
    assert_refused_with(&r, &refused(&i17_verdict(1, 99), "§8.2d"));
}

/// Guard: a record whose hash does not match keeps the existing refusal, which
/// is checked first.
#[test]
fn bh1119_mt_i17_hash_mismatch_still_refused_first() {
    let other: bitcoin::Txid = "1111111111111111111111111111111111111111111111111111111111111111"
        .parse()
        .unwrap();
    let r = encode_offline(
        &legacy_psbt(&[99], Some(other), false),
        &["--input-value", "0:10"],
    );
    assert_eq!(r.code, Some(1), "stderr:\n{}", r.err);
    let (prev, _) = legacy_parent();
    let want = refused(
        &format!(
            "input 0's non_witness_utxo hashes to {}, not {other}",
            prev.compute_txid()
        ),
        "§8.2d",
    );
    assert!(r.err.lines().any(|l| l == want), "stderr:\n{}", r.err);
}

/// Guard: the record's real vout still encodes, the value txid-bound.
#[test]
fn bh1119_mt_i17_existing_vout_still_encodes() {
    let (_, real_vout) = legacy_parent();
    let r = encode_offline(&legacy_psbt(&[real_vout], None, false), &[]);
    assert_eq!(r.code, Some(0), "stderr:\n{}", r.err);
    assert!(r.out.starts_with("mt1"), "no strings on stdout: {}", r.out);
    assert!(!r.err.contains("REFUSED"), "stderr:\n{}", r.err);
}
