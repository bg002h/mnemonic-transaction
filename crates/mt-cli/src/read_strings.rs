//! Turning what an operator typed back from steel into strings `mt` can parse.
//!
//! §1.1e's order, and the first step is the one that was unbuildable as
//! originally written: **SPLIT FIRST, then strip.** "Strip whitespace before
//! doing anything else" taken literally turns fourteen 90-character strings into
//! one 1,242-character blob, and the tool cannot parse its own output.
//!
//! Then: normalise to lowercase, restore any elided prefix, and only then hand
//! anything to the codec.

use crate::refusal::Refusal;
use mt_codec::consts::INVARIANT_PREFIX_SYMBOLS;

/// The `mt1` prefix plus the invariant symbols an elided line omits.
const ELIDED_DROP: usize = 3 + INVARIANT_PREFIX_SYMBOLS;

/// Split raw input into candidate strings, normalise, and restore elision.
pub fn read(raw: &str) -> Result<Vec<String>, Refusal> {
    let mut candidates: Vec<String> = Vec::new();

    // 1. SPLIT on any whitespace run containing a newline. Spaces and tabs
    //    *within* a line are grouping separators (§1.1e's --group-size) and are
    //    stripped, not treated as boundaries.
    for line in raw.lines() {
        let joined: String = line.split_whitespace().collect();
        if joined.is_empty() {
            continue;
        }
        // 2. A line containing more than one `mt1` prefix is split at each one.
        //    This is what makes the single-line pasted blob work rather than
        //    fail — and the recovery path is where that matters, because a
        //    refusal there is answered by retyping 1,242 characters off steel.
        let lower = joined.to_ascii_lowercase();
        let mut starts: Vec<usize> = lower.match_indices("mt1").map(|(i, _)| i).collect();
        if starts.first().copied() != Some(0) {
            starts.insert(0, 0);
        }
        for (n, &start) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(lower.len());
            let piece = &lower[start..end];
            if !piece.is_empty() {
                candidates.push(piece.to_string());
            }
        }
    }

    if candidates.is_empty() {
        return Err(Refusal::new(
            "decode",
            "§1.1e",
            "no strings found in the input",
            "mt splits input on line breaks, and on each `mt1` prefix within a \
             line. Nothing in this input looked like either.",
        )
        .with_remedy("Check the file, or pipe the strings in with `mt decode < file`."));
    }

    // 3. Restore elided lines from the set's full string (§3b).
    restore_elided(candidates)
}

/// Restore `--elide-prefix` output.
///
/// Detection needs no flag: a line beginning `mt1` is full, anything else is
/// elided. **Mixed input is legal** — an operator who elides "after a while"
/// produces exactly that.
fn restore_elided(candidates: Vec<String>) -> Result<Vec<String>, Refusal> {
    let full = candidates.iter().find(|s| s.starts_with("mt1"));

    let Some(full) = full else {
        return Err(Refusal::new(
            "decode",
            "§3b",
            format!(
                "all {} lines are elided; no prefix to restore",
                candidates.len()
            ),
            "An elided line carries only its index and payload — the set's \
             invariant prefix was cut once, on another line. Without at least one \
             full string there is nothing to restore from, and mt will not guess.",
        )
        .with_remedy(
            "Add the 8 characters following `mt1` on any intact string of the same \
             set. (They are recoverable by search if that string is lost — mt v0.1 \
             does not implement the search.)",
        ));
    };

    if full.len() < ELIDED_DROP {
        return Err(Refusal::new(
            "decode",
            "§3b",
            format!("the full string is only {} characters", full.len()),
            "A full string must carry `mt1` plus at least the 8-symbol invariant \
             prefix before anything can be restored from it.",
        ));
    }
    let prefix = full[..ELIDED_DROP].to_string();

    Ok(candidates
        .into_iter()
        .map(|s| {
            if s.starts_with("mt1") {
                s
            } else {
                format!("{prefix}{s}")
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str =
        "mt1p9h8jqq9qqqqgqqqqqqqyqherdfykhhpey6z2cvafak8804qd7g0dl6v8ex9wr2cvky023skwkeud2229sax";
    const B: &str =
        "mt1p9h8jqq9qqphgdqqqqqqqq0mllllupyqj6vqqqqqqqqzcqpfsw7ph2rt5w54kt768636cls8zxg0najlzunp";

    #[test]
    fn splits_on_lines() {
        let got = read(&format!("{A}\n{B}\n")).unwrap();
        assert_eq!(got, vec![A.to_string(), B.to_string()]);
    }

    /// The case that made "strip first" unbuildable: an operator copies several
    /// strings out of a terminal and they arrive as one line.
    #[test]
    fn splits_a_single_line_blob_at_each_prefix() {
        let got = read(&format!("{A}{B}")).unwrap();
        assert_eq!(got, vec![A.to_string(), B.to_string()]);
    }

    /// Spaces WITHIN a line are grouping separators, not boundaries.
    #[test]
    fn strips_grouping_within_a_line() {
        let grouped = A
            .chars()
            .collect::<Vec<_>>()
            .chunks(8)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(read(&grouped).unwrap(), vec![A.to_string()]);
    }

    #[test]
    fn normalises_case() {
        assert_eq!(read(&A.to_uppercase()).unwrap(), vec![A.to_string()]);
    }

    #[test]
    fn restores_an_elided_line() {
        let elided = &B[ELIDED_DROP..];
        assert_eq!(
            read(&format!("{A}\n{elided}\n")).unwrap(),
            vec![A.to_string(), B.to_string()]
        );
    }

    /// An operator who elides "after a while" produces mixed input, so mixed
    /// input is legal rather than an error.
    #[test]
    fn accepts_mixed_full_and_elided() {
        let got = read(&format!("{A}\n{B}\n{}\n", &B[ELIDED_DROP..])).unwrap();
        assert_eq!(got.len(), 3);
        assert!(got.iter().all(|s| s.starts_with("mt1")));
        assert_eq!(got[2], B);
    }

    #[test]
    fn refuses_all_elided_and_names_what_is_missing() {
        let r = read(&format!("{}\n{}\n", &A[ELIDED_DROP..], &B[ELIDED_DROP..])).unwrap_err();
        assert!(
            r.verdict.contains("all 2 lines are elided"),
            "got: {}",
            r.verdict
        );
        assert!(
            r.remedy.as_deref().unwrap().contains("8 characters"),
            "the refusal must name the shape of what is needed"
        );
    }
}
