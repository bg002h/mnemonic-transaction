//! FIX-F (bughunt-1119 FIX table row 23): the set prefix mt prints and restores
//! comes from what a clean plate carries, and encode's counts describe the
//! plates it prints. Keys bh-mt-I8, bh-mt-I21, bh-mt-I7 (M); sweep-1122 rows 13
//! and 14. Rulings T14, T19 (architect-bughunt-1119-questions.md:52, :57).
//!
//! Each test asserts the FIXED behaviour. Rows marked CONTROL pass on the base
//! and pin what the fix must not change. Spec: design/FIX_bh1119_mt_prefix_corrected.md.

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::exit::{INVALID, REPAIRED};

const ELIDED_DROP: usize = 11;
const CS: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// `mt`, offline by default: no `bitcoin-cli` on the child's `PATH`, so a
/// machine with one never sends it fixture txids. A test that wants a node
/// names a stub with `--bitcoin-cli`.
fn mt() -> Command {
    let mut c = Command::cargo_bin("mt").unwrap();
    c.env("PATH", "/nonexistent");
    c
}

fn even() -> Vec<String> {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mt-codec/src/test_vectors/mt1_v1.json"
    );
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == "even")
        .unwrap()["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

fn p5_psbt() -> String {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    v["finalized_psbt_b64"].as_str().unwrap().to_string()
}

fn tmp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Collapse every run of whitespace to one space (the warning blocks wrap).
fn flat(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Replace the character at 0-based `i` of `line` with `c`.
fn put(line: &str, i: usize, c: char) -> String {
    let mut v: Vec<char> = line.chars().collect();
    v[i] = c;
    v.into_iter().collect()
}

/// Substitute the next charset symbol at each 0-based offset.
fn damage(line: &str, at: &[usize]) -> String {
    let mut v: Vec<char> = line.chars().collect();
    for &i in at {
        let n = CS.find(v[i]).unwrap();
        v[i] = CS.as_bytes()[(n + 1) % 32] as char;
    }
    v.into_iter().collect()
}

fn run(verb: &str, extra: &[&str], text: &str) -> std::process::Output {
    let f = tmp(text);
    mt().arg(verb)
        .env("PATH", "/nonexistent")
        .args(extra)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap()
}

fn encode(extra: &[&str]) -> std::process::Output {
    let f = tmp(&p5_psbt());
    mt().arg("encode")
        .env("PATH", "/nonexistent")
        .args(extra)
        .arg("--in")
        .arg(f.path())
        .output()
        .unwrap()
}

/// The even set, offset 7 of string 1 (`j`, inside the invariant prefix) read as `x`.
fn even_prefix_damaged() -> Vec<String> {
    let mut st = even();
    assert_eq!(&st[0][7..8], "j");
    st[0] = put(&st[0], 7, 'x');
    st
}

fn join(lines: &[String]) -> String {
    lines.join("\n") + "\n"
}

// ---- P1: the printed set prefix (I8, T19, sweep row 14) -------------------------

#[test]
fn bh1119_mt_f_decode_prefix_is_corrected() {
    let o = run("decode", &[], &join(&even_prefix_damaged()));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("mt1 SET   6 strings, 1..6 all present, all begin mt1p9h8jqq9\n"),
        "corrected prefix absent:\n{err}"
    );
    assert!(!err.contains("mt1p9h8xqq9"), "typed prefix printed:\n{err}");
}

#[test]
fn bh1119_mt_f_decode_json_prefix_is_corrected() {
    let o = run("decode", &["--json"], &join(&even_prefix_damaged()));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let all = s(&o.stdout) + &s(&o.stderr);
    assert!(
        all.contains("\"set_prefix\": \"p9h8jqq9\","),
        "corrected json prefix absent:\n{all}"
    );
}

#[test]
fn bh1119_mt_f_inspect_prefix_is_corrected() {
    let o = run("inspect", &[], &join(&even_prefix_damaged()));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let all = s(&o.stdout) + &s(&o.stderr);
    assert!(
        all.contains("mt1 SET   6 strings, 1..6 all present, all begin mt1p9h8jqq9\n"),
        "corrected prefix absent:\n{all}"
    );
    assert!(!all.contains("mt1p9h8xqq9"), "typed prefix printed:\n{all}");
}

#[test]
fn bh1119_mt_f_inspect_json_prefix_is_corrected() {
    let o = run("inspect", &["--json"], &join(&even_prefix_damaged()));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let all = s(&o.stdout) + &s(&o.stderr);
    assert!(
        all.contains("\"set_prefix\": \"p9h8jqq9\","),
        "corrected json prefix absent:\n{all}"
    );
}

/// An unreadable first line followed by the whole set: the set decodes from the
/// other six, and the prefix must not come from the line mt could not read.
#[test]
fn bh1119_mt_f_prefix_not_from_unreadable_line() {
    let st = even();
    let garbage = st[0].replacen("mt1p9h8jqq9qq", "mt1zzzzzzqq9qq", 1);
    let mut lines = vec![garbage];
    lines.extend(st.iter().cloned());
    let o = run("decode", &[], &join(&lines));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("all begin mt1p9h8jqq9\n"),
        "set prefix absent:\n{err}"
    );
    assert!(
        !err.contains("mt1zzzzzzqq"),
        "unreadable line's prefix printed:\n{err}"
    );
}

