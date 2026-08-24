# Post-implementation mechanical claim check — mnemonic-transaction

Independent reviewer, mechanical claim check only (no design opinions). Checked
every commit message (`06445b5` P0 through `1b2859c` P6, plus the `ae918a0`
provenance commit), doc comments across `crates/*/src/**`, `refusals.toml`'s
header, `scripts/*.sh` headers, `design/PROVENANCE.md`, and cross-references
into `design/SPEC_mt_v0_1.md`.

## Verdict

`1C / 1I / 1M`, **41 claims checked, 2 false (1C, 1I), 1 imprecise (M), 38 true**

## Findings

### Critical 1 — `bch.rs` / `bch_decode.rs`: "Forked from `md-codec`" — the crate's own `mod.rs` says the opposite

**Where:**
`crates/mt-codec/src/string_layer/bch.rs:4-16`,
`crates/mt-codec/src/string_layer/bch_decode.rs:1-10`

**The claim (verbatim):**
- `bch.rs:4-5`: *"Forked from `md-codec` v0.4.x (`crates/md-codec/src/encoding.rs`) at the start of the mt1 v0.1 implementation per `design/DECISIONS.md` D-13."*
- `bch.rs:14-16`: *"mt1's string-layer header lives at the 5-bit symbol layer (per closure Q-5 — 2 symbols for `SingleString`, 8 symbols for `Chunked`)..."*
- `bch_decode.rs:1`: *"Syndrome-based BCH decoder for the MK regular and long codes."*
- `bch_decode.rs:3-4`: *"Forked from `md-codec` v0.4.x (`crates/md-codec/src/encoding/bch_decode.rs`) at the start of the mt1 v0.1 implementation..."*
- `bch_decode.rs:8-10`: *"The fork copy is expected to be retired once the `mc-codex32` shared-crate extraction lands (closure Q-9 trigger: both formats v1.0 with cross-validated conformance vectors)."*

**The truth:** All five sub-claims are false, and each is falsifiable against
another file in this same repository (some in the same directory):

1. **Wrong parent.** `crates/mt-codec/src/string_layer/mod.rs:3` states, in the
   same directory: *"**Ported from `mk-codec/src/string_layer/`, not from
   `md-codec`.**"* This matches `design/PROVENANCE.md` ("mk-codec 0.5.0...
   not md-codec") and the P1 commit message
   (`87dc95e`: "PORTED FROM mk-codec/src/string_layer/, not md-codec"). `bch.rs`
   and `bch_decode.rs` claim the opposite lineage.
2. **Nonexistent path.** `crates/md-codec/src/encoding.rs` and
   `crates/md-codec/src/encoding/bch_decode.rs` do not exist.
   `find /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src -type f`
   shows md-codec's actual files are `src/bch.rs` and `src/bch_decode.rs`, no
   `encoding/` subdirectory ever existed.
3. **Format letter not updated.** `bch_decode.rs:1` still reads "MK", not "MT".
4. **Nonexistent format described.** `bch.rs` describes a `SingleString` (2
   symbols) vs `Chunked` (8 symbols) header split. `grep -rn "SingleString"`
   across the whole crate and the spec returns only this one line — `mt1` has
   no `SingleString` form (`header.rs:16-18`: *"no `chunked` flag. `mt1` is
   always chunked"*), and its one header is **11** symbols
   (`consts::HEADER_SYMBOLS == 11`, asserted at compile time and by
   `header_is_symbol_aligned_per_field`), not 8.
5. **Dead future.** `mnemonic-key/design/FOLLOWUPS.md` records
   `mc-codex32-extraction-retired-2026-05-03` — *"original shared-crate plan
   retired in favor of ms1 adopting `rust-codex32` directly"* — and
   `mod.rs:8` (same directory) says so too: *"the shared-crate plan was
   **retired 2026-05-03**, not deferred."* The "closure Q-9 trigger" this
   text is waiting for can never fire.

**How I checked:**
```
$ diff <(sed -n '1,12p' crates/mt-codec/src/string_layer/bch_decode.rs) \
       <(sed -n '1,12p' /scratch/code/shibboleth/mnemonic-key/crates/mk-codec/src/string_layer/bch_decode.rs)
```
confirms `bch_decode.rs`'s header is a verbatim copy of **mk-codec's own**
`bch_decode.rs` header (which correctly describes MK's real history — mk1
really was forked from md-codec) with only `mk`→`mt`/`MK1`→`MT1`/
`MK_*_CONST`→`MT_*_CONST` substituted, missing line 1's "MK" and the whole
provenance sentence. Also ran:
```
$ grep -rn "SingleString" design/SPEC_mt_v0_1.md crates/mt-codec/src/**/*.rs crates/mt-codec/src/*.rs
crates/mt-codec/src/string_layer/bch.rs:15
$ find /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src -type f   # no encoding.rs, no encoding/
$ grep -n "HEADER_SYMBOLS: usize" crates/mt-codec/src/consts.rs  # = (HEADER_BITS/5) = 11
$ grep -n "mc-codex32-extraction-retired-2026-05-03" /scratch/code/shibboleth/mnemonic-key/design/FOLLOWUPS.md
```

**Why this is Critical, not Important:** `consts.rs`'s own opening paragraph
states the stakes of this exact class of error — a wrong constant "yields
chunks that are self-consistent and unreadable by every other implementation
... surfacing at recovery where it looks exactly like steel damage." This
project's standing rule (`mod.rs:11-12`, `PROVENANCE.md`) is that *"a defect
found in any of the three BCH implementations triggers checking the other
two"* — a safety mechanism specifically for this failure class. A reader
following `bch.rs`'s own header (rather than the correct `mod.rs` one
directory-listing away) would check `md-codec` instead of/in addition to the
crate the code actually shares lineage with, undermining the one cross-check
this project relies on to catch a BCH bug before it reaches steel. This is a
false claim about "whether a check exists / targets the right thing."

