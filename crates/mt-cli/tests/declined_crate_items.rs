//! **P1 row 12 — THE DECLINE, ASSERTED. The most valuable output of this phase
//! is the list of crate items `mt` did NOT take, and why.**
//!
//! `mnemonic-io-lib` was extracted from `me` with `me` as its only consumer, so
//! every line in it was a line `me` wanted. `mt` is the second consumer, in a
//! second repository, with different policy — and it takes **5 of the 11 public
//! items and 3 of the 7 modules**. That is the phase's headline finding and it
//! is not a complaint: the parts that fit are the parts P0 argued were
//! mechanism, and the parts that do not are the parts whose *shape* is policy
//! even though their contents are not.
//!
//! | crate item | mt |
//! | --- | --- |
//! | `fd::stdout_mode`, `fd::mode_of` | ADOPTED, row 9 |
//! | `remedy::history_purge_block`, `remedy::history_purge_recipes` | ADOPTED, row 8 |
//! | `channel::destination` | ADOPTED, row 10 |
//! | `write::write_private` | ADOPTED, row 10 |
//! | `exit::write_block`, `exit::WriteBlock` | **DECLINED** |
//! | `observation::PayloadKind` | **DECLINED** |
//! | `records::split_record_stream`, `records::no_records_guard` | **DECLINED** |
//!
//! **This file exists so a later phase cannot adopt them as tidying.** Each
//! test below fails if the corresponding crate item is wired in, and says which
//! one and why — because "we already depend on the crate, why not use the rest
//! of it" is a completely reasonable thing for the next reader to think.
//!
//! The third leg of the decline — **mt keeps `0o077` where `me` rules `0o044`**
//! — is held by `refusals.rs::refuses_a_group_writable_stdout_no_read_mask_can_see`,
//! which pins stdout mode 0620 as REFUSED. It lives there because it is a
//! §8.2h refusal with a `refusals.toml` entry and a mutation control; it is
//! named here so this list is complete.
//!
//! `observation::PayloadKind` has no test and cannot have one: its two variants
//! are `Bearer` and `CarriesNoSecret`, **every byte mt writes is bearer**, and
//! mt has no `wipe` and no fill image, so nothing could construct the second
//! variant. Adopting it would put a word into mt's vocabulary that nothing can
//! produce and make `exposure_matters()` a constant. That is an argument, not a
//! measurement, and it is recorded here rather than faked as an assertion.
#![cfg(unix)]

use std::process::Command;

const OFFLINE: &[&str] = &["--bitcoin-cli", "/nonexistent/bitcoin-cli"];

fn corpus() -> serde_json::Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// **`exit::write_block` and `exit::WriteBlock` are DECLINED, and this is the
/// measurement that says why.**
///
/// `write_block`'s `Destination::Terminal` arm returns a refusal
/// **unconditionally**. `me` refuses a terminal (F-253) because a terminal
/// persists in scrollback and sessions are logged. **`mt` has no terminal
/// refusal** — it has a terminal-aware WARNING instead — and giving it one is a
/// RULING, not a refactor: P0's own out-of-scope section rules that *"changing
/// what either tool treats as a dangerous destination is a RULING, never a
/// refactor"*.
///
/// So `mt encode` to a REAL pty must still exit 0 and still paint the strings.
/// Adopting `write_block` unchanged turns this red; calling it with
/// `stdout_is_tty` hard-coded `false` would keep it green and is a lie to a
/// function about an observable fact, which the next reader repairs.
///
/// `WriteBlock` goes with it: its `Terminal(PayloadKind)` variant would be
/// unconstructible in mt, and a dead variant in a shared decision type is
/// exactly how the policy behind it gets adopted later by someone tidying up.
///
/// Measured 2026-08-27: rc **0**, a 4264-byte typescript carrying all
/// **11** `mt1` strings. The typescript SIZE is not asserted — a pty rewrites
/// line endings and the stderr card shares the stream — but every string is.
#[test]
fn mt_paints_the_strings_across_a_real_terminal_and_exits_0() {
    let script = "/usr/bin/script";
    assert!(
        std::path::Path::new(script).exists(),
        "{script} (util-linux) is required: the decline under test is about a \
         TERMINAL destination, and there is no way to have one without a pty. \
         Deliberately a FAILURE and not a skip -- a skipped gate prints ok and \
         exit 0."
    );

    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("tx.hex");
    std::fs::write(&input, v["raw_hex"].as_str().unwrap()).unwrap();
    let typescript = dir.path().join("typescript");

    let mt = assert_cmd::cargo::cargo_bin("mt");
    let st = Command::new(script)
        .arg("-qec")
        .arg(format!(
            "{} encode {} --in {}",
            mt.display(),
            OFFLINE.join(" "),
            input.display()
        ))
        .arg(&typescript)
        .status()
        .expect("script (util-linux) is required to give mt a pty");

    assert_eq!(
        st.code(),
        Some(0),
        "mt has NO terminal refusal, and P1 does not give it one. If this is now \
         non-zero, `exit::write_block`'s unconditional Terminal arm has been \
         adopted -- which is a RULING and not a refactor."
    );

    let seen = std::fs::read_to_string(&typescript).unwrap();
    for s in v["strings"].as_array().unwrap() {
        let s = s.as_str().unwrap();
        assert!(
            seen.contains(s),
            "the strings must still reach the terminal. mt paints a bearer \
             artifact across scrollback by its own ruling, and warns instead of \
             refusing:\n{seen}"
        );
    }
}

