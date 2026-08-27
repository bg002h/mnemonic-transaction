//! §8.2f's PURGE RECIPE, RUN — P1 row 8.
//!
//! `mt` refuses bearer material on argv and then tells the operator how to get
//! it out of their shell history. What it told them was
//! `history -d $HISTCMD && fc -W` for zsh and
//! `history delete --contains <tx>` for fish, and **both are the defect they
//! are meant to remedy**:
//!
//! - on zsh 5.9.2 `history -d` prints timestamps; the builtin rejects the
//!   invocation, the operator sees no complaint they can act on, and the entry
//!   stays exactly where it was;
//! - every fish `history delete` spelling has to be handed the material to
//!   match on, at a prompt that records what is typed — so it removes one copy
//!   of the secret by writing a second.
//!
//! Both are named, by name, in `mnemonic_io_lib::remedy`'s own module header as
//! the reason `mt`'s text is not a source for the shared crate. Row 8 is `mt`
//! adopting the crate's text instead of keeping its own.
//!
//! ## What makes this a gate rather than a restatement
//!
//! **The recipe is taken out of `mt`'s own stderr and executed.** A test that
//! runs a hard-coded copy of a recipe proves the copy works; the operator is
//! handed what the binary printed, so that is what has to be run. And it is run
//! under a real interactive zsh on a pty, because the whole class of defect
//! here — an entry that is still in MEMORY while the file is edited — cannot
//! exist in a non-interactive shell that records no history at all.
//!
//! **The control runs first and is load-bearing.** A harness that records no
//! history reports "purged" for every recipe, including one that does nothing.
//! That is how F-273 came to be deferred rather than answered in the donor
//! crate, and the control is what caught it.
//!
//! **Two invocation shapes, and the second is the one a naive fix fails.** The
//! recipes match on the COMMAND — never on the secret, which is how an operator
//! types it into history a second time — so the pattern has to match the line
//! that actually leaked. `mt encode <material>` leaks a line beginning
//! `mt encode`; `mt <material>` leaks one beginning `mt` with no verb at all,
//! because §8.2f runs BEFORE clap and does not need a subcommand to fire. A
//! surface fixed at `mt <verb>` matches the first and misses the second, and a
//! recipe that misses is a recipe that reports success and purges nothing.
#![cfg(unix)]

use mnemonic_io_lib::remedy;
use std::process::{Command, Stdio};

/// 120 hex characters, carrying a marker that can be grepped for in a history
/// file. It is not a real transaction and does not need to be: §8.2f fires on
/// the SHAPE, before anything is parsed.
///
/// **The length is measured, not eyeballed.** The first draft of this constant
/// was 98 characters, and `looks_like_a_transaction`'s raw-hex arm requires
/// **100 or more** — so every gate below failed with clap echoing the material
/// back at exit 2, which is the leak the guard exists to prevent rather than
/// the RED these tests were written for.
const MATERIAL: &str = "cafef00d0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

/// The marker searched for in `HISTFILE`. Nothing else in the session contains
/// it.
const MARKER: &str = "cafef00d";

/// Planted alongside the leak so the recipe's BREADTH is measured rather than
/// assumed. The zsh recipe is targeted, so this must SURVIVE — a recipe that
/// took the whole session with it would pass a "the secret is gone" assertion
/// while destroying the operator's history.
const NEIGHBOUR: &str = "echo an-unrelated-neighbouring-command";

/// The timeout a fish session is pinned at. It must comfortably exceed fish's
/// own device-attribute wait (~10s under a `script` pty) or a slow *start*
/// would be mistaken for a hang.
const FISH_TIMEOUT_SECS: u32 = 30;

fn require(bin: &str, why: &str) -> String {
    assert!(
        std::path::Path::new(bin).exists(),
        "{bin} is required: {why} This is deliberately a FAILURE and not a skip -- \
         a skipped gate prints ok and exit 0. If CI lacks it, install it there rather \
         than weakening this."
    );
    bin.to_string()
}

/// Run `mt` with `args` and return its stderr. The material is passed on argv
/// **on purpose**: that is the invocation §8.2f exists to refuse.
fn mt_stderr(args: &[&str]) -> String {
    let out = Command::new(assert_cmd::cargo::cargo_bin("mt"))
        .args(args)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        err.contains("§8.2f"),
        "the guard did not fire for `mt {}`, so there is no emitted recipe to run:\n{err}",
        args.join(" ")
    );
    assert!(
        !err.contains(MATERIAL),
        "the refusal echoed the material back, which is the leak it exists to \
         prevent:\n{err}"
    );
    err
}

