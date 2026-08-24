//! §8's refusals, and the two warnings that are deliberately not refusals.
//!
//! **`mt` builds nothing, so everything it can get wrong is a failure to inspect
//! what it was handed.** Every check here runs before a single plate is cut, and
//! every refusal names the number that caused it — §8's closing line, and what
//! P5's tests assert against.
//!
//! **What is a refusal and what is a warning is a ruling, not a judgement call.**
//! §8.2c's legacy-unbound notice, §8.2g's file-mode notice and the whole of §8.4
//! are warnings *by name in the spec* — §8.4 says **"never refuse"** — and a
//! warning printed in the shape of a refusal teaches operators to skim both.

use crate::refusal::{Refusal, Warning};
use bitcoin::{Transaction, TxOut, Txid};

/// The absurd-fee ceiling, from `rust-bitcoin`'s own `DEFAULT_MAX_FEE_RATE`.
///
/// §8.2b adopts it rather than inventing a number: it is the ceiling the crate
/// this tool already depends on raises `AbsurdFeeRate` at.
pub const MAX_FEE_RATE_SAT_VB: u64 = 25_000;

/// Below this, §8.2b **warns and does not refuse.** A refusal floor would
/// hardcode today's relay policy into an artifact meant to be broadcast in 2040
/// — the same mistake as engraving a dollar figure.
pub const LOW_FEE_RATE_SAT_VB: u64 = 10;

/// §8.7b's ceiling: `mt1`'s 15-bit count field.
///
/// **Re-exported, never restated.** A second literal here would be free to
/// drift from the header layout it is derived from, and the one place that
/// would show is a refusal message quoting a ceiling the codec does not use.
pub use mt_codec::consts::MAX_CHUNKS;

// ── §8.1 / §8.3 — finalization, by each payload's own vocabulary ─────────────

/// §8.1, PSBT vocabulary: **every input carries a populated
/// `PSBT_IN_FINAL_SCRIPTSIG` or `PSBT_IN_FINAL_SCRIPTWITNESS`.**
///
/// Mandatory and not overridable. Neither format makes an unfinalized
/// transaction unrepresentable, so nothing upstream of `mt` refuses it for us.
pub fn finalized_guard_psbt(psbt: &bitcoin::Psbt) -> Result<(), Refusal> {
    let bad: Vec<usize> = psbt
        .inputs
        .iter()
        .enumerate()
        .filter(|(_, i)| {
            let sig = i.final_script_sig.as_ref().is_some_and(|s| !s.is_empty());
            let wit = i
                .final_script_witness
                .as_ref()
                .is_some_and(|w| !w.is_empty());
            !(sig || wit)
        })
        .map(|(n, _)| n)
        .collect();
    if bad.is_empty() {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§8.1",
        format!(
            "{} of {} inputs are not finalized (input {})",
            bad.len(),
            psbt.inputs.len(),
            bad.iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "A finalized PSBT input carries a populated PSBT_IN_FINAL_SCRIPTSIG or \
         PSBT_IN_FINAL_SCRIPTWITNESS. These inputs carry neither, so this \
         transaction cannot be broadcast — and a backup of something that cannot \
         be broadcast is not a backup. The PSBT format does not make this state \
         unrepresentable, so mt has to check it.",
    )
    .with_remedy("Run `bitcoin-cli finalizepsbt <psbt>` and pass the result."))
}

/// §8.3, raw-transaction vocabulary: **every input carries a non-empty
/// `scriptSig` or a non-empty witness.**
///
/// The disjunction is deliberate — §10.16 accepts legacy inputs, whose
/// satisfaction lives in the `scriptSig` and whose witness is empty.
pub fn finalized_guard_raw(tx: &Transaction) -> Result<(), Refusal> {
    let bad: Vec<usize> = tx
        .input
        .iter()
        .enumerate()
        .filter(|(_, i)| i.script_sig.is_empty() && i.witness.is_empty())
        .map(|(n, _)| n)
        .collect();
    if bad.is_empty() {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§8.3",
        format!(
            "{} of {} inputs carry no signature (input {})",
            bad.len(),
            tx.input.len(),
            bad.iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "Each of these inputs has an empty scriptSig AND an empty witness, so \
         nothing satisfies it. An unsigned transaction cannot be broadcast, so \
         engraving it produces a plate that is not a backup of anything.",
    )
    .with_remedy("Sign it first — `walletprocesspsbt`, then `finalizepsbt`."))
}

// ── §8.2b — value-blind acceptance ───────────────────────────────────────────

