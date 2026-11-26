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
use std::sync::atomic::{AtomicBool, Ordering};

static NAMED: AtomicBool = AtomicBool::new(false);

/// The operator named `--bitcoin-cli` on the command line (exit-code spec R9).
/// From here on, a `bitcoin-cli` that runs and does not answer adds IO (32) to
/// the exit code. An absent one does not: pointing the flag at nothing is the
/// documented way to force the offline path, and that is "no node", not a
/// failing one.
pub fn helper_named() {
    NAMED.store(true, Ordering::Relaxed);
}

/// A call to a named helper failed: note IO, once is enough.
fn helper_failed() {
    if NAMED.load(Ordering::Relaxed) {
        crate::out::note(crate::exit::Exit::IO);
    }
}

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
    /// No usable answer: the call failed with anything but Core's not-found
    /// (exit 5), or its reply is not an object whose `confirmations`, if
    /// present, is a non-negative integer. Never read as DEAD or PENDING.
    Unanswered,
}

/// What `getblockchaininfo` said about the chain (N4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainReply {
    /// A chain name mt knows (`main`, `test`, `testnet4`, `signet`, `regtest`).
    Known(bitcoin::Network),
    /// A string naming a chain mt does not know.
    Unrecognised,
    /// The call failed, or the reply is not a JSON object with a string `chain`.
    NotSaid,
}

/// One `bitcoin-cli -stdin` call: it answered (exit 0, stdout trimmed), or it
/// failed (could not run, or exited non-zero, keeping the exit code if any).
/// An empty answer is Core's null, not a failure.
enum Reply {
    Answered(String),
    Failed(Option<i32>),
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
    /// No usable answer: the call failed, or its reply was neither null nor an
    /// object whose `value` is an amount (N1, N2). Never read as `Null`.
    Unanswered,
}

impl Node {
    /// Locate `bitcoin-cli`. Absent or not runnable is **not an error** — §6a's
    /// posture is that offline operation is the constellation's default and an
    /// absent node is an absent answer, not a bad one.
    pub fn find(cli: &Path) -> Option<Self> {
        let node = Self {
            cli: cli.to_path_buf(),
        };
        match node.call(&["getblockcount"]) {
            Reply::Answered(_) => Some(node),
            Reply::Failed(_) => None,
        }
    }

    fn answer(&self, args: &[&str]) -> Option<String> {
        match self.call(args) {
            Reply::Answered(s) => Some(s),
            Reply::Failed(_) => None,
        }
    }