// ---- P2: the restore source for elided lines (I7) -------------------------------

fn elided_after(full: &str) -> Vec<String> {
    let st = even();
    let mut v = vec![full.to_string()];
    v.extend(st[1..].iter().map(|l| l[ELIDED_DROP..].to_string()));
    v
}

#[test]
fn bh1119_mt_f_elided_restore_uses_corrected_prefix() {
    let lines = elided_after(&even_prefix_damaged()[0]);
    let o = run("decode", &[], &join(&lines));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("CORRECTION APPLIED. 1 chunk needed repair:\n"),
        "only the full plate carries the damage:\n{err}"
    );
    assert_eq!(
        err.matches("pos   8   read x, corrected to j").count(),
        1,
        "the damage was reported on plates that do not carry it:\n{err}"
    );
    assert!(
        err.contains("all begin mt1p9h8jqq9\n"),
        "set prefix:\n{err}"
    );
}

#[test]
fn bh1119_mt_f_full_plate_prefix_damage_does_not_spend_elided_budget() {
    let mut lines = elided_after(&even_prefix_damaged()[0]);
    lines[1] = damage(&lines[1], &[5, 10, 15, 20]);
    let o = run("decode", &[], &join(&lines));
    assert_eq!(
        o.status.code(),
        Some(REPAIRED),
        "each plate is within its 4-symbol budget:\n{}",
        s(&o.stderr)
    );
    let clean = run("decode", &[], &join(&even()));
    assert!(clean.status.success());
    assert_eq!(
        s(&o.stdout),
        s(&clean.stdout),
        "decoded transaction differs"
    );
    let err = s(&o.stderr);
    assert!(
        err.contains("CORRECTION APPLIED. 2 chunks needed repair:\n"),
        "{err}"
    );
}

#[test]
fn bh1119_mt_f_verify_full_plate_prefix_damage_within_budget() {
    let mut lines = elided_after(&even_prefix_damaged()[0]);
    lines[1] = damage(&lines[1], &[5, 10, 15, 20]);
    let o = run("verify", &[], &join(&lines));
    let err = s(&o.stderr);
    assert_eq!(
        o.status.code(),
        Some(REPAIRED),
        "each plate is within its 4-symbol budget:\n{err}"
    );
    assert!(
        err.contains("mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives."),
        "{err}"
    );
}

#[test]
fn bh1119_mt_f_transliterated_prefix_noted_once() {
    let st = even();
    assert_eq!(&st[0][6..7], "8");
    let lines = elided_after(&put(&st[0], 6, 'b'));
    let o = run("decode", &[], &join(&lines));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert_eq!(
        err.matches("you typed b, mt read it as 8").count(),
        1,
        "one typed character, reported once:\n{err}"
    );
}

