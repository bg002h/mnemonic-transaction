//! The report — **stated once, three callers, one layout** (§1.1).
//!
//! `encode`, `decode` and `inspect` all print this. They differ only in the
//! stream they print to and, for `encode`, a two-row suffix. **No caller
//! reorders, reformats, or drops a row** — because if `encode` composed its own
//! version, the operator's pre-engraving view and the 2040 recoverer's view
//! would be two implementations of the same thing, free to disagree.
//!
//! Three rules govern every row:
//!
//! 1. **A row is never omitted for being unanswerable — it reads `UNKNOWN`.**
//!    Omission and ignorance look identical on a terminal, and a reader cannot
//!    tell a row that was skipped from one that never existed.
//! 2. **Read and verified are visually distinct, and there are THREE classes:**
//!    read off the plate, fetched from the chain, and *claimed by nobody who
//!    checked* — which is where an unverified fee sits.
//! 3. **`encode` appends, never edits.**

use crate::node::{Node, ParentState, Utxo};
use bitcoin::Transaction;
use std::fmt::Write as _;

/// Where an input's value came from.
///
/// **§10.10 rules THREE columns — verified, claimed, absent — over FIVE
/// sources**, and conflating any two of them is a defect this artifact has
/// already produced. The middle column is the one that matters: *"between them
/// sits operator-asserted **or PSBT-claimed** data, which nothing checked"*.
///
/// > **Collapsing it put an unverified number in the verified column — R6
/// > adversarial I-5.** Air-gapped `mt encode`, which is the constellation's own
/// > posture: a PSBT carries `witness_utxo` for a segwit input claiming 1.0 BTC.
/// > No node, so §6a's comparison does not run. Not legacy, so §8.2d's txid
/// > binding does not apply. **Nothing on any path checks that number**, and it
/// > is the one the operator uses to decide whether to cut at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Fetched from the chain (§6a). **Verified.**
    ChainFetched,
    /// Read from a `non_witness_utxo` whose hash reproduced the input's txid
    /// (§8.2d). **Verified** — by proof-of-work-anchored history rather than by
    /// anyone's word, and forging it would need a txid collision.
    TxidBound,
    /// Read from a PSBT's `witness_utxo`, with no node to check it against
    /// (§8.2c). **Claimed.** The wallet that wrote the PSBT said so; nothing
    /// since has agreed.
    PsbtClaimed,
    /// Supplied by the operator with `--input-value` (§8.2c). **Claimed.**
    OperatorAsserted,
    /// Not available at all.
    Unknown,
}

impl Provenance {
    /// Did anything actually check this number?
    ///
    /// The question the middle column exists to answer, asked once so no render
    /// site has to re-derive it — which is how the two classes got collapsed.
    pub fn is_verified(self) -> bool {
        matches!(self, Provenance::ChainFetched | Provenance::TxidBound)
    }

    /// How this value is labelled in the `INPUTS` rows.
    fn label(self) -> &'static str {
        match self {
            Provenance::ChainFetched => "from node",
            Provenance::TxidBound => "TXID-BOUND",
            Provenance::PsbtClaimed => "PSBT-CLAIMED — unverified",
            Provenance::OperatorAsserted => "OPERATOR-ASSERTED",
            Provenance::Unknown => "UNKNOWN",
        }
    }
}

/// Plate liveness. **Five states, not four** — the first one is asked before any
/// input is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// This transaction itself already confirmed. **Asked first**, because
    /// otherwise the success case reports as the theft case: every input of a
    /// confirmed transaction is spent (by itself) and every parent is confirmed,
    /// which is exactly the DEAD condition.
    AlreadyConfirmed,
    /// Every input unspent in the UTXO set.
    Live,
    /// An input was spent and its parent is **confirmed** — someone else took it.
    Dead,
    /// An input's parent has not confirmed. The plate may still become live.
    Pending,
    /// No node reachable at all.
    Unknown,
    /// A node answered, but cannot distinguish DEAD from PENDING — `gettxout`
    /// is null and there is no `-txindex` to ask about the parent.
    ///
    /// Reported as its own state rather than folded into DEAD, because
    /// **printing DEAD on evidence that cannot distinguish it from PENDING is
    /// the one error that gets a live engraving thrown away.**
    Indeterminate,
}

