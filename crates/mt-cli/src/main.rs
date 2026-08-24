//! The `mt` CLI.
//!
//! **stdout carries the artifact; stderr carries everything a human must see.**
//! That split is a hard interface boundary, not a formatting preference: the
//! output of `mt encode` exists to be piped, and the moment a legend line, a
//! banner or a blank separator shares that stream, every downstream consumer has
//! to parse `mt`'s prose out of its own input — and the first one that forgets
//! engraves a warning label as though it were a chunk.

// `Refusal` carries four strings and two optional ones — about 144 bytes — so
// clippy flags every `Result<_, Refusal>` as having a large Err variant. The
// lint is about hot paths, where a fat Err inflates every Result threaded
// through a call chain. Nothing here is a hot path: `mt` constructs at most ONE
// refusal per run and returns immediately. Boxing would put an allocation and a
// deref between the refusal and the operator to buy nothing, and every
// `Err(Refusal::new(...))` in the crate would grow a `Box::new`. Recorded as a
// judgement rather than suppressed silently.
#![allow(clippy::result_large_err)]

mod blocks;
mod input;
mod locktime;
mod node;
mod read_strings;
mod refusal;
mod report;
mod validate;

use clap::{Parser, Subcommand};
use std::io::{Read, Write};

use mt_codec::string_layer::pipeline;
use refusal::Refusal;

/// Engravable codex32 backups of signed Bitcoin transactions.
#[derive(Parser)]
#[command(name = "mt", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Turn a signed transaction into `mt1` strings for hand engraving.
    Encode(EncodeArgs),
    /// Read `mt1` strings back and emit BROADCASTABLE HEX on stdout.
    Decode(ReadArgs),
    /// Check a set of `mt1` strings — structurally, and never asking a node.
    Verify(ReadArgs),
    /// Report what is IN a set, consulting a node automatically when one is there.
    Inspect(ReadArgs),
}

/// Arguments shared by the two reading verbs.
#[derive(clap::Args)]
struct ReadArgs {
    /// Read the strings from a file. Defaults to stdin.
    #[arg(long, value_name = "PATH")]
    r#in: Option<std::path::PathBuf>,

    /// Compare against a transaction, by FULL txid. Takes a **PATH**.
    ///
    /// `verify` only. Comparing against the 20-bit set id would report a match
    /// for any transaction sharing those bits — 1 in 1,048,576 by accident, and
    /// under a second to construct deliberately.
    ///
    /// **The value name used to read `PSBT|HEX`, and mt's own help therefore
    /// invited the §8.2f leak it refuses.** An operator following it pastes the
    /// transaction, which lands in shell history and in `ps` — and then trips
    /// the refusal, having already leaked. It has always taken a path; nothing
    /// said so.
    #[arg(long, value_name = "PATH")]
    transaction: Option<std::path::PathBuf>,

    /// Suppress the report. Warnings and refusals are never suppressed.
    #[arg(long)]
    quiet: bool,

    /// Machine-readable report.
    #[arg(long)]
    json: bool,

    /// Path to `bitcoin-cli`. Pointing this at something absent forces the
    /// OFFLINE path — the mechanism every air-gapped gate and journey uses,
    /// because the alternative an implementer reaches for is editing `PATH`,
    /// which is process-global and would silently change neighbouring tests.
    #[arg(long, value_name = "PATH", default_value = "bitcoin-cli")]
    bitcoin_cli: std::path::PathBuf,
}

#[derive(clap::Args)]
struct EncodeArgs {
    /// Read the transaction from a file. Defaults to stdin.
    ///
    /// Never a command-line argument: §8.2f refuses that, because a finalized
    /// transaction is a bearer artifact and an argument lands in shell history
    /// and in `ps` output for every user on the machine.
    #[arg(long, value_name = "PATH")]
    r#in: Option<std::path::PathBuf>,

    /// Wallet id or fingerprint for the legend's `FROM` line.
    #[arg(long, value_name = "ID")]
    from: Option<String>,

    /// Wallet id or fingerprint for the legend's `TO` line.
    #[arg(long, value_name = "ID")]
    to: Option<String>,

    /// Free-text destination label.
    ///
    /// A separate flag **is** the ruling (§10.4): it makes the label an act of
    /// assertion by the operator rather than something that quietly appears.
    /// Nothing can check it against the transaction.
    #[arg(long, value_name = "TEXT")]
    to_label: Option<String>,

    /// An input's value, as `<index>:<amount>`. Repeatable, one per input.
    ///
    /// Per-input because a single total has two readings that differ by a whole
    /// input — deleted from the spec as a defect.
    #[arg(long, value_name = "INDEX:AMOUNT")]
    input_value: Vec<String>,

    /// Group the output every N characters, for hand engraving.
    ///
    /// Opt-in and never the default: grouping affects **stdout**, and the
    /// canonical artifact is ungrouped.
    #[arg(long, value_name = "N")]
    group_size: Option<usize>,

    /// Separator to use with `--group-size`. **Whitespace only.**
    ///
    /// `read_strings` strips whitespace and nothing else, so a non-whitespace
    /// separator produces an artifact `mt`'s own verbs refuse. The sequence that
    /// makes it expensive: choose `-`, engrave nine plates over several hours,
    /// type them back, and find that mt cannot read what mt produced — with the
    /// encode-time banner having said "verify the ENGRAVING, not this output".
    #[arg(long, value_name = "S", default_value = " ")]
    separator: String,

    /// Emit the set's invariant 8 characters on the first string only.
    ///
    /// The first string stays full, so the output is self-describing and
    /// `decode` needs no flag of its own.
    #[arg(long)]
    elide_prefix: bool,

    /// Suppress the inspection report. **Warnings and refusals are never
    /// suppressed**, on any verb.
    #[arg(long)]
    quiet: bool,

    /// Machine-readable report.
    #[arg(long)]
    json: bool,

    /// Path to `bitcoin-cli`, for a binary not on `PATH`.
    ///
    /// Pointing this at something that does not exist is how a run is forced
    /// **offline** — the mechanism every gate and journey that must run
    /// air-gapped uses.
    #[arg(long, value_name = "PATH", default_value = "bitcoin-cli")]
    bitcoin_cli: std::path::PathBuf,
}

fn main() -> std::process::ExitCode {
    // §8.2f RUNS BEFORE CLAP, and that ordering is the whole refusal.
    //
    // `mt encode <hex>` never reached this guard when it sat inside `encode`:
    // clap rejects the unexpected positional argument first, and **clap's error
    // message echoes the entire bearer transaction back to stderr**. So the
    // refusal written to stop a bearer artifact leaking into `ps` and shell
    // history leaked it itself, through the argument parser, with no refusal, no
    // purge command and no warning. The guard was correct about what it looked
    // at; it was simply downstream of something that had already printed.
    //
    // `mt verify mt1…` is the same hole reached by a likelier route: `md verify
    // <STRINGS>…` and `mk verify [MK1_STRINGS]…` both take their material
    // POSITIONALLY, so an operator carrying that habit across hits this on their
    // first try — and `mt1` strings, unlike `md1`/`mk1`, are bearer.
    let argv: Vec<String> = std::env::args().collect();
    if let Err(refusal) = validate::command_line_guard(&argv) {
        eprint!("{refusal}");
        return std::process::ExitCode::FAILURE;
    }

    let cli = Cli::parse();
    match cli.command {
        Command::Encode(args) => run(encode(args)),
        Command::Decode(args) => run(decode(args)),
        Command::Verify(args) => run(verify(args)),
        Command::Inspect(args) => run(inspect(args)),
    }
}