/// What §8.2b needs and `verify_transaction` never provided.
///
/// `rust-bitcoin`'s `verify_transaction` is a per-input **script** loop: it
/// iterates `tx.input` calling `verify_script_with_flags` and never compares
/// input value against output value. Outputs exceeding inputs, duplicate inputs
/// and an empty `vin` all pass every other refusal in §8.
pub fn value_guard(tx: &Transaction, input_values: &[Option<u64>]) -> Result<(), Refusal> {
    // `vin` non-empty.
    if tx.input.is_empty() {
        return Err(Refusal::new(
            "encode",
            "§8.2b",
            "the transaction has 0 inputs",
            "A transaction with an empty `vin` spends nothing. It is not \
             broadcastable and cannot be a backup of anything.",
        ));
    }

    // No duplicate outpoints. Two inputs naming one outpoint can never both be
    // spent, so the transaction is dead on arrival however well-formed it looks.
    let mut seen = std::collections::BTreeSet::new();
    for (n, i) in tx.input.iter().enumerate() {
        if !seen.insert(i.previous_output) {
            return Err(Refusal::new(
                "encode",
                "§8.2b",
                format!("input {n} repeats outpoint {}", i.previous_output),
                "Two inputs name the same previous output. Only one of them can \
                 ever be spent, so this transaction can never be accepted by any \
                 node — no matter how well-formed the rest of it is.",
            ));
        }
    }

    let out_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();

    // Every input's value must be known before the arithmetic means anything.
    // Where it is not, §8.2b's checks are SKIPPED rather than guessed — §8.2e
    // rules that a raw transaction with no node is warned about, never refused.
    let Some(in_total) = input_values.iter().copied().sum::<Option<u64>>() else {
        return Ok(());
    };

    if in_total < out_total {
        return Err(Refusal::new(
            "encode",
            "§8.2b",
            format!(
                "outputs total {} sat but inputs total only {in_total} sat",
                out_total
            ),
            "This transaction spends more than it takes in, so it is invalid by \
             consensus and no node will relay it. mt checks this because \
             rust-bitcoin's verify_transaction does not: it is a per-input SCRIPT \
             loop and never compares the two totals.",
        ));
    }

    let fee = in_total - out_total;
    let vb = vsize(tx);
    let rate = fee / vb.max(1) as u64;
    if rate > MAX_FEE_RATE_SAT_VB {
        return Err(Refusal::new(
            "encode",
            "§8.2b",
            format!(
                "fee rate {} sat/vB exceeds {}",
                thousands(rate),
                thousands(MAX_FEE_RATE_SAT_VB)
            ),
            format!(
                "Inputs total {in_total} sat and outputs total {out_total} sat, so \
                 this transaction pays {fee} sat in fees over {vb} vB. mt refuses \
                 above {} sat/vB — rust-bitcoin's own DEFAULT_MAX_FEE_RATE — \
                 because a fee that large is almost always a mistake in the INPUT \
                 VALUES, not an intention.",
                thousands(MAX_FEE_RATE_SAT_VB)
            ),
        )
        .with_remedy(
            "Check the values with --input-value <index>:<amount>, or re-run with a \
             node reachable so mt can fetch them.",
        ));
    }
    Ok(())
}

/// §8.2b's **warning**, not a refusal: there is no minimum fee.
///
/// The threshold is a heuristic and will age, which is fine here for a reason
/// worth stating: it is consumed **at encode time, by a human who is present**,
/// and is never engraved. A number that ages is only dangerous on steel.
pub fn low_fee_warning(tx: &Transaction, input_values: &[Option<u64>]) -> Option<Warning> {
    let in_total: u64 = input_values.iter().copied().sum::<Option<u64>>()?;
    let out_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
    let fee = in_total.checked_sub(out_total)?;
    let vb = vsize(tx).max(1) as u64;
    let rate = fee as f64 / vb as f64;
    if rate >= LOW_FEE_RATE_SAT_VB as f64 {
        return None;
    }
    Some(Warning::new(
        format!("fee rate is {rate:.1} sat/vB."),
        "This transaction may be engraved and then sit for years. A fee has to be \
         high enough to motivate a miner AT THE TIME IT IS BROADCAST, and nobody \
         knows what that will be. If it turns out too low, the holder may need \
         CPFP -- spending one of this transaction's outputs with a high-fee child, \
         which needs no key from the signer -- or out-of-band submission directly \
         to a miner, which bypasses relay policy entirely.",
    ))
}

// ── §8.2d — non_witness_utxo must hash to the input's txid ───────────────────