/// **`records::no_records_guard` is DECLINED, and the reason is one sentence of
/// its message.**
///
/// mt already refuses an empty stream on all three reading verbs, in its own
/// words and at its own section. The crate's guard advises *"pass them on argv,
/// with --in, or on stdin"* — and **mt REFUSES argv** (§8.2f), because an `mt1`
/// set is bearer and an argument lands in shell history and in `ps`. Adopting
/// it would print advice mt's own §8.2f guard exists to prevent following, in
/// the message an operator meets when they are already stuck.
#[test]
fn every_reading_verb_still_refuses_empty_input_in_mts_own_words() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.txt");
    std::fs::write(&empty, b"").unwrap();

    for verb in ["decode", "verify", "inspect"] {
        let out = Command::new(assert_cmd::cargo::cargo_bin("mt"))
            .arg(verb)
            .args(OFFLINE)
            .arg("--in")
            .arg(&empty)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        assert_eq!(
            out.status.code(),
            Some(1),
            "{verb}: mt's own empty-input refusal exits 1: {err}"
        );
        assert!(
            err.contains("§1.1e") && err.contains("no strings found in the input"),
            "{verb}: it is mt's message, at mt's section: {err}"
        );
        assert!(
            !err.contains("on argv"),
            "{verb}: the crate's guard advises passing records ON ARGV, and mt \
             REFUSES argv at §8.2f. If this string is here, `no_records_guard` \
             has been adopted and mt is now advising the operator to do the one \
             thing it refuses: {err}"
        );
        assert!(
            out.stdout.is_empty(),
            "{verb}: a refusal leaves no artifact"
        );
    }
}

/// **`records::split_record_stream` is DECLINED**, and these are the four
/// behaviours that would be lost with it.
///
/// The crate's reader skips blank lines and returns one record per line. mt's
/// `read_strings::read` strips grouping whitespace **within** a line, splits a
/// single-line blob at each `mt1` prefix, normalises case, and restores an
/// elided prefix from the first string. It is not a simpler version of mt's
/// reader; it is a different one.
///
/// Each row here is an input the crate's reader would hand back unchanged or
/// mangled, and mt reads all of them back to the same transaction.
#[test]
fn mts_own_reader_still_does_what_split_record_stream_cannot() {
    let v = corpus()["vectors"].as_array().unwrap()[0].clone();
    let strings: Vec<String> = v["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let elided: Vec<String> = v["strings_elided"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let want = v["raw_hex"].as_str().unwrap().to_string();

    let grouped: Vec<String> = strings
        .iter()
        .map(|s| {
            s.chars()
                .collect::<Vec<_>>()
                .chunks(4)
                .map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();

    for (label, text) in [
        ("one per line", strings.join("\n")),
        // ONE LINE, no separators between records at all.
        ("a single-line blob", strings.join("")),
        // Grouping whitespace WITHIN each line.
        ("grouped in fours", grouped.join("\n")),
        ("UPPERCASE", strings.join("\n").to_uppercase()),
        // The prefix elided on every string but the first.
        ("elided prefixes", elided.join("\n")),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("typed.txt");
        std::fs::write(&f, &text).unwrap();
        let out = Command::new(assert_cmd::cargo::cargo_bin("mt"))
            .arg("decode")
            .args(OFFLINE)
            .arg("--quiet")
            .arg("--in")
            .arg(&f)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(
            out.status.success(),
            "{label}: mt must read this back: {err}"
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            want,
            "{label}: mt's reader is what makes this work. A record stream that \
             skipped blank lines and split on newlines alone would lose it."
        );
    }
}
