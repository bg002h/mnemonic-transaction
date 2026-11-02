**⚠ 11 Important (0 Critical): every CONFIRMED mt bug-hunt finding is Important; 3 triage grades raised M→I (#6, #8, #12).**
architect grade-bughunt-mt: 12 answered; 0 prior rulings changed

# Grade and rulings: mt bug hunt (triage 7229177)

Input: `design/agent-reports/bughunt-triage-mt.md` (7229177) and the scratch fixes
`/scratch/cache/bughunt/mt/fix-NN.patch`. Settled set: #1-8 and #10-12 CONFIRMED; #9 FALSE
POSITIVE, not graded. Spec line numbers are for this worktree's `design/` spec.

## 1. Grades (likelihood test: Minor only if all of (1) fail-closed, (2) no failing printed command, (3) two or more listed unusual conditions hold)

| # | verb (placement) | grade | triage | reason |
|---|---|---|---|---|
| 1 | encode (stays) | I | I | §8.6 guard fails open. A keyless taproot script-path witness `[OP_TRUE, control block]` passes `has_signature_guard` because the script-path arm needs `w.len() > 2`. BIP-341 says "at least two witness elements left" means script path. Test (1) fails: the tool produces a card. Note: triage's rc=1 is only the fee prompt stopping the run (`--bitcoin-cli /nonexistent`). No §8.6 refusal fires. |
| 2 | encode (stays) | I | I | §8.6(a) SIGHASH_ALL check is skipped for a 64-byte DER signature (`30 3d`), so a SIGHASH_NONE signature is accepted. That is a wrong result: the outputs are unbound. It is rare for an honest signer (about 2^-56), but test (1) fails, and a wrong result stays I however rare. |
| 3 | encode (stays) | I | I | False refusal: a validly signed legacy P2SH 2-of-3 is refused with "input 0's satisfaction carries no signature" because the BIP-147 `OP_0` dummy stops the scriptSig push walk. Test (3) fails: a plain P2SH multisig is not unusual. |
| 4 | codec, read by decode (stays) | I | I | `pipeline` never compares `header.count` across chunks, so the library returns bytes from chunks of two sets of different sizes. The CLI `content_id_guard` catches it, and a mixed transaction cannot be validly signed, so there is no funds path. Still, the library returns a wrong result (test (1)), and library reach counts. |
| 5 | inspect node query (moves; C/I fixed now) | I | I | `has_txindex` is a substring test (`contains("txindex") && contains("\"synced\": true")`), so another synced index makes an unsynced txindex count as synced. A missing parent then prints PENDING where it should print UNKNOWN. That is a wrong status (test (1)). Core `rpc/node.cpp:385` (fetched master): `synced` is per index object, so the fix-05 approach is correct. |
| 6 | codec `decode_string`, read by decode (stays) | I | **M→I** | The library accepts the Kelvin sign (U+212A), which lowercases to `k`, where it should refuse it. The CLI refuses it. The data is identical, but test (1) requires a refusal, and the library does not refuse. BIP-173 line 80: the HRP is US-ASCII 33-126, and the data part is the bech32 alphabet only. |
| 7 | decode reader `read_strings.rs:599` (stays) | I | I | A panic (rc 101) on `MT1aaaaaaaéqqqqqqqqqq` (a char-boundary slice). A panic does not "name the problem", so test (1) fails. A single typo with a non-ASCII glyph is enough to reach it. |
| 8 | codec `invariant_prefix` (stays) | I | **M→I** | A panic on short input, library only. A panic is not fail-closed (test (1)), and the brief says library reach counts. |
| 10 | inspect JSON (moves), decode JSON (stays) | I | I | §10.10b (spec 3455): "Warnings and refusals are NEVER suppressed, on any verb". The JSON form drops DUPLICATE RESOLVED and CORRECTION APPLIED (inspect) and the correction notice (decode). This is an unmet guarantee. |
| 11 | decode flags (stays) | I | I | `--json --quiet` exits 0 and silently drops `--json`. The flag is silently ignored, the holder gets the wrong output, and the run looks successful (test (1)). |
| 12 | decode reader prefix repair (stays; repair-like logic) | I | **M→I** | Tests (1) and (2) hold: the tool refuses and names the problem. Test (3) fails. Triage lists three conditions, but (a) `1` read as `l` is not unusual: §1.1e (spec 1134-1145) calls it "the single most confusable glyph in the set", present in "every string a person types". And (c) "no BCH help" follows from (b) rather than being a separate condition. That leaves one unusual condition, and test (3) needs two. A spec-promised free repair then fails on the commonest misread plus one more confusable. Close call; I, so it is fixed now. |

Placement: no finding is Minor, so nothing defers to the toolkit migration. #5 and #10 (inspect) and #12
(repair-like) are in verbs or logic slated to move. As C/I they are fixed now (ruling 2026-11-01).

## 2. Rulings (exact behaviour)

Every refusal below uses the §10.10 format (`refusal.rs:78`, spec 2121) and exits 1
(`ExitCode::FAILURE`); spec 3488 leaves other codes to the implementation. "No new text" means the
FIX quotes the existing string verbatim.

- **R1 (#1).** Script path is detected per BIP-341: after removing an annex (last element starting
  0x50, when there are at least two elements), **two or more** elements left means script path. A
  script-path witness with no signature-shaped element is refused with the existing `has_signature_guard`
  text, no new text: `mt encode: REFUSED — §8.6, input {n}'s satisfaction carries no signature`, exit 1.
  RED row: the bughunt 01 witness `[OP_TRUE, control block]`.
- **R2 (#2).** Signature shape covers the whole DER length range: `30 3d` (64-byte DER plus sighash)
  through `30 45`, and taproot 64 or 65 bytes. A non-ALL sighash is refused with the existing
  `sighash_all_guard` text, no new text: `mt encode: REFUSED — §8.6, input {n} is signed with sighash
  0x{byte:02x}, not SIGHASH_ALL`, exit 1.
- **R3 (#3).** The scriptSig walk treats every push opcode as a push: 0x00 → empty element;
  0x01-0x4b, 0x4c/0x4d/0x4e (PUSHDATA1/2/4) → data; 0x4f and 0x51-0x60 → small-integer element. Any
  other opcode ends the walk, as today. Behaviour: a validly signed P2SH 2-of-3 encodes (rc 0, no §8.6
  refusal). Extra RED row: an `OP_0 <sig>` scriptSig whose sig is SIGHASH_NONE is still refused by R2.
- **R4 (#4).** New codec variant, not a new type (the crate is internal and `Error` is not
  `non_exhaustive`; only mt-cli matches it):
  `Error::CountMismatch { expected: usize, found: usize, index: usize }` (index 1-based, like
  `SetIdMismatch`). Display text: `chunk set size mismatch: expected {expected} chunks, chunk {index}
  carries {found}`. It is checked in `pipeline` immediately after the set-id check. fix-04's reuse of
  `IndexOutOfRange` is rejected: "chunk index 0 is out of range for a set of 3" misnames the
  fault. At the CLI it reaches `explain_failure`'s generic fallback, no new CLI text:
  `mt decode: REFUSED — §1.1, the set does not verify` followed by the variant text, exit 1.
- **R5 (#5).** `has_txindex` parses the `getindexinfo` reply as JSON. It returns true only when the
  object `txindex` exists and its `synced` is the boolean `true`. Anything else, including a parse
  failure, returns false, which gives UNKNOWN (the §6a fail-safe direction). No printed text changes.
- **R6 (#6).** `decode_string` refuses any non-ASCII character before case folding, with the existing
  `InvalidChar` variant, no new text: `character {ch:?} at data-part offset {position} is not in the
  bech32 alphabet`. `position` is the char offset in the data part. CLI behaviour is unchanged.
- **R7 (#7).** No panic. A string whose cut point is not a char boundary is unreadable. The CLI gives
  the existing `string {n} could not be read` refusal, no new text, exit 1. The RED row must also assert
  rc == 1 and that stderr contains `REFUSED`; the bughunt test checks only no-panic and rc ≠ 101.
- **R8 (#8).** `invariant_prefix` on short input returns the existing `Error::TooShort`, no new text.
- **R9 (#10).** In `--json`, every notice is one single-line string in the `warnings` array, the same
  for decode and inspect (spec 2671):
  - Duplicate: `chunk {n} was present twice; kept the copy needing {k} corrections`. This is decode's
    existing text; inspect adopts it.
  - Correction: one entry per repaired chunk, ordered as the prose `margin_report` orders them:
    `chunk {n} needed {c} of 4 corrections`, with `; NO MARGIN LEFT` appended when c ≥ 4.
  - The JSON omits the prose before/after detail. That was a tie, decided for the smaller text.
  - `render_json` (`report.rs:432`) also escapes U+0000-U+001F (`\n`, `\r`, `\t`, `\b`, `\f`, else
    `\u00XX`), per RFC 8259 §7 line 429 ("MUST be escaped ... control characters (U+0000 through
    U+001F)"). This is latent today (a probe with \x01 was refused before render), but the new
    warning strings carry user-derived content.
- **R10 (#11).** Refuse, do not honour either flag. The refusal follows the `json_unsupported_guard`
  precedent (`main.rs:2003`): §10.10 format, exit 1, not clap's exit 2. Exact text:
  `mt decode: REFUSED — §10.10, --json and --quiet contradict each other`;
  mechanism: ``--json` asks for the inspection report as a document and `--quiet` asks for no
  report; mt refuses rather than silently ignoring one.``;
  remedy: ``Drop `--quiet` for the JSON report on stderr, or drop `--json` for the hex alone.``
  It applies to every verb that takes both flags (`ReadArgs`). For inspect and verify, R11 fires
  first.
- **R11 (new, executed, untriaged).** `inspect --quiet` and `verify --quiet` are silently ignored. I
  ran both with and without `--quiet`, on the same input: inspect rc 0, stdout 616 bytes and stderr
  950 bytes either way; verify rc 0, stderr 65 bytes either way. `args.quiet` is read only at
  `main.rs:734` and `:1246`.
  - Ruling: refuse, mirroring the json guard. `mt {verb}: REFUSED — §10.10, --quiet has no meaning
    for `{verb}``; mechanism: ``{verb}'s output is its report; mt refuses rather than print it
    anyway.``; remedy: ``Drop `--quiet`.``; exit 1.
  - The `ReadArgs.quiet` doc stays. No current test uses `inspect --quiet` or `verify --quiet`.
  - Send it through triage (a RED row) before FIX-C adopts it.
- **R12 (#12).** Keep the free `mtl`/`mti`→`mt1` repair when the repaired string decodes directly
  *or* after positional autocorrect (fix-12's approach). Both notices print: the existing prefix
  notice and the existing autocorrect notice, so the autocorrect is never silent (§1.1e). There is
  no new text. The repair costs nothing from the t=4 budget. The existing "and something else wrong
  too" refusal still fires when neither path decodes.

Manual rows: none change. The toolkit manual (`origin/master` `docs/manual/src/40-cli-reference/`) has
41-mnemonic, 42-md, 43-ms, 44-mk-cli and 45-mr only; there is **no mt chapter**, and the mt repo has
no `docs/`. See open point O1.

## 3. FIX documents (all mt repo only; no new flag, type or command)

| FIX | findings | track | spec delta |
|---|---|---|---|
| FIX-A encode §8.6 shape | #1, #2, #3 (R1-R3) | small | One §8.6 sentence: "script path = two or more witness elements after the annex (BIP-341)" and "push opcodes include OP_0 and OP_1-16". Class B review (PSBT/transaction code). |
| FIX-B codec robustness | #4, #6, #8 (R4, R6, R8) | small | One sentence in the §1.1 decode rules, that every chunk carries the same count. One enum variant, which is not a new type. |
| FIX-C reader, JSON and flags | #7, #10, #11, #12, R11 if triage confirms (R7, R9-R12) | small | §10.10b gains the JSON warning texts (R9) and the two flag refusals (R10, R11). That is about 4 sentences, the top of the small-track budget. If R11 is not confirmed, drop it. Class A review (printed text). |
| FIX-D node txindex | #5 (R5) | small | None; behaviour already matches §6a. |

Order: FIX-A first (funds path), then FIX-C, then B and D. They touch disjoint files, except that B and C both
touch the reader's error mapping, so build B before C.

## 4. Open points (not decided here)

- **O1 (process).** No toolkit manual chapter exists for `mt`, so the "flag or printed-text change
  updates the manual in lockstep" rule has no target. Should FIX-C open one, or is mt exempt until the
  migration? This is a product and process decision for the coordinator.
- **O2 (untested).** Mixing chunks from two signings of the same taproot transaction (same count, same
  set id, different witnesses) is not covered by any hunt row. Recommend one more probe in the next
  hunt.
- **O3.** R11 needs a triage verdict (RED on unmodified code, GREEN with the guard) before FIX-C cites
  it.

## 5. Follow-ups (owning phase)

- Triage's note on a mode-0644 output permissions warning: secret-handling, never C/I (operator ruling
  2026-08-27). File it as a follow-up with its reproduction; owner: mt hardening backlog.
- #9 FALSE POSITIVE: no action.
- decode JSON goes to stderr and inspect JSON to stdout. This matches the spec (decode's report is on
  stderr). No action.
- encode's CLAIMED FEE row: noticed by triage, untested, no finding. Owner: next mt hunt.

Counts: 11 graded: 0 C, 11 I, 0 M, 0 N; 3 grades changed from triage (#6, #8, #12 M→I); 12 product points ruled (R1-R12); 3 open points; 4 FIX documents, all small-fix track.
