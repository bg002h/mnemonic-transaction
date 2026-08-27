//! `mt decode` and `mt verify`.
//!
//! The recovery path. Most of these assert on things that only matter years
//! later, to someone holding steel and no context — which is why they are worth
//! more than they look.

use assert_cmd::Command;
use std::io::Write;

fn mt() -> Command {
    Command::cargo_bin("mt").unwrap()
}

fn corpus() -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn tmp_with(s: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(s.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn strings_of(label: &str) -> Vec<String> {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn raw_of(label: &str) -> String {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == label)
        .unwrap()["raw_hex"]
        .as_str()
        .unwrap()
        .to_string()
}

/// P3's gate: every pinned vector round-trips through `decode`.
#[test]
fn decode_round_trips_every_vector() {
    for label in ["even", "uneven"] {
        let f = tmp_with(&strings_of(label).join("\n"));
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{label}: decode failed");
        assert_eq!(
            String::from_utf8(out.stdout).unwrap().trim(),
            raw_of(label),
            "{label}: decode did not reproduce the transaction"
        );
    }
}

/// §1.1e's split-then-strip. An operator copying several strings out of a
/// terminal produces one line, and "strip whitespace before doing anything else"
/// taken literally makes that unparseable.
#[test]
fn decode_accepts_the_shapes_an_operator_actually_produces() {
    let s = strings_of("even");
    let grouped: Vec<String> = s
        .iter()
        .map(|x| {
            x.chars()
                .collect::<Vec<_>>()
                .chunks(8)
                .map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();

    for (name, body) in [
        ("one per line", s.join("\n")),
        ("single-line blob", s.concat()),
        ("grouped", grouped.join("\n")),
        ("uppercase", s.join("\n").to_uppercase()),
        ("blank lines between", s.join("\n\n")),
        ("trailing whitespace", format!("{}\n  \n", s.join("\n"))),
    ] {
        let f = tmp_with(&body);
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "decode rejected {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap().trim(),
            raw_of("even"),
            "{name}: wrong bytes"
        );
    }
}

/// §1.1a takes strings "in any order".
#[test]
fn decode_is_order_independent() {
    let mut s = strings_of("even");
    s.reverse();
    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

/// The elided form round-trips with no flag on the reading side: detection is
/// unambiguous, because a line beginning `mt1` is full and anything else is not.
#[test]
fn decode_restores_elided_input_without_a_flag() {
    let s = strings_of("even");
    let drop = 3 + 8;
    let mut lines = vec![s[0].clone()];
    lines.extend(s[1..].iter().map(|x| x[drop..].to_string()));

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

/// Mixed full/elided is legal — an operator who elides "after a while" produces
/// exactly that.
#[test]
fn decode_accepts_mixed_full_and_elided() {
    let s = strings_of("even");
    let drop = 3 + 8;
    let mut lines = vec![s[0].clone(), s[1].clone()];
    lines.extend(s[2..].iter().map(|x| x[drop..].to_string()));

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

/// All-elided is refused, naming the shape of what is missing rather than
/// guessing.
#[test]
fn decode_refuses_all_elided_input() {
    let drop = 3 + 8;
    let lines: Vec<String> = strings_of("even")
        .iter()
        .map(|x| x[drop..].to_string())
        .collect();
    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();

    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("REFUSED — §3b"), "got: {err}");
    assert!(
        err.contains("8 characters"),
        "must name what is needed: {err}"
    );
    assert!(out.stdout.is_empty(), "stdout must stay empty on a refusal");
}

/// **stdout stays empty unless every check passes.** The documented pipeline
/// pipes stdout onward; a failure path that still printed hex would let it
/// broadcast a transaction that failed `mt`'s own checks.
#[test]
fn decode_writes_nothing_to_stdout_on_failure() {
    for body in ["not strings at all", "mt1qqqq", ""] {
        let f = tmp_with(body);
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(!out.status.success(), "{body:?} should not succeed");
        assert!(
            out.stdout.is_empty(),
            "{body:?}: stdout carried bytes on a failed decode"
        );
    }
}

/// `decode` prints the report on **stderr**, because it is the verb a recoverer
/// reaches for first — `inspect` is the one designed for them and they have no
/// way to know that. It must not pollute the pipe.
#[test]
fn decode_reports_on_stderr_and_hex_on_stdout() {
    let f = tmp_with(&strings_of("even").join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();

    assert!(stderr.contains("TX  "), "the report did not reach stderr");
    assert!(stderr.contains("mt1 SET"), "the set line is missing");
    assert_eq!(stdout.lines().count(), 1, "stdout must be exactly the hex");
    assert!(stdout.trim().chars().all(|c| c.is_ascii_hexdigit()));
}

// ── verify ───────────────────────────────────────────────────────────────────

#[test]
fn verify_reports_ok_on_a_clean_set() {
    let f = tmp_with(&strings_of("even").join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("mt verify: OK"), "got: {err}");
    assert!(err.contains("6 chunks"), "got: {err}");
}

/// **The margin report** — the Critical the journey walk found. A chunk repaired
/// four times passes as OK while sitting one scratch from unrecoverable, and a
/// verdict that hides that tells the operator the opposite of what they need.
#[test]
fn verify_reports_its_margin_not_just_a_verdict() {
    let mut s = strings_of("even");
    // damage one character in the first string's payload
    let mut chars: Vec<char> = s[0].chars().collect();
    let at = 40;
    chars[at] = if chars[at] == 'q' { 'p' } else { 'q' };
    s[0] = chars.into_iter().collect();

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "one damaged symbol must still verify OK"
    );

    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("CORRECTION APPLIED"),
        "no margin report: {err}"
    );
    assert!(
        err.contains("1 of 4 symbols"),
        "margin not quantified: {err}"
    );
    assert!(err.contains("pos "), "corrections not LOCALISED: {err}");
    assert!(
        err.contains("chunk   1"),
        "chunk numbering must be 1-based in human output: {err}"
    );
}

/// `--transaction` compares the FULL txid. A 20-bit compare would report a match
/// for any transaction sharing those bits.
#[test]
fn verify_transaction_matches_and_rejects() {
    let f = tmp_with(&strings_of("even").join("\n"));

    let right = tmp_with(&raw_of("even"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .arg("--transaction")
        .arg(right.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8(out.stderr).unwrap().contains("full txid"));

    let wrong = tmp_with(&raw_of("uneven"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .arg("--transaction")
        .arg(wrong.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a different transaction must be rejected"
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("not the one on these strings"), "got: {err}");
    assert!(
        err.contains("FULL txid"),
        "the refusal must say the comparison was on the full txid: {err}"
    );
}

/// §1.1: `verify` never asks a node, so it runs on an air-gapped machine. Forcing
/// the offline path must change nothing.
#[test]
fn verify_never_needs_a_node() {
    let f = tmp_with(&strings_of("even").join("\n"));
    let with = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let without = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert_eq!(
        with.status.success(),
        without.status.success(),
        "verify behaved differently with no bitcoin-cli on PATH"
    );
    assert_eq!(
        with.stderr, without.stderr,
        "verify's output depended on a node"
    );
}

// ── the drawer: duplicates, and a plate that is scrap ────────────────────────

/// Damage `n` characters of a string, at spread-out positions.
fn damage(s: &str, positions: &[usize]) -> String {
    let mut c: Vec<char> = s.chars().collect();
    for &p in positions {
        c[p] = if c[p] == 'q' { 'p' } else { 'q' };
    }
    c.into_iter().collect()
}

/// **§1.8's advice produces duplicates, so duplicates are the expected state of
/// a well-kept drawer** — not an anomaly. What matters is which copy `mt` uses
/// and whether it says so: the point of cutting a second plate is that the
/// better one gets used, and "first one wins" would report the margin of
/// whichever the operator happened to type first.
#[test]
fn a_duplicate_keeps_the_healthier_copy_and_says_which_it_dropped() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines.push(damage(&s[2], &[45])); // chunk 3, one wrong character

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "a duplicate must not fail the set");
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(err.contains("DUPLICATE RESOLVED. chunk 3"), "got: {err}");
    assert!(
        err.contains("KEPT       the copy needing 0 of 4"),
        "the CLEAN copy must win, not the first-typed one: {err}"
    );
    assert!(
        err.contains("DISCARDED  the copy needing 1 of 4"),
        "the discarded copy's margin is what tells the operator to re-cut: {err}"
    );
    // The kept copy is clean, so there is nothing to report a correction for.
    assert!(
        !err.contains("CORRECTION APPLIED"),
        "the margin report described a copy mt did not use: {err}"
    );
}

/// The reverse order. If "first one wins" were the rule this would keep the
/// damaged copy and the test above would still pass — so both orders are
/// asserted, and only the pair proves the rule.
#[test]
fn the_healthier_copy_wins_whichever_arrives_first() {
    let s = strings_of("even");
    let mut lines = vec![damage(&s[2], &[45])];
    lines.extend(s.iter().cloned());

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("KEPT       the copy needing 0 of 4"),
        "the damaged copy won because it was typed first: {err}"
    );
}

/// **A miscut plate must not kill a set that has a good copy of that chunk.**
/// §1.8's advice is to cut a second copy, and this is the drawer that followed
/// it: one string damaged past `t = 4`, one clean, both typed back. Failing on
/// the first unreadable string would refuse a completely recoverable set —
/// while holding the good copy.
#[test]
fn a_plate_damaged_past_the_budget_does_not_kill_a_recoverable_set() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines.push(damage(&s[4], &[20, 30, 40, 50, 60, 70])); // six: past t = 4

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an unreadable EXTRA string failed a set that is complete:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("UNREADABLE STRING"), "silently ignored: {err}");
    assert!(
        err.contains("string 7"),
        "the plate is not identified: {err}"
    );
    // **mt MUST NOT SAY THE PLATE IS SCRAP.** It could not read the string, so
    // it does not know which chunk it was, or whether it belonged to this set at
    // all — it may be a plate from a different engraving that got typed into the
    // same pile. The earlier wording DIRECTED A PHYSICAL ACTION ON STEEL mt had
    // never identified, and this test asserted it.
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !flat.contains("that plate is scrap"),
        "mt asserted something it cannot know about a plate:\n{err}"
    );
    assert!(
        flat.contains("cannot tell you which chunk that string was"),
        "the limit of what mt knows is not stated:\n{err}"
    );
    assert!(
        flat.contains("Do not discard the plate on this message alone"),
        "the operator is not warned off the destructive reading:\n{err}"
    );
}

/// The other half of the rule: tolerating an unreadable string must NOT tolerate
/// a MISSING chunk. Without this, the change above would turn a set that cannot
/// be recovered into one that reports OK.
#[test]
fn an_unreadable_string_with_no_other_copy_still_fails() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines[4] = damage(&s[4], &[20, 30, 40, 50, 60, 70]); // REPLACED, not added

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a set missing chunk 5 entirely reported success"
    );
}

