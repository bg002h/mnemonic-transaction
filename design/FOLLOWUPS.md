
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

### `fixa-wd-p2sh-redeem-der-shaped` — whole-diff s1 M2: DER-shaped P2SH redeemScript

- **Surfaced:** 2026-11-02 FIX-A whole-diff s1 (M2), `design/agent-reports/whole-diff-fix-a-s1.md`.
- **Location:** `crates/mt-cli/src/validate.rs` `satisfaction_elements`, the scriptSig walk.
- **Failure scenario:** the last scriptSig push cannot be dropped the way a witness's last element is, because P2PK and bare multisig end in a signature. A keyless P2SH whose redeemScript is `30 <n-3>`-shaped still counts as signed. The base already accepted this. Unusual conditions: (1) a deliberately DER-shaped redeemScript; (2) a scriptSig with no real signature.
- **Fix:** none; file as a declared heuristic residual with the v0 one-element one (b660e03).
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-wd-pushdata-len-overflow` — whole-diff s1 N1: `skip + len` can overflow on 32-bit

- **Surfaced:** 2026-11-02 FIX-A whole-diff s1 (N1), `design/agent-reports/whole-diff-fix-a-s1.md`.
- **Location:** `crates/mt-cli/src/validate.rs` `script_sig_pushes`, `skip + len`.
- **Failure scenario:** the u32 PUSHDATA4 length is cast to usize, so `skip + len` can overflow on a 32-bit target.
- **Fix:** use `checked_add`, or compare `len` against `b.len()` before adding.
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-wd-scriptpath-no-mod-unpinned` — whole-diff s2 M1: control-block modulus unpinned

- **Surfaced:** 2026-11-02 FIX-A whole-diff s2 (M1), `design/agent-reports/whole-diff-fix-a-s2.md`.
- **Location:** `is_taproot_script_path`, `(last.len() - 33) % 32 == 0`.
- **Failure scenario:** removing the modulus survives all rows. Only a v0 witness whose last element starts `0xc0`/`0xc1` with a length not of the form `33 + 32m` is affected, and it fails closed in the other direction.
- **Fix:** add a row only if wanted.
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-wd-der-bounds-unpinned` — whole-diff s2 M2: DER range lower bound and length byte unpinned

- **Surfaced:** 2026-11-02 FIX-A whole-diff s2 (M2), `design/agent-reports/whole-diff-fix-a-s2.md`.
- **Location:** `sig_shape`, the Ecdsa arm.
- **Failure scenario:** changing the lower bound 9 to 10, or dropping the `e[1] == n - 3` check, survives all rows. These are pre-existing lines the FIX kept; no row pins a short or length-mismatched DER element.
- **Fix:** none given (add rows for a short and a length-mismatched element).
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-wd-sigshape-old-arms-equivalent` — whole-diff s2 N1: equivalent mutant in the Taproot arm

- **Surfaced:** 2026-11-02 FIX-A whole-diff s2 (N1), `design/agent-reports/whole-diff-fix-a-s2.md`.
- **Location:** `sig_shape`, Taproot arm 65.
- **Failure scenario:** restoring `65 if e[0]==0x30 => Ecdsa` survives and is equivalent, since `Ecdsa(b)` and `SchnorrExplicit(b)` share `sighash()` and both count as a signature. No defect.
- **Fix:** none.
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixa-wd-equivalent-mutants-4f-ctx` — whole-diff s2 N2: two further equivalent mutants

- **Surfaced:** 2026-11-02 FIX-A whole-diff s2 (N2), `design/agent-reports/whole-diff-fix-a-s2.md`.
- **Location:** `script_sig_pushes` OP_1NEGATE arm (pushed value `0x81` vs empty); `satisfaction_elements` (`ctx_scriptpath_len1only`).
- **Failure scenario:** both mutants are equivalent for every shape the rows can build: neither value is signature-shaped in the Ecdsa context. No defect.
- **Fix:** none.
- **Status:** `deferred-minor`. **Owning phase:** none, ownerless residue.

### `fixc-elided-prefix-source-nonascii` — Option (c) can pick a later non-ASCII full line as prefix source

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), M1.
- **Location / scenario / fix (review text, verbatim):** The option (c) selection can pick a later `mt1` line with non-ASCII characters past byte 11 as the prefix source. This fails closed: the restored lines do not decode, so it refuses. It needs two unusual conditions: (1) non-ASCII damage on the first line inside its first 11 bytes, and (2) a second full line with damage of its own. It is not a wrong result.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-gate-step11-skips-alone` — Gate step 11 cannot run in a lone mt worktree

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), M2.
- **Location / scenario / fix (review text, verbatim):** §5 step 11 cannot run as printed in a lone mt worktree. It prints "SKIP: ../mnemonic-engrave not found; set MT_SPEC_SOURCE", so the Q0 mismatch the step expects is never observed. The step should name `MT_SPEC_SOURCE=<engrave checkout>/design/SPEC_mt_v0_1.md`, or say a SKIP is acceptable. It fails closed (SKIP, not a false ok). ## Nits
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-doc-anchor-quote-wraps` — Section 1 anchor quote spans a spec line wrap

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N1.
- **Location / scenario / fix (review text, verbatim):** The §1 anchor quote spans a line wrap in the spec. A literal grep for it misses, though a careful reader finds it.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-row-10c-needed-filter-loose` — Row 10c needed filter is loose

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N2.
- **Location / scenario / fix (review text, verbatim):** Row 10c's `needed` filter has the same loose pattern as the one I1 fixes. It passes only because 10c has no duplicate input. Use `contains(" needed ")` there too.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-doc-n-undefined` — Section 2.1 does not say how n is computed

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N3.
- **Location / scenario / fix (review text, verbatim):** §2.1 does not say how to compute `n`, the count of elided lines. It can be inferred from the restore loop.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-doc-short-mt1-first-line` — Short mt1 first line is skipped as prefix source, undocumented

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N4.
- **Location / scenario / fix (review text, verbatim):** An `mt1` first line shorter than 11 bytes is now skipped as a prefix source. That is correct, but the doc should say so.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-doc-correction-warnings-placement` — Placement wording for correction_warnings

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N5.
- **Location / scenario / fix (review text, verbatim):** "Add one function next to `margin_report`" (§2.4) can be read as "between `margin_report`'s doc comment and its `fn`". Say "above `margin_report`'s doc comment".
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-doc-format-not-fmt-clean` — Section 2.4 format! fails cargo fmt as printed

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N6.
- **Location / scenario / fix (review text, verbatim):** §2.4's `format!` with an inline `if` fails `cargo fmt --check` as printed. `cargo fmt` fixes it; the doc could say to run `cargo fmt` before step 2.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).

### `fixc-full-len-is-bytes` — Existing "only {} characters" message counts bytes

- **Surfaced:** 2026-11-02 FIX review r1 (FIX-C), N7.
- **Location / scenario / fix (review text, verbatim):** The existing "only {} characters" message counts with `full.len()`, which is a byte count, not a character count. This predates the fix and is outside its scope.
- **Status:** deferred-minor.
- **Owning phase:** none (ownerless residue).
