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
mod refusal;

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
        Command::Encode(args) => match encode(args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(r) => {
                eprint!("{r}");
                std::process::ExitCode::FAILURE
            }
        },
    }
}

fn encode(args: EncodeArgs) -> Result<(), Refusal> {
    let mut stderr = std::io::stderr();

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

    let sniffed = input::sniff(&raw)?;
    let (tx_bytes, from_raw_hex) = match sniffed {
        input::Input::RawHex(b) => (b, true),
        input::Input::Psbt(_) => {
            return Err(Refusal::new(
                "encode",
                "§8.1",
                "PSBT support lands in a later phase",
                "This build reads a raw signed transaction. PSBT extraction, and \
                 the refusals that depend on a PSBT's UTXO records, arrive with \
                 the rest of §8.",
            )
            .with_remedy("Run `bitcoin-cli finalizepsbt <psbt>` and pass the resulting hex."));
        }
    };

    let txid = txid_display(&tx_bytes)?;
    let strings = pipeline::encode(&tx_bytes, &txid).map_err(|e| {
        Refusal::new(
            "encode",
            "§3b",
            "transaction cannot be chunked",
            format!("{e}"),
        )
    })?;

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
