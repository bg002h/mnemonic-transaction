//! §8.4: **read two FIELDS and ask one question.**
//!
//! Fields are certain; scripts are somebody else's job. `mt` never hands the
//! transaction to a node for validation and never evaluates the sending
//! wallet's descriptor.
//!
//! | input | source | certain? |
//! | --- | --- | --- |
//! | `nLockTime` | transaction field | **yes** |
//! | `nSequence`, per input | transaction field | **yes** |
//! | current height / MTP | `bitcoind` if reachable, else absent | yes when present |
//!
//! **This module did not exist until a spec-first review looked for it.** The
//! whole of `mt`'s locktime handling was three match arms rendering
//! `block {n}` — so both of the failures §8.4 was written to close were present
//! in the shipped binary, and the section names each of them:
//!
//! 1. **A permanent falsehood.** With no threshold branch, `nLockTime =
//!    1800000000` reported `block 1800000000` — a block some thirty thousand
//!    years out, for a plate that actually unlocks in 2027. A holder could
//!    reasonably read that as "never" and discard it.
//! 2. **False reassurance, the worst failure available here.** With no
//!    `nSequence` rule, a transaction whose every input is final — one anybody
//!    can broadcast **today** — reported `LOCKED TO BLOCK 96`.
//!
//! **`mt` states the two facts and stops.** Operator ruling: *"'may be
//! immediately spendable' is accurate but incomplete. Just say whether the
//! transaction is locked to block x and current height is y."* Two numbers side
//! by side let the operator see which case they are in; a verdict cannot
//! distinguish a lock that has passed from one that was never enforced from one
//! still years away, and all three want different responses.

use bitcoin::Transaction;

/// Below this, `nLockTime` is a **block height**; at or above it, a **Unix
/// timestamp**. From `bitcoin`'s own `LOCK_TIME_THRESHOLD`.
pub const LOCK_TIME_THRESHOLD: u32 = 500_000_000;

/// The embedded reference pair, and **the only thing the estimate uses.**
///
/// Operator ruling: *"Use embedded timestamp above only ever. It's essentially
/// constant and reasonably reliable as an estimate."* An earlier draft branched
/// on whether a node was reachable; that was removed as too complex, and the
/// simplification is worth more than the accuracy it costs:
///
/// - **The answer is deterministic.** Two runs of `mt`, on any two machines,
///   with or without a node, produce the same engraved year for the same
///   transaction. Branching would make a permanent number on steel depend on the
///   operator's network.
/// - **The accuracy difference is immaterial** at a granularity of one year.
/// - It removes a whole class of question — what if the node disagrees, what if
///   it is syncing, what if it is on another chain.
pub const MT_REF_HEIGHT: u32 = 963_759;

/// The reference tip's **median-time-past**, not its header `nTime`.
///
/// MTP is monotonic and consensus-enforced; a header stamp is only loosely
/// constrained and may run up to two hours fast. At capture the tip's `nTime`
/// was `1787509876`, **36 minutes ahead** of its MTP — small here, unbounded in
/// general, and baking that slack into a decades-long projection would be
/// permanent.
///
/// Provenance: block 963,759,
/// `00000000000000000000b7060d74b6540e3b2accc9cb50f2a0d428b55911a455`.
pub const MT_REF_TIME: u64 = 1_787_507_701;

/// What the two fields say, once read together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lock {
    /// `nLockTime == 0`: there is nothing to enforce.
    None,
    /// A non-zero `nLockTime` that consensus will **ignore**, because every
    /// input is final.
    ///
    /// **This is the dangerous case**, and it is a field read rather than a
    /// script read, so it stays in scope.
    NotEnforced(u32),
    /// Enforced, and below the threshold: a block height.
    Height(u32),
    /// Enforced, and at or above the threshold: a Unix timestamp.
    Time(u32),
}

/// Read `nLockTime` and every input's `nSequence`, and say what they mean.
///
/// **`nSequence` is not optional.** `nLockTime` is enforced only when at least
/// one input has `nSequence != 0xFFFFFFFF`; a transaction with every input final
/// ignores its locktime entirely.
pub fn read(tx: &Transaction) -> Lock {
    let lt = tx.lock_time.to_consensus_u32();
    if lt == 0 {
        return Lock::None;
    }
    let enforced = tx.input.iter().any(|i| i.sequence.0 != 0xFFFF_FFFF);
    if !enforced {
        return Lock::NotEnforced(lt);
    }
    if lt < LOCK_TIME_THRESHOLD {
        Lock::Height(lt)
    } else {
        Lock::Time(lt)
    }
}

/// What the chain says, when a node is reachable. **Compare like with like:** a
/// height against the chain height, a timestamp against the chain's
/// median-time-past.
#[derive(Debug, Clone, Copy, Default)]
pub struct Chain {
    /// Current block height.
    pub height: Option<u64>,
    /// Current median-time-past — the monotonic, consensus-enforced figure,
    /// never the loosely-constrained header stamp.
    pub mtp: Option<u64>,
}

