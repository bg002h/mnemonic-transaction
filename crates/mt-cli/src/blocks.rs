//! The three mandatory `stderr` blocks, and the TTY welcome line.
//!
//! All four are things the operator actually reads, and all four were assigned
//! to no phase until R8 and R11 found them. They go to **`stderr`**, never
//! `stdout`: §0a rules that stdout carries the strings and nothing else, because
//! the output exists to be piped and the first consumer that forgets would
//! engrave a warning label as though it were a chunk.

use crate::refusal::Warning;
use std::io::IsTerminal;

/// Printed when `mt encode` is invoked with **stdin attached to a terminal**.
///
/// Without it the tool blocks on stdin with no prompt, so a new user's first
/// action looks like a hang. §10.10 names the cost plainly: *"a new user
/// concluding the tool does not work and leaving, which no other check
/// catches."*
///
/// This exists because the operator, walking the encode journey, asked *"stdin
/// doesn't mean from the command line?"* — **the confusion was the finding**.
pub fn welcome_if_tty() -> Option<String> {
    if std::io::stdin().is_terminal() {
        Some(
            concat!(
                "mt encode: reading a transaction from stdin.\n",
                "\n",
                "  Paste a finalized PSBT (base64) or a raw signed transaction (hex),\n",
                "  then press Ctrl-D on a new line. Or pass a file with --in.\n",
                "\n",
                "  Nothing has happened yet.\n",
            )
            .to_string(),
        )
    } else {
        None
    }
}

/// **The BEARER warning**, carrying both halves.
///
/// §8.6 refuses inputs whose satisfaction does not bind the outputs, so in the
/// ordinary case a holder cannot redirect the money. But that check reads
/// **witness shape**, not script — §8.2's removal left no script engine — so it
/// is a structural heuristic and not a proof. Saying only the first half would
/// engrave a guarantee `mt` cannot make.
pub fn bearer_warning() -> Warning {
    Warning::new(
        "anyone holding this engraving can broadcast this transaction.",
        "mt checked that every input carries a signature committing to the \
         outputs, so a holder should not be able to send the money anywhere \
         else. That check reads WITNESS SHAPE, not script — mt has no script \
         engine (§8.2). An exotic or hostile input CAN defeat it. Treat the \
         engraving as if a holder could take the funds.",
    )
}

/// **What correction does and does not cover** — printed ALWAYS, before cutting.
///
/// Nothing else in `mt`'s output contradicts the impression that "error
/// correction" has the operator covered, and §1.8's zero-redundancy ruling lives
/// only in the spec. The counting instruction is the operator's own check on the
/// damage BCH cannot touch: losing your place, skipping a glyph, doubling one —
/// which presents as total garbage rather than "four errors I can fix", yet is
/// trivially detectable by counting.
pub fn correction_coverage(lengths: &[usize]) -> Warning {
    let mut spans: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lengths.len() {
        let mut j = i;
        while j + 1 < lengths.len() && lengths[j + 1] == lengths[i] {
            j += 1;
        }
        spans.push(if i == j {
            format!("string {} is {}", i + 1, lengths[i])
        } else {
            format!("strings {}-{} are {}", i + 1, j + 1, lengths[i])
        });
        i = j + 1;
    }
    Warning::new(
        "before you cut: mt corrects up to 4 wrong CHARACTERS per string.",
        format!(
            "It cannot repair a MISSING or EXTRA character — those shift every \
             symbol after them. Count each string: {}. \
             \n\nIt cannot repair a missing STRING either. There is no \
             redundancy: all {} strings are required. To survive losing one, cut \
             a second copy — mt will not do it for you.",
            spans.join(", "),
            lengths.len()
        ),
    )
}

/// **Verify the STEEL, not this output.**
///
/// After `encode` succeeds the operator holds two copies of the same strings:
/// the ones on stdout and the ones they cut. Verifying the file proves nothing
/// about the engraving — it re-checks `mt`'s own output, which was correct by
/// construction. The whole point of BCH is to catch what the *hand* got wrong.
pub fn verify_the_steel() -> Warning {
    Warning::new(
        "when you are done, verify the ENGRAVING — not this output.",
        "Type the strings back from the steel and run:\n\n    mt verify < typed-from-steel.txt\n\n\
         Verifying the file mt just produced tests nothing that can fail.",
    )
}

