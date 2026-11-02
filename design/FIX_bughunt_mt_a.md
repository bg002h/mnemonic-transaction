# FIX-A: mt encode §8.6 satisfaction shape (bughunt #1, #2, #3)

Track: small-fix. Repo: mnemonic-transaction (mt) only. Branch `fix/bughunt-a`,
base `3bfbc98`. Rulings R1-R3 of `design/agent-reports/grade-bughunt-mt.md` §2
bind, except that **R2 is replaced by the SigCtx context rule** (architect
ruling QA1, `coordinator/reports/architect-bughunt-fixdocs.md`; R2's `30 3d`
byte test is gone). Review class B (PSBT/transaction code). Build order across the bughunt
FIXes: **FIX-A builds first**, then FIX-C, then FIX-B and FIX-D. FIX-A touches
only `crates/mt-cli/src/validate.rs` and the spec, so it does not conflict with
B or C (reader error mapping).

**All three STOPs in §7 are ruled** (architect report, QA1, Q0, QA3). The spec
delta (§2) lands in the **mt copy** `design/SPEC_mt_v0_1.md`. Everything is
decided; the build may start.

## 1. Behaviour after the fix

All refusals are existing §10.10 texts, exit 1 (`ExitCode::FAILURE`). No new
text, flag, type, command or JSON warning.

| input | today | after |
|---|---|---|
| p2tr witness `[OP_TRUE, control block(65)]` (no signature) | encodes (rc 0) | `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature`, rc 1 |
| same plus an annex `[OP_TRUE, cb, 50 aa]` | encodes | same refusal, rc 1 |
| p2wpkh `[64-byte DER sig ending 0x02, pk]` | encodes | `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL`, rc 1 |
| v0 P2WSH `[32-byte preimage, 64-byte witnessScript]`, no signature | encodes (rc 0) | `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature`, rc 1 |
| v0 P2WSH `[32-byte preimage, 65-byte witnessScript]`, no signature | refused with the wrong text "sighash 0x..", rc 1 (the script's last byte) | `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature`, rc 1 |
| P2PKH `<sig ALL> <65-byte uncompressed pubkey 04.. ending 0x07>` | refused "sighash 0x07, not SIGHASH_ALL" | encodes, rc 0 |
| P2SH 2-of-3 `OP_0 <sig ALL> <sig ALL> <redeem>` | refused "carries no signature" | encodes, rc 0 |
| P2SH `OP_0 <sig NONE> <redeem>` | refused "carries no signature" | refused "sighash 0x02, not SIGHASH_ALL" |
| P2SH `OP_1 <sig ALL> <redeem>` | refused "carries no signature" | encodes, rc 0 |
| P2PKH `OP_PUSHDATA4 <sig ALL>` `<pk>` | refused "carries no signature" | encodes, rc 0 |
| p2wpkh `[71-byte sig ALL, 33-byte pk]` | encodes | encodes (unchanged; guard) |
| p2wpkh `[73-byte DER sig ending 0x02, pk]` | refused sighash 0x02 | unchanged (guard) |

## 2. Spec delta (`design/SPEC_mt_v0_1.md` §8.6, the mt copy; Q0)

Insert this paragraph immediately after the paragraph that ends "**This is
still a heuristic and the spec does not claim otherwise.**" (§8.6, the
"structural recognizer is AMBIGUOUS" note), as a new `>` paragraph at the
same indentation:

> **Script path and pushes, exactly (bughunt FIX-A).** A witness is a taproot
> script-path spend when, after removing an annex (BIP-341: the last element
> begins `0x50` and at least two elements are present), **two or more**
> elements remain and the last is shaped like a control block (`33 + 32m`
> bytes, first byte `0xc0` or `0xc1`). A signature-shaped element is
> judged by where it sits. In a taproot witness (after removing an annex,
> exactly one element, or a script path as above) only a 64-byte Schnorr
> element or a 65-byte one with an explicit sighash byte counts. In any other
> witness and in a `scriptSig` only a DER ECDSA element `30 <len>` of 9 to 73
> bytes including its trailing sighash byte counts, so a 64-byte `30 3d` ECDSA
> signature is ECDSA and a 64- or 65-byte witnessScript or public key is not a
> signature. A one-element segwit v0 witness reads as a taproot key path;
> without the spent output `mt` cannot tell them apart. The walk over a `scriptSig` treats every push opcode as a
> push: `OP_0` (an empty element), direct pushes `0x01`-`0x4b`,
> `OP_PUSHDATA1/2/4`, and `OP_1NEGATE` and `OP_1`-`OP_16` (small-integer
> elements); any other opcode ends the walk.

