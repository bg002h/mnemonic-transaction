# R-round4-fold-check — mechanical fold check of `R-round3-near-miss.md`

Independent reviewer. Wrote none of this code, wrote none of `R-round3-near-miss.md`.

Scope: exactly the twelve findings in `design/agent-reports/R-round3-near-miss.md`
(0C / 8I / 4M), and whether the fold at `4f57bd5`/`40968cf` (range `e425c65..HEAD`)
fixed each one and/or broke anything. No fresh audit, no new lenses outside what
the brief asked for (per-guard near-miss construction).

Method: `cargo build`, then `./target/debug/mt` against constructed and
reconstructed inputs, offline via `--bitcoin-cli /nonexistent`. Fixtures reused
from `/tmp/mtr` (the round-3 reviewer's scratch dir) where present, and
regenerated fresh from the same recipe and diffed byte-identical before reuse.
Nothing written into the repo. `cargo nextest run --locked`: 198 tests, 198
passed, 0 skipped (reconfirmed, matches the fold's claim).

Facts taken as already settled, not re-derived: fmt/clippy/198 tests/journeys/
refusal gates/check-provenance/live smoke test all pass; `mt qr`, §8.7, §8.7c,
§8.8 deferred; no script engine; mainnet-params-on-regtest and the spec's
`~FALL 2034` self-disagreement are known and out of scope.

## Verdict

`9 FIXED / 1 PARTIAL / 2 NOT FIXED / 0 REGRESSED` (of the twelve), plus
**2 new Important, 1 new Minor** found by the near-miss guard testing.

Two of the twelve — findings 5 and 6, both Important — are **NOT FIXED**: their
exact reproductions from the round-3 report reproduce **byte-identical wrong
output** on the current tip. The fold's own commit message and doc comments
describe both as fixed ("per-suspect", "recognise the shape and say so"); the
code does not do what the comments claim.

## Finding-by-finding

| # | Finding | Status | Evidence |
|---|---|---|---|
| I-1 | malleability caveat on SegWit-only tx | **FIXED** | `mt decode < even.txt` (all-SegWit): no caveat text (grep exit 1). Positive case reconfirmed: built a real legacy-input tx (offline, replicating `tests/legacy.rs`'s `legacy_psbt` fixture via a throwaway `bitcoin=0.32.102` crate built `--offline`), ran it through `mt encode` then `mt decode` — caveat **does** print ("altered in flight and confirm under a DIFFERENT txid"). Both directions of the gate work. |
| I-2 | §8.2e fee-provenance wording | **FIXED** | `mt encode --input-value 0:4.0 --input-value 1:4.0 …`: warning now takes the UNKNOWN branch ("mt cannot compute the fee from it alone… THE FEE IS UNKNOWN"), report row correctly shows `FEE 0.00100000 BTC (CLAIMED — no input value verified)`. The false "mt fetched each input's value… real" text no longer appears when provenance is operator-asserted. |
| I-3 | "EVERY PLATE IS ACCOUNTED FOR" counts lines, not chunks | **FIXED** (see new-defect NI-2 for a related near miss) | Reran `dup_dmg.txt` (plate 2 typed twice, plate 3 skipped, plate 5 damaged) verbatim: now says "This set has 6 chunks and only 4 distinct ones are present… SO A PLATE IS MISSING AS WELL AS DAMAGED" **and** includes the stack hint "Chunk 2 arrived TWICE…" in the same message — both absent/wrong before. |
| I-4 | "mt cannot tell how many chunks" printed when it can tell | **FIXED** | Reran `short_dmg.txt` (5 of 6 typed, one damaged): now says "This set has 6 chunks and only 4 distinct ones are present, even counting the 1 unreadable string(s)… A PLATE IS MISSING AS WELL AS DAMAGED" instead of the old false "cannot tell" text. |
| I-5 | ambiguous branch is whole-set, not per-suspect | **NOT FIXED** | Rebuilt `un_two.txt` fresh from the report's exact script (diffed byte-identical to the round-3 fixture) and ran `mt verify`. Output is **byte-identical** to the report's quoted bad output: `string 8: 79 characters (expected 85) — 6 characters are MISSING` — still fabricated; string 8 is the legitimately short final chunk, unchanged in length. Traced why: `short_count` (`read_strings.rs:340`) is computed over **all strings**, not per suspect, so `is_ambiguous` (`:341`) is false for every suspect whenever ≥2 strings are short — exactly this scenario. The refactor extracted a closure and applied it inside the per-string loop (`:348`), but the gating predicate itself (`short_count == 1`) is textually the same aggregate condition as the pre-fold `ambiguous` variable it replaced. No test exercises the 2-suspect case (`grep -n "ambiguous" crates/mt-cli/tests/*.rs` finds one test, and it only covers the 1-suspect cases already marked CLEAN in the round-3 report). |
| I-6 | `mtl` + second defect misdiagnosed as elided+EXTRA | **NOT FIXED** | Rebuilt `sep_drop.txt` and `el_sep_drop.txt` fresh from the report's scripts (diffed byte-identical) and reran. Non-elided: **byte-identical** wrong output `string 3: 97 characters (expected 87) — 10 characters are EXTRA`. Elided: **byte-identical** wrong refusal `REFUSED — §3b, all 6 lines are elided; no prefix to restore`. Root cause: the new guard (`read_strings.rs:429-434`) only fires when the malformed `mtl…` candidate's length **exactly equals** `full_len` — true only when the second defect is a same-length substitution. The report's own reproduction uses a **dropped character** (length-changing), so the guard's length check never matches and the string falls through to the identical pre-fold misdiagnosis. Confirmed the guard DOES work on the narrower, unreported case: same separator misread + a pure substitution (no length change) now decodes correctly on both non-elided (`sep_sub.txt`) and elided (`el_sep_sub.txt`) variants. |
| I-7 | `b` autocorrect: wrong-symbol report + wasted budget | **FIXED** | `b8.txt` (plate has `8`, typed `b`): margin notice now reads "pos 7 you typed b, mt read it as 8 … This cost NONE of the 4-symbol repair budget" — correct symbol, correct cost accounting, in its own notice (not folded into the margin report). `four_plus_b.txt` (4 genuine errors + the `8`-as-`b`): now `OK` (was `REFUSED`) — no longer burns a repair on its own guess. |
| I-8 | `--json` inert on encode/decode/verify | **PARTIAL** | `verify --json` and `encode --json` now correctly REFUSE (`§10.10 --json has no meaning for 'verify'/'encode'`). `decode --json` is "wired" but the JSON block and the plain-prose notices (no-node warning, transliteration notices, set/margin reports) are written to the **same** stderr stream, one after another. In the tool's own core scenario — no node reachable, which is unconditional whenever `node.is_none()` — `mt decode --json 2>report.json` yields a stream that starts with valid JSON and then has prose appended after the closing `}`; feeding the whole stream to a parser fails (`json.JSONDecodeError: Extra data`). Confirmed by piping `mt decode --json --bitcoin-cli /nonexistent < even.txt`'s full stderr into `json.load` — fails; `inspect --json` (JSON on stdout, prose on stderr, pre-existing design) does not have this problem. The one automated test for this (`refusals.rs:1205 json_works_where_there_is_a_report_and_refuses_where_there_is_not`) masks it: it does `err.find('{')`/`err.rfind('}')` and parses only that slice, not the realistic "pipe stderr to a parser" case. See NI-3. |
| M-9 | dead `'I'` arm | **FIXED** | `candidates_for` (`read_strings.rs`) no longer has an `'I'` arm on the separator match; only `'l'|'i'`. Uppercase input (`tr a-z A-Z < even.txt`) still verifies OK (lowercased upstream, as before). |
| M-10 | redirected-output warning names a "file" for any pipe | **FIXED** | `mt encode … 2>&1 >/dev/null \| grep BEARER` now reads "the strings just left this terminal — and they are BEARER…" / "If it landed in a FILE, destroy it…" — no longer asserts a file exists. |
| M-11 | `mt1 SET` row shows hex id, not the steel's prefix | **FIXED** | `mt decode < even.txt`: row now reads `mt1 SET 6 strings, 1..6 all present, all begin mt1p9h8jqq9`, matching `encode`'s `PREFIX` row form. `render_json` emits `set_prefix` in place of `set_id`. |
| M-12 | `txid_paste_guard` only on `encode` | **FIXED** | Piping a bare txid to `mt decode` now REFUSES with `§10.10 this is a transaction ID…`, where before it fell through to the wrong §1.1e/§3b message. Guard is now called once from `read_input`, shared by all four verbs. |

## New defects introduced by this fold

### [Important] NI-1 — a duplicated *unreadable* line defeats the new distinct-chunk count, and reproduces the exact false claim I-3 was written to remove

**Where** `crates/mt-cli/src/main.rs:1204-1231` (`explain_failure`'s
`distinct`/`accounted` computation).

**What** The I-3 fix counts `distinct.len() + unreadable.len()` against `n` to
decide whether "every chunk COULD be here." `distinct` is deduplicated (a
`BTreeSet` over readable chunks' `header.index`), but `unreadable.len()` is a
raw **line count**, not deduplicated by content. Two unreadable lines with
**identical text** — the single most likely way to get two unreadable lines,
since it means the operator typed the same damaged plate twice — are counted
as two *potentially different* missing chunks, when they can be at most one.

**The near miss** The mechanical slip is the same one I-3 itself was written
to catch — typing one plate twice, skipping another — except the duplicated
plate is the *damaged* one instead of a clean one. `dup_unreadable.txt`: 6-chunk
set, chunks 1-4 clean, chunk 5 damaged past t=4 with 5 substitutions, chunk 5
**typed twice**, chunk 6 **never typed at all**.

**How I reproduced it**
```
$ mt verify --bitcoin-cli /nonexistent < dup_unreadable.txt
mt verify: REFUSED — §1.1, string 5, 6 could not be read: more than 4 characters
differ from what was engraved

  This set has 6 chunks. 4 read cleanly and 2 did not, so every chunk
  COULD be here — nothing is necessarily lost, and one plate is
  damaged past what BCH can repair.
```
4 distinct readable + 2 unreadable = 6 ≥ n, so mt asserts nothing is
*necessarily* lost. But both unreadable lines are byte-identical, so at most 5
distinct chunks are actually represented — chunk 6 is genuinely absent, exactly
as in I-3's own scenario. No stack-hint fires either: the duplicate-detection
`seen` map (`main.rs:1172-1183`) is built only from strings that
`decode_chunk` succeeds on, so a duplicated *unreadable* line produces no hint
at all — the operator gets a false "nothing is necessarily lost" with no
signal that anything is off.

**Suggested direction (non-authoritative)** Deduplicate `unreadable` by string
content (or by whatever positional/BCH-partial signal is available) before
adding it to `distinct.len()`, the same way `distinct` itself is deduplicated
for readable chunks.

### [Important] NI-2 — `mt decode --json` is not valid JSON on its own stream in the tool's core (no-node) scenario

**Where** `crates/mt-cli/src/main.rs:942-965` (decode's JSON/prose both to
`stderr`), contrast `crates/mt-cli/src/main.rs:1104-1129` (inspect's JSON on
`stdout`, prose on `stderr`, kept separate).

**What** Described under I-8/PARTIAL above. `decode` reserves stdout for the
hex payload (by design, for the "only vouched-for bytes reach stdout" reason
stated at `main.rs:974`), so its `--json` report has nowhere to go but stderr —
the same stream every prose warning also uses. Whenever any notice fires
(no-node warning fires whenever `node.is_none()`, i.e. every offline run, which
is this constellation's primary posture), the combined stream is JSON followed
by prose, which no JSON parser accepts whole.

**The near miss** The realistic caller: `mt decode --json --bitcoin-cli
/nonexistent < even.txt 2>report.json`, then `json.load(open('report.json'))`.

**How I reproduced it**
```
$ mt decode --json --bitcoin-cli /nonexistent < even.txt 2>&1 >/dev/null | python3 -c \
  "import json,sys; json.load(sys.stdin)"
json.decoder.JSONDecodeError: Extra data: line 17 column 1 (char 585)
```
The existing test (`refusals.rs:1205`) does not catch this because it slices
`err[err.find('{')..=err.rfind('}')]` before parsing — it validates that *a*
JSON object is embedded somewhere in stderr, not that stderr *is* JSON.

### [Minor] NI-3 — `mt decode`'s transliteration notice prints three times

**Where** `crates/mt-cli/src/main.rs:966-968`.

**What** Three consecutive, identical calls:
```rust
transliteration_notices(&read, &mut stderr);
transliteration_notices(&read, &mut stderr);
transliteration_notices(&read, &mut stderr);
```
`verify` (`main.rs:1009`) and `inspect` (`main.rs:1130`) each call it once.
`decode` alone triples it — almost certainly a copy/paste left over from
drafting, since the function is new to this fold (it did not exist before
`e425c65`).

**How I reproduced it**
```
$ mt decode --bitcoin-cli /nonexistent < b8.txt 2>&1 >/dev/null | grep -c "CHARACTERS mt READ"
3
$ mt verify --bitcoin-cli /nonexistent < b8.txt 2>&1 | grep -c "CHARACTERS mt READ"
1
$ mt inspect --bitcoin-cli /nonexistent < b8.txt 2>&1 >/dev/null | grep -c "CHARACTERS mt READ"
1
```
No wrong fact is asserted (the tripled block is byte-identical each time), so
this is cosmetic clutter rather than a misleading claim — but it is exactly
the kind of noise that trains an operator to skim a screen that, in the
b/6/8 case, is carrying a real distinction (I-7).

## Near-miss inputs I constructed per guard, and the result

**`positional_autocorrect` (`read_strings.rs:115` region)**
- Wrong-bytes fuzz, general confusables (`1`/`i`/`o`/`b`, 1 confusable + 0-4
  substitutions, reusing the round-3 `probe.py` harness against the *current*
  binary): 8 seeds × 300 = 2,400 trials, 0 wrong bytes (acceptance rate ~3.9%,
  consistent with round 3's ~2.7-4% range for a random confusable roll).
- Wrong-bytes fuzz, targeted at the new two-candidate `b` tie-break (1-3 `b`s +
  0-3 ordinary substitutions per trial, new harness `/tmp/mtr/probe_b.py`):
  12 seeds × 400 = 4,800 trials, 0 wrong bytes.
- Candidate-explosion cap: confirmed `MAX = 64` in `positional_autocorrect`
  (`read_strings.rs`, the `next.len() >= MAX` break). A string with 84 `b`s
  (`all_b.txt`) runs in 7ms and REFUSES safely (correctly — 84 wrong positions
  is nowhere near t=4). A string with exactly 6 `b`s at real confusable
  positions (2⁶ = 64, exactly the cap) recovers the **byte-identical correct**
  original transaction (`six_b.txt`, diffed against `even.rawhex`) — the cap
  does not cost correctness at the boundary it was sized for.
- Legitimate elided line beginning `mtl` by chance (the exact regression
  `5462bab` fixed, retested against the current binary in case the multi-
  candidate rewrite reopened it): `el_mtl_fresh.txt` — `verify: OK`, transaction
  re-derives, BCH repairs all three prefix positions. Still clean.

**`mtl`-separator refusal (`restore_elided`, `read_strings.rs:416+`)**
- Misread separator + length-changing second defect (dropped character), both
  non-elided and elided forms: **still misdiagnosed**, see I-6/NOT FIXED above
  — this is the finding, not a new near miss.
- Misread separator + length-*preserving* second defect (a substitution),
  constructed fresh as the boundary the guard's length check actually
  implements: both non-elided (`sep_sub.txt`) and elided (`el_sep_sub.txt`)
  forms now decode correctly, BCH repairing the single substitution. This
  narrower case works; it is not what the report reproduced.

**`explain_failure` (`main.rs:1199+`)**
- `dup_dmg.txt` reran verbatim (I-3's own repro): fixed, see table.
- `short_dmg.txt` reran verbatim (I-4's own repro): fixed, see table.
- `dup_unreadable.txt` (new): duplicated **unreadable** line + a truly missing
  chunk — false "nothing is necessarily lost." See NI-1.

**`length_report` (`read_strings.rs:283+`)**
- `un_two.txt` reran verbatim (I-5's own repro): **still broken**, see table.

**`json_unsupported_guard` (`main.rs`, new)**
- `verify --json`, `encode --json`: correctly refuse. No legitimate invocation
  found that these should have accepted — both verbs genuinely have no report
  to serialise, matching the finding's own suggested fix.
- `decode --json`, `inspect --json`: see I-8/PARTIAL and NI-2.

**`txid_paste_guard` on the reading verbs (`main.rs:693`, `read_input`)**
- Confirmed positive: bare txid piped to `decode` now refuses (M-12).
- Looked for a legitimate 64-hex-character input the guard could now wrongly
  catch on a reading verb. None found: every `mt1` string begins `mt1`, and
  `t` is not a hex digit, so no full string can ever match. An *elided* line
  is pure bech32 payload and could in principle be all-hex-valid by chance,
  but bech32's alphabet has 14 of 32 symbols hex-valid, so a 64-character
  elided line matching by chance is ~(14/32)⁶⁴ ≈ 10⁻²³ — same order of
  argument round 3 already made for base64 PSBTs and encode's copy of this
  guard, and it extends unchanged to the reading verbs.

**Malleability caveat (`report.rs:no_node_warning`, gated on `has_legacy`)**
- Negative (SegWit-only, `even.txt`): no caveat. Confirmed (I-1).
- Positive (a genuine legacy input): built a real legacy-spending PSBT offline
  by replicating `tests/legacy.rs`'s `legacy_psbt` fixture in a throwaway
  `bitcoin = "=0.32.102"` crate (built `--offline` against the already-cached
  registry, no network use), ran it through `mt encode` to get real `mt1`
  strings, then `mt decode` on those strings. The caveat **does** print. Both
  `decode`'s and `inspect`'s call sites use the same `tx.input.iter().any(|i|
  i.witness.is_empty())` predicate — consistent, no divergence between them.