/// Exit 0 means every check passed; non-zero otherwise. `mt decode`'s
/// documented pipeline depends on it.
fn run(r: Result<(), Refusal>) -> std::process::ExitCode {
    match r {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(refusal) => {
            eprint!("{refusal}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn encode(args: EncodeArgs) -> Result<(), Refusal> {
    let mut stderr = std::io::stderr();
    json_unsupported_guard(args.json, "encode")?;

    let raw = match &args.r#in {
        Some(path) => std::fs::read(path).map_err(|e| {
            Refusal::new(
                "encode",
                "§8.2e",
                format!("cannot read {}", path.display()),
                format!("The file could not be opened: {e}."),
            )
        })?,
        None => {
            // The TTY welcome line. Without it, a new user's first action looks
            // like a hang — §10.10, and the operator's own confusion found it.
            if let Some(w) = blocks::welcome_if_tty() {
                let _ = writeln!(stderr, "{w}");
            }
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf).map_err(|e| {
                Refusal::new(
                    "encode",
                    "§8.2e",
                    "cannot read stdin",
                    format!("Reading from standard input failed: {e}."),
                )
            })?;
            buf
        }
    };

    // §8.9 BEFORE §8.2e. Sniffing's step-4 refusal names what it saw — the first
    // eight bytes — so an operator can tell what mt thought it received. For an
    // `ms1` string those bytes are SECRET SEED ENTROPY, and the refusal would be
    // the second place they land. `me` learnt this the same way; the fix is
    // ordering, and nothing else.
    validate::secret_guard(&raw, "encode")?;

    // §8.2g — a WARNING, never a refusal.
    if let Some(w) = validate::file_mode_warning(args.r#in.as_deref()) {
        let _ = writeln!(stderr, "{w}");
    }

    separator_guard(&args.separator)?;

    txid_paste_guard(&raw, "encode")?;
    let sniffed = input::sniff(&raw)?;
    let asserted = parse_input_values(&args.input_value)?;
    check_input_value_indices(&asserted, &args)?;

    // Both payloads reach the same checks, each by ITS OWN VOCABULARY (§8.1).
    let (tx, mut values, from_raw_hex) = match sniffed {
        input::Input::Psbt(bytes) => {
            let psbt = bitcoin::Psbt::deserialize(&bytes).map_err(|e| {
                Refusal::new(
                    "encode",
                    "§8.2e",
                    "input carries the PSBT magic but does not parse",
                    format!(
                        "The `psbt\\xff` magic is present, so this was meant to be a \
                         PSBT, but decoding it failed: {e}."
                    ),
                )
                .with_remedy("Re-export it from the wallet that built it.")
            })?;

            validate::finalized_guard_psbt(&psbt)?; // §8.1
            validate::non_witness_utxo_guard(&psbt)?; // §8.2d
            validate::require_psbt_input_values(&psbt, &asserted)?; // §8.2c

            // A record beats an assertion, and WHICH record decides the column
            // the number renders in: a non_witness_utxo has just been bound to
            // the input's txid by §8.2d, while a witness_utxo is the wallet's
            // word and nothing has agreed with it. Nothing here can be None —
            // §8.2c refused that case a line ago.
            let values: Vec<Option<(u64, report::Provenance)>> = (0..psbt.inputs.len())
                .map(|n| {
                    // The LABEL comes back WITH the number, from one place. It
                    // used to be derived here from `non_witness_utxo.is_some()`
                    // while the value came from `psbt_input_value` — and a
                    // record that matched the txid but had no output at the
                    // input's vout made the two disagree, putting an unverified
                    // number under a verified heading.
                    validate::psbt_input_value(&psbt, n)
                        .map(|(v, src)| {
                            (
                                v,
                                match src {
                                    validate::ValueSource::TxidBound => {
                                        report::Provenance::TxidBound
                                    }
                                    validate::ValueSource::PsbtClaimed => {
                                        report::Provenance::PsbtClaimed
                                    }
                                },
                            )
                        })
                        .or_else(|| {
                            asserted
                                .iter()
                                .find(|(i, _)| *i as usize == n)
                                .map(|(_, v)| (*v, report::Provenance::OperatorAsserted))
                        })
                })
                .collect();
            // A supplied value that CONTRADICTS the record was discarded without
            // a word. The record wins -- correctly, it is the stronger source --
            // but silence lets an operator believe mt used their number, and the
            // number they typed is the one they will check the fee against.
            for (i, supplied) in &asserted {
                let n = *i as usize;
                if let Some((record, src)) = validate::psbt_input_value(&psbt, n) {
                    if record != *supplied {
                        let _ = writeln!(
                            stderr,
                            "{}",
                            refusal::Warning::new(
                                format!(
                                    "--input-value {n}:{} disagrees with the PSBT, and mt used the PSBT.",
                                    fmt_btc(*supplied)
                                ),
                                format!(
                                    "The PSBT's own record says {}, and mt used that: {}\n\
                                     \n\
                                     mt is not ignoring you by accident. If the PSBT is \
                                     wrong, the transaction is signed over the wrong \
                                     value and re-exporting it is the fix — changing the \
                                     number on mt's command line would only change what \
                                     mt PRINTS, not what the signature commits to.",
                                    fmt_btc(record),
                                    match src {
                                        validate::ValueSource::TxidBound =>
                                            "it is bound to the input's txid by §8.2d, so \
                                             it is the stronger of the two.",
                                        validate::ValueSource::PsbtClaimed =>
                                            "it is a witness_utxo, which nothing has \
                                             checked — so BOTH numbers are claims, and mt \
                                             prefers the one the signer saw.",
                                    }
                                ),
                            )
                        );
                    }
                }
            }
            (psbt.extract_tx_unchecked_fee_rate(), values, false)
        }
        input::Input::RawHex(b) => {
            let tx = decode_tx(&b, "encode")?;
            validate::finalized_guard_raw(&tx)?; // §8.3
            let values: Vec<Option<(u64, report::Provenance)>> = (0..tx.input.len())
                .map(|n| {
                    asserted
                        .iter()
                        .find(|(i, _)| *i as usize == n)
                        .map(|(_, v)| (*v, report::Provenance::OperatorAsserted))
                })
                .collect();
            (tx, values, true)
        }
    };

    input_index_range_guard(&asserted, tx.input.len())?;

    // §8.6 binds both payloads: an input whose satisfaction does not bind the
    // outputs is redirectable by any holder, and the legend's TO line is a lie.
    validate::satisfaction_guard(&tx)?;

    // §6a: the node is consulted AUTOMATICALLY. It is where an unbound value
    // becomes a bound one, so it runs BEFORE §8.2b's arithmetic.
    let node = node::Node::find(&args.bitcoin_cli);
    let mut bound_by_chain = vec![false; tx.input.len()];

    // ASKED FIRST, exactly as `Report::build` asks it: has THIS transaction
    // already confirmed?
    //
    // **Every input of a confirmed transaction is spent — by itself — and every
    // parent is confirmed, which is bit-for-bit the §8.5 condition.** Without
    // this question the success case is reported as the theft case, and the
    // refusal tells an operator whose payment WENT THROUGH that it "can never be
    // broadcast" and to build a new transaction. An operator who follows that
    // pays twice.
    //
    // §6a's five states already ruled the ordering and `report.rs` already
    // implements it; this was the SECOND SITE asking the weaker question. Found
    // by running against a real node — no offline or stubbed test could see it,
    // because all three §8.5 cases share `gettxout -> null` and differ only in
    // `getrawtransaction` on a txid the stub was never asked about.
    let already_confirmed = node
        .as_ref()
        .is_some_and(|nd| nd.is_confirmed(&txid_of(&tx)) == node::ParentState::Confirmed);

    if let (Some(nd), false) = (&node, already_confirmed) {
        for (n, inp) in tx.input.iter().enumerate() {
            let op = inp.previous_output;
            match nd.txout(&op.txid.to_string(), op.vout) {
                node::Utxo::Unspent(chain_sat) => {
                    bound_by_chain[n] = true;
                    match values[n] {
                        // §6a: two integers, both claiming to be this input's
                        // value. mt cannot tell which is wrong, so it refuses.
                        Some((claimed, _)) => {
                            validate::value_mismatch_guard(n, claimed, chain_sat)?;
                        }
                        None => {}
                    }
                    // The chain's answer wins the LABEL as well as the number:
                    // it is the only one anything checked.
                    values[n] = Some((chain_sat, report::Provenance::ChainFetched));
                }
                node::Utxo::Null => {
                    // §8.5 needs BOTH facts. `include_mempool` is false by
                    // ruling, so null is the EXPECTED answer for an unconfirmed
                    // parent — refusing on it alone states something untrue
                    // inside a refusal.
                    let confirmed =
                        nd.is_confirmed(&op.txid.to_string()) == node::ParentState::Confirmed;
                    validate::spent_input_guard(n, &op.to_string(), confirmed)?;
                }
            }
        }
    }

    // §8.2b — the value checks rust-bitcoin's verify_transaction never made.
    let sats: Vec<Option<u64>> = values.iter().map(|v| v.map(|(s, _)| s)).collect();
    validate::value_guard(&tx, &sats)?;

    // §8.7b — before chunking, so the ceiling is named rather than a codec error.
    let tx_bytes = bitcoin::consensus::serialize(&tx);
    debug_assert_eq!(txid_of(&tx), tx.compute_txid().to_string());
    validate::chunk_ceiling_guard(
        tx_bytes
            .len()
            .div_ceil(mt_codec::consts::PAYLOAD_CEILING_BYTES),
    )?;

    let txid = tx.compute_txid().to_string();
    let strings = pipeline::encode(&tx_bytes, &txid).map_err(|e| {
        Refusal::new(
            "encode",
            "§3b",
            "transaction cannot be chunked",
            format!("{e}"),
        )
    })?;

    // §8.2c's legacy warning fires ONLY where the value is bound by NOTHING —
    // no non_witness_utxo, no chain fetch. The earlier rule fired on every
    // legacy input while asserting mt could not bind the value by txid, which
    // §8.2d now does, so on the common path it printed a false capitalised
    // block and trained the operator to skip the rare case where it is true.
    let out_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
    let in_total: Option<u64> = values.iter().map(|v| v.map(|(s, _)| s)).sum();
    let fee_sat = in_total.and_then(|i| i.checked_sub(out_total));
    for (n, inp) in tx.input.iter().enumerate() {
        // GATE ON THE PROVENANCE, not on `witness.is_empty()`. "Is this input
        // non-witness" is not the question §8.2c asks — the question is whether
        // ANYTHING has checked its value, and the code had already answered that
        // a hundred lines earlier and thrown the answer away.
        let unverified = values[n].is_none_or(|(_, p)| !p.is_verified());
        let legacy = inp.witness.is_empty();
        if legacy && unverified {
            if let Some((claimed, prov)) = values[n] {
                let _ = writeln!(
                    stderr,
                    "{}",
                    validate::legacy_unbound_warning(
                        n,
                        claimed,
                        prov == report::Provenance::OperatorAsserted,
                        out_total,
                        fee_sat,
                    )
                );
            }
        }
    }

    // §8.2b again, in its WARNING half: no minimum fee, but say the rate.
    if let Some(w) = validate::low_fee_warning(&tx, &sats) {
        let _ = writeln!(stderr, "{w}");
    }

    // §8.4's negative subtraction: the lock height passed before this build's
    // reference, so there is no future date to estimate and saying "spendable
    // now" beats printing a past year.
    let lock = locktime::read(&tx);
    if let Some(w) = lock.below_reference_warning() {
        let _ = writeln!(stderr, "{w}");
    }

    // §6a at ENCODE time. The recovery-time warning names a block explorer,
    // which is useless to someone standing at an uncut plate: their decision is
    // cut-now-or-check-first, and it is still open.
    if node.is_none() {
        let _ = writeln!(stderr, "{}", blocks::encode_no_node_warning());
    }

    if from_raw_hex {
        // §8.2e has TWO branches and only one was ever printed. The warning said
        // flatly that mt "cannot see any input's value and cannot check the fee"
        // -- in the same run where it had just fetched every value from the
        // chain and printed the fee. What degrades on a raw transaction is
        // narrow, and A NODE CLOSES MOST OF IT; saying otherwise while doing
        // otherwise trains the operator to skim the warning.
        // GATED ON WHERE THE VALUES CAME FROM, not on whether a fee could be
        // computed. `--input-value` also produces a fee, and this branch then
        // said "mt fetched each input's value, so the fee above is real" in the
        // same run that printed OPERATOR-ASSERTED twice.
        let all_from_chain = !values.is_empty()
            && values
                .iter()
                .all(|v| v.is_some_and(|(_, p)| p == report::Provenance::ChainFetched));
        let body = if all_from_chain {
            "A raw transaction carries its inputs' OUTPOINTS but not their VALUES, \
             so mt cannot compute the fee from it alone.\n\
             \n\
             mt fetched each input's value instead, so the fee above is real. What \
             is still missing is everything a PSBT carries ABOUT the signing: \
             derivation paths, and the wallet's own record of what it meant to \
             spend. (§8.2e)"
        } else {
            "A raw transaction carries its inputs' OUTPOINTS but not their VALUES, \
             so mt cannot compute the fee from it alone.\n\
             \n\
             THE FEE IS UNKNOWN. mt cannot tell you whether it is 0.0001 BTC or \
             9 BTC. Supply the values with --input-value <index>:<amount>, or \
             re-run with a node reachable so mt can fetch them. (§8.2e)"
        };
        let _ = writeln!(
            stderr,
            "{}",
            refusal::Warning::new("this is a RAW TRANSACTION, not a PSBT.", body)
        );
    }

    // stderr: everything the operator must see, before the artifact.
    let _ = writeln!(stderr, "{}", blocks::bearer_warning());
    let lengths: Vec<usize> = strings.iter().map(|s| s.chars().count()).collect();
    let _ = writeln!(stderr, "{}", blocks::correction_coverage(&lengths));
    let _ = writeln!(stderr, "{}", blocks::verify_the_steel());

    if !args.quiet {
        // encode CALLS the report; it does not compose its own. If it did, the
        // operator's pre-engraving view and the 2040 recoverer's view would be
        // two implementations of the same thing, free to disagree — and this
        // artifact has produced that defect twice already.
        let claimed: Vec<(u32, u64, report::Provenance)> = values
            .iter()
            .enumerate()
            .filter_map(|(n, v)| v.map(|(s, p)| (n as u32, s, p)))
            .collect();
        let r = report::Report::build(&tx, &txid, node.as_ref(), &claimed);
        let _ = write!(stderr, "{}", r.render());

        // ...and APPENDS its two rows below STATUS. Anything encode needed to
        // CHANGE about a row would be a defect in the row, fixable in one place.
        let prefix = pipeline::invariant_prefix(&strings[0]).unwrap_or_default();
        let total: usize = lengths.iter().sum();
        let _ = writeln!(
            stderr,
            "CUT       {} strings, {total} characters",
            strings.len()
        );
        let _ = writeln!(
            stderr,
            "PREFIX    all {} strings begin mt1{prefix} — strings sharing that",
            strings.len()
        );
        let _ = writeln!(stderr, "          prefix belong together");
        let _ = writeln!(stderr);

        // §0a / §5: the five suggested legend fields.
        let outs: Vec<u64> = tx.output.iter().map(|o| o.value.to_sat()).collect();
        let _ = write!(
            stderr,
            "{}",
            blocks::legend(
                &lock,
                args.from.as_deref(),
                args.to.as_deref(),
                args.to_label.as_deref(),
                &outs,
                strings.len(),
            )
        );
        let _ = writeln!(stderr);
    }

    // §8.2g's other half: mt warns about the INPUT file's permissions and then
    // writes the strings to an output file it never mentions again. Only when
    // stdout is REDIRECTED — on a terminal the strings scroll past and there is
    // no file to destroy.
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        let _ = writeln!(stderr, "{}", blocks::redirected_output_warning());
    }

    // stdout: the strings, lowercase, and nothing else.
    let out = std::io::stdout();
    let mut out = out.lock();
    let rendered = render(&strings, &args);
    for line in rendered {
        let _ = writeln!(out, "{line}");
    }
    Ok(())
}