The mt copy is authoritative; the engrave copy is synced by engrave F-321
(Q0). No other spec text changes. README and `--help` do not describe the shapes
(checked: README's "What v0.1 deliberately does not do" says only that `mt`
recognises signatures "by shape"; `mt encode --help` names no shape), so there
is no lockstep edit beyond the spec.

## 3. Code changes (`crates/mt-cli/src/validate.rs` only)

These exact edits were applied as a scratch change, run green (§5), and
reverted.

### 3.1 `sig_shape` and `SigCtx` (R2 replaced by QA1)

Add above `sig_shape`:

    /// Where an element sits, which decides the only signature kinds it can be.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SigCtx {
        /// A taproot witness (key path or script path): BIP-340 Schnorr only.
        Taproot,
        /// Any other witness (segwit v0) or a scriptSig: DER ECDSA only.
        Ecdsa,
    }

Replace `pub fn sig_shape(e: &[u8]) -> Option<SigShape>` with
`pub fn sig_shape(e: &[u8], ctx: SigCtx) -> Option<SigShape>`:

- `SigCtx::Taproot`: length 64 gives `SchnorrDefault`; length 65 gives
  `SchnorrExplicit(e[64])`; any other length gives `None`.
- `SigCtx::Ecdsa`: with `n = e.len()`, if `(9..=73).contains(&n)`,
  `e[0] == 0x30` and `e[1] as usize == n - 3` give `Ecdsa(e[n - 1])`; else
  `None`. (This is the existing DER arm; its range stays `9..=73`, because
  narrowing it would let a 73-byte high-S DER signature skip the SIGHASH_ALL
  check, which row `02c` guards.) The old `64`, `65 if e[0] == 0x30` and `65`
  arms are removed.

`satisfaction_elements` returns `Vec<(Vec<u8>, SigCtx)>` (and its `v` is
declared `Vec<(Vec<u8>, SigCtx)>`). For a witness, after the annex strip and
before the script-path pops, compute
`let ctx = if w.len() == 1 || is_taproot_script_path(&w) { SigCtx::Taproot } else { SigCtx::Ecdsa };`
and finish with `v.extend(w.into_iter().map(|e| (e, ctx)));`. Every
scriptSig push is pushed as `(push, SigCtx::Ecdsa)`.

Both guards call `sig_shape(e, *c)`: in `has_signature_guard`
`elements.iter().any(|(e, c)| sig_shape(e, *c).is_some())`, and in
`sighash_all_guard` `.filter_map(|(e, c)| sig_shape(e, *c))`.

In the `SigShape` doc comment, replace "a 64-byte Schnorr element, or a
DER-encoded ECDSA one with a trailing sighash byte" with "a 64- or 65-byte
Schnorr element in a taproot witness, or a DER-encoded ECDSA element (9 to 73
bytes) elsewhere".

Why a context and not a byte test (QA1): a 64-byte `30 3d` test would read an
honest SIGHASH_DEFAULT Schnorr signature as ECDSA about 1 time in 2^16; a
full BIP-66 parse would still be a guess and leave the P2WSH and P2PKH cases
open. BIP-342 tapscript checks only Schnorr; a v0 witness and a scriptSig
carry only ECDSA (P2SH-wrapped v1 is not taproot); a valid v0 witnessScript
cannot start `0xc0`/`0xc1` (undefined opcodes), so a v0 witness never matches
the script-path shape.