**Minimal correction:** Replace `bch.rs:4-17` and `bch_decode.rs:1-10` with
text matching `mod.rs`'s accurate lineage statement ("ported from mk-codec's
`bch.rs`/`bch_decode.rs`, not forked directly from md-codec"), drop the
`SingleString`/closure-Q-5/8-symbol-header sentence (does not apply to mt1),
and drop or rewrite the "expected to be retired" sentence to reflect that the
shared-crate plan is already retired.

---

### Important 1 — `report.rs`: "Five states, not four" — the enum has six

**Where:** `crates/mt-cli/src/report.rs:75-76`, introduced in commit
`f05b5f174bceb8561c7d56e53c61d49627b8d5ad` (P4) and unchanged through P5/P6.

**The claim (verbatim):** *"Plate liveness. **Five states, not four** — the
first one is asked before any input is classified."*

**The truth:** `pub enum Status` immediately below has **six** variants:
`AlreadyConfirmed`, `Live`, `Dead`, `Pending`, `Unknown`, `Indeterminate`.

**How I checked:**
```
$ python3 -c "
import re
text = open('crates/mt-cli/src/report.rs').read()
m = re.search(r'pub enum Status \{(.*?)\n\}', text, re.S)
variants = re.findall(r'^\s{4}(\w+),', m.group(1), re.M)
print(variants, len(variants))"
['AlreadyConfirmed', 'Live', 'Dead', 'Pending', 'Unknown', 'Indeterminate'] 6
```
`git log -p --follow -- crates/mt-cli/src/report.rs` shows the doc comment and
all six variants (including `Indeterminate`) were added together, in the same
diff hunk of the P4 commit — this was never true, not a later staleness.

**Likely origin (context, not part of the claim being graded):** the spec's
own §1.1 table (`design/SPEC_mt_v0_1.md:548`) says *"PLATE LIVENESS is its own
row, and it has FIVE states, not two"* — `AlreadyConfirmed` + `LIVE` + `DEAD`
+ `PENDING` + `UNKNOWN`, folding the no-`-txindex`-ambiguous case into the
same `UNKNOWN` label. The P4 commit message describes splitting that
ambiguous case into its own `Indeterminate` state ("Indeterminate is its own
state rather than folded into DEAD") — a real 6th state the code correctly
implements (both `Unknown` and `Indeterminate` render distinct text, both
prefixed "UNKNOWN —") — but the doc comment's count was never updated for the
crate's own extra split.

**Minimal correction:** "Six states, not five" (or not four, per whichever
baseline is meant).

---

### Minor 1 — P3 commit: "fourteen 90-character strings" doesn't total to the "1,242-character blob" stated in the same sentence

**Where:** commit `423d3cceac06134fa28cab5ea00ee87aeaf76ae5` (P3), commit
message paragraph beginning "SPLIT FIRST, THEN STRIP."

**The claim (verbatim):** *"taken literally it turns fourteen 90-character
strings into ONE 1,242-CHARACTER BLOB."*

**The truth:** 14 × 90 = 1,260, not 1,242. The figure 1,242 is real — it is
the spec's own 535-byte/14-string example
(`design/SPEC_mt_v0_1.md:944`: `| 535 B | 14 | 1,242 | 1,099 | 143 |`) — but
not all 14 strings are 90 characters. `chunk::plan(535)` gives
`bytes_per_chunk = 39`, `last_chunk_bytes = 28`
(pinned by `matches_the_specs_measured_artifacts`, `(535, 14, 39, 28)`).
Converting to string length (`"mt1"` + 11-symbol header + `ceil(payload*8/5)`
payload symbols + 13-symbol checksum): a full 39-byte chunk is
`3 + 11 + 63 + 13 = 90` characters (13 of the 14 chunks), but the 28-byte last
chunk is `3 + 11 + 45 + 13 = 72` characters. `13×90 + 72 = 1,242` — matching
the spec table exactly, but "fourteen 90-character strings" is not an
accurate description of what makes that total: it's thirteen 90-character
strings and one 72-character string.

**How I checked:**
```
$ cargo nextest run --locked -E 'test(matches_the_specs_measured_artifacts)'
... (535, 14, 39, 28) pinned and passing
$ python3 -c "print(13*90+72)"   # 1242
$ python3 -c "print(14*90)"      # 1260
```

**Minimal correction:** "thirteen 90-character strings and a 72-character
last one" (or simply drop the per-string length claim and keep the totals,
which are correct).