/// Apply `--elide-prefix` and `--group-size` to what goes on stdout.
fn render(strings: &[String], args: &EncodeArgs) -> Vec<String> {
    let drop = "mt1".len() + mt_codec::consts::INVARIANT_PREFIX_SYMBOLS;
    strings
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let base = if args.elide_prefix && i > 0 {
                s[drop..].to_string()
            } else {
                s.clone()
            };
            match args.group_size {
                Some(n) if n > 0 => base
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(n)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(&args.separator),
                _ => base,
            }
        })
        .collect()
}

/// The txid in its display form, from the raw transaction bytes.
///
/// **Not** the double-SHA-256 of these bytes — that is the *wtxid* for any
/// segwit transaction. The txid hashes the same transaction with marker, flag
/// and witnesses stripped, which `bitcoin`'s `compute_txid` does.
fn txid_display(bytes: &[u8], verb: &str) -> Result<String, Refusal> {
    use bitcoin::consensus::Decodable;
    let tx = bitcoin::Transaction::consensus_decode(&mut &bytes[..]).map_err(|e| {
        Refusal::new(
            verb,
            "§8.2e",
            "input is not a decodable Bitcoin transaction",
            format!(
                "The bytes are valid hex but do not parse as a transaction: {e}. \
                 mt reads an ALREADY-SIGNED transaction; it does not build one."
            ),
        )
        .with_remedy(if verb == "encode" {
            "Check this is the output of `finalizepsbt`, not a template."
        } else {
            // On the RECOVERY path there is no PSBT to re-finalize: the operator
            // is holding steel. Encode-path advice sends them to look for a file
            // that has not existed for years.
            "Every checksum held, so this is not miscut steel — the strings are \
             more likely from two different engravings mixed together. Check that \
             every plate carries the same 8 characters after `mt1`."
        })
    })?;
    Ok(tx.compute_txid().to_string())
}