/// CONTROL: a first full line no repair reaches still supplies its typed prefix,
/// as on the base; the set decodes from its second, clean copy.
#[test]
fn bh1119_mt_f_undecodable_full_line_falls_back_to_typed_prefix() {
    let st = even();
    let broken = damage(&st[0], &[20, 25, 30, 35, 40]);
    let mut lines = vec![broken, st[0].clone()];
    lines.extend(st[1..].iter().map(|l| l[ELIDED_DROP..].to_string()));
    let o = run("decode", &[], &join(&lines));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let clean = run("decode", &[], &join(&even()));
    assert_eq!(
        s(&o.stdout),
        s(&clean.stdout),
        "decoded transaction differs"
    );
}

// ---- C: encode's counts under --elide-prefix (I21, T14, sweep row 13) -----------

fn emitted_chars(stdout: &str, sep: &str) -> usize {
    stdout
        .lines()
        .map(|l| l.replace(sep, "").chars().count())
        .sum()
}

#[test]
fn bh1119_mt_f_cut_counts_elided_characters() {
    let o = encode(&["--elide-prefix"]);
    assert!(o.status.success(), "{}", s(&o.stderr));
    let out = s(&o.stdout);
    let err = s(&o.stderr);
    assert_eq!(emitted_chars(&out, " "), 699);
    assert!(
        err.contains("CUT       9 strings, 699 characters\n"),
        "CUT must count the characters printed:\n{err}"
    );
}

#[test]
fn bh1119_mt_f_count_each_string_matches_elided_plates() {
    let o = encode(&["--elide-prefix"]);
    assert!(o.status.success(), "{}", s(&o.stderr));
    let lens: Vec<usize> = s(&o.stdout).lines().map(|l| l.chars().count()).collect();
    assert_eq!(lens, vec![88, 77, 77, 77, 77, 77, 77, 77, 72]);
    let err = flat(&s(&o.stderr));
    assert!(
        err.contains("Count each string: string 1 is 88, strings 2-8 are 77, string 9 is 72."),
        "the counting instruction must match the plates:\n{err}"
    );
}

#[test]
fn bh1119_mt_f_cut_ignores_group_separators() {
    let o = encode(&["--elide-prefix", "--group-size", "4"]);
    assert!(o.status.success(), "{}", s(&o.stderr));
    let out = s(&o.stdout);
    let err = s(&o.stderr);
    assert_eq!(emitted_chars(&out, " "), 699);
    assert!(
        err.contains("CUT       9 strings, 699 characters\n"),
        "{err}"
    );
    assert!(
        flat(&err)
            .contains("Count each string: string 1 is 88, strings 2-8 are 77, string 9 is 72."),
        "{err}"
    );
}

/// CONTROL (T14): the PREFIX row keeps the full-string prefix the holder restores.
#[test]
fn bh1119_mt_f_prefix_row_unchanged_under_elision() {
    let o = encode(&["--elide-prefix"]);
    assert!(o.status.success(), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("PREFIX    all 9 strings begin mt1pgej7qqg — strings sharing that\n"),
        "{err}"
    );
}

/// CONTROL: without --elide-prefix nothing changes.
#[test]
fn bh1119_mt_f_unelided_counts_unchanged() {
    let o = encode(&[]);
    assert!(o.status.success(), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert_eq!(emitted_chars(&s(&o.stdout), " "), 787);
    assert!(
        err.contains("CUT       9 strings, 787 characters\n"),
        "{err}"
    );
    assert!(
        flat(&err).contains("Count each string: strings 1-8 are 88, string 9 is 83."),
        "{err}"
    );
}

/// P2 (review r1 M1): a later full copy that resolves supplies the prefix when
/// the first `mt1` line does not resolve (the second-copy drawer, journey C).
#[test]
fn bh1119_mt_f_second_full_copy_supplies_prefix() {
    let st = even();
    let broken = damage(&st[0], &[7, 20, 25, 30, 35]);
    let mut lines = vec![broken, st[0].clone()];
    lines.extend(st[1..].iter().map(|l| l[ELIDED_DROP..].to_string()));
    lines[2] = damage(&lines[2], &[5, 10, 15, 20]);
    let o = run("decode", &[], &join(&lines));
    assert_eq!(
        o.status.code(),
        Some(REPAIRED),
        "the clean copy supplies the prefix:\n{}",
        s(&o.stderr)
    );
    assert!(
        s(&o.stderr).contains("CORRECTION APPLIED. 1 chunk needed repair:\n"),
        "{}",
        s(&o.stderr)
    );
}

// ---- E: positions on an elided plate (Brian 2026-11-22 ruling, SPEC §3b) --------

/// The even set with strings 2-6 elided, as `mt encode --elide-prefix` prints it.
fn even_elided() -> Vec<String> {
    let st = even();
    let mut lines = vec![st[0].clone()];
    lines.extend(st[1..].iter().map(|l| l[ELIDED_DROP..].to_string()));
    lines
}

/// One substitution at the 6th character of elided plate 2 prints `pos   6`,
/// the position counted from the first character engraved on that plate.
#[test]
fn bh1119_mt_f_elided_correction_prints_plate_position() {
    let mut lines = even_elided();
    assert_eq!(&lines[1][5..6], "d");
    lines[1] = damage(&lines[1], &[5]);
    let o = run("decode", &[], &join(&lines));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("              pos   6   read w, corrected to d\n"),
        "the plate position, not the restored-string position:\n{err}"
    );
    assert!(!err.contains("pos  17"), "{err}");
}

