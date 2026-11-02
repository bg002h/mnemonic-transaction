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

/// QA3: a legacy P2PKH `<sig ALL> <65-byte uncompressed pubkey 04..>` is an
/// honest signed spend. A scriptSig push is never Schnorr, so the pubkey's last
/// byte (here 0x07) is not a sighash and the input encodes.
#[test]
fn bughunt_mt_03e_uncompressed_p2pkh_encodes() {
    let mut pk = vec![0x04u8];
    pk.extend([0x55u8; 63]);
    pk.push(0x07);
    let sig = ecdsa_sig(0x44, 0x01);
    let mut ss = vec![sig.len() as u8];
    ss.extend(&sig);
    ss.push(65);
    ss.extend(&pk);
    let spk = bitcoin::ScriptBuf::new_p2pkh(
        &<bitcoin::PubkeyHash as bitcoin::hashes::Hash>::from_byte_array([0x44; 20]),
    );
    let out = encode_psbt(&finalized(spk, Some(bitcoin::ScriptBuf::from(ss)), None));
    assert!(
        out.status.success(),
        "an uncompressed-key P2PKH input was refused:\n{}",
        s(&out.stderr)
    );
}

/// A v0 P2WSH witness `[32-byte preimage, script]` with a `script_len`-byte
/// witnessScript and no signature anywhere.
fn p2wsh_keyless(script_len: usize) -> std::process::Output {
    // OP_SHA256 <32> OP_EQUALVERIFY, OP_NOP padding, then OP_TRUE.
    let mut ws = vec![0xa8u8, 0x20];
    ws.extend([0x66u8; 32]);
    ws.push(0x88);
    while ws.len() < script_len - 1 {
        ws.push(0x61);
    }
    ws.push(0x51);
    assert_eq!(ws.len(), script_len);
    let mut spk = vec![0x00, 0x20];
    spk.extend([0x44u8; 32]);
    let w = bitcoin::Witness::from_slice(&[vec![0x77u8; 32], ws]);
    encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)))
}

/// QA1: a 64-byte witnessScript in a v0 witness is not a Schnorr signature;
/// the keyless input is refused with the existing "no signature" text.
#[test]
fn bughunt_mt_01e_p2wsh_keyless_64_byte_script_refused() {
    assert_refused_with(
        &p2wsh_keyless(64),
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// QA1: the same with a 65-byte witnessScript (not an explicit-sighash Schnorr
/// signature), refused with the "no signature" text, not a sighash text.
#[test]
fn bughunt_mt_01f_p2wsh_keyless_65_byte_script_refused() {
    assert_refused_with(
        &p2wsh_keyless(65),
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// QA1 guard (GREEN before and after): a taproot key-path witness has one
/// element, so its context is Taproot; an honest 64-byte Schnorr signature
/// that happens to begin `30 3d` and end `0x02` is SIGHASH_DEFAULT and encodes.
#[test]
fn bughunt_mt_01g_taproot_keypath_303d_schnorr_encodes() {
    let mut sig = vec![0x30u8, 0x3d];
    sig.extend([0x5au8; 61]);
    sig.push(0x02);
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[sig])),
    ));
    assert!(
        out.status.success(),
        "an honest key-path Schnorr signature was refused:\n{}",
        s(&out.stderr)
    );
}

