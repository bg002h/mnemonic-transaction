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

---

## Addendum: the 2026-11-25 bug hunt compared with the 2026-11-19 bug hunt

_Written 2026-11-26 for the coordinator that holds the 2026-11-19 fix queue. Read-only comparison: nothing here was merged or pushed to a default branch._

_This is the per-repo copy. The full addendum, with every repo's 11-19 table, is appended to `BugHunt-2026-11-25-fable.md` in mnemonic-refugium (report PR #247, branch `claude/project-thread-ly86x4`)._

### Why this exists, and how to use it

Two hunts ran a week apart:

- **2026-11-19** (OpenAI Codex, gpt-6). It covered 7 repos: descriptor-mnemonic (md), mnemonic-engrave (me), mnemonic-key (mk), mnemonic-secret (ms), mnemonic-toolkit (tk), mnemonic-transaction (mt) and seedhammer (sh). The reports are `design/agent-reports/bug-hunt-2026-11-19.md` on each default branch. The keys are `bh-<repo>-<id>`.
- **2026-11-25** (Fable). It covered all 11 repos. Each repo has a `BugHunt-2026-11-25-fable.md` report in an open, add-only PR on branch `claude/project-thread-ly86x4`. The fixes are in a second set of open PRs on branch `claude/project-thread-utm1aa`, one per repo.

Before you run any queued 11-19 item:

1. Look the key up in the per-repo tables below. Each table says where the fix is (the merged PR on the default branch), whether the 11-25 hunt found that bug again or a related one, and what is still left to do.
2. If a row's last column says "nothing", the item is done. Don't redo it.
3. Every 11-25 fix branch starts from the current default branch (checked 2026-11-26: 0 commits behind in all 10 repos). So every 11-19 fix merged so far is already underneath the 11-25 fix PRs, and the 11-25 PRs do not undo any of them.

#### The 11-25 PRs

| Repo | Report PR (11-25 findings) | Fix PR (11-25 fixes) |
|---|---|---|
| descriptor-mnemonic | #75 | #76 |
| mnemonic-engrave | #67 | #68 |
| mnemonic-key | #42 | #43 |
| mnemonic-secret | #47 | #48 |
| mnemonic-toolkit | #226 | #227 |
| mnemonic-transaction | #31 | #32 |
| seedhammer | #28 | #29 |
| mnemonic-refugium | #247 | #248 |
| mnemonic-gui | #57 | #58 |
| refugium-wallet | #228 | #229 |
| rust-miniscript | #1 | none (it has only Info findings) |

All of these PRs were open and unmerged when this addendum was written.

#### How the 11-19 fix status was worked out (and its limits)

- The starting point is the closure record `coordinator/agent-reports/recon-bughunt-1119-closure.md` in mnemonic-refugium (2026-11-24). It maps 125 Critical/Important keys to 58 FIX rows and found 27 of them open and 6 unrouted.
- After that record was written, more PRs merged on 11-24 and 11-25. I matched them to keys by the key names their commit titles cite: md#70, #71, #74; mk#37, #38, #39; ms#41, #42; tk#211, #212, #214, #220; mt#26, #27, #29; me#64; sh#25. With those, **every 11-19 Critical/Important key that had a FIX row is now merged.** What is still open are keys that were graded Minor and filed as FOLLOWUPS lines.
- "Merged" means the PR that names the key is on the default branch. I did not re-run each key's reproduction against the merged code.
- The closure record lists ms "I5" as the stdin-alias bug, but that bug is ms M5 in the source report. Source I5 is "verify calls a Japanese Mnem card v0.1 entr". Both are open FOLLOWUPS lines (see the ms table).

### Summary of the overlap

The 11-25 hunt has 50 distinct findings. Compared with the 11-19 hunt:

- **6 are the same bug as an 11-19 finding.** In every case the 11-19 fix merged, but only in one place: one repo, or Rust and not the Go port. The 11-25 hunt found the same bug somewhere else.
- **5 are related, and I'm not sure whether they count as the same bug** (marked UNCERTAIN).
- **About 39 are new.** That includes all 16 findings from the four repos the 11-19 hunt did not cover: mnemonic-gui, mnemonic-refugium, refugium-wallet and rust-miniscript.
- **None of the 11-19 bugs that are still open was found again on 11-25.**