impl Status {
    fn render(self) -> &'static str {
        match self {
            Self::AlreadyConfirmed => "SPENT — ALREADY CONFIRMED (this transaction is in a block)",
            Self::Live => {
                "LIVE — every input unspent in the UTXO set (mempool not consulted;\n          \
                 a conflicting spend may already be in flight)"
            }
            Self::Dead => "DEAD — an input was spent by someone else. The engraving is scrap",
            Self::Pending => "PENDING — a parent has not confirmed. This may still become live",
            Self::Unknown => "UNKNOWN — no node reachable",
            Self::Indeterminate => {
                "UNKNOWN — an input is not in the UTXO set, and without -txindex\n                           this node cannot tell whether its parent confirmed. mt will not\n                           guess DEAD on evidence that cannot distinguish it from PENDING"
            }
        }
    }
}

/// Everything the report can say.
pub struct Report {
    /// The **txid** — witness-stripped, and *not* the hash of the engraved bytes.
    pub txid: String,
    /// Strings in the set, when the caller had strings.
    pub set: Option<(usize, usize)>,
    /// Outputs, as (address-or-script, satoshis).
    pub outputs: Vec<(String, u64)>,
    /// Fee in satoshis, and the weakest provenance of any input.
    pub fee: Option<(u64, Provenance)>,
    /// `nLockTime`, and the chain height if a node answered.
    pub locktime: (u32, Option<u64>),
    /// Inputs, as (outpoint, value, provenance).
    pub inputs: Vec<(String, Option<u64>, Provenance)>,
    /// Liveness.
    pub status: Status,
}

impl Report {
    /// Build the report for a transaction, consulting a node when one is there.
    pub fn build(
        tx: &Transaction,
        txid: &str,
        node: Option<&Node>,
        claimed: &[(u32, u64, Provenance)],
    ) -> Self {
        // ASKED FIRST: did THIS transaction already confirm? Every other row is
        // a guess about *why* the inputs are gone; this answers it exactly.
        let already = node.is_some_and(|n| n.is_confirmed(txid) == ParentState::Confirmed);
        // Whether "not found" is informative at all depends on the index.
        let txindex = node.is_some_and(Node::has_txindex);

        let mut inputs = Vec::new();
        let mut total_in = 0u64;
        let mut all_known = true;
        let mut any_dead = false;
        let mut any_pending = false;
        let mut any_unknown = false;
        let mut operator_supplied = false;

        for (idx, i) in tx.input.iter().enumerate() {
            let idx = idx as u32;
            let op = format!("{}:{}", i.previous_output.txid, i.previous_output.vout);
            match node {
                Some(n) => {
                    match n.txout(&i.previous_output.txid.to_string(), i.previous_output.vout) {
                        Utxo::Unspent(sats) => {
                            total_in += sats;
                            inputs.push((op, Some(sats), Provenance::ChainFetched));
                        }
                        Utxo::Null => {
                            all_known = false;
                            // DEAD requires the parent to be CONFIRMED, not merely
                            // found: getrawtransaction finds a mempool transaction
                            // too, and only confirmation means someone else took it.
                            match n.is_confirmed(&i.previous_output.txid.to_string()) {
                                ParentState::Confirmed => any_dead = true,
                                ParentState::InMempool => any_pending = true,
                                // Not found WITH an index means genuinely not on
                                // chain — PENDING. Without one it is unanswerable,
                                // and mt says UNKNOWN rather than guessing DEAD.
                                ParentState::NotFound if txindex => any_pending = true,
                                ParentState::NotFound => any_unknown = true,
                            }
                            inputs.push((op, None, Provenance::Unknown));
                        }
                    }
                }
                None => {
                    // No node — but the value may still be known, and by a
                    // route that is NOT the chain: a PSBT's own UTXO record, a
                    // txid-bound non_witness_utxo, or the operator's word.
                    // Which one it was decides the column it renders in.
                    match claimed.iter().find(|(i, _, _)| *i == idx) {
                        Some(&(_, sats, prov)) => {
                            total_in += sats;
                            if !prov.is_verified() {
                                operator_supplied = true;
                            }
                            inputs.push((op, Some(sats), prov));
                        }
                        None => {
                            all_known = false;
                            inputs.push((op, None, Provenance::Unknown));
                        }
                    }
                }
            }
        }

        let total_out: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
        // The FEE carries the WEAKEST provenance of any input. If one value was
        // asserted rather than fetched, the whole figure is claimed — and it is
        // the number the operator uses to decide whether to cut at all.
        let fee = if all_known && total_in >= total_out {
            // The WEAKEST provenance of any input. One claimed value makes the
            // whole figure claimed — it is a sum, so it is exactly as trustworthy
            // as its least trustworthy term.
            let prov = if operator_supplied {
                inputs
                    .iter()
                    .map(|(_, _, p)| *p)
                    .find(|p| !p.is_verified())
                    .unwrap_or(Provenance::OperatorAsserted)
            } else {
                Provenance::ChainFetched
            };
            Some((total_in - total_out, prov))
        } else {
            None
        };

        let status = if already {
            Status::AlreadyConfirmed
        } else if node.is_none() {
            Status::Unknown
        } else if any_dead {
            Status::Dead
        } else if any_unknown {
            Status::Indeterminate
        } else if any_pending {
            Status::Pending
        } else if all_known {
            Status::Live
        } else {
            Status::Indeterminate
        };

        Self {
            txid: txid.to_string(),
            set: None,
            outputs: tx
                .output
                .iter()
                .map(|o| {
                    let spk =
                        bitcoin::Address::from_script(&o.script_pubkey, bitcoin::Network::Bitcoin)
                            .map_or_else(|_| o.script_pubkey.to_string(), |a| a.to_string());
                    (spk, o.value.to_sat())
                })
                .collect(),
            fee,
            locktime: (
                tx.lock_time.to_consensus_u32(),
                node.and_then(Node::block_count),
            ),
            inputs,
            status,
        }
    }

