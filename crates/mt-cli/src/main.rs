//! The `mt` CLI.
//!
//! **stdout carries the artifact; stderr carries everything a human must see.**
//! That split is a hard interface boundary, not a formatting preference: the
//! output of `mt encode` exists to be piped, and the moment a legend line, a
//! banner or a blank separator shares that stream, every downstream consumer has
//! to parse `mt`'s prose out of its own input — and the first one that forgets
//! engraves a warning label as though it were a chunk.

mod blocks;
mod input;
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

    /// Compare against a transaction, by FULL txid.
    ///
    /// `verify` only. Comparing against the 20-bit set id would report a match
    /// for any transaction sharing those bits — 1 in 1,048,576 by accident, and
    /// under a second to construct deliberately.
    #[arg(long, value_name = "PSBT|HEX")]
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

    /// Separator to use with `--group-size`.
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

    // §8.2f FIRST, before a single byte is read. A bearer artifact on the
    // command line has ALREADY leaked — into the shell's history file and into
    // `ps` for every user on the machine — so the refusal has to reach the
    // operator whether or not the rest of the invocation makes sense.
    validate::command_line_guard(&std::env::args().collect::<Vec<_>>())?;

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

    let sniffed = input::sniff(&raw)?;
    let asserted = parse_input_values(&args.input_value)?;

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
                    let bound = psbt.inputs[n].non_witness_utxo.is_some();
                    validate::psbt_input_value(&psbt, n)
                        .map(|v| {
                            (
                                v,
                                if bound {
                                    report::Provenance::TxidBound
                                } else {
                                    report::Provenance::PsbtClaimed
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

    // §8.6 binds both payloads: an input whose satisfaction does not bind the
    // outputs is redirectable by any holder, and the legend's TO line is a lie.
    validate::satisfaction_guard(&tx)?;

    // §6a: the node is consulted AUTOMATICALLY. It is where an unbound value
    // becomes a bound one, so it runs BEFORE §8.2b's arithmetic.
    let node = node::Node::find(&args.bitcoin_cli);
    let mut bound_by_chain = vec![false; tx.input.len()];
    if let Some(nd) = &node {
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
    for (n, inp) in tx.input.iter().enumerate() {
        let legacy = inp.witness.is_empty();
        if legacy && !bound_by_chain[n] {
            if let Some((claimed, _)) = values[n] {
                let _ = writeln!(
                    stderr,
                    "{}",
                    validate::legacy_unbound_warning(n, claimed, out_total)
                );
            }
        }
    }

    // §8.2b again, in its WARNING half: no minimum fee, but say the rate.
    if let Some(w) = validate::low_fee_warning(&tx, &sats) {
        let _ = writeln!(stderr, "{w}");
    }

    if from_raw_hex {
        let _ = writeln!(
            stderr,
            "{}",
            refusal::Warning::new(
                "this is a RAW TRANSACTION, so mt cannot check what a PSBT would carry.",
                "A raw transaction has no UTXO records, so mt cannot see any input's \
                 value and cannot check the fee. Supply values with --input-value \
                 <index>:<amount>, or re-run with a node reachable so mt can fetch \
                 them. (§8.2e)"
            )
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
fn txid_display(bytes: &[u8]) -> Result<String, Refusal> {
    use bitcoin::consensus::Decodable;
    let tx = bitcoin::Transaction::consensus_decode(&mut &bytes[..]).map_err(|e| {
        Refusal::new(
            "encode",
            "§8.2e",
            "input is not a decodable Bitcoin transaction",
            format!(
                "The bytes are valid hex but do not parse as a transaction: {e}. \
                 mt reads an ALREADY-SIGNED transaction; it does not build one."
            ),
        )
        .with_remedy("Check this is the output of `finalizepsbt`, not a template.")
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
        let positions: Vec<String> = c
            .corrected_positions
            .iter()
            // data-part offset -> 1-based whole-string position
            .map(|p| (p + 1 + 3).to_string())
            .collect();
        let _ = writeln!(
            out,
            "  chunk {:>3}   {} of {T} symbols   pos {}{margin}",
            c.header.index + 1,
            c.corrected,
            positions.join(", ")
        );
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

fn decode(args: ReadArgs) -> Result<(), Refusal> {
    let text = read_input(&args.r#in, "decode")?;
    let strings = read_strings::read(&text)?;
    let (bytes, chunks) = pipeline::decode(&strings).map_err(|e| {
        Refusal::new(
            "decode",
            "§1.1a",
            "cannot reassemble the set",
            format!("{e}"),
        )
    })?;

    let mut stderr = std::io::stderr();
    if !args.quiet {
        // decode PRINTS THE REPORT, because decode is the verb a recoverer
        // reaches for first — `inspect` is the one designed for them, and they
        // have no way to know that. A silent decode hands a stranger sixty
        // kilobytes of hex before anything has told them what it does.
        let _ = writeln!(stderr, "TX        {}", txid_display(&bytes)?);
        let _ = writeln!(stderr, "mt1 SET   {} strings, all present", chunks.len());
        margin_report(&chunks, &mut stderr);
    }

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
    let text = read_input(&args.r#in, "verify")?;
    let strings = read_strings::read(&text)?;
    let (bytes, chunks) = pipeline::decode(&strings)
        .map_err(|e| Refusal::new("verify", "§1.1", "the set does not verify", format!("{e}")))?;

    let mut stderr = std::io::stderr();
    let set_id = chunks[0].header.chunk_set_id;
    let _ = writeln!(
        stderr,
        "mt verify: OK — {} chunks, set {set_id:#07x}, transaction re-derives.",
        chunks.len()
    );
    margin_report(&chunks, &mut stderr);

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
        let supplied = match input::sniff(&supplied)? {
            input::Input::RawHex(b) => b,
            input::Input::Psbt(_) => {
                return Err(Refusal::new(
                    "verify",
                    "§1.1",
                    "PSBT comparison lands with PSBT support",
                    "A supplied PSBT is compared against its EXTRACTED transaction \
                     (§10.13 c). Extraction arrives with the rest of §8.",
                ));
            }
        };
        // The FULL 32-byte txid, never the 20-bit set id: a 20-bit compare
        // reports a match for any transaction sharing those bits, and says so in
        // the words "prove identity".
        let want = txid_display(&bytes)?;
        let got = txid_display(&supplied)?;
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
    let strings = read_strings::read(&text)?;
    let (bytes, chunks) = pipeline::decode(&strings).map_err(|e| {
        Refusal::new(
            "inspect",
            "§1.1",
            "cannot reassemble the set",
            format!("{e}"),
        )
    })?;

    let tx = decode_tx(&bytes, "inspect")?;
    let txid = tx.compute_txid().to_string();

    // §6a: the node is consulted AUTOMATICALLY when one is reachable. The
    // operator is asked for nothing, because bitcoin-cli already holds
    // everything needed to reach it.
    let node = node::Node::find(&args.bitcoin_cli);

    let mut r = report::Report::build(&tx, &txid, node.as_ref(), &[]);
    r.set = Some((chunks.len(), chunks.len()));

    let out = std::io::stdout();
    let mut out = out.lock();
    let _ = write!(out, "{}", r.render());

    // The no-node warning goes to STDERR, in its recovery-time wording.
    if node.is_none() {
        let mut stderr = std::io::stderr();
        let _ = writeln!(stderr);
        let _ = write!(
            stderr,
            "{}",
            report::no_node_warning(tx.lock_time.to_consensus_u32(), &txid)
        );
    }
    margin_report(&chunks, &mut std::io::stderr());
    Ok(())
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
            let btc: f64 = v.parse().map_err(|_| {
                Refusal::new(
                    "encode",
                    "§8.2c",
                    format!("--input-value amount {v:?} is not a number"),
                    "The amount is in BTC, as a decimal — e.g. 0.05000000.",
                )
            })?;
            Ok((idx, (btc * 100_000_000.0).round() as u64))
        })
        .collect()
}
