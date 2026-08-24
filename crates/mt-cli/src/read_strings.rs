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
use mt_codec::string_layer::pipeline;

/// The `mt1` prefix plus the invariant symbols an elided line omits.
const ELIDED_DROP: usize = 3 + INVARIANT_PREFIX_SYMBOLS;

/// Split raw input into candidate strings, normalise, and restore elision.
/// A character `mt` transliterated before the codec ever saw the string.
///
/// **Kept separate from BCH's corrections, and reported separately.** They are
/// different events: BCH repairs DAMAGE, and spends one of four repairs doing
/// it; this replaces a character that CANNOT OCCUR in a valid string, and costs
/// nothing. Folding them together made the margin report say `read 6` about a
/// plate where the operator had typed `b` — naming a symbol that was never
/// engraved and never typed, which is precisely what a before-and-after exists
/// to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transliteration {
    /// 1-based position in the whole string.
    pub position: usize,
    /// What the operator typed.
    pub from: char,
    /// What mt read it as, chosen because the result decoded.
    pub to: char,
}

/// What `read` produced, and what it had to change to get there.
#[derive(Debug, Clone, Default)]
pub struct ReadStrings {
    /// The strings, restored and transliterated.
    pub strings: Vec<String>,
    /// Every character mt replaced, in input order.
    pub notes: Vec<Transliteration>,
    /// Did every transliterated string then decode with **zero** BCH
    /// corrections?
    ///
    /// **The notice claimed the reading was free and could not know it.** Where
    /// a confusable has two candidates and NEITHER is what was engraved — the
    /// character was simply damaged into a `b` — mt picks one, BCH repairs it,
    /// and repairs spent on mt's own guess are repairs the operator's real
    /// damage no longer has. Saying "this cost you nothing" there is false in
    /// the direction that matters.
    pub free: bool,
}

pub fn read(raw: &str, verb: &str) -> Result<ReadStrings, Refusal> {
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
            verb,
            "§1.1e",
            "no strings found in the input",
            "mt splits input on line breaks, and on each `mt1` prefix within a \
             line. Nothing in this input looked like either.",
        )
        .with_remedy("Check the file, or pipe the strings in with `mt decode < file`."));
    }

    // 2b. THE SEPARATOR, BEFORE ANYTHING READS THE PREFIX. `mtl…` and `mti…` are
    //     the `1` of `mt1` misread, and every step below asks "does this start
    //     with mt1" — so a candidate repaired later is a candidate that was
    //     already mistaken for an elided line and had a prefix prepended to it.
    //     This one substitution has to happen first.
    //     **KEPT ONLY IF IT DECODES**, exactly like every other autocorrect.
    //     The unguarded version silently corrupted a legitimate input: an
    //     ELIDED line carries bare bech32 symbols, and `m`, `t` and `l` are all
    //     in that alphabet — so roughly one elided line in 32,768 begins `mtl`
    //     by chance and was rewritten into a full-looking string with a wrong
    //     prefix. About one set in four thousand, on the RECOVERY path, silently.
    //     Speculate, verify, and discard the speculation if it does not decode.
    for c in &mut candidates {
        if c.len() > 3 && (c.starts_with("mtl") || c.starts_with("mti")) {
            let mut fixed = c.clone();
            fixed.replace_range(2..3, "1");
            if pipeline::decode_chunk(&fixed, None).is_ok() {
                *c = fixed;
            }
        }
    }

    // 3. Restore elided lines from the set's full string (§3b).
    let restored = restore_elided(candidates, verb)?;

    // §1.1e step 4: only now, and only for strings that did NOT parse.
    let mut notes = Vec::new();
    let mut free = true;
    let strings = restored
        .into_iter()
        .map(|s| {
            if pipeline::decode_chunk(&s, None).is_ok() {
                return s; // step 3: it parsed as written. STOP.
            }
            // **THE CANDIDATE THAT COSTS BCH THE LEAST, not the first that
            // decodes.** Both readings of `b` may decode — one because it is
            // right, the other because BCH REPAIRED THE BAD GUESS, spending
            // repairs the operator's own damage will need later. Taking the
            // first found meant mt guessed `6`, BCH corrected both positions
            // back off it, and the notice still claimed the reading was free.
            //
            // Fewest corrections is the honest tie-break: a candidate needing
            // zero is the character that was actually engraved.
            match positional_autocorrect(&s)
                .into_iter()
                .filter_map(|c| {
                    pipeline::decode_chunk(&c, None)
                        .ok()
                        .map(|d| (d.corrected, c))
                })
                .min_by_key(|(n, _)| *n)
                .map(|(_, c)| c)
            {
                Some(fixed) => {
                    if pipeline::decode_chunk(&fixed, None).is_ok_and(|d| d.corrected > 0) {
                        free = false;
                    }
                    for (i, (a, b)) in s.chars().zip(fixed.chars()).enumerate() {
                        if a != b {
                            notes.push(Transliteration {
                                position: i + 1,
                                from: a,
                                to: b,
                            });
                        }
                    }
                    fixed
                }
                None => s,
            }
        })
        .collect();
    Ok(ReadStrings {
        strings,
        notes,
        free,
    })
}

