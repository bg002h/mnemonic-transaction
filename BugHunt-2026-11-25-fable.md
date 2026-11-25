# Bug hunt 2026-11-25 (Fable): mnemonic-transaction

Report-only review of `mnemonic-transaction` at `9b094e0`, part of a constellation-wide hunt across all 11 bg002h repositories. Nothing in this repository was changed by the hunt; this file is the only addition.

**How it was done.** Eight Fable reviewers each audited an area (one of them the boundaries between repositories), reproducing what they could with locally built binaries or scratch tests that were not committed. Three further Fable verifiers then tried to disprove every High and Medium finding (and several Low ones) and re-graded them. Grades below are the verifiers' final grades where they ran.

**Grades.** Critical: can lose funds or leak a seed in ordinary use. High: wrong wallet, addresses or keys, or secret exposure, plausible in normal use, or recovery broken. Medium: needs an unusual but realistic input, or fails in a way a careful user might not notice. Low: fails closed, hardening, or a small exposure. Info: context, not a bug.

**Categories.** security (secret or key exposure, injection, integrity of what is trusted), funds (wrong address, amount, descriptor or recovery that could lose or strand coins), other.

**Status.** "Confirmed (reproduced)" means a command or test showed it. "Confirmed by reading" means the code path was traced but not executed. "Reviewer-confirmed, not re-verified" means the first reviewer reproduced or read it and no verifier re-checked it.

**Totals:** 1 Medium, 1 Low. No Critical findings.

## Summary

| ID | Grade | Category | Status | Finding |
|---|---|---|---|---|
| T1 | Medium | funds | Confirmed (reproduced) | A BIP-68 relative-locked transaction is reported (and engraved) as `NO TIMELOCK` |
| T2 | Low | security | Confirmed (reproduced) | The ms1 refusal only inspects the first 64 bytes of the input |

IDs: `md#`, `mk#`, `ms#`, `toolkit#`, `G`, `refugium#`, `wallet#`, `E`, `T`, `SH#`, `M` are per-repository findings; `X` findings are at a boundary between repositories and appear in every report they touch.

## Findings

## T1: A BIP-68 relative-locked transaction is reported (and engraved) as `NO TIMELOCK`

**Grade:** Medium · **Category:** funds · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `transaction`)

### T1. A BIP-68 relative-locked transaction is reported and engraved as `NO TIMELOCK` (cross-repo: spec owned by mnemonic-engrave, E2)

- Severity: **Medium**
- Status: **CONFIRMED**
- Evidence:
  - `crates/mt-cli/src/locktime.rs:89-101` — `read()` returns `Lock::None` whenever `nLockTime == 0`, and uses `nSequence` only to decide whether `nLockTime` is enforced.
  - `crates/mt-cli/src/locktime.rs:155-169` and `crates/mt-cli/src/report.rs:481-486` — the comments justify it with "a BIP-68 relative timelock lives in `OP_CSV` inside the witness script".
  - `design/SPEC_mt_v0_1.md:698` and `:2996-3013` (copy of the normative text) say the same and conclude the legend `NO TIMELOCK` is "precisely true about the fields `mt` read".
- Why it is wrong: BIP-68 is enforced on the transaction's fields, not in script. For `nVersion >= 2`, an input with `nSequence` bit 31 clear cannot be mined until the low 16 bits (blocks, or ×512 s when bit 22 is set) have elapsed since that input confirmed. `OP_CSV` only compels the spender to set the field. `mt` reads every input's `nSequence` already, so by the spec's own test ("a field read rather than a script read, so it stays in scope") this is in scope.
- User impact: the use case this tool is built for — a pre-signed recovery or inheritance spend kept on steel — is very often an `older(N)` path, which has `nLockTime = 0` and `nSequence = N`. `mt encode` and `mt inspect` print `LOCKTIME  NO TIMELOCK`, and the suggested legend (cut next to the transaction, permanent) says `NO TIMELOCK`. A recoverer who trusts the legend broadcasts early, gets `non-BIP68-final`, and has no indication from the plate of when it becomes valid. No funds move wrongly, but the steel states something false about the one property the holder most needs.
- Reproduction (`even.hex` is the `even` corpus vector; only `nSequence` and `nLockTime` are changed, which `mt inspect` does not sign-check):

  ```
  $ python3 - <<'E'
  b=bytearray.fromhex(open('even.hex').read().strip())
  off=4+2+1+36+1                      # version, marker+flag, vin count, outpoint, empty scriptSig
  b[off:off+4]=(0x00008000).to_bytes(4,'little')   # BIP-68: 32,768 blocks
  b[-4:]=b'\0\0\0\0'                               # nLockTime = 0
  open('csv.hex','w').write(b.hex()+'\n')
  E
  $ mt inspect --in csv.hex --bitcoin-cli /nonexistent 2>&1 | grep -n 'LOCKTIME\|TIMELOCK'
  7:LOCKTIME  NO TIMELOCK                      current height unknown (no node)
  18:             NO TIMELOCK                      current height unknown (no node)
  $ mt encode --in csv.hex --bitcoin-cli /nonexistent 2>&1 >/dev/null | grep -n 'TIMELOCK'
  69:LOCKTIME  NO TIMELOCK                      current height unknown (no node)
  85:    NO TIMELOCK                            # the legend line
  ```