/// §5's legend — the **suggested text an operator may cut beside their
/// strings**, printed on `stderr` by `mt encode`.
///
/// **§0a rules that `encode` prints these five fields, and nothing printed
/// them.** `--from`, `--to` and `--to-label` were accepted and silently
/// discarded for the whole of P2–P6: three of §10.10's twelve ruled flags
/// parsing into a struct nobody read. An independent spec-first review found it
/// by walking §5 and looking for the code.
///
/// **This is a SUGGESTION, not an artifact.** It goes to `stderr` because
/// stdout is the strings and nothing else, and because the layout on steel is
/// the operator's by ruling (§3b) — `mt` cannot see how strings are laid onto a
/// plate and does not try to.
///
/// The `~` on the year is load-bearing: a projection presented as a fact is the
/// mistake §9 refuses for fiat figures, and this text is cut into metal.
pub fn legend(
    lock: &crate::locktime::Lock,
    from: Option<&str>,
    to: Option<&str>,
    to_label: Option<&str>,
    outputs: &[u64],
) -> String {
    use core::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "SUGGESTED LEGEND — cut this beside the strings. mt cannot see your\n\
         plate, so the layout is yours (§3b); these are the five facts a\n\
         stranger needs BEFORE they can do anything with the steel.\n"
    );
    let _ = writeln!(s, "    BEARER - ANYONE HOLDING THIS CAN BROADCAST IT");
    let _ = writeln!(s, "    FORMAT: mt1 codex32");

    match from {
        Some(f) => {
            let _ = writeln!(s, "    FROM WALLET {f}");
        }
        None => {
            let _ = writeln!(s, "    FROM WALLET ????????        <-- NOT SUPPLIED");
        }
    }

    // The destination names a WALLET, not one truncated address. A free-text
    // label is allowed only behind its own flag, because nothing can check it
    // against the transaction — the separate flag IS the ruling (§10.4): it
    // makes the label an act of assertion rather than something that appears.
    //
    // **THE AMOUNT IS ONLY PRINTED WHEN mt KNOWS IT.** The earlier version put
    // the sum of ALL outputs beside the named wallet, so a transaction sending
    // 2.0 BTC with 5.999 BTC of change read `TO alice-cold 7.99900000 BTC` —
    // a figure that is not what alice-cold receives, suggested for PERMANENT
    // STEEL. mt cannot identify change: that needs the sending wallet's
    // descriptor, which it does not have and does not ask for (§6). With one
    // output there is nothing to confuse; with more than one, mt says so
    // instead of guessing.
    let name = to.or(to_label);
    let unverified = if to.is_none() && to_label.is_some() {
        "   <-- LABEL ONLY, unverified"
    } else {
        ""
    };
    match (name, outputs) {
        (Some(n), [only]) => {
            let _ = writeln!(s, "    TO {n}  {}{unverified}", btc(*only));
        }
        (Some(n), _) => {
            // No amount. A wrong number on steel outlives the plate.
            let _ = writeln!(s, "    TO {n}{unverified}");
        }
        (None, [only]) => {
            let _ = writeln!(s, "    TO ????????  {}   <-- NOT SUPPLIED", btc(*only));
        }
        (None, _) => {
            let _ = writeln!(s, "    TO ????????   <-- NOT SUPPLIED");
        }
    }
    let _ = writeln!(s, "    {}", lock.legend());

    // §10.4: optional, and LOUDLY WARNED when absent. A plate that does not say
    // where the money came from or where it went is one a recoverer cannot act
    // on -- and neither fact is in the transaction, so mt cannot fill them in.
    if from.is_none() || name.is_none() {
        let missing = match (from.is_none(), name.is_none()) {
            (true, true) => "FROM WALLET and TO are",
            (true, false) => "FROM WALLET is",
            _ => "TO is",
        };
        let _ = writeln!(
            s,
            "\n  {missing} NOT SUPPLIED. The transaction does not carry either\n  \
             fact — it names outpoints and scripts, not wallets — so mt cannot\n  \
             fill it in and will not guess. Supply --from / --to, or engrave the\n  \
             line by hand. A plate that says neither leaves a recoverer holding\n  \
             steel they cannot place."
        );
    }
    if outputs.len() > 1 {
        let _ = writeln!(
            s,
            "\n  NO AMOUNT on the TO line: this transaction has {} outputs and mt\n  \
             cannot tell which is the destination and which is CHANGE — that\n  \
             needs the sending wallet's descriptor, which mt never sees. Write\n  \
             the amount yourself if you want it on the plate; the report above\n  \
             lists every output.",
            outputs.len()
        );
    }
    s
}