/// Garbage in must still say what is wrong with the input, not report a missing
/// plate — the operator would go looking in a drawer for something that was
/// never the problem.
#[test]
fn unreadable_input_reports_the_read_error_not_a_missing_chunk() {
    let f = tmp_with("mt1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq");
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        !err.contains("missing") || err.contains("checksum") || err.contains("BCH"),
        "a garbage input was reported as a missing plate: {err}"
    );
}

/// **The margin report gives the BEFORE-value, not only the position.**
///
/// A bare position tells the operator to go and look; a before-and-after tells
/// them what to look for — and it is the only way to distinguish a MIS-CUT plate
/// from a MIS-READ one. If the steel really says the corrected character, the
/// plate is fine and the typist slipped.
#[test]
fn the_margin_report_gives_before_and_after_values() {
    let s = strings_of("even");
    let mut lines = s.clone();
    lines[0] = damage(&s[0], &[40, 55]);

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(err.contains("2 of 4 symbols"), "{err}");
    // Both corrections, each with what was read and what replaced it.
    let reads = err.matches("read ").count();
    assert_eq!(reads, 2, "expected one before-value per correction: {err}");
    assert_eq!(err.matches("corrected to ").count(), 2, "{err}");
    // The characters actually on the plate. `damage` wrote 'q' or 'p'.
    for (pos, was) in [(41usize, 'q'), (56, 'q')] {
        assert!(
            err.contains(&format!("pos {pos:>3}   read {was}, corrected to ")),
            "position {pos} is missing its before-value: {err}"
        );
    }
}