/// Pull one shell's recipe **out of what the binary printed**.
///
/// Not out of `remedy::history_purge_recipes` — the operator runs what is on
/// their screen, so that is what this file executes. The block is emitted
/// through `Refusal`'s `verbatim` channel precisely so these lines survive
/// unwrapped; if they are ever re-wrapped this function stops finding a recipe
/// and every gate below goes red, which is the correct outcome.
fn recipe_from_stderr(err: &str, shell: &str) -> String {
    let tag = format!("{shell}:");
    let line = err
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(&tag))
        .unwrap_or_else(|| {
            panic!("no `{tag}` recipe line in mt's stderr:\n{err}");
        });
    let recipe = line[tag.len()..].trim().to_string();
    assert!(
        !recipe.is_empty(),
        "the `{tag}` line carried no recipe:\n{err}"
    );
    recipe
}

/// Plant `NEIGHBOUR`, then `planted`, then run `recipe`, in one interactive zsh
/// on a pty. Returns `HISTFILE` **after the shell has exited**, because that is
/// when a shell writes back what it was still holding in memory — the trap the
/// emitted text is named for.
fn zsh_history_after(planted: &str, recipe: &str) -> String {
    let zsh = require(
        "/usr/bin/zsh",
        "row 8's gate is 'the recipe mt EMITS, run under a real interactive zsh, \
         actually removes the entry', and there is no way to run it without zsh.",
    );
    let script = require(
        "/usr/bin/script",
        "a shell only records history when it believes it is interactive, which needs a pty.",
    );

    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::write(
        d.join(".zshrc"),
        "HISTFILE=$ZDOTDIR/histfile\nHISTSIZE=1000\nSAVEHIST=1000\n",
    )
    .unwrap();
    std::fs::write(d.join("histfile"), "").unwrap();
    let input = d.join("in.zsh");
    std::fs::write(&input, format!("{NEIGHBOUR}\n{planted}\n{recipe}\n")).unwrap();

    let st = Command::new(script)
        .arg("-qec")
        .arg(format!("{zsh} -i -s < '{}'", input.display()))
        .arg("/dev/null")
        .env("ZDOTDIR", d)
        .env("HOME", d)
        // `mt` IS on this machine's PATH, and the planted line is typed at a
        // live prompt. A harness that runs the binary under test would be
        // measuring it as well as the recipe -- and would put a second refusal
        // in the typescript. /usr/bin and /bin carry `sed`, which the recipe
        // needs, and no `mt`.
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("`script` (util-linux) is required to give zsh a pty");
    assert!(
        st.status.code().is_some(),
        "the zsh session was killed rather than exiting; nothing can be concluded"
    );
    std::fs::read_to_string(d.join("histfile")).unwrap()
}

/// **THE CONTROL, and it runs first for a reason.** A harness that fails to
/// record history at all reports "purged" for every recipe including a broken
/// one — which is how the donor crate's first draft of this shape passed while
/// measuring nothing.
#[test]
fn the_harness_records_history_at_all() {
    let h = zsh_history_after(
        &format!("mt encode {MATERIAL}"),
        "true nothing-was-purged-here",
    );
    assert!(
        h.contains(MARKER),
        "with NO purge attempt the planted material must reach disk, or this file \
         is measuring itself rather than the recipe. HISTFILE was:\n{h}"
    );
    assert!(
        h.contains(NEIGHBOUR),
        "the neighbouring command must be recorded too, or the breadth assertion \
         below is vacuous. HISTFILE was:\n{h}"
    );
}

/// **F-264's reproduction, kept as a test.** Editing the history FILE while the
/// entry is still in MEMORY changes nothing — and reports success. It is the
/// reason the emitted zsh recipe has five steps rather than one, and if it ever
/// stops holding, zsh's save semantics changed and the recipe may be
/// simplifiable. Re-measure before simplifying.
#[test]
fn editing_the_history_file_alone_is_the_trap_the_emitted_text_warns_about() {
    let h = zsh_history_after(
        &format!("mt encode {MATERIAL}"),
        "sed -i '/\\bmt\\b/d' \"$HISTFILE\"",
    );
    assert!(
        h.contains(MARKER),
        "if this ever stops holding, re-measure zsh's save semantics before \
         shortening the recipe. HISTFILE was:\n{h}"
    );
}