## Claims I checked and found TRUE

**Numbers (test/gate counts, constants, sizes):**
- P6/HEAD: 117 tests, 117 passing — `cargo nextest run --locked` → `117 tests
  run: 117 passed, 0 skipped`.
- `fmt`, `clippy -D warnings`, `build` all exit 0 on HEAD.
- `check-refusal-coverage.sh`: 15 refusal tests over 12 ruled refusals, each
  test/check resolves — ran it, output matches exactly, and the `REQUIRED`
  list in the script itself has 12 entries.
- `mutate-refusals.sh`: all 15 refusal tests go red when their check is
  removed — ran it (60s wall), all 15 report "ok — red without the check".
- `journeys.sh`: A, B (both forms), C all pass — ran it, every assertion
  reports "ok".
- CI (`ci.yml`) runs fmt, clippy, build, nextest, both P5 gates, and journeys,
  unconditionally (no `hashFiles(...)` guard) — matches P5/P6's claim that the
  guard was removed.
- `MAX_FEE_RATE_SAT_VB = 25_000` matches `bitcoin` 0.32.102's
  `Psbt::DEFAULT_MAX_FEE_RATE = FeeRate::from_sat_per_vb_unchecked(25_000)`.
- Header bit widths (version 5 / chunk_set_id 20 / count 15 / index 15 = 55
  bits = 11 symbols), `INVARIANT_PREFIX_SYMBOLS == 8`,
  `MAX_DATA_SYMBOLS == 88` (11+64+13), `MAX_CHUNKS == 32,768` — all match
  `consts.rs`'s own passing tests and `SPEC_mt_v0_1.md:3354-3363`'s table.