/// §8.2d: where a PSBT input carries `non_witness_utxo` — the **whole previous
/// transaction**, which BIP-174 requires for legacy inputs — hash it and require
/// the result to equal that input's `previous_output.txid`.
///
/// **This is a hash comparison, not script evaluation**, so it sits inside
/// §8.4's scope ruling: `mt` never executes a script and learns nothing about
/// the wallet's policy. Forging a passing value would need a txid collision.
///
/// It exists because §8.6 accepts legacy inputs on the grounds that
/// `non_witness_utxo` *binds the amount* — true of the mechanism, and false of
/// `mt` until something performed the check.
pub fn non_witness_utxo_guard(psbt: &bitcoin::Psbt) -> Result<(), Refusal> {
    for (n, inp) in psbt.inputs.iter().enumerate() {
        let Some(prev) = &inp.non_witness_utxo else {
            continue;
        };
        let want: Txid = psbt.unsigned_tx.input[n].previous_output.txid;
        let got = prev.compute_txid();
        if got != want {
            return Err(Refusal::new(
                "encode",
                "§8.2d",
                format!("input {n}'s non_witness_utxo hashes to {got}, not {want}"),
                "A PSBT input's non_witness_utxo is the WHOLE previous transaction, \
                 and hashing it must reproduce that input's previous_output txid. \
                 These differ, so the record does not describe the output being \
                 spent — and the value mt would read from it would be the wrong \
                 one. The fee absorbs any such error in full.",
            )
            .with_remedy("Re-export the PSBT from the wallet that built it."));
        }
    }
    Ok(())
}

/// Read an input's value from whichever UTXO record the PSBT carries.
///
/// **`non_witness_utxo` is preferred, and the order is the whole point.** Core
/// puts BOTH records on a segwit input — it has since the 2020 fee-attack
/// disclosure — and the two are not equally trustworthy: §8.2d has just hashed
/// the `non_witness_utxo` and matched it to this input's txid, while nothing
/// anywhere has checked the `witness_utxo`. Reading the weaker record while
/// labelling the row `TXID-BOUND` would put an unverified number under a
/// verified heading, which is R6 adversarial I-5 wearing a different hat.
///
/// `None` where neither is present — which §8.2c turns into a requirement to
/// supply the value, and §8.2c's warning into a live one.
pub fn psbt_input_value(psbt: &bitcoin::Psbt, n: usize) -> Option<(u64, ValueSource)> {
    let inp = psbt.inputs.get(n)?;
    let vout = psbt.unsigned_tx.input.get(n)?.previous_output.vout as usize;
    if let Some(prev) = &inp.non_witness_utxo {
        if let Some(o) = prev.output.get(vout) {
            return Some((o.value.to_sat(), ValueSource::TxidBound));
        }
        // The record is present and its hash matched, but it has no output at
        // this vout -- so it does NOT describe the outpoint being spent, and
        // falling through to witness_utxo here is fine as long as the LABEL
        // falls through with it. Returning the source alongside the number is
        // what makes that structural.
    }
    inp.witness_utxo
        .as_ref()
        .map(|w: &TxOut| (w.value.to_sat(), ValueSource::PsbtClaimed))
}

/// Which record a PSBT value came from — returned WITH the number, never
/// derived separately.
///
/// **The caller used to pick the label from `non_witness_utxo.is_some()` while
/// this function picked the number.** An adversarial review found the gap: §8.2d
/// hashes the record and matches the txid, but nothing checked that the record
/// has an output at the input's `vout`. A record that matches the txid and is
/// too short falls back to `witness_utxo` for the value — while the caller,
/// seeing `non_witness_utxo` present, labelled it `TXID-BOUND`. An **unverified**
/// number under a **verified** heading, which is the exact defect the three
/// provenance columns exist to prevent, reached through a different door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    /// Read from a `non_witness_utxo` whose hash reproduced the input's txid
    /// **and** which has an output at that vout.
    TxidBound,
    /// Read from a `witness_utxo`, which nothing has checked.
    PsbtClaimed,
}

// ── §8.2c — require values a PSBT lacks; warn on an UNBOUND legacy input ─────