/// **THE GATE.** The zsh recipe `mt` prints, run under a real interactive zsh,
/// removes the line that leaked — **for both invocation shapes**.
///
/// `mt <material>` with no verb is not an exotic spelling: §8.2f runs before
/// clap, so it fires on an argv clap would have rejected, and `md verify
/// <STRINGS>` / `mk verify [MK1_STRINGS]` are the habits that produce it. A
/// purge surface fixed at `mt <verb>` matches the first row here and misses the
/// second.
#[test]
fn the_emitted_zsh_recipe_purges_the_line_that_leaked_it() {
    for argv in [
        vec!["encode", MATERIAL],
        vec![MATERIAL],
        vec!["verify", MATERIAL],
    ] {
        let planted = format!("mt {}", argv.join(" "));
        let err = mt_stderr(&argv);
        let recipe = recipe_from_stderr(&err, "zsh");
        let h = zsh_history_after(&planted, &recipe);
        assert!(
            !h.contains(MARKER),
            "for `{planted}` the emitted zsh recipe\n    {recipe}\nreported success \
             and purged nothing. HISTFILE after the session exited was:\n{h}"
        );
        assert!(
            h.contains(NEIGHBOUR),
            "the zsh recipe is a TARGETED delete, and it took an unrelated command \
             with it. A recipe that clears the whole history would pass the \
             assertion above while destroying the operator's session. HISTFILE \
             was:\n{h}"
        );
    }
}

/// The recipes `mt` emits are the shared crate's, byte for byte — not a copy
/// that can drift out of step with the harness that measured them.
///
/// The surface is `mt` plus the verb **when a verb was typed**, and bare `mt`
/// otherwise, which is `me`'s `argv_surface` rule reflected: the words come
/// from a four-item allowlist, so the pattern can never carry material.
#[test]
fn the_emitted_recipes_are_the_shared_crates_and_not_a_copy() {
    for (argv, surface) in [
        (vec!["encode", MATERIAL], "mt encode"),
        (vec!["verify", MATERIAL], "mt verify"),
        (vec![MATERIAL], "mt"),
    ] {
        let err = mt_stderr(&argv);
        for (shell, expected) in remedy::history_purge_recipes(surface) {
            assert_eq!(
                recipe_from_stderr(&err, shell),
                expected,
                "`mt {}` must emit the crate's {shell} recipe for surface `{surface}`",
                argv.join(" ")
            );
        }
    }
}

/// **`history -d` is NAMED as a warning and OFFERED as no recipe**, asserted on
/// what `mt` printed.
///
/// The naive spelling — *stderr does not contain `history -d`* — goes RED
/// against the CORRECT text, because the text deliberately names the command in
/// order to warn against it. The only way to make that assertion green is to
/// delete the warning, recreating the defect. So the two halves are asserted
/// separately, and against structure rather than prose.
#[test]
fn history_d_is_named_as_a_warning_and_offered_as_no_recipe() {
    let err = mt_stderr(&["encode", MATERIAL]);
    assert!(
        err.contains("history -d"),
        "it must still be NAMED -- an operator who knows the command needs to be \
         told it does not work:\n{err}"
    );
    for shell in ["zsh", "bash", "fish"] {
        let recipe = recipe_from_stderr(&err, shell);
        assert!(
            !recipe.contains("history -d"),
            "{shell}'s emitted recipe OFFERS `history -d`, which on zsh prints \
             timestamps and deletes nothing: {recipe}"
        );
    }
}

// ── the fish half ────────────────────────────────────────────────────────────