    /// Run `bitcoin-cli -stdin`, passing every argument on **stdin**.
    fn call(&self, args: &[&str]) -> Reply {
        use std::io::Write;
        let mut child = match Command::new(&self.cli)
            .arg("-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            // Absent: the offline path, not a failure (R9).
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Reply::Failed(None),
            Err(_) => {
                helper_failed();
                return Reply::Failed(None);
            }
        };
        {
            let Some(stdin) = child.stdin.as_mut() else {
                helper_failed();
                return Reply::Failed(None);
            };
            for a in args {
                if writeln!(stdin, "{a}").is_err() {
                    let _ = child.wait();
                    helper_failed();
                    return Reply::Failed(None);
                }
            }
        }
        let Ok(out) = child.wait_with_output() else {
            helper_failed();
            return Reply::Failed(None);
        };
        if !out.status.success() {
            // Core's own error replies are answers, not a failing helper:
            // 5 is not-found (RPC_INVALID_ADDRESS_OR_KEY), 89 is an older
            // Core's method-not-found (-32601) for `getindexinfo`.
            if !matches!(out.status.code(), Some(5 | 89)) {
                helper_failed();
            }
            return Reply::Failed(out.status.code());
        }
        Reply::Answered(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Current chain height.
    pub fn block_count(&self) -> Option<u64> {
        self.answer(&["getblockcount"])?.parse().ok()
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
            Reply::Failed(_) => Utxo::Unanswered,
            Reply::Answered(s) if s.is_empty() => Utxo::Null,
            Reply::Answered(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(serde_json::Value::Null) => Utxo::Null,
                Ok(v) => extract_value_sats(&v).map_or(Utxo::Unanswered, Utxo::Unspent),
                Err(_) => Utxo::Unanswered,
            },
        }
    }

    /// The chain's current **median-time-past**.
    ///
    /// §8.4's *"compare like with like"*: a timestamp `nLockTime` is compared
    /// against MTP, never against a block height and never against the header
    /// stamp. MTP is what consensus actually enforces a time-lock against, and
    /// it is monotonic; a header `nTime` may run up to two hours fast.
    pub fn median_time(&self) -> Option<u64> {
        // JSON, and only a non-negative integer counts (N5b): `1.7e9` is no MTP.
        let json = self.answer(&["getblockchaininfo"])?;
        serde_json::from_str::<serde_json::Value>(&json)
            .ok()?
            .get("mediantime")?
            .as_u64()
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
    pub fn chain(&self) -> ChainReply {
        let Some(json) = self.answer(&["getblockchaininfo"]) else {
            return ChainReply::NotSaid;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else {
            return ChainReply::NotSaid;
        };
        match v.get("chain").and_then(serde_json::Value::as_str) {
            Some("main") => ChainReply::Known(bitcoin::Network::Bitcoin),
            Some("test") => ChainReply::Known(bitcoin::Network::Testnet),
            Some("testnet4") => ChainReply::Known(bitcoin::Network::Testnet4),
            Some("signet") => ChainReply::Known(bitcoin::Network::Signet),
            Some("regtest") => ChainReply::Known(bitcoin::Network::Regtest),
            Some(_) => ChainReply::Unrecognised,
            None => ChainReply::NotSaid,
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
    ///
    /// The reply is parsed as JSON and only txindex's own boolean
    /// `synced: true` counts; anything else, a parse failure included, is "no
    /// index", which reads UNKNOWN (§6a's fail-safe direction), because another
    /// synced index once made an unsynced txindex read as synced (bug hunt #5).
    pub fn has_txindex(&self) -> bool {
        self.answer(&["getindexinfo"]).is_some_and(|s| {
            serde_json::from_str::<serde_json::Value>(s.trim()).is_ok_and(|v| {
                v.get("txindex").and_then(|t| t.get("synced"))
                    == Some(&serde_json::Value::Bool(true))
            })
        })
    }

    /// Has this transaction **confirmed**?
    ///
    /// Not merely "is it findable": `getrawtransaction` finds a transaction in
    /// the mempool too, and **found is not confirmed**. Only one of those means
    /// someone else took the money — which is why `DEAD` requires this and not
    /// the weaker question.
    pub fn is_confirmed(&self, txid: &str) -> ParentState {
        match self.call(&["getrawtransaction", txid, "true"]) {
            Reply::Failed(Some(5)) => ParentState::NotFound,
            Reply::Failed(_) => ParentState::Unanswered,
            Reply::Answered(json) => match serde_json::from_str::<serde_json::Value>(&json) {
                Ok(serde_json::Value::Object(o)) => match o.get("confirmations") {
                    None => ParentState::InMempool,
                    Some(c) => match c.as_u64() {
                        Some(0) => ParentState::InMempool,
                        Some(_) => ParentState::Confirmed,
                        None => ParentState::Unanswered,
                    },
                },
                _ => ParentState::Unanswered,
            },
        }
    }
}

/// Pull `"value": 0.00123` out of `gettxout`'s JSON and convert to satoshis.
///
/// §6a rules the value is USED, not merely its null-ness: `gettxout` returns
/// both `value` and `scriptPubKey`, which is the stated reason for choosing it
/// over a bare existence check.
fn extract_value_sats(v: &serde_json::Value) -> Option<u64> {
    let n = v.as_object()?.get("value")?;
    if !n.is_number() {
        return None;
    }
    let f = n.as_f64()?;
    if f.is_sign_negative() {
        return None;
    }
    let a = bitcoin::Amount::from_btc(f).ok()?;
    (a <= bitcoin::Amount::MAX_MONEY).then_some(a.to_sat())
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
        assert_eq!(
            extract_value_sats(&serde_json::from_str(json).unwrap()),
            Some(5_000_000)
        );
    }

    #[test]
    fn parses_a_whole_btc_value() {
        assert_eq!(
            extract_value_sats(&serde_json::from_str(r#"{"value": 1.00000000}"#).unwrap()),
            Some(100_000_000)
        );
    }

    #[test]
    fn absent_value_is_none() {
        assert_eq!(
            extract_value_sats(&serde_json::from_str(r#"{"bestblock":"0000"}"#).unwrap()),
            None
        );
    }
}
