//! Reaching a node — by **shelling out to `bitcoin-cli -stdin`**, never by
//! speaking JSON-RPC.
//!
//! `bitcoin-cli` already holds the RPC URL, the cookie or credentials, the
//! network, the datadir and the wallet selection. So §6a's *"the operator is
//! asked for nothing"* is true **by construction** rather than by adding a flag
//! they must fill in: `mt` works exactly when the operator's node works, with no
//! second place to configure and no way for `mt`'s idea of the node to drift
//! from `bitcoin-cli`'s.
//!
//! **`-stdin` is not optional, and the reason is §8.2f.** Arguments go on stdin,
//! one per line, never on the command line: `bitcoin-cli gettxout <txid> 0 false`
//! puts the txid in `ps` for every user on the machine. That is the same leak
//! §8.2f refuses for transactions — smaller, since a txid is not a bearer
//! instrument, but free to avoid. `bitcoin-cli`'s own help calls `-stdin`
//! *"recommended for sensitive information"*.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A node, or the absence of one.
#[derive(Debug, Clone)]
pub struct Node {
    cli: PathBuf,
}

/// What a node can say about an input's parent transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentState {
    /// In a block. Combined with a null `gettxout`, this is the ONLY combination
    /// that means someone else spent the input.
    Confirmed,
    /// Found, unconfirmed. The engraving may still become live.
    InMempool,
    /// Not found. With `-txindex` this means it is genuinely not on chain;
    /// without it, the answer is simply unavailable.
    NotFound,
}

/// What `gettxout` says about one outpoint, and what it cannot say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Utxo {
    /// Present in the UTXO set, with its value in satoshis.
    ///
    /// **"Unspent in the UTXO set", not "unspent".** `include_mempool` is
    /// `false` by ruling, so a conflicting spend already sitting unconfirmed
    /// still reads as present here.
    Unspent(u64),
    /// `gettxout` returned null: spent, or never existed, or the parent has not
    /// confirmed. Which of those it is takes a second question.
    Null,
}

impl Node {
    /// Locate `bitcoin-cli`. Absent or not runnable is **not an error** — §6a's
    /// posture is that offline operation is the constellation's default and an
    /// absent node is an absent answer, not a bad one.
    pub fn find(cli: &Path) -> Option<Self> {
        let node = Self {
            cli: cli.to_path_buf(),
        };
        node.call(&["getblockcount"]).map(|_| node)
    }