/// §1.1e's **positional autocorrect** — a repair attempted on FAILURE, never a
/// preprocessing pass.
///
/// The bech32 alphabet deliberately omits `1`, `b`, `i` and `o` **because they
/// are confusable when engraved**, which is exactly what makes them repairable:
/// past the `mt1` prefix, any of them is a misreading of something else, and
/// there is only one candidate each. At index 2 the reverse holds — that
/// position IS the `1` of `mt1`, so an `l`, `i` or `I` there is the same
/// misreading in the other direction.
///
/// **Never touches a string that already parses.** §1.1e's order is: try the
/// string as written, and only then attempt correction. A preprocessing pass
/// would silently rewrite valid input, and `b` → `6` on a string that was
/// already right changes the payload.
fn positional_autocorrect(s: &str) -> Vec<String> {
    // Each confusable, with EVERY in-alphabet character it could have been.
    // `1`, `b`, `i` and `o` are absent from bech32 precisely because they are
    // confusable when engraved, so any of them on the page is a misreading —
    // but `b` has TWO candidates, and mt's own refusal remedy says so: it lists
    // `b/6` and `8/b`.
    //
    // **Guessing one of the two was wrong in both directions.** Picking `6`
    // reported a symbol that was never engraved and never typed, and a wrong
    // pick is itself an error that burns one of the four BCH repairs. But
    // leaving `b` alone is worse than either: `b` IS NOT IN THE ALPHABET, so the
    // string does not convert to symbols at all and BCH never gets to see it.
    //
    // So mt does not choose — it TRIES BOTH and keeps whichever decodes. The
    // checksum is the arbiter, which is what it is for.
    fn candidates_for(i: usize, c: char) -> &'static [char] {
        match (i, c) {
            // The separator: that position IS the `1` of `mt1`. `I` is
            // unreachable — `read` lowercases before this runs.
            (2, 'l' | 'i') => &['1'],
            (n, '1') if n > 2 => &['l'],
            (n, 'i') if n > 2 => &['l'],
            (n, 'o') if n > 2 => &['0'],
            (n, 'b') if n > 2 => &['6', '8'],
            _ => &[],
        }
    }

    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<Vec<char>> = vec![chars.clone()];
    let mut touched = false;
    for (i, c) in chars.iter().enumerate() {
        let cands = candidates_for(i, *c);
        if cands.is_empty() {
            continue;
        }
        touched = true;
        // Cross-product, bounded: only `b` branches, and a string needing more
        // than a handful of repairs is past t = 4 anyway. The cap keeps a
        // pathological input from exploding the search.
        const MAX: usize = 64;
        let mut next = Vec::with_capacity(out.len() * cands.len());
        for base in &out {
            for &cand in cands {
                if next.len() >= MAX {
                    break;
                }
                let mut v = base.clone();
                v[i] = cand;
                next.push(v);
            }
        }
        out = next;
    }
    if !touched {
        return Vec::new();
    }
    out.into_iter().map(|v| v.into_iter().collect()).collect()
}

/// Could this line be mt1 material — full or elided?
///
/// Only the CHARSET, deliberately. An elided line has no `mt1` prefix and no
/// structure mt can check before restoring it, so the one thing that separates
/// it from a mnemonic or a descriptor is that every character is a bech32
/// symbol. `1`, `b`, `i` and `o` are absent from that charset precisely because
/// they are confusable when engraved, which is what makes it discriminating:
/// English words are full of them.
fn looks_like_mt1_material(s: &str) -> bool {
    const ALPHABET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    !s.is_empty() && s.chars().all(|c| ALPHABET.contains(c))
}