/// A typed `o` for the `0` at the 15th character of elided plate 2 prints `pos  15`.
#[test]
fn bh1119_mt_f_elided_transliteration_prints_plate_position() {
    let mut lines = even_elided();
    assert_eq!(&lines[1][14..15], "0");
    lines[1] = put(&lines[1], 14, 'o');
    let o = run("decode", &[], &join(&lines));
    assert!(o.status.success(), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("  pos  15   you typed o, mt read it as 0\n"),
        "the plate position, not the restored-string position:\n{err}"
    );
    assert!(!err.contains("pos  26"), "{err}");
}

/// CONTROL: the same character on a full plate 2 keeps its whole-string position
/// (a full plate begins `mt1`, so plate and string positions coincide).
#[test]
fn bh1119_mt_f_full_plate_correction_keeps_string_position() {
    let mut st = even();
    assert_eq!(&st[1][16..17], "d");
    st[1] = damage(&st[1], &[16]);
    let o = run("decode", &[], &join(&st));
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert!(
        err.contains("              pos  17   read w, corrected to d\n"),
        "{err}"
    );
}

/// CONTROL: with no full plate the v0.1 §3b answer is unchanged: it names the 8
/// characters following `mt1` and says the search is not implemented.
#[test]
fn bh1119_mt_f_all_elided_still_names_the_prefix() {
    let lines: Vec<String> = even_elided()[1..].to_vec();
    let o = run("decode", &[], &join(&lines));
    assert_eq!(o.status.code(), Some(INVALID), "{}", s(&o.stderr));
    assert!(o.stdout.is_empty());
    assert!(
        flat(&s(&o.stderr)).contains(
            "Add the 8 characters following `mt1` on any intact string of the same set. \
             (They are recoverable by search if that string is lost — mt v0.1 does not \
             implement the search.)"
        ),
        "{}",
        s(&o.stderr)
    );
}

// ---- E1, `--json` (Brian 2026-11-22, #23 Q1): the warning string counts the plate --

/// Even set, strings 2-6 elided, a typed `o` for the `0` at the 15th character of
/// elided plate 2 (row 18's input).
fn elided_plate2_typed_o() -> String {
    let mut lines = even_elided();
    assert_eq!(&lines[1][14..15], "0");
    lines[1] = put(&lines[1], 14, 'o');
    join(&lines)
}