| 11-25 finding | 11-19 key | Match | 11-19 fix (merged) | Where the bug was still present on 11-25 | 11-25 fix PR (open) |
|---|---|---|---|---|---|
| X7 (md-codec pin before 0.48.6 can mint a card the decoder refuses) | bh-md-C2 | SAME | md#59, released in md-codec 0.48.6/0.48.7 | consumers' md-codec pins (engrave, refugium) | engrave#68 `fc5f7c5`, refugium#248 `f8a0d9b`: both pin descriptor-mnemonic `c8cde23` |
| E5 (engrave's mt-codec pin) | bh-mt-I18 | SAME | mt#21 | engrave's mt-codec pin `eec7910b` | engrave#68 `67c3f1c`: pins mt `9b094e0b` |
| X4 (WIF slot writes a depth-0, zero-chain-code xpub onto cards) | bh-mk-C1 | SAME | mk#30 (mk-cli; not released yet) | toolkit mints these cards; md and the Go port derive from them; GUI and installer pin mk-cli 0.17.0 | tk#227 `59057e10` (refuses a WIF slot in template mode), md#76 `59c158e` (md refuses it). Still to do: release mk-cli and move the GUI/installer pins (gui#58 records that no release exists yet) |
| SH#6 (Go: change list equals receive list) | bh-md-I6 | SAME, Go port | md#64 (Rust) | seedhammer `address/`, `gui/` | seedhammer#29 `c2326a3` |
| SH#4 (Go `mk.Encode` lacks the origin refusals) | bh-mk-I9 (same function as bh-sh-I10) | SAME, Go port | mk#29 (Rust, mk-codec 0.6.0); sh#13 fixed only the network check | seedhammer `mk/encode.go` | seedhammer#29 `19b6075` |
| md#3 (placeholder with no wildcard minted as `/*`) | bh-md-C1 | SAME root (same lexer line, `parse/template.rs:345`) | md#60 fixed only the `md decompose` route | md template lexer (encode/address/descriptor) | md#76 `59c158e` and `6604c88` (bare `@N` refused at mint) |
| toolkit#3 (Word Card parity budget below what is stated) | bh-tk-I6 | UNCERTAIN: same weakness (one wrong word loses the card), different positions (header and checkpoint words today; the stop-sign word on 11-19) | tk#212 (stop-sign recovery) | header and checkpoint words | tk#227 `e6fd50c8`, `573dc14f`: the help and manual now state the budget the decoder really has. The decoder itself is unchanged |
| md#4 (`c:` over a non-key child mints a card md cannot restore) | bh-md-I5 | UNCERTAIN: same class (the encoder accepts what restore refuses); md#62 added the mint-time self-decode, but that check does not type-check | md#62 | md-codec mint check | md#76 `f8b4a67`, `ca48a05`, `5ad5a9b`, `fcefa83` (type rule at mint; address/descriptor refuse invalid miniscript unless `--allow-invalid-miniscript`) |
| SH#3 (Go `md.Decode` accepts four shapes Rust refuses) | bh-md-C2 | UNCERTAIN: one of the four shapes (an override keyed on baseline `@0`) is the card C2 used to mint | md#59 (Rust encoder) | Go decoder | seedhammer#29 `216a8af` |
| mk#2 (`mk verify --from-md1` passes a non-cosigner) | bh-mk-I2, I15 | RELATED: encode got the membership check, verify did not | mk#31 | `mk verify` | mk#43 `015d106` |
| toolkit#6 (`--account` >= 2^31 panics in six commands) | toolkit FOLLOWUP `xpub-search-account-window-2p31-panic` (listed as "previously filed" in the 11-19 tk report) | RELATED: same panic class; the earlier fix covered xpub-search only | tk#214 | bundle, verify-bundle, export-wallet, addresses, restore, convert | tk#227 `2d0057e8` |

Not an 11-19 match, but worth knowing:

- X6 (non-ASCII characters read as codex32 symbols by older ms/mk codec pins) repeats a bug from the **October 31** hunt that was fixed in the codecs. mk#43 `34c69b8` covers mk-cli's own md1 input. The consumer pins are tracked by the 11-25 X6/X8 entries.
- toolkit#7 (legacy `sh(sortedmulti)` defaults to BIP-48 `1'`) sits next to the 11-19 "previously filed" toolkit line `xpub-search-sh-wsh-default-origin-notice`, but it is a different wrapper and a different bug. It is fixed by tk#227 `6c99456c` plus the BIP-defaults follow-ups.
- engrave E3 (`--as md1` accepts a contradictory origin) is the md1 route of the ruling in `FIX_sec_canonical_keeps_xpub_depth` (me#54, 11-23). That ruling was not an 11-19 key. engrave#68 `094979e` fixes it.

### Work that the 11-25 PRs create for later

These items follow from the 11-25 fixes. Check the coordinator's queue against them so nothing is done twice:

- **md-codec 0.49.0 re-pins.** md#76 changes minting (explicit origins on keyed cards, the bare `@N` refusal, the type rule, thresh "any k of n"). After it merges, mnemonic-refugium, mnemonic-engrave (their md-codec pins must stay equal), mnemonic-toolkit, refugium-wallet and the seedhammer port each need a re-pin or a check. engrave#68 and refugium#248 pin `c8cde23` (0.48.7), not 0.49.0.
- **mk-cli release.** The bh-mk-C1 fix (mk#30) is still unreleased. gui#58 and the toolkit installer still pin `mk-cli-v0.17.0` (11-25 X4, G5).
- **seedhammer provenance pins.** seedhammer#29 `dde21fe` re-pins md, mk and mt to what the ports converge to (SH#5). Re-check them after md#76 merges.

### mnemonic-transaction (mt): 11-19 keys

Source: `design/agent-reports/bug-hunt-2026-11-19.md` on `main`. 11-25 fix PR = mt#32. No 11-25 mt finding (T1, T2) matches an 11-19 key; the one overlap is in engrave (E5 = I18).

| Key | 11-19 finding | Fixed by (merged) | 11-25 overlap | Left to do |
|---|---|---|---|---|
| C1 | RPC error becomes a "scrap" verdict | mt#17 | none | nothing |
| C2 | Failed artifact/report writes return success | mt#29 | none | nothing |
| C3 | Failed replacement destroys the backup | mt#29 | none | nothing |
| I1 | Trailing raw tx bytes dropped | mt#20 | none | nothing |
| I2 | PSBT intake accepts a second record | mt#20 | none | nothing |
| I3 | Inspect reports the template txid | mt#23 | none | nothing |
| I4 | Encoding to the input path overwrites the source | mt#29 | none | nothing |
| I5 | Inspect rejects a binary PSBT before sniffing | mt#23 | none | nothing |
| I6 | Inspect misroutes grouped plate input | mt#23 | none | nothing |
| I7 | Repaired prefix copied onto undamaged plates | mt#19 | none | nothing |
| I8 | Recovery report gives an absent prefix | mt#19 | none | nothing |
| I9 | Grouping separator makes an unreadable backup | mt#26 | none | nothing |
| I10 | Quiet recovery hides a correction | mt#22 | none | nothing |
| I11 | Decode/inspect accept a comparison they skip | mt#26 | none | nothing |
| I12 | stdin and a file silently selects the file | mt#26 | none | nothing |
| I13 | Node JSON formatting flips pending to confirmed | mt#17 | none | nothing |
| I14 | Testnet4 output shown as mainnet | mt#17 | none | nothing |
| I15 | Scientific notation scales a value by 1e8 | mt#17 | none | nothing |
| I16 | Encode mints txs Core can reject | mt#18 | none | nothing |
| I17 | A value makes a nonexistent outpoint engravable | mt#18 | none | nothing |
| I18 | Reordered chunks give different bytes | mt#21 | **E5 SAME** (engrave's pin) | engrave#68 merge |
| I19 | Nonzero padding passes verification | mt#21 | none | nothing |
| I20 | A PSBT global key breaks the base64 export | mt#20 | none | nothing |
| I21 | Cut counts include elided prefix characters | mt#19 | none | nothing |
| I22 | Printed finalization remedy is refused by mt | mt#27 | none | nothing |
| I23 | Verify prints OK before its comparison | mt#22 | none | nothing |
| I24 | A non-UTF-8 path panics the CLI | mt#26 | none | nothing |
| I25 | Live smoke test fails under an ordinary umask | mt#27 | none | nothing |
| M1-M8, N1 | Minors and Nit | FOLLOWUPS lines filed 2026-11-24 (`f3f7eea`) | none re-found | as FOLLOWUPS say |