/// §8.2c: where a UTXO record is absent **from a PSBT**, `mt` requires the
/// operator to supply that input's value, **per input**.
///
/// **The two words "from a PSBT" are load-bearing.** Read without them this
/// clause would refuse a raw transaction with no node, contradicting §8.2e's
/// *"mt never refuses the bytes"*.
pub fn require_psbt_input_values(
    psbt: &bitcoin::Psbt,
    supplied: &[(u32, u64)],
) -> Result<(), Refusal> {
    let missing: Vec<usize> = (0..psbt.inputs.len())
        .filter(|&n| {
            psbt_input_value(psbt, n).is_none() && !supplied.iter().any(|(i, _)| *i as usize == n)
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§8.2c",
        format!(
            "input {} carries no UTXO record and no supplied value",
            missing
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "A finalized PSBT normally carries every input's UTXO record, so mt \
         computes the fee itself and asks for nothing. This one does not, and \
         without the value §8.2b cannot check that inputs cover outputs or that \
         the fee is not absurd.",
    )
    .with_remedy("Supply it per input: --input-value 0:0.05000000"))
}

/// §8.2c's legacy warning, which **fires only when the value is UNBOUND**.
///
/// Not on every legacy input. The earlier rule fired *"whenever any input is
/// legacy"* while asserting `mt` could not bind the value by txid — which
/// §8.2d now does — so in the common case, a legacy input carrying
/// `non_witness_utxo`, it printed a false, capitalised block. **A warning that
/// cries wolf on the normal path has negative value**, because it trains the
/// operator to ignore the rare case where it is true.
pub fn legacy_unbound_warning(n: usize, claimed_sat: u64, out_total_sat: u64) -> Warning {
    let fee = claimed_sat.saturating_sub(out_total_sat);
    Warning::new(
        format!("input {n} is a legacy (pre-SegWit) input."),
        format!(
            "The fee you will pay is:   (what is REALLY at that input) - {}\n\
             You have told mt it holds:  {}\n\
             So mt shows a fee of:       {}\n\
             \n\
             NOTHING HAS VERIFIED THAT VALUE. This input carries no \
             non_witness_utxo, so mt could not bind it by txid (see 8.2d), and a \
             legacy signature does not commit to the amount either. A wrong value \
             still produces a perfectly valid transaction -- and the fee absorbs \
             the entire difference. If that input actually holds 10 BTC, this \
             transaction pays 9.01 BTC in fees and a miner will simply take it.\n\
             \n\
             Verify the input value out of band before you cut this plate.",
            btc(out_total_sat),
            btc(claimed_sat),
            btc(fee),
        ),
    )
}

// ── §8.2f — a bearer artifact passed as a command-line argument ──────────────

/// §8.2f: **refuse** a PSBT or transaction passed as a command-line argument,
/// and tell the operator how to clean up.
///
/// A finalized transaction is a **bearer** artifact — anyone holding it can
/// broadcast it, exactly like the plate it becomes. As an argument it lands in
/// the shell's history file in plaintext and in `ps` output for every user on
/// the machine.
///
/// **The siblings' precedent does not transfer, and the reason is the whole
/// point.** `md verify <STRINGS>…` takes its material positionally — but
/// `md1`/`mk1` strings are watch-only public material, where a leak costs
/// privacy. A finalized transaction is bearer, where it costs the money.
pub fn command_line_guard(args: &[String]) -> Result<(), Refusal> {
    // The verb, read from argv — this guard runs BEFORE clap, so nothing has
    // parsed one yet, and a refusal that says `mt mt:` tells the operator less
    // than one that names what they typed.
    let verb = args
        .get(1)
        .filter(|a| matches!(a.as_str(), "encode" | "decode" | "verify" | "inspect"))
        .cloned()
        .unwrap_or_else(|| "encode".to_string());

    for a in args.iter().skip(1) {
        if !looks_like_a_transaction(a) {
            continue;
        }
        // NEVER echo the argument. Printing it back into the refusal would put
        // the bearer material in a SECOND place -- the same defect the refusal
        // exists to name.
        let what = if a.to_ascii_lowercase().starts_with("mt1") {
            "an mt1 set"
        } else {
            "a transaction"
        };
        return Err(Refusal::new(
            verb,
            "§8.2f",
            format!(
                "{what} was passed as a command-line argument ({} characters)",
                a.chars().count()
            ),
            "It is now in your shell history and was visible in `ps` while this \
             ran. A finalized transaction — and the mt1 strings it becomes — is \
             BEARER: anyone who reads it can broadcast it. mt reads from a FILE \
             or STDIN only, and does not print the argument back, which would \
             put it in a second place. (md and mk DO take their strings as \
             arguments; md1/mk1 are watch-only, so a leak there costs privacy \
             rather than the money.)",
        )
        .with_remedy(format!(
            "Remove it:  {}\nThen re-run:  mt <verb> < file",
            purge_command()
        )));
    }
    Ok(())
}

/// Recognise the shapes §8.2f is about, and nothing else.
///
/// Deliberately narrow: a false positive here refuses a legitimate flag value.
///
/// **`mt1` strings belong here, and the siblings are exactly why they are easy
/// to miss.** `md verify <STRINGS>…` and `mk verify [MK1_STRINGS]…` both take
/// their material positionally — but `md1`/`mk1` are watch-only public material,
/// where a leak costs privacy, while an `mt1` set is the engraved transaction
/// itself, where it costs the money. Same shape, different hazard class, and an
/// operator carrying the habit across reaches for it first.
fn looks_like_a_transaction(a: &str) -> bool {
    if a.starts_with("cHNidP8") {
        return true;
    }
    // An mt1 string: the bearer artifact in the form an operator has typed off
    // steel. The shortest real one measured is 83 characters.
    let lower = a.to_ascii_lowercase();
    if lower.starts_with("mt1") && lower.len() >= 40 {
        return true;
    }
    let body = a.strip_prefix("0x").unwrap_or(a);
    // A raw transaction is at least a version, a counted input, an output and a
    // locktime; nothing legitimate on mt's command line is 100+ hex characters.
    body.len() >= 100 && body.len() % 2 == 0 && body.chars().all(|c| c.is_ascii_hexdigit())
}

/// The purge command, **specific to the operator's shell**, detected from
/// `$SHELL`.
///
/// Two limits stated rather than papered over: it cannot know who read the
/// history before now, and it cannot reach backups.
fn purge_command() -> &'static str {
    match std::env::var("SHELL").unwrap_or_default() {
        s if s.ends_with("zsh") => "history -d $HISTCMD && fc -W        # zsh",
        s if s.ends_with("fish") => "history delete --contains <tx>      # fish",
        s if s.ends_with("bash") => "history -d $HISTCMD && history -w   # bash",
        _ => "clear your shell's history file by hand",
    }
}