    /// Render. Rows are never omitted — an unanswerable row reads `UNKNOWN`.
    pub fn render(&self) -> String {
        let mut s = String::new();
        if let Some((n, _)) = self.set {
            let _ = writeln!(s, "mt1 SET   {n} strings, 1..{n} all present");
        }
        let _ = writeln!(s, "TX        {}", self.txid);

        let _ = writeln!(s, "OUT       {} output(s)", self.outputs.len());
        for (addr, sats) in &self.outputs {
            let _ = writeln!(s, "            {addr}   {}", btc(*sats));
        }

        match self.fee {
            Some((sats, p)) if p.is_verified() => {
                let _ = writeln!(s, "FEE       {}", btc(sats));
            }
            Some((sats, _)) => {
                let _ = writeln!(
                    s,
                    "FEE       {}   (CLAIMED — no input value verified)",
                    btc(sats)
                );
            }
            None => {
                let _ = writeln!(
                    s,
                    "FEE       UNKNOWN   (needs input values, which the transaction\n          \
                     does not carry)"
                );
            }
        }

        // §8.4's spellings only. NEVER a verdict like "PASSED": mt cannot make a
        // claim about spendability, because a BIP-68 relative timelock lives in
        // OP_CSV inside the witness script and reading it means evaluating the
        // sending wallet's script.
        let (lt, height) = self.locktime;
        let _ = match (lt, height) {
            (0, _) => writeln!(s, "LOCKTIME  NO TIMELOCK"),
            (n, Some(h)) => writeln!(s, "LOCKTIME  block {n}, current height {h}"),
            (n, None) => writeln!(s, "LOCKTIME  block {n}, current height UNKNOWN"),
        };

        let _ = writeln!(s, "INPUTS    {} input(s)", self.inputs.len());
        for (op, val, prov) in &self.inputs {
            let v = match val {
                Some(sats) => format!("{}   {}", btc(*sats), prov.label()),
                None => "UNKNOWN".to_string(),
            };
            let short: String = op.chars().take(16).collect();
            let _ = writeln!(s, "            {short}…   {v}");
        }

        let _ = writeln!(s, "STATUS    {}", self.status.render());
        s
    }
}

fn btc(sats: u64) -> String {
    format!("{}.{:08} BTC", sats / 100_000_000, sats % 100_000_000)
}