/// Read the strings an operator typed back, from a file or stdin.
fn read_input(path: &Option<std::path::PathBuf>, verb: &str) -> Result<String, Refusal> {
    let bytes = match path {
        Some(p) => std::fs::read(p).map_err(|e| {
            Refusal::new(
                verb,
                "§1.1e",
                format!("cannot read {}", p.display()),
                format!("The file could not be opened: {e}."),
            )
        })?,
        None => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .map_err(|e| Refusal::new(verb, "§1.1e", "cannot read stdin", format!("{e}")))?;
            buf
        }
    };
    // A pasted TXID reaches the reading verbs as easily as `encode` -- more
    // easily, since `inspect` is the verb a recoverer is pointed at and a txid
    // is what an explorer shows them.
    txid_paste_guard(&bytes, verb)?;

    // §8.9 binds the READING verbs too, and this is where it has to sit: an
    // operator who reaches for the wrong tool pastes ms1 into `mt decode`, and
    // every refusal below this point quotes what it saw.
    validate::secret_guard(&bytes, verb)?;

    String::from_utf8(bytes).map_err(|_| {
        Refusal::new(
            verb,
            "§1.1e",
            "input is not text",
            "mt1 strings are bech32 characters. This input is not valid UTF-8, so \
             it cannot be a set of strings typed back from steel.",
        )
    })
}

/// The margin report §1.1 requires: not just a verdict, but how much of the
/// correction budget each chunk spent — and WHERE.
///
/// A chunk repaired four times passes while sitting **one scratch from
/// unrecoverable**, with zero redundancy behind it. A verdict that hides that
/// tells the operator the opposite of what they need.
fn margin_report(chunks: &[mt_codec::DecodedChunk], out: &mut impl Write) {
    const T: usize = 4;
    let mut repaired: Vec<&mt_codec::DecodedChunk> =
        chunks.iter().filter(|c| c.corrected > 0).collect();
    if repaired.is_empty() {
        return;
    }
    // Descending: the nearest-to-limit chunk is the one to act on, and under a
    // failed re-derivation it is also the likeliest mis-correction.
    repaired.sort_by(|a, b| b.corrected.cmp(&a.corrected));

    let _ = writeln!(
        out,
        "\nCORRECTION APPLIED. {} chunk{} needed repair:",
        repaired.len(),
        if repaired.len() == 1 { "" } else { "s" }
    );
    for c in &repaired {
        // 1-based for humans; `index` is a wire field and appears in no message.
        let margin = if c.corrected >= T {
            "   <-- NO MARGIN LEFT"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  chunk {:>3}   {} of {T} symbols{margin}",
            c.header.index + 1,
            c.corrected
        );
        // WHAT was repaired away, not only WHERE. A bare position tells the
        // operator to go and look; a before-and-after tells them what to look
        // for — and it is the only way to distinguish a MIS-CUT plate from a
        // MIS-READ one. If the steel really says the corrected character, the
        // plate is fine and the typist slipped.
        for (n, &p) in c.corrected_positions.iter().enumerate() {
            let was = c.corrected_from.get(n).copied().map_or('?', symbol_char);
            let now = c.corrected_to.get(n).copied().map_or('?', symbol_char);
            // data-part offset -> 1-based whole-string position
            let _ = writeln!(
                out,
                "              pos {:>3}   read {was}, corrected to {now}",
                p + 1 + 3
            );
        }
    }
    if let Some(worst) = repaired.first() {
        if worst.corrected >= T {
            let _ = writeln!(
                out,
                "\n  Chunk {} is at its correction limit. One more damaged symbol in\n  \
                 that string and this transaction is unrecoverable. Re-cut it.",
                worst.header.index + 1
            );
        }
    }
    let _ = writeln!(out);
}

/// A bech32 symbol value as the character an operator sees on steel.
fn symbol_char(v: u8) -> char {
    const ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    ALPHABET.get(v as usize).map_or('?', |&b| b as char)
}

/// What a set carried besides its bytes: duplicates resolved, strings that
/// could not be read at all.
///
/// **Both are silent successes if nobody prints them**, and both name a piece of
/// steel the operator should act on. A duplicate that `mt` quietly resolved
/// still means one of the two plates is closer to unrecoverable than the other,
/// and an unreadable string means a plate is already scrap while the set as a
/// whole is fine.
/// What `mt` replaced before the codec ever saw the strings.
///
/// **Separate from the margin report, deliberately.** BCH repairs DAMAGE and
/// spends one of four repairs doing it; this replaces a character that CANNOT
/// OCCUR in a valid string, and costs nothing. Folding them together made the
/// margin report say `read 6` about a plate where the operator typed `b`.
fn transliteration_notices(read: &read_strings::ReadStrings, out: &mut impl Write) {
    let notes = &read.notes;
    if notes.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "\nCHARACTERS mt READ DIFFERENTLY. These are not in the mt1 alphabet at \
         all, so\nthey cannot be what was engraved — mt tried every alternative \
         and kept the one\nthat cost the checksum least:"
    );
    for n in notes {
        let _ = writeln!(
            out,
            "  pos {:>3}   you typed {}, mt read it as {}",
            n.position, n.from, n.to
        );
    }
    if read.free {
        let _ = writeln!(
            out,
            "\n  This cost NONE of the 4-symbol repair budget — the reading \
             checksums\n  exactly, so it is what was engraved."
        );
    } else {
        // The honest case, and the one worth saying out loud: neither reading
        // was right, so BCH had to repair mt's choice — and repairs spent here
        // are repairs the operator's own damage no longer has.
        let _ = writeln!(
            out,
            "\n  NEITHER reading checksummed on its own, so BCH had to repair the \
             one mt\n  chose — see the corrections below. That character was \
             probably DAMAGED\n  rather than misread, and the repair came out of \
             the 4-symbol budget."
        );
    }
    let _ = writeln!(out);
}