// ── §8.2g — a source file readable by anyone but its owner ───────────────────

/// §8.2g: **warn loudly** — never refuse — when the source file's mode has any
/// group or other bit set.
///
/// It works in more cases than "a named file", which was worth checking: with
/// `mt encode < tx.psbt` an `fstat` on fd 0 still returns the underlying file's
/// mode, so the redirect form is checkable too. Piped input gives a FIFO and
/// typed input gives no file — in both, the permissions are **unknown** rather
/// than silently skipped.
#[cfg(unix)]
pub fn file_mode_warning(path: Option<&std::path::Path>) -> Option<Warning> {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::AsRawFd;

    let (md, label) = match path {
        Some(p) => (std::fs::metadata(p).ok()?, p.display().to_string()),
        None => {
            let f = std::io::stdin();
            let md = std::fs::File::from(
                // SAFETY: borrowed, not owned -- into_raw_fd would close stdin.
                unsafe { std::os::unix::io::BorrowedFd::borrow_raw(f.as_raw_fd()) }
                    .try_clone_to_owned()
                    .ok()?,
            )
            .metadata()
            .ok()?;
            (md, "the redirected input file".to_string())
        }
    };
    // A FIFO or a TTY is not a file whose mode means anything here.
    if !md.file_type().is_file() {
        return None;
    }
    let mode = md.mode() & 0o777;
    if mode & 0o077 == 0 {
        return None;
    }
    Some(Warning::new(
        format!("{label} is mode {mode:04o} — readable by other users on this machine."),
        "A finalized transaction is BEARER. Anyone who can read this file can \
         broadcast it. It is exactly as dangerous as the plate you are about to \
         cut.\n\nIt says nothing about who read the file BEFORE now, and nothing \
         about backups or directories it has passed through. It is the check that \
         is available, not a guarantee.",
    ))
}

// ── §8.5 / §6a — what a node says about the inputs ───────────────────────────

/// §8.5: `gettxout` returns `null` for an input **AND the parent is confirmed**
/// → refuse. The output was spent or never existed.
///
/// **The `and the parent is confirmed` clause is not a refinement, it is the
/// difference between a true and a false refusal message.** Refusing on `null`
/// alone told an operator *"the output is spent or never existed"* — for a
/// parent sitting unconfirmed in the mempool that is a false statement of fact
/// inside a refusal, since `include_mempool` is `false` by ruling and `null` is
/// the *expected* answer there. **A mempool-only parent is a WARNING.**
pub fn spent_input_guard(n: usize, outpoint: &str, parent_confirmed: bool) -> Result<(), Refusal> {
    if !parent_confirmed {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§8.5",
        format!("input {n} ({outpoint}) is not in the UTXO set"),
        "gettxout returned null for this outpoint AND its parent transaction is \
         confirmed, so the output was spent or never existed. This transaction \
         can never be broadcast — engraving it would produce a plate that looks \
         like a backup and is not one.",
    )
    .with_remedy("Build a new transaction from outputs that are still unspent."))
}