/// §6a's no-node warning, in its **recovery-time** wording.
///
/// The encode-time version closes *"consider re-running with a node before
/// cutting"*, which is useless to a recoverer: the engraving already exists, so
/// that names a decision made years ago. Their decision is **broadcast or
/// don't** — irreversible in the other direction.
pub fn no_node_warning(locktime: u32, txid: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "WARNING: no bitcoind reachable. mt read this transaction from the"
    );
    let _ = writeln!(
        s,
        "         strings, but could confirm NOTHING about it against the chain:"
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "           - do these inputs still exist, or were they spent?  UNKNOWN"
    );
    let _ = writeln!(
        s,
        "           - was this transaction already broadcast?           UNKNOWN"
    );
    let _ = writeln!(
        s,
        "           - has the locktime passed?                          UNKNOWN"
    );
    let _ = writeln!(
        s,
        "             locked to block {locktime}, current height unknown"
    );
    let _ = writeln!(
        s,
        "           - what fee does it pay?                             UNKNOWN"
    );
    let _ = writeln!(
        s,
        "             (the fee needs input values, which are not in the tx)"
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "         Everything above this line was read from the engraving itself"
    );
    let _ = writeln!(
        s,
        "         and is what the transaction SAYS. None of it is confirmed."
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "         TO RESOLVE ALL FOUR AT ONCE, either:");
    let _ = writeln!(
        s,
        "           - run mt inspect again with a bitcoind reachable, or"
    );
    let _ = writeln!(s, "           - look this txid up in any block explorer:");
    let _ = writeln!(s, "               {txid}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locktime_never_renders_a_verdict() {
        for (lt, h) in [
            (900_000u32, Some(963_663u64)),
            (900_000, None),
            (0, Some(1)),
        ] {
            let r = Report {
                txid: "a".into(),
                set: None,
                outputs: vec![],
                fee: None,
                locktime: (lt, h),
                inputs: vec![],
                status: Status::Unknown,
            };
            let out = r.render();
            assert!(
                !out.contains("PASSED"),
                "the report rendered a verdict §8.4 forbids"
            );
            assert!(
                !out.contains("SPENDABLE"),
                "the report claimed spendability"
            );
        }
    }

    /// Rule 1: never omitted, always `UNKNOWN`. Omission and ignorance look
    /// identical on a terminal.
    #[test]
    fn no_row_is_omitted_when_unanswerable() {
        let r = Report {
            txid: "abc".into(),
            set: None,
            outputs: vec![],
            fee: None,
            locktime: (900_000, None),
            inputs: vec![("x:0".into(), None, Provenance::Unknown)],
            status: Status::Unknown,
        };
        let out = r.render();
        for row in ["TX ", "OUT ", "FEE ", "LOCKTIME ", "INPUTS ", "STATUS "] {
            assert!(out.contains(row), "row {row:?} was omitted");
        }
        assert_eq!(
            out.matches("UNKNOWN").count(),
            4,
            "unanswerable rows must say UNKNOWN"
        );
    }

    /// LIVE means "unspent in the UTXO set", not "unspent" — `include_mempool`
    /// is false, so a conflicting unconfirmed spend still reads as present.
    #[test]
    fn live_is_qualified() {
        assert!(Status::Live.render().contains("in the UTXO set"));
        assert!(Status::Live.render().contains("mempool not consulted"));
    }

    /// The success case must not read as the theft case.
    #[test]
    fn already_confirmed_is_distinct_from_dead() {
        assert!(
            Status::AlreadyConfirmed
                .render()
                .contains("ALREADY CONFIRMED")
        );
        assert!(!Status::AlreadyConfirmed.render().contains("someone else"));
        assert!(Status::Dead.render().contains("someone else"));
    }

    #[test]
    fn no_node_warning_names_both_ways_out() {
        let w = no_node_warning(900_000, "deadbeef");
        assert!(w.contains("run mt inspect again with a bitcoind"));
        assert!(w.contains("block explorer"));
        assert!(
            w.contains("deadbeef"),
            "the txid must be printed for lookup"
        );
        assert!(w.contains("None of it is confirmed"));
    }
}
