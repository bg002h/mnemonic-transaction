
### `no-file-mode-warning-on-windows` — §8.2g's permission warning is POSIX-only

- **Surfaced:** 2026-09-17, adding the cross-platform release matrix.
  `build mt (windows-x86_64)` failed to compile:
  `error[E0425]: cannot find function file_mode_warning in module validate`.
- **Cause:** `validate::file_mode_warning` is `#[cfg(unix)]` (it inspects
  `0o077` mode bits) but `main.rs:414` called it unconditionally. A
  `#[cfg(not(unix))]` arm returning `None` now exists, which is **exactly what
  the unix arm already returns** when it cannot stat the source — a pipe, a
  terminal, a failed `metadata`. This adds no new silent path; it reuses the
  existing unknown one.
- **The arm now WARNS rather than returning `None`.** The first version returned
  `None`, matching what unix returns when it cannot stat the source. That was
  wrong in a way worth naming: on unix `None` means *"this input has no mode to
  read"*; on Windows it would have meant *"mt did not look"* — rendered
  identically to *"mt looked and it was fine"*. Silence indistinguishable from an
  all-clear is the failure this module exists to avoid. It now says the check did
  not run, and that nothing is known either way.
- **Still open:** the real check is an NTFS ACL inspection, not a mode mask —
  different work than a cfg arm.
- **Not a downgrade of anything else:** every other refusal and warning `mt`
  makes is platform-independent and unaffected.
- **Status:** open.
- **Tier:** platform-support.

### `countmismatch-variant-blank-line` — CountMismatch variant snippet lacks the blank line the enum uses between variants

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-B).
- **Location:** FIX doc §2 R4 new text (`crates/mt-codec/src/error.rs`).
- **Scenario / text:** The §2 R4 new text puts `CountMismatch { .. }` directly
  under the previous variant; `cargo fmt` does not add it. Cosmetic.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `mixed-set-missing-chunk-wording` — Mixed-set holder sees "chunk 1 of 2 is missing" in the ordering case

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-B).
- **Location:** FIX doc §2 R4 ordering paragraph (`decode`, pre-loop `bpc`).
- **Scenario / text:** When the first readable string has the smaller count,
  the doc's decided ordering names a missing chunk rather than the count
  mismatch. Fails closed, decided in the doc; optional wording only.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `decode-prefix-after-bch-correction` — Wrong printed prefix after BCH correction

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-B), "Out of scope" bullet.
- **Text (verbatim):** Pre-existing on base, for filing; not graded here.
  Wrong printed prefix after BCH correction: a set whose first string has one
  error inside its first 8 data symbols (`mt1p9h8jxq9…` for `mt1p9h8jqq9…`,
  file /scratch/cache/bughunt/mt-fix-review-b/corr.txt). `mt decode` prints
  `all begin mt1p9h8jxq9` while `mt verify` prints `OK — 6 chunks, set
  0x2dcf2`, on base and with the fixes. Cause: `invariant_prefix(&strings[0])`
  reads the uncorrected input (main.rs:754/1265/1639). The holder is told to
  look for a prefix that no plate carries.
- **Status:** open (needs triage).
- **Owning phase:** none (not yet triaged).