/// QA1 guard (GREEN before and after): the context is computed before the
/// script-path pops, so a two-signature script path `[sig64, sig64, script,
/// control block]` is Taproot and both 64-byte signatures count.
#[test]
fn bughunt_mt_01h_taproot_scriptpath_two_sigs_encodes() {
    let w = bitcoin::Witness::from_slice(&[
        vec![0x5au8; 64],
        vec![0x5bu8; 64],
        vec![0x51u8],
        control_block_65(),
    ]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert!(
        out.status.success(),
        "a signed taproot script path was refused:\n{}",
        s(&out.stderr)
    );
}

/// QA1: a 65-byte witnessScript whose last byte is 0x01 (here the data push
/// `01 01`) is not a Schnorr signature with SIGHASH_ALL; the keyless input is
/// refused with the "no signature" text.
#[test]
fn bughunt_mt_01i_p2wsh_keyless_65_byte_script_ending_01_refused() {
    let mut ws = vec![0xa8u8, 0x20];
    ws.extend([0x66u8; 32]);
    ws.push(0x88);
    while ws.len() < 63 {
        ws.push(0x61);
    }
    ws.extend([0x01, 0x01]);
    assert_eq!(ws.len(), 65);
    let mut spk = vec![0x00, 0x20];
    spk.extend([0x44u8; 32]);
    let w = bitcoin::Witness::from_slice(&[vec![0x77u8; 32], ws]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// R1: a two-element script path `[script, control block]` is a script path,
/// so its script is popped and never read as a signature, even when the
/// script (here a 49-byte push whose bytes are DER-shaped and end 0x01) looks
/// like an ECDSA signature with SIGHASH_ALL.
#[test]
fn bughunt_mt_01j_two_element_scriptpath_der_shaped_script_refused() {
    let mut script = vec![0x30u8, 0x2e];
    script.extend([0x01u8; 47]);
    assert_eq!(script.len(), 49);
    let w = bitcoin::Witness::from_slice(&[script, control_block_65()]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
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

// ---- #5, FIX-D rows (ruling R5) -------------------------------------------
//
// R5: `has_txindex` parses the `getindexinfo` reply as JSON and returns true
// only when the top-level object has a member `txindex` that is an object whose
// own member `synced` is the boolean `true`. Anything else, including a parse
// failure, is false, which reads UNKNOWN (§6a's fail-safe direction). Each row
// drives `mt inspect` against a stub `bitcoin-cli` whose `getindexinfo` prints
// `reply`; `gettxout` and `getrawtransaction` fail, so every parent is NOT
// FOUND and the STATUS row depends on `has_txindex` alone.

/// Runs `mt inspect` on the `even` vector against a stub node; returns
/// (rc, stdout, stderr).
fn inspect_with_indexinfo(reply: &str) -> (Option<i32>, String, String) {
    assert!(
        !reply.contains('\''),
        "the stub echoes the reply in single quotes"
    );
    let script = r#"#!/bin/sh
verb=""
while read -r line; do
  if [ -z "$verb" ]; then verb="$line"; fi
done
case "$verb" in
  getblockcount) echo 963832 ;;
  getindexinfo)  echo '@REPLY@' ;;
  *) exit 1 ;;
esac
"#
    .replace("@REPLY@", reply);
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
    (out.status.code(), s(&out.stdout), s(&out.stderr))
}

fn assert_status(reply: &str, want: &str, why: &str) {
    let (rc, o, e) = inspect_with_indexinfo(reply);
    assert_eq!(rc, Some(0), "inspect failed:\n{e}");
    let other = if want == "PENDING" {
        "UNKNOWN"
    } else {
        "PENDING"
    };
    assert!(
        o.contains(&format!("STATUS    {want}")) && !o.contains(&format!("STATUS    {other}")),
        "{why}; reply {reply}; expected STATUS {want}:\n{o}"
    );
}

/// Compact JSON (no space after the colon) is the same document as the
/// pretty one: a synced txindex reads PENDING.
#[test]
fn bughunt_mt_05b_txindex_compact_json_synced_is_pending() {
    assert_status(
        r#"{"txindex":{"synced":true,"best_block_height":963832}}"#,
        "PENDING",
        "a synced txindex in compact JSON was not recognised",
    );
}

/// A truncated reply does not parse, so it proves nothing: UNKNOWN.
#[test]
fn bughunt_mt_05c_txindex_truncated_reply_is_unknown() {
    assert_status(
        r#"{"txindex": {"synced": true, "best_block_height": 963832}"#,
        "UNKNOWN",
        "a reply that is not valid JSON was trusted",
    );
}

/// Only txindex's OWN `synced` member counts, not one nested deeper.
#[test]
fn bughunt_mt_05d_txindex_nested_synced_true_is_unknown() {
    assert_status(
        r#"{"txindex": {"synced": false, "best_block_height": 10, "detail": {"synced": true}}}"#,
        "UNKNOWN",
        "a nested synced:true was read as txindex's own",
    );
}

/// No `txindex` member at all: the word appearing as a value does not count.
#[test]
fn bughunt_mt_05e_txindex_absent_name_in_value_is_unknown() {
    assert_status(
        r#"{"coinstatsindex": {"synced": true, "best_block_height": 963832}, "note": "txindex"}"#,
        "UNKNOWN",
        "a reply with no txindex member was read as a synced txindex",
    );
}

/// Guard (passes today, must keep passing): Core's pretty form, synced.
#[test]
fn bughunt_mt_05f_txindex_pretty_synced_true_is_pending() {
    assert_status(
        r#"{"txindex": {"synced": true, "best_block_height": 963832}}"#,
        "PENDING",
        "a synced txindex was not recognised",
    );
}

/// Guard (passes today, must keep passing): `synced` must be the boolean.
#[test]
fn bughunt_mt_05g_txindex_synced_string_is_unknown() {
    assert_status(
        r#"{"txindex": {"synced": "true", "best_block_height": 963832}}"#,
        "UNKNOWN",
        "the string \"true\" was read as the boolean",
    );
}

// ---- #7 ----------------------------------------------------------------------

#[test]
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

// ---- whole-diff s1: I1 RED row and survivor-killing rows -----------------------

/// I1: a keyless v0 P2WSH `[32 bytes, 64-byte witnessScript]` whose script is
/// DER-shaped (`30 3d`, 48-byte push, OP_2DROP, OP_NOPs, `01 01`) is not a
/// signature: BIP-141's last witness item is the witnessScript.
#[test]
fn bughunt_mt_01k_p2wsh_keyless_der_shaped_64_byte_script_refused() {
    let mut ws = vec![0x30u8, 0x3d];
    ws.extend([0x99u8; 47]);
    ws.push(0x6d);
    while ws.len() < 62 {
        ws.push(0x61);
    }
    ws.extend([0x01, 0x01]);
    assert_eq!(ws.len(), 64);
    let mut spk = vec![0x00, 0x20];
    spk.extend([0x44u8; 32]);
    let w = bitcoin::Witness::from_slice(&[vec![0x77u8; 32], ws]);
    let out = encode_psbt(&finalized(bitcoin::ScriptBuf::from(spk), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

/// Survivor validate.rs:1186 (honest side): a key-path 65-byte Schnorr
/// signature with explicit SIGHASH_ALL encodes.
#[test]
fn bughunt_mt_01l_taproot_keypath_65_byte_schnorr_all_encodes() {
    let mut sig = vec![0x5au8; 64];
    sig.push(0x01);
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[sig])),
    ));
    assert!(
        out.status.success(),
        "honest 65-byte key path refused:\n{}",
        s(&out.stderr)
    );
}

/// Survivor validate.rs:1186 (fail-open side): a script path whose second
/// signature is 65-byte Schnorr SIGHASH_NONE is refused by its sighash.
#[test]
fn bughunt_mt_01m_taproot_scriptpath_65_byte_none_refused() {
    let mut none = vec![0x5bu8; 64];
    none.push(0x02);
    let w =
        bitcoin::Witness::from_slice(&[vec![0x5au8; 64], none, vec![0x51u8], control_block_65()]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL",
    );
}

/// Survivor validate.rs:1265 (R3, OP_1NEGATE): P2SH
/// `OP_1NEGATE <sig ALL> <redeem: <pk> CHECKSIGVERIFY OP_1NEGATE OP_EQUAL>`
/// counts the signature after the OP_1NEGATE push and encodes.
#[test]
fn bughunt_mt_03f_op1negate_before_sig_counts() {
    let mut redeem = vec![0x21u8, 0x02];
    redeem.extend([0x30u8; 32]);
    redeem.extend([0xad, 0x4f, 0x87]);
    let mut sig = vec![0x30u8, 68];
    sig.resize(70, 0x22);
    sig.push(0x01);
    let mut ss = vec![0x4fu8, sig.len() as u8];
    ss.extend(&sig);
    ss.push(redeem.len() as u8);
    ss.extend(&redeem);
    let mut spk = vec![0xa9u8, 0x14];
    spk.extend([0x44u8; 20]);
    spk.push(0x87);
    let out = encode_psbt(&finalized(
        bitcoin::ScriptBuf::from(spk),
        Some(bitcoin::ScriptBuf::from(ss)),
        None,
    ));
    assert!(
        out.status.success(),
        "OP_1NEGATE-led P2SH refused:\n{}",
        s(&out.stderr)
    );
}

/// Survivor validate.rs:1264 (PUSHDATA4 guard): a scriptSig that is a
/// truncated `OP_PUSHDATA4` is refused, not a panic.
#[test]
fn bughunt_mt_03g_truncated_pushdata4_refused() {
    let mut spk = vec![0xa9u8, 0x14];
    spk.extend([0x44u8; 20]);
    spk.push(0x87);
    let out = encode_psbt(&finalized(
        bitcoin::ScriptBuf::from(spk),
        Some(bitcoin::ScriptBuf::from(vec![0x4eu8, 0x00])),
        None,
    ));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature",
    );
}