/// §1.1e: **the expected length comes from the strings themselves — the MODAL
/// length across the set.**
///
/// Every chunk with `index < count − 1` carries the same payload length, so the
/// most common string length in a set *is* the expected one, and any string
/// differing from it is the suspect. The **final** chunk is shorter whenever the
/// payload does not divide evenly, so exactly one string may differ — and its
/// length cannot be checked until the set is complete, which is the thing this
/// check exists to gate.
///
/// **The obvious derivation is circular**, which is why the spec states the
/// modal rule outright: per-chunk length follows from `bytes_per_chunk`, which
/// follows from the total payload length, which is not known until every chunk
/// is assembled.
///
/// **Without this, a DROPPED CHARACTER reported a MISSING PLATE.** An omission
/// shifts every symbol after it, so the string fails its checksum, decodes to
/// nothing, and the set reports `chunk 3 of 9 is missing` — *an accusation about
/// the operator's steel*, sending them to hunt for a plate that is sitting in
/// front of them. BCH repairs substitutions; it cannot repair a length.
pub fn length_report(strings: &[String], failed: &[usize], verb: &str) -> Option<Refusal> {
    if strings.len() < 3 || failed.is_empty() {
        // With one or two strings there is no mode to speak of, and with nothing
        // failing there is nothing to explain.
        return None;
    }
    let mut counts: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for s in strings {
        *counts.entry(s.chars().count()).or_default() += 1;
    }
    let (&modal, &n) = counts
        .iter()
        .max_by_key(|&(_, n)| *n)
        .expect("strings is non-empty");
    if n < 2 {
        // No length occurs twice, so there is no mode and nothing to compare
        // against. Silence beats a guess.
        return None;
    }

    // **LENGTH ALONE CANNOT DECIDE THIS, and that is why the check is consulted
    // on FAILURE rather than run up front.** A set whose payload does not divide
    // evenly has one legitimately SHORT final chunk, and a dropped character
    // produces a short string too — indistinguishable by length. The
    // discriminator is that the legitimate short chunk PARSES: its checksum
    // holds, so it never reaches this path. §1.1e's own order says the same
    // thing — try the string as written, and only then attempt repair.
    let suspect: Vec<(usize, usize)> = failed
        .iter()
        .filter_map(|&pos| {
            let len = strings.get(pos - 1)?.chars().count();
            (len != modal).then_some((pos, len))
        })
        .collect();
    if suspect.is_empty() {
        return None;
    }

    // **IS IT AMBIGUOUS, OR CAN mt TELL?** Exactly one string per UNEVEN set is
    // short by design, so a single short string could be a miscount or could be
    // the final chunk — and guessing either way is a false statement about the
    // operator's steel.
    //
    // The readable strings carry their own `index` and `count`. **If one of them
    // IS the final chunk** (`index == count − 1`) **and is the modal length**,
    // then this set's payload divides evenly, no chunk is short, and a short
    // string is a genuine miscount. If no readable string is the last one, the
    // short unreadable one may well be it.
    let final_chunk_seen_at_modal_length = strings.iter().any(|x| {
        pipeline::decode_chunk(x, None)
            .is_ok_and(|c| c.header.index + 1 == c.header.count && x.chars().count() == modal)
    });
    // **PER SUSPECT, not per set.** Requiring a single suspect meant a SECOND
    // failure elsewhere flipped the legitimately-short final chunk back into the
    // accusing branch — mt inventing a character deficit on the one plate that
    // is short by design. The two events are independent: a missing character on
    // one plate says nothing about another.
    // **NO AGGREGATE CONDITION IN A PER-SUSPECT TEST.** The first attempt at this
    // extracted a closure and left `short_count == 1` inside it — textually the
    // same whole-set predicate it replaced — so a SECOND short string anywhere
    // flipped every suspect back into the accusing branch, and mt again told the
    // operator that the legitimately-short final chunk was missing 6 characters.
    // Moving code without moving its assumptions.
    //
    // The question per suspect is simply: could THIS string be the final chunk?
    // It could, if it is short and no readable string has already claimed that
    // role at modal length. Two short strings means mt can tell even less, not
    // more — so hedging on both is right, and accusing either is not.
    let is_ambiguous = |len: usize| len < modal && !final_chunk_seen_at_modal_length;

    let mut list = String::new();
    {
        use core::fmt::Write as _;
        for (n, len) in &suspect {
            if is_ambiguous(*len) {
                let _ = writeln!(
                    list,
                    "string {n}: {len} characters, where this set's usual length is {modal}"
                );
                continue;
            }
            let (word, delta) = if *len < modal {
                ("MISSING", modal - len)
            } else {
                ("EXTRA", len - modal)
            };
            let plural = if delta == 1 { " is" } else { "s are" };
            let _ = writeln!(
                list,
                "string {n}: {len} characters (expected {modal}) — {delta} character{plural} {word}"
            );
        }
    }

    let ambiguous = suspect.len() == 1 && is_ambiguous(suspect[0].1);

    Some(
        Refusal::new(
            verb,
            "§1.1e",
            if ambiguous {
                format!(
                    "string {} did not read, and it is the only one shorter than \
                 {modal} — which is also what a final chunk looks like",
                    suspect[0].0
                )
            } else {
                format!(
                    "{} string{} the wrong length for this set (most are {modal})",
                    suspect.len(),
                    if suspect.len() == 1 { " is" } else { "s are" }
                )
            },
            if ambiguous {
                "This string is shorter than the others, AND every set has one \
             legitimately short chunk — the last one, whenever the payload does \
             not divide evenly. So mt cannot tell whether characters are missing \
             from this string or whether this simply IS the final chunk, and it \
             will not accuse your steel of a miscount it cannot demonstrate. \
             What it can tell you: the string did not read, and a length error \
             is not something BCH repairs — it corrects substitutions, and an \
             omission shifts every symbol after it."
            } else {
                "A character is MISSING or EXTRA, not wrong. BCH repairs SUBSTITUTIONS \
             — up to 4 per string — but an omission or an insertion shifts every \
             symbol after it and cannot be corrected. mt stops here rather than \
             decoding, because a length error reports as a MISSING PLATE once it \
             reaches the codec, and that sends you looking for steel that is not \
             lost.\n\
             \n\
             The expected length is the most common one in this set: every string \
             but the last carries the same payload, so exactly one may be shorter."
            },
        )
        .with_remedy("Re-read these from the plate, counting characters:")
        .with_verbatim(list),
    )
}