**Residual, accepted and inside §8.6's declared heuristic:** a one-element v0
P2WSH witness (a no-argument witnessScript of exactly 64 or 65 bytes) reads as
a taproot key path. It cannot be closed without prevouts. Follow-up to file in
`design/FOLLOWUPS.md` (owner mt: prevout-aware classification for PSBT input;
owning phase: ownerless residue).

### 3.2 `is_taproot_script_path` (R1)

Change `w.len() >= 3` to `w.len() >= 2`. Keep the control-block shape test
unchanged (without it a p2wpkh `[sig, 33-byte pubkey]` would be read as a
script path; row `01d` guards this). Change its doc comment's first line from
"at least three elements" to "at least two elements (BIP-341)".

### 3.3 `script_sig_pushes` (R3)

In the `match op`, add these arms before `0x01..=0x4b`, and the `0x4e` arm
after the `0x4d` arm:

    0x00 => (1usize, 0usize),
    0x4e if b.len() >= 5 => (5, u32::from_le_bytes([b[1], b[2], b[3], b[4]]) as usize),
    0x4f => {
        out.push(vec![0x81]);
        b = &b[1..];
        continue;
    }
    0x51..=0x60 => {
        out.push(vec![op - 0x50]);
        b = &b[1..];
        continue;
    }

Order inside the match: `0x00`, `0x01..=0x4b`, `0x4c`, `0x4d`, `0x4e`, `0x4f`,
`0x51..=0x60`, `_ => return out`. Change the doc comment "Only pushes — a
`scriptSig` in a standard transaction contains nothing else." to "Every push
opcode counts: `OP_0`, direct pushes, `OP_PUSHDATA1/2/4`, `OP_1NEGATE` and
`OP_1`-`OP_16`; any other opcode ends the walk."

Run `cargo fmt -p mt-cli` after the edits.

## 4. Tests

### 4.1 Tests that change

None. `crates/mt-cli/tests/refusals.rs` §8.6 tests
(`refuses_a_signature_that_is_not_sighash_all`,
`refuses_a_satisfaction_carrying_no_signature`,
`a_control_block_is_not_counted_as_a_signature`) and `refusals.toml` stay
as they are and must stay green.

### 4.2 RED rows (`crates/mt-cli/tests/bughunt.rs`, committed with this doc)

Rows 01, 02, 03 are the triage rows (a5e5fd7). The rest are added by this FIX
(R1-R3 not otherwise covered, plus two guards). "Unmodified" = base `3bfbc98`
plus the test commit, run 2026-11-02. Rows 01e, 01f, 03e come from architect
ruling QA1/QA3.