impl Lock {
    /// The **`stderr` report** row, in §8.4's own spellings.
    ///
    /// Five of them, and no sixth: `mt` may not invent one.
    pub fn report_row(&self, chain: Chain) -> String {
        match *self {
            Lock::None => match chain.height {
                Some(h) => format!("NO TIMELOCK                      current height {h}"),
                None => {
                    "NO TIMELOCK                      current height unknown (no node)".to_string()
                }
            },
            // The report says NOT ENFORCED and names the value; the LEGEND says
            // NO TIMELOCK. §8.4 gives this state both spellings, on two
            // different surfaces, and never says so -- see `legend`.
            Lock::NotEnforced(n) => {
                format!("nLockTime {n} present but NOT ENFORCED (all inputs final)")
            }
            Lock::Height(n) => match chain.height {
                Some(h) => format!("LOCKED TO BLOCK {n}          current height {h}"),
                None => format!("LOCKED TO BLOCK {n}          current height unknown (no node)"),
            },
            Lock::Time(t) => match chain.mtp {
                Some(m) => format!(
                    "LOCKED UNTIL {}   current MTP {}",
                    iso_minutes(u64::from(t)),
                    iso_minutes(m)
                ),
                None => format!(
                    "LOCKED UNTIL {}   current MTP unknown (no node)",
                    iso_minutes(u64::from(t))
                ),
            },
        }
    }

    /// The **engraved legend** form, which is a different surface and a
    /// different set of words.
    ///
    /// **The height is the fact; the season is the courtesy.** A height alone is
    /// meaningless to a human and a season alone is unverifiable, so the plate
    /// carries both and a reader can always fall back to the number.
    ///
    /// `NO TIMELOCK` is reserved for `nLockTime = 0` **or all inputs final** —
    /// precisely true about the fields `mt` read, and silent about scripts it
    /// did not. Engraving `IMMEDIATELY SPENDABLE` instead would be a positive
    /// claim about spendability that `mt` cannot substantiate: a BIP-68 relative
    /// timelock lives in `OP_CSV` inside the witness script, and a
    /// relative-locked spend has `nLockTime = 0`.
    pub fn legend(&self) -> String {
        match *self {
            // Both states, one spelling, 11 characters, normative everywhere.
            Lock::None | Lock::NotEnforced(_) => "NO TIMELOCK".to_string(),
            Lock::Height(n) => match season_year(n) {
                // A lock already behind this build's reference gets the height
                // and NO estimate: there is no future date to name, and a past
                // year on steel is worse than silence.
                None => format!("LOCKED TO BLOCK {n}"),
                Some((season, year)) => format!("LOCKED TO BLOCK {n} ~{season} {year}"),
            },
            Lock::Time(t) => format!("LOCKED UNTIL {}", iso_minutes(u64::from(t))),
        }
    }

    /// §8.4's negative-subtraction warning: the lock height passed before this
    /// build's reference, so there is no future date to estimate.
    pub fn below_reference_warning(&self) -> Option<crate::refusal::Warning> {
        let Lock::Height(n) = *self else { return None };
        if n >= MT_REF_HEIGHT {
            return None;
        }
        Some(crate::refusal::Warning::new(
            format!("nLockTime {n} is BELOW this build's reference height {MT_REF_HEIGHT}."),
            "This transaction is not meaningfully time-locked -- its lock height \
             passed before mt was built. Treat it as spendable now.",
        ))
    }
}

/// Project a height to a season and year, using the embedded pair and nothing
/// else.
///
///     estimated unlock = MT_REF_TIME + (target − MT_REF_HEIGHT) × 600 s
///
/// `None` when the target is at or below the reference — a negative subtraction
/// has no future date to name.
///
/// **Stated to the YEAR, deliberately.** Ten minutes is a target, not a rate;
/// the reference pair ages; and the number is engraved, so a projection
/// presented as a fact is the mistake §9 refuses for fiat figures. The `~` marks
/// the whole estimate as approximate.
pub fn season_year(target: u32) -> Option<(&'static str, i64)> {
    if target < MT_REF_HEIGHT {
        return None;
    }
    let secs = MT_REF_TIME + u64::from(target - MT_REF_HEIGHT) * 600;
    let (y, m, _) = civil_from_unix(secs);
    // NORTHERN-HEMISPHERE meteorological quarters, by ruling. A reader in Sydney
    // is wrong by about six months -- bounded and small, because the mandatory
    // height sits beside it and is unambiguous everywhere.
    let season = match m {
        3..=5 => "SPRING",
        6..=8 => "SUMMER",
        9..=11 => "FALL",
        _ => "WINTER",
    };
    Some((season, y))
}

