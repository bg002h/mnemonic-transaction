# FIX-A: mt encode §8.6 satisfaction shape (bughunt #1, #2, #3)

Track: small-fix. Repo: mnemonic-transaction (mt) only. Branch `fix/bughunt-a`,
base `3bfbc98`. Rulings R1-R3 of `design/agent-reports/grade-bughunt-mt.md` §2
bind. Review class B (PSBT/transaction code). Build order across the bughunt
FIXes: **FIX-A builds first**, then FIX-C, then FIX-B and FIX-D. FIX-A touches
only `crates/mt-cli/src/validate.rs` and the spec, so it does not conflict with
B or C (reader error mapping).

**Two STOPs below (§7) must be answered by the architect before the build
starts.** STOP 1 can change one line of §3.1; STOP 2 decides which repo the
spec delta (§2) lands in. Everything else is decided.

## 1. Behaviour after the fix

All refusals are existing §10.10 texts, exit 1 (`ExitCode::FAILURE`). No new
text, flag, type, command or JSON warning.

| input | today | after |
|---|---|---|
| p2tr witness `[OP_TRUE, control block(65)]` (no signature) | encodes (rc 0) | `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature`, rc 1 |
| same plus an annex `[OP_TRUE, cb, 50 aa]` | encodes | same refusal, rc 1 |
| p2wpkh `[64-byte DER sig ending 0x02, pk]` | encodes | `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL`, rc 1 |
| P2SH 2-of-3 `OP_0 <sig ALL> <sig ALL> <redeem>` | refused "carries no signature" | encodes, rc 0 |
| P2SH `OP_0 <sig NONE> <redeem>` | refused "carries no signature" | refused "sighash 0x02, not SIGHASH_ALL" |
| P2SH `OP_1 <sig ALL> <redeem>` | refused "carries no signature" | encodes, rc 0 |
| P2PKH `OP_PUSHDATA4 <sig ALL>` `<pk>` | refused "carries no signature" | encodes, rc 0 |
| p2wpkh `[71-byte sig ALL, 33-byte pk]` | encodes | encodes (unchanged; guard) |
| p2wpkh `[73-byte DER sig ending 0x02, pk]` | refused sighash 0x02 | unchanged (guard) |

## 2. Spec delta (`design/SPEC_mt_v0_1.md` §8.6; repo per STOP 2)

Insert this paragraph immediately after the paragraph that ends "**This is
still a heuristic and the spec does not claim otherwise.**" (§8.6, the
"structural recognizer is AMBIGUOUS" note), as a new `>` paragraph at the
same indentation:

> **Script path and pushes, exactly (bughunt FIX-A).** A witness is a taproot
> script-path spend when, after removing an annex (BIP-341: the last element
> begins `0x50` and at least two elements are present), **two or more**
> elements remain and the last is shaped like a control block (`33 + 32m`
> bytes, first byte `0xc0` or `0xc1`). A signature-shaped element is a
> 64-byte Schnorr element, a 65-byte Schnorr element with an explicit sighash
> byte, or a DER ECDSA element `30 <len>` of 9 to 73 bytes including its
> trailing sighash byte, **including the 64-byte `30 3d` form**, which is
> classed as ECDSA. The walk over a `scriptSig` treats every push opcode as a
> push: `OP_0` (an empty element), direct pushes `0x01`-`0x4b`,
> `OP_PUSHDATA1/2/4`, and `OP_1NEGATE` and `OP_1`-`OP_16` (small-integer
> elements); any other opcode ends the walk.

No other spec text changes. README and `--help` do not describe the shapes
(checked: README's "What v0.1 deliberately does not do" says only that `mt`
recognises signatures "by shape"; `mt encode --help` names no shape), so there
is no lockstep edit beyond the spec.

## 3. Code changes (`crates/mt-cli/src/validate.rs` only)

These exact edits were applied as a scratch change, run green (§5), and
reverted.

### 3.1 `sig_shape` (R2)

In the `match e.len()` of `pub fn sig_shape`, insert as the **first** arm,
before `64 => Some(SigShape::SchnorrDefault),`:

    // 64-byte DER (30 3d ...) plus sighash: ECDSA, not Schnorr (R2).
    64 if e[0] == 0x30 && e[1] == 0x3d => Some(SigShape::Ecdsa(e[63])),

Leave the DER arm's range `9..=73` as it is (a superset of R2's `30 3d`..`30
45`; narrowing it would let a 73-byte high-S DER signature skip the SIGHASH_ALL
check, which row `02c` guards).

In the `SigShape` doc comment, replace "a 64-byte Schnorr element, or a
DER-encoded ECDSA one with a trailing sighash byte" with "a 64- or 65-byte
Schnorr element, or a DER-encoded ECDSA one (9 to 73 bytes, the 64-byte `30 3d`
form included) with a trailing sighash byte".

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
plus the test commit, run 2026-11-02.

