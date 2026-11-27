//! Shared test scaffolding.
//!
//! **`node_stub` lives here because it was needed in TWO files and existed in
//! one.** An independent false-PASS review mutated `mt inspect` to behave as
//! though no node were ever reachable and the entire 117-test suite plus every
//! gate stayed green: `inspect`'s node-consultation path — the one a 2040
//! recoverer actually exercises — had no coverage at all, while `inspect.rs`'s
//! own module doc claimed the tests *"run both offline and with a node"*. The
//! stub was defined in `refusals.rs` and drove only `mt encode`.
//!
//! A `tests/common/` directory is not compiled as its own test binary, so this
//! is shared scaffolding rather than a fourth suite.

#![allow(dead_code)] // each test binary uses a different subset

use bitcoin::consensus::deserialize;

/// A stand-in `bitcoin-cli` that answers from a script rather than a chain.
///
/// **Not a convenience.** §8.5 and §6a are the two refusals that cannot fire
/// without a node, so testing them against the real one would make them
/// unrunnable in CI — and untested is how a refusal that never fires looks from
/// the outside.
///
/// **IT ANSWERS `getrawtransaction` PER TXID, and that is the whole point.**
/// An earlier version answered the same for every argument, which made it model
/// *"somebody else took the input"* and *"this transaction already confirmed"*
/// IDENTICALLY — they share `gettxout -> null` and a confirmed parent, and
/// differ only in whether the TRANSACTION'S OWN txid is on chain. A live node
/// found the defect that stub could not: `encode` refused a confirmed
/// transaction with *"this transaction can never be broadcast"* and told the
/// operator to build a new one, which is advice to pay twice.
pub struct Stub {
    dir: tempfile::TempDir,
    path: std::path::PathBuf,
}

/// Build a stub. `gettxout` is the raw JSON to return (empty = Core's null:
/// no output, exit 0), and `confirmations` maps a txid PREFIX to a
/// confirmation count; any txid not listed exits 5, Core's not-found code
/// (RPC_INVALID_ADDRESS_OR_KEY), which mt reads as NOT FOUND.
pub fn node_stub(gettxout: &str, confirmations: &[(&str, u32)]) -> Stub {
    // A `case` arm per known txid, matched by prefix so a test can name a txid
    // without pasting all 64 characters.
    let mut arms = String::new();
    for (txid, n) in confirmations {
        use core::fmt::Write as _;
        let _ = writeln!(arms, "    {txid}*) echo '{{\"confirmations\": {n}}}' ;;");
    }
    let script = format!(
        r#"#!/bin/sh
# bitcoin-cli's -stdin form: one argument per line.
verb=""; arg1=""
while read -r line; do
  if [ -z "$verb" ]; then verb="$line"
  elif [ -z "$arg1" ]; then arg1="$line"
  fi
done
case "$verb" in
  getblockcount)     echo 963832 ;;
  getindexinfo)      echo '{{"txindex": {{"synced": true, "best_block_height": 963832}}}}' ;;
  getblockchaininfo) echo '{{"chain": "regtest", "mediantime": 1700000000}}' ;;
  gettxout)          {gettxout} ;;
  getrawtransaction)
    case "$arg1" in
{arms}      *) exit 5 ;;
    esac ;;
  *) exit 1 ;;
esac
"#,
        gettxout = if gettxout.is_empty() {
            "exit 0".to_string()
        } else {
            format!("echo '{gettxout}'")
        },
    );
    // A DIRECTORY, not a NamedTempFile. A NamedTempFile stays open for writing
    // for its whole lifetime, and Linux refuses to exec a file that is open for
    // writing -- ETXTBSY. The failure is silent: exec fails, Node::find returns
    // None, and every chain row reads UNKNOWN, so the test reports "no node
    // reachable" rather than "the stub could not run".
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bitcoin-cli");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    Stub { dir, path }
}

impl Stub {
    pub fn path(&self) -> &std::path::Path {
        let _ = &self.dir; // the TempDir must outlive the path
        &self.path
    }
}

/// The txids of the fixture transaction and of its inputs' parents.
pub fn fixture_txids(v: &serde_json::Value) -> (String, Vec<String>) {
    let hex = v["raw_hex"].as_str().expect("fixture has no raw_hex");
    let tx: bitcoin::Transaction = deserialize(&hex_to_bytes(hex)).unwrap();
    let own = tx.compute_txid().to_string();
    let parents = tx
        .input
        .iter()
        .map(|i| i.previous_output.txid.to_string())
        .collect();
    (own, parents)
}

/// Hex to bytes. Duplicated from the suites rather than exported from them,
/// because a `tests/common/` module cannot depend on its own consumers.
pub fn hex_to_bytes(s: &str) -> Vec<u8> {
    s.trim()
        .as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

/// The constellation exit codes (mnemonic-engrave
/// `SPEC_constellation_exit_codes.md` §3), as the tests name them. 1 to 63 are
/// sums of the six bits; 64 and 101 never combine.
pub mod exit {
    pub const OK: i32 = 0;
    pub const NO: i32 = 1;
    pub const INVALID: i32 = 2;
    pub const UNVERIFIED: i32 = 4;
    pub const REFUSED: i32 = 8;
    pub const REPAIRED: i32 = 16;
    pub const IO: i32 = 32;
    pub const USAGE: i32 = 64;
    pub const INTERNAL: i32 = 101;
}
