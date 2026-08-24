# R-round5-verify — mechanical fold check of `R-round4-fold-check.md`'s five claimed fixes

Independent reviewer. Wrote none of this code, wrote none of `R-round4-fold-check.md`.

Scope: exactly the five items named in the dispatch brief (I-5, I-6, `--json`
finding 8/NI-2, the duplicated-unreadable-line NI-1, and the tripled
`transliteration_notices` NI-3), covering the fold at `2001870`/`3fb3055`
(range `b22c322..HEAD`). No fresh audit, no new lenses.

Method: `cargo build -p mt-cli --locked` (after `touch`ing `main.rs` to force a
real recompile — confirmed "Compiling mt-cli" in the output, not a cache hit).
All repro strings constructed fresh in my own scratch dir
(`/tmp/claude-.../scratchpad/mtv5`), not reused from any prior reviewer's
`/tmp/mtr`, using the exact python one-liners quoted in `R-round3-near-miss.md`
(for `un_two.txt`, `sep_drop.txt`, `el_sep_drop.txt`) and the exact prose
description in `R-round4-fold-check.md` (for `dup_unreadable.txt`, since that
report gave a description, not a script, for that one fixture). Every run below
is offline via `--bitcoin-cli /nonexistent`. `cargo nextest run --locked`: 201
tests, 201 passed, 0 skipped, matching the brief's already-settled count.

Facts taken as already settled, not re-derived: fmt/clippy/journeys/refusal
gates/check-provenance/live smoke test all pass; `mt qr`, §8.7, §8.7c, §8.8
deferred; no script engine; mainnet-params-on-regtest and the spec's
`~FALL 2034` self-disagreement are known and out of scope.

## Verdict

`4 FIXED / 1 PARTIAL / 0 NOT FIXED` of five. Nothing broken: full suite is
201/201, and every legitimate-input regression check below still behaves
correctly.

The one PARTIAL (I-6) is a real, mechanistically-confirmed gap, not a
reviewer quibble: the fix works when the set has at least one *other*
undamaged `mt1`-prefixed string to establish "full length" against. It is
blind when the corrupted string is the *only* one that ever carried the full
invariant prefix — which is exactly the `--elide-prefix` case the finding's
own reproduction included from round 3 onward.

## The five

| Fix | Status | Command | What it printed |
|---|---|---|---|
| **I-5** — `length_report` ambiguity now per-suspect | **FIXED** | `mt verify --bitcoin-cli /nonexistent < un_two.txt` (rebuilt from `R-round3-near-miss.md`'s exact script: `uneven.txt` string 4 gets 1 char dropped, string 8 gets 5 substitutions at intact length) | `string 4: 84 characters, where this set's usual length is 85` / `string 8: 79 characters, where this set's usual length is 85` — no fabricated deficit on either suspect (was: `string 8: 79 characters (expected 85) — 6 characters are MISSING`, byte-identical wrong output as of round 4). Both hedged, neither accused. |
| **I-6** — `mtl`-separator guard, nearer-full-than-elided | **PARTIAL** | Non-elided: `mt decode --bitcoin-cli /nonexistent < sep_drop.txt` (rebuilt from R-round3's script: `mtl` + dropped char on an otherwise-intact 6-string set). Elided: `mt verify --bitcoin-cli /nonexistent < el_sep_drop.txt` (same damage applied to the ONE full-prefix line of a freshly-built `--elide-prefix` set) | Non-elided: **FIXED** — `mt tried the obvious repair (that character is the 1 of the mt1 prefix) and the string still did not read, so there is a SECOND defect on the same plate... Re-read that plate` (was: false `10 characters are EXTRA`). Elided: **NOT FIXED, byte-identical to round 4** — `REFUSED — §3b, all 6 lines are elided; no prefix to restore` / `Add the 8 characters following mt1 on any intact string of the same set` — same wrong message as before the fold. Root cause read at `crates/mt-cli/src/read_strings.rs:438-451`: `full_len` is computed only from candidates that already `starts_with("mt1")`. In the elided fixture, the damaged line is the *only* candidate that was ever full-length, and it now starts with `mtl`, so it is excluded from that filter — `full_len` is `None`, `looks_full` is `is_some_and` on `None` (always `false`), and the guard can never fire on this shape by construction, not by threshold. |
| **`--json` (finding 8 / NI-2)** — warnings inside the document, decided before writing | **FIXED** | `mt decode --json --bitcoin-cli /nonexistent < even.txt 2>decode.stderr >decode.stdout`, then `json.loads()` (not `.find('{')` slicing) on the **whole** stderr file; separately `python3 -c "json.load(open('inspect.stdout'))"` on `mt inspect --json`'s **whole** stdout | decode: stderr parses whole (`keys: ['fee','inputs','locktime','outputs','set_prefix','status','strings','txid','warnings']`, `warnings: ['no bitcoind reachable...']`), stdout is hex-only (1 line). inspect: stdout parses whole with the same shape, stderr is empty under `--json`. `verify --json` and `encode --json` still correctly REFUSE §10.10 (unchanged, reconfirmed). |
| **duplicated unreadable line (NI-1)** — dedup `unreadable` by content | **FIXED** | `mt verify --bitcoin-cli /nonexistent < dup_unreadable.txt` (rebuilt per R-round4's prose description: 6-chunk set, chunks 1-4 clean, chunk 5 damaged past t=4 with 5 substitutions, chunk 5 typed twice, chunk 6 never typed) | `This set has 6 chunks and only 4 distinct ones are present, even counting the 1 unreadable string(s) as chunks you hold. SO A PLATE IS MISSING AS WELL AS DAMAGED` — correctly deduplicated (was the false `4 read cleanly and 2 did not... nothing is necessarily lost`). |
| **Minor — tripled `transliteration_notices`** | **FIXED** | `mt decode --bitcoin-cli /nonexistent < b8.txt 2>&1 >/dev/null \| grep -c "CHARACTERS mt READ"` | `1` (was `3`). `verify` and `inspect` on the same input both also print `1`, matching. |

## Legitimate inputs I re-checked

| Input | Expected | Actual |
|---|---|---|
| Uneven set (`uneven.txt`, 8 chunks, final string legitimately 79 chars), no damage | decodes OK, not flagged suspicious | `mt verify --bitcoin-cli /nonexistent < uneven.txt` → `OK — 8 chunks, set 0x3b426, transaction re-derives.` |
| Elided line beginning `mtl` by chance (3 in-budget substitutions on a genuinely elided line, `even_elided.txt` line 2 → `el_mtl.txt`) | decodes correctly, BCH-repaired, not refused as a misread separator | `mt verify --bitcoin-cli /nonexistent < el_mtl.txt` → `OK — 6 chunks... CORRECTION APPLIED. 1 chunk needed repair: chunk 2, 3 of 4 symbols (pos 12 m→q, pos 13 t→q, pos 14 l→p)`. The I-6 fold did not regress the `5462bab` protection. |
| Duplicated READABLE chunk (`dup_only.txt`: plate 2 typed twice, plate 3 skipped, nothing damaged) | refuses, reports the duplicate | `mt verify --bitcoin-cli /nonexistent < dup_only.txt` → `REFUSED — §1.1... chunk 3 of 6 is missing. Chunk 2 arrived TWICE. If you are working from a stack...` — unchanged, correct. |
| `--json` on `inspect` and on `decode`, both parsed whole | both parse as single JSON documents, `json.loads`/`json.load` succeed with no leftover data | Both PARSES OK (see table above); no `Extra data` error on either stream. |

## Incidental

None outside the five and their direct regression checks.