// ── the content id: §1.1's last check ───────────────────────────────────────

/// **The check `verify`'s OK line has always claimed.** Two independent reviews
/// found it missing from opposite directions — one reading the spec against the
/// code, one forging the state it defends against — and until it existed
/// `verify` printed *"transaction re-derives"* on every run without deriving
/// anything.
///
/// The forgery is one call: encode transaction B's bytes under transaction A's
/// txid. Every checksum holds, every header is intact and names A, and the
/// payload is B — which is precisely what a BCH mis-correction produces, and
/// what a chunk cannot detect about itself.
fn forged_set() -> Vec<String> {
    let a_txid = corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()["txid"]
        .as_str()
        .unwrap()
        .to_string();
    let b_bytes = hex_to_bytes(&raw_of("uneven"));
    mt_codec::string_layer::pipeline::encode(&b_bytes, &a_txid).unwrap()
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    s.trim()
        .as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn verify_refuses_a_set_that_does_not_re_derive_its_id() {
    let f = tmp_with(&forged_set().join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "verify accepted a set whose payload is a DIFFERENT transaction"
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("REFUSED — §1.1,"), "got: {err}");
    assert!(
        !err.contains("transaction re-derives."),
        "verify still printed its OK line: {err}"
    );
    assert!(
        err.contains("MIS-CORRECTION"),
        "the likely cause is unnamed: {err}"
    );
    assert!(
        err.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("every checksum holds"),
        "got: {err}"
    );
    // Collapse whitespace before asserting on PROSE: the refusal formatter wraps
    // at 68 columns, so any phrase long enough to be worth asserting on is long
    // enough to be split by it.
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("does NOT prove every byte"),
        "the honest limit is missing — the txid does not cover the witness: {err}"
    );
}