fn btc(sats: u64) -> String {
    format!("{}.{:08} BTC", sats / 100_000_000, sats % 100_000_000)
}

/// The output file is BEARER too, and nothing said so.
///
/// `mt` warns in detail that the INPUT file is world-readable (§8.2g) — and
/// then writes the `mt1` strings to a file it never mentions again. Those
/// strings are the engraving: `mt`'s own bearer warning says *"a finalized
/// transaction — **and the mt1 strings it becomes** — is BEARER: anyone who
/// reads it can broadcast it."*
///
/// Printed only when stdout is REDIRECTED. On a terminal the strings scroll
/// past and the advice would be noise; redirected, there is a file sitting on
/// disk that outlives the session.
pub fn redirected_output_warning() -> crate::refusal::Warning {
    crate::refusal::Warning::new(
        "the file you just wrote is BEARER, exactly like the plate.",
        "mt sent the strings to a file rather than a terminal. Anyone who reads \
         that file can broadcast this transaction — it is the engraving, in a \
         form that copies itself.\n\
         \n\
         When the plates are cut and verified, DESTROY IT: `shred -u <file>` on \
         Linux, `rm -P <file>` on macOS. Plain `rm` unlinks the name and leaves \
         the bytes. And check it is not already in a backup, a sync folder, or \
         your editor's undo history.",
    )
}

/// §6a's **encode-time** no-node warning.
///
/// A different moment from the recovery-time one, so different words: the
/// operator is standing at the machine with the plate uncut, and their decision
/// is *cut now or check first*. The recovery-time wording — *"look this txid up
/// in a block explorer"* — is useless here, because there is nothing to look up
/// yet and the choice is still open.
pub fn encode_no_node_warning() -> crate::refusal::Warning {
    crate::refusal::Warning::new(
        "no bitcoind reachable — mt could not check the chain before you cut.",
        "These are the questions a node would have answered, and mt has NOT:\n\
         \n\
         \x20 - are these inputs still unspent, or did something else take them?\n\
         \x20 - what fee does this actually pay?\n\
         \x20 - how far away is the locktime, in real blocks?\n\
         \n\
         Engraving takes about 21 minutes per plate and is permanent. Running \
         mt again with a node reachable takes seconds and answers all three. \
         If the inputs turn out to be spent, the plate is scrap the moment it \
         leaves the machine.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_warning_carries_both_halves() {
        // Assert on the BODY, not the rendered string: wrapping inserts line
        // breaks mid-phrase, so a rendered-text assertion tests the wrapper
        // rather than the content it is supposed to guard.
        let w = bearer_warning();
        let s = format!("{} {}", w.headline, w.body);
        assert!(s.contains("broadcast"), "must say what a holder CAN do");
        assert!(
            s.contains("WITNESS SHAPE") && s.contains("no script engine"),
            "must say the check is structural, not a proof"
        );
        assert!(
            s.contains("CAN defeat it"),
            "must say an exotic input can defeat the check"
        );
        assert!(
            !s.contains("cannot redirect"),
            "must not state an unqualified guarantee §8.6 does not deliver"
        );
    }

    /// The operator counts characters against this. A wrong span sends them to
    /// re-cut a string that is correct.
    #[test]
    fn correction_coverage_groups_equal_lengths() {
        let w = correction_coverage(&[90, 90, 90, 72]);
        assert!(w.body.contains("strings 1-3 are 90"), "got: {}", w.body);
        assert!(w.body.contains("string 4 is 72"), "got: {}", w.body);
        assert!(w.body.contains("all 4 strings are required"));
    }

    #[test]
    fn correction_coverage_handles_a_single_string() {
        let w = correction_coverage(&[87]);
        assert!(w.body.contains("string 1 is 87"), "got: {}", w.body);
    }

    #[test]
    fn steel_instruction_names_the_command() {
        assert!(verify_the_steel().to_string().contains("mt verify <"));
    }
}
