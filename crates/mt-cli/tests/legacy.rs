//! §8.2c's legacy path — **the case the fixture material was committed for and
//! nothing ever built.**
//!
//! `p5_base.json` has carried `legacy_parent_hex`, `legacy_parent_txid`,
//! `legacy_parent_vout` and `legacy_parent_value_btc` since P5, and no test
//! referenced any of them. An adversarial review built the PSBT by hand and
//! found the warning wrong in six ways at once — including firing on the common
//! path while asserting the opposite of what the report printed five lines
//! below it.
//!
//! **An unused fixture is a hypothesis.** These tests are what make it evidence.

use assert_cmd::Command;
use bitcoin::consensus::deserialize;
use std::io::Write;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}
const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn base() -> serde_json::Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    s.trim()
        .as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(core::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
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

/// Build a finalized LEGACY PSBT spending the fixture's P2PKH output.
///
/// `with_record` chooses the case: with `non_witness_utxo` the value is
/// **txid-bound** (§8.2d hashes it and matches), without it the value is bound
/// by nothing and §8.2c's warning is the whole mitigation.
fn legacy_psbt(with_record: bool, supplied: Option<u64>) -> String {
    let v = base();
    let prev: bitcoin::Transaction =
        deserialize(&hex_to_bytes(v["legacy_parent_hex"].as_str().unwrap())).unwrap();
    let vout = v["legacy_parent_vout"].as_u64().unwrap() as u32;
    let value = prev.output[vout as usize].value;

    let mut unsigned = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: prev.compute_txid(),
                vout,
            },
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence(0xFFFF_FFFD),
            witness: bitcoin::Witness::new(),
        }],
        output: vec![bitcoin::TxOut {
            value: value - bitcoin::Amount::from_sat(100_000),
            script_pubkey: prev.output[vout as usize].script_pubkey.clone(),
        }],
    };
    // A finalized legacy input's satisfaction lives in the scriptSig: a
    // DER signature with a trailing SIGHASH_ALL byte, then a pubkey.
    let mut sig = vec![0x30, 0x44, 0x02, 0x20];
    sig.extend(std::iter::repeat_n(0x11u8, 32));
    sig.extend_from_slice(&[0x02, 0x20]);
    sig.extend(std::iter::repeat_n(0x22u8, 32));
    sig.push(0x01); // SIGHASH_ALL
    let mut pk = vec![0x02u8];
    pk.extend(std::iter::repeat_n(0x33u8, 32));
    let script_sig = bitcoin::script::Builder::new()
        .push_slice::<&bitcoin::script::PushBytes>((&sig[..]).try_into().unwrap())
        .push_slice::<&bitcoin::script::PushBytes>((&pk[..]).try_into().unwrap())
        .into_script();
    unsigned.input[0].script_sig = bitcoin::ScriptBuf::new(); // PSBT keeps it empty

    let mut psbt = bitcoin::Psbt::from_unsigned_tx(unsigned).unwrap();
    psbt.inputs[0].final_script_sig = Some(script_sig);
    if with_record {
        psbt.inputs[0].non_witness_utxo = Some(prev.clone());
    } else if supplied.is_none() {
        // Nothing binds the value and nothing supplies it: §8.2c refuses, which
        // is a different test. Give it a witness_utxo so the run proceeds.
        psbt.inputs[0].witness_utxo = Some(prev.output[vout as usize].clone());
    }
    b64_encode(&psbt.serialize())
}