/// §6a: compare the value the chain returns against the PSBT's own UTXO record
/// and **refuse on mismatch, naming both numbers.**
///
/// **This is normative and sits outside §8's numbering** — which is exactly why
/// P5's exhaustiveness gate reads a committed list rather than parsing §8. A
/// script over §8's item numbers is structurally unable to see this one.
///
/// Since §8.2's removal, the chain's own answer is the only value check `mt` has
/// for a segwit input, and an earlier draft acted only on whether the result was
/// `null` — throwing it away. This is a comparison of two integers, not script
/// evaluation, so it sits inside §8.4's scope ruling.
pub fn value_mismatch_guard(n: usize, claimed_sat: u64, chain_sat: u64) -> Result<(), Refusal> {
    if claimed_sat == chain_sat {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§6a",
        format!(
            "input {n}: the record says {} but the chain says {}",
            btc(claimed_sat),
            btc(chain_sat)
        ),
        "mt fetched this outpoint with gettxout and the value it returned does \
         not match the UTXO record carried alongside it. One of the two is wrong, \
         and mt cannot tell which. Every fee figure derived from the wrong one is \
         wrong by the difference — and the fee absorbs it in full.",
    )
    .with_remedy("Re-export the PSBT from the wallet that built it, and compare."))
}

// ── §8.6 — the satisfaction must bind the outputs ────────────────────────────

/// What a witness element looks like, structurally.
///
/// **Limited by §8.2's removal, and the spec does not claim otherwise.** Without
/// a script engine `mt` can tell that an element is *shaped* like a signature —
/// a 64-byte Schnorr element, or a DER-encoded ECDSA one with a trailing sighash
/// byte — but not that the script it satisfies actually **requires** one. A
/// crafted witness carrying a signature-shaped element the script never checks
/// would pass. This is a structural heuristic, not a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigShape {
    /// 64 bytes: a BIP-340 Schnorr signature with implicit `SIGHASH_DEFAULT`.
    SchnorrDefault,
    /// 65 bytes: Schnorr plus an explicit sighash byte.
    SchnorrExplicit(u8),
    /// DER-encoded ECDSA plus a trailing sighash byte.
    Ecdsa(u8),
}

impl SigShape {
    /// The sighash byte, or `None` for taproot's implicit `SIGHASH_DEFAULT`.
    fn sighash(self) -> Option<u8> {
        match self {
            SigShape::SchnorrDefault => None,
            SigShape::SchnorrExplicit(b) | SigShape::Ecdsa(b) => Some(b),
        }
    }
}

/// Recognise a signature-shaped element.
pub fn sig_shape(e: &[u8]) -> Option<SigShape> {
    match e.len() {
        64 => Some(SigShape::SchnorrDefault),
        65 if e[0] == 0x30 => Some(SigShape::Ecdsa(e[64])),
        65 => Some(SigShape::SchnorrExplicit(e[64])),
        // DER: 0x30 <len> 0x02 <r> 0x02 <s>, then one sighash byte.
        n if (9..=73).contains(&n) && e[0] == 0x30 && e[1] as usize == n - 3 => {
            Some(SigShape::Ecdsa(e[n - 1]))
        }
        _ => None,
    }
}

/// The elements of an input's satisfaction that can be signatures.
///
/// **`scriptSig` AND witness alike.** An earlier version named only the witness,
/// written when legacy inputs were refused; §10.16 now accepts them, and a
/// legacy input's signature lives in the `scriptSig`. A `SIGHASH_NONE` legacy
/// input would otherwise pass with its outputs unbound.
///
/// **A taproot script-path witness is recognised BY SHAPE**, because the
/// structural recognizer is ambiguous and this repo's own fixture proves it: a
/// Schnorr signature with an explicit sighash byte is 65 bytes, and a BIP-341
/// control block is `33 + 32m`, so at `m = 1` it is **also 65 bytes**. A
/// keyless leaf spent at depth 1 yields `[preimage, script, control-block(65)]`
/// — and a length-based recognizer counts the control block as the signature it
/// is looking for. So: last element is the control block, second-last the leaf
/// script, and signatures are counted only among the rest.
pub fn satisfaction_elements(input: &bitcoin::TxIn) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();

    if !input.witness.is_empty() {
        let mut w: Vec<Vec<u8>> = input.witness.iter().map(<[u8]>::to_vec).collect();
        // Strip an annex first (BIP-341: present iff >=2 elements and the last
        // begins 0x50), then the control block and leaf script.
        if w.len() >= 2 && w.last().is_some_and(|e| e.first() == Some(&0x50)) {
            w.pop();
        }
        if is_taproot_script_path(&w) {
            w.pop(); // control block
            w.pop(); // leaf script
        }
        v.extend(w);
    }

    // A legacy or wrapped-segwit scriptSig: a sequence of pushes.
    for push in script_sig_pushes(input.script_sig.as_bytes()) {
        v.push(push);
    }
    v
}

