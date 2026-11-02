//! Bug-hunt triage rows (mt hunt 2026-10-31). Each test asserts the CORRECT
//! behaviour, so it fails on unmodified code when the reported bug is real.

use assert_cmd::Command;
use std::io::Write;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}
const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn corpus() -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn even_strings() -> Vec<String> {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

fn tmp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    f
}

fn run_in(verb: &str, extra: &[&str], content: &str) -> std::process::Output {
    let f = tmp(content);
    mt().arg(verb)
        .args(["--bitcoin-cli", OFFLINE])
        .args(extra)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap()
}

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// ---- PSBT builders for #1-#3 -------------------------------------------------

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

fn pb(b: &[u8]) -> &bitcoin::script::PushBytes {
    b.try_into().unwrap()
}

/// A one-input, one-output finalized PSBT whose input is `spk` worth 1,000,000
/// sats (a witness_utxo only), carrying the given finalized satisfaction.
fn finalized(
    spk: bitcoin::ScriptBuf,
    script_sig: Option<bitcoin::ScriptBuf>,
    witness: Option<bitcoin::Witness>,
) -> String {
    let unsigned = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: "11".repeat(32).parse().unwrap(),
                vout: 0,
            },
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence(0xFFFF_FFFD),
            witness: bitcoin::Witness::new(),
        }],
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(999_000),
            script_pubkey: spk.clone(),
        }],
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(unsigned).unwrap();
    psbt.inputs[0].witness_utxo = Some(bitcoin::TxOut {
        value: bitcoin::Amount::from_sat(1_000_000),
        script_pubkey: spk,
    });
    psbt.inputs[0].final_script_sig = script_sig;
    psbt.inputs[0].final_script_witness = witness;
    b64_encode(&psbt.serialize())
}

fn encode_psbt(b64: &str) -> std::process::Output {
    run_in("encode", &[], b64)
}

// ---- #1 ----------------------------------------------------------------------

/// A taproot script-path spend whose leaf needs no stack items has witness
/// `[script, control block]`. It carries no signature and must be refused.
#[test]
#[ignore = "RED row owed to FIX-A; its build removes this"]
fn bughunt_mt_01_keyless_taproot_scriptpath_not_signed() {
    let mut spk = vec![0x51, 0x20];
    spk.extend([0x44u8; 32]);
    let mut cb = vec![0xc0u8];
    cb.extend([0x55u8; 63]);
    cb.push(0x01); // 65 bytes, last byte 0x01
    let w = bitcoin::Witness::from_slice(&[vec![0x51u8], cb]); // leaf = OP_TRUE
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert!(
        !out.status.success(),
        "a witness [OP_TRUE, control block] carries no signature but encode accepted it:\n{}",
        s(&out.stderr)
    );
}

// ---- #2 ----------------------------------------------------------------------