    /// Run `bitcoin-cli -stdin`, passing every argument on **stdin**.
    fn call(&self, args: &[&str]) -> Option<String> {
        use std::io::Write;
        let mut child = Command::new(&self.cli)
            .arg("-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        {
            let stdin = child.stdin.as_mut()?;
            for a in args {
                writeln!(stdin, "{a}").ok()?;
            }
        }
        let out = child.wait_with_output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Current chain height.
    pub fn block_count(&self) -> Option<u64> {
        self.call(&["getblockcount"])?.parse().ok()
    }

    /// Is this outpoint in the UTXO set?
    ///
    /// `include_mempool` is **false**: a plate sits in a drawer for years, and
    /// the mempool is a statement about the last few hours. The cost is that a
    /// mempool-spent input reads as unspent, which is why `LIVE` says *"unspent
    /// in the UTXO set"* rather than *"unspent"*.
    pub fn txout(&self, txid: &str, vout: u32) -> Utxo {
        let vout = vout.to_string();
        match self.call(&["gettxout", txid, &vout, "false"]) {
            Some(s) if !s.is_empty() => extract_value_sats(&s).map_or(Utxo::Null, Utxo::Unspent),
            _ => Utxo::Null,
        }
    }

    /// The chain's current **median-time-past**.
    ///
    /// §8.4's *"compare like with like"*: a timestamp `nLockTime` is compared
    /// against MTP, never against a block height and never against the header
    /// stamp. MTP is what consensus actually enforces a time-lock against, and
    /// it is monotonic; a header `nTime` may run up to two hours fast.
    pub fn median_time(&self) -> Option<u64> {
        let json = self.call(&["getblockchaininfo"])?;
        let at = json.find("\"mediantime\"")?;
        let rest = &json[at + 12..];
        let colon = rest.find(':')?;
        let tail = &rest[colon + 1..];
        let end = tail.find(|c: char| !c.is_ascii_digit() && !c.is_whitespace())?;
        tail[..end].trim().parse().ok()
    }

    /// Which chain this node is on.
    ///
    /// **So the report's addresses are addresses on the operator's network.**
    /// A `scriptPubKey` carries no network, so mt cannot know from the
    /// transaction alone — it rendered every output with MAINNET parameters, and
    /// a regtest transaction therefore showed `bc1q…` for an output the node
    /// calls `bcrt1q…`: the same witness program under a different HRP, so the
    /// printed string is not an address anywhere.
    ///
    /// Read from the node rather than asked of the operator, which is §6a's
    /// posture: `bitcoin-cli` already knows, so mt does not add a flag they
    /// must remember to set correctly.
    pub fn chain(&self) -> Option<bitcoin::Network> {
        let json = self.call(&["getblockchaininfo"])?;
        let at = json.find("\"chain\"")?;
        let rest = &json[at + 7..];
        let start = rest.find('"')? + 1;
        let end = rest[start..].find('"')? + start;
        match &rest[start..end] {
            "main" => Some(bitcoin::Network::Bitcoin),
            "test" => Some(bitcoin::Network::Testnet),
            "signet" => Some(bitcoin::Network::Signet),
            "regtest" => Some(bitcoin::Network::Regtest),
            _ => None,
        }
    }

    /// Does this node have `-txindex`?
    ///
    /// **It decides whether "not found" means PENDING or UNKNOWN.** Without the
    /// index, `getrawtransaction` searches only the mempool, so a miss cannot
    /// distinguish "never broadcast" from "confirmed long ago" — and §6a forbids
    /// printing DEAD on evidence that cannot tell those apart, because telling a
    /// recoverer their engraving is scrap when it is merely early is the worst
    /// error available.
    pub fn has_txindex(&self) -> bool {
        self.call(&["getindexinfo"])
            .is_some_and(|s| s.contains("txindex") && s.contains("\"synced\": true"))
    }

    /// Has this transaction **confirmed**?
    ///
    /// Not merely "is it findable": `getrawtransaction` finds a transaction in
    /// the mempool too, and **found is not confirmed**. Only one of those means
    /// someone else took the money — which is why `DEAD` requires this and not
    /// the weaker question.
    pub fn is_confirmed(&self, txid: &str) -> ParentState {
        match self.call(&["getrawtransaction", txid, "true"]) {
            Some(json) if !json.is_empty() => {
                let confirmed = json.contains("\"confirmations\"")
                    && !json.contains("\"confirmations\": 0")
                    && !json.contains("\"confirmations\":0");
                if confirmed {
                    ParentState::Confirmed
                } else {
                    // Found, but not confirmed: it is sitting in the mempool.
                    // FOUND IS NOT CONFIRMED, and only one of those means
                    // someone else took the money.
                    ParentState::InMempool
                }
            }
            _ => ParentState::NotFound,
        }
    }
}

/// Pull `"value": 0.00123` out of `gettxout`'s JSON and convert to satoshis.
///
/// §6a rules the value is USED, not merely its null-ness: `gettxout` returns
/// both `value` and `scriptPubKey`, which is the stated reason for choosing it
/// over a bare existence check.
fn extract_value_sats(json: &str) -> Option<u64> {
    let at = json.find("\"value\"")?;
    let rest = &json[at + 7..];
    let colon = rest.find(':')?;
    let tail = &rest[colon + 1..];
    let end = tail
        .find(|c: char| c != '.' && !c.is_ascii_digit() && !c.is_whitespace())
        .unwrap_or(tail.len());
    let btc: f64 = tail[..end].trim().parse().ok()?;
    Some((btc * 100_000_000.0).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_binary_is_absence_not_failure() {
        assert!(
            Node::find(Path::new("/nonexistent/bitcoin-cli")).is_none(),
            "an absent bitcoin-cli must yield None, never panic"
        );
    }

    #[test]
    fn parses_a_value_into_satoshis() {
        let json = r#"{ "bestblock": "0000", "confirmations": 3, "value": 0.05000000, "scriptPubKey": {} }"#;
        assert_eq!(extract_value_sats(json), Some(5_000_000));
    }

    #[test]
    fn parses_a_whole_btc_value() {
        assert_eq!(
            extract_value_sats(r#"{"value": 1.00000000}"#),
            Some(100_000_000)
        );
    }

    #[test]
    fn absent_value_is_none() {
        assert_eq!(extract_value_sats(r#"{"bestblock":"0000"}"#), None);
    }
}