- `MT_LONG_CONST = 0x68bf21dfe54a35f0481` (P1 commit's claimed derived value)
  matches `consts.rs`'s current constant.
- `sha256(crates/mt-codec/src/test_vectors/mt1_v1.json) =
  ab5b3729b62d49f00dab206e973e177eafdb711d873c3a7c7968d22304b66087` — matches
  P0's claimed hash, the pin in `tests/vectors.rs`, and is byte-identical to
  `mnemonic-engrave`'s `design/vectors/mt1_v1_vectors.json` at commit
  `ddc4e087248d90423e9c0f0c1e25108277b1e1d8`.
- `--elide-prefix` drops exactly 11 characters after the first string — test
  `elide_prefix_drops_exactly_eleven_characters_after_the_first` passes.
- `chunk::plan(535) == (count: 14, bytes_per_chunk: 39, last_chunk_bytes:
  28)` — pinned test passes (see Minor 1 above for the one imprecise
  paraphrase of this fact).
- `gen-mt1-vectors.py --self-test` (run live in `mnemonic-engrave`):
  "mk corpus: 40/40 verify (19 regular, 21 long)" — matches
  `PROVENANCE.md`'s "40/40 of mk-codec's committed corpus" claim exactly.
- `mk-codec` version pin is `0.5.0` —
  `mnemonic-key/crates/mk-codec/Cargo.toml:3`.
- `Cargo.toml`: `resolver = "3"`, `edition = "2024"`, `rust-version = "1.85"`,
  `license = "MIT OR Unlicense"`, `[workspace.lints.rust] missing_docs =
  "warn"`, `[workspace.lints.clippy] all = "warn"`, `[profile.test]` /
  `[profile.dev]` both `opt-level = 2` — all present as claimed.
- `rust-toolchain.toml`: `channel = "1.85.0"` — present as claimed.
- The 12-item `REQUIRED` list in `check-refusal-coverage.sh` matches the "12
  ruled refusals" figure cited in the P5 commit and P6 gate line.

**Code descriptions (behavior of this code and of `rust-bitcoin`):**
- `rust-bitcoin` 0.32.102's `verify_transaction_with_flags` iterates
  `tx.input.enumerate()` calling `verify_script_with_flags` per input and
  never sums/compares input vs. output totals —
  `consensus/validation.rs:82-105`.
- `PSBT_IN_FINAL_SCRIPTSIG` (0x07) / `PSBT_IN_FINAL_SCRIPTWITNESS` (0x08) map
  to rust-bitcoin's `final_script_sig` / `final_script_witness` fields —
  `psbt/map/input.rs`.
- `bitcoin-cli --help`'s `-stdin` text is verbatim *"recommended for sensitive
  information such as passphrases"* — matches `node.rs`'s doc comment,
  checked against the installed `bitcoin-cli` (Core v25.0.0).
- `bitcoin-cli help walletprocesspsbt`: `finalize` defaults to `true` —
  matches the P5 commit's claim about why the "unfinalized" fixture needed
  regenerating.
- `base64("psbt\xff") == "cHNidP8="` — matches `input.rs`'s
  `PSBT_BASE64_PREFIX` comment.
- `decode()` in `pipeline.rs` reads every string (not `?` on the first
  failure), keeps `Unreadable` entries, fails only when a slot ends up empty
  — matches the P6 commit's central claim; `DecodedSet`/`Duplicate`/
  `Unreadable`/`corrected_from`/`corrected_to` all exist as described.
- `mt encode`'s stdout carries only lowercase ungrouped `mt1…` lines (no other
  writes to stdout in `encode()`); `mt decode`'s stdout write happens only
  after every fallible step — confirmed by reading `main.rs` and by
  `journeys.sh`'s passing assertions (`lacks ... "WARNING"`, `lacks ...
  "FEE"`, every line matches `^mt1[0-9a-z]*$`).
- `md verify <STRINGS>…` really does take strings positionally — confirmed
  against `descriptor-mnemonic/crates/md-cli/src/main.rs`'s `Verify` clap
  variant (`strings: Vec<String>`, `num_args = 1..`).

**Cross-references:**
- §10.10 (cited in `blocks.rs` for the TTY-welcome-line cost quote) resolves
  into `SPEC_mt_v0_1.md`'s §10 item 10 ("The CLI surface — RULED"), and the
  quoted text *"a new user concluding the tool does not work and leaving,
  which no other check catches"* appears there verbatim (split across a line
  wrap).
- §8.2, §8.7, §8.7c, §8.8 are non-refusal numbered items in spec §8 (script
  validity removed / `mt qr`-deferred pointers) — confirmed by listing §8's
  numbered markers directly.
- §6a's value-mismatch refusal is a real subsection ("### 6a. When `bitcoind`
  is reachable...") inside §6, outside §8's numbering, as `refusals.toml` and
  both gate scripts claim.
- `mnemonic-transaction/design/SPEC_mt_v0_1.md` and
  `design/vectors/mt1_v1_vectors.md` are byte-identical both to the current
  `mnemonic-engrave` copies and to those files at commit
  `ddc4e087248d90423e9c0f0c1e25108277b1e1d8` — `diff` exit 0 in all
  directions, matching `PROVENANCE.md`'s table.
- `mnemonic-key/design/FOLLOWUPS.md` anchor
  `mc-codex32-extraction-retired-2026-05-03` exists and confirms the
  2026-05-03 retirement date cited in `PROVENANCE.md`/`mod.rs`.
- `gh api repos/bg002h/mnemonic-transaction/branches/main/protection` still
  returns 403 *"Upgrade to GitHub Pro or make this repository public"* today
  — repo confirmed private (`gh repo view` → `isPrivate: true`) — matches the
  `ae918a0` provenance commit's claim exactly, and it has not gone stale.

**Negatives / absence claims:**
- CI's `if: hashFiles(...)` guard on the test/coverage/mutation steps is
  genuinely gone from `.github/workflows/ci.yml` (all steps unconditional),
  matching the P5/P6 claim.
- `ChecksumFailed` in `error.rs` is never constructed anywhere in the crate
  (`grep -rn "ChecksumFailed" crates/` returns only its own definition) — not
  reported as a finding since no doc comment claims it is used, but noted for
  completeness; `clippy -D warnings` does not flag it because it is a `pub`
  item.

## Not checked / out of scope

- Citations to review-round findings (`R6 adversarial I-5`, `R8 C-5`,
  `R11 C3`, `R12`, etc.) in commit messages and the spec are UNVERIFIED — the
  underlying review transcripts are not committed to this repo and were not
  available to check against.
- BIP-93's `BCH(93,80,8)` / `BCH(108,93,8)` parameters were cross-checked for
  internal consistency (identical across `mt-codec`, `mk-codec`, and the
  P1/P2 commit messages) but not re-derived from the BIP text itself.