/// A taproot script-path spend, by shape: at least three elements, the last a
/// control block of `33 + 32m` bytes with a valid leaf version in its first byte.
fn is_taproot_script_path(w: &[Vec<u8>]) -> bool {
    let Some(last) = w.last() else { return false };
    w.len() >= 3 && last.len() >= 33 && (last.len() - 33) % 32 == 0 && (last[0] & 0xfe) == 0xc0
}

/// Pull the pushed data out of a `scriptSig`. Only pushes — a `scriptSig` in a
/// standard transaction contains nothing else.
fn script_sig_pushes(mut b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(&op) = b.first() {
        let (skip, len) = match op {
            0x01..=0x4b => (1usize, op as usize),
            0x4c if b.len() >= 2 => (2, b[1] as usize),
            0x4d if b.len() >= 3 => (3, u16::from_le_bytes([b[1], b[2]]) as usize),
            _ => return out,
        };
        if b.len() < skip + len {
            return out;
        }
        out.push(b[skip..skip + len].to_vec());
        b = &b[skip + len..];
    }
    out
}

/// §8.6: **every input's satisfaction must bind the outputs.** Two cases, and
/// the previous draft caught only the first.
///
/// a. **A signature with a non-`ALL` sighash.** A `SIGHASH_NONE` input leaves
///    the outputs unbound, so a holder — or anyone who photographs the plate —
///    can redirect the funds while the signature stays valid, and the legend's
///    `TO` line becomes false.
///
/// b. **NO signature at all.** The previous rule was written over *signatures*
///    and silently assumed every input has one. **A miniscript satisfaction need
///    not** — this project's own RCW fixture had a tier that was `after(N) AND
///    sha256(H)`, a timelock and a hash preimage with no key, and stock
///    rust-miniscript accepted it. An input satisfied by preimage alone commits
///    to **nothing**: any holder can rewrite every output and re-satisfy it.
///    That is strictly worse than (a), which at least binds the inputs.
pub fn satisfaction_guard(tx: &Transaction) -> Result<(), Refusal> {
    has_signature_guard(tx)?;
    sighash_all_guard(tx)?;
    Ok(())
}

/// §8.6 **(b)**: every input must carry at least one signature.
///
/// **Split from (a) so each is mutable by name.** `tests/refusals.toml` gives
/// one entry per refusal and `scripts/mutate-refusals.sh` neuters the named
/// check — two refusals sharing one function would make one of the two
/// mutations untargeted, and an untargeted control is the vacuous kind this
/// project has already paid for twice.
pub fn has_signature_guard(tx: &Transaction) -> Result<(), Refusal> {
    for (n, inp) in tx.input.iter().enumerate() {
        let elements = satisfaction_elements(inp);
        if elements.iter().any(|e| sig_shape(e).is_some()) {
            continue;
        }
        return Err(Refusal::new(
            "encode",
            "§8.6",
            format!("input {n}'s satisfaction carries no signature"),
            "Nothing in this input's witness or scriptSig is shaped like a \
             signature, so its satisfaction commits to NOTHING — any holder \
             can rewrite every output and satisfy it again. A miniscript \
             branch of a timelock and a hash preimage does exactly this, and \
             it is strictly worse than a SIGHASH_NONE input, which at least \
             binds the inputs.",
        ));
    }
    Ok(())
}

