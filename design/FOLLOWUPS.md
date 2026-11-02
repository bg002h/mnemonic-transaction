
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

### `unreadable-branch-masks-mixed-count` — Unreadable-string explanation says "nothing is necessarily lost" for a mixed-count set

- **Surfaced:** 2026-11-02 whole-diff review (FIX-B), M1.
- **Location:** `crates/mt-cli/src/main.rs:1706-1790` (`explain_failure`).
- **Scenario / text:** A set with a count disagreement under one set id plus
  one unreadable string takes the unreadable branch ("every chunk COULD be
  here — nothing is necessarily lost", "Re-read that plate"). Fails closed
  (exit 1). Fix: in the unreadable branch, if `readable` holds two distinct
  `header.count` values, fall through to the generic refusal. SetIdMismatch
  has the same shape.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `countmismatch-text-noun` — CountMismatch text "chunk 2 carries 3" omits the noun

- **Surfaced:** 2026-11-02 whole-diff review (FIX-B), N1.
- **Location:** `crates/mt-codec/src/error.rs` CountMismatch display text.
- **Scenario / text:** Reads better as "carries a count of 3"; the FIX doc and
  RED rows pin the current wording, so a change needs a doc edit.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).
### `v0-one-element-witness-reads-as-taproot-keypath` — prevout-aware classification for PSBT input

- **Surfaced:** 2026-11-02, FIX-A doc §3.1 (residual accepted under QA1).
- **Location:** `crates/mt-cli/src/validate.rs`, `satisfaction_elements`
  (`w.len() == 1` gives `SigCtx::Taproot`).
- **Failure scenario:** a one-element v0 P2WSH witness (a no-argument
  witnessScript of exactly 64 or 65 bytes) reads as a taproot key path. It
  cannot be closed without the spent output.
- **Fix:** classify by the spent script when the PSBT carries the prevout.
- **Owner:** mt. **Owning phase:** none, ownerless residue.
- **Status:** open.

### `fixa-spec-overclaims-witnessscript-not-a-signature` — §2 spec delta overclaims

- **Surfaced:** 2026-11-02 FIX review r1 (M1), `design/FIX_bughunt_mt_a.md` §2 spec delta.
- **Location:** the sentence "a 64- or 65-byte witnessScript or public key is not a signature" (now in `design/SPEC_mt_v0_1.md` §8.6, "Script path and pushes, exactly").
- **Failure scenario:** the sentence overclaims. A v0 witnessScript of 64 or 65 bytes shaped `30 <n-3> .. 01` (a crafted push-48 script) reads as an ECDSA ALL signature, so a keyless spend encodes. Unusual conditions: (1) a keyless v0 P2WSH; (2) a witnessScript crafted to be DER-shaped. It sits inside §8.6's declared heuristic.
- **Fix:** replace that clause with "a 64- or 65-byte witnessScript or public key is not read as a Schnorr signature".
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-doc-arm-order-wording` — N1: §3.3 arm-order wording

- **Surfaced:** 2026-11-02 FIX review r1 (N1), `design/FIX_bughunt_mt_a.md` §3.3.
- **Location:** §3.3 "add these arms before `0x01..=0x4b`".
- **Failure scenario:** §3.3 says "add these arms before `0x01..=0x4b`" but lists `0x4e`, `0x4f` and `0x51..=0x60`, which the order line then places later. This is harmless, because the patterns are disjoint (pre-fold text).
- **Fix:** none given by the review (wording only).
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-spec-der-len-undefined` — N2: spec delta never defines `<len>`

- **Surfaced:** 2026-11-02 FIX review r1 (N2), `design/FIX_bughunt_mt_a.md` §2 spec delta.
- **Location:** §2 / `design/SPEC_mt_v0_1.md` §8.6 "`30 <len>`".
- **Failure scenario:** "`30 <len>`" never says that len is the element length minus 3, which is what the code checks (pre-fold text).
- **Fix:** none given by the review (state that len is the element length minus 3).
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-fold-report-wrong-failopen-class` — N3: fold report misnames 01f's class

- **Surfaced:** 2026-11-02 FIX review r1 (N3), `design/agent-reports/fixdoc-fold-a.md` line 18.
- **Location:** `fixdoc-fold-a.md` line 18.
- **Failure scenario:** "same fail-open class" is wrong. On base, both 0x51 and 0x87 fail closed with the wrong text, as the ruling says ("refused with the wrong text"). The doc's 01f row text is correct.
- **Fix:** none given by the review (report wording only).
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.