| row | ruling | asserts | unmodified result |
|---|---|---|---|
| `bughunt_mt_01_keyless_taproot_scriptpath_not_signed` | R1 | encode fails | FAIL: encode accepted `[OP_TRUE, cb]` |
| `bughunt_mt_01b_keyless_scriptpath_refusal_text` | R1 | rc 1, first stderr line `mt encode: REFUSED — §8.6, input 0's satisfaction carries no signature` | FAIL: rc 0 |
| `bughunt_mt_01c_keyless_scriptpath_with_annex_refused` | R1 (annex) | same line, rc 1 | FAIL: rc 0 |
| `bughunt_mt_01d_p2wpkh_two_elements_is_not_scriptpath` | R1 guard | p2wpkh `[71-byte sig ALL, pk33]` encodes, rc 0 | PASS (guard) |
| `bughunt_mt_02_64_byte_ecdsa_sighash_none_refused` | R2 | encode fails with "sighash 0x02" | FAIL: rc 0 |
| `bughunt_mt_02b_64_byte_ecdsa_sighash_none_refusal_text` | R2 | rc 1, first line `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x02, not SIGHASH_ALL` | FAIL: rc 0 |
| `bughunt_mt_02c_73_byte_ecdsa_sighash_none_still_refused` | R2 guard | same line for a 73-byte DER sig | PASS (guard) |
| `bughunt_mt_03_p2sh_multisig_op0_dummy_counts_signatures` | R3 | P2SH 2-of-3 encodes | FAIL: refused "carries no signature" |
| `bughunt_mt_03b_p2sh_op0_sighash_none_still_refused` | R3 extra row | rc 1, first line the sighash 0x02 refusal | FAIL: first line is "carries no signature" |
| `bughunt_mt_03c_small_int_before_sig_counts` | R3 (OP_1-16) | P2SH `OP_1 <sig ALL> <redeem>` encodes | FAIL: refused "carries no signature" |
| `bughunt_mt_03d_pushdata4_sig_counts` | R3 (PUSHDATA4) | P2PKH `4e <len> <sig ALL> <pk>` encodes | FAIL: refused "carries no signature" |

RED command (before §3), and GREEN command (after §3):

    PATH=$HOME/.cargo/bin:$PATH cargo nextest run --locked -p mt-cli -E 'test(/^bughunt_mt_0[123]/)' > red-a.log 2>&1
    grep -E "Summary|FAIL \[" red-a.log

Expected before §3: `11 tests run: 2 passed, 9 failed` (failed: 01, 01b, 01c,
02, 02b, 03, 03b, 03c, 03d). Expected after §3: `11 tests run: 11 passed`.
One row by exact name: `-E 'test(/(^|::)bughunt_mt_01b_keyless_scriptpath_refusal_text$/)'`
(1 test run).

## 5. Scratch GREEN evidence (author)

With §3 applied, `cargo nextest run --locked -p mt-cli` ran 248 tests: 243
passed, 5 failed. The 5 are bughunt rows owned by other FIXes (05, 07, 10, 11,
12); every FIX-A row passed and no other test changed. `validate.rs` was then
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
    **expected matched count 11, 11 passed**. A short or empty count is a FAIL.
11. Coordinator mutation gate:
    `coordinator/scripts/mutation-gate.sh <worktree> 3bfbc98`; the missed
    list goes to the review brief.

## 7. STOPs (architect)

### STOP 1: the 64-byte `30 3d` test misfires on honest Schnorr signatures

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

The grade (§3) says FIX-A is "mt repo only". The mt `CLAUDE.md` says the spec
is a copy from mnemonic-engrave, which owns it: "Change the design there,
then re-sync the copies here". `design/PROVENANCE.md` pins it to engrave
`1182a0b8`, but `git log -- design/SPEC_mt_v0_1.md` shows later local edits
in this repo (a0d702e, 35266f3, 2effa8b, a0a70a8), so the copy has already
drifted. Decide: (a) edit the mt
copy (as those local edits did) and file the engrave sync as a follow-up, or
(b) land §2 in mnemonic-engrave first and re-sync.

### STOP 3 (route, not part of FIX-A): uncompressed P2PKH falsely refused

Found while writing this doc (probe run, not a committed row): a legacy P2PKH
input whose scriptSig is `<sig ALL> <65-byte uncompressed pubkey 04..>` is
refused with `mt encode: REFUSED — §8.6, input 0 is signed with sighash 0x..,
not SIGHASH_ALL` (the pubkey's last byte), rc 1, because a 65-byte element not
starting `0x30` is read as an explicit-sighash Schnorr signature. Present on
base; outside R1-R3. Fixing it needs a choice (exclude `04`-led 65-byte
elements in a scriptSig, or classify by output type as in STOP 1 (c)), so it
goes to the architect and, if ruled in, to its own FIX or to FIX-A by an
amended ruling.