/// **stdout must stay empty.** This is the funds-critical half: with the check
/// absent, `decode` emitted the WRONG transaction's broadcastable hex, and the
/// documented pipeline pipes stdout straight onward.
#[test]
fn decode_emits_nothing_when_the_id_does_not_re_derive() {
    let f = tmp_with(&forged_set().join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        out.stdout.is_empty(),
        "decode emitted broadcastable hex for a transaction that failed mt's \
         own checks: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn inspect_refuses_a_set_that_does_not_re_derive_its_id() {
    let f = tmp_with(&forged_set().join("\n"));
    let out = mt()
        .args(["inspect", "--bitcoin-cli", "/nonexistent", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "inspect described a forged set");
}

/// The control: a genuine set must still pass, or the guard is just a refusal.
#[test]
fn a_genuine_set_re_derives_its_id() {
    for label in ["even", "uneven"] {
        let f = tmp_with(&strings_of(label).join("\n"));
        let out = mt()
            .args(["verify", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{label}: a genuine set was refused:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            String::from_utf8(out.stderr)
                .unwrap()
                .contains("transaction re-derives.")
        );
    }
}

/// **The suspect list is the whole value of the failure report.** "Something is
/// wrong somewhere in 1,242 characters" leaves the operator with a pile of steel
/// and nowhere to start; a ranked list is half an hour of work. The margin
/// report already computed the ranking — this only had to print it.
#[test]
fn the_failure_report_ranks_the_suspects_by_corrections_applied() {
    let mut s = forged_set();
    // Damage two chunks by different amounts, so the ranking has an order to get
    // right. Both stay inside t = 4, so both still decode.
    let mut c: Vec<char> = s[3].chars().collect();
    for i in [30, 40, 50] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    s[3] = c.into_iter().collect();
    let mut c: Vec<char> = s[1].chars().collect();
    c[42] = if c[42] == 'q' { 'p' } else { 'q' };
    s[1] = c.into_iter().collect();

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();

    let worst = err
        .find("chunk   4   3 of 4")
        .unwrap_or_else(|| panic!("chunk 4 missing:\n{err}"));
    let next = err
        .find("chunk   2   1 of 4")
        .unwrap_or_else(|| panic!("chunk 2 missing:\n{err}"));
    assert!(
        worst < next,
        "the suspects are not ranked most-corrected first:\n{err}"
    );
    assert!(
        err.contains("<-- most suspect"),
        "the top suspect is unmarked:\n{err}"
    );
    assert!(
        err.contains("needed no correction"),
        "the clean chunks are not accounted for, so the operator cannot tell how \
         many plates they can skip:\n{err}"
    );
}

/// With NO chunk corrected, miscorrection is not the explanation and there is no
/// ranking to offer. Saying so beats printing an empty list under a heading that
/// promises one.
#[test]
fn the_failure_report_says_so_when_there_is_nothing_to_rank() {
    let f = tmp_with(&forged_set().join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("No chunk needed any correction"),
        "an unranked failure printed a ranking heading: {err}"
    );
    assert!(
        err.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("two different transactions"),
        "the remaining explanation is not named: {err}"
    );
}

// ── §1.1e's length check ────────────────────────────────────────────────────

/// **A dropped character reported a MISSING PLATE.** An omission shifts every
/// symbol after it, so the string fails its checksum, contributes no chunk, and
/// the set says `chunk 3 of 6 is missing` — an accusation about the operator's
/// steel, sending them to hunt for a plate sitting in front of them. BCH repairs
/// substitutions; it cannot repair a length.
#[test]
fn a_dropped_character_is_named_as_a_length_error_not_a_missing_plate() {
    let mut s = strings_of("even");
    s[2] = format!("{}{}", &s[2][..40], &s[2][41..]);

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(err.contains("REFUSED — §1.1e,"), "got: {err}");
    assert!(
        !err.contains("is missing"),
        "mt still accused the operator of a lost plate:\n{err}"
    );
    assert!(
        err.contains("string 3: 86 characters (expected 87)"),
        "the suspect string and both lengths must be named:\n{err}"
    );
    assert!(err.contains("MISSING"), "{err}");
    // The verb is the one the operator typed, not a hardcoded one.
    assert!(err.starts_with("mt verify:"), "{err}");
}

#[test]
fn an_extra_character_is_named_as_extra() {
    let mut s = strings_of("even");
    s[4] = format!("{}q{}", &s[4][..30], &s[4][30..]);
    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("string 5: 88 characters (expected 87)") && err.contains("EXTRA"),
        "got: {err}"
    );
}

/// **The control that decides the design.** A set whose payload does not divide
/// evenly has one legitimately SHORT final chunk — indistinguishable from a
/// dropped character BY LENGTH. The discriminator is that the legitimate one
/// PARSES, which is why the check is consulted on the failure path rather than
/// run up front. Without this test, a length-only check would refuse every
/// uneven set.
#[test]
fn a_legitimately_short_final_chunk_is_not_a_length_error() {
    let s = strings_of("uneven");
    let lengths: Vec<usize> = s.iter().map(|x| x.chars().count()).collect();
    assert!(
        lengths.last() < lengths.first(),
        "the fixture is not uneven, so this control proves nothing: {lengths:?}"
    );
    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an uneven set was refused as a length error:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A string damaged past `t = 4` but the RIGHT LENGTH is not a length error, and
/// must keep its own message. Otherwise the new check would relabel every
/// unreadable string as a miscount.
#[test]
fn a_correct_length_string_that_fails_keeps_its_own_message() {
    let mut s = strings_of("even");
    let mut c: Vec<char> = s[1].chars().collect();
    for i in [10, 20, 30, 40, 50, 60] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    s[1] = c.into_iter().collect();
    assert_eq!(s[1].chars().count(), 87, "the damage changed the length");

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        !err.contains("§1.1e"),
        "a correctly-sized unreadable string was reported as a miscount:\n{err}"
    );
}

/// **More than 4 typos in one string used to report a MISSING PLATE**, with all
/// nine plates on the table. `explain_failure` computed the unreadable list,
/// which was live and correct, and then discarded it whenever the damage was
/// substitutions rather than length. §1.1e's fold closed that door for the
/// length branch only; this is the same door, the other hinge.
#[test]
fn too_many_typos_names_the_string_and_says_every_plate_is_accounted_for() {
    let mut s = strings_of("even");
    let mut c: Vec<char> = s[1].chars().collect();
    for i in [10, 20, 30, 40, 50, 60, 70] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    s[1] = c.into_iter().collect();
    assert_eq!(
        s[1].chars().count(),
        87,
        "the damage must not change the length"
    );

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        !flat.contains("is missing"),
        "mt named a plate as lost while the operator holds all of them:\n{err}"
    );
    assert!(
        flat.contains("string 2 could not be read"),
        "the unreadable string is not identified:\n{err}"
    );
    // **THE CLAIM MUST BE ABOUT CHUNKS, NOT LINES TYPED.** An earlier version
    // said "EVERY PLATE IS ACCOUNTED FOR. Nothing is lost" on the strength of
    // `strings.len()` — so typing one plate twice and skipping the next, the
    // likeliest slip in the procedure, kept the count at n while a chunk was
    // genuinely absent, and mt asserted the opposite categorically.
    assert!(
        flat.contains("so every chunk COULD be here"),
        "the clause that turns 'go and find a plate' into 're-read this one' is \
         missing, or overclaims:\n{err}"
    );
    assert!(
        !flat.contains("Nothing is lost —") || flat.contains("nothing is necessarily lost"),
        "mt asserted nothing is lost, categorically:\n{err}"
    );
    assert!(
        flat.contains("0/o") || flat.contains("confusable"),
        "no help for the re-read:\n{err}"
    );
}

/// **A single short string is ambiguous only when it MIGHT be the final chunk.**
/// Exactly one string per uneven set is short by design — but the readable
/// strings carry their own index and count, so mt can tell. Both directions are
/// asserted, because a message that hedges when it could be certain is as wrong
/// as one that accuses when it cannot be.
#[test]
fn a_short_string_is_only_called_ambiguous_when_it_really_is() {
    // EVEN set: every chunk the same length, and the final chunk reads fine.
    // mt can prove a short string is a miscount.
    let mut s = strings_of("even");
    s[2] = format!("{}{}", &s[2][..40], &s[2][41..]);
    let f = tmp_with(&s.join("\n"));
    let err = String::from_utf8(
        mt().args(["verify", "--in"])
            .arg(f.path())
            .output()
            .unwrap()
            .stderr,
    )
    .unwrap();
    assert!(
        err.contains("the wrong length for this set"),
        "mt hedged where it could be certain:\n{err}"
    );

    // UNEVEN set with the FINAL chunk damaged past t = 4: mt cannot tell a
    // miscount from the chunk that is legitimately shorter.
    let mut s = strings_of("uneven");
    let last = s.len() - 1;
    let mut c: Vec<char> = s[last].chars().collect();
    for i in [10, 20, 30, 40, 50, 60] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    s[last] = c.into_iter().collect();
    let f = tmp_with(&s.join("\n"));
    let err = String::from_utf8(
        mt().args(["verify", "--in"])
            .arg(f.path())
            .output()
            .unwrap()
            .stderr,
    )
    .unwrap();
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("which is also what a final chunk looks like"),
        "mt accused a plate of a miscount it cannot demonstrate:\n{err}"
    );
    assert!(
        flat.contains("will not accuse your steel"),
        "the honest limit is not stated:\n{err}"
    );
}