/// Plant `NEIGHBOUR`, then `planted`, then `history save`, then `recipe`, in one
/// interactive fish on a pty. `None` runs no recipe at all — the control.
///
/// `TERM` is a real terminal on purpose: under `TERM=dumb` fish skips the
/// prompt machinery entirely, which is where half of this behaviour lives. The
/// cost is fish's ~10s wait for a device-attribute reply `script`'s pty never
/// sends.
fn fish_history_after(planted: &str, recipe: Option<&str>) -> (String, bool) {
    let fish = require(
        "/usr/bin/fish",
        "row 8's gate covers the fish recipe too, and there is no way to run it \
         without fish.",
    );
    let script = require(
        "/usr/bin/script",
        "fish only records history when it believes it is interactive, which needs a pty.",
    );
    let timeout = require(
        "/usr/bin/timeout",
        "a fish `history delete` spelling can block on a prompt, so the harness must \
         be able to survive one.",
    );

    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let data = d.join("data");
    let config = d.join("config");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&config).unwrap();

    let mut cmds = format!("{NEIGHBOUR}\n{planted}\nhistory save\n");
    if let Some(r) = recipe {
        cmds.push_str(r);
        cmds.push('\n');
    }
    cmds.push_str("exit\n");
    let cmds_path = d.join("cmds.fish");
    std::fs::write(&cmds_path, cmds).unwrap();

    let st = Command::new(timeout)
        .arg(FISH_TIMEOUT_SECS.to_string())
        .arg(script)
        .arg("-qc")
        .arg(format!("{fish} -i"))
        .arg(d.join("typescript"))
        .stdin(Stdio::from(std::fs::File::open(&cmds_path).unwrap()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env("HOME", d)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_CONFIG_HOME", &config)
        .env("PATH", "/usr/bin:/bin")
        .env("TERM", "xterm")
        .status()
        .expect("`script` (util-linux) is required to give fish a pty");

    let history =
        std::fs::read_to_string(data.join("fish").join("fish_history")).unwrap_or_default();
    // `timeout` reports a killed child as 124.
    (history, st.code() == Some(124))
}

/// **THE fish CONTROL.** Nothing below it means anything until it passes.
#[test]
fn the_fish_harness_records_history_at_all() {
    let (h, timed_out) = fish_history_after(&format!("mt encode {MATERIAL}"), None);
    assert!(
        !timed_out,
        "the control session was killed at {FISH_TIMEOUT_SECS}s; nothing can be concluded"
    );
    assert!(
        h.contains(MARKER),
        "with NO purge attempt the planted material must reach disk, or this file is \
         measuring itself rather than the recipe. fish_history was:\n{h}"
    );
}

/// **THE fish GATE.** The recipe `mt` prints, run under a real interactive fish,
/// removes the entry — unattended, with nobody there to answer a prompt.
///
/// **The INVARIANT is asserted, never the mechanism.** `history delete --prefix`
/// hangs on fish 4.8.1 and returns unattended on 3.7.0, and in both it leaves
/// the secret; a test that pinned the hang would have passed locally and gone
/// red the first time CI ran it while the finding was sound. What must hold on
/// every fish is: the session finishes, and the material is gone.
#[test]
fn the_emitted_fish_recipe_purges_the_line_that_leaked_it() {
    let planted = format!("mt encode {MATERIAL}");
    let err = mt_stderr(&["encode", MATERIAL]);
    let recipe = recipe_from_stderr(&err, "fish");

    let (h, timed_out) = fish_history_after(&planted, Some(&recipe));
    assert!(
        !timed_out,
        "the emitted fish recipe `{recipe}` blocked until the session was killed at \
         {FISH_TIMEOUT_SECS}s. A recipe that waits for an answer is not one an \
         operator can be handed in a refusal message."
    );
    assert!(
        !h.contains(MARKER),
        "the emitted fish recipe reported success and purged nothing. fish_history \
         after the session exited was:\n{h}"
    );
}

/// The fish recipe's COST is measured, and the emitted text is required to state
/// it. `history clear-session` matches on nothing — which is why it needs to be
/// told nothing, and also why it takes the whole session with it.
///
/// The second half is the half that rots: the measurement stays true on its own,
/// while the sentence describing it is one tidy-up away from being deleted.
#[test]
fn the_fish_recipe_costs_the_whole_session_and_mts_text_says_so() {
    let planted = format!("mt encode {MATERIAL}");
    let err = mt_stderr(&["encode", MATERIAL]);
    let recipe = recipe_from_stderr(&err, "fish");

    let (h, _) = fish_history_after(&planted, Some(&recipe));
    assert!(
        !h.contains(NEIGHBOUR),
        "this assertion exists to FAIL if fish ever gains a targeted purge that does \
         not name the secret -- at which point the recipe should become that, and the \
         cost sentence should go. fish_history was:\n{h}"
    );
    assert!(
        err.contains("whole session"),
        "mt's own output must SAY that the fish recipe clears the session's entire \
         history, because it measurably does:\n{err}"
    );
}
