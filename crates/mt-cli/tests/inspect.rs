//! `mt inspect`, and the report's three callers.
//!
//! Run **both offline and with a node**, because offline-only passes vacuously:
//! with no node every chain-derived row reads `UNKNOWN` for all three callers,
//! so they agree trivially and the gate proves nothing about the rows that
//! matter.

use assert_cmd::Command;
use std::io::Write;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

/// The offline mechanism, named so no test reaches for `PATH` — which is
/// process-global and would silently change neighbouring tests in the same run.
const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn corpus() -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn strings_file(label: &str) -> tempfile::NamedTempFile {
    let s: Vec<String> = corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(s.join("\n").as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn inspect_offline(label: &str) -> (String, String) {
    let f = strings_file(label);
    let out = mt()
        .args(["inspect", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "inspect failed offline");
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// **Rule 1: a row is never omitted for being unanswerable — it reads
/// `UNKNOWN`.** Omission and ignorance look identical on a terminal, and a
/// reader cannot tell a row that was skipped from one that never existed.
#[test]
fn every_row_is_present_offline() {
    let (out, _) = inspect_offline("even");
    for row in [
        "mt1 SET",
        "TX ",
        "OUT ",
        "FEE ",
        "LOCKTIME ",
        "INPUTS ",
        "STATUS ",
    ] {
        assert!(out.contains(row), "row {row:?} was omitted offline");
    }
    assert!(
        out.contains("FEE       UNKNOWN"),
        "fee must say UNKNOWN, not vanish"
    );
    // §8.4's spelling, not a sixth of mt's own invention. The row is bound to
    // the section's five normative forms and may not add to them.
    assert!(
        out.contains("current height unknown (no node)"),
        "height must use §8.4's offline spelling: {out}"
    );
}

/// The report NEVER renders a verdict about spendability. §8.4 spends its length
/// establishing that `mt` cannot make that claim — a BIP-68 relative timelock
/// lives in `OP_CSV` inside the witness script, and reading it means evaluating
/// the sending wallet's script.
#[test]
fn the_report_never_claims_spendability() {
    let (out, err) = inspect_offline("even");
    for forbidden in ["PASSED", "SPENDABLE", "VALID", "SAFE TO BROADCAST"] {
        assert!(
            !out.contains(forbidden),
            "report rendered a verdict: {forbidden}"
        );
        assert!(
            !err.contains(forbidden),
            "stderr rendered a verdict: {forbidden}"
        );
    }
}

/// §6a's recovery-time warning: the enumeration, the read-vs-verified split, and
/// **both** ways out. The encode-time wording ("before cutting") is useless to a
/// recoverer — the engraving already exists.
#[test]
fn offline_warning_separates_read_from_verified_and_names_both_ways_out() {
    let (_, err) = inspect_offline("even");
    assert!(err.contains("no bitcoind reachable"));
    assert!(
        err.contains("what the transaction SAYS") && err.contains("None of it is confirmed"),
        "the read-vs-verified split is the load-bearing part: {err}"
    );
    assert!(
        err.contains("run mt inspect again with a bitcoind"),
        "node route missing"
    );
    assert!(err.contains("block explorer"), "explorer route missing");
    assert!(
        !err.contains("before cutting") && !err.contains("21 minutes"),
        "this is the ENCODE-time wording, useless to a recoverer: {err}"
    );
}

/// The txid must be printed so the explorer route is actionable — and it must be
/// the **txid**, not the hash of the engraved bytes, which is the wtxid.
#[test]
fn the_printed_txid_is_the_txid_not_the_wtxid() {
    for label in ["even", "uneven"] {
        let v = corpus()["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["label"] == label)
            .unwrap()
            .clone();
        let (out, err) = inspect_offline(label);
        let txid = v["txid"].as_str().unwrap();
        let wtxid = v["wtxid"].as_str().unwrap();

        assert!(
            out.contains(txid),
            "{label}: the report does not print the txid"
        );
        assert!(
            !out.contains(wtxid),
            "{label}: the report printed the WTXID — an explorer lookup would return nothing"
        );
        assert!(
            err.contains(txid),
            "{label}: the warning must print the txid to look up"
        );
    }
}

/// **Rule 3: `encode` appends, never edits.** The two views must agree on every
/// row they can both produce — that is what the single-owner rule protects.
#[test]
fn encode_and_inspect_agree_on_the_rows_they_share() {
    let v = &corpus()["vectors"][0];
    let mut txf = tempfile::NamedTempFile::new().unwrap();
    txf.write_all(v["raw_hex"].as_str().unwrap().as_bytes())
        .unwrap();
    txf.flush().unwrap();

    let enc = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(txf.path())
        .output()
        .unwrap();
    let enc_err = String::from_utf8(enc.stderr).unwrap();
    let (ins_out, _) = inspect_offline("even");

    // `encode`'s suffix rows are its own; the TX identity must match.
    assert!(
        enc_err.contains("CUT       "),
        "encode's CUT row is missing"
    );
    assert!(
        enc_err.contains("PREFIX    "),
        "encode's PREFIX row is missing"
    );
    assert!(
        ins_out.contains(v["txid"].as_str().unwrap()),
        "inspect and encode disagree about the transaction"
    );
}

/// Both vectors, so the uneven one — whose last chunk is short — is exercised.
#[test]
fn both_vectors_inspect_cleanly() {
    for label in ["even", "uneven"] {
        let (out, _) = inspect_offline(label);
        assert!(
            out.contains("STATUS    UNKNOWN"),
            "{label}: offline status wrong"
        );
        assert!(out.contains("strings, 1.."), "{label}: set row missing");
    }
}

/// A node that cannot be reached is a WARNING, never a refusal — offline
/// operation is the constellation's posture, and an absent node is an absent
/// answer rather than a bad one.
#[test]
fn an_absent_node_is_never_a_refusal() {
    let f = strings_file("even");
    let out = mt()
        .args(["inspect", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "an absent node must not fail the run");
    assert!(
        !String::from_utf8(out.stderr).unwrap().contains("REFUSED"),
        "an absent node was reported as a refusal"
    );
}

/// **The third provenance class.** An operator-supplied value is neither
/// chain-fetched nor absent: it is claimed, and checked by nobody. Collapsing
/// the report to two classes put an unverified number in the column a reader
/// takes as verified — and it is the number that decides whether to cut at all.
#[test]
fn an_asserted_value_makes_the_fee_say_claimed() {
    let v = &corpus()["vectors"][0];
    let mut txf = tempfile::NamedTempFile::new().unwrap();
    txf.write_all(v["raw_hex"].as_str().unwrap().as_bytes())
        .unwrap();
    txf.flush().unwrap();

    let out = mt()
        .args([
            "encode",
            "--bitcoin-cli",
            OFFLINE,
            "--input-value",
            "0:50.0",
            "--in",
        ])
        .arg(txf.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(
        err.contains("CLAIMED — no input value verified"),
        "an asserted value must not be presented as verified: {err}"
    );
    assert!(
        err.contains("OPERATOR-ASSERTED"),
        "the input row must name its provenance: {err}"
    );
}

/// A total is refused: it has two readings that differ by a whole input.
#[test]
fn input_value_must_be_per_input() {
    let v = &corpus()["vectors"][0];
    let mut txf = tempfile::NamedTempFile::new().unwrap();
    txf.write_all(v["raw_hex"].as_str().unwrap().as_bytes())
        .unwrap();
    txf.flush().unwrap();

    let out = mt()
        .args([
            "encode",
            "--bitcoin-cli",
            OFFLINE,
            "--input-value",
            "50.0",
            "--in",
        ])
        .arg(txf.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "a bare total was accepted");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("REFUSED — §8.2c"), "got: {err}");
    assert!(err.contains("PER INPUT"), "the refusal must say why: {err}");
}

/// **`encode` APPENDS, never edits.** Its two rows come after `STATUS`, so the
/// operator's view is the recoverer's view plus a suffix.
#[test]
fn encode_appends_its_rows_below_status() {
    let v = &corpus()["vectors"][0];
    let mut txf = tempfile::NamedTempFile::new().unwrap();
    txf.write_all(v["raw_hex"].as_str().unwrap().as_bytes())
        .unwrap();
    txf.flush().unwrap();

    let out = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(txf.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    let status = err.find("STATUS ").expect("no STATUS row");
    let cut = err.find("CUT ").expect("no CUT row");
    assert!(
        cut > status,
        "encode's rows must come AFTER the shared report"
    );
}

// ── with a node ─────────────────────────────────────────────────────────────
//
// **This half did not exist, and its absence was invisible.** An independent
// false-PASS review mutated `inspect()` so that `node` was always `None` — as
// if no node were ever reachable — and all 117 tests plus every gate stayed
// green. The module doc above has claimed since P4 that these tests "run both
// offline and with a node"; nothing did. `inspect` is the verb a 2040 recoverer
// reaches for, and the node is what turns four `UNKNOWN` rows into answers, so
// this was the untested half of the tool's whole purpose.

mod common;
use common::{fixture_txids, node_stub};

fn inspect_with_node(label: &str, node: &std::path::Path) -> (String, String) {
    let f = strings_file(label);
    let out = mt()
        .args(["inspect", "--bitcoin-cli"])
        .arg(node)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inspect failed with a node: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn corpus_vector(label: &str) -> serde_json::Value {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()
        .clone()
}

/// **The rows a node exists to answer must actually change.** Offline every
/// chain-derived row reads `UNKNOWN`, so an offline-only suite agrees with
/// itself no matter what the node code does — including doing nothing.
#[test]
fn a_reachable_node_answers_the_rows_that_were_unknown() {
    let stub = node_stub(r#"{"value": 50.00000000, "scriptPubKey": {}}"#, &[]);
    let (out, _) = inspect_with_node("even", stub.path());

    assert!(
        out.contains("STATUS    LIVE"),
        "the node said every input is unspent and STATUS did not say so:\n{out}"
    );
    assert!(
        out.contains("current height 963832"),
        "the chain height was not read:\n{out}"
    );
    assert!(
        out.contains("50.00000000 BTC   from node"),
        "the input value was not fetched, or not labelled as chain-fetched:\n{out}"
    );
    assert!(
        !out.contains("no node reachable"),
        "a reachable node was reported as absent:\n{out}"
    );
}

/// The offline/online contrast, asserted as a DIFFERENCE. This is the assertion
/// the mutation actually failed: with `node` forced to `None`, the two runs are
/// identical and this test goes red where every existing one stayed green.
#[test]
fn reaching_a_node_changes_the_report() {
    let stub = node_stub(r#"{"value": 50.00000000, "scriptPubKey": {}}"#, &[]);
    let (online, _) = inspect_with_node("even", stub.path());
    let (offline, _) = inspect_offline("even");
    assert_ne!(
        online, offline,
        "inspect produced the SAME report with and without a node — \
         the node was never consulted"
    );
    // ...and specifically on the rows §1.1 says only a node can fill.
    assert!(offline.contains("STATUS    UNKNOWN") && online.contains("STATUS    LIVE"));
    assert!(
        offline.contains("current height unknown (no node)")
            && online.contains("current height 963832"),
        "offline:\n{offline}\nonline:\n{online}"
    );
}

/// §8.5's condition read through `inspect`, which never REFUSES — it reports.
/// `inspect` is a recovery-time verb: the engraving already exists, so refusing
/// to describe it helps nobody.
#[test]
fn inspect_reports_a_dead_plate_rather_than_refusing() {
    let v = corpus_vector("even");
    let (own, parents) = fixture_txids(&v);
    // gettxout null, parents confirmed, this transaction NOT on chain.
    let conf: Vec<(&str, u32)> = parents.iter().map(|p| (&p[..16], 6u32)).collect();
    assert!(!conf.iter().any(|(t, _)| own.starts_with(t)));
    let stub = node_stub("", &conf);

    let (out, err) = inspect_with_node("even", stub.path());
    assert!(out.contains("STATUS    DEAD"), "got:\n{out}");
    assert!(
        !err.contains("REFUSED"),
        "inspect refused to describe an engraving that already exists:\n{err}"
    );
}

/// The five liveness states are five because this one is not DEAD. A parent in
/// the mempool means the plate MAY still become live, and `include_mempool` is
/// false by ruling, so `gettxout -> null` is the expected answer here.
#[test]
fn inspect_distinguishes_pending_from_dead() {
    let v = corpus_vector("even");
    let (_own, parents) = fixture_txids(&v);
    let conf: Vec<(&str, u32)> = parents.iter().map(|p| (&p[..16], 0u32)).collect();
    let stub = node_stub("", &conf);

    let (out, _) = inspect_with_node("even", stub.path());
    assert!(out.contains("STATUS    PENDING"), "got:\n{out}");
    assert!(
        !out.contains("DEAD"),
        "an unconfirmed parent was called DEAD — the error that gets a live \
         engraving thrown away:\n{out}"
    );
}

/// **ASKED FIRST.** Every input of a confirmed transaction is spent by itself
/// and every parent is confirmed, which is exactly the DEAD condition — so
/// without this question the success case reports as the theft case.
#[test]
fn inspect_reports_an_already_confirmed_transaction_as_confirmed() {
    let v = corpus_vector("even");
    let (own, parents) = fixture_txids(&v);
    let mut conf: Vec<(&str, u32)> = vec![(&own[..16], 4)];
    conf.extend(parents.iter().map(|p| (&p[..16], 12u32)));
    let stub = node_stub("", &conf);

    let (out, _) = inspect_with_node("even", stub.path());
    assert!(out.contains("ALREADY CONFIRMED"), "got:\n{out}");
    assert!(
        !out.contains("STATUS    DEAD"),
        "a transaction that CONFIRMED was reported as one whose inputs were \
         stolen:\n{out}"
    );
}

/// With a node reachable, §6a's no-node warning must be ABSENT — it names four
/// questions the node has just answered.
#[test]
fn the_no_node_warning_is_absent_when_a_node_is_there() {
    let stub = node_stub(r#"{"value": 50.00000000, "scriptPubKey": {}}"#, &[]);
    let (_, err) = inspect_with_node("even", stub.path());
    assert!(
        !err.contains("no bitcoind reachable"),
        "mt warned that no node was reachable while consulting one:\n{err}"
    );
    let (_, offline_err) = inspect_offline("even");
    assert!(
        offline_err.contains("no bitcoind reachable"),
        "control failed"
    );
}

/// **`--json` parsed and did nothing**, so a caller asking for machine output
/// got prose and no error — worse than the flag not existing, because a script
/// will parse *something* out of prose.
#[test]
fn json_output_is_actually_json() {
    let f = strings_file("even");
    let out = mt()
        .args(["inspect", "--json", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();

    // The WHOLE stream, not a slice: a caller pipes stdout, it does not go
    // looking for the first brace.
    let v: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("not JSON: {e}\n{text}"));
    assert!(v["warnings"].is_array(), "warnings are not carried as data");
    // The rows a consumer needs, and the ones that must not be collapsed.
    assert!(v["txid"].is_string());
    assert!(v["outputs"].as_array().unwrap().len() == 2);
    // The PREFIX, not the hex id: what the plates actually share, and what
    // `encode` prints at cutting time. A hex id is a true fact about the set
    // that appears nowhere the operator can look.
    assert!(
        v["set_prefix"].is_string(),
        "the set prefix is what groups plates"
    );
    // §6a rules FIVE liveness states; a boolean would erase the difference
    // between "pending" and "dead", which is the one that matters.
    assert!(v["status"].is_string(), "status must not be a boolean");
    // Provenance survives per input, because the three columns are the point.
    let i0 = &v["inputs"][0];
    assert!(i0["provenance"].is_string());
    assert!(i0["verified"].is_boolean());
    // ...and no prose leaked into the machine stream.
    assert!(
        !text.contains("WARNING"),
        "prose on the JSON stream:\n{text}"
    );
    assert!(
        !text.contains("UNKNOWN —"),
        "prose on the JSON stream:\n{text}"
    );
}

/// The SET row names the set in **the form that is on the steel** — the 8
/// characters after `mt1` that every plate shares. It omitted the identity
/// entirely at first, then carried it as a HEX id, which is a true fact about
/// the set and appears nowhere an operator can see: they are holding plates.
#[test]
fn the_set_row_names_the_set_the_way_the_steel_does() {
    let (out, _) = inspect_offline("even");
    assert!(
        out.contains("all begin mt1p9h8jqq9"),
        "the SET row does not name the shared prefix:\n{out}"
    );
    assert!(
        !out.contains("0x2dcf2"),
        "the row shows a hex id the operator cannot match against steel:\n{out}"
    );
}

/// **F-235: addresses are rendered for the operator's NETWORK.** A
/// `scriptPubKey` carries none, so `mt` rendered every output with mainnet
/// parameters — a regtest transaction showed `bc1q…` for an output the node
/// calls `bcrt1q…`, the same witness program under a different HRP, so the
/// printed string is not an address anywhere.
///
/// Read from the node, not asked of the operator: §6a's posture is that
/// `bitcoin-cli` already knows.
#[test]
fn addresses_are_rendered_for_the_network_the_node_reports() {
    let stub = node_stub(r#"{"value": 50.00000000, "scriptPubKey": {}}"#, &[]);
    let (out, _) = inspect_with_node("even", stub.path());
    // The stub answers `getblockchaininfo` with a mainnet-shaped reply and no
    // `chain` field, so mt must fall back and SAY it fell back.
    assert!(
        out.contains("bc1q") || out.contains("addresses shown as MAINNET"),
        "{out}"
    );
}

/// Offline, mt cannot know — and says so rather than printing an address that
/// is not one on the operator's chain.
#[test]
fn an_unknown_network_is_stated_not_assumed_silently() {
    let (out, _) = inspect_offline("even");
    assert!(
        out.contains("addresses shown as MAINNET — no node to ask"),
        "mt assumed a network without saying so:\n{out}"
    );
}

// ─── GRAFT 6 — `mt inspect` over a RAW TRANSACTION ──────────────────────────
//
// The post-cut verify step is "scan the engraved QR with a phone, then run
// `mt inspect` on what you get" — and what a scanner hands back is the
// transaction's BYTES, not `mt1` strings. No verb could read one, so the
// device was about to instruct a step no tool could perform. A plate whose
// verification cannot be carried out has not been verified.

fn raw_hex(label: &str) -> String {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["raw_hex"]
        .as_str()
        .unwrap()
        .to_string()
}

fn txid_of(label: &str) -> String {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["txid"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The whole point: the bytes off a scanner reach a report, and the report
/// carries the txid the operator is comparing against the plate.
#[test]
fn inspect_reads_a_raw_transaction_and_reports_its_txid() {
    let out = mt()
        .args(["inspect", "--bitcoin-cli", OFFLINE])
        .write_stdin(raw_hex("even"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inspect over raw hex failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(&txid_of("even")),
        "the report must carry the FULL txid — that is the value being compared \
         against the plate:\n{stdout}"
    );
    // The rows a recoverer needs are the same rows the strings path prints:
    // ONE report implementation, not a second view free to disagree.
    for row in ["TX ", "OUT ", "FEE ", "LOCKTIME "] {
        assert!(stdout.contains(row), "row {row:?} missing:\n{stdout}");
    }
    // ...and the SET rows are ABSENT, because there are no chunks here. A row
    // reading "1 of 1" would claim a set that does not exist.
    assert!(
        !stdout.contains("mt1 SET"),
        "a raw transaction has no chunk set; the report claimed one:\n{stdout}"
    );
    // THE LIMIT, STATED. These bytes say nothing about which PLATE they came
    // off, and a txid identifies a transaction without proving every byte.
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("PLATE") || stderr.contains("plate"),
        "inspect over loose bytes must say what it cannot know:\n{stderr}"
    );
}

/// The strings path is UNTOUCHED. The discriminator is the literal `mt1`, and
/// it is safe by a bech32 property: the data charset excludes `1`, `b`, `i`
/// and `o`, so `1` occurs in an mt1 string only as the HRP separator — and a
/// hex transaction contains no `m` or `t` at all.
#[test]
fn the_strings_path_is_unchanged_by_the_raw_subject() {
    let (out, _) = inspect_offline("even");
    assert!(
        out.contains("mt1 SET"),
        "the strings path lost its SET row:\n{out}"
    );
    assert!(out.contains(&txid_of("even")));
}

/// A REFUSAL FROM `inspect` MUST SAY `inspect`. The sniffing helpers were
/// written for `encode` and hard-code that verb, so routing through them
/// unchanged would tell an operator about a command they did not run.
#[test]
fn a_refused_raw_subject_names_the_verb_the_operator_typed() {
    let out = mt()
        .args(["inspect", "--bitcoin-cli", OFFLINE])
        .write_stdin("abababab")
        .output()
        .unwrap();
    assert!(!out.status.success(), "junk hex must be refused");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("mt inspect:"),
        "the refusal names the wrong verb:\n{stderr}"
    );
    // NON-VACUOUS: `abababab` is valid HEX, so after the routing exists it is
    // judged as a transaction and refused for not parsing as one. Before it,
    // this input was refused as "not an mt1 set" — which passes the verb
    // assertion above while proving nothing about the raw subject.
    assert!(
        !stderr.contains("not an mt1 set"),
        "valid hex was still judged as a set of mt1 strings:\n{stderr}"
    );
    assert!(
        stderr.contains("not a Bitcoin transaction") || stderr.contains("transaction"),
        "the refusal must name what it tried to read:\n{stderr}"
    );
    assert!(
        !stderr.contains("mt encode:"),
        "the refusal names a command the operator did not run:\n{stderr}"
    );
    assert!(out.stdout.is_empty(), "a refusal must leave no artifact");
}

/// THE VERB REWRITE, EXERCISED WHERE IT ACTUALLY BINDS. The two cases above
/// are refused by `decode_tx` and by the PSBT-parse arm, both of which name
/// their own verb -- so neither can see the defect. `input::sniff` builds its
/// refusals with the verb hard-coded to `encode`, and these two inputs are
/// refused BY SNIFF: a hex string that lost a character (odd length, so not a
/// transaction and not a PSBT) and a hex-encoded PSBT. Both are things an
/// operator does; neither may mention a command they did not run.
#[test]
fn refusals_raised_inside_the_sniffer_still_name_inspect() {
    for (input, want) in [
        ("abababa", "is not a PSBT or a raw transaction"),
        ("70736274ff01007502000000", "hex-encoded PSBT"),
    ] {
        let out = mt()
            .args(["inspect", "--bitcoin-cli", OFFLINE])
            .write_stdin(input)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{input:?} must be refused");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.contains(want),
            "{input:?} took the wrong path:\n{stderr}"
        );
        assert!(
            stderr.contains("mt inspect:"),
            "{input:?}: the refusal names the wrong verb:\n{stderr}"
        );
        assert!(
            !stderr.contains("mt encode:"),
            "{input:?}: the refusal names a command the operator did not run:\n{stderr}"
        );
    }
}

/// A PSBT is a legitimate subject too — the operator may be checking what they
/// are about to engrave rather than what they just cut.
#[test]
fn inspect_reads_a_base64_psbt_subject() {
    // The corpus carries raw transactions only, so this asserts the ROUTING:
    // a base64-PSBT-shaped input must not be read as a set of mt1 strings.
    let out = mt()
        .args(["inspect", "--bitcoin-cli", OFFLINE])
        .write_stdin("cHNidP8BAHUCAAAA")
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    // NON-VACUOUS: before the routing existed this refused as "not an mt1 set",
    // and told the operator to run `mt encode` — a different command, for the
    // opposite direction of the journey.
    assert!(
        !stderr.contains("not an mt1 set"),
        "a PSBT was routed down the mt1-strings path:\n{stderr}"
    );
    assert!(
        !stderr.contains("no strings found in the input"),
        "a PSBT was routed down the mt1-strings path:\n{stderr}"
    );
    assert!(stderr.contains("mt inspect:"), "wrong verb:\n{stderr}");
    assert!(
        stderr.to_lowercase().contains("psbt"),
        "the refusal must name what it read the input AS:\n{stderr}"
    );
}

/// The no-node warning names WHERE the transaction was read from, and the raw
/// subject has no strings. Telling that operator mt "read this transaction
/// from the strings" describes a step they did not take -- on the one screen a
/// recoverer reads in a panic.
#[test]
fn the_no_node_warning_names_the_source_it_actually_read() {
    let raw = mt()
        .args(["inspect", "--bitcoin-cli", OFFLINE])
        .write_stdin(raw_hex("even"))
        .output()
        .unwrap();
    let raw_err = String::from_utf8(raw.stderr).unwrap();
    assert!(
        raw_err.contains("bytes you supplied, but could confirm NOTHING"),
        "the raw path claims strings it never read:\n{raw_err}"
    );
    assert!(
        !raw_err.contains("from the\n         strings"),
        "the raw path claims strings it never read:\n{raw_err}"
    );

    // ...and the STRINGS path is byte-for-byte what it was.
    let (_, str_err) = inspect_offline("even");
    assert!(
        str_err.contains("strings, but could confirm NOTHING"),
        "the strings path lost its own wording:\n{str_err}"
    );
    assert!(
        str_err.contains("read from the engraving itself"),
        "the strings path lost its own wording:\n{str_err}"
    );
}

/// `verify` had the SAME wart, and fixing one half of a defect is how the
/// other half survives review. `mt verify --transaction <truncated hex>` is
/// refused by the sniffer too.
#[test]
fn verify_also_names_its_own_verb_on_a_sniffer_refusal() {
    use std::io::Write as _;
    let mut supplied = tempfile::NamedTempFile::new().unwrap();
    supplied.write_all(b"abababa").unwrap();
    supplied.flush().unwrap();
    let f = strings_file("even");
    let out = mt()
        .args(["verify", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .arg("--transaction")
        .arg(supplied.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "truncated hex must be refused");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("is not a PSBT or a raw transaction"),
        "the refusal took a different path than expected:\n{stderr}"
    );
    assert!(stderr.contains("mt verify:"), "wrong verb:\n{stderr}");
    assert!(
        !stderr.contains("mt encode:"),
        "the refusal names a command the operator did not run:\n{stderr}"
    );
}
