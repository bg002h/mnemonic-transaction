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