/// The `warnings` array of a `--json` document, exactly.
fn json_warnings(doc: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(doc).unwrap_or_else(|e| panic!("{e}:\n{doc}"));
    v["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("no warnings array:\n{doc}"))
        .iter()
        .map(|w| w.as_str().unwrap().to_string())
        .collect()
}

/// `mt decode --json` (document on stderr): the transliteration warning prints
/// the plate position, as the human row does (row 18).
#[test]
fn bh1119_mt_f_decode_json_elided_transliteration_prints_plate_position() {
    let o = run("decode", &["--json"], &elided_plate2_typed_o());
    assert!(o.status.success(), "{}", s(&o.stderr));
    assert_eq!(
        json_warnings(&s(&o.stderr)),
        [
            "no bitcoind reachable: nothing here is confirmed against the chain",
            "pos 15: you typed o, mt read it as 0",
        ],
        "the plate position, not the restored-string position (26)"
    );
}

/// `mt inspect --json` (document on stdout): the same warning, the same position.
#[test]
fn bh1119_mt_f_inspect_json_elided_transliteration_prints_plate_position() {
    let o = run("inspect", &["--json"], &elided_plate2_typed_o());
    assert!(o.status.success(), "{}", s(&o.stderr));
    assert_eq!(
        json_warnings(&s(&o.stdout)),
        [
            "no bitcoind reachable: nothing here is confirmed against the chain",
            "pos 15: you typed o, mt read it as 0",
        ],
        "the plate position, not the restored-string position (26)"
    );
}

// ---- Q2 (Brian 2026-11-22, #23 Q2): a typo inside the restored prefix prints once --

/// Input B of fold r1b: even string 1 with the next charset symbol at 0-based
/// offsets 20, 25, 30, 35, 40 (unreadable: over budget) and its offset 6 `8`
/// typed `b` (inside the prefix), then all six strings elided. P2 finds no full
/// line that resolves and falls back to the typed prefix, so every elided line
/// inherits the `b`.
fn unreadable_full_typed_b_then_all_elided() -> String {
    let st = even();
    assert_eq!(&st[0][6..7], "8");
    let mut lines = vec![put(&damage(&st[0], &[20, 25, 30, 35, 40]), 6, 'b')];
    lines.extend(st.iter().map(|l| l[ELIDED_DROP..].to_string()));
    join(&lines)
}

/// The `pos` warnings of a `--json` warnings array, and the rest unchanged.
fn assert_one_prefix_warning(w: &[String]) {
    let pos: Vec<&String> = w.iter().filter(|x| x.starts_with("pos ")).collect();
    assert_eq!(
        pos,
        ["pos 7: you typed b, mt read it as 8"],
        "once, at the full plate's position, none for the elided plates:\n{w:#?}"
    );
    let rest: Vec<&String> = w.iter().filter(|x| !x.starts_with("pos ")).collect();
    assert_eq!(
        rest,
        [
            "no bitcoind reachable: nothing here is confirmed against the chain",
            "string 1 could not be read: character 'b' at data-part offset 3 is not in the bech32 alphabet",
        ],
        "{w:#?}"
    );
}

/// Text: exactly one transliteration row, `pos   7` on the full plate; the six
/// elided plates print none for it.
#[test]
fn bh1119_mt_f_prefix_transliteration_inherited_by_elided_prints_once() {
    let o = run("decode", &[], &unreadable_full_typed_b_then_all_elided());
    assert!(o.status.success(), "{}", s(&o.stderr));
    let err = s(&o.stderr);
    assert_eq!(
        err.matches("  pos   7   you typed b, mt read it as 8\n")
            .count(),
        1,
        "once, on the full plate:\n{err}"
    );
    assert_eq!(err.matches("you typed").count(), 1, "{err}");
}

/// `mt decode --json` (document on stderr): exactly one `pos 7` warning.
#[test]
fn bh1119_mt_f_decode_json_prefix_transliteration_inherited_by_elided_prints_once() {
    let o = run(
        "decode",
        &["--json"],
        &unreadable_full_typed_b_then_all_elided(),
    );
    assert!(o.status.success(), "{}", s(&o.stderr));
    assert_one_prefix_warning(&json_warnings(&s(&o.stderr)));
}

/// `mt inspect --json` (document on stdout): the same.
#[test]
fn bh1119_mt_f_inspect_json_prefix_transliteration_inherited_by_elided_prints_once() {
    let o = run(
        "inspect",
        &["--json"],
        &unreadable_full_typed_b_then_all_elided(),
    );
    assert!(o.status.success(), "{}", s(&o.stderr));
    assert_one_prefix_warning(&json_warnings(&s(&o.stdout)));
}

// ---- Q3 (Brian 2026-11-22, #23 Q3): the full plate is unreadable ------------------

/// Input A of fold r1b: even string 1 with the next charset symbol at 0-based
/// offsets 7 (`j` typed `n`, inside the prefix), 20, 25, 30, 35 (unreadable: over
/// budget), then all six strings elided. P2 falls back to the typed prefix, so
/// every elided line inherits the `n` and each chunk's BCH spends one symbol on it.
fn unreadable_full_prefix_n_then_all_elided() -> String {
    let st = even();
    assert_eq!(&st[0][7..8], "j");
    let mut lines = vec![damage(&st[0], &[7, 20, 25, 30, 35])];
    assert_eq!(&lines[0][..11], "mt1p9h8nqq9");
    lines.extend(st.iter().map(|l| l[ELIDED_DROP..].to_string()));
    join(&lines)
}

/// The CORRECTION block after the fix (Brian #23 Q3): the real chunk count, each
/// chunk's `1 of 4 symbols` with no `pos` row, and the one `pos   8` row under a
/// line naming the full plate, after the chunk lines.
const Q3_BLOCK: &str = "\nCORRECTION APPLIED. 6 chunks needed repair:\n\
  \x20 chunk   1   1 of 4 symbols\n\
  \x20 chunk   2   1 of 4 symbols\n\
  \x20 chunk   3   1 of 4 symbols\n\
  \x20 chunk   4   1 of 4 symbols\n\
  \x20 chunk   5   1 of 4 symbols\n\
  \x20 chunk   6   1 of 4 symbols\n\
  \x20 string 1 of the input   its prefix was restored onto the elided strings\n\
  \x20             pos   8   read n, corrected to j\n\n";

/// Shared by the decode, verify and inspect text rows.
fn assert_q3_block(err: &str) {
    assert_eq!(
        err.matches("pos   8   read n, corrected to j\n").count(),
        1,
        "once, under the line naming the full plate:\n{err}"
    );
    for n in 1..=6 {
        assert_eq!(
            err.matches(&format!("\n  chunk   {n}   1 of 4 symbols\n"))
                .count(),
            1,
            "chunk {n} keeps its line:\n{err}"
        );
    }
    assert_eq!(
        err.matches(Q3_BLOCK).count(),
        1,
        "the block, exactly:\n{err}"
    );
}

#[test]
fn bh1119_mt_f_unreadable_full_plate_prefix_correction_prints_once() {
    let o = run("decode", &[], &unreadable_full_prefix_n_then_all_elided());
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    assert_q3_block(&s(&o.stderr));
}

#[test]
fn bh1119_mt_f_verify_unreadable_full_plate_prefix_correction_prints_once() {
    let o = run("verify", &[], &unreadable_full_prefix_n_then_all_elided());
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    assert_q3_block(&s(&o.stderr));
}

#[test]
fn bh1119_mt_f_inspect_unreadable_full_plate_prefix_correction_prints_once() {
    let o = run("inspect", &[], &unreadable_full_prefix_n_then_all_elided());
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    assert_q3_block(&s(&o.stderr));
}

/// CONTROL (`mt decode --json`, document on stderr): the correction warnings
/// carry no position and stay as they are (Brian #23 Q3 (d)).
#[test]
fn bh1119_mt_f_decode_json_unreadable_full_plate_correction_warnings_unchanged() {
    let o = run(
        "decode",
        &["--json"],
        &unreadable_full_prefix_n_then_all_elided(),
    );
    assert_eq!(o.status.code(), Some(REPAIRED), "{}", s(&o.stderr));
    assert_eq!(
        json_warnings(&s(&o.stderr)),
        [
            "no bitcoind reachable: nothing here is confirmed against the chain",
            "string 1 could not be read: BCH correction failed: regular code: more than 4 substitutions or pathological pattern",
            "chunk 1 needed 1 of 4 corrections",
            "chunk 2 needed 1 of 4 corrections",
            "chunk 3 needed 1 of 4 corrections",
            "chunk 4 needed 1 of 4 corrections",
            "chunk 5 needed 1 of 4 corrections",
            "chunk 6 needed 1 of 4 corrections",
        ]
    );
}