- Fix direction: fix the spec in mnemonic-engrave first, re-sync, then add a per-input relative-lock reading (`version >= 2 && sequence & (1<<31) == 0 && sequence & 0xFFFF != 0`) to `locktime::read`, the report row and the legend. With a node, `gettxout`'s `confirmations` on the input gives the remaining wait.

**Verifier re-check** (source: `verify-3`)

### E2 / T1 — a BIP-68 relative-locked transaction is reported and engraved as `NO TIMELOCK`

- Verdict: **CONFIRMED**. Grade: **Medium**. Category: **funds**. Cross-repo: mnemonic-engrave (spec owner),
  mnemonic-transaction (code).
- Protocol fact: BIP-68 is a rule on the transaction's own fields. rust-bitcoin (vendored,
  `mnemonic-toolkit/vendor/bitcoin/src/blockdata/transaction.rs`): the `Sequence` doc lists "Indicating and encoding
  [BIP-68] relative lock-times" (:312); `is_relative_lock_time` is `self.0 & LOCK_TIME_DISABLE_FLAG_MASK == 0`
  (:393-395); bit 22 selects 512-second units (:399-407); "BIP-68 only uses the low 16 bits" (:499);
  `Version::TWO` is "post-BIP-68" (:1158). `OP_CSV` (BIP-112) only forces the spender to set that field. So the spec
  sentence "A BIP-68 relative timelock lives in the witness script as `OP_CSV` … Reading it means evaluating the
  sending wallet's script" (`mnemonic-engrave/design/SPEC_mt_v0_1.md:2848-2866`, also :691) is wrong, and its
  conclusion that `NO TIMELOCK` is "precisely true about the fields `mt` read" is false: `mt` reads `nSequence`.
- Code: `mnemonic-transaction/crates/mt-cli/src/locktime.rs:89-101` returns `Lock::None` whenever `nLockTime == 0`;
  `legend()` (:155-169) prints `NO TIMELOCK`.
- Reproduced: `work-memt/csv.hex` is version 2, one input with `nSequence = 0x00008000` (32,768 blocks),
  `nLockTime = 0`. `mt inspect --in csv.hex --bitcoin-cli /nonexistent` prints
  `LOCKTIME  NO TIMELOCK                      current height unknown (no node)`, exit 0.
- Grade: the steel legend states something false about when the money can move; the typical pre-signed recovery
  spend (`older(N)`) is exactly this shape. No coin moves wrongly (an early broadcast is refused `non-BIP68-final`),
  so not High.

## T2: The ms1 refusal only inspects the first 64 bytes of the input

**Grade:** Low · **Category:** security · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `transaction`)

### T2. §8.9's ms1 refusal only looks at the first 64 bytes of the input

- Severity: **Low**
- Status: **CONFIRMED** (no echo of the secret was found on any path)
- Evidence: `crates/mt-cli/src/validate.rs:1741-1751` — `secret_guard` builds `head` from `raw.iter().take(64)` and tests `starts_with("ms1")` / `contains("\nms1")` (the third clause repeats the first).
- User impact: an ms1 string that is not within the first 64 bytes is not refused. `mt verify`/`decode` treat it as one more unreadable string and still exit 0 with `OK`; `mt encode`/`inspect` fall to the §8.2e refusal, which prints the first 8 bytes of the input (not the secret, since it is past byte 64). So the spec's "mt will not read it" does not hold, although nothing leaks.
- Reproduction:

  ```
  $ (cat even.mt1; echo ms10entrsqqg5y2z9pzs3gg5y2z9pzs3gg5y2z9pzs3gg5y2z9pzs3gg5y2z9q5f042qmrw90mw) | mt verify
  mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.
  UNREADABLE STRING. string 7 of the input could not be read:
    character '1' at data-part offset 8 is not in the bech32 alphabet
  …exit 0
  $ (printf '# backup of my seed, written 2026-11-25, keep safe and never share!!\n'; echo ms10entrs…) | mt decode
  mt decode: REFUSED — §1.1e, this input is not an mt1 set (2 line(s), none of them mt1)     # not the §8.9 refusal
  ```
- Fix direction: scan every line (after trimming and lowercasing) for an `ms1` start, not the first 64 bytes.

**Verifier re-check** (source: `verify-2`)

### T2: mt's ms1 refusal inspects only the first 64 bytes

**Verdict: CONFIRMED. Grade: Low. Category: security (hardening).**

`secret_guard` (`mt-cli/src/validate.rs:1741-1750`) takes `raw.iter().take(64)`; its third clause repeats the
first. An ms1 first in the input is refused (§8.9, rc=1). After a 6-chunk set it is not:

```
$ (cat even.mt1; echo ms10entrs…) | mt verify
mt verify: OK — 6 chunks, set 0x2dcf2, transaction re-derives.
UNREADABLE STRING. string 7 of the input could not be read: character '1' at data-part offset 10 …   rc=0
```

I looked for an echo on every path I could reach and found none: `verify` prints an offset only, `decode`
after a 68-byte comment line gives the §1.1e refusal with no input bytes, `encode` gives §8.2e with the first
8 bytes of the comment (`23 20 62 61 63 6b 75 70`), not of the ms1. So the spec's "mt will not read it" is not
met, and no secret is exposed.

---

_Generated by [Claude Code](https://claude.ai/code) on 2026-11-25. Findings were not fixed; each fix needs its own review under this repository's gates._