| row | ruling | asserts | unmodified result |
|---|---|---|---|
| `bughunt_mt_01_keyless_taproot_scriptpath_not_signed` | R1 | encode fails | FAIL: encode accepted `[OP_TRUE, cb]` |
| `bughunt_mt_01b_keyless_scriptpath_refusal_text` | R1 | rc 1, first stderr line `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature` | FAIL: rc 0 |
| `bughunt_mt_01c_keyless_scriptpath_with_annex_refused` | R1 (annex) | same line, rc 1 | FAIL: rc 0 |
| `bughunt_mt_01d_p2wpkh_two_elements_is_not_scriptpath` | R1 guard | p2wpkh `[71-byte sig ALL, pk33]` encodes, rc 0 | PASS (guard) |
| `bughunt_mt_02_64_byte_ecdsa_sighash_none_refused` | R2 (now by SigCtx) | encode fails with "sighash 0x02" | FAIL: rc 0 |
| `bughunt_mt_02b_64_byte_ecdsa_sighash_none_refusal_text` | R2 (now by SigCtx) | rc 1, first line `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL` | FAIL: rc 0 |
| `bughunt_mt_02c_73_byte_ecdsa_sighash_none_still_refused` | R2 guard | same line for a 73-byte DER sig | PASS (guard) |
| `bughunt_mt_03_p2sh_multisig_op0_dummy_counts_signatures` | R3 | P2SH 2-of-3 encodes | FAIL: refused "carries no signature" |
| `bughunt_mt_03b_p2sh_op0_sighash_none_still_refused` | R3 extra row | rc 1, first line the sighash 0x02 refusal | FAIL: first line is "carries no signature" |
| `bughunt_mt_03c_small_int_before_sig_counts` | R3 (OP_1-16) | P2SH `OP_1 <sig ALL> <redeem>` encodes | FAIL: refused "carries no signature" |
| `bughunt_mt_03d_pushdata4_sig_counts` | R3 (PUSHDATA4) | P2PKH `4e <len> <sig ALL> <pk>` encodes | FAIL: refused "carries no signature" |
| `bughunt_mt_01e_p2wsh_keyless_64_byte_script_refused` | QA1 | v0 P2WSH `[32-byte preimage, 64-byte witnessScript]`: rc 1, first line `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature` | FAIL: rc 0 (encode accepted) |
| `bughunt_mt_01f_p2wsh_keyless_65_byte_script_refused` | QA1 | same line, rc 1, for a 65-byte witnessScript | FAIL: wrong first line `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x51, not SIGHASH_ALL` (0x51 is the test script's last byte) |
| `bughunt_mt_03e_uncompressed_p2pkh_encodes` | QA3 | P2PKH `<sig ALL> <65-byte 04.. pubkey ending 0x07>` encodes, rc 0 | FAIL: refused `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x07, not SIGHASH_ALL` |

Rows 01e, 01f and 03e are in `bughunt.rs` (the 01e/01f witnesses are built by
the shared helper `p2wsh_keyless(script_len)`: witness
`[32 bytes of 0x77, witnessScript]`, the script being `OP_SHA256 <32 bytes>
OP_EQUALVERIFY`, `OP_NOP` padding, then `OP_TRUE`). Row `01d` and `02c` are the
guards for the rule's Ecdsa-context side; `03e` is the guard for the scriptSig
side.

RED command (before §3), and GREEN command (after §3):

    PATH=$HOME/.cargo/bin:$PATH cargo nextest run --locked -p mt-cli -E 'test(/^bughunt_mt_0[123]/)' > red-a.log 2>&1
    grep -E "Summary|FAIL \[" red-a.log

Expected before §3: `14 tests run: 2 passed, 12 failed` (failed: 01, 01b, 01c,
01e, 01f, 02, 02b, 03, 03b, 03c, 03d, 03e; passed: the guards 01d, 02c).
Expected after §3: `14 tests run: 14 passed`. The filter matches all 14 rows
(run 2026-11-02 on base `3bfbc98` plus these rows: exactly the counts above).
One row by exact name: `-E 'test(/(^|::)bughunt_mt_01b_keyless_scriptpath_refusal_text$/)'`
(1 test run).

## 5. Scratch GREEN evidence (author)

Architect, with §3 including the SigCtx rule applied as a scratch change:
`cargo nextest run --locked -p mt-cli` ran 251 tests: 246 passed, 5 failed.
The 5 are bughunt rows owned by other FIXes (05, 07, 10, 11, 12); every FIX-A
row and every existing §8.6 test passed. The fold agent re-ran that scratch
`validate.rs`: all 14 FIX-A rows passed and the same 5 rows failed (nextest
stopped early at 34 of 251 on its default fail-fast). `validate.rs` was then
restored with `git checkout`.

## 6. Gate (builder, on the final head)

Capture each run once to a file and grep it. `$HOME/.cargo/bin` first in PATH;
`cargo --version` must print 1.85.0.

1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo nextest run --locked` (workspace). Expected: every test passes
   except bughunt rows owned by FIX-B/C/D that are still red on this branch;
   list each failure by name in the commit and confirm none is a FIX-A row or
   a pre-existing test.
5. `cargo test --locked --doc`
6. `./scripts/check-refusal-coverage.sh`
7. `./scripts/mutate-refusals.sh` (it owns the working tree; run it in a
   scratch copy of the worktree, not under `/tmp`)
8. `./scripts/journeys.sh`
9. `bash ci/docs-only.test.sh`, `bash ci/code-docs-check.test.sh`,
   `bash ci/code-docs-check.sh`
10. FIX-A rows by exact name:
    `cargo nextest run --locked -p mt-cli -E 'test(/^bughunt_mt_0[123]/)'`:
    **expected matched count 14, 14 passed**. A short or empty count is a FAIL.
11. Coordinator mutation gate:
    `coordinator/scripts/mutation-gate.sh <worktree> 3bfbc98`; the missed
    list goes to the review brief.

## 7. STOPs (architect)

### STOP 1: the 64-byte `30 3d` test misfires on honest Schnorr signatures

**RULED (QA1): Replace the byte test with the SigCtx context rule (§3.1); R2 is changed. The 65-byte `0x30` arm was not a misfire (it reports `e[64]` in both readings) and is removed with the other length arms. The ruling also found and absorbed a new fail-open (v0 P2WSH keyless 64/65-byte script; rows 01e, 01f).**

R2 rules that `30 3d` marks a 64-byte ECDSA element. A 64-byte BIP-340 Schnorr
signature is 64 uniform bytes, so about 1 in 65,536 honest taproot key-path
signatures begins `30 3d`; §3.1 then reads its last byte as a sighash and,
unless that byte happens to be `0x01`, refuses a valid SIGHASH_DEFAULT spend
with a false "signed with sighash 0x.., not SIGHASH_ALL". It fails closed but
names the wrong problem. Options:

- (a) the 2-byte test as written in §3.1 (R2 as ruled; misfire about 2^-16 per
  Schnorr signature);
- (b) a full DER structure test for 64-byte elements: `30 3d 02 rlen r 02 slen
  s sighash` with `4 + rlen + 2 + slen + 1 == 64` and BIP-66 rules (misfire
  about 2^-40);
- (c) classify by the spent output's type (p2tr vs not), which needs the
  prevout in the guard and changes `sig_shape`'s signature.

The existing `65 if e[0] == 0x30` arm already misfires on about 1 in 256
explicit-sighash Schnorr signatures the same way; the ruling should say
whether it is in scope. If (a), §3.1 stands as written.

### STOP 2: which repo the spec delta lands in

**RULED (Q0): (a), edit the mt copy; engrave sync is owed by engrave F-321 (OPEN, overdue). No PROVENANCE edit.**

The grade (§3) says FIX-A is "mt repo only". The mt `CLAUDE.md` says the spec
is a copy from mnemonic-engrave, which owns it: "Change the design there,
then re-sync the copies here". `design/PROVENANCE.md` pins it to engrave
`1182a0b8`, but `git log -- design/SPEC_mt_v0_1.md` shows later local edits
in this repo (a0d702e, 35266f3, 2effa8b, a0a70a8), so the copy has already
drifted. Decide: (a) edit the mt
copy (as those local edits did) and file the engrave sync as a follow-up, or
(b) land §2 in mnemonic-engrave first and re-sync.

### STOP 3 (route, not part of FIX-A): uncompressed P2PKH falsely refused

**RULED (QA3): CONFIRMED, Important; absorbed into FIX-A. The SigCtx rule is the discriminator (a scriptSig is never Schnorr); row `03e`.**

Found while writing this doc (probe run, not a committed row): a legacy P2PKH
input whose scriptSig is `<sig ALL> <65-byte uncompressed pubkey 04..>` is
refused with `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x..,
not SIGHASH_ALL` (the pubkey's last byte), rc 1, because a 65-byte element not
starting `0x30` is read as an explicit-sighash Schnorr signature. Present on
base; outside R1-R3. Fixing it needs a choice (exclude `04`-led 65-byte
elements in a scriptSig, or classify by output type as in STOP 1 (c)), so it
goes to the architect and, if ruled in, to its own FIX or to FIX-A by an
amended ruling.

## 8. Commit message

The build commit message carries this line verbatim:

    Spec copy: mt is authoritative; engrave sync owed by F-321.

The coordinator appends to engrave F-321: `bughunt FIX-A..C add mt-spec text at
§8.6, §1.1, §10.10; carry them in the sync.`
