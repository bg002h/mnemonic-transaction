//! §8's three-part refusal format, and the distinction that keeps it readable.
//!
//! ```text
//! mt encode: REFUSED — §8.2b, fee rate 31,250 sat/vB exceeds 25,000.
//!
//!   <mechanism: what was read, what the rule is, and WHY the rule exists>
//!
//!   <what to do — omitted entirely when there is nothing>
//! ```
//!
//! **`REFUSED` is reserved for refusals.** Warnings use `WARNING:` and carry no
//! `§`-reference in their first line. `mt` warns far more often than it refuses,
//! and a format that blurred the two would teach operators to skim both — so a
//! reader scanning `stderr` can tell at a glance which output stopped the run.
//!
//! The verdict line is **machine-parseable** — stable prefix, `§`-reference, and
//! the number that caused it — so a test asserts on the reference and the value
//! without matching prose that will be reworded.

use std::fmt;

/// A refusal, in the ruled shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The verb that refused, e.g. `encode`.
    pub verb: String,
    /// The spec reference, e.g. `§8.2b`.
    pub section: String,
    /// One line, naming the number that caused it.
    pub verdict: String,
    /// What was read, what the rule is, and why it exists.
    pub mechanism: String,
    /// What to do. `None` when there is nothing — padding a refusal with advice
    /// that does not apply is worse than saying nothing.
    pub remedy: Option<String>,
    /// Preformatted lines printed after the remedy, **never re-wrapped**.
    ///
    /// For content whose LAYOUT carries meaning: a ranked suspect list is a
    /// table, and reflowing it as prose strips the column alignment that makes
    /// it scannable. `wrap` splits on whitespace, so it silently collapsed
    /// `chunk   4   3 of 4` into `chunk 4 3 of 4`.
    pub verbatim: Option<String>,
}

impl Refusal {
    /// Build a refusal.
    pub fn new(
        verb: impl Into<String>,
        section: impl Into<String>,
        verdict: impl Into<String>,
        mechanism: impl Into<String>,
    ) -> Self {
        Self {
            verb: verb.into(),
            section: section.into(),
            verdict: verdict.into(),
            mechanism: mechanism.into(),
            remedy: None,
            verbatim: None,
        }
    }

    /// Attach the remedy.
    #[must_use]
    pub fn with_remedy(mut self, remedy: impl Into<String>) -> Self {
        self.remedy = Some(remedy.into());
        self
    }

    /// Attach preformatted lines, printed as-is.
    #[must_use]
    pub fn with_verbatim(mut self, block: impl Into<String>) -> Self {
        self.verbatim = Some(block.into());
        self
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "mt {}: REFUSED — {}, {}",
            self.verb, self.section, self.verdict
        )?;
        writeln!(f)?;
        for line in wrap(&self.mechanism, 68) {
            writeln!(f, "  {line}")?;
        }
        if let Some(r) = &self.remedy {
            writeln!(f)?;
            for line in wrap(r, 68) {
                writeln!(f, "  {line}")?;
            }
        }
        if let Some(v) = &self.verbatim {
            writeln!(f)?;
            for line in v.lines() {
                writeln!(f, "  {line}")?;
            }
        }
        Ok(())
    }
}

/// A warning. Deliberately a different type from [`Refusal`], so a warning
/// cannot accidentally be printed in the shape that means "this stopped".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// One line.
    pub headline: String,
    /// The mechanism, as with a refusal: state it rather than the caution.
    pub body: String,
}

impl Warning {
    /// Build a warning.
    pub fn new(headline: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            body: body.into(),
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "WARNING: {}", self.headline)?;
        writeln!(f)?;
        for line in wrap(&self.body, 68) {
            writeln!(f, "  {line}")?;
        }
        Ok(())
    }
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + word.len() > width {
                out.push(core::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_line_is_machine_parseable() {
        let r = Refusal::new(
            "encode",
            "§8.2b",
            "fee rate 31,250 sat/vB exceeds 25,000",
            "why",
        );
        let first = r.to_string().lines().next().unwrap().to_string();
        assert!(first.starts_with("mt encode: REFUSED — §8.2b,"));
        assert!(
            first.contains("31,250"),
            "the verdict line must name the number that caused it"
        );
    }

    /// A reader scanning stderr must be able to tell, at a glance, which output
    /// stopped the run.
    #[test]
    fn warnings_are_not_shaped_like_refusals() {
        let w = Warning::new("no bitcoind reachable", "these checks did not run");
        let s = w.to_string();
        assert!(s.starts_with("WARNING:"));
        assert!(
            !s.contains("REFUSED"),
            "a warning must never read as a refusal"
        );
        assert!(
            !s.lines().next().unwrap().contains('§'),
            "a warning's first line carries no section reference"
        );
    }

    #[test]
    fn a_refusal_with_nothing_to_suggest_says_nothing() {
        let r = Refusal::new("encode", "§8.1", "not finalized", "mechanism");
        assert!(
            !r.to_string().contains("Consider"),
            "an absent remedy must not be padded with advice"
        );
    }
}