// ---- s2 killing rows ---------------------------------------------------------

const NOSIG: &str = "mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature";

fn p2pkh_spk() -> bitcoin::ScriptBuf {
    bitcoin::ScriptBuf::new_p2pkh(
        &<bitcoin::PubkeyHash as bitcoin::hashes::Hash>::from_byte_array([0x44; 20]),
    )
}

fn schnorr65(first: u8, flag: u8) -> Vec<u8> {
    let mut v = vec![first];
    v.extend([0x5au8; 63]);
    v.push(flag);
    v
}

#[test]
fn s2_a_taproot_65_byte_sighash_none_refused() {
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[schnorr65(0x5a, 0x02)])),
    ));
    assert_refused_with(
        &out,
        "mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL",
    );
}

#[test]
fn s2_a2_taproot_65_byte_sighash_all_encodes() {
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[schnorr65(0x5a, 0x01)])),
    ));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_b_op1negate_before_sig_encodes() {
    let sig = ecdsa_sig(0x44, 0x01);
    let mut ss = vec![0x4fu8, sig.len() as u8];
    ss.extend(&sig);
    ss.push(33);
    ss.extend(pk33());
    let out = encode_psbt(&finalized(
        p2pkh_spk(),
        Some(bitcoin::ScriptBuf::from(ss)),
        None,
    ));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_b2_op16_before_sig_encodes() {
    let sig = ecdsa_sig(0x44, 0x01);
    let mut ss = vec![0x60u8, sig.len() as u8];
    ss.extend(&sig);
    ss.push(33);
    ss.extend(pk33());
    let out = encode_psbt(&finalized(
        p2pkh_spk(),
        Some(bitcoin::ScriptBuf::from(ss)),
        None,
    ));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_b3_op_nop_ends_walk_before_sig() {
    let sig = ecdsa_sig(0x44, 0x01);
    let mut ss = vec![0x61u8, sig.len() as u8];
    ss.extend(&sig);
    let out = encode_psbt(&finalized(
        p2pkh_spk(),
        Some(bitcoin::ScriptBuf::from(ss)),
        None,
    ));
    assert_refused_with(&out, NOSIG);
}

#[test]
fn s2_c_truncated_pushdata_refuses_not_panics() {
    for ss in [vec![0x4eu8, 0x01], vec![0x4d, 0x01], vec![0x4c]] {
        let out = encode_psbt(&finalized(
            p2pkh_spk(),
            Some(bitcoin::ScriptBuf::from(ss.clone())),
            None,
        ));
        assert_refused_with(&out, NOSIG);
    }
}

#[test]
fn s2_d_annex_signed_scriptpath_encodes() {
    let w = bitcoin::Witness::from_slice(&[
        vec![0x5au8; 64],
        vec![0x51u8],
        control_block_65(),
        vec![0x50u8, 0xaa],
    ]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_d2_annex_keypath_encodes() {
    let w = bitcoin::Witness::from_slice(&[vec![0x5au8; 64], vec![0x50u8, 0xaa]]);
    let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_d3_keypath_sig_starting_0x50_is_not_annex() {
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[vec![0x50u8; 64]])),
    ));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_e_keypath_65_byte_sig_starting_c0_is_not_scriptpath() {
    let out = encode_psbt(&finalized(
        p2tr_spk(),
        None,
        Some(bitcoin::Witness::from_slice(&[schnorr65(0xc0, 0x01)])),
    ));
    assert!(out.status.success(), "{}", s(&out.stderr));
}

#[test]
fn s2_f_keyless_scriptpath_with_64_and_65_byte_leaf_refused() {
    for n in [64usize, 65] {
        let mut leaf = vec![0x61u8; n];
        leaf[n - 1] = 0x51;
        let w = bitcoin::Witness::from_slice(&[leaf, control_block_65()]);
        let out = encode_psbt(&finalized(p2tr_spk(), None, Some(w)));
        assert_refused_with(&out, NOSIG);
    }
}
// ---- FIX-C rows (design/FIX_bughunt_mt_c.md) ---------------------------------
//
// The rows above check only the absence of the bug; these pin the rulings'
// exact texts (grade R7, R9-R12).

/// Whitespace-flattened stderr: the refusal body is wrapped at 68 columns, so
/// a sentence is matched on its words, not its line breaks.
fn flat(b: &[u8]) -> String {
    s(b).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// R7: a cut point inside a multi-byte character is an unreadable string, and
/// the CLI says so with the existing `string {n} could not be read` refusal.
#[test]
fn bughunt_mt_07b_non_ascii_cut_point_is_an_unreadable_string() {
    for verb in ["decode", "inspect", "verify"] {
        let out = run_in(verb, &[], "MT1aaaaaaa\u{e9}qqqqqqqqqq\n");
        let e = s(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{verb}: rc {:?}\n{e}",
            out.status.code()
        );
        assert!(out.stdout.is_empty(), "{verb}: stdout not empty");
        assert!(!e.contains("panicked"), "{verb} panicked:\n{e}");
        let first = e.lines().next().unwrap_or("");
        assert!(
            first.starts_with(&format!("mt {verb}: REFUSED"))
                && first.contains("string 1 could not be read"),
            "{verb}: first line is not the unreadable-string refusal:\n{e}"
        );
    }
}

/// R7 with elided lines present: the first line (full form) has a non-ASCII
/// character at byte 10 and the five elided lines need its first 11 bytes as
/// their prefix. No panic; a refusal, exit 1, with R7's existing `string {n}
/// could not be read` text (architect QC2: `n` is the 1-based position of the
/// prefix line).
#[test]
fn bughunt_mt_07c_non_ascii_in_the_prefix_line_with_elided_lines_refuses() {
    let st = even_strings();
    let mut c: Vec<char> = st[0].chars().collect();
    c[10] = '\u{e9}';
    let mut lines = vec![c.into_iter().collect::<String>()];
    lines.extend(st[1..].iter().map(|x| x[11..].to_string()));
    for verb in ["decode", "inspect", "verify"] {
        let out = run_in(verb, &[], &lines.join("\n"));
        let e = s(&out.stderr);
        assert!(!e.contains("panicked"), "{verb} panicked:\n{e}");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{verb}: rc {:?}\n{e}",
            out.status.code()
        );
        assert!(out.stdout.is_empty(), "{verb}: stdout not empty");
        assert_eq!(
            e.lines().next().unwrap_or(""),
            format!(
                "mt {verb}: REFUSED — §1.1, string 1 could not be read: more than 4 characters differ from what was engraved"
            ),
            "{verb}: wrong first line:\n{e}"
        );
    }
}

/// R7 (architect QC2, option c): with a second intact full line, the prefix is
/// taken from it, the set reads through the main path, and the unreadable
/// string 1 is refused with the same first line (richer mechanism follows).
#[test]
fn bughunt_mt_07d_second_full_line_supplies_the_prefix() {
    let st = even_strings();
    let mut c: Vec<char> = st[0].chars().collect();
    c[10] = '\u{e9}';
    let mut lines = vec![c.into_iter().collect::<String>(), st[1].clone()];
    lines.extend(st[2..].iter().map(|x| x[11..].to_string()));
    let out = run_in("decode", &[], &lines.join("\n"));
    let e = s(&out.stderr);
    assert!(!e.contains("panicked"), "decode panicked:\n{e}");
    assert_eq!(
        out.status.code(),
        Some(1),
        "rc {:?}\n{e}",
        out.status.code()
    );
    assert!(out.stdout.is_empty(), "stdout not empty");
    assert_eq!(
        e.lines().next().unwrap_or(""),
        "mt decode: REFUSED — §1.1, string 1 could not be read: more than 4 characters differ from what was engraved"
    );
    let flat = e.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("read cleanly and 1 did not"),
        "not the main path:\n{e}"
    );
}

/// R9: the duplicate-chunk notice in `inspect --json` is decode's existing text.
#[test]
fn bughunt_mt_10b_inspect_json_duplicate_text_is_exact() {
    let st = even_strings();
    let mut dup = st.clone();
    dup.push(st[0].clone());
    let want = "chunk 1 was present twice; kept the copy needing 0 corrections";
    let o = run_in("inspect", &["--json"], &dup.join("\n"));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let doc: serde_json::Value = serde_json::from_str(&s(&o.stdout)).unwrap();
    let w: Vec<&str> = doc["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(
        w,
        vec![
            "no bitcoind reachable: nothing here is confirmed against the chain",
            want
        ],
        "inspect --json warnings"
    );
}

fn exact_warnings(doc: &str) -> Vec<String> {
    let j: serde_json::Value =
        serde_json::from_str(doc).unwrap_or_else(|e| panic!("no JSON document ({e}):\n{doc}"));
    j["warnings"]
        .as_array()
        .expect("no warnings array")
        .iter()
        .map(|w| w.as_str().unwrap().to_string())
        .collect()
}

/// Flip each listed char of `x` to a different bech32 character.
fn damage(x: &str, at: &[usize]) -> String {
    let mut c: Vec<char> = x.chars().collect();
    for &i in at {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    c.into_iter().collect()
}

/// R9: one correction entry per repaired chunk, exact text, the same on
/// `inspect --json` (stdout) and `decode --json` (stderr).
#[test]
fn bughunt_mt_10c_json_correction_text_is_exact() {
    let mut bad = even_strings();
    bad[0] = damage(&bad[0], &[10]);
    let want = "chunk 1 needed 1 of 4 corrections";
    for verb in ["inspect", "decode"] {
        let o = run_in(verb, &["--json"], &bad.join("\n"));
        assert!(o.status.success(), "{verb}: {}", s(&o.stderr));
        let doc = if verb == "inspect" {
            s(&o.stdout)
        } else {
            s(&o.stderr)
        };
        let w = exact_warnings(&doc);
        let hits: Vec<_> = w
            .iter()
            .filter(|x| x.starts_with("chunk ") && x.contains(" corrections"))
            .collect();
        assert_eq!(hits, vec![want], "{verb} --json warnings = {w:?}");
    }
}

/// R9: at the budget the entry carries `; NO MARGIN LEFT`, and entries follow
/// the prose margin report's order (most corrections first). Architect QC3:
/// inspect's whole array is `[no-bitcoind, corrections...]`; in decode every
/// correction entry follows every duplicate entry and precedes the stdout-mode
/// headline when there is one.
#[test]
fn bughunt_mt_10d_json_correction_entries_ordered_and_no_margin_left() {
    let mut bad = even_strings();
    bad[0] = damage(&bad[0], &[10]);
    bad[1] = damage(&bad[1], &[10, 20, 30, 40]);
    let want = vec![
        "chunk 2 needed 4 of 4 corrections; NO MARGIN LEFT".to_string(),
        "chunk 1 needed 1 of 4 corrections".to_string(),
    ];
    let is_needed = |x: &String| x.starts_with("chunk ") && x.contains(" needed ");
    let o = run_in("inspect", &["--json"], &bad.join("\n"));
    assert!(o.status.success(), "inspect: {}", s(&o.stderr));
    assert_eq!(
        exact_warnings(&s(&o.stdout)),
        vec![
            "no bitcoind reachable: nothing here is confirmed against the chain".to_string(),
            want[0].clone(),
            want[1].clone()
        ],
        "inspect --json warnings"
    );
    // decode, with a duplicate of the clean chunk 3 added so both kinds appear
    let mut dup = bad.clone();
    dup.push(even_strings()[2].clone());
    for (k, input) in [&bad, &dup].into_iter().enumerate() {
        let o = run_in("decode", &["--json"], &input.join("\n"));
        assert!(o.status.success(), "decode: {}", s(&o.stderr));
        let w = exact_warnings(&s(&o.stderr));
        let hits: Vec<String> = w.iter().filter(|x| is_needed(x)).cloned().collect();
        assert_eq!(hits, want, "decode --json correction entries: {w:?}");
        let first = w.iter().position(is_needed).unwrap();
        let last = w.iter().rposition(is_needed).unwrap();
        if let Some(d) = w.iter().rposition(|x| x.contains("was present twice")) {
            assert!(d < first, "a duplicate entry follows a correction: {w:?}");
        } else {
            assert_eq!(k, 0, "no duplicate entry: {w:?}");
        }
        if last + 1 < w.len() {
            assert_eq!(last + 2, w.len(), "more than the headline follows: {w:?}");
        }
    }
}

/// R10: `decode --json --quiet` is refused with the ruled text, exit 1.
#[test]
fn bughunt_mt_11b_decode_json_quiet_is_refused() {
    let out = run_in("decode", &["--json", "--quiet"], &even_strings().join("\n"));
    let e = s(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "rc {:?}\n{e}",
        out.status.code()
    );
    assert!(
        out.stdout.is_empty(),
        "stdout not empty: {}",
        s(&out.stdout)
    );
    assert_eq!(
        e.lines().next().unwrap_or(""),
        "mt decode: REFUSED — §10.10, --json and --quiet contradict each other"
    );
    let f = flat(&out.stderr);
    for want in [
        "`--json` asks for the inspection report as a document and `--quiet` asks for no report; mt refuses rather than silently ignoring one.",
        "Drop `--quiet` for the JSON report on stderr, or drop `--json` for the hex alone.",
    ] {
        assert!(f.contains(want), "missing {want:?} in:\n{e}");
    }
}

/// R11 (triage row): `--quiet` on `inspect` and `verify` is refused, exit 1.
#[test]
fn bughunt_mt_r11_quiet_on_inspect_and_verify_is_refused() {
    for verb in ["inspect", "verify"] {
        let out = run_in(verb, &["--quiet"], &even_strings().join("\n"));
        let e = s(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{verb}: rc {:?}\n{e}",
            out.status.code()
        );
        assert!(out.stdout.is_empty(), "{verb}: stdout not empty");
        assert_eq!(
            e.lines().next().unwrap_or(""),
            format!("mt {verb}: REFUSED — §10.10, --quiet has no meaning for `{verb}`")
        );
        let f = flat(&out.stderr);
        for want in [
            format!("{verb}'s output is its report; mt refuses rather than print it anyway."),
            "Drop `--quiet`.".to_string(),
        ] {
            assert!(f.contains(&want), "{verb}: missing {want:?} in:\n{e}");
        }
    }
}

/// R10/R11 order: on `inspect` and `verify`, `--json --quiet` gets the R11
/// refusal (it fires first), not R10's and not verify's `--json` refusal.
#[test]
fn bughunt_mt_r11_json_quiet_on_inspect_and_verify_gets_the_quiet_refusal() {
    for verb in ["inspect", "verify"] {
        let out = run_in(verb, &["--json", "--quiet"], &even_strings().join("\n"));
        let e = s(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{verb}: rc {:?}\n{e}",
            out.status.code()
        );
        assert!(out.stdout.is_empty(), "{verb}: stdout not empty");
        assert_eq!(
            e.lines().next().unwrap_or(""),
            format!("mt {verb}: REFUSED — §10.10, --quiet has no meaning for `{verb}`")
        );
    }
}

/// R12: the `mtl` repair plus a positional autocorrect decodes, and the
/// autocorrect is never silent (its existing notice prints).
#[test]
fn bughunt_mt_12b_mtl_repair_keeps_the_autocorrect_notice() {
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
    let e = s(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "rc {:?}\n{e}",
        out.status.code()
    );
    assert_eq!(s(&out.stdout), s(&clean.stdout));
    assert!(
        e.contains("you typed b, mt read it as 6"),
        "the autocorrect notice is missing:\n{e}"
    );
    // Architect QC4: the free `mtl` repair stays silent.
    assert!(
        !e.contains("you typed l") && !e.contains("you typed i"),
        "the mtl repair printed a notice:\n{e}"
    );
}