fn set_notices(set: &mt_codec::string_layer::pipeline::DecodedSet, out: &mut impl Write) {
    const T: usize = 4;
    for d in &set.duplicates {
        let _ = writeln!(
            out,
            "\nDUPLICATE RESOLVED. chunk {} was present twice.",
            d.index + 1
        );
        let _ = writeln!(
            out,
            "  KEPT       the copy needing {} of {T} corrections",
            d.kept_corrections
        );
        let _ = writeln!(
            out,
            "  DISCARDED  the copy needing {} of {T} corrections",
            d.discarded_corrections
        );
        if d.discarded_corrections == 0 {
            // A PRISTINE PLATE IS NOT A RE-CUT CANDIDATE. Advising 21 minutes of
            // engraving on a copy that spent NONE of its budget contradicts the
            // two lines directly above it — and mt does not know why the string
            // was typed twice. Usually it is a stack, not a defect.
            let _ = writeln!(
                out,
                "  Both copies are clean and carry the same payload, so nothing is\n  \
                 wrong with either plate. You most likely typed one twice."
            );
        } else {
            let left = T.saturating_sub(d.discarded_corrections);
            let _ = writeln!(
                out,
                "  Both carry the same payload, so nothing is ambiguous. mt kept the\n  \
                 healthier copy — but the discarded plate has {left} correction{}\n  \
                 left before it is unrecoverable, and it is the one to re-cut.",
                if left == 1 { "" } else { "s" }
            );
        }
    }
    for u in &set.unreadable {
        let _ = writeln!(
            out,
            "\nUNREADABLE STRING. string {} of the input could not be read:",
            u.input_position
        );
        let _ = writeln!(out, "  {}", u.reason);
        // WHAT mt KNOWS, AND NOTHING MORE. It could not read the string, so it
        // does not know which chunk it was, or whether it belonged to this set
        // at all -- it may be a plate from a DIFFERENT engraving that got typed
        // into the same pile, or a stray line. The previous wording told the
        // operator "that plate is scrap. Re-cut it from the strings mt has
        // verified", which DIRECTS A PHYSICAL ACTION ON STEEL mt never
        // identified: acting on it discards a plate that may be the only copy
        // of something else.
        for line in [
            "This set is complete WITHOUT it, so nothing here is missing. mt",
            "cannot tell you which chunk that string was, or whether it belongs",
            "to this set at all — it could not read it. Do not discard the plate",
            "on this message alone: check whether it is from another engraving",
            "first, and if it is one of THESE, re-cut it from the strings mt has",
            "just verified.",
        ] {
            let _ = writeln!(out, "  {line}");
        }
    }
    if !set.duplicates.is_empty() || !set.unreadable.is_empty() {
        let _ = writeln!(out);
    }
}