/// §8.6 **(a)**: every signature must be `SIGHASH_ALL`, or taproot's
/// `SIGHASH_DEFAULT`.
///
/// A `SIGHASH_NONE` input leaves the outputs unbound, so a holder — or anyone
/// who photographs the plate — can redirect the funds while the signature stays
/// valid, and the legend's `TO` line becomes false.
pub fn sighash_all_guard(tx: &Transaction) -> Result<(), Refusal> {
    for (n, inp) in tx.input.iter().enumerate() {
        for s in satisfaction_elements(inp)
            .iter()
            .filter_map(|e| sig_shape(e))
        {
            let Some(byte) = s.sighash() else { continue }; // SIGHASH_DEFAULT
            if byte != 0x01 {
                return Err(Refusal::new(
                    "encode",
                    "§8.6",
                    format!("input {n} is signed with sighash 0x{byte:02x}, not SIGHASH_ALL"),
                    format!(
                        "{} leaves this transaction's outputs UNBOUND. A holder — \
                         or anyone who photographs the plate — can redirect the \
                         funds while the signature stays valid, and the legend's TO \
                         line becomes false. Accepted: SIGHASH_ALL, and taproot's \
                         SIGHASH_DEFAULT.",
                        sighash_name(byte)
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Name a sighash byte, so the refusal says what it saw.
fn sighash_name(b: u8) -> &'static str {
    match b {
        0x01 => "SIGHASH_ALL",
        0x02 => "SIGHASH_NONE",
        0x03 => "SIGHASH_SINGLE",
        0x81 => "SIGHASH_ALL|ANYONECANPAY",
        0x82 => "SIGHASH_NONE|ANYONECANPAY",
        0x83 => "SIGHASH_SINGLE|ANYONECANPAY",
        _ => "this sighash flag",
    }
}

// ── §8.7b — the 32,768-chunk ceiling ─────────────────────────────────────────

/// §8.7b: over the ceiling → refuse, **naming the chunk count and the ceiling.**
///
/// **Deliberately unreachable for anything broadcastable.** 32,768 chunks is
/// 1,310,720 bytes, and Bitcoin's own standardness limit is ~100,000 vbytes — so
/// a transaction large enough to trip this could not be relayed even if `mt`
/// engraved it. It exists for completeness. For scale, the largest artifact
/// measured in §3b is **89 chunks, 2.2% of the ceiling.**
pub fn chunk_ceiling_guard(chunks: usize) -> Result<(), Refusal> {
    if chunks <= MAX_CHUNKS {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§8.7b",
        format!(
            "{} chunks exceeds the ceiling of {}",
            thousands(chunks as u64),
            thousands(MAX_CHUNKS as u64)
        ),
        "mt1's header carries a 15-bit chunk count, so a set cannot exceed 32,768 \
         chunks. Both verbs share the limit, since both use the same header. A \
         transaction this large could not be relayed by any node even if mt \
         engraved it — Bitcoin's standardness limit is roughly 100,000 vbytes.",
    ))
}

// ── §8.9 — secrets ───────────────────────────────────────────────────────────

/// §8.9: **refuse secret material, as `me` already does for `ms1`.**
///
/// **This must run BEFORE §8.2e names what it saw.** §8.2e's step 4 refusal
/// prints the first 8 bytes of unrecognised input so an operator can tell what
/// `mt` thought it received — and for an `ms1` string those bytes are **secret
/// seed entropy**, echoed to `stderr` and into whatever captured it. `me` learnt
/// this: *"NEVER interpolate the raw input string here — a mangled-HRP `ms1`
/// would print its intact secret body to stderr."* Same defect, same fix, and
/// ordering is the whole of it.
pub fn secret_guard(raw: &[u8], verb: &str) -> Result<(), Refusal> {
    let head: String = raw
        .iter()
        .take(64)
        .map(|&b| (b as char).to_ascii_lowercase())
        .collect();
    let looks_secret = head.trim_start().starts_with("ms1")
        || head.contains("\nms1")
        || head.trim_start().starts_with("ms1");
    if !looks_secret {
        return Ok(());
    }
    // The refusal NEVER interpolates the input. Not a prefix, not a length in
    // characters, not "what was seen" -- an ms1 body is secret seed entropy and
    // this message is the second place it would land.
    Err(Refusal::new(
        verb,
        "§8.9",
        "the input is ms1 — SECRET seed entropy",
        "ms1 is a codex32 secret share: whoever holds enough of them holds the \
         wallet. mt engraves TRANSACTIONS, which are bearer but spend only what \
         they already commit to; a seed is bearer over everything, forever. mt \
         will not read it, will not echo it, and has not printed any part of it \
         above.",
    )
    .with_remedy(
        "If you meant to engrave a seed, that is `me` with a SeedHammer, entered by \
         hand on the device — never through a tool that could log it.",
    ))
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// Virtual size, for the fee rate. Weight units, rounded up to vbytes.
fn vsize(tx: &Transaction) -> usize {
    tx.weight().to_vbytes_ceil() as usize
}

/// Satoshis as BTC, in the 8-decimal form every refusal message uses.
fn btc(sat: u64) -> String {
    format!("{}.{:08} BTC", sat / 100_000_000, sat % 100_000_000)
}

/// Thousands separators, so a big number in a verdict line is readable.
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
