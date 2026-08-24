//! §8.4 — the two fields, and the two failures the section was written to close.
//!
//! Both were present in the shipped binary until a spec-first review looked for
//! the code implementing them and found none. They are asserted here as
//! CONTRASTS — the same transaction with one field changed — because each
//! failure is a *wrong answer*, not a missing one, and a test that only checks
//! the right case passes against a tool that always says the same thing.

use assert_cmd::Command;
use bitcoin::consensus::{deserialize, serialize};
use std::io::Write;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

const OFFLINE: &str = "/nonexistent/bitcoin-cli";

fn base_tx() -> bitcoin::Transaction {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let hex = v["raw_hex"].as_str().unwrap();
    let bytes: Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(core::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect();
    deserialize(&bytes).unwrap()
}

/// Encode a transaction offline and return the `LOCKTIME` row.
fn locktime_row(tx: &bitcoin::Transaction) -> String {
    use core::fmt::Write as _;
    let mut hex = String::new();
    for b in serialize(tx) {
        let _ = write!(hex, "{b:02x}");
    }
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(hex.as_bytes()).unwrap();
    f.flush().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let out = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    err.lines()
        .find(|l| l.starts_with("LOCKTIME"))
        .unwrap_or_else(|| panic!("no LOCKTIME row:\n{err}"))
        .to_string()
}

fn with_locktime(n: u32) -> bitcoin::Transaction {
    let mut tx = base_tx();
    tx.lock_time = bitcoin::absolute::LockTime::from_consensus(n);
    tx
}

fn all_inputs_final(mut tx: bitcoin::Transaction) -> bitcoin::Transaction {
    for i in &mut tx.input {
        i.sequence = bitcoin::Sequence(0xFFFF_FFFF);
    }
    tx
}

/// **A permanent falsehood.** `nLockTime = 1800000000` is a Unix TIMESTAMP —
/// 2027 — and reporting it as a block height names a block some thirty thousand
/// years out. A holder could reasonably read that as "never" and discard the
/// plate.
#[test]
fn a_timestamp_locktime_is_never_presented_as_a_height() {
    let row = locktime_row(&with_locktime(1_800_000_000));
    assert!(
        row.contains("LOCKED UNTIL 2027-"),
        "a timestamp was not rendered as a date: {row}"
    );
    assert!(
        !row.contains("1800000000"),
        "the raw timestamp leaked into the row as though it were a height: {row}"
    );
    assert!(
        !row.contains("BLOCK"),
        "a timestamp was presented as a block: {row}"
    );
}

/// The contrast, one value apart: just below the threshold it IS a height.
#[test]
fn a_value_below_the_threshold_is_a_height() {
    let row = locktime_row(&with_locktime(499_999_999));
    assert!(row.contains("LOCKED TO BLOCK 499999999"), "{row}");
    let row = locktime_row(&with_locktime(500_000_000));
    assert!(
        row.contains("LOCKED UNTIL"),
        "the threshold is exclusive on the wrong side: {row}"
    );
}

/// **False reassurance, which §8.4 calls the worst failure available here.**
/// `nLockTime` is enforced only if some input has `nSequence != 0xFFFFFFFF`; a
/// transaction with every input final ignores it, so reporting a lock would
/// describe a plate anyone can broadcast today as time-locked.
#[test]
fn a_locktime_no_input_enforces_is_reported_as_not_enforced() {
    let enforced = locktime_row(&with_locktime(96));
    let ignored = locktime_row(&all_inputs_final(with_locktime(96)));

    assert!(
        enforced.contains("LOCKED TO BLOCK 96"),
        "a non-final input must make the lock live: {enforced}"
    );
    assert!(
        ignored.contains("NOT ENFORCED (all inputs final)"),
        "a lock consensus will ignore was reported as a lock: {ignored}"
    );
    assert_ne!(
        enforced, ignored,
        "the nSequence field changed and the report did not"
    );
}

/// §8.4's five spellings and no sixth. The row is bound to the section's set,
/// and `mt` may not invent one — two `mt` versions would otherwise cut different
/// plates for one transaction, and a recoverer matching against documentation
/// would find neither.
#[test]
fn the_row_uses_only_the_ruled_spellings() {
    let rows = [
        locktime_row(&with_locktime(0)),
        locktime_row(&with_locktime(96)),
        locktime_row(&with_locktime(1_800_000_000)),
        locktime_row(&all_inputs_final(with_locktime(96))),
    ];
    for row in &rows {
        let body = row.trim_start_matches("LOCKTIME").trim();
        assert!(
            body.starts_with("NO TIMELOCK")
                || body.starts_with("LOCKED TO BLOCK ")
                || body.starts_with("LOCKED UNTIL ")
                || body.starts_with("nLockTime "),
            "a sixth spelling: {row}"
        );
        // §8.4 forbids a verdict: the report states facts and stops.
        for forbidden in ["SPENDABLE", "may be", "PASSED", "SAFE"] {
            assert!(
                !row.contains(forbidden),
                "the row rendered a verdict: {row}"
            );
        }
    }
    // `NO TIMELOCK`, that exact spelling, 11 characters, normative everywhere.
    assert_eq!("NO TIMELOCK".len(), 11);
}

/// A lock whose height is already behind this build's reference has no future
/// date to name, so §8.4 requires a warning rather than a past year.
#[test]
fn a_lock_below_the_reference_height_warns() {
    let tx = with_locktime(900_000);
    use core::fmt::Write as _;
    let mut hex = String::new();
    for b in serialize(&tx) {
        let _ = write!(hex, "{b:02x}");
    }
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(hex.as_bytes()).unwrap();
    f.flush().unwrap();
    let out = mt()
        .args(["encode", "--bitcoin-cli", OFFLINE, "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("is BELOW this build's reference height 963759"),
        "no below-reference warning: {err}"
    );
    assert!(
        flat.contains("Treat it as spendable now"),
        "the warning must say what it means: {err}"
    );
    assert!(!err.contains("REFUSED"), "§8.4 says NEVER REFUSE: {err}");
}