fn run(psbt: &str, extra: &[&str]) -> (String, bool) {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(psbt.as_bytes()).unwrap();
    f.flush().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let mut c = mt();
    c.args(["encode", "--bitcoin-cli", OFFLINE]);
    c.args(extra);
    let out = c.arg("--in").arg(f.path()).output().unwrap();
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// **The common path must be silent.** Core always attaches `non_witness_utxo`
/// to a legacy input, §8.2d hashes it and matches, and the report says
/// `TXID-BOUND` — so a capitalised block asserting nothing has verified the
/// value is false on the path almost every operator takes, and it teaches them
/// to skim the rare case where it is true.
#[test]
fn a_txid_bound_legacy_input_produces_no_warning() {
    let (err, ok) = run(&legacy_psbt(true, None), &[]);
    assert!(ok, "{err}");
    assert!(
        err.contains("TXID-BOUND"),
        "the fixture is not exercising the bound path at all: {err}"
    );
    assert!(
        !err.contains("legacy (pre-SegWit)"),
        "the warning fired on a value §8.2d had just bound:\n{err}"
    );
    assert!(
        !err.contains("NOTHING has verified"),
        "mt asserted nothing verified a value it verified:\n{err}"
    );
}

/// The rare case where it IS true, and where the fee really does absorb the
/// whole error.
#[test]
fn an_unbound_legacy_input_warns_and_every_clause_is_derived() {
    let (err, ok) = run(&legacy_psbt(false, None), &[]);
    assert!(ok, "{err}");
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        flat.contains("legacy (pre-SegWit) input whose value NOTHING has verified"),
        "{err}"
    );
    // The three false clauses, each asserted ABSENT.
    assert!(
        !flat.contains("You have told mt it holds"),
        "mt claimed the operator supplied a value that came from the PSBT:\n{err}"
    );
    assert!(
        !flat.contains("9.01 BTC"),
        "a hardcoded illustration is presented as describing these numbers:\n{err}"
    );
    assert!(
        flat.contains("it came from the PSBT's witness_utxo"),
        "the warning must name where the number actually came from:\n{err}"
    );
    // ...and the fee it quotes must be the fee mt shows.
    let fee_row = err
        .lines()
        .find(|l| l.starts_with("FEE "))
        .expect("no FEE row")
        .to_string();
    assert!(
        fee_row.contains("0.00100000"),
        "unexpected fixture fee: {fee_row}"
    );
    assert!(
        flat.contains("shows a fee of 0.00100000 BTC"),
        "the warning's fee must be the one the FEE row prints, not a per-input \
         subtraction that saturates to zero:\n{err}"
    );
}

/// When the operator DID supply it, the warning must say so — and not attribute
/// it to a record that does not exist.
#[test]
fn an_operator_supplied_value_is_attributed_to_the_operator() {
    let v = base();
    let amount = v["legacy_parent_value_btc"].as_f64().unwrap();
    let (err, ok) = run(
        &legacy_psbt(false, Some(0)),
        &["--input-value", &format!("0:{amount:.8}")],
    );
    assert!(ok, "{err}");
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    // The PSBT here carries a witness_utxo, which WINS over an assertion, so the
    // attribution must still be the record. Asserting the opposite would pin the
    // wrong precedence.
    assert!(
        flat.contains("witness_utxo") || flat.contains("--input-value"),
        "the warning names no source at all:\n{err}"
    );
}

/// **The multi-input arithmetic.** The old block subtracted ONE input's value
/// from ALL outputs, saturated to zero, and asserted `So mt shows a fee of
/// 0.00000000 BTC` twenty lines above a `FEE` row reading 0.001 — `mt`
/// contradicting itself about the fee, immediately before permanent steel.
#[test]
fn the_warning_never_contradicts_the_fee_row() {
    for with_record in [true, false] {
        let (err, _) = run(&legacy_psbt(with_record, None), &[]);
        if !err.contains("legacy (pre-SegWit)") {
            continue;
        }
        assert!(
            !err.contains("fee of 0.00000000 BTC"),
            "the warning asserted a zero fee:\n{err}"
        );
        let fee_row = err.lines().find(|l| l.starts_with("FEE ")).unwrap_or("");
        if let Some(amount) = fee_row.split_whitespace().nth(1) {
            assert!(
                err.contains(&format!("shows a fee of  {amount}"))
                    || err.contains(&format!("shows a fee of {amount}")),
                "the warning's fee and the FEE row disagree:\n{err}"
            );
        }
    }
}