/// Restore `--elide-prefix` output.
///
/// Detection needs no flag: a line beginning `mt1` is full, anything else is
/// elided. **Mixed input is legal** — an operator who elides "after a while"
/// produces exactly that.
fn restore_elided(candidates: Vec<String>, verb: &str) -> Result<Vec<String>, Refusal> {
    // A line that BEGINS `mt` but is not `mt1` is a misread separator that the
    // repair could not confirm — because the line carries a second defect too.
    // It is NOT an elided line, and treating it as one prepends 11 characters
    // and then accuses the plate of having "10 characters EXTRA" when it is in
    // fact one SHORT. Recognise the shape and say so.
    // ...but ONLY when its LENGTH says it is a full string. An elided line is
    // bare bech32 symbols, and `m`, `t` and `l` are all in that alphabet, so
    // roughly one in 32,768 begins `mtl` legitimately — and refusing THAT is
    // the same near-miss defect in the other direction. A full string is
    // ELIDED_DROP characters longer than an elided one, which settles it.
    // **THE REFERENCE CANNOT COME ONLY FROM `mt1` LINES.** With
    // `--elide-prefix` exactly ONE line is full — and if that is the line whose
    // separator was misread, there is no `mt1` candidate left to measure
    // against, `full_len` is None, and the guard below can never fire. That is
    // the whole of the elided variant, and it survived the previous fix.
    //
    // So: prefer a real full line when one exists; otherwise derive what a full
    // line WOULD be from the elided ones, which are exactly ELIDED_DROP shorter.
    let full_len = candidates
        .iter()
        .filter(|c| c.starts_with("mt1"))
        .map(|c| c.chars().count())
        .max()
        .or_else(|| {
            let others: Vec<usize> = candidates
                .iter()
                .filter(|c| !(c.starts_with("mtl") || c.starts_with("mti")))
                .map(|c| c.chars().count())
                .collect();
            // The modal elided length, plus what elision removed.
            let mut counts: std::collections::BTreeMap<usize, usize> =
                std::collections::BTreeMap::new();
            for n in &others {
                *counts.entry(*n).or_default() += 1;
            }
            counts
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(len, _)| len + ELIDED_DROP)
        });
    // **NEARER FULL THAN ELIDED, not exactly full.** Requiring an exact match
    // meant the guard fired only when the second defect was a same-length
    // SUBSTITUTION — and the case it was written for is a DROPPED CHARACTER,
    // which changes the length, so the string fell through to the identical
    // pre-fix misdiagnosis. A full string and an elided one differ by
    // ELIDED_DROP characters; anything within half that of full is full.
    let looks_full = |n: usize| {
        full_len.is_some_and(|f| {
            let midpoint = f.saturating_sub(ELIDED_DROP / 2);
            n >= midpoint
        })
    };
    if let Some(bad) = candidates.iter().find(|c| {
        c.len() > 3
            && (c.starts_with("mtl") || c.starts_with("mti"))
            && looks_full(c.chars().count())
    }) {
        return Err(Refusal::new(
            verb,
            "§1.1e",
            format!(
                "a string begins `{}` — the `1` of `mt1` misread, and something \
                 else wrong too",
                &bad[..3]
            ),
            "mt tried the obvious repair (that character is the `1` of the `mt1` \
             prefix) and the string still did not read, so there is a SECOND \
             defect on the same plate. mt stops rather than treating it as an \
             ELIDED line — which is what a line not beginning `mt1` normally is, \
             and which would prepend 11 characters and then report a length error \
             about a plate that is one character short.",
        )
        .with_remedy(
            "Re-read that plate. Its first three characters are `mt1` — an `m`, a \
             `t`, and the DIGIT one.",
        ));
    }

    let full = candidates.iter().find(|s| s.starts_with("mt1"));

    let Some(full) = full else {
        // **IS THIS AN mt1 SET AT ALL?** Asked FIRST, because the answer decides
        // whether "no prefix to restore" is help or nonsense. A BIP-39 mnemonic,
        // an `md1` string or any text file has no `mt1` line either — and the
        // elision refusal then tells the operator to go and find 8 characters of
        // a set that does not exist. The bech32 charset separates the cases for
        // free: an elided mt1 line is 5-bit symbols and nothing else.
        if !candidates.iter().any(|c| looks_like_mt1_material(c)) {
            return Err(Refusal::new(
                verb,
                "§1.1e",
                format!(
                    "this input is not an mt1 set ({} line(s), none of them mt1)",
                    candidates.len()
                ),
                "mt reads mt1 strings — the codex32 form mt encode produces. Every \
                 line here contains characters outside the bech32 alphabet, so none \
                 of them is an mt1 string, elided or otherwise.\n\
                 \n\
                 If you have a MNEMONIC, a DESCRIPTOR, an xpub, or an md1/mk1 \
                 string: those belong to other tools in this family, not to mt. mt \
                 engraves signed TRANSACTIONS.",
            )
            .with_remedy("If you meant to encode a transaction, that is `mt encode < tx.psbt`."));
        }
        return Err(Refusal::new(
            verb,
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
            verb,
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
        let got = read(&format!("{A}\n{B}\n"), "decode").unwrap().strings;
        assert_eq!(got, vec![A.to_string(), B.to_string()]);
    }

    /// The case that made "strip first" unbuildable: an operator copies several
    /// strings out of a terminal and they arrive as one line.
    #[test]
    fn splits_a_single_line_blob_at_each_prefix() {
        let got = read(&format!("{A}{B}"), "decode").unwrap().strings;
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
        assert_eq!(
            read(&grouped, "decode").unwrap().strings,
            vec![A.to_string()]
        );
    }

    #[test]
    fn normalises_case() {
        assert_eq!(
            read(&A.to_uppercase(), "decode").unwrap().strings,
            vec![A.to_string()]
        );
    }

    #[test]
    fn restores_an_elided_line() {
        let elided = &B[ELIDED_DROP..];
        assert_eq!(
            read(&format!("{A}\n{elided}\n"), "decode").unwrap().strings,
            vec![A.to_string(), B.to_string()]
        );
    }

    /// An operator who elides "after a while" produces mixed input, so mixed
    /// input is legal rather than an error.
    #[test]
    fn accepts_mixed_full_and_elided() {
        let got = read(&format!("{A}\n{B}\n{}\n", &B[ELIDED_DROP..]), "decode")
            .unwrap()
            .strings;
        assert_eq!(got.len(), 3);
        assert!(got.iter().all(|s| s.starts_with("mt1")));
        assert_eq!(got[2], B);
    }

    #[test]
    fn refuses_all_elided_and_names_what_is_missing() {
        let r = read(
            &format!("{}\n{}\n", &A[ELIDED_DROP..], &B[ELIDED_DROP..]),
            "decode",
        )
        .unwrap_err();
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
