//! RED rows for FIX_bh1119_mt_node_tristate (bughunt 2026-11-19 keys
//! bh-mt-C1, I13, I14, I15; architect ruling T11).
//!
//! A node's answer has THREE outcomes, not two: an answer (a value, or Core's
//! null), and NO usable answer (the call failed, or its reply could not be
//! read). The third must never read as DEAD. Every stub below speaks Core's
//! real mapping (bitcoin-cli.cpp ParseResult/ParseError): a null result prints
//! nothing and exits 0; an RPC error exits with abs(code), so a transaction
//! `getrawtransaction` cannot find exits 5 (RPC_INVALID_ADDRESS_OR_KEY).

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::fixture_txids;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

/// The P5 base fixture: two inputs (3 BTC and 5 BTC), regtest, from Core v25.
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

/// A scripted `bitcoin-cli -stdin`. `arms` are `case "$verb"` arms placed
/// BEFORE the defaults, so a row overrides a default by naming the verb.
/// Defaults: a synced txindex, a regtest chain, `getrawtransaction` not found
/// (exit 5, Core's real code), anything else exit 1.
struct Stub {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
}

fn stub(arms: &str) -> Stub {
    let script = format!(
        "#!/bin/sh\nverb=\"\"; arg1=\"\"\nwhile read -r line; do\n  if [ -z \"$verb\" ]; then verb=\"$line\"\n  elif [ -z \"$arg1\" ]; then arg1=\"$line\"\n  fi\ndone\ncase \"$verb\" in\n  getblockcount) echo 963832 ;;\n{arms}\n  getindexinfo) echo '{{\"txindex\": {{\"synced\": true, \"best_block_height\": 963832}}}}' ;;\n  getblockchaininfo) echo '{{\"chain\": \"regtest\", \"mediantime\": 1700000000}}' ;;\n  getrawtransaction) exit 5 ;;\n  *) exit 1 ;;\nesac\n"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bitcoin-cli");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    Stub { _dir: dir, path }
}

/// A `getrawtransaction` arm: each parent of the fixture answers `reply`;
/// any other txid (the fixture's own) is not found, exit 5.
fn parents_answer(reply: &str) -> String {
    let (_own, parents) = fixture_txids(&base());
    let mut arms = String::new();
    for p in &parents {
        arms.push_str(&format!("      {}*) echo '{reply}' ;;\n", &p[..16]));
    }
    format!("  getrawtransaction)\n    case \"$arg1\" in\n{arms}      *) exit 5 ;;\n    esac ;;")
}

/// As `parents_answer`, but the fixture's OWN txid answers `own_reply`.
fn own_and_parents_answer(own_reply: &str, parent_reply: &str) -> String {
    let (own, parents) = fixture_txids(&base());
    let mut arms = format!("      {}*) echo '{own_reply}' ;;\n", &own[..16]);
    for p in &parents {
        arms.push_str(&format!("      {}*) echo '{parent_reply}' ;;\n", &p[..16]));
    }
    format!("  getrawtransaction)\n    case \"$arg1\" in\n{arms}      *) exit 5 ;;\n    esac ;;")
}

/// Run `mt <verb> --bitcoin-cli <stub> [extra] --in <fixture raw_hex>`;
/// returns (exit code, stdout, stderr).
fn run(verb: &str, s: &Stub, extra: &[&str]) -> (Option<i32>, String, String) {
    let f = file_with(base()["raw_hex"].as_str().unwrap());
    let out = mt()
        .args([verb, "--bitcoin-cli"])
        .arg(&s.path)
        .args(extra)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const CONFIRMED: &str = r#"{"txid": "00", "confirmations": 6}"#;

/// PROPOSED (architect confirms): the STATUS row for "the node did not give a
/// usable answer", exactly as `inspect` prints it.
const STATUS_UNANSWERED: &str = "STATUS    UNKNOWN — the node did not answer every question: a call failed or\n                           its reply could not be read. mt will not guess DEAD from a missing\n                           answer. Check the node, then run mt again\n";

/// PROPOSED (architect confirms): `encode`'s warning headline for the same.
const ENCODE_UNANSWERED_HEADLINE: &str = "WARNING: the node did not answer every question — mt could not check every input before you cut.";

fn assert_unanswered_inspect(o: &str, why: &str) {
    assert!(!o.contains("STATUS    DEAD"), "{why}: reported DEAD:\n{o}");
    assert!(
        !o.to_lowercase().contains("scrap"),
        "{why}: said scrap:\n{o}"
    );
    assert!(
        o.ends_with(STATUS_UNANSWERED),
        "{why}: STATUS row is not the unanswered text:\n{o}"
    );
}

// ---- C1: a FAILED gettxout is not Core's null ------------------------------

/// C1 control: Core's null (no output, exit 0) with confirmed parents is DEAD.
#[test]
fn bh1119_mt_c1_null_gettxout_confirmed_parent_still_dead() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(
        o.contains("STATUS    DEAD — an input was spent by someone else"),
        "{o}"
    );
}

/// C1: `gettxout` FAILS (exit 28, Core warming up) and the parents are
/// confirmed. That is no answer, not a null: never DEAD.
#[test]
fn bh1119_mt_c1_failed_gettxout_inspect_is_not_dead() {
    let s = stub(&format!(
        "  gettxout) echo 'error code: -28' >&2; exit 28 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "a failed gettxout");
}

/// C1, `--json`: the status string is "Indeterminate" (T11), never "Dead".
#[test]
fn bh1119_mt_c1_failed_gettxout_inspect_json_is_indeterminate() {
    let s = stub(&format!(
        "  gettxout) exit 1 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &["--json"]);
    assert_eq!(rc, Some(0), "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).expect("inspect --json is JSON");
    assert_eq!(v["status"], "Indeterminate", "{o}");
}

/// C1, encode: a failed `gettxout` warns and PROCEEDS (T11: the no-node path),
/// it does not refuse with "can never be broadcast".
#[test]
fn bh1119_mt_c1_failed_gettxout_encode_warns_and_proceeds() {
    let s = stub(&format!(
        "  gettxout) exit 1 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("encode", &s, &[]);
    assert_eq!(rc, Some(0), "encode refused on a failed gettxout:\n{e}");
    assert!(!e.contains("REFUSED"), "{e}");
    assert!(!o.trim().is_empty(), "no strings produced");
    assert!(
        e.contains(ENCODE_UNANSWERED_HEADLINE),
        "warning missing:\n{e}"
    );
    assert!(
        !e.contains("no bitcoind reachable"),
        "a node WAS reachable:\n{e}"
    );
}

/// C1: a `gettxout` reply that exits 0 but carries no readable `value` is no
/// answer either (it was read as null, hence DEAD).
#[test]
fn bh1119_mt_c1_unreadable_gettxout_reply_is_not_dead() {
    let s = stub(&format!(
        "  gettxout) echo '{{\"bestblock\": \"00\", \"confirmations\": 3}}' ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "a gettxout reply with no value");
}

/// C1: the parent's reply is unreadable (`confirmations` is a string). That
/// is no answer about the parent, so not DEAD.
#[test]
fn bh1119_mt_c1_unreadable_parent_reply_is_not_dead() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        parents_answer(r#"{"txid": "00", "confirmations": "6"}"#)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "an unreadable parent reply");
}

/// C1: the question ASKED FIRST (did this transaction itself confirm?) got an
/// unreadable reply. The DEAD evidence is then exactly the success case's
/// shape, so the status is unanswered, not DEAD.
#[test]
fn bh1119_mt_c1_unreadable_own_tx_reply_is_not_dead() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        own_and_parents_answer("not json", CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "an unreadable own-transaction reply");
}

/// C1, encode: same as above; encode warns and proceeds rather than telling
/// the operator a possibly-confirmed payment "can never be broadcast".
#[test]
fn bh1119_mt_c1_unreadable_own_tx_reply_encode_warns_and_proceeds() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        own_and_parents_answer("not json", CONFIRMED)
    ));
    let (rc, o, e) = run("encode", &s, &[]);
    assert_eq!(rc, Some(0), "encode refused:\n{e}");
    assert!(!e.contains("can never be broadcast"), "{e}");
    assert!(!o.trim().is_empty(), "no strings produced");
    assert!(
        e.contains(ENCODE_UNANSWERED_HEADLINE),
        "warning missing:\n{e}"
    );
}

/// C1: a parent lookup that fails with anything but Core's not-found code
/// (exit 5) is no answer. With txindex it read PENDING ("a parent has not
/// confirmed"), a claim nothing supports.
#[test]
fn bh1119_mt_c1_failed_parent_lookup_is_unanswered_not_pending() {
    let s = stub("  gettxout) exit 0 ;;\n  getrawtransaction) exit 1 ;;");
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(
        !o.contains("STATUS    PENDING"),
        "a failed lookup read as PENDING:\n{o}"
    );
    assert_unanswered_inspect(&o, "a failed parent lookup");
}

/// C1 control: Core's own not-found (exit 5) with a synced txindex stays
/// PENDING.
#[test]
fn bh1119_mt_c1_not_found_parent_with_txindex_still_pending() {
    let s = stub("  gettxout) exit 0 ;;");
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(o.contains("STATUS    PENDING"), "{o}");
}

// ---- I13: confirmations parsed as JSON ---------------------------------------

/// I13: `"confirmations":  0` (two spaces) is zero confirmations: PENDING.
#[test]
fn bh1119_mt_i13_two_space_zero_confirmations_is_pending() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        parents_answer(r#"{"txid": "00", "confirmations":  0}"#)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(o.contains("STATUS    PENDING"), "got:\n{o}");
}

/// I13, encode: the same reply must not refuse §8.5.
#[test]
fn bh1119_mt_i13_encode_two_space_zero_is_not_refused() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        parents_answer(r#"{"txid": "00", "confirmations":  0}"#)
    ));
    let (rc, _o, e) = run("encode", &s, &[]);
    assert_eq!(rc, Some(0), "a mempool parent was refused:\n{e}");
    assert!(!e.contains("REFUSED"), "{e}");
}

/// I13 guard: Core omits `confirmations` for a mempool transaction: PENDING.
#[test]
fn bh1119_mt_i13_absent_confirmations_is_pending() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        parents_answer(r#"{"txid": "00"}"#)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(o.contains("STATUS    PENDING"), "got:\n{o}");
}

/// I13 guard: compact `"confirmations":6` is confirmed: DEAD with a null.
#[test]
fn bh1119_mt_i13_compact_confirmed_is_dead() {
    let s = stub(&format!(
        "  gettxout) exit 0 ;;\n{}",
        parents_answer(r#"{"txid":"00","confirmations":6}"#)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(o.contains("STATUS    DEAD"), "got:\n{o}");
}

// ---- I14: the node's chain, read as JSON and named ---------------------------

/// I14: a testnet4 node gives testnet4 addresses and names the network.
#[test]
fn bh1119_mt_i14_testnet4_node_prints_testnet4_addresses() {
    let s = stub(
        "  getblockchaininfo) echo '{\"chain\":\"testnet4\",\"mediantime\":1700000000}' ;;\n  gettxout) exit 0 ;;",
    );
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(
        o.contains("OUT       1 output(s)   (testnet4)\n"),
        "got:\n{o}"
    );
    assert!(o.contains("tb1q"), "no testnet address:\n{o}");
    assert!(
        !o.contains("bc1q"),
        "a mainnet address on a testnet4 node:\n{o}"
    );
}

/// I14: a chain mt does not know is not "no node to ask".
#[test]
fn bh1119_mt_i14_unknown_chain_is_named_as_such() {
    let s = stub(
        "  getblockchaininfo) echo '{\"chain\":\"futurenet\",\"mediantime\":1700000000}' ;;\n  gettxout) exit 0 ;;",
    );
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(
        o.contains("OUT       1 output(s)   (addresses shown as MAINNET — the node named a chain mt does not know)\n"),
        "got:\n{o}"
    );
}

/// I14: a node that answers but will not say its chain is not "no node".
#[test]
fn bh1119_mt_i14_chainless_reply_is_named_as_such() {
    let s = stub("  getblockchaininfo) exit 1 ;;\n  gettxout) exit 0 ;;");
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(
        o.contains("OUT       1 output(s)   (addresses shown as MAINNET — the node did not say which chain it is on)\n"),
        "got:\n{o}"
    );
}

// ---- I15: the value parsed as a JSON number, exactly ------------------------

fn input_sats(o: &str) -> Vec<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(o).expect("inspect --json is JSON");
    v["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["sats"].clone())
        .collect()
}

/// I15: `5e-8` BTC is 5 satoshis.
#[test]
fn bh1119_mt_i15_scientific_value_is_five_sats() {
    let s = stub("  gettxout) echo '{\"value\":5e-8,\"scriptPubKey\":{}}' ;;");
    let (rc, o, e) = run("inspect", &s, &["--json"]);
    assert_eq!(rc, Some(0), "{e}");
    assert_eq!(
        input_sats(&o),
        vec![serde_json::json!(5), serde_json::json!(5)],
        "{o}"
    );
}

/// I15: more than 8 decimal places is not an amount: no answer, never a
/// rounded value labelled as fetched from the chain.
#[test]
fn bh1119_mt_i15_overprecise_value_is_unanswered() {
    let s = stub("  gettxout) echo '{\"value\":0.000000051,\"scriptPubKey\":{}}' ;;");
    let (rc, o, e) = run("inspect", &s, &["--json"]);
    assert_eq!(rc, Some(0), "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).unwrap();
    assert_eq!(v["status"], "Indeterminate", "{o}");
    assert_eq!(
        input_sats(&o),
        vec![serde_json::Value::Null, serde_json::Value::Null],
        "{o}"
    );
}

/// I15: a negative value, `-0` included, is no answer (it read as a null,
/// then PENDING).
#[test]
fn bh1119_mt_i15_negative_value_is_unanswered() {
    for v in ["-1", "-0"] {
        let s = stub(&format!(
            "  gettxout) echo '{{\"value\":{v},\"scriptPubKey\":{{}}}}' ;;"
        ));
        let (rc, o, e) = run("inspect", &s, &[]);
        assert_eq!(rc, Some(0), "{e}");
        assert_unanswered_inspect(&o, &format!("value {v}"));
    }
}

/// I15: above 21,000,000 BTC is no answer.
#[test]
fn bh1119_mt_i15_value_above_money_range_is_unanswered() {
    let s = stub("  gettxout) echo '{\"value\":21000000.00000001,\"scriptPubKey\":{}}' ;;");
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "a value above MoneyRange");
}

/// I15 guard: a JSON `null` printed by a wrapper is Core's null.
#[test]
fn bh1119_mt_i15_json_null_reply_is_null() {
    let s = stub(&format!(
        "  gettxout) echo null ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(o.contains("STATUS    DEAD"), "{o}");
}

/// I15 guard: Core's own decimal form is unchanged.
#[test]
fn bh1119_mt_i15_core_decimal_value_is_exact() {
    let s = stub("  gettxout) echo '{\"value\": 3.00000000, \"scriptPubKey\": {}}' ;;");
    let (rc, o, e) = run("inspect", &s, &["--json"]);
    assert_eq!(rc, Some(0), "{e}");
    assert_eq!(
        input_sats(&o),
        vec![
            serde_json::json!(300_000_000u64),
            serde_json::json!(300_000_000u64)
        ],
        "{o}"
    );
}

/// C1 guard (precedence, §6a's existing order): input 0 is Core's null with a
/// confirmed parent, input 1's `gettxout` fails. Input 0 alone proves DEAD.
#[test]
fn bh1119_mt_c1_dead_input_outranks_unanswered_input() {
    let (_own, parents) = fixture_txids(&base());
    let s = stub(&format!(
        "  gettxout)\n    case \"$arg1\" in\n      {}*) exit 0 ;;\n      *) exit 1 ;;\n    esac ;;\n{}",
        &parents[0][..16],
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert!(
        o.contains("STATUS    DEAD — an input was spent by someone else"),
        "{o}"
    );
}

// ---- I-1 (review r1): a txindex that is not reported synced ----------------
//
// Core answers `getrawtransaction` from a txindex that is still building
// (node/transaction.cpp GetTransaction calls FindTx whether or not the index
// is synced), so an old parent reads confirmed while THIS transaction, mined
// above the index frontier, exits 5. Own "not found" proves nothing then, and
// "input null + parent confirmed" is also exactly the already-mined shape.

const TXINDEX_BUILDING: &str =
    "  getindexinfo) echo '{\"txindex\": {\"synced\": false, \"best_block_height\": 10}}' ;;";

/// RULED (architect, I-1): the STATUS row when an input is spent, its parent
/// confirmed, and the node has not reported a synced txindex.
const STATUS_INDEX_NOT_SYNCED: &str = "STATUS    UNKNOWN — an input is not in the UTXO set, its parent confirmed, and this\n                           node has not reported its -txindex synced: mt cannot tell whether this\n                           transaction is already mined or someone else spent the input. mt will\n                           not guess DEAD. Run mt again once getindexinfo shows the txindex synced\n";

/// RULED (architect, I-1): `encode`'s warning headline for the same.
const ENCODE_INDEX_NOT_SYNCED_HEADLINE: &str = "WARNING: the node has not reported its -txindex synced — mt cannot tell whether this transaction is already mined.";

fn assert_index_not_synced_inspect(o: &str, why: &str) {
    assert!(!o.contains("STATUS    DEAD"), "{why}: reported DEAD:\n{o}");
    assert!(
        !o.to_lowercase().contains("scrap"),
        "{why}: said scrap:\n{o}"
    );
    assert!(
        o.ends_with(STATUS_INDEX_NOT_SYNCED),
        "{why}: STATUS row is not the index-not-synced text:\n{o}"
    );
}

fn assert_index_not_synced_encode(rc: Option<i32>, o: &str, e: &str, why: &str) {
    assert_eq!(rc, Some(0), "{why}: encode refused:\n{e}");
    assert!(!e.contains("can never be broadcast"), "{why}: {e}");
    assert!(!e.contains("REFUSED"), "{why}: {e}");
    assert!(!o.trim().is_empty(), "{why}: no strings produced");
    assert!(
        e.contains(ENCODE_INDEX_NOT_SYNCED_HEADLINE),
        "{why}: warning missing:\n{e}"
    );
    assert!(
        e.contains("Do not build a replacement transaction"),
        "{why}: pay-twice caution missing:\n{e}"
    );
}

/// I-1: txindex building, own txid not found (exit 5), inputs null, parents
/// confirmed -> not DEAD; the index-not-synced UNKNOWN text.
#[test]
fn bh1119_mt_c1_unsynced_txindex_own_not_found_is_not_dead() {
    let s = stub(&format!(
        "{TXINDEX_BUILDING}\n  gettxout) exit 0 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_index_not_synced_inspect(&o, "txindex building");
}

/// I-1, `--json`: "Indeterminate", never "Dead".
#[test]
fn bh1119_mt_c1_unsynced_txindex_own_not_found_json_is_indeterminate() {
    let s = stub(&format!(
        "{TXINDEX_BUILDING}\n  gettxout) exit 0 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &["--json"]);
    assert_eq!(rc, Some(0), "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).expect("inspect --json is JSON");
    assert_eq!(v["status"], "Indeterminate", "{o}");
}

/// I-1, encode: no §8.5 refusal (its remedy pays twice on a mined plate);
/// warns and proceeds.
#[test]
fn bh1119_mt_c1_unsynced_txindex_own_not_found_encode_is_not_never_broadcast() {
    let s = stub(&format!(
        "{TXINDEX_BUILDING}\n  gettxout) exit 0 ;;\n{}",
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("encode", &s, &[]);
    assert_index_not_synced_encode(rc, &o, &e, "txindex building");
}

/// I-1, the other getindexinfo states that are not "synced": a failed call
/// (Core before 0.21: method not found, -32601 -> exit 89) and a reply with no
/// txindex member. Neither is a synced index, so neither licenses DEAD.
#[test]
fn bh1119_mt_c1_unreported_txindex_own_not_found_is_not_dead() {
    for (arm, why) in [
        ("  getindexinfo) exit 89 ;;", "getindexinfo failed"),
        ("  getindexinfo) echo '{}' ;;", "no txindex member"),
    ] {
        let s = stub(&format!(
            "{arm}\n  gettxout) exit 0 ;;\n{}",
            parents_answer(CONFIRMED)
        ));
        let (rc, o, e) = run("inspect", &s, &[]);
        assert_eq!(rc, Some(0), "{why}: {e}");
        assert_index_not_synced_inspect(&o, why);
        let (rc, o, e) = run("encode", &s, &[]);
        assert_index_not_synced_encode(rc, &o, &e, why);
    }
}

/// The index state is read BEFORE this transaction's own question: an index
/// that finishes between the two would otherwise license DEAD on a "not
/// found" given while it was still building. The stub reports synced only
/// after the own txid was asked.
fn index_syncs_after_own_question() -> Stub {
    let (_own, parents) = fixture_txids(&base());
    let mut arms = String::new();
    for p in &parents {
        arms.push_str(&format!("      {}*) echo '{CONFIRMED}' ;;\n", &p[..16]));
    }
    stub(&format!(
        "  getindexinfo)\n    if [ -e \"$(dirname \"$0\")/own-asked\" ]; then echo '{{\"txindex\": {{\"synced\": true, \"best_block_height\": 963832}}}}'; else echo '{{\"txindex\": {{\"synced\": false, \"best_block_height\": 10}}}}'; fi ;;\n  gettxout) exit 0 ;;\n  getrawtransaction)\n    case \"$arg1\" in\n{arms}      *) : > \"$(dirname \"$0\")/own-asked\"; exit 5 ;;\n    esac ;;"
    ))
}

#[test]
fn bh1119_mt_c1_txindex_state_read_before_own_question_inspect() {
    let s = index_syncs_after_own_question();
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_index_not_synced_inspect(&o, "index synced after the own question");
}

#[test]
fn bh1119_mt_c1_txindex_state_read_before_own_question_encode() {
    let s = index_syncs_after_own_question();
    let (rc, o, e) = run("encode", &s, &[]);
    assert_index_not_synced_encode(rc, &o, &e, "index synced after the own question");
}

/// N6 order 5 before 6 (review r1 M-1): an unanswered input outranks the
/// txindex UNKNOWN (parent not found, no synced index).
#[test]
fn bh1119_mt_c1_unanswered_outranks_txindex_unknown() {
    let (_own, parents) = fixture_txids(&base());
    let s = stub(&format!(
        "{TXINDEX_BUILDING}\n  gettxout)\n    case \"$arg1\" in\n      {}*) exit 1 ;;\n      *) exit 0 ;;\n    esac ;;",
        &parents[0][..16]
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "unanswered beside a txindex UNKNOWN");
}

/// N6 order 5 before 6a: an unanswered input outranks index-not-synced (the
/// failed call is the one a re-run in seconds can fix).
#[test]
fn bh1119_mt_c1_unanswered_outranks_index_not_synced() {
    let (_own, parents) = fixture_txids(&base());
    let s = stub(&format!(
        "{TXINDEX_BUILDING}\n  gettxout)\n    case \"$arg1\" in\n      {}*) exit 1 ;;\n      *) exit 0 ;;\n    esac ;;\n{}",
        &parents[0][..16],
        parents_answer(CONFIRMED)
    ));
    let (rc, o, e) = run("inspect", &s, &[]);
    assert_eq!(rc, Some(0), "{e}");
    assert_unanswered_inspect(&o, "unanswered beside index-not-synced");
}

// ---- mt-median-time-substring (folded by architect I-1 ruling) --------------

/// The fixture with a TIMESTAMP lock (2027) that its inputs enforce, so the
/// LOCKTIME row asks the node for MTP.
fn timelocked_hex() -> String {
    let hex = base()["raw_hex"].as_str().unwrap().to_string();
    let mut tx: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&common::hex_to_bytes(&hex)).unwrap();
    tx.lock_time = bitcoin::absolute::LockTime::from_consensus(1_800_000_000);
    for i in &mut tx.input {
        i.sequence = bitcoin::Sequence(0xFFFF_FFFE);
    }
    bitcoin::consensus::encode::serialize_hex(&tx)
}

fn locktime_row_with(blockchaininfo: &str) -> String {
    let s = stub(&format!(
        "  getblockchaininfo) echo '{blockchaininfo}' ;;\n  gettxout) exit 0 ;;"
    ));
    let f = file_with(&timelocked_hex());
    let out = mt()
        .args(["inspect", "--bitcoin-cli"])
        .arg(&s.path)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let o = String::from_utf8_lossy(&out.stdout).into_owned();
    o.lines()
        .find(|l| l.starts_with("LOCKTIME"))
        .unwrap_or_else(|| panic!("no LOCKTIME row:\n{o}"))
        .to_string()
}

/// `getblockchaininfo` is parsed as JSON: a `mediantime` that is not a JSON
/// integer is no answer, never the digits before its first non-digit
/// (`1.7e9` read as 1, printed as 1970).
#[test]
fn bh1119_mt_median_time_exponent_reply_is_not_1970() {
    let row = locktime_row_with(r#"{"chain": "regtest", "mediantime": 1.7e9}"#);
    assert!(row.contains("LOCKED UNTIL 2027-"), "{row}");
    assert!(
        !row.contains("1970"),
        "an unreadable MTP was printed: {row}"
    );
}

/// Guard: Core's integer `mediantime` is read and printed as a date.
#[test]
fn bh1119_mt_median_time_integer_reply_is_read() {
    let row = locktime_row_with(r#"{"chain": "regtest", "mediantime": 1700000000}"#);
    assert!(row.contains("current MTP 2023-11-14T22:13Z"), "{row}");
}