/// §1.1e's **positional autocorrect**, the one Important the folds kept losing.
///
/// The bech32 alphabet omits `1`, `b`, `i` and `o` *because they are confusable
/// when engraved* — which is exactly what makes them repairable: past the `mt1`
/// prefix each is a misreading with only one candidate. At index 2 the reverse
/// holds, since that position IS the `1` of `mt1`.
#[test]
fn a_confusable_misreading_is_repaired_positionally() {
    let mut s = strings_of("even");
    // Substitute the four confusables into one string.
    let mut c: Vec<char> = s[1].chars().collect();
    let mut n = 0;
    for ch in c.iter_mut().skip(3) {
        match *ch {
            '0' if n < 2 => {
                *ch = 'o';
                n += 1;
            }
            '6' if n < 4 => {
                *ch = 'b';
                n += 1;
            }
            'l' if n < 6 => {
                *ch = '1';
                n += 1;
            }
            _ => {}
        }
    }
    assert!(
        n > 0,
        "the fixture contains none of the confusable characters"
    );
    s[1] = c.into_iter().collect();

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{n} confusable substitutions were not repaired:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `mtl…` and `mti…` — the separator misread. That position IS the `1`.
#[test]
fn a_misread_separator_is_repaired() {
    for wrong in ['l', 'i'] {
        let mut s = strings_of("even");
        s[0] = format!("mt{wrong}{}", &s[0][3..]);
        let f = tmp_with(&s.join("\n"));
        let out = mt()
            .args(["verify", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "mt{wrong} was not repaired to mt1:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// **AUTOCORRECT NEVER TOUCHES A STRING THAT ALREADY PARSES.** It is a repair
/// attempted on failure, not a preprocessing pass — a pass would rewrite valid
/// input, and `b` → `6` on a string that was already right changes the payload.
///
/// Asserted by DECODING: if any clean string were rewritten, the bytes would
/// differ.
#[test]
fn autocorrect_leaves_a_clean_set_byte_identical() {
    for label in ["even", "uneven"] {
        let f = tmp_with(&strings_of(label).join("\n"));
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8(out.stdout).unwrap().trim(),
            raw_of(label),
            "{label}: a clean set did not survive the autocorrect pass byte-identically"
        );
    }
}

/// And it must not rescue damage it cannot explain: a string with genuine
/// substitutions beyond `t = 4` still fails, rather than being quietly rewritten
/// into some other valid string.
#[test]
fn autocorrect_does_not_manufacture_a_valid_string() {
    let mut s = strings_of("even");
    let mut c: Vec<char> = s[3].chars().collect();
    for i in [10, 20, 30, 40, 50, 60, 70] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    s[3] = c.into_iter().collect();
    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "damage past t = 4 was silently accepted"
    );
}

/// **The autocorrect I had just added silently corrupted legitimate input.**
///
/// An ELIDED line carries bare bech32 symbols, and `m`, `t` and `l` are all in
/// that alphabet — so roughly one elided line in 32,768 begins `mtl` by chance.
/// The unguarded separator repair rewrote it to `mt1…`, which made it look like
/// a FULL string with a wrong prefix, and the set then failed with a message
/// about miscounted characters. About one set in four thousand, on the recovery
/// path, silently.
///
/// The fix is the rule every other autocorrect already followed: speculate,
/// verify, and discard the speculation if it does not decode.
#[test]
fn an_elided_line_that_happens_to_begin_mtl_is_left_alone() {
    let elided: Vec<String> = corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()["strings_elided"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();

    let mut lines = elided.clone();
    // Force the collision on an elided line.
    lines[1] = format!("mtl{}", &elided[1][3..]);
    assert!(!lines[1].starts_with("mt1"));

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an elided line beginning `mtl` was corrupted by the separator repair:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even"),
        "the set decoded, but not to the right bytes"
    );
}

/// The control: a genuinely misread separator on a FULL string is still
/// repaired. Only the pair shows the guard narrowed the repair rather than
/// removing it.
#[test]
fn a_full_string_with_a_misread_separator_is_still_repaired() {
    for wrong in ['l', 'i'] {
        let mut s = strings_of("even");
        s[0] = format!("mt{wrong}{}", &s[0][3..]);
        let f = tmp_with(&s.join("\n"));
        let out = mt()
            .args(["decode", "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "mt{wrong} was not repaired:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap().trim(),
            raw_of("even")
        );
    }
}

/// **The two-suspect case, which no test covered and whose fix therefore did not
/// take.** An uneven set has one legitimately short final chunk; a SECOND short
/// string anywhere used to flip every suspect back into the accusing branch, so
/// `mt` told the operator that a 79-character chunk designed to be 79 characters
/// was missing six of them.
///
/// The first fix extracted a per-suspect closure and left the whole-set
/// condition `short_count == 1` inside it — textually the same predicate it
/// replaced. **Moving code without moving its assumptions.**
#[test]
fn two_short_strings_do_not_make_mt_accuse_the_final_chunk() {
    let mut s = strings_of("uneven");
    let last = s.len() - 1;
    let short = s[last].chars().count();
    let modal = s[0].chars().count();
    assert!(short < modal, "the fixture is not uneven");

    // The final chunk damaged past t = 4...
    let mut c: Vec<char> = s[last].chars().collect();
    for i in [10, 20, 30, 40, 50, 60] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    s[last] = c.into_iter().collect();
    // ...and a SECOND, independent failure: a dropped character elsewhere.
    s[1] = format!("{}{}", &s[1][..40], &s[1][41..]);

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();

    assert!(
        !err.contains(&format!("{short} characters (expected {modal})")),
        "mt quantified a deficit on the chunk that is short BY DESIGN:\n{err}"
    );
    assert!(
        err.contains(&format!(
            "string {}: {short} characters, where this set's usual",
            last + 1
        )),
        "the final chunk should be described, not accused:\n{err}"
    );
    // The genuinely-dropped character is still named as one.
    assert!(
        err.contains("string 2:"),
        "the real length error is missing:\n{err}"
    );
}

/// **`mtl` plus a LENGTH-CHANGING second defect**, which is what the finding
/// actually reproduced and what the first fix missed: it required the malformed
/// candidate's length to equal a full string's EXACTLY, so it fired only when
/// the second defect was a same-length substitution.
#[test]
fn a_misread_separator_with_a_dropped_character_is_named_not_treated_as_elided() {
    let mut s = strings_of("even");
    let t = format!("mtl{}", &s[0][3..]);
    s[0] = format!("{}{}", &t[..40], &t[41..]); // one character SHORT

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("the `1` of `mt1` misread"),
        "mt treated it as an elided line again:\n{err}"
    );
    assert!(
        !err.contains("characters are EXTRA"),
        "mt prepended a prefix and then blamed the plate for the length:\n{err}"
    );
}

/// **A duplicated UNREADABLE line must not be counted as two chunks.** The
/// distinct-chunk fix counted readable chunks distinctly and then added
/// `unreadable.len()` — so two copies of one damaged plate reproduced the very
/// "nothing is necessarily lost" claim the fix was written to eliminate,
/// through a duplicate instead of through a line count.
#[test]
fn a_duplicated_unreadable_line_is_one_chunk_not_two() {
    let mut s = strings_of("even");
    let mut c: Vec<char> = s[4].chars().collect();
    for i in [10, 20, 30, 40, 50, 60] {
        c[i] = if c[i] == 'q' { 'p' } else { 'q' };
    }
    let damaged: String = c.into_iter().collect();
    // Drop chunk 4 entirely, and supply the damaged plate TWICE.
    s.remove(3);
    s.pop();
    s.push(damaged.clone());
    s.push(damaged);

    let f = tmp_with(&s.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    let flat = err.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !flat.contains("so every chunk COULD be here"),
        "two copies of one unreadable plate were counted as two chunks:\n{err}"
    );
    assert!(
        flat.contains("A PLATE IS MISSING AS WELL AS DAMAGED"),
        "the true state is not named:\n{err}"
    );
}

/// **The `--elide-prefix` variant of the misread separator, which survived two
/// fixes.** With elision exactly ONE line is full — so when that line is the one
/// whose separator was misread, there is no `mt1` candidate left to measure
/// against, and a guard that derived "what a full line looks like" only from
/// `mt1` lines could never fire.
///
/// The reference now falls back to the elided lines, which are exactly
/// `ELIDED_DROP` characters shorter than a full one.
#[test]
fn a_misread_separator_on_the_only_full_line_is_named_not_treated_as_elided() {
    let elided: Vec<String> = corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()["strings_elided"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();

    let mut lines = elided.clone();
    let t = format!("mtl{}", &elided[0][3..]);
    lines[0] = format!("{}{}", &t[..40], &t[41..]); // separator misread AND one short

    let f = tmp_with(&lines.join("\n"));
    let out = mt()
        .args(["verify", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("the `1` of `mt1` misread"),
        "the only full line was treated as elided:\n{err}"
    );
    assert!(
        !err.contains("all 6 lines are elided"),
        "mt still reports every line as elided:\n{err}"
    );
}

/// The control that keeps the fallback honest: a CLEAN elided set, where the
/// derived reference must not make anything look wrong.
#[test]
fn a_clean_elided_set_is_unaffected_by_the_derived_full_length() {
    let elided: Vec<String> = corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()["strings_elided"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let f = tmp_with(&elided.join("\n"));
    let out = mt()
        .args(["decode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        raw_of("even")
    );
}

// ── P1 step 2: `-` on the three READING verbs ────────────────────────────────
//
// `inspect` is exercised here rather than in `inspect.rs` because the unit
// under test is not a verb — it is the ONE `stdin_dash` field on the shared
// `ReadArgs` struct (`src/main.rs`), which `decode`, `verify` and `inspect` all
// derive from. Splitting the gate across two suites would let a later edit
// green one half while the other silently regressed.

/// **`mt decode -`, `mt verify -` and `mt inspect -` must WORK, not error.**
///
/// F-250 gave `mt encode` the dash and stopped there; the three reading verbs
/// still answered `error: unexpected argument '-' found` at exit **2**,
/// measured. `-` meaning *read stdin* is honoured by `cat`, `tar`, `curl`,
/// `gpg` and `jq`, and the recovery path is exactly where an operator reaches
/// for a habit rather than for the manual.
///
/// **Asserted as EQUALITY with the flagless run on BOTH streams**, which is the
/// shape F-250 already uses (`tests/encode.rs`): a bare `success()` would also
/// pass for a `-` silently taken as a filename, or one that suppressed the
/// report.
#[test]
fn a_bare_dash_means_stdin_on_every_reading_verb() {
    let strings = strings_of("even").join("\n");
    for verb in ["decode", "verify", "inspect"] {
        let plain = mt()
            .args([verb, "--bitcoin-cli", "/nonexistent/bitcoin-cli"])
            .write_stdin(strings.clone())
            .output()
            .unwrap();
        let dashed = mt()
            .args([verb, "--bitcoin-cli", "/nonexistent/bitcoin-cli", "-"])
            .write_stdin(strings.clone())
            .output()
            .unwrap();

        assert!(
            dashed.status.success(),
            "`mt {verb} -` must succeed; got {:?}\n{}",
            dashed.status.code(),
            String::from_utf8_lossy(&dashed.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&dashed.stdout),
            String::from_utf8_lossy(&plain.stdout),
            "`mt {verb} -`: `-` must change NOTHING on stdout"
        );
        assert_eq!(
            String::from_utf8_lossy(&dashed.stderr),
            String::from_utf8_lossy(&plain.stderr),
            "`mt {verb} -`: `-` must not add or remove a line of the report"
        );
    }
}

/// **The control F-250 carries, on the three verbs that just gained the dash.**
///
/// `-` must be the ONLY positional admitted. If the field opened a general
/// positional, the pre-clap §8.2f guard would be the only thing between a
/// mistyped argument and silent acceptance — and that guard is deliberately
/// narrow, so `wat` sails straight past it.
#[test]
fn a_dash_does_not_open_the_door_to_other_positionals_on_reading_verbs() {
    let strings = strings_of("even").join("\n");
    for verb in ["decode", "verify", "inspect"] {
        let out = mt()
            .args([verb, "--bitcoin-cli", "/nonexistent/bitcoin-cli", "wat"])
            .write_stdin(strings.clone())
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "`mt {verb} wat`: a stray positional must still be refused"
        );
    }
}

// ── P1 row 13 — F-275: `mt decode` into a world-readable stdout ──────────────
//
// `mt encode` refuses a mode-0644 stdout at §8.2h. `mt decode` wrote
// BROADCASTABLE HEX into the identical destination at exit 0, with no guard and
// no warning — measured before this row: 445 bytes, rc 0, **stderr empty**.
//
// The inconsistency is itself the hazard, because an operator who learned that
// mt refuses a world-readable output will believe decode is protected too.
//
// **RULED BY THE OPERATOR 2026-08-27: WARN, DO NOT REFUSE.** The default umask
// is 022, so `mt decode > tx.hex` creates the file 0644 — an encode-style
// refusal would reject the ordinary invocation on every default machine. And
// `--out` is NOT what closes it: §6b's `--out` reasoning is entirely about the
// refusal encode prints, so adding the channel to decode would half-close a
// hazard while reading as a whole fix.

#[cfg(unix)]
fn decode_to_mode(mode: u32, extra: &[&str]) -> (u64, String, Option<i32>) {
    use std::os::unix::fs::PermissionsExt;
    let f = tmp_with(&strings_of("even").join("\n"));
    let dir = tempfile::tempdir().unwrap();
    let sink = dir.path().join("tx.hex");
    let handle = std::fs::File::create(&sink).unwrap();
    std::fs::set_permissions(&sink, std::fs::Permissions::from_mode(mode)).unwrap();

    // `std::process::Command`, not `assert_cmd`: the whole subject is the MODE
    // OF FD 1, so stdout has to be a real file at a chosen mode.
    let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("mt"))
        .arg("decode")
        .args(extra)
        .args(["--bitcoin-cli", "/nonexistent/bitcoin-cli"])
        .arg("--in")
        .arg(f.path())
        .stdout(std::process::Stdio::from(handle))
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    (
        std::fs::metadata(&sink).unwrap().len(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
        o.status.code(),
    )
}

/// **THE GATE, and the EXIT CODE is the half that makes it one.**
///
/// It is asserted UNCHANGED at 0, and the artifact is asserted byte-for-byte
/// against the 0600 control — so a refusal smuggled in as a warning goes RED
/// here rather than shipping as "we made it safer".
#[test]
#[cfg(unix)]
fn decode_warns_about_a_world_readable_stdout_and_still_exits_0() {
    let (control_bytes, _, control_rc) = decode_to_mode(0o600, &["--quiet"]);
    assert_eq!(control_rc, Some(0));
    assert!(control_bytes > 0, "the control must produce the hex");

    let (bytes, err, rc) = decode_to_mode(0o644, &["--quiet"]);
    assert_eq!(
        rc,
        Some(0),
        "F-275 is a WARNING. mt decode must still succeed -- the default umask \
         is 022, so refusing here would reject `mt decode > tx.hex` on every \
         default machine: {err}"
    );
    assert_eq!(
        bytes, control_bytes,
        "and it must still write the SAME artifact, not a truncated one"
    );
    assert!(
        err.contains("0644"),
        "it must name the mode it measured: {err}"
    );
    assert!(
        err.contains("WARNING"),
        "and it must be shaped like a warning, so a reader scanning stderr can \
         tell it did not stop the run: {err}"
    );
    assert!(
        !err.contains("REFUSED"),
        "warn, do not refuse -- the operator ruled it: {err}"
    );
}

/// **THE CONTROL.** A warning that always fires cannot pass this, and without
/// it the gate above is satisfied by printing the sentence unconditionally.
///
/// A pipe is the same case and matters more, because it is what the documented
/// pipeline does: an anonymous pipe is mode 0600, so nothing fires there either.
#[test]
#[cfg(unix)]
fn decode_says_nothing_about_an_owner_only_stdout() {
    let (bytes, err, rc) = decode_to_mode(0o600, &["--quiet"]);
    assert_eq!(rc, Some(0));
    assert!(bytes > 0);
    assert_eq!(
        err, "",
        "an owner-only stdout is exactly what we want, and mt must not say a \
         word about it"
    );

    // ...and the pipeline, which `assert_cmd` gives us as an anonymous pipe.
    let piped = mt()
        .args([
            "decode",
            "--quiet",
            "--bitcoin-cli",
            "/nonexistent/bitcoin-cli",
        ])
        .arg("--in")
        .arg(tmp_with(&strings_of("even").join("\n")).path())
        .output()
        .unwrap();
    assert!(piped.status.success());
    assert_eq!(
        String::from_utf8_lossy(&piped.stderr),
        "",
        "an anonymous pipe is mode 0600; the documented pipeline must stay silent"
    );
}

/// The wording is **derived from the mode**, exactly as at the two `encode`
/// sites — F-260, applied to the site F-275 created rather than re-introduced
/// there.
#[test]
#[cfg(unix)]
fn the_decode_warning_says_what_the_mode_grants_not_what_the_rule_is_called() {
    let (_, err, rc) = decode_to_mode(0o620, &["--quiet"]);
    assert_eq!(rc, Some(0), "still a warning: {err}");
    assert!(err.contains("0620"), "{err}");
    assert!(
        !err.contains("grant read"),
        "`0620 & 0o044 == 0` -- no read bit is set outside the owner: {err}"
    );
    assert!(
        err.contains("write"),
        "and it must say what IS granted. On THIS verb that is the sharper \
         hazard: someone who can write the file can replace the hex with a \
         different transaction before it is broadcast: {err}"
    );

    let (_, err, _) = decode_to_mode(0o644, &["--quiet"]);
    assert!(
        err.contains("read") && err.contains("group and others"),
        "the control: at 0644 the read claim is TRUE and must survive: {err}"
    );
}

/// **The warning survives `--quiet`, and is DATA under `--json`.**
///
/// Warnings are never suppressed, on any verb — so `--quiet` must not silence
/// it. And `--json` must stay parseable: mt's own rule is that with `--json` the
/// prose becomes data inside the one document, so the warning goes into the
/// `warnings` array rather than being printed beside the JSON, where it would
/// have to be sliced off before anything could parse it.
#[test]
#[cfg(unix)]
fn the_decode_warning_is_never_suppressed_and_is_data_under_json() {
    // --quiet: the report is gone, the warning is not.
    let (_, err, rc) = decode_to_mode(0o644, &["--quiet"]);
    assert_eq!(rc, Some(0));
    assert!(
        err.contains("0644"),
        "--quiet must not silence a warning: {err}"
    );

    // --json: stderr is ONE document, and the warning is inside it.
    let (_, err, rc) = decode_to_mode(0o644, &["--json"]);
    assert_eq!(rc, Some(0));
    let doc: serde_json::Value = serde_json::from_str(err.trim()).unwrap_or_else(|e| {
        panic!("--json stderr must parse as one document ({e}):\n{err}");
    });
    let warnings = doc["warnings"]
        .as_array()
        .expect("the json report carries a warnings array")
        .iter()
        .map(|w| w.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        warnings.contains("0644"),
        "the mode warning must be IN the document, not printed beside it: {warnings}"
    );
}

/// **`--out` was NOT added to `decode`, and this pins it.**
///
/// It is the fix an implementer reaches for, and §6 rules it out of scope: §6b's
/// `--out` reasoning is entirely about the refusal `encode` prints. A `decode
/// --out` would half-close the hazard while reading as a whole fix, and the
/// operator's ruling for this site was a warning.
#[test]
fn decode_has_no_out_flag() {
    let o = mt()
        .args(["decode", "--out", "/dev/null"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(!o.status.success(), "decode must not accept --out: {err}");
    assert!(
        err.contains("unexpected argument '--out'"),
        "clap must not know it: {err}"
    );
}