/// A 64-byte DER+sighash ECDSA signature ending 0x02 (SIGHASH_NONE) must be
/// classed as ECDSA and refused by the SIGHASH_ALL rule.
#[test]
#[ignore = "RED row owed to FIX-A; its build removes this"]
fn bughunt_mt_02_64_byte_ecdsa_sighash_none_refused() {
    let mut sig = vec![0x30, 0x3d, 0x02, 0x20];
    sig.extend([0x11u8; 32]);
    sig.extend([0x02, 0x19]);
    sig.extend([0x22u8; 25]);
    sig.push(0x02); // SIGHASH_NONE
    assert_eq!(sig.len(), 64);
    let mut pk = vec![0x02u8];
    pk.extend([0x33u8; 32]);
    let mut spk = vec![0x00, 0x14];
    spk.extend([0x44u8; 20]);
    let w = bitcoin::Witness::from_slice(&[sig, pk]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert!(
        !out.status.success(),
        "a SIGHASH_NONE ECDSA signature was accepted:\n{}",
        s(&out.stderr)
    );
    assert!(
        s(&out.stderr).contains("sighash 0x02"),
        "expected the sighash refusal, got:\n{}",
        s(&out.stderr)
    );
}

// ---- #3 ----------------------------------------------------------------------

/// A finalized 2-of-3 P2SH multisig scriptSig `OP_0 <sig> <sig> <redeem>` is
/// validly signed; encode must not refuse it as unsigned.
#[test]
#[ignore = "RED row owed to FIX-A; its build removes this"]
fn bughunt_mt_03_p2sh_multisig_op0_dummy_counts_signatures() {
    use bitcoin::opcodes::all::{OP_CHECKMULTISIG, OP_PUSHBYTES_0, OP_PUSHNUM_2, OP_PUSHNUM_3};
    let mut sig = vec![0x30, 0x44, 0x02, 0x20];
    sig.extend([0x11u8; 32]);
    sig.extend([0x02, 0x20]);
    sig.extend([0x22u8; 32]);
    sig.push(0x01);
    let pks: Vec<Vec<u8>> = (0..3u8)
        .map(|i| {
            let mut p = vec![0x02u8];
            p.extend([0x30 + i; 32]);
            p
        })
        .collect();
    let redeem = bitcoin::script::Builder::new()
        .push_opcode(OP_PUSHNUM_2)
        .push_slice(pb(&pks[0]))
        .push_slice(pb(&pks[1]))
        .push_slice(pb(&pks[2]))
        .push_opcode(OP_PUSHNUM_3)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script();
    let script_sig = bitcoin::script::Builder::new()
        .push_opcode(OP_PUSHBYTES_0)
        .push_slice(pb(&sig))
        .push_slice(pb(&sig))
        .push_slice(pb(redeem.as_bytes()))
        .into_script();
    let spk = bitcoin::ScriptBuf::new_p2sh(&redeem.script_hash());
    let out = encode_psbt(&finalized(spk, Some(script_sig), None));
    assert!(
        out.status.success(),
        "a validly signed 2-of-3 P2SH input was refused:\n{}",
        s(&out.stderr)
    );
}

// ---- FIX-A rows (rulings R1-R3) ------------------------------------------------

/// Asserts exit code 1 and that stderr's FIRST line is exactly `verdict`.
fn assert_refused_with(out: &std::process::Output, verdict: &str) {
    let err = s(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 with `{verdict}`, got {:?}:\n{err}",
        out.status.code()
    );
    assert_eq!(
        err.lines().next().unwrap_or(""),
        verdict,
        "wrong verdict line:\n{err}"
    );
}

/// A taproot output script (`OP_1 <32 bytes>`).
fn p2tr_spk() -> bitcoin::ScriptBuf {
    let mut spk = vec![0x51, 0x20];
    spk.extend([0x44u8; 32]);
    bitcoin::ScriptBuf::from(spk)
}

/// A depth-1 BIP-341 control block (65 bytes) whose last byte is 0x01, so a
/// length-only recognizer reads it as a Schnorr signature with SIGHASH_ALL.
fn control_block_65() -> Vec<u8> {
    let mut cb = vec![0xc0u8];
    cb.extend([0x55u8; 63]);
    cb.push(0x01);
    cb
}

/// A DER ECDSA signature of `der_len` bytes after `30 <len>` (r of 32 bytes,
/// s filling the rest) followed by the sighash byte `flag`.
fn ecdsa_sig(der_len: u8, flag: u8) -> Vec<u8> {
    let s_len = der_len - 2 - 32 - 2;
    let mut sig = vec![0x30, der_len, 0x02, 0x20];
    sig.extend([0x11u8; 32]);
    sig.extend([0x02, s_len]);
    sig.extend(std::iter::repeat_n(0x22u8, s_len as usize));
    sig.push(flag);
    sig
}

/// A compressed public key.
fn pk33() -> Vec<u8> {
    let mut p = vec![0x02u8];
    p.extend([0x33u8; 32]);
    p
}

/// R1: the bughunt 01 witness is refused with the existing §8.6(b) text, exit 1.
#[test]
fn bughunt_mt_01b_keyless_scriptpath_refusal_text() {
    let w = bitcoin::Witness::from_slice(&[vec![0x51u8], control_block_65()]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// R1: the annex is removed first; `[OP_TRUE, control block, annex]` is the
/// same keyless script-path spend and is refused the same way.
#[test]
fn bughunt_mt_01c_keyless_scriptpath_with_annex_refused() {
    let w = bitcoin::Witness::from_slice(&[vec![0x51u8], control_block_65(), vec![0x50, 0xaa]]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// R1 guard (GREEN before and after): a P2WPKH witness `[sig, pubkey]` has two
/// elements but its last is no control block, so it is not a script path and
/// its signature still counts.
#[test]
fn bughunt_mt_01d_p2wpkh_two_elements_is_not_scriptpath() {
    let mut spk = vec![0x00, 0x14];
    spk.extend([0x44u8; 20]);
    let w = bitcoin::Witness::from_slice(&[ecdsa_sig(0x44, 0x01), pk33()]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert!(
        out.status.success(),
        "a signed P2WPKH input was refused:\n{}",
        s(&out.stderr)
    );
}

/// R2: the bughunt 02 witness is refused with the existing §8.6(a) text, exit 1.
#[test]
fn bughunt_mt_02b_64_byte_ecdsa_sighash_none_refusal_text() {
    let sig = ecdsa_sig(0x3d, 0x02);
    assert_eq!(sig.len(), 64);
    let mut spk = vec![0x00, 0x14];
    spk.extend([0x44u8; 20]);
    let w = bitcoin::Witness::from_slice(&[sig, pk33()]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL",
    );
}

/// R2 guard (GREEN before and after): a 73-byte (`30 46`, high-S) SIGHASH_NONE
/// signature is still refused; the DER range is not narrowed.
#[test]
fn bughunt_mt_02c_73_byte_ecdsa_sighash_none_still_refused() {
    let sig = ecdsa_sig(0x46, 0x02);
    assert_eq!(sig.len(), 73);
    let mut spk = vec![0x00, 0x14];
    spk.extend([0x44u8; 20]);
    let w = bitcoin::Witness::from_slice(&[sig, pk33()]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL",
    );
}

/// R3 extra row: `OP_0 <sig NONE> <redeem>` reaches the signature after the
/// OP_0 and is refused by R2's sighash rule, not as unsigned.
#[test]
fn bughunt_mt_03b_p2sh_op0_sighash_none_still_refused() {
    use bitcoin::opcodes::all::{OP_CHECKMULTISIG, OP_PUSHBYTES_0, OP_PUSHNUM_1};
    let redeem = bitcoin::script::Builder::new()
        .push_opcode(OP_PUSHNUM_1)
        .push_slice(pb(&pk33()))
        .push_opcode(OP_PUSHNUM_1)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script();
    let script_sig = bitcoin::script::Builder::new()
        .push_opcode(OP_PUSHBYTES_0)
        .push_slice(pb(&ecdsa_sig(0x44, 0x02)))
        .push_slice(pb(redeem.as_bytes()))
        .into_script();
    let spk = bitcoin::ScriptBuf::new_p2sh(&redeem.script_hash());
    let out = encode_psbt(&finalized(spk, Some(script_sig), None));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL",
    );
}

/// R3: a small-integer push (OP_1) before the signature is a push, so the walk
/// reaches the signature: `OP_1 <sig> <redeem>` encodes.
#[test]
fn bughunt_mt_03c_small_int_before_sig_counts() {
    use bitcoin::opcodes::all::{OP_CHECKSIG, OP_DROP, OP_PUSHNUM_1};
    let redeem = bitcoin::script::Builder::new()
        .push_opcode(OP_DROP)
        .push_slice(pb(&pk33()))
        .push_opcode(OP_CHECKSIG)
        .into_script();
    let script_sig = bitcoin::script::Builder::new()
        .push_opcode(OP_PUSHNUM_1)
        .push_slice(pb(&ecdsa_sig(0x44, 0x01)))
        .push_slice(pb(redeem.as_bytes()))
        .into_script();
    let spk = bitcoin::ScriptBuf::new_p2sh(&redeem.script_hash());
    let out = encode_psbt(&finalized(spk, Some(script_sig), None));
    assert!(
        out.status.success(),
        "a signed input with a leading OP_1 was refused:\n{}",
        s(&out.stderr)
    );
}

/// R3: OP_PUSHDATA4 (0x4e, 4-byte little-endian length) is a push, so a
/// signature it pushes counts: `PUSHDATA4 <sig> <pubkey>` on P2PKH encodes.
#[test]
fn bughunt_mt_03d_pushdata4_sig_counts() {
    let sig = ecdsa_sig(0x44, 0x01);
    let mut ss = vec![0x4eu8];
    ss.extend((sig.len() as u32).to_le_bytes());
    ss.extend(&sig);
    ss.push(33);
    ss.extend(pk33());
    let spk = bitcoin::ScriptBuf::new_p2pkh(
        &<bitcoin::PubkeyHash as bitcoin::hashes::Hash>::from_byte_array([0x44; 20]),
    );
    let out = encode_psbt(&finalized(spk, Some(bitcoin::ScriptBuf::from(ss)), None));
    assert!(
        out.status.success(),
        "a signature pushed by OP_PUSHDATA4 was not counted:\n{}",
        s(&out.stderr)
    );
}

// ---- #5 ----------------------------------------------------------------------

/// `getindexinfo` says txindex is still syncing while another index is synced.
/// A parent lookup that finds nothing then proves nothing: the status must be
/// UNKNOWN, not PENDING.
#[test]
#[ignore = "RED row owed to FIX-D; its build removes this"]
fn bughunt_mt_05_txindex_unsynced_other_index_synced_is_unknown() {
    let script = r#"#!/bin/sh
verb=""
while read -r line; do
  if [ -z "$verb" ]; then verb="$line"; fi
done
case "$verb" in
  getblockcount) echo 963832 ;;
  getindexinfo)  echo '{"txindex": {"synced": false, "best_block_height": 10}, "basic block filter index": {"synced": true, "best_block_height": 963832}}' ;;
  *) exit 1 ;;
esac
"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bitcoin-cli");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let f = tmp(&even_strings().join("\n"));
    let out = mt()
        .args(["inspect", "--bitcoin-cli"])
        .arg(&path)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    let o = s(&out.stdout);
    assert!(out.status.success(), "{}", s(&out.stderr));
    assert!(
        !o.contains("STATUS    PENDING"),
        "txindex is NOT synced yet a missing parent was reported PENDING:\n{o}"
    );
    assert!(o.contains("STATUS    UNKNOWN"), "expected UNKNOWN:\n{o}");
}

// ---- #7 ----------------------------------------------------------------------

#[test]
#[ignore = "RED row owed to FIX-C; its build removes this"]
fn bughunt_mt_07_non_ascii_near_line_start_does_not_panic() {
    let out = mt()
        .args(["decode", "--bitcoin-cli", OFFLINE])
        .write_stdin("MT1aaaaaaa\u{e9}qqqqqqqqqq\n")
        .output()
        .unwrap();
    assert!(
        !s(&out.stderr).contains("panicked") && out.status.code() != Some(101),
        "decode panicked on holder input (rc {:?}):\n{}",
        out.status.code(),
        s(&out.stderr)
    );
}

// ---- #9 ----------------------------------------------------------------------

/// If a report shows a FEE amount, its warning must not say the fee is UNKNOWN.
/// Checked on every offline path that prints a report from a PSBT.
#[test]
fn bughunt_mt_09_offline_warning_does_not_contradict_fee_row() {
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/p5_base.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let psbt = v["finalized_psbt_b64"].as_str().unwrap().to_string();
    for verb in ["inspect", "encode"] {
        let out = run_in(verb, &[], &psbt);
        let err = s(&out.stderr);
        let o = s(&out.stdout);
        let all = format!("{o}\n{err}");
        let shows_amount = all
            .lines()
            .any(|l| l.starts_with("FEE ") && l.contains(" BTC"));
        let flat = all.split_whitespace().collect::<Vec<_>>().join(" ");
        let says_unknown = flat.contains("what fee does it pay? UNKNOWN");
        assert!(
            !(shows_amount && says_unknown),
            "{verb}: FEE row shows an amount but the warning says UNKNOWN:\n{all}"
        );
    }
}

// ---- #10 ---------------------------------------------------------------------

/// `inspect --json` prints its document on stdout; `decode --json` prints it on
/// stderr (stdout stays the bare hex), so the caller names the stream.
fn json_warnings(doc: &str) -> Vec<String> {
    let j: serde_json::Value =
        serde_json::from_str(doc).unwrap_or_else(|e| panic!("no JSON document ({e}):\n{doc}"));
    j["warnings"]
        .as_array()
        .expect("no warnings array")
        .iter()
        .map(|w| w.as_str().unwrap().to_lowercase())
        .collect()
}

/// A duplicated chunk, and a chunk needing repair, are reported in prose; the
/// JSON `warnings` array must carry them too (exact wording is open).
#[test]
#[ignore = "RED row owed to FIX-C; its build removes this"]
fn bughunt_mt_10_inspect_json_carries_duplicate_and_correction_notices() {
    let st = even_strings();
    let mut dup = st.clone();
    dup.push(st[0].clone());
    let o = run_in("inspect", &["--json"], &dup.join("\n"));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let w = json_warnings(&s(&o.stdout));
    assert!(
        w.iter()
            .any(|x| x.contains("twice") || x.contains("duplicate")),
        "inspect --json omits the duplicate-chunk notice; warnings = {w:?}"
    );

    let mut bad = st.clone();
    let mut c: Vec<char> = bad[0].chars().collect();
    c[10] = if c[10] == 'q' { 'p' } else { 'q' };
    bad[0] = c.into_iter().collect();
    let o = run_in("inspect", &["--json"], &bad.join("\n"));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let w = json_warnings(&s(&o.stdout));
    assert!(
        w.iter()
            .any(|x| x.contains("correct") || x.contains("repair")),
        "inspect --json omits the correction notice; warnings = {w:?}"
    );
    let o = run_in("decode", &["--json"], &bad.join("\n"));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let w = json_warnings(&s(&o.stderr));
    assert!(
        w.iter()
            .any(|x| x.contains("correct") || x.contains("repair")),
        "decode --json omits the correction notice; warnings = {w:?}"
    );
}

// ---- #11 ---------------------------------------------------------------------

/// OPEN product question: refuse the combination, or honour --json. Either is
/// fine; silently exiting 0 with bare hex and no JSON is not.
#[test]
#[ignore = "RED row owed to FIX-C; its build removes this"]
fn bughunt_mt_11_decode_json_quiet_does_not_silently_drop_json() {
    let out = run_in("decode", &["--json", "--quiet"], &even_strings().join("\n"));
    // `decode --json` puts its document on stderr (stdout is the hex).
    let e = s(&out.stderr);
    assert!(
        !out.status.success() || e.trim_start().starts_with('{'),
        "decode --json --quiet exited 0 and ignored --json; stderr:\n{e}\nstdout:\n{}",
        s(&out.stdout)
    );
}

// ---- #12 ---------------------------------------------------------------------

/// A plate with `mt1` misread as `mtl` AND one `6` misread as `b`: both are
/// repairable by the reader's own transliteration, so decode should succeed
/// with the same bytes as the clean plate. (Product question: is accepting two
/// simultaneous repairs wanted? The report says yes.)
#[test]
#[ignore = "RED row owed to FIX-C; its build removes this"]
fn bughunt_mt_12_mtl_prefix_plus_confusable_still_decodes() {
    let st = even_strings();
    let clean = run_in("decode", &[], &st.join("\n"));
    assert!(clean.status.success());
    let mut bad = st.clone();
    let (i, pos) = st
        .iter()
        .enumerate()
        .find_map(|(i, x)| x[3..].find('6').map(|p| (i, p + 3)))
        .expect("no '6' in the fixture strings");
    let mut c: Vec<char> = bad[i].chars().collect();
    c[pos] = 'b';
    c[2] = 'l';
    bad[i] = c.into_iter().collect();
    let out = run_in("decode", &[], &bad.join("\n"));
    assert!(
        out.status.success(),
        "mtl + one 6->b was refused:\n{}",
        s(&out.stderr)
    );
    assert_eq!(s(&out.stdout), s(&clean.stdout));
}

/// FIX-B R4 at the CLI: chunks from sets of different sizes reach
/// `explain_failure`'s generic fallback with the `CountMismatch` text, exit 1,
/// for every verb that reads a set (decode, verify, inspect).
#[test]
fn bughunt_mt_04_cli_count_mismatch_refusal() {
    let txid = "abcde00000000000000000000000000000000000000000000000000000000000";
    let a = mt_codec::encode(&[0x11u8; 70], txid).unwrap();
    let b = mt_codec::encode(&[0x22u8; 100], txid).unwrap();
    assert_eq!((a.len(), b.len()), (2, 3), "need count 2 vs 3");
    let input = format!("{}\n{}\n", a[0], b[1]);
    for verb in ["decode", "verify", "inspect"] {
        let out = run_in(verb, &[], &input);
        let err = s(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{verb}: {err}");
        assert!(
            out.stdout.is_empty(),
            "{verb} wrote stdout: {}",
            s(&out.stdout)
        );
        assert_eq!(
            err.lines().next(),
            Some(format!("mt {verb}: REFUSED — §1.1, the set does not verify").as_str()),
            "{verb}: {err}"
        );
        assert!(
            err.contains("chunk set size mismatch: expected 2 chunks, chunk 2 carries 3"),
            "{verb}: {err}"
        );
    }
}