/// `YYYY-MM-DDTHH:MMZ`.
pub fn iso_minutes(secs: u64) -> String {
    let (y, m, d) = civil_from_unix(secs);
    let sod = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}Z",
        sod / 3600,
        (sod % 3600) / 60
    )
}

/// Civil date from a Unix timestamp — Howard Hinnant's `civil_from_days`.
///
/// Written out rather than pulled in: a date library is a dependency, an
/// engraved year is forever, and this is twelve lines with a pinned test.
fn civil_from_unix(secs: u64) -> (i64, u32, u32) {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference pair must render as the timestamp its comment claims.
    /// **A constant whose comment states a date acquires a way to be wrong for
    /// free**, and this one is engraved-adjacent.
    #[test]
    fn the_reference_pair_renders_as_its_documented_date() {
        assert_eq!(iso_minutes(MT_REF_TIME), "2026-08-23T17:55Z");
    }

    #[test]
    fn civil_dates_match_known_values() {
        assert_eq!(iso_minutes(0), "1970-01-01T00:00Z");
        assert_eq!(iso_minutes(1_000_000_000), "2001-09-09T01:46Z");
        assert_eq!(iso_minutes(1_800_000_000), "2027-01-15T08:00Z");
    }

    /// §8.4's worked example, block 1,383,520 — and **the spec's own answer for
    /// it is off by one season under the spec's own algorithm.**
    ///
    /// §5 and §8.4 both render it `~FALL 2034`. Computing it as ruled —
    /// `MT_REF_TIME + (1_383_520 − MT_REF_HEIGHT) × 600` — gives
    /// **2034-08-16**, which is SUMMER under the northern meteorological
    /// quarters §8.4 also rules. The projection lands **15 days** from the
    /// September boundary, and §8.4 anticipates exactly this: *"The exception is
    /// a projection landing near a season boundary, which can tip"*, against a
    /// measured drift of *"+16 to −34 days"* over this very span.
    ///
    /// So the algorithm is pinned here and the example is filed as a spec
    /// defect. **This is the case the spec built its own escape hatch for** —
    /// the height is the fact and the season is the courtesy, which is why the
    /// height is mandatory and the estimate carries a `~`.
    #[test]
    fn the_worked_example_projects_to_summer_not_the_spec_s_fall() {
        assert_eq!(season_year(1_383_520), Some(("SUMMER", 2034)));
        // The projection itself, so a future reader can check the seasons table
        // rather than re-deriving the arithmetic.
        let secs = MT_REF_TIME + u64::from(1_383_520u32 - MT_REF_HEIGHT) * 600;
        assert_eq!(iso_minutes(secs), "2034-08-16T18:05Z");
    }

    /// The boundary itself, both sides, so the season table cannot drift.
    #[test]
    fn the_season_boundaries_are_the_meteorological_quarters() {
        for (iso, want) in [
            ("2034-03-01", "SPRING"),
            ("2034-05-31", "SPRING"),
            ("2034-06-01", "SUMMER"),
            ("2034-08-31", "SUMMER"),
            ("2034-09-01", "FALL"),
            ("2034-11-30", "FALL"),
            ("2034-12-01", "WINTER"),
            ("2035-02-28", "WINTER"),
        ] {
            // Find a height whose projection lands on that civil day.
            let target = days_to_unix(iso);
            // CEILING division: truncating lands the projection a few minutes
            // BEFORE midnight, i.e. on the previous day -- which for a boundary
            // date is the previous SEASON, and the test would be asserting the
            // wrong side of the line it exists to pin.
            let h = MT_REF_HEIGHT + ((target - MT_REF_TIME).div_ceil(600)) as u32;
            let (season, _) = season_year(h).unwrap();
            assert_eq!(season, want, "{iso} projected to {season}");
        }
    }

    /// `YYYY-MM-DD` at midnight UTC, for the boundary table above.
    fn days_to_unix(iso: &str) -> u64 {
        let (y, rest) = iso.split_once('-').unwrap();
        let (m, d) = rest.split_once('-').unwrap();
        let (y, m, d): (i64, i64, i64) =
            (y.parse().unwrap(), m.parse().unwrap(), d.parse().unwrap());
        // Inverse of civil_from_unix: Hinnant's days_from_civil.
        let y2 = if m <= 2 { y - 1 } else { y };
        let era = y2.div_euclid(400);
        let yoe = y2 - era * 400;
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        ((era * 146_097 + doe - 719_468) * 86_400) as u64
    }

    #[test]
    fn a_height_below_the_reference_has_no_estimate() {
        assert_eq!(season_year(MT_REF_HEIGHT - 1), None);
        assert_eq!(season_year(900_000), None);
    }

    /// The threshold is the difference between a block number and a date, and
    /// getting it backwards engraves a falsehood that outlives the plate.
    #[test]
    fn the_threshold_decides_height_versus_timestamp() {
        assert_eq!(LOCK_TIME_THRESHOLD, 500_000_000);
    }
}
