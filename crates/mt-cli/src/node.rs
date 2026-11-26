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
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// The `--bitcoin-cli` path the operator named, if they named one.
static NAMED: OnceLock<PathBuf> = OnceLock::new();
/// The one stderr line naming a failed helper has been printed.
static SAID: AtomicBool = AtomicBool::new(false);

/// The operator named `--bitcoin-cli` on the command line (exit-code spec R9),
/// even as `--bitcoin-cli bitcoin-cli`: naming it makes the node required.
/// From here on, a `bitcoin-cli` that is missing, cannot be started, fails,
/// or gives a reply mt cannot use adds IO (32) to the exit code (§3 test 2,
/// "failed or gone missing"). Only the DEFAULT `bitcoin-cli` that is absent
/// or failing is the offline path, a warning that sets nothing.
pub fn helper_named(path: &Path) {
    let _ = NAMED.set(path.to_path_buf());
}

/// Was `--bitcoin-cli` named on the command line?
pub fn is_named() -> bool {
    NAMED.get().is_some()
}

/// A call to a named helper failed, or its reply could not be used: note IO,
/// and say so once on stderr, naming the helper, so a typo and a node that is
/// down are told apart (R5). Core's real answers (a null `gettxout`, a
/// not-found from `getrawtransaction`, a `getindexinfo` without a txindex)
/// are answers and never come here.
fn helper_failed(reason: &dyn std::fmt::Display) {
    let Some(path) = NAMED.get() else {
        return;
    };
    crate::out::note(crate::exit::Exit::IO);
    if !SAID.swap(true, Ordering::Relaxed) {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr(),
            "mt: --bitcoin-cli {}: {reason}",
            path.display()
        );
    }
}

/// `helper_failed`, then `v`: for the branches where a reply arrived but
/// carries nothing mt can use.
fn unusable<T>(v: T) -> T {
    helper_failed(&"gave a reply mt could not read");
    v
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
    /// Locate `bitcoin-cli`. Absent or not runnable is **not a refusal** —
    /// §6a's posture is that offline operation is the constellation's default
    /// and an absent node is an absent answer, not a bad one. (A NAMED helper
    /// that is absent still adds IO to the exit code, R9.)
    pub fn find(cli: &Path) -> Option<Self> {
        let node = Self {
            cli: cli.to_path_buf(),
        };
        match node.call(&["getblockcount"]) {
            // A reply that is no block height is no node: a named helper that
            // gives one has failed (R9), and the run takes the no-node path
            // with its warning.
            Reply::Answered(s) if s.parse::<u64>().is_err() => unusable(None),
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
    ///
    /// Core's own error replies are answers, not a failing helper, but only
    /// from the call that gives them: 5 (not-found, RPC_INVALID_ADDRESS_OR_KEY)
    /// from `getrawtransaction`, and 89 (an older Core's method-not-found,
    /// -32601) from `getindexinfo`. Any other non-zero exit is a failure.
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
            // Missing or not startable. For the default `bitcoin-cli` that is
            // the offline path; for a named one it is a failure (R9).
            Err(e) => {
                helper_failed(&format_args!("could not be started: {e}"));
                return Reply::Failed(None);
            }
        };
        let mut broke = None;
        if let Some(stdin) = child.stdin.as_mut() {
            for a in args {
                if let Err(e) = writeln!(stdin, "{a}") {
                    broke = Some(e);
                    break;
                }
            }
        }
        let out = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                helper_failed(&format_args!("could not be run: {e}"));
                return Reply::Failed(None);
            }
        };
        if !out.status.success() {
            let answer = match args.first() {
                Some(&"getrawtransaction") => Some(5),
                Some(&"getindexinfo") => Some(89),
                _ => None,
            };
            let code = out.status.code();
            if code.is_none() || code != answer {
                match code {
                    Some(n) => helper_failed(&format_args!("exited {n}")),
                    None => helper_failed(&"was stopped by a signal"),
                }
            }
            return Reply::Failed(code);
        }
        if let Some(e) = broke {
            helper_failed(&format_args!("did not read its arguments: {e}"));
            return Reply::Failed(None);
        }
        Reply::Answered(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Current chain height.
    pub fn block_count(&self) -> Option<u64> {
        self.answer(&["getblockcount"])?
            .parse()
            .map_or_else(|_| unusable(None), Some)
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
                Ok(v) => {
                    extract_value_sats(&v).map_or_else(|| unusable(Utxo::Unanswered), Utxo::Unspent)
                }
                Err(_) => unusable(Utxo::Unanswered),
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
        let t = serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| v.get("mediantime")?.as_u64());
        if t.is_none() {
            helper_failed(&"gave a reply mt could not read");
        }
        t
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
            return unusable(ChainReply::NotSaid);
        };
        match v.get("chain").and_then(serde_json::Value::as_str) {
            Some("main") => ChainReply::Known(bitcoin::Network::Bitcoin),
            Some("test") => ChainReply::Known(bitcoin::Network::Testnet),
            Some("testnet4") => ChainReply::Known(bitcoin::Network::Testnet4),
            Some("signet") => ChainReply::Known(bitcoin::Network::Signet),
            Some("regtest") => ChainReply::Known(bitcoin::Network::Regtest),
            Some(_) => ChainReply::Unrecognised,
            None => unusable(ChainReply::NotSaid),
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
        // `{}` (no txindex) is a real answer; a reply that is not JSON is not.
        self.answer(&["getindexinfo"]).is_some_and(|s| {
            match serde_json::from_str::<serde_json::Value>(s.trim()) {
                Ok(v) => {
                    v.get("txindex").and_then(|t| t.get("synced"))
                        == Some(&serde_json::Value::Bool(true))
                }
                Err(_) => unusable(false),
            }
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
                        None => unusable(ParentState::Unanswered),
                    },
                },
                _ => unusable(ParentState::Unanswered),
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
    fn a_missing_binary_is_absence_not_a_node() {
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