fn decode(args: ReadArgs) -> Result<(), Refusal> {
    let text = read_input(&args.r#in, "decode")?;
    let read = read_strings::read(&text, "decode")?;
    let strings = read.strings.clone();
    let set = pipeline::decode(&strings).map_err(|e| explain_failure(&strings, "decode", &e))?;

    // §1.1's LAST CHECK, before anything reaches stdout.
    content_id_guard(&set.bytes, &set.chunks, "decode")?;

    let mut stderr = std::io::stderr();
    if !args.quiet {
        // decode PRINTS §1.1's REPORT — the same one, not a summary of its own.
        // It is the verb a recoverer reaches for first (`inspect` is the one
        // designed for them, and they have no way to know that), so a decode
        // that stays quiet hands a stranger sixty kilobytes of hex — a bearer
        // instrument in the most broadcastable form there is — before anything
        // has told them the destination, the amount or the locktime. The next
        // command they plausibly type is `sendrawtransaction`.
        //
        // It printed TWO HAND-COMPOSED LINES until an independent spec-first
        // review compared §1.1a against the code: a txid and a chunk count, and
        // none of the rows that decide whether to broadcast. That is also a
        // second implementation of the `mt1 SET` row, in a different format —
        // the drift the single-owner rule exists to prevent.
        let tx = decode_tx(&set.bytes, "decode")?;
        let txid = tx.compute_txid().to_string();
        let node = node::Node::find(&args.bitcoin_cli);
        let mut r = report::Report::build(&tx, &txid, node.as_ref(), &[]);
        r.set = Some((set.chunks.len(), set.chunks.len()));
        r.set_prefix = pipeline::invariant_prefix(&strings[0]).ok();
        // `--json` is honoured HERE too. It was wired into `inspect` alone, so
        // on this verb it parsed and did nothing -- the precise defect
        // render_json's own doc comment condemns, one commit after I wrote the
        // condemnation.
        let _ = write!(
            stderr,
            "{}",
            if args.json {
                report::render_json(&r)
            } else {
                r.render()
            }
        );

        // §6a's no-node warning belongs here for the same reason the report
        // does: this reader is a recoverer, and four rows just said UNKNOWN.
        if node.is_none() {
            let _ = writeln!(stderr);
            let _ = write!(
                stderr,
                "{}",
                report::no_node_warning(
                    &locktime::read(&tx),
                    &txid,
                    tx.input.iter().any(|i| i.witness.is_empty()),
                )
            );
        }
        transliteration_notices(&read, &mut stderr);
        transliteration_notices(&read, &mut stderr);
        transliteration_notices(&read, &mut stderr);
        set_notices(&set, &mut stderr);
        margin_report(&set.chunks, &mut stderr);
    }
    let bytes = set.bytes;

    // stdout ONLY on success: every check above passed, so these bytes are
    // vouched for. A failure path that still printed hex would let the
    // documented pipeline broadcast a transaction that failed mt's own checks.
    let out = std::io::stdout();
    let mut out = out.lock();
    let mut hex = String::with_capacity(bytes.len() * 2);
    {
        use core::fmt::Write as _;
        for b in &bytes {
            let _ = write!(hex, "{b:02x}");
        }
    }
    let _ = writeln!(out, "{hex}");
    Ok(())
}

fn verify(args: ReadArgs) -> Result<(), Refusal> {
    json_unsupported_guard(args.json, "verify")?;
    let text = read_input(&args.r#in, "verify")?;
    let read = read_strings::read(&text, "verify")?;
    let strings = read.strings.clone();
    let set = pipeline::decode(&strings).map_err(|e| explain_failure(&strings, "verify", &e))?;

    // ...AND the reassembled transaction re-derives that id. This is the check
    // the OK line has always claimed; until it existed, the claim was a
    // sentence rather than a test.
    content_id_guard(&set.bytes, &set.chunks, "verify")?;

    let mut stderr = std::io::stderr();
    let set_id = set.chunks[0].header.chunk_set_id;
    let _ = writeln!(
        stderr,
        "mt verify: OK — {} chunks, set {set_id:#07x}, transaction re-derives.",
        set.chunks.len()
    );
    transliteration_notices(&read, &mut stderr);
    set_notices(&set, &mut stderr);
    margin_report(&set.chunks, &mut stderr);
    let bytes = &set.bytes;

    // §1.1: verify NEVER asks a node. A predicate whose answer changes between
    // runs is not a predicate, and keeping it offline is what lets it run on an
    // air-gapped machine.
    if let Some(path) = &args.transaction {
        let supplied = std::fs::read(path).map_err(|e| {
            Refusal::new(
                "verify",
                "§1.1",
                format!("cannot read {}", path.display()),
                format!("{e}"),
            )
        })?;
        // §1.1 rules `--transaction <psbt|hex>`, and a supplied PSBT is compared
        // against its EXTRACTED transaction (§10.13 c) -- so the flag accepted
        // half of its own ruling and refused the other half. The PSBT is the
        // form a wallet exports; the hex is what `finalizepsbt` returns. An
        // operator checking their steel against what they built has the PSBT.
        let supplied = match input::sniff(&supplied)? {
            input::Input::RawHex(b) => b,
            input::Input::Psbt(bytes) => {
                let psbt = bitcoin::Psbt::deserialize(&bytes).map_err(|e| {
                    Refusal::new(
                        "verify",
                        "§1.1",
                        "the supplied PSBT does not parse",
                        format!("{e}"),
                    )
                })?;
                // Finalized, by §8.1's vocabulary: an unfinalized PSBT extracts
                // to a transaction with empty witnesses, whose txid is the same
                // but whose BYTES are not what was engraved. Comparing against
                // it would report a match for something unbroadcastable.
                validate::finalized_guard_psbt(&psbt).map_err(|r| {
                    Refusal::new(
                        "verify",
                        "§1.1",
                        "the supplied PSBT is not finalized",
                        format!(
                            "{} mt compares against the transaction a PSBT \
                             EXTRACTS to, and an unfinalized one extracts to \
                             something that cannot be broadcast.",
                            r.verdict
                        ),
                    )
                })?;
                bitcoin::consensus::serialize(&psbt.extract_tx_unchecked_fee_rate())
            }
        };
        // The FULL 32-byte txid, never the 20-bit set id: a 20-bit compare
        // reports a match for any transaction sharing those bits, and says so in
        // the words "prove identity".
        let want = txid_display(bytes, "verify")?;
        let got = txid_display(&supplied, "verify")?;
        if want != got {
            return Err(Refusal::new(
                "verify",
                "§1.1",
                "the supplied transaction is not the one on these strings",
                format!(
                    "The strings reassemble to txid {want}. The supplied \
                     transaction is {got}. These differ in the FULL txid, not \
                     merely in the 20-bit set id."
                ),
            ));
        }
        let _ = writeln!(stderr, "  --transaction matches, on the full txid.");
    }
    Ok(())
}

fn inspect(args: ReadArgs) -> Result<(), Refusal> {
    let text = read_input(&args.r#in, "inspect")?;
    let read = read_strings::read(&text, "inspect")?;
    let strings = read.strings.clone();
    let set = pipeline::decode(&strings).map_err(|e| explain_failure(&strings, "inspect", &e))?;

    content_id_guard(&set.bytes, &set.chunks, "inspect")?;

    let tx = decode_tx(&set.bytes, "inspect")?;
    let txid = tx.compute_txid().to_string();

    // §6a: the node is consulted AUTOMATICALLY when one is reachable. The
    // operator is asked for nothing, because bitcoin-cli already holds
    // everything needed to reach it.
    let node = node::Node::find(&args.bitcoin_cli);

    let mut r = report::Report::build(&tx, &txid, node.as_ref(), &[]);
    r.set = Some((set.chunks.len(), set.chunks.len()));
    r.set_prefix = pipeline::invariant_prefix(&strings[0]).ok();

    let out = std::io::stdout();
    let mut out = out.lock();
    let _ = write!(
        out,
        "{}",
        if args.json {
            report::render_json(&r)
        } else {
            r.render()
        }
    );

    // The no-node warning goes to STDERR, in its recovery-time wording.
    if node.is_none() {
        let mut stderr = std::io::stderr();
        let _ = writeln!(stderr);
        let _ = write!(
            stderr,
            "{}",
            report::no_node_warning(
                &locktime::read(&tx),
                &txid,
                tx.input.iter().any(|i| i.witness.is_empty()),
            )
        );
    }
    transliteration_notices(&read, &mut std::io::stderr());
    set_notices(&set, &mut std::io::stderr());
    margin_report(&set.chunks, &mut std::io::stderr());
    Ok(())
}

/// Turn a codec failure into the message that actually helps.
///
/// **A dropped character reports as a MISSING PLATE once it reaches the codec.**
/// An omission shifts every symbol after it, so the string fails its checksum,
/// contributes no chunk, and the set says `chunk 3 of 6 is missing` — *an
/// accusation about the operator's steel*, sending them to hunt for a plate that
/// is sitting in front of them. §1.1e's length check exists to say the true
/// thing instead, and it is consulted HERE, on the failure path, because length
/// alone cannot tell a dropped character from a legitimately short final chunk.
fn explain_failure(strings: &[String], verb: &str, e: &mt_codec::Error) -> Refusal {
    // Which strings could not be read at all. Recomputed here rather than
    // threaded through the codec's error type: this runs only on the failure
    // path, where the operator is already stopped and one more pass over a
    // dozen strings costs nothing.
    let unreadable: Vec<usize> = strings
        .iter()
        .enumerate()
        .filter(|(_, s)| pipeline::decode_chunk(s, None).is_err())
        .map(|(i, _)| i + 1)
        .collect();
    if let Some(r) = read_strings::length_report(strings, &unreadable, verb) {
        return r;
    }

    // **THE SAME DOOR, THE OTHER HINGE.** §1.1e's length check closed this for a
    // MISSING or EXTRA character; a string with more than four SUBSTITUTIONS is
    // the same operator holding the same complete set, and it fell through to
    // `chunk 2 of 9 is missing` — nine plates on the table and mt naming one as
    // lost. `unreadable` is live, correct and in scope at this very line, and
    // the earlier version discarded it.
    // ONE CHUNK TWICE WHILE ANOTHER IS ABSENT is the fingerprint of the single
    // likeliest mechanical slip in the whole procedure: working from a stack,
    // typing one plate twice and skipping the next. The statement "chunk 5 is
    // missing" is TRUE, so the operator is not misdirected — merely left to find
    // the cause by counting. The hint is nearly free, because the duplicate is
    // already detected on the way through.
    let mut seen: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for c in strings
        .iter()
        .filter_map(|s| pipeline::decode_chunk(s, None).ok())
    {
        *seen.entry(c.header.index + 1).or_default() += 1;
    }
    let doubled: Vec<usize> = seen
        .iter()
        .filter(|&(_, n)| *n > 1)
        .map(|(&i, _)| i)
        .collect();
    let stack_hint = if doubled.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nChunk {} arrived TWICE. If you are working from a stack, check \
             whether you typed one plate twice and skipped another — that single \
             slip produces exactly this.",
            doubled
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    if !unreadable.is_empty() {
        // How many chunks the set SHOULD have, according to the strings that
        // did read. Without this mt cannot say "every plate is accounted for",
        // and that clause is the whole difference between "re-read one plate"
        // and "go and find a plate".
        let readable: Vec<_> = strings
            .iter()
            .filter_map(|s| pipeline::decode_chunk(s, None).ok())
            .collect();
        let count = readable.first().map(|c| c.header.count);
        // DISTINCT CHUNKS, not lines typed. `strings.len()` counts what the
        // operator entered, and the likeliest slip -- working from a stack,
        // typing one plate twice and skipping the next -- keeps that count at n
        // while a chunk is genuinely absent. mt then asserted "EVERY PLATE IS
        // ACCOUNTED FOR. Nothing is lost", categorically, and wrong.
        let distinct: std::collections::BTreeSet<usize> =
            readable.iter().map(|c| c.header.index).collect();

        let list = unreadable
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        // An unreadable string could be ANY chunk, so the best case is that each
        // one is a chunk not otherwise present.
        let accounted = match count {
            Some(n) if distinct.len() + unreadable.len() >= n => format!(
                "This set has {n} chunks. {} read cleanly and {} did not, so every \
                 chunk COULD be here — nothing is necessarily lost, and one plate \
                 is damaged past what BCH can repair.",
                distinct.len(),
                unreadable.len()
            ),
            Some(n) => format!(
                "This set has {n} chunks and only {} distinct ones are present, \
                 even counting the {} unreadable string(s) as chunks you hold. SO A \
                 PLATE IS MISSING AS WELL AS DAMAGED — re-reading the damaged one \
                 will not complete the set.",
                distinct.len(),
                unreadable.len()
            ),
            None => "mt cannot tell how many chunks this set should have: no string \
                     read cleanly, and the count is written on the strings."
                .to_string(),
        };

        return Refusal::new(
            verb,
            "§1.1",
            format!(
                "string {list} could not be read: more than 4 characters differ \
                 from what was engraved"
            ),
            format!(
                "BCH repairs up to 4 wrong characters per string. Past that it \
                 cannot tell which are wrong, so it refuses rather than guessing \
                 — a wrong guess produces a valid-looking string carrying the \
                 wrong bytes.\n\
                 \n\
                 {accounted}{stack_hint}"
            ),
        )
        .with_remedy(
            "Re-read that plate from the steel, character by character. Confusable \
             pairs to check first: 0/o, 1/l/i, b/6, 2/z, 5/s, 8/b.",
        );
    }
    Refusal::new(
        verb,
        "§1.1",
        "the set does not verify",
        format!("{e}{stack_hint}"),
    )
}

/// §1.1's last check: **the reassembled transaction must re-derive the id every
/// chunk header carries.**
///
/// It was the one check in §1.1's list with no code behind it, and `verify`
/// printed *"transaction re-derives"* on every run without ever deriving it.
/// Two independent reviews found it from opposite directions — one by reading
/// the spec against the code, one by FORGING the exact state it defends
/// against: valid checksums, intact headers, wrong payload. With the check
/// absent, `verify` passed that set and `decode` emitted the wrong
/// transaction's broadcastable hex.
///
/// **This is what makes recovery decidable**, and the plan leaned on it when it
/// ruled bespoke header-corruption tests a won't-fix: past the `t = 4` budget a
/// recoverer can disregard headers entirely and search orderings, *with the
/// content id validating the result*.
///
/// **What it does NOT prove, stated because the honest limit matters here.** The
/// txid identifies the transaction; it does not cover the witness data, which is
/// most of the payload. Damage inside a signature does not change the txid, so
/// this can pass on bytes that will not broadcast. Per-string correction is
/// BCH's job.
fn content_id_guard(
    bytes: &[u8],
    chunks: &[mt_codec::DecodedChunk],
    verb: &str,
) -> Result<(), Refusal> {
    let expected = chunks[0].header.chunk_set_id;
    let txid = txid_display(bytes, verb)?;
    let derived = pipeline::content_id_from_txid_display(&txid)
        .map_err(|e| Refusal::new(verb, "§1.1", "cannot derive the content id", format!("{e}")))?;
    if derived == expected {
        return Ok(());
    }

    // THE MARGIN REPORT IS ALREADY THE SUSPECT LIST. Miscorrection risk rises
    // with corrections applied: a chunk that needed none is almost certainly
    // intact, and the one that spent its whole budget is the one most likely to
    // have spent more than it had. Ordering is the entire value -- "something is
    // wrong somewhere in 1,242 characters" leaves the operator with a pile of
    // steel and nowhere to start.
    let mut ranked: Vec<&mt_codec::DecodedChunk> =
        chunks.iter().filter(|c| c.corrected > 0).collect();
    ranked.sort_by(|a, b| b.corrected.cmp(&a.corrected));

    let mut list = String::new();
    {
        use core::fmt::Write as _;
        for (n, c) in ranked.iter().enumerate() {
            let tag = if n == 0 { "   <-- most suspect" } else { "" };
            let _ = writeln!(
                list,
                "  chunk {:>3}   {} of 4 symbols corrected{tag}",
                c.header.index + 1,
                c.corrected
            );
        }
        let clean = chunks.len() - ranked.len();
        if clean > 0 {
            let _ = write!(
                list,
                "The other {clean} chunk{} needed no correction and {} almost \
                 certainly right.",
                if clean == 1 { "" } else { "s" },
                if clean == 1 { "is" } else { "are" }
            );
        }
    }

    let mechanism = "These chunks do not add up to the transaction they name. The likeliest \
         cause is MIS-CORRECTION: a chunk took more than 4 damaged symbols, and \
         BCH repaired it into a valid string that is not what you engraved. A \
         chunk cannot detect this about itself.\n\
         \n\
         The rarer cause is a chunk carried in from a DIFFERENT transaction whose \
         20-bit id collides with this one. mt cannot tell the two apart, and your \
         action is the same either way.\n\
         \n\
         NOTE: this check identifies the TRANSACTION. It does NOT prove every \
         byte. Damage inside the witness data — the signatures, most of the \
         payload — does not change the txid, so mt can pass this check on bytes \
         that will not broadcast.";

    let ranked_block = if ranked.is_empty() {
        None
    } else {
        Some(list.clone())
    };
    let remedy = if ranked.is_empty() {
        // No chunk was corrected, so miscorrection is not the explanation and
        // there is no ranking to offer. Saying so beats an empty list.
        "No chunk needed any correction, so this is not miscorrection — the set \
         is most likely mixing chunks from two different transactions. Check \
         that every plate came from the same engraving."
            .to_string()
    } else {
        "Most likely first — re-type these from the steel, in this order:".to_string()
    };

    let refusal = Refusal::new(
        verb,
        "§1.1",
        format!(
            "{} chunks, set {expected:#07x}, every checksum holds, but the \
             transaction re-derives {derived:#07x}",
            chunks.len()
        ),
        mechanism,
    )
    .with_remedy(remedy);
    Err(match ranked_block {
        Some(b) => refusal.with_verbatim(b),
        None => refusal,
    })
}

/// §10.10: **a txid is recognisable as such, so say so.**
///
/// 64 hex characters is a transaction ID, not a transaction — and it is an easy
/// thing to reach for, because it is what a block explorer shows and what `mt`
/// itself prints in its `TX` row. Without this it fell through to "the bytes
/// are valid hex but do not parse as a transaction", which is true and sends
/// the operator to look at the wrong thing entirely.
fn txid_paste_guard(raw: &[u8], verb: &str) -> Result<(), Refusal> {
    let text: String = core::str::from_utf8(raw)
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if text.len() != 64 || !text.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(Refusal::new(
        verb,
        "§10.10",
        "this is a transaction ID (64 hex characters), not a transaction",
        "A txid NAMES a transaction; it does not contain one. mt engraves the \
         transaction itself — the bytes a node would broadcast — because a txid \
         is useless to a recoverer holding steel and no chain.\n\
         \n\
         mt prints the txid in its own TX row, which is probably where this came \
         from.",
    )
    .with_remedy(
        "Fetch the transaction: `bitcoin-cli getrawtransaction <txid>`, then pass \
         that. Or `bitcoin-cli finalizepsbt <psbt>` if you have not broadcast yet.",
    ))
}

/// A flag that cannot work must REFUSE, not sit there inert.
///
/// `--json` is meaningful on the two verbs that PRINT A REPORT (`inspect` and
/// `decode`). On `encode` and `verify` there is no report to serialise —
/// `encode`'s stdout is the strings and `verify`'s output is a verdict — so the
/// flag has nothing to do.
///
/// **Doing nothing quietly is the defect, not the absence of the feature.** A
/// caller who asks for machine output and receives prose with exit 0 will parse
/// *something* out of it. Refusing tells them at once, in the only place they
/// are looking.
fn json_unsupported_guard(json: bool, verb: &str) -> Result<(), Refusal> {
    if !json {
        return Ok(());
    }
    Err(Refusal::new(
        verb,
        "§10.10",
        format!("--json has no meaning for `{verb}`"),
        "`--json` serialises the inspection REPORT, and this verb does not print \
         one: `encode`'s stdout is the mt1 strings themselves, and `verify` \
         answers with a verdict and a margin report.\n\
         \n\
         mt refuses rather than accepting the flag and ignoring it — a caller who \
         asks for machine output and gets prose with exit 0 will parse something \
         out of the prose.",
    )
    .with_remedy("Use `mt inspect --json` or `mt decode --json` for the report."))
}

/// A transaction's txid in display form.
///
/// Named rather than inlined because §8.5's guard and the report must ask about
/// **the same** txid — and the one thing that must never be used here is the
/// double-SHA of the engraved bytes, which is the *wtxid* for any segwit
/// transaction.
fn txid_of(tx: &bitcoin::Transaction) -> String {
    tx.compute_txid().to_string()
}

/// Decode raw bytes into a transaction, with a refusal that names the problem.
fn decode_tx(bytes: &[u8], verb: &str) -> Result<bitcoin::Transaction, Refusal> {
    use bitcoin::consensus::Decodable;
    bitcoin::Transaction::consensus_decode(&mut &bytes[..]).map_err(|e| {
        Refusal::new(
            verb,
            "§8.2e",
            "the reassembled bytes are not a Bitcoin transaction",
            format!(
                "The strings reassembled cleanly, but the result does not parse: \
                 {e}. Every checksum held, so this is not miscut steel — it is \
                 more likely a chunk from a different set."
            ),
        )
    })
}

/// Parse `--input-value <index>:<amount>`.
///
/// **Per input, never a total.** A supplied total has two readings — "this IS
/// the input sum" and "this is ADDED to the bound inputs" — that differ by a
/// whole input, and which one an implementer picked would decide whether §8.2b's
/// refusals fire at all.
///
/// **The amount is parsed as a DECIMAL STRING into satoshis, never through
/// `f64`.** The previous version did `s.parse::<f64>()` then
/// `(btc * 100_000_000.0).round() as u64`, and an adversarial review found the
/// hole by typing what a person mistypes: `--input-value 0:inf` **panicked**,
/// and `0:1e30` panicked with it. `-5` and `NaN` did not panic — worse, they
/// produced a silent nonsense value that tripped §8.2b for the wrong reason, so
/// the operator got a refusal about their outputs when the fault was their
/// input.
///
/// Parsing the string also gives the honest refusal for `0:1.234567891`: nine
/// decimal places is not a satoshi amount, and rounding it silently is how a
/// wrong number gets engraved.
fn parse_input_values(raw: &[String]) -> Result<Vec<(u32, u64)>, Refusal> {
    raw.iter()
        .map(|s| {
            let (i, v) = s.split_once(':').ok_or_else(|| {
                Refusal::new(
                    "encode",
                    "§8.2c",
                    format!("--input-value {s:?} is not <index>:<amount>"),
                    "Values are supplied PER INPUT, as `--input-value 0:0.05`. A \
                     single total is not accepted: it has two readings that differ \
                     by a whole input.",
                )
            })?;
            let idx: u32 = i.parse().map_err(|_| {
                Refusal::new(
                    "encode",
                    "§8.2c",
                    format!("--input-value index {i:?} is not a number"),
                    "The index is the input's position, counting from 0.",
                )
            })?;
            Ok((idx, parse_btc(v)?))
        })
        .collect()
}

/// §1.1e: the separator must be something the READING side strips.
///
/// `read_strings` strips whitespace and nothing else, so a separator of any
/// other kind lands on **stdout** — the stream the operator engraves — and mt's
/// own verbs then refuse the result. The sequence that makes it expensive:
/// choose `-`, cut nine plates over several hours, type them back, and find
/// that mt cannot read what mt produced — having been told, by mt, to "verify
/// the ENGRAVING, not this output".
fn separator_guard(sep: &str) -> Result<(), Refusal> {
    if sep.is_empty() || sep.chars().all(char::is_whitespace) {
        return Ok(());
    }
    Err(Refusal::new(
        "encode",
        "§1.1e",
        format!("--separator {sep:?} is not whitespace"),
        "mt strips WHITESPACE when it reads strings back, and nothing else. A \
         separator of any other kind lands on stdout — the stream you engrave — \
         and mt's own verbs then refuse the result: the codec sees it as a data \
         character outside the bech32 alphabet.\n\
         \n\
         mt refuses this now rather than after nine plates are cut.",
    )
    .with_remedy("Use a space, a tab, or omit --separator (a space is the default)."))
}

/// §8.2c: an index naming an input the transaction does not have.
///
/// **Silently ignored**, and the consequence is not cosmetic: a mistyped index
/// means the input the operator MEANT to supply still has no value, so §8.2b's
/// arithmetic silently does not run — no fee check, no inputs-cover-outputs
/// check — and mt prints `FEE UNKNOWN` while they believe they supplied it.
fn input_index_range_guard(asserted: &[(u32, u64)], inputs: usize) -> Result<(), Refusal> {
    let Some((i, _)) = asserted.iter().find(|(i, _)| *i as usize >= inputs) else {
        return Ok(());
    };
    Err(Refusal::new(
        "encode",
        "§8.2c",
        format!("--input-value names input {i}, but this transaction has {inputs} input(s)"),
        "Indices count from 0. A value supplied for an input that does not exist \
         is silently no value at all for the input you meant — and §8.2b's fee \
         and balance checks then do not run, while mt prints FEE UNKNOWN as \
         though you had supplied nothing.",
    ))
}

/// §8.2c: an index that names one input twice.
///
/// **Both were silently ignored**, and the consequence is not cosmetic: a
/// mistyped index means the input the operator MEANT to supply still has no
/// value, so §8.2b's arithmetic silently does not run — no fee check, no
/// inputs-cover-outputs check — and `mt` prints `FEE UNKNOWN` while the operator
/// believes they supplied it.
///
/// Checked against the transaction's real input count, which is why it runs
/// after sniffing rather than inside the parser.
fn check_input_value_indices(asserted: &[(u32, u64)], _args: &EncodeArgs) -> Result<(), Refusal> {
    let mut seen = std::collections::BTreeSet::new();
    for (i, _) in asserted {
        if !seen.insert(*i) {
            return Err(Refusal::new(
                "encode",
                "§8.2c",
                format!("--input-value names input {i} more than once"),
                "Two values for one input have no defined meaning, and taking \
                 either silently would decide the fee. mt does not choose.",
            ));
        }
    }
    Ok(())
}

/// Satoshis as BTC, for messages composed here.
fn fmt_btc(sats: u64) -> String {
    format!("{}.{:08} BTC", sats / 100_000_000, sats % 100_000_000)
}

/// A BTC amount as a decimal string, in satoshis.
///
/// Accepts `<digits>` or `<digits>.<1..=8 digits>`, and nothing else — no sign,
/// no exponent, no `inf`, no `NaN`, no ninth decimal place.
fn parse_btc(v: &str) -> Result<u64, Refusal> {
    let bad = |why: &str| {
        Refusal::new(
            "encode",
            "§8.2c",
            format!("--input-value amount {v:?} is not a BTC amount"),
            format!(
                "{why} An amount is plain decimal BTC with at most 8 places — \
                 `0.05000000`, `1`, `21000000`. mt does not accept a sign, an \
                 exponent, or a value it would have to round: the fee absorbs \
                 every error in an input value, in full."
            ),
        )
    };
    let (whole, frac) = match v.split_once('.') {
        Some((w, f)) => (w, f),
        None => (v, ""),
    };
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("That is not a decimal number."));
    }
    if !frac.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("The fractional part is not digits."));
    }
    if frac.len() > 8 {
        return Err(bad(&format!(
            "It has {} decimal places; a satoshi is the eighth.",
            frac.len()
        )));
    }
    let whole: u64 = whole
        .parse()
        .map_err(|_| bad("The whole part is too large."))?;
    let mut sats = whole
        .checked_mul(100_000_000)
        .ok_or_else(|| bad("The whole part is too large."))?;
    if !frac.is_empty() {
        let scale = 10u64.pow((8 - frac.len()) as u32);
        let f: u64 = frac
            .parse()
            .map_err(|_| bad("The fractional part is not a number."))?;
        sats = sats
            .checked_add(f * scale)
            .ok_or_else(|| bad("The amount is too large."))?;
    }
    // 21,000,000 BTC is every satoshi that will ever exist. A larger value is
    // not a mistake mt should carry into a fee calculation.
    const MAX_SATS: u64 = 21_000_000 * 100_000_000;
    if sats > MAX_SATS {
        return Err(bad("It exceeds 21,000,000 BTC, the entire supply."));
    }
    Ok(sats)
}
