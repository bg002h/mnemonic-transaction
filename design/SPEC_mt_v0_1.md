# SPEC — `mt`, the mnemonic-transaction format (v0.1 draft)

Status: **GREEN — 0 Critical / 0 Important, 2026-08-23.** Closed after R6 (three
lenses: fold-propagation, implementability, adversarial — 6C/27I) and R7's fold
verification, then held green through three live journey walks, two out-of-scope
sweeps and the pre-implementation gates R11/R12. Risk-set work by the project's
own definition — it touches funds, addresses and a new normative format — so the
post-implementation adversarial review over the whole diff remains mandatory and
non-deferrable.

> **This line said "DRAFT, in R0 … no code may be written" until 2026-08-23 —
> R12 B5.** It predated R6/R7 closing the spec and was outside every grep the
> intervening gates ran. **Anyone opening the spec directly — which they must,
> it is the source of truth — hit the same contradiction R12's C1 was about**:
> a document telling its reader not to proceed, with nothing in it recording
> that the condition had been met.

Written 2026-08-22 from a brainstorm with the operator; folded 2026-08-23 after
R0 round 0. Every number in it was measured; the probes and raw results are in
`design/measurements/`, and the reproduce path is a command, not a memory.

---

## 0. What this is, in one paragraph

`mt` engraves a **signed Bitcoin transaction** on steel. It is handed a
transaction that is already built, already signed and already finalized, and it
renders it in one of **two forms**, because two different people engrave two
different ways:

| verb | form | engraved how | payload | size limit |
| --- | --- | --- | --- | --- |
| **`mt encode`** | `mt1` chunked codex32, **on stdout** | **by hand**, or by any tool the operator chooses | raw signed transaction (§3b) | **32,768 chunks / ~1,280 KB** |
| ~~`mt qr`~~ | ~~QR symbols + legend~~ | **DEFERRED out of v0.1 — §0a** | | |

`mt qr` decides how many symbols that takes and at what error-correction level.
**Neither verb decides how the result is laid onto steel** — one string per
plate, all of them on one, or any split, is the operator's choice or
`mnemonic-engrave`'s (§0a). `mt encode` emits a
character string with **BCH error correction**, so a hand engraver who cuts a
character wrong can still recover the transaction.

That is the whole of it. It does not build transactions, does not sign, holds no
private key, does not choose which UTXOs to spend or what fee to pay, and does
not invent an encoding for the transaction itself.

> **Scope ruling, operator, 2026-08-23 — transaction construction is removed.**
> An earlier draft had three verbs: produce, present, engrave. **Produce and
> present are gone; engrave split in two.** *"Constructing transactions is really a wallet function, and we
> want users to test their wallets in wallet software before going through all
> the work to hand engrave or machine engrave durable backups."* Presenting a
> PSBT to a signing device falls to the same argument — the wallet already does
> it, and doing it here would add a second path that nobody exercises on the way
> to a plate.
>
> This deleted the section the spec itself called *"the most dangerous section in
> this spec"* (the four-tier input-amount trust ladder), and with it two of the
> Criticals R0 round 0 found. **A tool that never builds a transaction can never
> get an input amount wrong.**

**What that shifts, and it is not a pure simplification.** `mt` is now a *pure
receiver* of transactions built elsewhere. Everything it can still get wrong is
a failure to inspect what it was handed, so §8 — the refusals — carries the
entire safety argument. It gets stricter in this fold, not looser.

## 0a. `mt qr` is DEFERRED out of v0.1

**Operator ruling 2026-08-23.** v0.1 has **one verb**: `mt encode`. QR
conversion is deferred to its own cycle.

**The reason is that QR is a CROSS-FORMAT concern, not an `mt` one.** `md1` and
`mk1` will want the same conversion, and building it inside `mt` first would
mean either duplicating it for them or refactoring it out later. Where it
belongs — a shared crate, the toolkit, or `me` — is a design question that
deserves its own cycle rather than being settled as a side effect of shipping a
transaction format. The same instinct made `me` the constellation's single
`sysw` writer (§10.9).

**What it costs: NO artifact loses its path.** `mt1`'s ceiling is **32,768
chunks / 1,310,720 bytes** (§3), above Bitcoin's own ~100 KB standardness limit —
so every transaction that will relay can be encoded, including RCW `wsh` tier 1
at five inputs (89 chunks, **2.2%** of the ceiling).

> **An earlier version of this section said the cut "costs one artifact of
> seven", naming that same 89-chunk wallet as losing its path.** That rested on
> a 64-chunk ceiling `mt1` never had — `md-codec`'s 6-bit `count` field, which
> §3 widened to 15 bits. The scope cut costs **machine engraving**, not
> transaction sizes.

**What it removes from v0.1**, all of it QR-only: §4's entire configuration
search, §5's plate legend, the `sysw` transaction `Class`, the record framing,
§8.7c's transport ceiling, and §10.17's firmware work. Four of the open
questions in §10 go with them. **Those sections are retained in this document
rather than deleted**, because the measurements behind them are real and the
next cycle starts from them — but nothing in them binds v0.1.

**What remains is small enough to state in a sentence:** read a finalized PSBT,
validate it per §8, and emit an `mt1` chunked codex32 string on stdout, with
warnings on stderr.

> **A consequence worth naming: a v0.1 plate carries the string and nothing
> else.** §5's legend is `mt qr`'s, and `mt encode`'s layout is the operator's
> by ruling (§3b) — so no `BEARER` line, no `FROM`/`TO`, no locktime line
> reaches the steel unless the operator puts it there.
>
> **`mt encode` therefore PRINTS suggested legend text on `stderr`**, which the
> operator may engrave beside their strings. `mt` does not control the layout
> and does not withhold the words.
>
> **`stdout` IS THE STRINGS AND NOTHING ELSE — operator ruling 2026-08-23, and
> it is a hard interface boundary, not a formatting preference.** The output of
> `mt encode` exists to be **piped**: into a file, into `mt qr` when that cycle
> lands, into whatever the operator's engraving path is. The moment a legend
> line, a banner, a count or a blank separator shares that stream, every
> downstream consumer has to parse `mt`'s prose out of its own input — and the
> first one that forgets engraves a warning label as if it were a chunk.
> **Suggested legend text is `stderr`, unconditionally**, alongside the
> warnings, for exactly the reason §3b already gives: stdout carries the
> artifact, stderr carries everything a human must see.
>
> **And the honest consequence, which this spec must design around rather than
> hope away: the realistic plate has NO legend on it.** The operator ruling is
> blunt — the legend is *"a warning message that a user would be encouraged to
> engrave but probably won't."* Every line of it is optional, hand-cut, and paid
> for in the operator's own labour, at the end of a job whose real product is
> already finished.
>
> So **`mt` may not treat any legend field as present**, and no journey in this
> spec may be walked from a plate that has one. The recovery path that has to
> work is the bare one: **`mt1` strings, nothing else, into `mt inspect`** —
> which is precisely why §1.1 has `inspect` consult a node and reconstruct
> what the legend would have said (§10.21 closes on the single field inspection
> *cannot* reconstruct). A legend, where an operator does cut one, is a
> **convenience that shortens a recovery** — never a component one depends on.
>
> **It is NOT §5's field set applied verbatim, and an earlier version of this
> section said it was — U-5.** §5's set was designed for a `mt qr` plate, where
> every symbol sits beside one legend. Hand-engraved strings split the text in
> two:
>
> | printed | text |
> | --- | --- |
> | **once** | `BEARER…`, `FROM`, `TO`, `LOCKED TO BLOCK n ~SEASON year`, `FORMAT: mt1 codex32` |
> | **per string** | `n/m` — string `n` of `m`, which `mt` knows exactly |
>
> **`PLATE n OF m` is dropped, because `mt` cannot compute `m`.** §3b rules that
> how many plates you use is the operator's decision, so the denominator would
> be invented — and **`PLATE 1 OF 1` cut onto each of five plates is a false
> completeness claim on permanent steel**, read by someone who then stops
> looking for the other four.
>
> **`STRING n OF m` is strictly better anyway**, not merely computable: it names
> which *data unit* is missing rather than which plate, and it survives any
> layout — three strings per plate, one per plate, all of them on one.
>
> **Set membership needs no legend field at all.** The header packs its
> invariant fields first — `version(5) + chunk_set_id(20) + count(15)` — so
> bits 0–39 are identical across a set, and at 5 bits per symbol **the first 8
> characters after `mt1` are the same on every string in it**.
> Verified on real `md1` output, where four chunks of one wallet all read
> `md1fveszps…`. A recoverer groups plates **by eye, without decoding**, so
> `mt encode` prints the shared prefix once and tells them the rule:
>
>     All 14 strings begin `mt1qzrf8xk2`. Strings sharing that prefix belong
>     to this transaction; strings that do not, do not.

## 1. The operator's decisions, recorded

Each of these is a ruling, with the reasoning that produced it. Several
overturned an earlier assumption and are marked.

1. **`mt` is the constellation's fourth format tool, with the same verbs as the
   others.** Operator ruling 2026-08-23: **`encode`**, plus **`decode`**,
   **`verify`** and **`inspect`**. `md` and `mk` both carry exactly this set —
   neither has a verb named `string`, and both call the emit path `encode`.

       md encode  ->  descriptor      ->  md1 string(s)
       mk encode  ->  key card        ->  mk1 string(s)
       mt encode  ->  finalized PSBT  ->  mt1 string(s)

   **This renames the previous draft's `mt string`.** That name only made sense
   as a contrast with `mt qr`; with the QR verb deferred (§0a) the contrast is
   gone, and `encode` is what a user who already drives `md` will reach for.

   **`decode` is not optional, and §9 said the opposite until 2026-08-23.** §9
   claimed v0.1 shipped no decoder, which was written when "reading a plate"
   meant the deferred static-scan verb. §9 now carries the retraction and the
   distinction that resolves it: **optical reading stays deferred; reassembling
   `mt1` strings does not**, because it needs no scanner and no camera. See
   decision 1a above.

   **`verify` and `inspect` follow the siblings, whose division is consistent
   across all three.** Read from their own help text:

   | | `verify` | `inspect` |
   | --- | --- | --- |
   | `md` | *"Verify backup strings re-encode to a given template"* — `--template` **required** | *"Decode + pretty-print everything the codec sees"* |
   | `mk` | *"BCH check + **optional** content match"* | *"structural commentary in addition to decode"* |
   | `ms` | *"is valid (and **optionally** round-trips against a phrase)"* | *"structural fields and decoder verdict"* |

   **ALL HUMAN-FACING OUTPUT NUMBERS CHUNKS FROM 1** — `chunk n` means wire
   `index n−1`. The zero-based `index` is a wire field (§10.13 a2) and **appears
   nowhere in output**.

   > **The document was 0-based in its rules and 1-based in every report, and
   > the correction report sat between them saying only "chunk 7" — R6
   > implementability I-6.** Both readings were supported, and the consequence
   > is not cosmetic: that report ends *"Chunk 7 is at its correction limit …
   > **Re-cut it.**"* **An operator re-cutting the wrong string spends ~21
   > minutes duplicating a good plate and leaves the one-scratch-from-
   > unrecoverable string on the shelf.** The same ambiguity governs the FAILED
   > report's ranked suspect list, whose entire value is telling the operator
   > *which three of fourteen* to retype.
   >
   > One rule rather than two: **positions and chunk numbers are both 1-based
   > in output**, wire fields are 0-based, and each conversion is stated as an
   > equation where it is used.

   **`mt verify` is STRUCTURAL ONLY.** Operator ruling 2026-08-23, and it is
   what the siblings already do — **none of the three touches external state.**
   It checks: every string parses, every BCH checksum holds, the set is complete
   (**chunks 1 through `count` present**, i.e. wire indices `0..count-1`), every
   chunk carries the same
   `chunk_set_id`, and the reassembled transaction re-derives that id.

   **TWO CHUNKS SHARING AN INDEX IS NOT AN ERROR, AND AN EARLIER DRAFT OF THIS
   LINE MADE IT ONE.** It required *"no duplicates"* — which put this section in
   direct contradiction with **§1.8**, whose entire answer to plate loss is
   *"the operator is free to engrave duplicate copies."* **The one mitigation
   the spec offers for its largest accepted risk was the one thing `verify`
   rejected**, so an operator who followed §1.8 was rewarded with a set that
   would not verify. Found by walking Journey C, 2026-08-23.

   It bites hardest in the case that produces duplicates *without anyone
   choosing to*: an operator re-cuts a miscut string, and the old plate is still
   in the drawer. Now two chunks claim index 7, **they are not identical**, and
   a recoverer who deletes "the extra one" has even odds of keeping the bad copy
   and binning the good one.

   **The mechanism to resolve this already exists and was going unused: every
   chunk carries its own BCH checksum**, so each candidate is testable
   independently of the others.

   | two chunks, same index | what is true | what `mt` does |
   | --- | --- | --- |
   | **one passes BCH, the rest fail** | the re-cut case — the miscut copies are detectably bad | **use the good one, and ANNOUNCE it** — printed as a finding, not a log line. See the note on what `mt` does and does not have proof of |
   | **both pass, bytes identical** | a deliberate duplicate copy, per §1.8 | **accept silently.** This is the spec's own advice being followed, and reporting it would make correct behaviour look like a problem |
   | **two or more pass, bytes differ** | valid chunks disagreeing — different transactions, or damage that landed inside the code word | **refuse loudly.** The only genuinely ambiguous case, and the only one where guessing could pick a wrong transaction |

   **"BYTES" MEANS THE POST-CORRECTION PAYLOAD, NEVER THE AS-TYPED
   CHARACTERS.** R6 implementability I-4. Two copies of one chunk that differ
   only by an error BCH repaired are **the same chunk** — identical after
   correction, different before — so comparing as-typed characters would send
   the commonest real case (a re-cut plate plus a slightly mistyped
   transcription) to the *refuse loudly* row.

   > **It also decides which row fires for the case §1.8 tells operators to
   > create.** An operator who cuts a spare copy, then types both back with one
   > fat-fingered character in one of them, has two chunks whose characters
   > differ and whose payloads are identical. Post-correction: accept silently,
   > which is right — they engraved a duplicate on purpose and both are good.
   > As-typed: refuse, on a set with nothing wrong with it.

   **THE RULE IS OVER `n` CANDIDATES, NOT TWO, AND MAJORITY VOTE IS FORBIDDEN.**
   R6 adversarial I-7. Partition the candidates at an index by BCH-validity,
   then the valid ones by exact bytes; **accept only if exactly one distinct
   valid byte string remains**, and refuse otherwise.

   > **Written for two, the table returned opposite verdicts on three.** Given
   > `{genuine, forgeA, forgeA}` at one index, a pairwise reading finds
   > `forgeA == forgeA` (row 2, accept silently) *and* `genuine ≠ forgeA` (row
   > 3, refuse loudly). An implementer resolving that contradiction by majority
   > vote **hands the decision to whoever can add the most strings** — and a
   > drawer holding a re-cut plate and two predecessors reaches three
   > candidates with nobody attacking anything.
   >
   > **Row 1 says `mt` has proof, and it does not.** It has proof that one
   > *checksum* holds — not that the surviving chunk is the one that was
   > engraved. **The genuine string fails BCH whenever the operator mistyped
   > more than `t = 4` characters**, which is an ordinary event: it is the
   > entire reason the margin report exists. So the discarded candidate may be
   > the real one, and the old wording promised *"no operator decision is
   > needed"* while making that decision silently.
   >
   > Hence **announce**. `mt` still picks the checksum-valid chunk — that is
   > the right default and the operator's ruling — but it says which candidate
   > it discarded and that a badly-mistyped genuine string looks exactly like
   > this. Row 2 keeps *accept silently* because byte-identical copies are
   > §1.8's own advice being followed, and there is nothing to decide.

   > **The third row is why this is a split rather than a relaxation.** Dropping
   > the duplicate rule outright would accept two disagreeing valid chunks and
   > silently use whichever arrived first. That is the failure this whole
   > document is organised against — a plausible answer where there should be a
   > refusal — and it is rare enough to be invisible in testing and severe
   > enough to matter.
   >
   > **Nothing here weakens the content-id check.** `verify` re-derives the id
   > from the reassembled transaction either way (§10.13 c), so the surviving
   > copy still has to produce the right transaction. Duplicate resolution
   > chooses *which bytes to try*; it does not decide whether the result is
   > correct.

   **What the correction DOES and DOES NOT cover — printed ALWAYS, before
   cutting.** Operator ruling 2026-08-23. Nothing in `mt`'s output contradicts
   the impression that "error correction" has the operator covered, and §1.8's
   zero-redundancy ruling lives only in the spec:

   | damage | BCH? | what catches it |
   | --- | --- | --- |
   | up to **4 wrong characters** per string | **corrects it** | — |
   | a **missing** or **extra** character | no — every later symbol shifts | the **length check** (decision 1e in §1) |
   | a **missing string** | no | `count`, and `n/m` beside each string |
   | a **lost plate** | no | **nothing. The transaction is gone** |

       Before you cut: mt corrects up to 4 wrong CHARACTERS per string.

         It cannot repair a MISSING or EXTRA character — those shift every
         symbol after them. Count each string: strings 1-13 are 90
         characters, string 14 is 72.

         It cannot repair a missing STRING or a lost PLATE. There is no
         redundancy: all 14 strings are required. To survive losing a
         plate, cut a second copy — mt will not do it for you.

   **Counting is the operator's own check on the damage BCH cannot touch**, and
   it is the failure a careful person actually has when hand-cutting over a
   thousand characters: losing their place, skipping a glyph, doubling one. That
   damage does not present as "four errors I can fix" — it presents as total
   garbage — yet it is trivially detectable by counting.

   **`verify` must be run against the STEEL, and `mt` says so.** After `encode`
   succeeds the operator holds two copies of the same strings: the ones on
   stdout and the ones they cut. **Verifying the file proves nothing about the
   plate** — it re-checks the tool's own output, which was correct by
   construction. The whole point of BCH is to catch what the *hand* got wrong.

       Now engrave these strings.

       When you are done, verify the ENGRAVING, not this output:
         mt verify < typed-from-plates.txt

       Type them back from the steel. Verifying the file you just created
       tests nothing that can fail.

   **`verify` REPORTS ITS MARGIN, not just its verdict.** Usability journey
   walk, U-2 — the one Critical it found, and five correctness rounds had missed
   it because nothing in the spec was *wrong*; a step was simply silent.

   BCH corrects up to **`t = 4` symbol errors per chunk** (§3a). A plate miscut
   in four places therefore **passes `verify` as OK** — while sitting **one
   scratch from unrecoverable**, with §1.8's zero redundancy behind it and no
   second copy unless the operator made one. A verdict that hides how much of
   its budget it just spent is telling the operator the opposite of what they
   need.

       mt verify: OK — 14 chunks, set 0x0e17e, transaction re-derives.

         CORRECTION APPLIED. 3 chunks needed repair:
           chunk  2   1 of 4 symbols   pos 61
           chunk  7   4 of 4 symbols   pos 13, 29, 30, 78   <-- NO MARGIN LEFT
           chunk 11   2 of 4 symbols   pos 9, 52

         chunk 7, with the corrections marked:
           MT1QZRF8XK2V[q>p]HQ9WRDG5S8XE7M2[v>d][8>g]4KP3NAYU6TC...
                                                   ...5J2W[l>1]E7RQ
             pos 13   read q   corrected to p
             pos 29   read v   corrected to d
             pos 30   read 8   corrected to g
             pos 78   read l   corrected to 1

         Chunk 7 is at its correction limit. One more damaged symbol in
         that string and this transaction is unrecoverable. Re-cut it.

   **`verify` LOCALISES every correction, and stops there.** Operator ruling
   2026-08-23, from walking Journey C: *"verify highlights the location of the
   errors and it is on the user to not be an idiot and check typing and
   engraving."*

   **The ambiguity `verify` does not resolve, and does not try to.** The
   operator types 1,242 characters back from steel, so a corrected symbol has
   two possible authors — **the engraving is wrong, or the typing is wrong** —
   and BCH cannot tell them apart. A wrong symbol is a wrong symbol regardless
   of which hand made it. The two call for opposite responses: a miscut chunk 7
   is one scratch from unrecoverable and must be re-cut, while four fat-fingered
   keystrokes mean **the plate is perfect** and re-cutting it wastes an hour on
   steel that was already right.

   > **Localisation is what makes this the operator's job rather than an
   > unanswerable question.** `pos 29 read v corrected to d` is a claim they can
   > settle in seconds against the plate itself: **if position 29 on the steel
   > reads `d`, they mistyped; if it reads `v`, they miscut.** One glance at one
   > character, and the ambiguity is gone. That is why the report prints what
   > each symbol **was** alongside what it was corrected **to** — the
   > before-value is the entire diagnostic, and a report giving only counts and
   > positions would leave the operator with nothing to compare.
   >
   > So `mt` supplies the fact and declines the inference. It cannot see the
   > steel; the operator is holding it.

   **Two mechanisms were considered at this step and BOTH are refused**,
   recorded so a later reader knows they were weighed rather than missed:

   | rejected | why |
   | --- | --- |
   | **a "type it twice and compare" flag** | two transcriptions agreeing is real evidence, but it doubles the most tedious step in the whole journey to answer a question one glance at the plate answers. `mt` would be buying with the operator's hour what localisation gives away |
   | **diffing against `encode`'s original output** | it would localise perfectly — and it requires keeping the strings **in a file**, which is a bearer instrument (§7) that §8's `0600` refusal already treats as hazardous. **The reason to engrave is that the file goes away.** A verify path that depends on retaining it inverts the point of the artifact |

   > **Neither refusal costs the operator anything they cannot get another way**,
   > which is the test §0a's journey rule sets: a divergence earns a change only
   > when the wrong outcome is worse than telling the user nothing. Here the
   > operator is told a great deal — every position, every before-value — and
   > what remains is a comparison only they can perform.

   **`verify` still returns OK** — the transaction *is* recoverable today, and
   inventing a refusal would overrule the operator on their own plate. What
   changes is that the margin is **stated**, so re-cutting one string is a
   decision they can make rather than one they never knew was available.

   **WHEN EVERY CHECKSUM HOLDS AND THE TRANSACTION STILL DOES NOT RE-DERIVE.**
   Journey C, step 3. The spec required this check from the start and stated
   what it proves — and **never said what happens when it fails**, which is the
   silence class this method keeps finding: `verify`'s single most important
   check had no specified failure output at all.

   It is not a contradiction, it is BCH working as designed. `t = 4` means four
   symbol errors are *corrected*; **more than four can land on a different valid
   code word**, and the decoder then "corrects" a chunk into something that
   checksums perfectly and is not what was engraved. Per-chunk verification
   cannot see this.

   > **THE CONTENT ID IDENTIFIES THE TRANSACTION. IT DOES NOT PROVE THE BYTES.**
   > Operator ruling 2026-08-23, disposing of R6 adversarial C-2: *"we just want
   > to id with content id not error correct. Codex32 handles errors."*
   >
   > **An earlier version of this very paragraph — written the same day —
   > claimed the content id was "the only thing that can" catch miscorrection.
   > That is false, and the spec already contained the sentence that falsifies
   > it.** §10.13(c) notes that *"a legacy `scriptSig` is part of the txid
   > preimage while a witness is not"*, uses that fact to settle a different
   > question, and then calls the txid *"a canonical hash of exactly this
   > content"* three sentences later. It is not: the txid is blind to the
   > **entire witness region**, which is where the signatures live and which is
   > the bulk of every artifact §3b measures. Damage there re-derives the
   > expected id and passes — not improbably, but **always**.
   >
   > **The design is unchanged; the CLAIM is withdrawn.** The error layer is
   > BCH, per chunk, `t = 4`, plus §1.1e's length check for the shifts BCH
   > cannot see. The content id's job is to answer *"do these chunks belong to
   > the transaction they claim to?"* — which the txid answers regardless of
   > witness coverage, because a set id that matched a different transaction
   > would need a 20-bit collision. **Calling it "the funds-load-bearing
   > invariant" borrowed `md-codec`'s phrase for a check that does less here**,
   > and `mt` should not claim a proof it does not perform (§0's posture, and
   > the same principle as the offline report's read-versus-verified split).
   >
   > **What the operator is owed instead is the limit, stated plainly** — which
   > is what the report below does.

       mt verify: FAILED — 14 chunks, set 0x0e17e, every checksum holds,
                  but the transaction does not re-derive its id.

         These chunks do not add up to the transaction they name. The
         likeliest cause is MIS-CORRECTION: a chunk took more than 4 damaged
         symbols, and BCH repaired it into a valid string that is not what
         you engraved. A chunk cannot detect this about itself.

         NOTE: this check identifies the transaction. It does NOT prove
         every byte. Damage inside the witness data (the signatures --
         most of the payload) does not change the txid, so mt can pass
         this check on bytes that will not broadcast. Error correction is
         BCH's job, per string, up to 4 characters.

         Most likely first — re-type these from the steel, in this order:
           chunk  7   4 of 4 symbols corrected   <-- most suspect
           chunk 11   2 of 4
           chunk  2   1 of 4
         The other 11 chunks needed no correction and are almost certainly
         right.

   > **The margin report is already the suspect list, and that is the whole
   > design here.** Miscorrection risk rises with the number of corrections
   > applied: a chunk that needed none is almost certainly intact, and the one
   > that spent its entire budget is the one most likely to have spent more than
   > it had. So `verify` does not need a new mechanism to localise this failure
   > — it needs to **print the counts it already computed, in descending
   > order**, and say what they mean. An operator retyping three strings instead
   > of fourteen is the difference between a five-minute fix and abandoning the
   > plate.
   >
   > **Ordering is the entire value.** *"Something is wrong somewhere in 1,242
   > characters"* is a report that leaves the operator with a pile of steel and
   > nowhere to start; the same failure with a ranked suspect list is a
   > half-hour of work. Neither costs `mt` anything it did not already know.
   >
   > **The rarer cause is named too, and not guessed at:** a chunk carried in
   > from a *different* transaction whose 20-bit `chunk_set_id` collides (§3b)
   > would also pass every per-chunk check. `mt` cannot distinguish the two
   > causes and says so rather than asserting miscorrection — but the operator's
   > action is identical, so the report leads with the likely cause and the
   > remedy that covers both.

   **It never asks a node.** A predicate whose answer changes between runs is not
   a predicate, and keeping `verify` offline means it runs on an air-gapped
   machine — which is this constellation's posture. Chain questions live in
   `inspect` (§6a), where they are reported as observations rather than folded
   into a verdict.

   **Optionally, `--transaction <psbt|hex>`** — the sibling round-trip. `mt`'s
   form is unusually strong: because the content id **is** the txid (§10.13 c),
   `mt` compares hashes rather than structures. `md verify` can only re-encode
   and diff.

   > **It compares the FULL 32-byte txid, not the 20-bit set id, and an earlier
   > version of this paragraph implied otherwise while claiming to "prove
   > identity" — R6 adversarial I-1.** Comparing against the set id is a
   > **20-bit** check: 1 in 1,048,576 by accident, and **under a second to
   > construct deliberately** — 2^20 double-SHA-256 operations. `mt verify`
   > would then report that the plate holds a transaction it does not hold,
   > using the words *"prove identity"*. Nothing forces the narrow compare:
   > `mt` is holding the whole reassembled transaction, so the full txid is
   > free.
   >
   > **A supplied PSBT is compared against its EXTRACTED transaction**, per
   > §10.13(c) — the same resolution that section already made, for the same
   > reason: a PSBT holds two transactions whose txids differ for every legacy
   > and `sh(wsh(…))` input, so leaving the basis unstated lets `--transaction`
   > report a **mismatch on the correct transaction**.

   **`inspect` reports what is IN the artifact**: chunk count and indices, the
   set id, and the decoded transaction's own facts — outputs, fee, locktime,
   per-input value provenance, and **plate liveness** (below).

   **`inspect` consults the local node automatically when one is reachable.**
   Operator ruling 2026-08-23, matching §6a's *"the operator is asked for
   nothing"*. This is what lets `inspect` produce its full report **from an
   `mt1` string alone** — the decoded transaction carries its inputs' outpoints
   but not their values, so without a node the fee and provenance rows are
   simply unavailable. With one, `gettxout` supplies both.

   > That repairs §1's claim rather than weakening it: *"the operator and the
   > 2040 recoverer see the same output"* holds whenever the recoverer has a
   > node, and when they do not, `inspect` **names the rows it could not
   > produce** exactly as §6a enumerates its skipped checks. Third use of the
   > same pattern — the node rescues a raw-transaction payload (§8.2e), the node
   > answers unspentness (§8.5), the node completes this report.

   **PLATE LIVENESS is its own row, and it has FIVE states, not two.** Operator
   ruling 2026-08-23: *"a transaction may be invalid because its input has been
   spent, which is different than its input hasn't been broadcast yet."* Those
   are opposite situations for a recoverer and `gettxout` alone conflates them —
   it returns a bare `null` for both.

   **BEFORE CLASSIFYING ANY INPUT, `mt` ASKS WHETHER THIS TRANSACTION ITSELF
   ALREADY CONFIRMED** — `getrawtransaction <our txid> true`. If it did, the
   status is **SPENT — ALREADY CONFIRMED**, naming the block, and no input is
   classified at all.

   > **Without that first question the table reports the SUCCESS case as the
   > theft case — R6 adversarial I-8.** A plate broadcast in 2029 and confirmed
   > has every input spent (by itself) and every parent confirmed, so
   > `gettxout` returns `null` and `getrawtransaction` finds the parents: the
   > DEAD row fires and prints *"the input was spent by someone else. The plate
   > is scrap."* **Someone else did not take the money — the operator's own
   > payment went through.** The recoverer is told a theft occurred, on the
   > happiest path this artifact has.
   >
   > It is cheap because `mt` already holds the txid (§1.1's `TX` row), and it
   > is checked **first** because every other row is a guess about *why* the
   > inputs are gone. Knowing they were spent by *this* transaction answers
   > that question exactly, and the remaining states only make sense once it
   > has been ruled out.

   | state | how `mt` knows | what the recoverer does |
   | --- | --- | --- |
   | **LIVE** | `gettxout` returns a value | the input is unspent **in the UTXO set**. A conflicting spend may already sit in a mempool this node did not consult — see below |
   | **DEAD** | `null`, **and** the parent is **CONFIRMED** (`getrawtransaction <parent> true` returns `confirmations ≥ 1`) | the input was spent by someone else. **The plate is scrap** |
   | **PENDING** | `null`, and the parent is **not found — or is found only in the mempool** | the parent has not confirmed. **The plate may still become live** — find out what happened to the parent |
   | **UNKNOWN** | `null`, and no `-txindex` | `mt` cannot distinguish DEAD from PENDING and says so |

   **The parent lookup needs `-txindex`**, which most nodes do not run:
   `getrawtransaction` *"only returns a transaction if it is in the mempool. If
   `-txindex` is enabled"* it resolves any confirmed transaction. So `mt` uses
   the index when it is there and **reports UNKNOWN rather than guessing** when
   it is not — never printing DEAD on evidence that cannot distinguish it from
   PENDING.

   > **LIVE MEANS "UNSPENT IN THE UTXO SET", NOT "UNSPENT" — R6 adversarial
   > I-3.** `include_mempool` is `false` by ruling (§6a), so `gettxout` answers
   > from the UTXO set only. If someone has already broadcast a conflicting
   > spend that is sitting unconfirmed, `gettxout` still returns a value, LIVE
   > fires, and the old action column said **"broadcast it"** — sending a
   > recoverer into a race `mt` has told them they already won.
   >
   > **§6a knew the mechanism and the table did not inherit it:** *"a
   > mempool-spent input reads as unspent, which is the opposite of the caution
   > this section argues for."* That was recorded as an encode-time limitation,
   > and the liveness table, added later, carried no trace of it — the same
   > shape as the encode-shaped offline warning.
   >
   > **Qualified rather than re-queried**, on the operator's standing ruling
   > that `mt` guarantees nothing and names what it could not check. Switching
   > the liveness report to `include_mempool = true` was the alternative and is
   > rejected here: it would give one RPC two behaviours depending on caller,
   > and §8.5's refusal genuinely wants `false` for the drawer-years reason
   > §6a gives.
   >
   > **Telling a recoverer their plate is scrap when it is merely early is the
   > worst error available here**, because it is the one that gets a live plate
   > thrown away.
   >
   > **AND THE RULE AS FIRST WRITTEN PRODUCED EXACTLY THAT — R6 adversarial
   > C-3.** DEAD required only that `getrawtransaction` **find** the parent, and
   > the spec quotes the falsifying sentence three lines below the table:
   > `getrawtransaction` *"only returns a transaction if it is in the
   > mempool"*. So for a parent sitting **unconfirmed in the mempool** — a CPFP
   > chain, unconfirmed change, or simply a congested hour — `gettxout` returns
   > `null` (`include_mempool` is `false` by ruling) **and** the parent is
   > found. Verdict: DEAD, *"the plate is scrap"*. The truth is the PENDING
   > row's own words: the plate is perfectly good and becomes spendable the
   > moment the parent confirms.
   >
   > **The document contained its own refutation, in the same section, and named
   > the resulting error the worst one available.** What it lacked was the word
   > *confirmed*. `found` and `confirmed` are not the same predicate, and only
   > one of them means someone else took the money.

   **`inspect` OWNS the report; `encode` CALLS it.** Operator ruling
   2026-08-23. `encode` does not compose its own version of §10.10's report — it
   invokes `inspect` on what it just produced and appends the rows only it can
   know: **how many strings to cut, and how many characters in total.**
   **It counts strings and characters, never plates** — `mt` cannot see how the
   strings are laid onto steel, and `md inspect` cannot say how many plates an
   `md1` string takes either. That is not a codec's business.

   > **The point of the ownership rule is that the two CANNOT DRIFT.** If
   > `encode` composed its own report, the operator's pre-engraving view and the
   > recoverer's post-hoc view would be two implementations of the same thing,
   > free to disagree — and this artifact has already produced that defect twice
   > (§7's mitigations naming legend fields §5 had deleted; §11 asserting a chunk
   > rule §3b had retracted). With `inspect` as the single owner, **the operator
   > and the 2040 recoverer are looking at the same output**, and `inspect` is
   > independently testable in a way an inline report inside `encode` would not
   > be.

   #### The report, stated once — three callers, one layout

   **This block is normative and it is the only place the layout appears.** It
   is written out because the report had acquired **three callers** — a
   pre-engraving operator, a recoverer with a node, a recoverer without — and
   its rows had only ever been specified *obliquely*, a clause at a time, in
   four different sections. That is the precondition for exactly the drift the
   ownership rule above exists to prevent: a single owner in the code means
   nothing while the specification of what it owns is scattered.

       mt1 SET   0x0e17e    14 strings, 1..14 all present
       TX        9a3f21c0d4e5b6a7f8091a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f
       OUT       1 output
                   bc1p8rrz...s6n0vcl        0.05000000 BTC
       FEE       0.00012000 BTC   (12 sat/vB over 1000 vB)
       LOCKTIME  block 1383520, ~SUMMER 2034   current height 1402887
       INPUTS    1 input
                   9a3f21c0:0   0.05012000 BTC   from node       LIVE
       STATUS    LIVE — every input unspent in the UTXO set (mempool not
                 consulted; a conflicting spend may already be in flight)

   | row | present when | source |
   | --- | --- | --- |
   | `mt1 SET` | the caller had strings — `inspect` and `decode` | the chunk headers (§10.13 a2) |

   > **`verify` WAS IN THAT LIST AND SHOULD NOT HAVE BEEN — F-240.** §1.1's own
   > worked `verify` output, above, is a single `OK` line plus the margin report:
   > no report rows at all. And §1.1 rules `verify` **structural only**, never
   > consulting a node — which is exactly what lets it run on an air-gapped
   > machine. A table naming it as a report caller contradicts both, and the
   > table is what an implementer reads first.
   >
   > `decode` and `inspect` are the report's two callers. `verify` prints its
   > verdict, its duplicate and unreadable notices, and its margin report.
   | `TX` | **always** | the **txid** — double-SHA-256 of the decoded transaction **with marker, flag and witnesses stripped**. Needs no node and no network. **Not** a hash of the engraved bytes; see the note below |
   | `OUT` | **always** | the transaction itself |
   | `FEE` | a node is reachable, **or** the input was a PSBT carrying values | inputs minus outputs — the transaction alone carries no input values (§6a). **Carries the WEAKEST provenance of any input, inline**: `(CLAIMED — no input value verified)` when any input's value is neither chain-fetched nor txid-bound |
   | `LOCKTIME` | **always**; the `current height` clause needs a node | `nLockTime`, plus §8.4's threshold rule and the season projection. **Wording is §8.4's five normative spellings, by reference — this row may not invent a sixth** |
   | `INPUTS` | **always**; the value and provenance columns need a node | outpoints from the transaction, values from `gettxout` |
   | `STATUS` | **always** — `UNKNOWN` with no node | the liveness table above: `SPENT — ALREADY CONFIRMED` first, then LIVE / DEAD / PENDING / UNKNOWN |

   > **THIS BLOCK PRINTED A VERDICT §8.4 FORBIDS — R6 adversarial I-4, and it
   > is the same drift as I-8 in the opposite direction.** The `LOCKTIME` row
   > ended `— PASSED`, which is not among §8.4's five permitted spellings, and
   > §8.4 exists to establish that **`mt` cannot make a claim about
   > spendability at all**: a BIP-68 relative timelock lives in `OP_CSV` inside
   > the witness script, a relative-locked spend carries `nLockTime = 0`, and
   > reading it means evaluating the sending wallet's script — out of scope by
   > ruling.
   >
   > **This project's own RCW fixture has exactly such a leaf** (`OP_CSV`,
   > 32,768 blocks, ~7 months). A transaction spending it with `nLockTime` at a
   > height already reached would print `— PASSED` and `LIVE`, and be rejected
   > as non-final for months. §8.4 calls that *"false reassurance … the worst
   > failure available here"*, closed it in its own text — **and §1.1 reopened
   > it by printing a verdict in an example.**
   >
   > `— PASSED` is deleted, and the row is bound to §8.4 by reference so the
   > two cannot drift again. **A section that declares itself normative can
   > overrule another section by accident**, which is an argument for binding
   > by reference rather than restating.

   **Three rules govern every row, and they are what make the report honest:**

   1. **A row is never omitted for being unanswerable — it reads `UNKNOWN`.**
      Omission and ignorance look identical on a terminal, and the reader cannot
      tell a row that was skipped from one that never existed. §6a's warning
      then enumerates every `UNKNOWN` and names both ways to resolve them.
   2. **Read and verified are visually distinct, and there are THREE classes,
      not two.** `TX`, `OUT` and `LOCKTIME` come off the plate. `STATUS` and
      chain-fetched values come off the chain. **Between them sits
      operator-asserted or PSBT-claimed data, which nothing checked** — §10.10
      already enumerates all three (*"chain-fetched (§6a), txid-bound (§8.2d),
      or operator-asserted (§8.2c)"*) and this rule collapsed them to two.

      > **The collapse put an unverified number in the verified column — R6
      > adversarial I-5.** Air-gapped `mt encode`, the constellation's own
      > posture: a PSBT carries `witness_utxo` for a segwit input claiming
      > 1.0 BTC. No node, so §6a's comparison does not run. Not legacy, so
      > §8.2d's txid binding does not apply. §8.2c's warning fires *"when, and
      > only when, the value is bound by nothing"* — and the spec treats a
      > segwit amount as bound **by the signature**. But **§8.2's removal means
      > `mt` verifies no signature**, so that binding is asserted and never
      > computed. This is precisely the defect §8.2d was created to close for
      > legacy inputs, reappearing on the segwit side.
      >
      > The number the operator uses to decide whether to cut the transaction
      > at all was therefore printed as chain-verified with no warning
      > anywhere.
      > **Honest bound, stated rather than hidden:** a wrong `witness_utxo`
      > also invalidates the signature, so the transaction cannot confirm —
      > §7's accepted hazard. **The wrong number stands regardless**, and it is
      > the number that drives the decision.
   3. **`encode` appends, never edits.** Its two extra rows go **below**
      `STATUS`, so the operator's view is the recoverer's view plus a suffix:

          CUT       14 strings, 1,242 characters
          PREFIX    all 14 strings begin mt1qzrf8xk2 — strings sharing that
                    prefix belong together

      Anything `encode` needs to *change* about a row is a defect in the row,
      fixable in one place.

      > **The `PREFIX` row was missing here until R6 fold-propagation I-8, and
      > the omission is worth more than the fix.** §0a and §10.10 both rule that
      > `encode` prints the shared prefix, §10.10 makes it a ruled row with its
      > own justification — and this block, which declares itself *"the only
      > place the layout appears"* and ends *"no caller reorders, reformats, or
      > drops a row"*, dropped it. It also said **"two extra rows"** while
      > showing **one**, so its own sentence contradicted its own example.
      >
      > **The section whose entire purpose is that two views CANNOT DRIFT
      > drifted from a third view within hours of being written.** Declaring a
      > single source of truth does not create one; only the content does.

   > **Per caller, the only differences are the stream and the suffix**, which
   > is the whole point of one owner: `inspect` prints it on **stdout** (it is
   > the artifact); `decode` prints it on **stderr** beside the hex (§1.1a);
   > `encode` prints it on **stderr** before the strings (§0a), with the `CUT`
   > row appended. **No caller reorders, reformats, or drops a row.**

1a. **`mt decode` reads `mt` output and emits BROADCASTABLE HEX, and ships in
   v0.1.** Operator ruling
   2026-08-23: *"we need a decode to read mt output."* It takes `mt1` strings —
   from a file, from stdin, typed or pasted, in any order — and reassembles the
   transaction.

   **What it must do, and each of these is a property the format already
   provides rather than new machinery:**

   | step | what makes it possible |
   | --- | --- |
   | accept chunks in any order | `index` in every header (§10.13 a2) |
   | know when the set is complete | `count` in every header |
   | reject chunks from a different transaction | the 20-bit `chunk_set_id` |
   | correct a miscut character | BCH, `t = 4` per chunk (§3a) |
   | **prove the result is the right transaction** | re-derive the content id from the decoded transaction and compare (§10.13 c) |

   That last row is the one that matters. It is the *"funds-load-bearing
   invariant"* `md-codec`'s own source names, and it is what turns `decode` from
   a convenience into the check that the engraving round-trips at all.

   **Its output is raw transaction HEX on stdout — not a PSBT, not JSON, not a
   pretty-print.** Operator ruling 2026-08-23, settled by checking what the
   ecosystem's broadcast paths accept:

   | endpoint | accepts |
   | --- | --- |
   | `bitcoin-cli sendrawtransaction` | *"The **hex string** of the raw transaction"* |
   | Esplora `POST /tx` | *"The transaction should be provided as **hex** in the request body"* |
   | Esplora `POST /txs/package` | a JSON array of **hex** strings |

   **Hex is the only format that reaches all of them without conversion**, and
   the recoverer's last step is always a broadcast. So `decode` hands them
   exactly what the next command wants:

       mt decode < plates.txt > tx.hex \
         && bitcoin-cli sendrawtransaction "$(cat tx.hex)"

   This closes the pipe `mt` sits in the middle of: **hex or PSBT in
   (§8.2e), `mt1` strings onto steel, hex back out.** Everything human goes to
   stderr at both ends, so the pipe stays clean.

   **`decode` WRITES NOTHING TO STDOUT UNLESS EVERY CHECK IN THE TABLE ABOVE
   PASSES, and exits non-zero otherwise.** Stated normatively because it was
   not stated at all — R6 adversarial I-2.

   > **The gap shipped a documented path that broadcasts a transaction failing
   > `mt`'s own integrity check.** `decode`'s required steps end with *"prove
   > the result is the right transaction"*, and nothing said what happens when
   > that fails. An implementer could reasonably print the hex with a warning
   > on stderr — consistent with §8.2e's *"`mt` never refuses the bytes"*. Then
   > the spec's own flagship one-liner ran it through `xargs`, **which consumes
   > stdout only and is blind to both stderr and the exit code**.
   >
   > That contradicted, in the same section, the justification given for
   > `decode` printing its report at all: *"no path through this tool
   > broadcasts a transaction the operator was never shown."* The tool shipped
   > exactly such a path, in a copy-pasteable line.
   >
   > **So the one-liner changed too**, above: `> tx.hex && …` respects the exit
   > code where `| xargs` cannot. **An example command is specification** —
   > people run what is printed, and a pipeline that cannot observe failure
   > teaches a habit no amount of normative prose undoes.
   >
   > **This does not conflict with §8.2e.** That ruling is about `mt` never
   > refusing to *read* bytes on the way IN; this is about what `mt` vouches
   > for on the way OUT. Reading anything and emitting only what verifies are
   > the same posture, not opposite ones.

   **`decode` PRINTS THE INSPECTION SUMMARY ON `stderr`, and does not stay
   silent.** Operator ruling 2026-08-23, from walking Journey B. The reasoning
   is a divergence, not a preference: **`decode` is the verb a recoverer reaches
   for first**, because it is the obvious one. `inspect` is the verb that was
   *designed* for them, and they have no way to know that.

   A silent `decode` therefore hands a stranger sixty kilobytes of hex — a
   bearer instrument, in the single most broadcastable form that exists —
   **before anything has told them what it does**. The next command they type is
   plausibly `sendrawtransaction`, and the first thing they learn about the
   destination, the amount and the locktime is whatever the chain does with it.

   So `decode` emits **§1.1's `inspect` report on `stderr`** while the hex
   goes to stdout. This costs the pipe nothing — stdout is byte-identical either
   way, because §0a's boundary is what makes the summary free — and combined
   with §1.1a's rule that **stdout stays empty unless every check passes**, it
   means no path through this tool broadcasts a transaction the operator was
   never shown. `--quiet` suppresses the report for scripted use; the default is
   loud, and `--quiet` does **not** relax the stdout rule.

   > **`inspect` remains the verb, and `decode` remains a pipe fitting.** This
   > is not a merge. `inspect` is the one that reports *without* handing over
   > the weapon, and it is what §5's `FORMAT:` tag should point a recoverer
   > toward (§10.21). `decode` merely stops being silent about what it just
   > reconstructed.

   **`decode` is also how `encode` gets tested.** A format whose encoder has no
   decoder can only be verified against itself; with both, every artifact in §3b
   becomes a round-trip vector.

1b. **One ENGRAVING form in v0.1.** `mt qr` is deferred to its own cycle
   (§0a) because QR conversion is a cross-format concern `md1` and `mk1` share. `mt qr` is deferred to its own cycle
   (§0a) because QR conversion is a cross-format concern `md1` and `mk1` share.
   The two-verb design below is retained as the eventual shape.
1c. ~~Two engraving verbs, `qr` and `string`.~~ **Superseded**: the engraving
   split is `encode` now and `qr` later (§0a); the verb set is `md`'s. Signed, finalized
   transactions only. Transaction construction and PSBT presentation are wallet
   functions and are out of scope (§9). **This overrules the previous draft's
   produce/present/engrave triple**, which split on *stage of the transaction*;
   these two split on *how the steel is cut*.
1d. **`mt encode` exists so a transaction can be HAND engraved, with fault
   tolerance.** Operator ruling 2026-08-23: *"For some shorter transactions,
   users will want codex32 style fault tolerant hand engraving."* It gives `mt`
   the human-readable, error-correcting property the rest of the constellation
   is built on, and makes it usable by someone with no SeedHammer.

   > **The original wording said "without it, the only route onto steel is a
   > machine" — and §0a inverted that.** With `mt qr` deferred, `mt encode` is
   > not the alternative to the machine, it is **the only route at all**, and
   > everything up to the 32,768-chunk ceiling goes through it. The ruling's word
   > *"shorter"* described a verb that had a sibling; it now describes the whole
   > tool. Nothing about the format changes — but nobody should read this item
   > as scoping `mt encode` to small transactions, because §8.7b's ceiling is the
   > only bound and it sits above Bitcoin's own relay limit.
1e. **The human text surface: what `mt` suggests engraving, and what it accepts
   back.** Operator rulings 2026-08-23. A string leaves `mt` as text and comes
   back typed by a person, and both ends need rules.

   **Engrave UPPERCASE; accept anything.** `mt` suggests uppercase because it is
   more legible on steel — fewer ascenders and descenders, more distinct
   letterforms under a scratch — and the fork's own keyboard path already emits
   it. **Input is case-insensitive**, because bech32 treats all-upper and
   all-lower as identical and normalising costs nothing. (Mixed case is invalid
   *bech32*; `mt` normalises before that rule bites.)

   **`--elide-prefix` — emit the invariant 8 characters ONCE.** Operator ruling
   2026-08-23: *"the hand engraving user might put many strings on one plate and
   skip repeating the header after a while, keeping the payload characters for
   each string vertically aligned to indicate a dropped header."*

   Per-field alignment (§10.13 a2) makes this expressible: `version +
   chunk_set_id + count` is **exactly 8 characters** and `index` is exactly 3, so
   there is a clean boundary to cut at.

       mt encode --elide-prefix

       mt1qzrf8xk2 q9d7b4h2... <- string 1, FULL
                   qp4m2e7k9... <- strings 2..n: index + payload only
                   qzr8xk2vt...

   **The FIRST string is always emitted in full and the rest are elided.** That
   makes the output **self-describing**: `mt decode` and `mt verify` take the
   prefix from string 1 and need no extra flag, so the round trip closes with
   one option on one side.

   **Detection is unambiguous and needs no flag on input:** a line beginning
   `mt1` is a full string; anything else is elided and is prefixed with the set's
   invariant 8 characters before anything else happens. Mixed input is therefore
   legal — which matters, because an operator who elides *"after a while"* will
   produce exactly that.

   > **AN ELIDED STRING IS NOT A VALID `mt1` STRING, and this is the property
   > that governs the whole feature.** The BCH checksum is computed over the
   > **full** data including the invariant fields, so an elided line will not
   > verify on its own. It is a **display form**, never a wire form: `mt`
   > restores the prefix and only then parses, checksums or corrects. Nothing
   > downstream of that restoration knows elision happened.
   >
   > **If every line is elided — refuse, and say what is missing.** Without one
   > full string there is no prefix to restore, and guessing is not `mt`'s to
   > do silently. The refusal names the shape of what is needed: the **8
   > characters following `mt1` on any intact string of the same set**.
   >
   > **The prefix is not lost with the string that carried it.** `version` is a
   > known constant and `count` is guessable from the number of strings held, so
   > only `chunk_set_id` is genuinely unknown — **20 bits, 1,048,576
   > candidates**, one BCH verification each. And the content id settles it
   > independently: the set id **is** the top 20 bits of the reassembled
   > transaction's txid (§10.13 c), so a wrong guess cannot survive reassembly.
   > Recorded because an operator deciding whether to elide deserves to know the
   > failure is recoverable rather than terminal; **`mt` v0.1 does not implement
   > the search** and the refusal above says so plainly.

   **What it costs and saves, measured — and the figure was WRONG until S0's
   generator produced real strings.** Eliding drops **11 characters** per elided
   string, not 8: the 8-symbol invariant prefix **and the `mt1` prefix**, since
   an elided line carries index + payload only. So the saving is
   `11 × (n − 1)`:

   | artifact | strings | unelided | elided | saved |
   | --- | --- | --- | --- | --- |
   | 162 B | 5 | 395 | 351 | 44 |
   | 405 B | 11 | 953 | 843 | 110 |
   | 535 B | 14 | 1,242 | 1,099 | **143** |
   | 742 B | 19 | 1,701 | 1,503 | 198 |
   | 2,498 B | 63 | 5,698 | 5,016 | 682 |

   Against §10.13(a2)'s +1 symbol per chunk, the 14-string case is **129
   characters cheaper than the 49-bit layout that could not elide at all**.

   > **This is the first defect found by IMPLEMENTATION rather than review, and
   > it arrived from S0 — before `mt-codec` exists.** Eleven review rounds read
   > `8 × (n − 1)` and none caught it, because the arithmetic is only wrong once
   > you ask what an elided line *literally contains*. The generator's own
   > output settles it: the `even` vector's strings go 522 → 467, a saving of
   > **55 = 11 × 5**, and `uneven` goes 674 → 597, **77 = 11 × 7**.
   >
   > That is the whole argument for an independently-derived vector landing
   > before the crate: a spec figure nobody could falsify by reading became
   > falsifiable the moment something generated real strings.

   **Spaces are stripped on input, and offered on output.** `mt encode` takes an
   optional grouping — every N characters, space-separated — **for hand
   engraving only**, since a person cutting ~90 characters needs somewhere to
   keep their place.

   **Whatever grouping the operator chose, `mt decode` and `mt verify` SPLIT
   FIRST, then strip.** Input is split into candidate strings on **any run of
   whitespace containing a newline**; spaces and tabs *within* a line are
   grouping separators and are stripped. **A line containing more than one
   `mt1`/`MT1` prefix is split at each prefix.**

   > **An earlier version said only "strip whitespace before doing anything
   > else", which is unbuildable — R6 implementability I-2.** Followed
   > literally, fourteen 90-character strings become **one 1,242-character
   > blob** and the tool cannot parse its own output. The rule could not mean
   > what it said, and the spec never said what it did mean, so three readings
   > were all defensible: split on newlines then strip within (refuses the
   > single-line paste an operator gets by copying a terminal), scan for `mt1`
   > and slice between (accepts it), or strip everything and re-split by
   > counting characters (needs the length §1.1e cannot compute before
   > assembly).
   >
   > **The recovery path is where this bites**, and a refusal there is answered
   > by an operator retyping 1,242 characters off steel. The prefix-split
   > clause is what makes the pasted-blob case work rather than fail.

   **Every string in a set has a KNOWN length, checked before decoding**, because
   it catches the one damage class BCH cannot.

   > **There is no universal constant, and an earlier version of this section
   > said "exactly 90 characters".** Wrong for most transactions. `md-codec`
   > balances — `bytes_per_chunk = ceil(len / count)` — which is usually **less**
   > than the 40-byte ceiling the chunk *count* derives from:
   >
   > | tx bytes | chunks | bytes/chunk | full string | last string |
   > | --- | --- | --- | --- | --- |
   > | 162 | 5 | 33 | **80** | 75 |
   > | 405 | 11 | 37 | **87** | 83 |
   > | 535 | 14 | 39 | **90** | 72 |
   > | 742 | 19 | 40 | **91** | 63 |
   > | 560 | 14 | 40 | **91** | **91** |
   > | 2,498 | 63 | 40 | **91** | 56 |
   >
   > 91 occurs only when the arithmetic lands on a 40-byte chunk. **This is the
   > third time in this document a LIMIT has been read as a RULE** — after
   > "363 bits per chunk" and "a flat 40 payload bytes per chunk" (§3b).

   **`mt` computes both lengths and states them.** Every string but the last is
   one length; the last is the remainder, equal to the others only when the
   payload divides evenly.

   **AT DECODE TIME THE EXPECTED LENGTH COMES FROM THE STRINGS THEMSELVES —
   the MODAL length across the set.** Every chunk with `index < count − 1`
   carries the same payload length, so the most common string length in the set
   *is* the expected one, and any string differing from it is the suspect. The
   **final** chunk's expected length cannot be checked until the set is
   complete, and is not checked before then.

   > **Stated because the obvious derivation is CIRCULAR — R6 implementability
   > I-3.** Per-chunk length follows from `bytes_per_chunk`, which follows from
   > the total payload length, which is not known until every chunk is
   > assembled — **which is the thing this check exists to gate.** For `encode`
   > it is trivial (it holds the payload); for `decode` and `verify` it is not,
   > and three implementers diverge: modal length, `count` × an assumed 40-byte
   > chunk (landing in the flat-40 defect), or skipping the check when the set
   > is incomplete — **which is precisely the case §1.1e was written for.**
   >
   > The message says *"a character is MISSING … Re-read the plate"*, which is
   > **an accusation about the operator's steel**. A wrong expected value makes
   > it a false one, and sends someone to re-read a plate that is correct.

       string 7: 89 characters (expected 90) — a character is MISSING, not
                 wrong. BCH repairs substitutions; an omission shifts every
                 symbol after it and cannot be corrected. Re-read the plate.

   **AUTOCORRECT NEVER TOUCHES A STRING THAT ALREADY PARSES.** It is a
   **repair attempted on failure**, not a preprocessing pass. The order is:

       1. split (above), strip whitespace, **normalise to LOWERCASE**
       2. check the length against this set's computed value
       3. TRY THE STRING AS WRITTEN — if it parses and the checksum holds, STOP
       4. only then attempt correction, positionally (below)
       5. re-check, and report the verdict either way

   **Step 3 is not an optimisation, it is a safety rule**: the corrections run in
   **opposite directions** depending on position, so a naive character map
   applied to a *correct* string would rewrite `mt1…` as `mtl…`, destroy the
   separator, and turn a perfectly good transcription into one that cannot parse
   at all. **A repair pass that can damage valid input is worse than no repair
   pass.**

   > **`mt encode` WRITES LOWERCASE, and the direction of "normalise case" was
   > never stated — R6 implementability I-1.** Every case rule in this document
   > governs a **deferred** artifact (the QR payload, the `sysw` record), and
   > *"engrave UPPERCASE"* is advice to a human about steel, not a statement
   > about the byte stream. §0a makes stdout normative and never said which
   > case, while the document's own examples split both ways.
   >
   > It is not style: **stdout is declared the artifact** and gets piped, hashed
   > and diffed, so two implementations emitting different bytes for the same
   > transaction both look correct while nothing compares equal. §3's EPD §6.4
   > argument also turns on lowercase being what is *stored*. Uppercase remains
   > the engraving advice; the two are independent, and normalising is free.
   >
   > **The correction table below is read AFTER normalisation**, which the
   > second half of the same gap made ambiguous: the table is written in mixed
   > case, so an implementer who normalised to uppercase first had a table that
   > matched nothing.

   **POSITIONS IN OUTPUT ARE 1-BASED, COUNTED OVER THE WHITESPACE-STRIPPED
   STRING INCLUDING THE HRP.** Position 1 is `m`, position 3 is the separator,
   position 4 is the first data symbol. **A BCH codeword index `k` is position
   `k + 4`.** Where the operator engraved with grouping, positions do **not**
   count the spaces.

   > **The document used two conventions and neither was stated — R6
   > implementability I-5, measured by column rather than read.** One worked
   > example put the corrected character at 0-based index 12 and called it
   > `pos 12`; another put a caret under 0-based index 11 and called it
   > `position 12`. Same document, same concept, opposite conventions — **and
   > the second example was broken outright**, since index 11 of
   > `mt1qzrf8xk2v` is `v` and index 12 is `.`, neither of them the `b` it
   > claimed to correct. Both are regenerated above from computed offsets.
   >
   > **It matters because §1.1's whole miscut-versus-mistyped design rests on
   > this number being checkable against steel:** *"if position 29 on the steel
   > reads `d`, they mistyped; if it reads `v`, they miscut."* An off-by-one
   > sends the operator to a character matching neither value, where they learn
   > nothing — right after being told this single comparison settles it.
   >
   > **A third offset is invisible and worth naming:** BCH error positions come
   > out of the decoder as indices into `data || checksum`
   > (`crates/md-codec/src/bch_decode.rs:22`), so codeword index 0 is the first
   > *data* symbol. An implementer wiring the corrector's output straight into
   > the report is off by four **and will never notice, because the report is
   > prose.** Hence the mapping is stated as an equation.

   **Correction is POSITIONAL, because `mt1` has a fixed HRP.** In the index
   language this section uses for string structure — 0-based — the separator is
   at index 2, always, and data begins at index 3. **Add 1 for the positions
   reports print:**

   | position | character | correct to | why |
   | --- | --- | --- | --- |
   | 0–1 | must be `mt` | — | the HRP |
   | **2** | `l`, `I`, `i` | **`1`** | this is the **separator**, and `1` is the only legal character here |
   | **3+** | `1`, `i` | **`l`** | `1` is *never* data — it is the separator |
   | 3+ | `o` | `0` | excluded from the charset |
   | 3+ | `b` | `6` | excluded |

   **`1` at index 2 is correct and must never be altered. `1` after index 2 is
   always an error.** Same glyph, opposite verdicts, decided only by where it
   sits — which is exactly why a positionless map is unsafe.

   **Why correcting before decoding costs nothing.** bech32's data charset excludes `1`, `b`, `i` and `o` *precisely*
   because they are confusable, so a typed excluded character is not a wrong
   symbol — **it is not a symbol at all**, and BCH never sees it. Repairing it
   before decoding therefore **costs nothing from the `t = 4` budget**, which
   stays available for genuine substitution errors:

   The prefix case matters most: **every string a person types contains the
   single most confusable glyph in the set**, and `mtl…` or `mtI…` does not
   merely fail its checksum — it has no separator and will not parse at all.

   **Autocorrect announces itself, localises, and states its verdict.**
   Operator ruling: never silently. A silent fix means the operator never learns
   which engraved glyph reads badly, and so never re-cuts it before the next
   scratch lands there.

       string 3: corrected `o` -> `0` at position 41. Checksum now valid.
                 That character reads badly on your plate — consider re-cutting it.

       string 9: corrected `b` -> `6` at position 16. Checksum STILL INVALID.
                 mt1qzrf8xk2v9d7b4...
                                ^ here        <- could not resolve

2. **Its own repository**, `mnemonic-transaction`, with **`mt-cli` and
   `mt-codec`** — matching the constellation's pattern exactly, and not a
   subcommand of `me`. Every normative format has this shape: `descriptor-mnemonic`
   is `md-cli` + `md-codec` for `md1`, `mnemonic-key` is `mk-cli` + `mk-codec`
   for `mk1`, `mnemonic-secret` is `ms-cli` + `ms-codec` for `ms1`. **`mt-cli`
   builds the `mt` binary**, as `md-cli` builds `md`. (An earlier draft said
   "`mt-codec` and an `mt` CLI", which named the binary where the siblings name
   the crate — a rename that is cheap now and annoying after a release.)
   `me` is the one repo with no codec, because it defines no format; that is
   precisely why `mt1` cannot live there. **This overrules the recommendation in
   §Section 1 of the brainstorm**, which argued `mt` had no wire format left to
   define and belonged next to `me bundle`. See §2 for what the codec does in
   fact specify; the objection was answered rather than ignored.
3. **The QR carries the standard form, never a codex32 string** (F-234) —
   **`mt qr` only, deferred (§0a)**; recorded here because it is what §3a's
   one-layer-per-medium principle rejected, and that principle is live.
4. **UR is dropped entirely. Both verbs share the `mt1` chunk header and NOTHING
   ELSE** — each medium carries the error correction native to it (§3a). The QR
   payload is **bech32 uppercase**, the constellation's own alphabet.
   **This overrules the previous draft's `ur:psbt`, which itself overruled
   `ur:bytes`** — three positions in one cycle, and §3 records why each fell.
   The payload remains a fully finalized PSBT. See §3.
5. **Reed-Solomon density is the highest that still minimises engraved area** —
   **`mt qr` only, deferred (§0a)**. Stated in area rather than plates, because
   `mt` cannot see how strings map to steel.
6. **Provenance rides in the engraved legend, not in the wire format.**
7. **`mt` does not offer a locktime CHOICE. It reads the transaction and warns
   if the plate would be immediately broadcastable.** Operator ruling 2026-08-23:
   *"Timelocking happens by user at their wallet software. We do not create
   transactions. We merely read transaction and warn if immediate."* **This
   overrules the previous draft's `--timelocked` / `--immediate` flags**, which
   made `mt` a party to a decision it does not own. See §8.4.
8. **Redundancy is zero. `mt` protects against damage to a plate, not against a
   missing plate.** The operator is free to engrave duplicate copies. **This
   closes §10.6, the previous draft's largest open question.** See §3.

## 2. What `mt-codec` actually specifies

The payload is a Bitcoin transaction, specified by Bitcoin. The container is a
QR, specified by ISO/IEC 18004. **Neither needs `mt`.** What is unspecified —
and what this codec is for — is everything between them:

> **This sentence named a third specification, UR, until 2026-08-23, and §3
> thirty lines below already said *"there is no UR, and no third-party envelope
> of any kind."*** The two were in the document together, in direct
> contradiction, and the drop was ruled a full cycle earlier. **The envelope
> being `mt`'s own is not a footnote to this paragraph — it is half of the
> paragraph's point**, since the fragmentation `mt-codec` specifies is now the
> item that was previously delegated. Found by walking Journey C, not by any
> gate: no superseded-term sweep catches it, because every word in the sentence
> is still current vocabulary and `UR` appears legitimately eleven other times
> in this document as history.

- how one transaction maps onto **one or more** QR symbols, and onto plates;
- how a recoverer reassembles them, and how they know a fragment is missing;
- which (module size, QR version, ECC level, tiling) configuration is chosen for
  a given transaction, **deterministically and with every tie broken**, so two
  encoders agree;
- what is engraved **beside** the symbols, so the plate is self-describing;
- what `mt` refuses to engrave at all.

...and, for `mt encode`, **the string format itself**: the `mt` HRP (rendering as `mt1…`,
where `1` is bech32's separator — §10.13b), the chunk
header, and the BCH checksum that makes hand engraving fault-tolerant (§3b).

> **CORRECTION — the previous draft said the opposite, and it is worth saying
> why it was wrong.** It read: *"It is a plate format rather than a string
> format, which is why it has no bech32 HRP and no BCH checksum."* Adding
> `mt encode` falsifies that sentence outright. `mt-codec` now defines a bech32
> string format with an HRP and a BCH checksum, exactly like `md-codec` and
> `mk-codec`.
>
> This **strengthens decision 2** rather than embarrassing it. The brainstorm's
> objection was that `mt` had no wire format left to define and belonged as an
> `me` subcommand. That objection is now decisively answered: `mt` defines a
> normative string encoding, which is precisely what earns a codec crate of its
> own in this constellation.

## 3. The envelope: none — `mt1` chunks, both verbs

There is **no UR**, and no third-party envelope of any kind. Both verbs
fragment with the **`mt1` chunk header** (§3b), and `mt qr` puts the resulting
chunks into QR symbols directly.

> **CORRECTION, and this is the third envelope position in one cycle. All three
> are recorded because the reasoning matters more than the answer.**
>
> 1. **`ur:bytes`** — the original draft. R0 round 0 killed it: BCR-2020-005
>    states the `bytes` type *"exists only for testing and validation of UR
>    implementations and MUST NOT be used for any other purpose."*
> 2. **`ur:psbt`** — the compliant replacement, on the operator's *"don't go
>    off-label"* ruling. Correct as far as it went: the BCR-2020-006 registry has
>    58 rows and `psbt` is its only transaction-shaped type, so this was the one
>    conformant way to carry a transaction under UR.
> 3. **No UR at all** — operator ruling 2026-08-23: *"I don't think UR wrapper
>    complexity is worth it."*
>
> **What made (3) available was §10.2**, ruled the same day. UR's real defence
> was never conformance — it was that UR is the only fragmentation the Bitcoin
> ecosystem implements, so a recoverer could reassemble engraved symbols with
> off-the-shelf wallet software. Once `mt` ships its own static-scan reader, the
> ecosystem reassembles nothing and that defence is void.
>
> **What it costs, measured** (`RESULTS_ecc_selection_2026-08-22.txt`, at
> 0.60 mm, same finalized-PSBT artifacts): dropping UR **saves a whole plate on
> 3 of 7 artifacts** — `tr` tier 4 goes 2 plates to 1, `tr` tier 1 at 5 inputs
> goes 5 to 4, `wsh` tier 1 at 5 inputs goes 6 to 5 — **and buys one to two ECC
> levels on the other 4.** Under §1.8, which spends slack on damage tolerance,
> UR was spending the exact currency the artifact needs.
>
> **The efficiency numbers, which decide this on their own**
> (`RESULTS_qr_modes_2026-08-22.txt`, gated against published v40 limits):
> raw binary 100%, **base45 97%**, bech32 uppercase 91%, base64 75%, and **UR
> bytewords ~73%** — the same density as plain uppercase hex. UR was paying a
> 27% tax for a wrapper whose benefit §10.2 removed.

**Fragmentation: the `mt1` chunk header, for both verbs.** Operator ruling
2026-08-23. It carries `version`, a 20-bit `chunk_set_id`, `count` and `index` —
n-of-m **plus a set identifier**, so symbols from two different transactions
cannot be combined. That is strictly stronger than UR, which has a payload
checksum but no set identity, and it means **one fragmentation scheme to
specify, test, teach a recoverer, and get wrong only once.**

> **`mt1` WIDENS `count` and `index`, and an earlier version of this section said
> the header was shared "verbatim" with `md-codec`. That was unbuildable.**
> R3 lens 3 found it. `md-codec`'s header packs
> `version(4) + chunked(1) + chunk_set_id(20) + count−1(6) + index(6) = 37 bits`,
> against `mt1`'s
> `version(5) + chunk_set_id(20) + count−1(15) + index(15) = 55 bits`
> (`crates/md-codec/src/chunk.rs`), and `write()` refuses any `count` outside
> `1..=64` with `ChunkCountOutOfRange`. **Six bits caps a set at 64 chunks** —
> while §3b's own table measures the largest `mt qr` artifact at **96**, and
> §3b and §8.7b both **stated at the time** that the 64-chunk ceiling was what
> distinguished `mt encode` from `mt qr`. The ruled encoding could not be
> written by the ruled header. (Both of those sentences are gone: the ceiling is
> now 32,768 for **both** verbs, and what distinguishes them is what a chunk
> *costs*, not how many are permitted.)
>
> **`mt1` uses 15 bits each for `count` and `index`** — a **55-bit** header
> admitting **32,768 chunks = 1,310,720 bytes**, `mt1`'s ceiling for **both**
> verbs. The widths are set by **per-field symbol alignment** (§10.13 a2), not by
> a capacity target: 15 bits is 3 whole characters where 12 would straddle.
>
> **Why 15, and the bound is Bitcoin's rather than ours.** Operator ruling
> 2026-08-23. `MAX_STANDARD_TX_WEIGHT = 400,000` (verified in `bitcoin`
> 0.32.101's `policy.rs`) is 100,000 vbytes, so **a transaction above ~100 KB
> will not relay** and `mt` could never usefully engrave one. That is **2,500
> chunks**; 32,768 covers it with **13.1x** headroom. An 8-bit field gave 10 KB and
> would have refused an ordinary 20-input multisig spend; 14 bits would give
> 640 KB, six times what any node accepts, bought for nothing.
>
> **The cost is ONE CHARACTER per engraved string.** `md-codec` sizes chunks
> against a 320-bit budget that sits *below* codex32's 400-bit capacity, so a
> wider header does not change the chunk count — it consumes slack. Measured: a
> chunk-string goes from **89 to 90 characters** at the 49-bit header this box
> was written under; at the ruled 55-bit header a 40-byte chunk is **91** and the
> 162-byte five-chunk artifact totals **395** characters (four strings of 80
> plus a last of 75 — §10.13 a2's table, recomputed). The *cost per widening* is what
> this box measures, not the current length. Both `41 + 320 = 361` and `49 + 320 = 369` fit the
> 400-bit capacity.
>
> **Sizing this field for hand engraving would have been sizing the wrong
> constraint.** Nobody hand-cuts 2,500 strings whatever the format permits —
> *effort* limits that, not `count`. The header must serve the largest consumer,
> which is the machine path (§0a), and it is the one field that cannot be
> widened after v0.1 without breaking the wire format. That is consistent with §10.13, which already forks the
> codec with its own NUMS constant and HRP rather than reusing `md-codec`'s; the
> fork extends to the field widths. Cost is **4 bits per chunk**: 48 bytes on the
> 96-chunk artifact, which changes no plate count.
>
> **What is shared is `mt1`'s header, identically across both verbs** — not
> `md-codec`'s.
>
> **CORRECTION: an earlier version of this box said `mt encode` "keeps the
> 64-chunk limit because that is a property of the codex32 container". That is
> false, and it was mine.** codex32 limits a **single string** — 80 data symbols
> plus 13 checksum, `BCH(93,80,8)` — and says **nothing** about how many strings
> form a set. The 64 comes entirely from `md-codec` writing `count` into **6
> bits**, which `mt1` no longer shares. **`mt1`'s ceiling is 32,768 chunks for
> both verbs**, and every artifact measured in §3b fits it many times over.
>
> **Why 64 was right for `md1` and wrong for `mt1`, measured.** Encoding this
> repo's pathological wallet with the real `md` binary: the keyless template is
> **4 chunks**, and the keyed form carrying all **11 xpubs is 23 chunks** — about
> a third of 64. `md-codec`'s bound has ~3× headroom over the worst real
> descriptor. The same wallet's five-input **spend** needs **89 chunks**, because
> a transaction carries the witnesses, signatures and script paths a descriptor
> only describes. `mt1` is a different format with different sizing, which is
> why it has its own codec.

    mt encode:  mt1 chunk -> BCH + codex32 text -> engraved as characters
    mt qr:      mt1 chunk -> bytes              -> engraved as a QR symbol
                ^ identical header both ways

**Consequence: §10.13 now gates both verbs, not one.** Whether `md-codec`'s
header and reassembly take a transaction-shaped payload cleanly was already
open; it is now load-bearing for everything `mt` emits.

**What a symbol carries: `mt1` chunks, bech32 UPPERCASE.** Measured
(`RESULTS_ecc_selection_2026-08-22.txt`, `qr_payload_forms`), all four
candidates carrying the same chunk header:

| form | efficiency | worst plate cost | usable in a `sysw` record? |
| --- | --- | --- | --- |
| codex32 string inside the QR | 63–65% | +2 plates | yes |
| bytes + base45 — *rejected, see below* | 85.5–86% | — | **NO** |
| **bytes + bech32 UPPERCASE** | **80.3–80.7%** | **+1 plate** on one artifact | **yes — chosen** |
| bytes, raw binary | 88.4–88.8% | — | no |

> **base45 was chosen on 2026-08-23 and is REVERSED here, because it cannot
> reach the machine.** R2 lens 3 found the collision. **base45's alphabet
> contains SPACE** (index 36, RFC 9285), and EPD §6.4 — the `sysw` record rule —
> is normative and emphatic:
>
> > *"Every record MUST be the canonical, unbroken string — **no interior
> > spaces, no hyphens, no grouping of any kind**."*
>
> **The reason is about engraving, not parsing**, and it is why the rule does not
> bend: records engrave **verbatim**, so *"a record carrying separator characters
> the BCH checksum never covered turns a scratch on the operator's only copy into
> silently-absorbed damage rather than a detected error."* A character outside
> the checksum's coverage is a hole in the guarantee, cut into the only copy.
>
> **EPD §6.4 HAS A SECOND CLAUSE — ALL-LOWERCASE — AND I DENIED IT IN THIS
> SPEC.** R3 lens 3 reported that bech32 uppercase collides with it, citing
> `design/SPEC_encrypted_payload_delivery.md:806-825` by exact line range. In
> commit `52ad001` I **refuted that Critical**, writing that EPD §6.4 carries no
> lowercase clause and that the rule belongs to EPD §6.6's hashing. **That was
> wrong.** I checked `SPEC_systemwide_payloads.md` — a secondary document that
> quotes EPD §6.4 *in part* — found no lowercase clause in the fragment, and
> concluded the clause did not exist, without opening the file the reviewer had
> named. The primary source says:
>
> > *"**All-lowercase.** … without this the same wallet has two spec-legal
> > encodings — and therefore two different EPD §6.6 hashes. … **Pinned here at
> > EPD §6.4, not inside EPD §6.6**, so the engraved artefact and the hash agree by
> > construction."*
>
> It states the proposition I denied, in the terms I denied it, and its last
> sentence pre-empts my exact reasoning. **A partial quote in a secondary
> document is not the clause** — a negative inherits the scope of the search
> that produced it, and mine searched the wrong file.
>
> **The design survives; the justification did not.** The same commit ruled that
> the record stores lowercase and `mt` uppercases only for the QR, which
> satisfies EPD §6.4 as actually written. So bech32 remains correct — but for a
> reason the spec had stated falsely, and a reader would have learned that EPD §6.4
> has no case rule.
>
> **This is the third format to collide with EPD §6.4, and the precedent is
> settled.**
> `FreeText` and `Passphrase` hit it too, and *"the exemption is refused —
> relaxing EPD §6.4 for two classes would weaken the rule for all of them."* They
> were hex-encoded instead, at 2×. Hex-escaping base45 would land at **48.5%**,
> worse than raw binary and worse than the UR this cycle dropped for waste.

**Why bech32 uppercase satisfies all three constraints at once**, which is why it
is the constellation's alphabet rather than a stylistic choice:

| constraint | bech32 uppercase |
| --- | --- |
| **EPD §6.4** — no interior spaces; every character inside the checksum | ✓ 32-character alphabet, no space |
| **EPD §6.4 — ALL-LOWERCASE**, a second clause of the same rule | ✓ **only because the record stores lowercase.** bech32 is case-insensitive by design and uppercase→lowercase is lossless (verified 1:1), so the payload survives the constraint — but the *record* must be written lowercase, not merely be convertible |
| **which case is STORED** — the record and the QR are different artifacts | the `sysw` record stores **lowercase**; `mt` uppercases **only** when encoding the QR symbol, where alphanumeric mode needs it. The uppercase form never reaches a record |
| **QR alphanumeric** — for 11-bits-per-2-characters packing | ✓ when uppercased |

The rejected base45 satisfies only the third; hex satisfies the first two at
twice the cost.
`md1`, `mk1` and codex32 already store lowercase and uppercase for QR, so
`mt qr` and `mt encode` now share one alphabet.

> **Correction to a figure I quoted while recommending this.** The 91% measured
> for bech32 in `RESULTS_qr_modes_2026-08-22.txt` is for a **bare** payload. With
> `mt1` chunk headers added before encoding, the measured figure is **80.4%** —
> the overhead compounds. The plate consequence is one extra plate on RCW `wsh`
> tier 1 at five inputs (5 → 6) and no change on the other four artifacts.

## 3a. The medium-appropriate ECC principle

**Each medium carries exactly one error-correction layer: the one native to it.**
This is the rule that rejected codex32-in-QR, and it generalises.

1. **One layer per medium**, chosen to match **how that medium physically
   fails**:

   | medium | fails as | native correction |
   | --- | --- | --- |
   | hand-engraved string | **per character** — a miscut stroke, a wrong glyph, one scratched letter | **BCH** over 5-bit symbols, `t = 4` per chunk |
   | machine-engraved QR | **per region** — a scratch across modules, corrosion, a dent | **Reed-Solomon** + QR codeword interleaving, which spreads a local blot across many RS blocks |

2. **Never stack them, because a redundant layer is paid for in the same
   currency as the native one: plate area.** §4's objective spends every leftover
   byte on ECC, so carrying BCH inside a QR does not add protection *on top of*
   Reed-Solomon — it buys BCH parity **with area that would otherwise have bought
   RS parity**, at a worse rate. Measured above: 64% against 88.8%, up to two
   extra plates, and a lower ECC level everywhere else. **Stacking made the
   artifact strictly less damage-tolerant**, which is the opposite of what a
   second checksum intuitively promises.

3. **What legitimately crosses both media is FRAMING, not correction.** The
   chunk header — version, `chunk_set_id`, `count`, `index` — is about identity
   and assembly, so it is shared verbatim. Damage is medium-specific; identity is
   not.

So the split is clean, in the operator's own words: **QR is for machine
engraving, codex32 is for hand engraving.**

    mt encode:  chunk header + payload -> BCH + codex32 -> engraved characters
    mt qr:      chunk header + payload -> bech32U -> QR (Reed-Solomon) -> modules
                ^ identical header, medium-appropriate correction
                ^ and PER-CHUNK conversion in both (below)

**The base32 conversion is PER CHUNK, never over the concatenated stream.**
Operator ruling 2026-08-23. `mt encode` has no choice here — codex32 is
per-chunk by construction, each chunk becoming a complete string with its own
HRP and checksum — so this rules the only verb where the question arises,
`mt qr`, **to follow the convention `mt encode` already has.**

**Why, and it is not the size.** Measured on the 3,809 B artifact: per-chunk is
**7,054 characters**, whole-stream **7,016** — a 0.5% saving for whole-stream.
What per-chunk buys instead:

- **One chunking rule across both verbs.** A recoverer's chunk 7 is byte-
  identical in either medium before the medium-specific encoding, which is what
  makes §3a's "identical header" claim true at the byte level rather than only
  at the field level.
- **Chunk independence, which is the point of chunking.** Whole-stream couples
  every chunk's characters to every byte before it, so a damaged chunk shifts
  its neighbours' alignment.

> **The failure mode if two implementers split here is silent and
> misdiagnosed** — R5 readiness computed it. The two strings **share no
> character after position ~74**, yet the first chunk still parses with a valid
> header. The corruption surfaces only at the content-id compare, which reports
> *"this is a different transaction"* — pointing the recoverer at the wrong
> plate rather than at the wrong software.

## 3b. The string form: `mt1`, for hand engraving

**`mt encode` emits a chunked codex32 string with BCH error correction**, in the
same string layer `md1` and `mk1` already use. This is the constellation-native
form: human-readable, hand-engravable, and — the point — **fault tolerant**.

**The machinery exists and is proven; `mt1` is a new payload in it, not a new
codec.** `md-codec` ships a syndrome-based BCH *corrector*, not merely a
detector: `decode_with_correction` and `CorrectionDetail` in
`crates/md-codec/src/lib.rs:48`, Berlekamp–Massey over `GF(1024)` in
`crates/md-codec/src/bch_decode.rs`, on the `BCH(93,80,8)` regular-code variant of
BIP-93. A hand engraver who cuts a character wrong gets it corrected rather than
discovering years later that the plate is scrap.

**The payload is the raw signed transaction, NOT the PSBT — deliberately, and
for a different reason than §3.** F-234 binds the *QR*, because the QR is the
escape hatch for a recoverer holding no `mt`-aware software; it must therefore
carry a form the wider ecosystem might read. An `mt1` string is
the opposite case: **nothing but `mt`-aware software will ever parse it**, so
F-234's argument does not apply and size is what matters. Dropping the PSBT
wrapper saves the **+58 to +61 bytes per input** measured in §3 — which at 5 bits
per character is real engraving time by hand.

### The chunking rule — NORMATIVE, and stated here for `mt-codec` itself

**R6's implementability lens filed this as a Critical and it was right: no line
in this document said how `mt-codec` chooses `count`.** The rule existed only as
prose *about `md-codec`*, inside a correction box that warns the reader against
reading a limit as a rule — so the spec required the exact inference it told the
reader not to make. Two implementers diverge on the spec's own arithmetic: one
reads the 400-bit single-string capacity and sizes chunks at `(400 − 49)/8 = 43`
bytes, and a 535-byte transaction becomes **13 chunks, not 14**. Every plate
then fails the other implementation's §1.1e length check as damaged steel.

Stated normatively, for `mt-codec`, in the only place it is stated:

    PAYLOAD_BYTES_PER_CHUNK_CEILING = 40
    count           = ceil(payload_len / 40)          # never 0; count >= 1
    bytes_per_chunk = ceil(payload_len / count)       # BALANCED, not filled
    chunk i         = payload[i*bytes_per_chunk ..]   # last takes the remainder

**Two constants, two different jobs, and conflating them is the whole defect.**
`40` is the **ceiling the count is derived from** — it never describes a chunk's
size. `bytes_per_chunk` is what a chunk actually carries, it is `≤ 40`, and it is
**equal across the set except for the last**. A 535-byte payload gives
`count = 14` and `bytes_per_chunk = 39`, not thirteen 41-byte chunks and not
fourteen chunks of 40/40/…/15.

> **Why balanced rather than filled, restated so it is not re-derived:** this
> matches `md-codec` (`crates/md-codec/src/chunk.rs:267-273`), and the
> constellation's Rust-primary rule makes that binding — a sibling's normative
> behaviour is not re-litigated in a downstream spec. §10.12 records the ruling.

### What fits

A chunk carries **at most 40 payload bytes** and `mt1`'s header admits **32,768
chunks**,
so the ceiling is **1,310,720 B** — above Bitcoin's own ~100 KB standardness limit,
so `mt1` encodes any transaction that will relay (§3). (An earlier draft said 64 chunks / 2,560 B,
inheriting `md-codec`'s 6-bit `count` field that `mt1` does not use — see §3.) Measured
(`RESULTS_envelope_2026-08-22.txt`, `RESULTS_rcw_2026-08-22.txt`):

| artifact | raw bytes | chunks | fits? |
| --- | --- | --- | --- |
| RCW `tr` key-path, 1-in/1-out | 162 | **5** | yes |
| RCW `tr` tier 4, 1-in/1-out | 405 | **11** | yes |
| RCW `tr` tier 1, 1-in/1-out | 535 | **14** | yes |
| RCW `wsh` tier 1, 1-in/1-out | 742 | **19** | yes |
| RCW `tr` tier 1, 5-in/2-out | 2498 | **63** | yes, barely |
| RCW `wsh` tier 1, 5-in/2-out | 3538 | **89** | yes — 2% of `mt1`'s 32,768 |

**Both verbs share the 32,768-chunk ceiling**, because both use `mt1`'s header.
What differs is what a chunk *costs*: one chunk is one hand-cut string of ~96
characters, or about 1/24th of a machine-engraved QR symbol. **The same count is
two orders of magnitude apart in human effort**, which is why §8.7b warns in
characters and the deferred QR verb would warn in plates and minutes.

> **CORRECTION — every number above was ~13% low until 2026-08-23, and so was
> the ceiling.** R0 round 1 (S-1) found that the probe helper feeding all of
> them modelled a chunk as `(bytes*8).div_ceil(363)`. 363 = 80 codex32 symbols
> x 5 bits − 37 header bits, i.e. what a chunk *could* carry if the chunker
> **filled** to long-form capacity. **It does not.** `md-codec` sizes chunks by
> `SINGLE_STRING_PAYLOAD_BIT_LIMIT = 64 * 5 = 320` bits
> (`crates/md-codec/src/chunk.rs:224`), applied over `payload_bytes.len() * 8`
> (`crates/md-codec/src/chunk.rs:253-254`) — **40 bytes is the CEILING the chunk
> count is derived from, not the size of each chunk.**
>
> **An earlier version of this box called it "a flat 40 bytes per chunk", and
> that mis-describes the chunker — R4 lens 1.** `md-codec` computes
> `chunks_needed` against the 320-bit ceiling and then splits the payload
> **`bytes_per_chunk = ceil(len / count)`**, each chunk taking that many bytes
> and the **last taking whatever remains** (`crates/md-codec/src/chunk.rs:267-273`).
> No chunk is padded to 40.
>
> **An intermediate version of this box said "the last chunk is not a short
> remainder", and that describes a different split — R5 readiness.** Under
> `ceil` the last chunk *is* the remainder and is normally shorter: a 535-byte
> payload over 14 chunks gives `ceil(535/14) = 39` bytes each for the first
> thirteen and **28** for the last. Two implementers, one following the sentence
> and one following the code, produce different chunk boundaries and therefore
> **plates neither can read**. Correcting the flat-40 error introduced this one
> in the same paragraph. The **chunk
> counts in this spec are unaffected** — they derive from the ceiling, which is
> what `chunks_needed` uses — but the **per-chunk sizes** differ on any payload
> that is not a multiple of the chunk count. This is the same error class as the
> 363-vs-320 correction above: a limit read as a rule.
>
> **`mt1` balances too**, which §10.12 already implies by forbidding fill, and which
> §4's *"never leave redundancy unbought"* requires: a padded chunk spends plate
> area on nothing.
>
> **The error was per-chunk, and it is easiest to see there.** The old model
> put **45.4 payload bytes** in a chunk where the chunker puts **40** — about
> 13% too many — so every chunk count derived from it was that much too low.
> At the time the chunk count was capped at 64, so the mistake also showed up as
> a **2,904 B versus 2,560 B** total ceiling, and a transaction inside that
> 344-byte band would have been called "fits" and then returned
> `ChunkCountExceedsMax`. Those two totals are themselves now historical — the
> cap is 32,768 chunks (§3) — but the per-chunk figure is the durable part and it
> is what §3b's table rests on.
>
> The defect was **one shared helper replicated across seven probe binaries**,
> so every chunk count in every results file was wrong the same way — a corpus
> can be uniformly wrong and still look perfectly self-consistent. It is now the
> named constant `CHUNK_PAYLOAD_BITS = 320` carrying the citation and the
> history, and **all thirteen binaries were rebuilt and re-run**. Capacity
> conclusions moved: single-sig `tr` key-path from a 26-input ceiling to **23**,
> RCW `wsh` tier 1 from 4 inputs to **3**.
>
> The measurements README *did* label the old counts "a floor", which was the
> right hedge. §3b dropped the label and presented them as counts, and §8.7b
> refused against them. **The caveat existed and was lost in transit.**
> Whether `mt1` should instead FILL its chunks, raising the ceiling, is §10.12.

### Layout on steel is the user's, not `mt`'s

> **Scope ruling, operator, 2026-08-23.** *"How many codex32 characters fit a
> hand engraved plate? As many as a user wants. It is not our concern."*

**`mt encode` emits a string. That is the whole of its output.** Font size,
characters per plate, how many plates, what order they are laid out in, whether
the string is cut by hand or by machine, and whether anything is engraved beside
it are all the user's decisions. This spec does not constrain any of them, and
§4's configuration search does not apply to this verb.

An earlier version of this section derived a chars-per-plate table from the
fork's font ladder and drew plate counts from it. **That was out of scope and is
deleted.** What survives from it is the one part that *is* a property of the
codec rather than of anyone's steel: the **32,768-chunk ceiling** above, which
binds regardless of how the string is engraved, and which §8.7b refuses
against.

> **The distinction that decides what belongs here:** what `mt` *emits* is this
> spec's concern; what a user does with steel is not. `mt qr` is the exception
> only because it emits an engraving, so plate geometry is part of its output.

### The one thing `mt encode` does say about the plate

> **Ruling, operator, 2026-08-23:** *"Hand cut plates get a warning on stderr.
> And that's it."*

`mt encode` prints a warning at encode time that the artifact is **bearer** —
anyone holding the resulting plate can broadcast it — and takes no further
interest in the steel. It does not require a legend, does not reserve space for
one, and cannot verify that any warning reached the plate.

**On `stderr` specifically, and this is load-bearing rather than incidental.**
The `mt1` string goes to **stdout**, so the ordinary invocation pipes it to a
file or another tool. A warning on stdout would be captured by that redirection
and silently swallowed; on stderr it reaches the operator's terminal either way.
This is the first fixed point of §10.10's CLI contract: **stdout carries the
artifact, stderr carries everything the human must see.**

**The accepted risk, stated plainly rather than buried.** That warning is seen
**once**, by the person doing the encoding. The person holding the plate in 2040
is a different person, and the plate itself says nothing. This is a deliberate
asymmetry between the two verbs — `mt qr` engraves `BEARER - ANYONE HOLDING THIS
CAN BROADCAST IT` as the first line of a legend `mt` controls, and `mt encode`
has
no such mechanism because it emits no engraving. §7 records it as an accepted
risk, not as a mitigation.


## 4. Choosing the configuration — MOVED (§0a)

> **Moved to `design/SPEC_mt_qr_DEFERRED.md` on 2026-08-23.** It chose module
> size, QR version, ECC level and tiling — every one an `mt qr` input, and
> nothing in v0.1 reads them. §0a defers `mt qr` to a cross-format cycle shared
> with `md1` and `mk1`, so the selection rules and their measurements wait there
> with the verb.
>
> **The heading stays rather than the section being deleted**, because §5's
> plate-area reservations still refer to this material and a reader arriving at
> a missing §4 would have no idea where it went.

## 5. The plate legend — LIVE for `mt encode`, sized for the deferred `mt qr`

> **The heading said "`mt qr` only, DEFERRED" until 2026-08-23, and that was
> wrong.** Five fields of this section are printed by `mt encode` in v0.1, so
> labelling the whole thing deferred invited exactly the cut that the QR
> extraction nearly made. What is deferred is the **plate-area** material — the
> millimetre reservations, lines per plate, and the ECC-level tradeoff — which
> stays here beside the fields it sizes rather than moving, because separating a
> field from its measurement helps nobody.
>
> **Retained for the deferred QR cycle, and for one live purpose:** §0a rules
> that `mt encode` **prints these FIVE fields on `stderr`** as suggested text
> the operator may engrave beside their string. (Five until §10.21 added
> `FORMAT: mt1 codex32` on 2026-08-23; this note said "five" for the rest of
> that day — R6 fold-propagation I-4.) The measurements and the field
> choices below are what that suggestion is made of.


Everything constellation-specific lives here, in engraved text, never in the QR.

**The legend carries only what a human needs BEFORE the QR is decoded.** Five
fields, **152 characters**, 6 lines — measured,
`RESULTS_legend_budget_2026-08-22.txt`:

| field | chars | why |
| --- | --- | --- |
| `BEARER - ANYONE HOLDING THIS CAN BROADCAST IT` | 45 | the plate carries a transaction anyone holding it can broadcast; this is not a backup in the sense the other formats are |

> **"BROADCAST", not "SPEND" — operator ruling 2026-08-23.** §8.6 refuses inputs
> whose satisfaction does not bind the outputs, so in the ordinary case a holder
> **cannot redirect the money**: the destination is fixed by signatures they
> cannot alter. What they can do is **cause the transaction to happen** —
> sending the funds where the operator already chose, at a moment the operator
> did not.
>
> **`mt` GUARANTEES NOTHING HERE, AND AN EARLIER VERSION OF THIS NOTE SAID IT
> DID — R6 adversarial C-4.** It asserted flatly that a holder *"cannot redirect
> the money"*, and used that assertion to choose **text cut permanently into
> steel**. The spec knows better two sections away: §8.6 recognises a signature
> **by shape**, because §8.2's removal left no script engine, and it says so —
> *"a crafted witness carrying a signature-shaped element that the script never
> checks would pass. This is a structural heuristic, not a proof."*
>
> **A concrete input defeats it.** A taproot script-path leaf satisfied by a
> 64-byte hash preimage — witness `[preimage(64), script, control_block(65)]` —
> has its last two elements stripped as control block and leaf script, leaving a
> 64-byte element that the shape rule counts as a Schnorr signature with
> `SIGHASH_DEFAULT`. `mt` accepts. The satisfaction commits to **nothing**, and
> any holder re-satisfies the same leaf with different outputs and keeps the
> money. That is precisely what §8.6(b) was added to close, reached **through**
> the recogniser rather than around it.
>
> **Operator ruling 2026-08-23, and it settles the class rather than the
> instance:** *"we aren't guaranteeing anything. We are helping users understand
> what they are doing. We may be incomplete and can warn users of that."* So
> this spec claims no unqualified property about what a holder can do. `mt`
> reports what it checked, names what it could not, and the wording stays
> `BROADCAST` — which remains right for the ordinary case and, unlike `SPEND`,
> does not invite a reader to imagine a power the design tries to withhold.
>
> **The warning `mt encode` prints therefore says both halves:**
>
>     BEARER: anyone holding this plate can broadcast this transaction.
>
>       mt checked that every input carries a signature committing to the
>       outputs, so a holder should not be able to send the money anywhere
>       else. That check reads WITNESS SHAPE, not script -- mt has no
>       script engine (8.2). An exotic or hostile input CAN defeat it.
>       Treat the plate as if a holder could take the funds.
>
> So `SPEND` was wrong in both directions. It **overstates** the holder's power,
> implying theft that §8.6 exists to prevent; and it **misnames the real
> hazard**, which is *timing* — a payment completed early, or after the operator
> changed their mind, or after the destination wallet became unreachable. A
> reader who takes "spend" literally builds the wrong model of what they are
> holding.
>
> Cost: 41 → 45 characters, legend 141 → **145**, still **6 lines**
> (`RESULTS_legend_budget_2026-08-22.txt`), so §4's reservation does not move.

| `FROM WALLET <8 hex>` | 20 | wallet id or seed fingerprint. The transaction does **not** say what it spends *from* (§6). **Optional — loudly warned when absent** (§10.4) |
| `LOCKED TO BLOCK <n> ~<SEASON> <year>` / `LOCKED UNTIL <t>` | 35 | the single most actionable fact. Reads **`NO TIMELOCK`** when there is no enforced `nLockTime`. **A statement about the transaction's fields, never about spendability** — `mt` does not evaluate scripts, so it reports the lock it read and lets the reader conclude (§8.4) |
| `TO <wallet id, fp or label>  <amount>` | 34 | names the destination **wallet**, not one truncated address — operator ruling, §10.4. **Optional — loudly warned when blank.** A free-text label is allowed **only behind an explicit flag**, since nothing can check it against the transaction |

> **`PLATE n OF m` WAS A FIELD HERE AND IS DELETED — R8 coverage C-3.** §0a had
> already dropped it (*"`mt` cannot compute `m`… `PLATE 1 OF 1` cut onto each of
> five plates is a false completeness claim on permanent steel"*) and **this
> table still listed it**, so the spec carried the field and its own deletion at
> once. Operator ruling 2026-08-23 goes further and removes the *category*:
> **`mt` does not know how strings map to plates at all** — all on one, one
> each, or any split — and that is the operator's decision or
> `mnemonic-engrave`'s, never `mt`'s.
>
> **The `n/m` printed per string stays**, and is a different claim: `mt` knows
> exactly how many *strings* it emitted. It is the *plate* denominator that
> would be invented.
| `FORMAT: mt1 codex32` | 19 | **names the encoding, so a stranger can start.** Operator ruling 2026-08-23, closing §10.21 — see the note below for why this field is not the least important of the five but arguably the most |

> **`FORMAT: mt1 codex32` — the only field a recoverer cannot do without, and
> the only one naming a standard rather than this project.** Operator ruling
> 2026-08-23, closing §10.21.
>
> Every other field on this plate is a **convenience**: `mt inspect` can
> reconstruct the destination, the amount, the locktime and the bearer warning
> from the string itself, and with a node reachable it can say more than the
> legend ever could (§1.1). **What no amount of inspection recovers is which
> program to run.** `MT1QZRF8X…` in a search engine returns nothing, and a
> recoverer who cannot name the format cannot reach any of the other fields.
>
> **`codex32` is why this wording and not a URL.** The operator raised a GitHub
> URL, which is a reasonable instinct and the wrong durability: a domain or an
> org name must **outlive the plate**, twenty years is longer than most of them
> last, and a lapsed name someone else buys back is *worse* than no line at all
> — it points a recoverer holding a bearer instrument at a stranger. `codex32`
> is **BIP-93**, published and archived independently of this project, so the
> tag stays findable through a channel that does not depend on this repository
> existing. `mt1` alone does not have that property; `codex32` does. A repo URL
> may accompany it as *additional* suggested text (§0a) — it is a convenience
> layered on a durable tag, never the tag itself.
>
> Cost: **+19 characters.** It briefly took the legend to 7 lines; deleting
> `PLATE n OF m` on the same day took it back to **6**, so the net is 5 fields,
> **152 characters, 6 lines** and §4's reservation never moved. Free for `mt encode`, where the legend is `stderr` text
> and `mt` owns no layout; **not** free for the deferred `mt qr` cycle, where
> the legend briefly reached 7 lines, which `legend.rs` showed would have cost
> the deferred QR cycle real area. **Deleting `PLATE n OF m` the same day took
> it back to 6, so that cost never materialised** — the two edits were made
> hours apart and neither knew about the other. Recorded because a future
> reader will find the 7-line note in the history and should know it was
> withdrawn, not carried.

Plus, **not part of the 152-character budget above**, one `n/m` label beside
**each engraved unit** — a string for `mt encode`, a symbol for the deferred
`mt qr` — naming the `mt1` chunk it carries (§10.8's ruling). **`n/m` is the
only completeness claim `mt` can make**, because it counts what `mt` emitted;
any denominator over *plates* would be invented (§0a).

> **This budget rests on a DOC COMMENT, not on the fork's font metrics, and the
> doubt is not resolved here.** `legend.rs` hardcodes `CHARS_PER_LINE = 35.0`
> and `LINES_FULL_PLATE = 20.0` taken "per `crates/me-cli/src/lib.rs:46`" — and
> this project's own rule forbids describing code from its doc comment. The
> fork's real grid is `CharsPerLine = (plateSize − 2·outerMargin) /
> fixedCharWidth` and `LinesPerPlate = (plateSize − 2·outerMargin) / fontMM`
> (`backup/backup.go:87-97`) over a **six-rung** ladder `FontSizes`
> (`backup/backup.go:82`), pinned by the fork's own `TestFontSizeLadder`
> (`backup/sizes_test.go:29-56`) at 22/13, 26/15, 30/17, 34/20, 38/23 and 44/26
> characters-per-line / lines-per-plate. **`legend.rs`'s two values are the
> 3.8 mm rung of six, treated as universal.**
>
> §4's 4.25 mm line pitch compounds it: that is `85/20`, using the **full** plate
> height where §4 uses 79 mm everywhere else, and 4.25 mm is not a rung of
> `FontSizes` at all. The nearest real rungs put 6 lines at 26.4 mm (4.4 mm) or
> 22.8 mm (3.8 mm) against §4's 25.5 mm.
>
> Magnitude is under a millimetre, but **§4's entire plate table and this
> section's 6-line reservation both stand on it.** Filed as §10.14 rather than
> patched, because regenerating §4's table is a measurement task, not a wording
> one. Note the fork test was **not executed** — Go is absent from this machine —
> so the six rungs above are the fork's committed pins cross-checked against an
> independent derivation from the source formulas, which is two agreeing
> derivations and not a run.

> **What the `TO` line does NOT do.** It was `TO <truncated addr>` until
> 2026-08-23, showing **one** output and truncated — so a transaction with
> change named one destination, silently omitted the rest, and offered an
> address that could not be checked by eye. R0 round 1 (R-14) filed that as a
> Critical against §7's pinned-destination mitigation. The operator's ruling
> replaces it with a **wallet identity**, which names the counterparty instead
> of one of its scripts and does not degrade with output count.
>
> It is still not a full disclosure: it is one line, it is optional, and it says
> nothing when the destination is not a known wallet (§10.4). `mt` prints every
> output in full at encode time; the plate carries the summary.

### What was dropped, and why

The first draft listed ten fields and measured **474 characters at one input**,
growing to 1,066 at five — against a 300-character budget
(`crates/me-cli/src/lib.rs:48`). It could never have fitted.

Four fields were cut on one principle: **everything derivable from the decoded
transaction is duplication.** The txid, the input outpoints and the full
destination address are all *in* the transaction. Engraving them buys nothing —
and in the one case where it might seem to, an unreadable QR, the duplicate is
useless anyway because you still have no transaction to broadcast.

| dropped | recoverable how |
| --- | --- |
| txid | hash the decoded transaction |
| input outpoints | they are the transaction's inputs |
| fee rate and date | inputs − outputs, and the PSBT carries the input amounts |

> **§7's mitigations were written against the ten-field legend and were not
> re-read when it became five. R0 round 0 found this from two independent
> directions.** Four sections went on promising fields that no longer existed,
> and two of §7's four hazard mitigations named them. §7 below is corrected: it
> now claims only what §5 actually engraves. **A diff falsifies text it never
> touches** — the legend rewrite made those sentences false without editing
> them.

**The stub is a hint, never an authority.** It is the top 4 bytes of a canonical
md1 identity, form-aware — WalletPolicyId for a keyed wallet, the key-stable
WalletDescriptorTemplateId for a keyless template — reusing `mk1`'s existing
derivation — all three citations below are in the **`mnemonic-key` repo**, not
this one, so `plan-cite-check.sh` cannot resolve them and they were checked by
hand: `POLICY_ID_STUB_BYTES = 4` at `crates/mk-codec/src/consts.rs:60`, the form-aware
rule documented at `crates/mk-codec/src/key_card.rs:25-33`, and the derivation
`derive_stub_from_md1_card` at `crates/mk-cli/src/cmd/mod.rs:126`. So one convention
spans the constellation. If the legend says wallet X and the transaction spends
wallet Y's UTXOs, **the transaction wins.** The stub exists to help a human find
the right plates, not to validate anything, and nothing may branch on it.

**Where the stub comes from is unspecified, and that is an open question**, not
a settled design: `FROM WALLET` is a mandatory field sized into §4's
reservation, and nothing says what supplies it or what happens when it is
absent. See §10.4.

## 6. Why provenance is asymmetric

**"Goes to" is already in the transaction.** Outputs carry scriptPubKeys; any
standard decoder yields addresses and amounts. Encoding destinations into the
wire format would create a second source of truth that can disagree with the
transaction — and on disagreement a recoverer would have to guess which to
believe. That is a funds-safety hazard, not a feature. It is displayed, never
encoded.

**"Comes from" is partly absent.** A signed transaction references inputs as
outpoints only; the source scriptPubKeys live in the *previous* transactions.
Without them you cannot tell which wallet it spends. **`mt qr`'s** finalized
PSBT closes part of this by carrying each input's UTXO record — value and
scriptPubKey — so that payload does describe what it spends. **`mt encode`'s
does not**: a raw transaction carries outpoints only, so a string plate is
silent about both the input amounts and the source scripts. It still does not name the *wallet*, hence the stub, and
hence the stub living in text, because it is the one constellation-specific fact
on the plate and F-234 forbids that inside the QR.

> **`mt` PREFERS a finalized PSBT and accepts a raw signed transaction with
> §8.2e's loud warning — corrected, R6 implementability I-7. Even for
> `mt encode`,
> whose engraved payload is the extracted raw transaction.** Input format and
> payload format are separate decisions; requiring a PSBT is what keeps §8.2 and
> §8.2b runnable at all.
>
> **Do not lean on the PSBT's input amounts as trusted without a rule.** For
> segwit inputs they are committed to by the signature (BIP-341 `sha_amounts`,
> BIP-143), so they are cryptographically bound. **For legacy inputs nothing
> commits to them at all** — R0 lens 2's finding, and it survives the scope cut
> even though the section it was filed against is gone. §8.6 is the rule.

### 6a. When `bitcoind` is reachable, check the inputs are still unspent

**Operator ruling 2026-08-22, rescoped 2026-08-23.** If a node is available,
`mt` resolves every input itself and the operator is asked for nothing.

This section used to source input *amounts* for transaction construction. `mt`
no longer constructs, and the amounts now arrive inside the payload, so its job
is narrower and sharper: **before you spend an evening cutting steel, is this
transaction still worth engraving?**

The call is **`gettxout <txid> <vout> false`**, verified against a live Core
v25.0.0 node. It is the right RPC for three reasons:

- it returns `value` and `scriptPubKey` together, so the PSBT's claimed UTXO
  records can be checked against the chain rather than trusted;
- **it answers unspentness in the same call**, because it queries the UTXO set
  rather than the chain. A spent or nonexistent output returns `null`;
- it needs **no `-txindex`**, unlike `getrawtransaction`.

**NO NODE IS A WARNING, NOT A SILENCE.** Operator ruling 2026-08-23:
*"bitcoind might not be available and we need a warning for that."* An earlier
draft made every check in this section conditional on a node being reachable and
said nothing when one was not — so the quietest possible run was also the
least-verified one, and the operator could not tell the difference. `mt` names
what it could not check:

    WARNING: no bitcoind reachable. These checks did NOT run:

      - are the inputs still unspent?        (§8.5)   UNKNOWN
      - do the PSBT's input values match
        the chain?                           (§6a)    UNKNOWN
      - has the locktime already passed?     (§8.4)   UNKNOWN
        locked to block 1383520, current height unknown

    The transaction may already be unspendable, and cutting 1,242
    characters by hand is not quick. Consider re-running with a node
    before you start.

> **THAT WARNING IS ENCODE-SHAPED, AND THE RECOVERY PATH NEEDS ITS OWN.**
> Operator ruling 2026-08-23 — *"warn about what cannot be confirmed"* — applied
> to `mt inspect` and `mt decode`, where the advice above is useless: the plate
> already exists, so *"before cutting"* names a decision made years ago, and
> *"a plate is ~21 minutes"* prices something the reader is not buying.
>
> **The enumeration carries over unchanged; the consequence line does not.** The
> recoverer's decision is not *cut or don't cut*, it is **broadcast or don't
> broadcast** — irreversible in the other direction, and taken by someone who
> may know nothing about this transaction beyond what `mt` just told them:
>
>     WARNING: no bitcoind reachable. mt read this transaction from the
>     strings, but could confirm NOTHING about it against the chain:
>
>       - do these inputs still exist, or were they already spent?  UNKNOWN
>       - was this transaction already broadcast?                   UNKNOWN
>       - has the locktime passed?                                  UNKNOWN
>         locked to block 1383520, current height unknown
>       - what fee does it pay?                                     UNKNOWN
>         (the fee needs input values, which are not in the tx)
>
>     Everything above this line was read from the plate itself and is
>     what the transaction SAYS. None of it is confirmed.
>
>     TO RESOLVE ALL FOUR AT ONCE, either:
>       - run mt inspect again with a bitcoind reachable, or
>       - look this txid up in any block explorer:
>           9a3f21c0d4e5b6a7f8091a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f
>
> **The resolution line is a ruling, and it exists because the honest report
> creates the deadlock it resolves.** Operator ruling 2026-08-23 — *"warn to
> make bitcoind available or check txid in blockexplorer"* — closing the last
> divergence of Journey B.
>
> With no node, **every actionable field reads `UNKNOWN`**, which is correct and
> leaves the recoverer no basis to act *or* to discard. The plate goes back in
> the drawer, and the tool's most likely offline outcome is **paralysis** —
> arrived at through nothing but accurate reporting. Four `UNKNOWN`s tell them
> nothing they can do, which is the exact condition under which a line of
> guidance beats silence.
>
> **`mt` can print the txid because it just reconstructed the transaction**, so
> this costs no new capability: it is computable with no node and no network.
>
> **It is NOT the double-SHA-256 of the bytes `decode` emits, and an earlier
> version of this sentence said it was — R6 adversarial C-1.** Those bytes are
> the network serialization, so hashing them gives the **wtxid** (BIP-141). The
> **txid** hashes the same transaction with marker, flag and witnesses
> stripped. Every artifact §3b measures is `tr` or `wsh`, so **segwit is the
> normal case here, not an edge** — for the smallest measured artifact the two
> are hashes over 162 and 94 bytes, different preimages and different values.
>
> **The consequence was a wrong action, not a wrong number.** A recoverer
> following the line below pastes a wtxid into a block explorer, gets nothing
> back, and concludes the transaction was never broadcast — when it may have
> confirmed years ago. Explorers index the txid. **This row exists to be looked
> up, so it must be the value that can be.** That makes the second
> option genuinely offline-friendly — the recoverer needs *someone's* internet,
> not their own node, and a block explorer answers all four questions from that
> one string.
>
> **Naming both options matters more than either.** A recoverer told only to
> *"run a node"* in 2040 faces a multi-hundred-gigabyte sync before they can
> read their own plate; one told only to *"use an explorer"* is being pushed to
> hand a third party a bearer instrument's txid. The two have opposite
> trade-offs — effort versus privacy — and which one is acceptable is the
> recoverer's call, not this spec's.
>
> **The last two lines are the load-bearing ones**, and they are the difference
> between the two warnings. An offline `inspect` still reports a destination, an
> amount and a locktime, all of them read correctly — and a reader who has just
> been shown a clean report is *more* likely to act on it, not less. **The
> warning's job is to separate what was read from what was verified**, because
> the report looks identical either way and nothing else on the screen marks the
> difference.
>
> `UNKNOWN` is also already the right word: §6a's liveness table uses it for
> exactly this — `mt` cannot distinguish DEAD from PENDING and says so, rather
> than picking the reassuring one.

**Enumerating the skipped checks is the point.** *"No node"* alone tells the
operator nothing they can act on; a list of what is therefore unknown tells them
exactly what they are trading for convenience, and the plate-time reminder tells
them what it costs to be wrong. This is the same principle as §8.2c: state the
mechanism, not the caution.

**Not a refusal.** Offline operation is the constellation's posture (§0), and
§8.5 refuses only on a node's *positive* answer that an output is spent — an
absent node is an absent answer, not a bad one.

**Use the value it returns, not merely its null-ness.** `gettxout` returns
`value` and `scriptPubKey` — which is this section's stated reason for choosing
it over `getrawtransaction` — and an earlier draft acted only on whether the
result was `null`. R3's information lens (I-2) caught that: since §8.2's
removal, **the chain's own answer is the only value check `mt` has for a segwit
input**, and it was being thrown away. `mt` compares the fetched `value` against
the PSBT's UTXO record for that input and **refuses on mismatch**, naming both
numbers. This is a comparison of two integers, not script evaluation, so it sits
inside §8.4's scope ruling.

`include_mempool` is passed **false** deliberately. The default is `true`, and
mempool state is the wrong basis for an artifact meant to sit in a drawer for
years.

> **Known limitation, from R0 lens 2, not yet resolved.** `false` also means a
> mempool-spent input reads as *unspent*, which is the opposite of the caution
> this section argues for. And a `null` cannot distinguish "already spent" from
> "this node is still syncing, or is on the wrong chain". §8.5 states the rule
> that follows; whether `mt` should additionally require the node to be out of
> IBD is **§10.5**.

## 7. Threat model

An `mt` plate is unlike every other plate in the constellation. `md1` and `mk1`
are watch-only public material: losing one costs privacy, not money. `ms1` is a
secret, and `me` refuses to push it over NFC at all. **An `mt` plate is
broadcastable by whoever holds it.** In hazard terms it sits nearer `ms1` than
`md1`, and the existing tooling's assumption that "public string" means "safe to
engrave" does not hold here.

**Every mitigation below names a field §5 actually engraves.** Where there is no
mitigation, the row says so instead of inventing one.

| hazard | mitigation |
| --- | --- |
| **Bearer** — holder can broadcast (`mt qr`) | a timelock bounds it in *time*, not in space, and only when §8.4's `nSequence` condition holds; the `BEARER` line is the first line of a legend `mt` controls |
| **Bearer** — holder can broadcast (`mt encode`) | **accepted risk, not mitigated on the plate.** `mt` emits a string, not an engraving, so it has no mechanism to put a warning on hand-cut steel (§3b). It warns once on `stderr` at encode time, to the person encoding — who is not the person holding the plate later. The timelock bound still applies |
| **Pinned destination** — a 2040 recoverer pays a 2026 address whose keys may be lost | **cannot be fixed; partly disclosed.** §5's `TO` line names the destination **wallet** (id or fingerprint), which does not degrade with output count as the old truncated-address form did — but it is **optional**, and says nothing when the destination is not a known wallet (§10.4). `mt` displays every output in full at encode time; the plate carries a summary |
| **Indistinguishable from a watch-only plate** — an `mt1` plate sits in the same drawer as `md1` and `mk1` plates, in the same script, differing in **one HRP character**, and is the only one of the three that **moves money when whoever picks it up broadcasts it** | for `mt qr` the `BEARER` legend line carries the difference. For `mt encode` there is **no mitigation** — see the bearer row above and §3b. R0 round 1 (R-13) |
| **Pinned fee** — a 2026 fee rate may be unbroadcastable in 2040 | **cannot be fixed by `mt`, and is NOT on the plate.** `mt` warns below 10 sat/vB (§8.2b) and names two things a future holder can try, guaranteeing neither: **CPFP** — spending one of this transaction's outputs with a high-fee child, which needs no key from the original signer, unlike **RBF**, which requires signing a replacement and is therefore useless to a plate holder — and **out-of-band submission** straight to a miner, which bypasses relay policy and is the escape hatch when a fee is too low for the parent to reach a mempool at all. **Neither is recoverable from an `mt encode` plate's own contents**, since a raw transaction carries no input amounts (§6) |
| **Silent invalidation** — one ordinary spend of any input voids the plate, and nothing on it says so | **not mitigated on the plate.** The input outpoints were cut from the legend (§5), so a holder cannot check unspentness from the plate alone — they must decode the QR first. `mt` checks it at encode time (§6a, §8.5); after that the hazard is open and undisclosed on steel |
| **Non-`ALL` sighash** — an input signed with `SIGHASH_NONE` or `SIGHASH_SINGLE` leaves outputs unbound, so a plate-holder can redirect the funds and the `TO` line becomes a lie | refused at encode time, §8.6 — **structurally**, since §8.2's removal left no script engine |
| **Wrong input value** — a legacy input whose claimed value is wrong yields a valid transaction, and **the fee absorbs the entire difference** | **not detectable by `mt`.** §8.2's removal means no signature is verified, and a legacy sighash never committed to the amount anyway. Mitigated only by §8.2c's `stderr` warning, which states the arithmetic `(real input value) − (output total)` since the output total is the one term `mt` knows for certain. **Nothing reaches the steel for `mt qr`** — §5's legend is full (§8.2c). An `mt encode` operator controls their own plate and may add a reminder; `mt qr`'s operator cannot |
| **Well-formed but INVALID** — a transaction with a bad signature engraves cleanly and fails at broadcast, years later | **accepted, not mitigated.** Operator ruling 2026-08-23 removed script verification from v0.1 (§8.2). §8.1 sees a witness, §8.2b sees balanced values, §8.6 sees correct sighash flags — none of them verifies the signature. `mt` may add this someday |

> The last two rows are the honest state of this design. R0 lens 2 found that the
> previous draft claimed the plate carried outpoints when §5 had removed them,
> which turned an *undisclosed* hazard into a *falsely mitigated* one. Recording
> "not mitigated" is worse-looking and more useful.

## 8. Refusals

**This section now carries the whole safety argument.** `mt` builds nothing, so
everything it can get wrong is a failure to inspect what it was handed. All are
machine-checkable before a single plate is cut. **Every refusal below binds BOTH
verbs** unless it names one — a hand-engraved plate is exactly as bearer, and
exactly as permanent, as a machine-engraved one.

**THE REFUSAL MESSAGE FORMAT — RULED 2026-08-23**, closing the last open item in
§10.10. Every refusal `mt` prints has three parts, in this order, on `stderr`:

    mt encode: REFUSED — §8.2b, fee rate 31,250 sat/vB exceeds 25,000.

      Inputs total 0.50000000 BTC and outputs total 0.00400000 BTC, so this
      transaction pays 0.49600000 BTC in fees over 1,588 vB. mt refuses
      above 25,000 sat/vB because a fee that large is almost always a
      mistake in the input values, not an intention.

      Supply the input values with --input-value <index>:<amount>, or
      re-run with a node reachable so mt can fetch them.

- **The verdict line** — `<verb>: REFUSED — §<ref>, <reason with the number
  that caused it>`. **One line, and it always names both the section and the
  value.** §8 promises each refusal *"names the number that caused it"*; this is
  where that promise is kept, and it is what P5's tests assert against.
- **The mechanism** — what was read, what the rule is, and **why the rule
  exists**. Stating the mechanism rather than the caution is §8.2c's posture
  applied everywhere: an operator who understands *why* can tell a real problem
  from a mis-supplied value.
- **What to do**, when there is something — the flag to supply, the command to
  re-run. **Omitted entirely when there is nothing**, rather than padded with
  advice that does not apply.

> **Two properties make this testable rather than decorative.** The verdict line
> is **machine-parseable** — a stable prefix, a `§`-reference, and a number — so
> a P5 test asserts on the reference and the value without matching prose that
> will be reworded. And the parts are **ordered by what a reader needs first**:
> the verdict answers *what happened*, and everything below it is optional
> reading for someone who wants to know why.
>
> **`REFUSED` is reserved for refusals.** A warning is not a refusal, and §8's
> warnings use `WARNING:` with no `§`-reference in the first line — a reader
> scanning `stderr` must be able to tell, at a glance, which output stopped the
> run. That distinction is load-bearing: `mt` warns far more often than it
> refuses, and a format that blurs the two teaches operators to skim both.

1. **Not fully finalized** → refuse, **on both payloads, by their own
   vocabulary.** For a PSBT: every input carries a populated
   `PSBT_IN_FINAL_SCRIPTSIG` or `PSBT_IN_FINAL_SCRIPTWITNESS`. For a raw
   transaction: every input carries a non-empty `scriptSig` **or** a non-empty
   witness. Neither format makes an unfinalized transaction unrepresentable —
   §3's retraction — so this check is mandatory on both verbs and may not be
   skipped or overridden.
2. **Script validity is NOT checked in v0.1.** **Operator ruling 2026-08-23:
   *"We don't care if transaction is valid for initial version. We might never
   care but we might add it someday."*** The previous draft ran real
   libbitcoinconsensus verification here; that is removed, and with it `mt`'s
   only dependency on a consensus engine.

   > **What this costs, stated plainly because nothing else in §8 covers it.**
   > `mt` no longer detects a transaction that is **well-formed but invalid** —
   > most importantly one carrying a **bad signature**. Such a transaction has a
   > witness present (passes §8.1), balances (passes §8.2b), and carries correct
   > sighash flags (passes §8.6). It engraves cleanly and **fails at broadcast**,
   > which for this artifact means years later, in exactly the situation it was
   > cut for. §7 records this as an accepted hazard.
   >
   > **It also weakens §8.6.** That refusal reasons about whether an input's
   > *satisfaction binds the outputs*, and without a script engine `mt` can only
   > inspect the witness **structurally** — it can see that a stack element is
   > shaped like a signature, not that the script requires one. See §8.6.
   >
   > The upside is real and is why the ruling is defensible: `mt` becomes a tool
   > that parses a PSBT, checks structure and arithmetic, reads two locktime
   > fields and asks a node two questions. That is a far smaller thing to get
   > right than one embedding a consensus engine, and this artifact's other
   > failure modes — the plate being bearer, the destination being stale, the
   > inputs being spent — are ones validity checking never addressed anyway.

2b. **Value-blind acceptance** → refuse. **Now one of the few checks `mt` runs,
   since §8.2 is gone.** `verify_transaction` is a per-input
   *script* loop — read from `consensus/validation.rs` in the `bitcoin` 0.32.101 crate (lines 82-107 of the registry source),
   it iterates `tx.input` calling `verify_script_with_flags` and returns — so it
   never compares input value against output value. Outputs exceeding inputs,
   duplicate inputs and an empty `vin` all pass every other refusal here.
   `mt` must therefore check, at minimum:

   - **inputs ≥ outputs** (`SendingTooMuch`);
   - **an absurdly HIGH fee** — `rust-bitcoin`'s own ceiling is
     `DEFAULT_MAX_FEE_RATE = 25,000 sat/vB`, raised as `AbsurdFeeRate`. This is
     the direction that loses money, and it is what a wrong input value produces
     (§8.2c);
   - **NO minimum fee — but a WARNING below 10 sat/vB.** Operator rulings
     2026-08-23. A refusal floor would hardcode today's relay policy into an
     artifact meant to be broadcast in 2040, the same mistake as engraving a
     dollar figure (§9). `mt` reports the rate and warns:

           WARNING: fee rate is 3.2 sat/vB.

           This transaction may be engraved and then sit for years. A fee has
           to be high enough to motivate a miner AT THE TIME IT IS BROADCAST,
           and nobody knows what that will be. If it turns out too low, the
           holder may need CPFP -- spending one of this transaction's outputs
           with a high-fee child, which needs no key from the signer -- or
           out-of-band submission directly to a miner, which bypasses relay
           policy entirely.

     **The 10 sat/vB threshold is a heuristic and will age**, which is fine here
     for a reason worth stating: it is consumed **at encode time, by a human who
     is present**, and is never engraved. A number that ages is only dangerous
     on steel;
   - **no duplicate outpoints**, and **`vin` non-empty**.

   > **The spec convicts itself here.** §3 rejected the `lean` PSBT form on the
   > grounds that *"the safe API a recoverer reaches for refuses it"*. That API
   > is `extract_tx()`, and it refuses on **three** counts — `MissingInputValue`,
   > `SendingTooMuch` and `AbsurdFeeRate`. §8 adopted the first and ignored the
   > other two while citing the same API as its standard of care.

2c. **Input values: require them when the PSBT lacks them, and WARN whenever a
   legacy input is present.** Operator rulings 2026-08-23: *"Only require user to
   supply utxo values if not part of the psbt"*, and *"Just warn users legacy
   input exists and they will pay a fee equivalent to what is really present at
   the input minus sum of outputs. Explain that this could be very large if they
   are wrong about what the value of the input is."*

   A finalized PSBT in the MIN form normally carries every input's UTXO record
   (§3), so `mt` computes the fee itself and asks for nothing. Where a record is
   absent **from a PSBT**, `mt` requires the operator to supply **that input's
   value, per input** — since §8.2b cannot check the value balance without it.

   > **The two words "from a PSBT" remove a coin-flip that decides whether the
   > pinned vector encodes or refuses — R11 M3.** Read without them, this clause
   > *requires* a value for any input lacking a record, which contradicts
   > §8.2e's *"`mt` never refuses the bytes"* and its `✗` for §8.2b on
   > *raw, no node*. §10.10's table already scopes the refusal to *"when the
   > **PSBT** lacks them"*; this makes §8.2c agree with it in its own body
   > rather than only by cross-reference. **A raw transaction with no node is
   > warned about (§8.2c's legacy path, §8.2e's loud warning), never refused
   > for want of values.**

   > **The alternative "or the total across all inputs" was deleted — R6
   > adversarial I-6 — because it is two rules wearing one sentence.** Two
   > inputs: input 0 is txid-bound at 1.0 BTC (§8.2d), input 1 carries nothing,
   > and the operator supplies *the total*, 2.0 BTC, against 1.99 BTC of
   > outputs. Reading A — the supplied total **is** the input sum — gives a fee
   > of 0.01 BTC. Reading B — it is **added** to the already-bound inputs —
   > gives 1.01 BTC. **The same sentence states both**, one is wrong by an
   > entire input, and which one an implementer picked decides whether §8.2b's
   > `AbsurdFeeRate` and its `inputs ≥ outputs` refusal fire at all.
   >
   > Per-input is what every neighbouring rule already requires — §8.2b, §8.2d,
   > §6a's per-input comparison, and the report's per-input `INPUTS` rows — so
   > the alternative bought nothing and cost a wrong fee.

   **The legacy warning fires only when the value is UNBOUND** — not on every
   legacy input. R3's information lens found the earlier rule actively harmful:
   it fired *"whenever any input is legacy"* while its body asserted `mt` could
   not bind the value by txid, **which §8.2d now does**. In the common case —
   a legacy input carrying `non_witness_utxo`, which BIP-174 requires — that
   printed a false, capitalised, eleven-line block, **training the operator to
   ignore the rare case where it is true.** A warning that cries wolf on the
   normal path has negative value.

   So it fires when, and only when, the value is bound by nothing: no
   `non_witness_utxo` (§8.2d), no chain fetch (§6a). It
   states the mechanism rather than a caution:

       WARNING: input 0 is a legacy (pre-SegWit) input.

       The fee you will pay is:   (what is REALLY at that input) - 0.99000000 BTC
       You have told mt it holds:  1.00000000 BTC
       So mt shows a fee of:       0.01000000 BTC

       NOTHING HAS VERIFIED THAT VALUE. This input carries no
       non_witness_utxo, so mt could not bind it by txid (see 8.2d), and a
       legacy signature does not commit to the amount either. A wrong value
       still produces a perfectly valid transaction -- and the fee absorbs the
       entire difference. If that input actually holds 10 BTC, this transaction
       pays 9.01 BTC in fees and a miner will simply take it.

       Verify the input value out of band before you cut this plate.

   > **`mt` CANNOT put that reminder on a `mt qr` plate, and an earlier draft
   > said it could — R2 lens 2 (S-3), the third recurrence of this class in this
   > artifact.** §7 named *"the engraved out-of-band reminder"* as the
   > mitigation, and §5's legend has **no such field**. So the instruction only
   > lands where the operator controls the plate — **`mt encode`**, whose layout
   > is theirs by ruling (§3b). For **`mt qr`** the
   > legend is `mt`-controlled and full, so the warning reaches the operator on
   > `stderr` **before** they cut and nothing reaches the steel. §7 records that
   > asymmetry rather than claiming a mitigation `mt qr` does not have.
   >
   > **Two stale details were removed from this argument rather than updated —
   > R6 fold-propagation I-5.** It said *"five fields over six lines … with no
   > room for a sixth"*, which §10.21 falsified the same day by adding a sixth
   > field (`FORMAT: mt1 codex32`); and it called
   > the verb **`mt string`**, a name §1.1 renamed to `encode`. Neither carried
   > any weight — the point is that §5 has **no field for this reminder**, which
   > is true at five fields, at six, and at any count §4's deferred cycle
   > settles on. **An argument that names a number it does not need acquires a
   > way to become false for free**, which is exactly what happened here.

   **The output total is the anchor and `mt` knows it with certainty** — it is in
   the transaction. Everything uncertain sits on the other side of the
   subtraction, which is what makes the warning stateable as arithmetic rather
   than as advice.

   > **Why this is the residual hazard §8 cannot close.** The value is not in the
   > transaction; it lives in the already-confirmed previous output. **No miner
   > can alter it and no attacker can inflate the fee that way** — a miner would
   > have to rewrite a block. The entire risk is that the claimed value is wrong,
   > and whether anything catches that depends on the input type:
   >
   > | input | sighash commits to the amount? | a wrong value produces |
   > | --- | --- | --- |
   > | SegWit v0 (BIP-143) | **yes** | an invalid signature — caught by anyone who verifies |
   > | Taproot (BIP-341) | **yes** | an invalid signature — caught |
   > | **legacy** | **no** | **a valid signature and a catastrophic fee** |
   >
   > This is exactly what BIP-143 was written for: *"eliminates the possibility
   > to lie to offline signing devices about the fee of a transaction."* And
   > §8.2's removal widened it — `mt` verifies no signatures at all now, so for a
   > legacy input the claimed value is checked against **nothing by signature**.
   > **§8.2d closes part of this**: where the input carries `non_witness_utxo`,
   > `mt` binds the value by txid, which is a hash comparison rather than script
   > evaluation. The residue this warning exists for is an input whose value
   > arrives with **no** `non_witness_utxo` — supplied by the operator under this
   > refusal — where the warning and the engraved reminder are the whole
   > mitigation.

2d. **`non_witness_utxo` present but not matching the input's txid** → refuse.
   Where a PSBT input carries `non_witness_utxo` — the **whole previous
   transaction**, which BIP-174 requires for legacy inputs — `mt` hashes it and
   requires the result to equal that input's `previous_output.txid`, then reads
   the value from `output[vout]`. A mismatch is a refusal naming both txids.

   **This is a hash comparison, not script evaluation**, so it sits inside the
   2026-08-23 scope ruling (§8.4): `mt` never executes a script, never asks a
   node, and learns nothing about the wallet's policy. Forging a passing value
   would need a txid collision.

   > **Added by R2 lens 1 (F-1), which found the spec asserting this binding
   > without anyone performing it.** §8.6 accepts legacy inputs on the grounds
   > that `non_witness_utxo` *"binds the amount"* — true of the mechanism, and
   > false of `mt` until this refusal existed. An acceptance resting on an
   > unperformed check is the same defect as the original legacy refusal, whose
   > premise was also wrong.
   >
   > **This materially narrows §8.2c's hazard.** A legacy input carrying
   > `non_witness_utxo` now has its value bound by proof-of-work-anchored
   > history rather than by the operator's word. What remains unbound — and what
   > §8.2c's warning still exists for — is an input whose value arrives with
   > **no** `non_witness_utxo` at all, or by operator assertion under §8.2c.

2e. **Which serialisations `mt` accepts, and why all three.** Operator ruling
   2026-08-23, settled by checking what the tools actually hand a user:

   | form | recognised by | where a user gets it |
   | --- | --- | --- |
   | **binary PSBT** | the `psbt\xff` magic | a `.psbt` file from a wallet |
   | **base64 PSBT** | the `cHNidP8` prefix | what wallets export and display |
   | **raw transaction hex** | bare hex, no magic | **Bitcoin Core's default output** — see below |

   **Sniffing is an ORDERED PROCEDURE, not a set of recognisers.** R6
   implementability I-11: the table above is true of the three canonical forms
   and silent about everything a user actually hands a tool. `mt` applies, in
   order:

       1. Read all input. Strip leading and trailing whitespace (including
          CRLF). Do NOT strip interior whitespace yet.
       2. Binary PSBT   -- first 5 bytes are 70 73 62 74 ff.
       3. Otherwise, remove ALL interior whitespace, so line-wrapped exports
          at 64 or 76 columns are handled; then:
          a. base64 PSBT -- begins cHNidP8
          b. raw hex     -- every remaining character is [0-9a-fA-F], case
                            insensitive, an optional 0x prefix is stripped
                            first, and the length is even
       4. Nothing matches -> refuse, naming what was seen (first 8 bytes as
          hex, and the detected length), never a bare "invalid input".

   > **Each numbered step exists because a real user input falls through the
   > previous one.** Line-wrapped base64 is what `openssl`-style exports and
   > many wallets produce; a trailing newline is what every `.psbt` file and
   > every terminal paste carries; uppercase hex and a `0x` prefix are both
   > plausible and neither was mentioned. Under the old text, one implementer
   > accepts a wrapped `.psbt` and another refuses it, **and the refusal is
   > answered by a user who has done nothing wrong.**
   >
   > **Order matters at one place in particular:** binary is tested *before*
   > whitespace removal, because `0x09`, `0x0a` and `0x20` are ordinary bytes
   > inside a binary PSBT and stripping them corrupts it. Everything after
   > step 2 is text.
   >
   > **Hex-encoded PSBT (`70736274ff…`) is the one genuinely ambiguous input**:
   > it is valid hex *and* a PSBT. It matches step 3b, fails to parse as a
   > transaction, and the refusal **must name the real problem** — *"this looks
   > like a hex-encoded PSBT; decode it or pass the `.psbt` file"* — because
   > "invalid transaction" sends the user to look at the wrong thing.

   > **Core's canonical workflow ENDS in hex, which is why refusing it was
   > untenable.** `finalizepsbt` takes `extract` (boolean, **default `true`**):
   > *"If true and the transaction is complete, extract and return the complete
   > transaction in normal network serialization instead of the PSBT."* So the
   > moment a PSBT is finalized — the exact state `mt` requires — **Core stops
   > returning a PSBT and returns hex.** A user must pass `extract=false`
   > explicitly to keep the PSBT form.
   >
   > The earlier PSBT-only ruling would therefore have refused **the default
   > output of the reference implementation**, for the one transaction state
   > this tool exists to consume. That is a stronger reason than "refusing the
   > engraved bytes is unhelpful", and it is why §8.2e accepts hex rather than
   > tolerating it.

   **A raw signed transaction is ACCEPTED, with a loud warning.** Operator
   ruling 2026-08-23: *"we can't refuse raw hex signed tx. We
   have to warn loudly if they paste it and state what we can't verify."*

   **Refusing was the wrong response to someone holding the exact bytes that get
   engraved**, and it was never a special case: a raw transaction is simply the
   **no-UTXO-records** input §8.2c already covers. What degrades is narrow, and
   a node closes most of it:

   | check | PSBT | raw, no node | raw, **node** |
   | --- | --- | --- | --- |
   | §8.1 finalized | ✓ | ✓ | ✓ |
   | §8.6 satisfaction binds outputs | ✓ | ✓ | ✓ |
   | §8.2b value balance | ✓ | **✗** | **✓ via `gettxout`** |
   | the fee | ✓ | **unknown** | **✓** |

       WARNING: this is a raw signed transaction, not a PSBT.

         A raw transaction carries its inputs' OUTPOINTS but not their
         VALUES, so mt cannot compute the fee from it alone.

         [no node]   The fee is UNKNOWN. mt cannot tell you whether it is
                     0.0001 BTC or 9 BTC. Supply input values, or a node.
         [with node] mt fetched each input's value from the chain:
                     fee 0.00012 BTC, 3.2 sat/vB.

   **`mt` never refuses the bytes — it refuses to pretend it checked something
   it did not.** This supersedes the earlier PSBT-only input ruling.

2f. **A PSBT or transaction passed as a COMMAND-LINE ARGUMENT** → **refuse**,
   and tell the operator how to clean up. Operator ruling 2026-08-23.

   **A finalized transaction is a BEARER artifact** — anyone holding it can
   broadcast it, exactly like the plate it becomes. As an argument it lands in
   the shell's history file in plaintext and in `ps` output for every user on
   the machine. `mt` reads from a **file or stdin** only.

       mt encode: refusing a transaction passed as a command-line argument.

         It is now in your shell history and was visible in `ps` while this
         ran. A finalized transaction is BEARER: anyone who reads it can
         broadcast it.

         Get it out of your shell history. Run the recipe for YOUR shell,
         below, and run it in the SHELL THAT LEAKED IT.

         Then re-run:  mt <verb> < file

         TO PURGE WHAT ALREADY LEAKED -- match on the COMMAND, never on the
         secret ...
             zsh:    fc -W; sed -i '/\bmt encode\b/d' "$HISTFILE"; \
                     h=$HISTSIZE; HISTSIZE=0; HISTSIZE=$h; fc -R
             bash:   history -w; sed -i '/\bmt encode\b/d' "$HISTFILE"; \
                     history -c; history -r
             fish:   history clear-session

   **EVERY shell's recipe is printed, and none is detected from `$SHELL`** —
   changed 2026-08-27, P1 row 8, when the recipes moved to the shared
   `mnemonic_io_lib::remedy`. Detection was worse than useless here: `$SHELL`
   is the operator's LOGIN shell and says nothing about the shell that is
   actually holding the leaked entry in memory, which is the only one the
   recipe works in. Printing all three costs four lines and cannot point at
   the wrong one.

   **What was printed before did not work, and that is why this changed.** The
   old text offered zsh `history -d $HISTCMD && fc -W`: on zsh 5.9.2 `-d`
   prints timestamps, so the builtin rejects the invocation and the entry stays
   where it was. It offered fish `history delete --contains <tx>`, which has to
   be handed the material at a prompt that records what is typed — removing one
   copy of the secret by writing a second. Both are RUN, under real interactive
   shells on a pty, in `crates/mt-cli/tests/history_purge.rs`.

   The recipes match on the **command**, never on the secret, and the surface
   is `mt` plus the verb only when one was typed — §8.2f fires before clap, so
   `mt <transaction>` leaks a line with no verb in it, and a pattern fixed at
   `mt encode` would match nothing and purge nothing.

   Two limits stated rather than papered over: it cannot know who read the
   history before now, and it cannot reach backups.

   > **The siblings' precedent does not transfer, and the reason is the whole
   > point.** `md verify <STRINGS>...` and `mk verify [MK1_STRINGS]...` do take
   > their material as positional arguments — but `md1`/`mk1` strings are
   > **watch-only public material**, where a leak costs privacy. A finalized
   > transaction is bearer, where it costs the money. Same shape, different
   > hazard class.

2g. **The source file is readable by anyone but its owner** → **warn loudly.**
   Operator ruling 2026-08-23. `mt` checks `mode & 0o077 == 0` — no group bits,
   no other bits — accepting `600`, `400`, `700` and warning on `644`, `640`,
   `604`.

       WARNING: /home/bcg/tx.psbt is mode 0644 — readable by every user
                on this machine.

         A finalized transaction is BEARER. Anyone who can read this file
         can broadcast it. It is exactly as dangerous as the plate you are
         about to cut.

         chmod 600 /home/bcg/tx.psbt

   **It works in more cases than "a named file", which was worth checking.**
   Verified by experiment: with `mt encode < tx.psbt` an `fstat` on fd 0 still
   returns the underlying file's mode, so the redirect form is checkable too.
   Piped input (`cat … | mt`) gives a FIFO and typed input gives no file — in
   both `mt` says the permissions are **unknown** rather than silently skipping
   the check.

   Two honest limits: it says nothing about who read the file **before** now,
   and nothing about backups or directories it has passed through. It is the
   check that is available, not a guarantee.

2h. **`stdout` is a world-readable FILE** → **refuse**, unless
   `--allow-world-readable`. Operator ruling 2026-08-24, from the Goal 1 journey
   walk (F-244), scoped *"all of `me`, and `mt` too"*.

   §8.2g's other half. `mt` warned in detail that the INPUT file was readable by
   others and then wrote the strings — *the engraving itself* — to a file it
   never mentioned again. The warning fired on **any** redirection and never read
   the mode, so it cried wolf on a `0600` file and warned no harder on a `0644`
   one.

       mt: stdout is a world-readable file, and these strings are BEARER.

         Anyone who can read that file can broadcast this transaction. It is
         the engraving, in a form that copies itself.

           --out <FILE>              mt creates it owner-only (0600)
           umask 077                 then re-run; the shell creates it 0600
           chmod 600 <file>          then re-run — `>` keeps the mode
           --allow-world-readable    proceed anyway

   **WARN ON INPUT, REFUSE ON OUTPUT — the asymmetry is deliberate.** An input
   file's exposure has **already happened**; refusing to read it prevents
   nothing and blocks the operator's work. An output file's exposure is one
   `write` away and has not happened yet, so declining to create it badly is the
   whole of the remedy. **You warn about damage done and refuse damage you are
   about to do.**

   **`mt encode` HAS `--out` — REVERSED 2026-08-27 by §6b of
   `SPEC_constellation_cli_uniformity`, built as P1 row 10.** This paragraph
   used to read *"the remedies are the shell's, because `mt encode` has no
   `--out`"*, and argued from `mt`'s stdout being the strings *by ruling (§3b)*.

   **That citation was checked and §3b does not say it.** §3b rules *which
   stream* carries the artifact — stdout the artifact, stderr what the human
   must see — which is a rule about streams, not about whether a file channel
   exists. So the old sentence contradicted nothing except its own restatement
   of §3b.

   The reason to add the channel is §8.2h itself: `umask` and `chmod` are
   remedies that exist **because** there was no `--out`, and neither creates the
   file owner-only in the run the operator just made. `--out` does, through the
   shared crate's `write_private` — which also tightens a target that **already
   exists**, since `OpenOptions::mode()` binds on create only and re-running a
   command is the case an operator actually hits (F-244).

   **It OVERWRITES**, ruled by the operator 2026-08-26, and **it is on `encode`
   alone**: §6b's reasoning is entirely about the refusal this section defines,
   and that refusal fires from `encode`.

   The shell remedies stay, below `--out`, for an operator who cannot change the
   command line.

   **KEYED ON MODE BITS, NOT ON "is it a file".** `mode & 0o077 == 0` passes.
   **CHARACTER DEVICES are exempt** — a terminal and `/dev/null` persist nothing,
   so neither can leak.

   > **CORRECTED 2026-08-24 by R0 round 0 (finding I3). The first version of this
   > paragraph said `S_ISFIFO` "is not a file" and exempted every FIFO.** Measured
   > false, and the measurement was cited as proof it could not happen:
   >
   > | destination | mode | leaks? |
   > | --- | --- | --- |
   > | anonymous pipe (`\|`) | **0600** | no — the mode test passes it unaided |
   > | **named FIFO** (`mkfifo`) | **0666** | **yes — a third party reading it receives the bytes, verified** |
   > | `/dev/null` | **0666** | no — character device, persists nothing |
   > | regular file | umask-dependent | yes when group/other-readable |
   >
   > So the exemption belongs to **character devices**, not to FIFOs. And it is
   > load-bearing in the other direction too: `/dev/null` is 0666, so a mode-only
   > check with no `S_ISCHR` exemption refuses `mt encode … > /dev/null`.

   **The persistence warning is NOT replaced by this.** A `0600` file still
   outlives the session, so `redirected_output_warning`'s advice to
   `shred -u` it once the plates are cut and verified still fires on every
   redirection. The refusal is **additive**: a new check about *who can read it*,
   beside the existing warning about *how long it lasts*.

   **With `--out` that warning names the FILE and says `mt` made it 0600**, and
   drops the sentence *"stdout is not a terminal"* — which is about a redirect
   that did not happen, since `--out` writes to a file whether or not stdout is
   a terminal. The `shred -u` advice is identical and is written once.

3. **An unsigned or unfinalized transaction offered for engraving** → refuse. It
   cannot be broadcast, so it is not a backup.
4. **Read the locktime FIELDS, compare against the chain if a node is there,
   and warn. Never refuse, never on a flag, and never by reading scripts.**
   Operator rulings 2026-08-23: *"Timelocking happens by user at their wallet
   software… We merely read transaction and warn if immediate"*, and — the scope
   line that decides how this is implemented — *"we can know with certainty if a
   transaction is locked to a specific block. And we can ask `bitcoind`, if
   available, what the current block is. But we are not in the business of
   handing the transaction to `bitcoind` to check validity or reading scripts to
   evaluate for timelocks in the sending wallet's descriptor."*

   **So `mt` reads two FIELDS and asks one question.** Fields are certain;
   scripts are somebody else's job.

   | input | source | certain? |
   | --- | --- | --- |
   | `nLockTime` | transaction field | **yes** |
   | `nSequence`, per input | transaction field | **yes** |
   | current block height | `bitcoind` if reachable, else absent | yes when present |

   **`nLockTime` IS NOT ALWAYS A BLOCK HEIGHT, and an earlier version of this
   section assumed it was.** Verified against source:
   `LOCK_TIME_THRESHOLD: u32 = 500_000_000`. Below that value `nLockTime` is a
   **block height**; at or above it, a **Unix timestamp**. `mt` branches on the
   threshold before it compares anything or engraves anything.

   > **Two failures came from the missing branch — R2 lens 2 (S-4).**
   >
   > 1. **A permanent falsehood on steel.** A transaction with
   >    `nLockTime = 1800000000` would have engraved `LOCKED TO BLOCK
   >    1800000000` — a block number some thirty thousand years out, for a plate
   >    that actually unlocks in 2027. A holder could reasonably read that as
   >    "never" and discard it.
   > 2. **False reassurance, which this section had CLAIMED to close.**
   >    Comparing a *timestamp* against a *height* makes every timestamp look
   >    enormously distant, so `mt` would stay silent about a plate whose
   >    time-lock has **already passed** and which is spendable today. §8.4
   >    asserted that the `nSequence` rule closed false reassurance; this was a
   >    second road to it, needing no script read.

   **`mt` states the two facts and stops.** Operator ruling 2026-08-23:
   *"'may be immediately spendable' is accurate but incomplete. Just say whether
   the transaction is locked to block x and current height is y."*

   So the `stderr` report is a statement of what was read, not a verdict — with
   the units named, never mixed:

       LOCKED TO BLOCK 1383520          current height 963663
       LOCKED UNTIL 2027-03-14T00:00Z   current MTP 2026-08-23T03:00Z
       NO TIMELOCK                      current height 963663
       nLockTime 900000 present but NOT ENFORCED (all inputs final)
       LOCKED TO BLOCK 900000           current height unknown (no node)

   > **THESE FIVE ARE THE `stderr` REPORT'S SPELLINGS. THE LEGEND'S ARE
   > DIFFERENT, AND THIS SECTION NEVER SAID SO — F-239.** Forty lines below,
   > §8.4 rules that *"`NO TIMELOCK` is reserved for a transaction with
   > `nLockTime = 0` **or with all inputs final**"* — so for one state, an
   > unenforced non-zero `nLockTime`, this list says `nLockTime 900000 present
   > but NOT ENFORCED` and that sentence says `NO TIMELOCK`. Both are correct,
   > **on different surfaces**, and nothing stated the split until an
   > implementer had to choose:
   >
   > | surface | spelling | why |
   > | --- | --- | --- |
   > | the `stderr` REPORT | `nLockTime 900000 present but NOT ENFORCED (all inputs final)` | disposable, and can afford the value — it tells the operator WHY the lock does not bind |
   > | the engraved LEGEND | `NO TIMELOCK` | 11 characters, cut into steel, and precisely true about the fields `mt` read |
   >
   > **This is the same two-spellings-one-input class §8.4 calls a real defect**
   > (R6 implementability I-8), landing on §8.4 itself. `mt` implements the split
   > (`Lock::report_row` versus `Lock::legend`).

   **Why facts beat a verdict here.** *"May be immediately spendable"* is true of
   almost any transaction and tells the operator nothing they can act on — it
   cannot distinguish a lock that has already passed from one that was never
   enforced from one still years away, and all three want different responses.
   Two numbers side by side let the operator see which case they are in. It also
   keeps `mt` inside its own scope: a height comparison is arithmetic on fields,
   whereas *"spendable"* is a claim about a transaction's fate that depends on
   scripts, fees and unspent inputs — none of which `mt` evaluates.

   **The block height is MANDATORY, and the estimate names a SEASON.** Operator
   ruling 2026-08-23: *"Estimate year and season (spring, summer, winter, fall)
   and mandate output of blockheight at unlock time."* So the legend always
   carries the raw unlock height — the one figure that is exact, consensus-
   defined, and re-derivable forever — and the estimate rides beside it as an
   orientation aid:

       LOCKED TO BLOCK 1383520 ~SUMMER 2034

   **The height is the fact; the season is the courtesy.** A height alone is
   meaningless to a human (§8.4's original problem) and a season alone is
   unverifiable, so the plate carries both and a reader can always fall back to
   the number.

   > **THE WORKED EXAMPLE SAID `~FALL 2034` UNTIL 2026-08-24, AND THE ALGORITHM
   > ABOVE SAYS SUMMER — F-238, found by implementing it.** Block 1,383,520 is
   > 419,761 blocks past `MT_REF_HEIGHT`, so the projection is
   > `MT_REF_TIME + 419_761 × 600 s` = **2034-08-16**, and August is SUMMER under
   > the northern meteorological quarters this section also rules.
   >
   > **The example was wrong and the rule was right, which is the only ordering
   > that could be corrected safely** — the rule governs every plate and the
   > example is one datum. `mt` implements the rule and pins it
   > (`locktime::tests::the_worked_example_projects_to_summer_not_the_spec_s_fall`,
   > with the season boundaries pinned separately).
   >
   > **It is a MINOR because this section predicted it.** The projection lands
   > **15 days** from the September boundary, against a drift this same paragraph
   > measures at "+16 to −34 days" — so it is exactly the *"projection landing
   > near a season boundary, which can tip"* that the `~` was put there for. The
   > height beside it is unambiguous and settles any dispute, which is why the
   > height is mandatory and the season is the courtesy.

   **Season precision is supported by the measured block rate, and this was
   checked rather than assumed.** Over three windows ending at height 963,759 the
   realised interval was **9.945 to 10.116 min/block** — within ±1.2% of the
   10-minute target — which over the 419,761 blocks of the worked example is
   **+16 to −34 days** of drift. A season is ~91 days, so the error sits inside
   one comfortably. **The exception is a projection landing near a season
   boundary**, which can tip; the `~` marks the whole estimate as approximate and
   the height beside it is what settles any dispute.

   **Seasons are NORTHERN-HEMISPHERE, by ruling.** Operator, 2026-08-23. So
   `SPRING` / `SUMMER` / `FALL` / `WINTER` are the meteorological quarters of the
   northern year — `~FALL 2034` would mean roughly September to November —
   regardless of where the plate is read.

   > **The residual, stated because a plate cannot be asked a question.** A
   > reader in Sydney sees `~SUMMER 2034` and, reading it locally, is wrong by
   > about six months. The harm is bounded and small for one reason: **the
   > mandatory block height sits beside it and is unambiguous everywhere.** The
   > height is the fact and the season is the courtesy, so a misread courtesy
   > costs an orientation, not a recovery. That asymmetry is exactly why the
   > height is mandatory and the estimate is not.

   - **Legend:** `LOCKED TO BLOCK <n> ~<SEASON> <year>` for a height,
     **`LOCKED UNTIL <time>`** for a timestamp, or **`NO TIMELOCK`** — that exact
     spelling, 11 characters, normative everywhere.

     > **This string existed in TWO spellings across four sites — `NO TIMELOCK`
     > and `NO BLOCK TIMELOCK`, 11 versus 17 characters — and §8.4 contradicted
     > itself twice (R5 readiness).** It is **engraved permanently**, so drifting
     > spelling is not a style question: two `mt` versions would cut different
     > plates for the same transaction, and a recoverer matching against
     > documentation would find neither. The 6-character difference also changes
     > what fits the line.

     **A timestamp
     is never presented as a height.**
   - **Compare like with like:** a height against the chain height, a timestamp
     against the chain's **median-time-past** — which §6a's node already
     reports, and which is the monotonic, consensus-enforced figure rather than
     the loosely-constrained header stamp.
   - **Height or MTP comes from `bitcoind` when reachable**, and is reported as
     unknown otherwise. This is the whole of `mt`'s use of the chain here — it never
     hands the transaction to the node for validation.
   **A height means nothing to a human; `mt` estimates the date.** Operator
   ruling 2026-08-23: *"estimate unlock date for time locked transactions
   assuming 10 minute block times. Will need to embed a timestamp in binary for
   reference."*

       estimated unlock  =  reference_time + (target_height − reference_height) × 600 s

   **The estimate uses the embedded constant, and ONLY the embedded constant.**
   Operator rulings 2026-08-23: *"Embed fallback timestamp blockheight in case
   bitcoind not available at compile time"*, then — simplifying — *"Use embedded
   timestamp above only ever. It's essentially constant and reasonably reliable
   as an estimate."*

       MT_REF_HEIGHT = 963_759
       MT_REF_TIME   = 1_787_507_701   // 2026-08-23T17:55:01Z

   **`mt` never consults a node for this.** An earlier draft branched — live
   height when a node was reachable, the constant otherwise — and that was
   removed as too complex. The simplification is worth more than the accuracy it
   costs, for three reasons:

   - **The answer is deterministic.** Two runs of `mt`, on any two machines,
     with or without a node, produce the **same engraved year** for the same
     transaction. Branching would have made a permanent number on steel depend
     on the operator's network.
   - **The accuracy difference is immaterial at this granularity.** The estimate
     is stated to the **year** (below), and a reference pair drifting by even a
     few months moves a projection years out by less than the rounding.
   - **It removes a whole class of question** — what if the node disagrees, what
     if it is syncing, what if it is on another chain — from a number that only
     ever orients a human.

   `MT_REF_TIME` is the tip's **median-time-past**, not its header `nTime`. MTP
   is monotonic and consensus-enforced, while a header stamp is only loosely
   constrained — it may run up to two hours fast and need not exceed its
   parent's. At capture the tip's `nTime` was `1787509876`, **36 minutes ahead**
   of its MTP; small here, unbounded in general, and baking that slack into a
   decades-long projection would be permanent.

   Provenance for whoever refreshes it: block 963,759,
   `00000000000000000000b7060d74b6540e3b2accc9cb50f2a0d428b55911a455`.

   **A NEGATIVE subtraction means the lock is already behind us — warn.**
   Operator ruling: *"If subtraction is negative, warn user transaction is not
   time locked."* When `target_height < MT_REF_HEIGHT` there is no future date to
   estimate, and `mt` says so rather than printing a past year:

       WARNING: nLockTime 900000 is BELOW this build's reference height 963759.
                This transaction is not meaningfully time-locked -- its lock
                height passed before mt was built. Treat it as spendable now.

   **The legend keeps `LOCKED TO BLOCK <n>` and OMITS the `~<SEASON> <year>`
   estimate.** `NO TIMELOCK` is reserved for a transaction with `nLockTime = 0`
   or with all inputs final.

   > **This section said `NO TIMELOCK`, and §8.4 forty lines away said `LOCKED
   > TO BLOCK 900000, current height 963663` — for the same transaction — R6
   > implementability I-8.** Two engraved spellings for one input, in one
   > document, and **§8.4 had already established that class as a real defect**:
   > *"two `mt` versions would cut different plates for the same transaction,
   > and a recoverer matching against documentation would find neither."* The
   > `NO TIMELOCK` spelling was pinned to fix exactly this, and this instance
   > survived the fix.
   >
   > **§8.4's rule wins on the substance, not just on precedence.** `NO
   > TIMELOCK` is a claim about the *fields*, and a transaction with an enforced
   > past `nLockTime` **has one** — saying otherwise engraves something false on
   > steel that outlives the claim. What is meaningless here is only the
   > *estimate*, so only the estimate is dropped: the height is the fact and the
   > year was always the courtesy (§10.23). The `stderr` warning above still
   > fires in full.

   Note this is a **separate** determination from §8.4's `nSequence` rule: a
   locktime can be unenforced *and* in the past, and either alone is enough to
   make the plate immediately broadcastable.

   Reporting the **current height** alongside (§8.4's two facts) is unaffected —
   that comes from the node when one is reachable and is a fact `mt` observed,
   not an input to this estimate.

   **Stated to the year, deliberately.** Three separate reasons, and they all
   point the same way:

   - **Ten minutes is a target, not a rate.** Difficulty retargeting holds the
     average near it over the long run, but the realised interval drifts with
     hashrate between adjustments. Month or day precision would claim accuracy
     the method does not have.
   - **The reference pair ages.** A binary built in 2026 and run in 2031 carries
     a five-year-old anchor, and the error grows with that gap. `mt` prints the
     reference pair alongside the estimate so the operator can see how fresh it
     is, and prefers a live node when one is there.
   - **It is engraved, and engraved numbers are forever.** The legend carries
     `~<year>` with the tilde, because a projection presented as a fact is the
     mistake §9 refuses for fiat figures. The difference that makes a year
     acceptable where a dollar amount is not: block rate is
     **consensus-targeted**, so it depends on nothing external, whereas a
     currency figure depends on everything.

   Measured cost **of this field alone: +6 characters**, and it did not move
   the line count when it landed (130 → 136, 6 lines), so §4's reservation and
   plate table were unaffected by it.

   > **That 136 is this field's own delta, not the legend's size.** Two later
   > rulings moved the total — `BROADCAST` (§5, +4) and `FORMAT: mt1 codex32`
   > (§10.21, +19, which briefly crossed to 7 lines) and `PLATE n OF m` (−12,
   > deleted, which took it back). The live figure is **152 characters, 5
   > fields, 6 lines**, and it is emitted by `legend.rs` rather than carried
   > in prose, because this sentence is exactly how the earlier stale number
   > survived: a per-field delta reads like a total, and nobody re-ran the
   > probe. Read the current size from
   > `RESULTS_legend_budget_2026-08-22.txt`, never from a delta.

   - **A lock that has already passed is reported the same way**, because the
     two numbers say so: `LOCKED TO BLOCK 900000, current height 963663` is a
     plate that is live now, and the operator can read that without `mt`
     concluding it for them.

   **`nSequence` is not optional, and omitting it causes the dangerous error.**
   `nLockTime` is enforced only when at least one input has
   `nSequence != 0xFFFFFFFF`. A transaction with every input final ignores its
   locktime — so reading `nLockTime` alone would engrave `LOCKED TO BLOCK
   900000` on a plate anyone can broadcast today. That is **false reassurance on
   steel**, the worst failure available here, and it is a field read rather than
   a script read, so it stays in scope. `nSequence` appeared nowhere in the
   534-line draft that first specified this rule.

   > **What `mt` therefore CANNOT see, disclosed rather than glossed.** A BIP-68
   > **relative** timelock lives in the witness script as `OP_CSV`, and a
   > relative-locked spend has **`nLockTime = 0`**. Reading it means evaluating
   > the sending wallet's script, which is out of scope by ruling. One of the
   > RCW's own taproot leaves is exactly this — `OP_CSV` with `008000` = 32,768
   > blocks, roughly seven months (`RESULTS_rcw_2026-08-22.txt`).
   >
   > **`mt` will therefore OVER-WARN on such transactions**, which is the safe
   > direction: it says a plate might be spendable when it is not, and the
   > operator — who chose the wallet — can disregard it. The unsafe direction,
   > false reassurance, is closed by the `nSequence` rule above.
   >
   > **This is why the legend states an OBSERVATION, not a conclusion.** An
   > earlier draft engraved `IMMEDIATELY SPENDABLE`, which is a positive claim
   > about spendability that `mt` can no longer substantiate — engrave it on a
   > `OP_CSV`-locked transaction and the steel permanently asserts something
   > false. A `stderr` warning is disposable; a legend line is forever. The
   > legend now reads **`NO TIMELOCK`**: precisely true about the fields
   > `mt` read, and silent about scripts it did not.
5. **`gettxout` returns `null` for any input, AND the parent is confirmed** →
   refuse, when a node is reachable. The output was spent or never existed. See
   §6a's limitation and §10.5 for the IBD case.

   > **The `and the parent is confirmed` clause is not a refinement, it is the
   > difference between a true and a false refusal message — R6 adversarial C-3,
   > second site.** The earlier text refused on `null` alone and told the
   > operator *"the output is spent or never existed."* For a parent sitting
   > unconfirmed in the mempool that is **a false statement of fact inside a
   > refusal**: the output exists and is unspent, and it enters the UTXO set the
   > moment the parent confirms. `include_mempool` is `false` by ruling (§6a),
   > so `null` is the *expected* answer for an unconfirmed parent, not a
   > finding.
   >
   > **A mempool-only parent is a WARNING, not a refusal** — §6a's PENDING
   > state, reported with the same words. Refusing it would block an operator
   > from engraving a transaction whose parent is thirty seconds from
   > confirming, and the refusal would explain itself with something untrue.
6. **Any input whose satisfaction does not bind the outputs** → refuse. Two
   cases, and the previous draft caught only the first:

   a. **A signature with a non-`ALL` sighash.** A `SIGHASH_NONE` input leaves
      the outputs unbound, so a holder — or anyone who photographs the plate —
      can redirect the funds while the signature stays valid, and the legend's
      `TO` line becomes false. `SIGHASH_SINGLE` and `SIGHASH_ANYONECANPAY` are
      refused on the same grounds. Accepted: `SIGHASH_ALL`, and taproot's
      `SIGHASH_DEFAULT`.

   b. **NO signature at all.** R0 round 1 (R-4). The previous rule was written
      over *signatures* and silently assumed every input has one. **A miniscript
      satisfaction need not.** This project's own RCW fixture is the proof: its
      tier 4 was `after(N) AND sha256(H)` — a timelock and a hash preimage, no
      key — until commit `d1889e4` added one, and stock rust-miniscript accepted
      the `wsh` form throughout. An input satisfied by preimage alone commits to
      **nothing**: any holder can rewrite every output and re-satisfy it. That
      is strictly worse than the `SIGHASH_NONE` case (a), which at least binds
      the inputs.

      So the rule is over the **satisfaction**, not the signature: every input
      must carry at least one signature, and every signature must be (a)-clean.

      > **BOTH SPENDING STRUCTURES, not just the witness — R2 lens 2 (S-1).**
      > An earlier version of this refusal named only the **witness**, written
      > when legacy inputs were refused. §10.16 now **accepts** them, and a
      > legacy input's signature lives in the **`scriptSig`**, which that
      > wording never examined — while §8.1 admits such an input by disjunction
      > (*"a non-empty `scriptSig` **or** a non-empty witness"*). So a
      > `SIGHASH_NONE` **legacy** input would have passed every refusal here
      > with its outputs unbound, making §7's *"refused at encode time"* false
      > and the plate redirectable by any holder. `mt` inspects **`scriptSig`
      > and witness alike**, applying (a) and (b) to whichever carries the
      > satisfaction.
      >
      > **The structural recognizer is AMBIGUOUS, and the fixture in this repo
      > proves it — R2 lens 2.** A Schnorr signature carrying an explicit sighash
      > byte is **65 bytes**; a BIP-341 control block is `33 + 32m`, so at
      > `m = 1` it is also **65 bytes**. They are indistinguishable by length.
      > The RCW's own taproot witness measures
      > `[64, 64, 64, 32, 143, 65]` (`RESULTS_rcw_2026-08-22.txt`) — three
      > signatures, a preimage, a 143-byte leaf script, and that trailing **65 is
      > a control block**, not a signature.
      >
      > So (b)'s *"every input must carry at least one signature"* is
      > **grindable**: a keyless leaf spent at depth 1 yields
      > `[preimage, script, control-block(65)]`, and a length-based recognizer
      > counts the control block as the signature it is looking for. `mt` must
      > therefore recognise a taproot script-path witness **by shape** — last
      > element is the control block, second-last the leaf script — and count
      > signatures only among the remaining elements. **This is still a
      > heuristic and the spec does not claim otherwise.**
      >
      > **Limited by §8.2's removal.** Without a script engine `mt` inspects the
      > spending structure **structurally** — it can tell that a stack element is
      > *shaped* like a signature (a 64-byte Schnorr element, or a DER-encoded
      > ECDSA one with a trailing sighash byte), but not that the script it
      > satisfies actually **requires** one. A crafted witness carrying a
      > signature-shaped element that the script never checks would pass. This
      > is a structural heuristic, not a proof, and the spec should not claim
      > more than that.

   > **Legacy inputs are ACCEPTED. Operator ruling 2026-08-23:** *"Do not
   > exclude legacy inputs. It is user responsibility to know their inputs for
   > such edge cases."* The previous draft refused them, and its stated reason
   > was false: it claimed a legacy amount is unverifiable because the sighash
   > does not commit to it. The first clause is true; the conclusion does not
   > follow, since BIP-174 requires `non_witness_utxo` for a legacy input —
   > the **whole previous transaction** — so hashing it and matching the txid
   > binds the amount without any help from the sighash. **§8.2d makes `mt`
   > actually perform that check**; without it, this justification would assert a
   > binding nobody computes, which is the same defect as the premise it
   > replaced.
   >
   > `sh(wsh(…))` is therefore no longer an unclassified case: wrapped-segwit
   > inputs are segwit inputs, and every input type is accepted.

7. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** The `mt qr` plate-budget
   refusal. It cannot fire in v0.1 — the verb that would trip it is deferred (§0a)
   — and it was moreover **unrunnable as written**: R6 found its threshold has no
   input path, and a refusal whose threshold cannot be supplied is not a refusal.
   The number is kept so §8.7b's base and every citation still resolve.

7b. **Over the 32,768-chunk ceiling** → refuse, naming the chunk count and the
   ceiling. Both verbs share it, since both use `mt1`'s header (§3).

   > **This refusal is deliberately unreachable for anything broadcastable.**
   32,768 chunks is 1,310,720 bytes, and Bitcoin's own standardness limit is
   ~100,000 vbytes — so a transaction large enough to trip this **could not be
   relayed even if `mt` engraved it** (§3). It exists for completeness, not as a
   working constraint. For scale: the largest artifact measured in §3b is
   **89 chunks, 2.2% of the ceiling.**
   >
   > **An earlier version of this refusal said "over the 64-chunk container"**
   > and cited that same 89-chunk artifact as a wallet that *"hit this"*. Both
   > were wrong: 64 was `md-codec`'s 6-bit field, never `mt1`'s (§3's
   > correction), and at 32,768 the artifact is nowhere near the limit. It also
   > pointed at `mt qr` *"which has no such limit"* — both verbs share it.
7c. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** The `sysw` section-ceiling
   refusal (`MAX_SECTION_LEN = 8191`). Deferred with the verb (§0a); no v0.1
   behaviour depends on it, and it still **cannot carry a number** until the
   record framing is chosen — four candidate framings give four ceilings.

8. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** Module size and its 0.60 mm
   default. **Engraving geometry only `mt qr` has** — `mt encode` emits
   characters and reserves no area at all — and it sat in this list beside live
   refusals, reading as v0.1 behaviour. Same shape as §8.7 and §8.7c, which the
   first sweep caught and this one did not. The number is kept so §8.9 keeps its
   place and citations resolve.
9. **Secrets** → refuse, as `me` already does for `ms1`.

> **What §8 does NOT check, enumerated because §8.2's removal made the list
> longer and nothing else states it — R2 lens 2.** These are **commitment
> checks**: one hash each, no script engine needed, and `mt` performs none of
> them.
>
> | unchecked | what it would catch |
> | --- | --- |
> | **script-hash** — does the revealed `witnessScript` hash to the `scriptPubKey`? | a witness script that is not the one being spent |
> | **taproot tweak** — does the internal key + merkle root tweak to the output key? | a control block that does not belong to this output |
> | **k-of-n sufficiency** — are there enough signatures for the policy? | an under-signed multisig that will never validate |
>
> Each is cheap and none is script *evaluation* in the sense §8.4's scope ruling
> excludes — they are hashes over data already in the PSBT. They are listed here
> rather than implemented because adding refusals is the operator's call, and
> because §8.2's removal was itself a ruling that `mt` does not verify
> validity. **The consequence stands either way: a transaction can fail every
> one of these and still be engraved.**

Every refusal names the number that caused it. A refusal that says only "too
large" costs the operator a round trip.

## 9. Out of scope for v0.1

**Transaction construction, and PSBT presentation to a signing device** — both
removed by operator ruling 2026-08-23 (§0). Coin selection, fee estimation,
change handling and input selection go with them: they are wallet decisions with
their own failure modes, they are better tested in wallet software before
anything is engraved, and folding them in would make `mt` a wallet.

**`mt qr` IS OUT OF SCOPE FOR v0.1** — deferred to a cross-format QR cycle
(§0a), taking §4, §5, the `sysw` transaction `Class`, the record framing and
§10.17's firmware work with it.

> **CORRECTION — an earlier version of this section said "a decoder is out of
> scope for v0.1" and that a plate cut by `mt` v0.1 could not be read back by
> `mt` v0.1. Operator ruling 2026-08-23 reverses it: `mt decode` ships in
> v0.1.**
>
> The claim was written when "reading a plate" meant §10.2's **static-scan**
> verb — a camera pointed at engraved QR symbols. Two things make that framing
> wrong now. `mt qr` is deferred (§0a), so v0.1 engraves **characters**, not
> symbols; and reassembling `mt1` chunks into a transaction **needs no scanner
> and no camera at all** — it takes strings a human typed or pasted. The
> obstacle I described was never in the way of the thing that matters.
>
> **A format whose own tool cannot read its own output is not falsifiable**, and
> both siblings have a decoder — `md decode`, `mk decode`. `mt` shipping without
> one would have been the anomaly.

**What IS still out of scope: reading a plate OPTICALLY.** §10.2's static-scan
verb — camera, symbol detection, reassembly from images — is deferred with
`mt qr` (§0a), because there are no engraved symbols in v0.1 to scan. That
leaves one real gap — and **§10.21 closed the version of it stated here, while
the hazard survives on other grounds (R6 fold-propagation I-7).** §5 now
suggests a `FORMAT: mt1 codex32` line, so the legend *has* a field naming the
encoding. But §0a rules that **the realistic plate carries no legend at all** —
every line of it is optional, hand-cut, and paid for in the operator's own
labour — so a recoverer may still hold steel that names nothing.

**The gap moved from "the spec has no such field" to "the operator probably did
not cut it", which is not an argument for deleting this entry.** A suggestion
`mt` prints and an operator skips leaves exactly the hazard the original
sentence described; what changed is that `mt` has now done everything it can
about it. `mt1…` identifies the string to someone who already knows
the constellation; it says nothing to someone who does not.

Also out: signing; broadcasting; RBF or CPFP; watching the chain to detect
invalidation after engraving; any machine-readable provenance (ruled: legend
only); sealed or encrypted plates; and Merkle inclusion proofs of input
existence (`gettxoutproof`) with the block-work and timestamp framings that went
with them — removed 2026-08-23, since they existed to establish trust in input
amounts offline for *construction*, and the amounts now arrive bound inside a
signed PSBT.

## 10. Open questions

1. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** The F-234 optical test plate has not been cut — and the module size is now. `mt qr`
   material, deferred with the verb (§0a). The number is kept so citations
   to §10.1 from elsewhere in this document, and from commit messages, keep
   resolving.

2. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** Will a wallet reassemble multi-part UR from STATIC symbols? OUT OF. `mt qr`
   material, deferred with the verb (§0a). The number is kept so citations
   to §10.2 from elsewhere in this document, and from commit messages, keep
   resolving.

3. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** Is UR worth its expansion? What goes in the QR? CLOSED. UR is. `mt qr`
   material, deferred with the verb (§0a). The number is kept so citations
   to §10.3 from elsewhere in this document, and from commit messages, keep
   resolving.

4. **SETTLED** — The legend's FROM and TO fields. Reasoning in §12.4.

5. **SETTLED** — Should mt require the node to be out of IBD before trusting. Reasoning in §12.5.

6. **SETTLED** — How much fountain redundancy. Reasoning in §12.6.

7. **SETTLED** — Back-side engraving. Reasoning in §12.7.

8. **SETTLED** — How does a recoverer learn the fragment parameters. Reasoning in §12.8.

9. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** How does the engraving reach the machine? ANSWERED, operator ruling. `mt qr`
   material, deferred with the verb (§0a). The number is kept so citations
   to §10.9 from elsewhere in this document, and from commit messages, keep
   resolving.

10. **The CLI surface — RULED.** Operator rulings 2026-08-23.

    | | |
    | --- | --- |
    | verbs | **`encode`**, `decode`, `verify`, `inspect` — matching `md` and `mk` |
    | **input** | a finalized PSBT (preferred) **or a raw signed transaction** (§8.2e) — from a **file or stdin**, never a command-line argument (§8.2f) |
    | `mt encode` output | the **codex32 string on stdout**, lowercase, ungrouped — hand engraving |
    | stderr | every warning and refusal a human must see (§3b) |

    > **A `mt qr` output row sat in this table until 2026-08-23, and the table
    > CONTRADICTED ITSELF:** the `verbs` row lists `encode`, `decode`, `verify`,
    > `inspect` — `mt qr` is not among them — while the row below described what
    > `mt qr` emits. **A CLI surface cannot describe the output of a verb it
    > does not offer.** The row moved to `design/SPEC_mt_qr_DEFERRED.md` with
    > the rest of the deferred material.
    | flags | **none for locktime** (§8.4) |

    **Why a PSBT is PREFERRED — not required. §8.2e superseded the PSBT-only
    ruling and this heading still asserted it (R6, two lenses).** Raw signed
    transaction hex **is accepted**, loudly warned, with `mt` stating what it
    cannot verify: the operator ruling was *"we can't refuse raw hex signed tx.
    We have to warn loudly if they paste it and state what we can't verify."*
    Bitcoin Core's `finalizepsbt` defaults `extract=true` and returns hex, so a
    PSBT-only rule would refuse **the default output of the standard tool**.

    Input format and payload format remain independent, and the table below is
    why a PSBT is worth preferring: §8 is written in PSBT vocabulary and
    degrades unevenly without one. **Read it as the cost of pasting hex, not as
    a refusal.**

    | refusal | finalized PSBT | raw signed transaction |
    | --- | --- | --- |
    | §8.1 finalized? | reads `PSBT_IN_FINAL_*` | reads scriptSig/witness — **works** |
    | ~~§8.2 script-valid?~~ | *removed from v0.1* | *removed from v0.1* |
    | §8.2b value balance? | UTXO records give input values | **needs a node or operator-supplied values** — a raw transaction carries no input amounts, and §6a's `gettxout` supplies them when one is reachable (§8.2c otherwise) |
    | §8.6 satisfaction binds outputs? | parses the witness | parses the witness — **works** |

    So accepting raw hex would **silently disable two refusals**, including the
    only check that inputs ≥ outputs, while the artifact looked identical. `mt`
    therefore *prefers* a PSBT (§8.2e supersedes the requirement), runs the full
    refusal set against it, and then —
    for `mt encode` — extracts the raw transaction as the payload. Nothing is
    lost: a PSBT is what wallet software emits at the point this workflow
    starts, which is exactly the *"test it in your wallet first"* flow §0 is
    built around.

> **A row was removed here on 2026-08-23, and it had gone stale twice.** The
> report carried *"the headroom — chunks against 64 (`mt encode`) or characters
> against 8,191 (`mt qr`)"*. **Both ceilings it named are gone**: 64 was never
> `mt1`'s (§3's correction — it was `md-codec`'s 6-bit field), and 8,191 is
> `sysw`'s, deferred with `mt qr` (§0a). It survived both corrections because
> each fixed a *ceiling* and neither re-read the row that cited it.
>
> **It would not have earned its place even with the right number.** Against
> `mt1`'s real 32,768-chunk ceiling the worst measured artifact is **2.2%** and
> the pathological wallet's descriptor is 0.6%, so the row would report ~98%
> headroom every time — a figure that never varies and never informs. A ceiling
> is worth reporting only where it binds, and this one binds at sizes nobody
> hand-engraves. It returns for the QR cycle, where `sysw`'s 8,191 *does* bind
> at realistic sizes.

    **The SUCCESS-PATH REPORT is `inspect`'s, and `encode` calls it** (§1's
    verb rulings). What follows is the report's content; the ownership rule is
    that `encode` invokes `inspect` rather than composing a second copy, so the
    operator's pre-engraving view and a recoverer's later `mt inspect` cannot
    disagree.

    **`mt` was specified silent when nothing is wrong.** R3's information lens (I-1) found that stdout carries the artifact
    and stderr carries warnings and refusals, so the fee, the string count, the
    configuration and **the outputs themselves had no channel at all** — while
    §5 and §7 both justify `TO` being an optional one-line summary on the
    grounds that *"`mt` prints every output in full at encode time."* Nothing
    defined that printing.

    **It goes to `stderr`, with the warnings**, because stdout is the artifact
    and writing a report there would corrupt `mt encode`'s output. Before any
    plate is cut, `mt` reports:

    | | |
    | --- | --- |
    | **every output** | address in full, amount, and which are change if a wallet was supplied |
    | **the fee** | absolute and as sat/vB — the number §8.2b's warning thresholds refer to, printed whether or not a warning fires |
    | **the locktime** | §8.4's two facts |
    | ~~the plate count~~ | **DELETED 2026-08-23** — `mt` cannot see how strings are laid onto steel, so it has no plate count and cannot price engraving time in plates |
    | **the configuration** (`mt qr` only, deferred) | module size, QR version, ECC level, symbol count — §4's answer |
    | **the engraving size** | how many strings to cut and **how many characters in total** — the unit the person doing the cutting actually experiences |
    | **the set prefix** | the **first 8 characters after `mt1`**, shared by every string in this set, with the rule stated — see below |
    | **the value provenance** | per input: chain-fetched (§6a), txid-bound (§8.2d), or operator-asserted (§8.2c) |

    ~~**THE SPEC NAMES THREE FLAGS while requiring SEVEN operator inputs**~~
    **CLOSED 2026-08-23 — twelve flags are ruled in the table above, and the
    node location stopped being an input at all.** The finding and its history
    are kept below because the *lesson* outlived the gap.

    > **The original wording was "ZERO FLAGS", and it was falsified by a
    > one-second `grep` the sentence itself prescribes.** `--transaction`
    > (§1.1's `verify`) and `--quiet` (§1.1a's `decode`) were both added by
    > rulings on 2026-08-23, *after* this claim was written. **An absence-claim
    > is only as wide as the search that produced it**, and this one outlived
    > its own search by a day — the standing lesson about negatives inheriting
    > their scope, landing on a sentence that names the command to re-run.
    >
    > The finding it supports is **unchanged and still open**: three flags
    > (`--transaction`, `--quiet`, `--elide-prefix`) is not seven, so the gap is
    > 4 rather than 7. Most consequentially
    **§8.7's plate budget has no input at all**, which makes that numbered
    refusal unrunnable as written: a refusal whose threshold cannot be supplied
    is not a refusal.

    The inputs `mt` needs, and which section needs them:

    | input | needed by | absent → |
    | --- | --- | --- |
    | the PSBT | everything | refuse |
    | ~~plate budget~~ | ~~§8.7~~ | **DELETED 2026-08-23** — §8.7 moved to the deferred QR spec; `mt` cannot see how strings map to steel |
    | `FROM` wallet id / fingerprint | §5 | warn, engrave blank |
    | `TO` wallet id / fingerprint | §5 | warn, engrave blank |
    | `TO` free-text label | §10.4 | **requires an explicit flag** by ruling |
    | input values | §8.2c, when the PSBT lacks them | refuse |
    | ~~module size~~ | ~~§8.8~~ | **DELETED 2026-08-23** — `mt qr` only, deferred; `mt encode` has no geometry to configure |
    | ~~node location~~ | ~~§6a~~ | **NOT AN INPUT** — `mt` shells out to `bitcoin-cli -stdin`, which already holds it. See the ruling below |

    **Naming them is a prerequisite for implementation, not a nicety**: two
    implementers given this table will still choose different flag *spellings*,
    but they will at least build the same tool. Given different tables they build
    different tools.

    **A TTY on stdin gets a welcome line, not silence.** Operator ruling
    2026-08-23. `mt encode` with nothing piped in **blocks waiting on stdin**,
    and to anyone who does not know the paste-then-Ctrl-D idiom that is
    indistinguishable from a hang: no output, no prompt, no cursor movement. The
    natural response is Ctrl-C and the conclusion that the tool is broken.

        mt encode: reading a transaction from stdin.
                   Paste it and press Ctrl-D, or Ctrl-C to abort.

    **The test is one line** — stdin is a TTY rather than a pipe — and it is the
    same check that tells `mt` a paste is coming rather than a redirect. The
    failure it prevents is not a wrong result but **a new user concluding the
    tool does not work and leaving**, which no other check catches.

    It is also the one place `mt` would otherwise stop doing what it does
    everywhere else: §8.2c states the fee arithmetic, §8.4 states two facts,
    §6a enumerates the skipped checks. **A tool that silently waits is the
    exception.**

    **Unrecognised input is NAMED, not merely rejected.** `me` already has a
    `classify` module and `md`/`mk` classify their input too, so this is the
    constellation's habit rather than a new idea. A txid is 64 hex characters
    and recognisable as such:

        mt encode: that is a transaction ID (a 64-character hash), not a
                   transaction. mt needs the transaction itself — a txid
                   identifies one, it does not contain one.

    **The SET PREFIX row, and why it is a row rather than a footnote.**
    Operator ruling 2026-08-23. `mt1`'s header packs its invariant fields first
    — `version(5) + chunk_set_id(20) + count(15)` — so bits 0–39 are identical
    across every chunk of a set, and at 5 bits per symbol **the first 8
    characters after `mt1` are the same on all of them**. Only `index` varies,
    in the 3 characters that follow.

    > **8, not 7, and now EXACT — recomputed from the layout.** 40 invariant
    > bits give `40 / 5 = 8` whole symbols with nothing left over, where the
    > previous layouts had 37 and 38 invariant bits and left the eighth
    > character straddling the `index` boundary. **That exactness is what lets a
    > hand engraver stop repeating the header** (§10.13 a2): there is a clean
    > character boundary to elide at.

    **Verified on real output rather than derived**: the four `md1` chunks of
    this repo's pathological wallet all read `md1fveszps…`.

        All 14 strings begin `mt1qzrf8xk2`. Strings sharing that prefix belong
        to this transaction; strings that do not, do not.

    **This is the only grouping rule a recoverer can apply without software.**
    They may hold engravings from two transactions, or part of a set whose
    siblings are elsewhere, and the prefix separates them **by eye** — no
    decoding, no checksum, no tool. It costs one line at encode time and hands
    the 2040 reader a rule they would otherwise have to be told by someone who
    is not there.

    **Input and output serialisations are now settled** — three accepted input
    forms (§8.2e) and raw hex out of `decode` (decision 1a in §1).

    **THE TWO BEHAVIOURAL QUESTIONS ARE RULED HERE; ONLY SPELLINGS REMAIN
    OPEN.** R6 implementability I-10 drew the distinction and it is the right
    one: this section anticipated the objection with *"two implementers given
    this table will still choose different flag spellings, but they will at
    least build the same tool"* — **true of spellings, false of the two below,
    which are behaviour.**

    a. **Grouping affects `stdout`, and the CANONICAL artifact is the UNGROUPED
       string.** `decode` and `verify` accept both (§1.1e splits then strips).
       Without this, an operator who asked for grouping gets spaces inside the
       stream §0a declares to be the artifact, and every downstream consumer
       must strip them — including `mt qr` when that lands. Grouping is a
       hand-engraving courtesy, so it is **opt-in and never the default**.

    b1. **`mt` REACHES A NODE BY SHELLING OUT TO `bitcoin-cli -stdin`, NOT BY
       SPEAKING JSON-RPC.** Operator ruling 2026-08-23 — *"Don't you just do
       bitcoin-cli command?"* — which replaces the proposed `--rpc <url>` flag.

       > **It makes §6a's "the operator is asked for nothing" true by
       > construction.** `bitcoin-cli` already holds the RPC URL, the cookie or
       > credentials, the network, the datadir and the wallet selection, from
       > `bitcoin.conf` and its own defaults. **`mt` works exactly when the
       > operator's node works** — there is no second place to configure, and no
       > way for `mt`'s idea of the node to drift from `bitcoin-cli`'s.
       >
       > A `--rpc <url>` flag would have re-asked for information the operator
       > has already given their node once, and would have obliged `mt` to
       > handle cookie files and RPC auth — a surface it has no other reason to
       > have.
       >
       > **`-stdin` is not optional, and the reason is §8.2f.** Arguments go on
       > **stdin, one per line**, never on the command line: a `bitcoin-cli
       > gettxout <txid> 0 false` invocation puts the txid in `ps` for every
       > user on the machine. That is the same leak §8.2f refuses for
       > transactions — smaller, since a txid is not a bearer instrument, but
       > free to avoid. `bitcoin-cli`'s own help calls `-stdin` *"recommended
       > for sensitive information"*.
       >
       > **Verified against a live node before ruling**, not inferred:
       >
       >     printf 'getblockcount\n' | bitcoin-cli -stdin          -> 963808
       >     printf 'gettxout\n<txid>\n0\nfalse\n' | bitcoin-cli -stdin  -> empty, exit 0
       >
       > The second is the shape §6a keys on: a spent or nonexistent output
       > yields **empty output and exit 0**, which is the `null` this spec's
       > liveness table reads.
       >
       > **The only flag is `--bitcoin-cli <path>`**, for a binary not on
       > `PATH`. Absent and not found → §6a's no-node warning, unchanged.

    b2. **`--elide-prefix` (`mt encode`) emits the set's invariant 8 characters
       on the first string only** (§3b). It changes the **display** form, never
       the wire form: `decode` and `verify` restore the prefix before parsing,
       and accept full, elided and mixed input with no flag of their own.

    b. **`--quiet` suppresses the INSPECTION REPORT only. Warnings and refusals
       are NEVER suppressed**, on any verb. It was defined only for `decode`,
       and its scope was the open question: a `--quiet` that silenced §8's
       warnings would let a script engrave a plate whose hazards nobody saw,
       which is the opposite of what §0a's stderr split is for. It does **not**
       relax §1.1a's rule that stdout stays empty unless every check passes.

    **THE FLAG SPELLINGS — RULED 2026-08-23.** Operator-approved; sibling
    precedent taken verbatim wherever it exists, so an operator who knows `md`
    does not learn a second dialect for the same concept.

    | flag | takes | serves |
    | --- | --- | --- |
    | `--in <path>` | a path; **stdin** when absent | the PSBT or raw transaction (§8.2e). Never a command-line argument (§8.2f) |
    | `--from <id>` | wallet id or fingerprint | §5's `FROM` field |
    | `--to <id>` | wallet id or fingerprint | §5's `TO` field |
    | `--to-label <text>` | free text | §10.4 — **a separate flag IS the ruling**: it makes the label an act of assertion rather than something that quietly appears |
    | `--input-value <index>:<amount>` | **repeatable, per input** | §8.2c. Per-input because a single total has two readings that differ by a whole input — deleted as a defect earlier the same day |
    | `--group-size <n>` | count | grouping for hand engraving (§1.1e). **`md`'s spelling, unchanged** |
    | `--separator <s>` | string | grouping separator. **`md`'s spelling, unchanged** |
    | `--elide-prefix` | — | §3b |
    | `--quiet` | — | suppresses the inspection report only; never warnings or refusals |
    | `--transaction <psbt\|hex>` | on `verify` | §1.1's full-txid comparison |
    | `--json` | — | machine-readable report. `md` has it |
    | `--bitcoin-cli <path>` | a path | only for a binary not on `PATH` — see (b1) |

    > **Two spellings are deliberate departures, recorded so they are not read
    > as oversights.** `md verify` calls the equivalent of `--transaction`
    > `--template`; `mt`'s argument **is** a transaction rather than a template,
    > so the name follows the thing rather than the sibling. And there is **no
    > `--rpc`** — (b1) rules that `bitcoin-cli` already holds the node's
    > location.

    **Still unspecified, and deliberately: exit codes beyond 0.** `0 = every
    check passed` is fixed in (b); the rest of the code space is implementation's.
    **The refusal-message format is RULED** — §8's preamble, three parts, with a
    machine-parseable verdict line. It was declared unspecified here while being
    the one item P5's tests assert against (R11 I4). Each input the table above requires needs some flag —
    input values per input (§8.2c), `FROM`/`TO` identities, and the free-text
    `TO` label behind its own flag (§10.4). **The node location is NOT among
    them** — (b1) deleted it, because `bitcoin-cli` already holds it. Naming them
    is a CLI-design task, not a codec one. **Exit codes are the exception that
    should not wait**: §1.1a's documented pipeline now depends on a non-zero
    exit, so `0 = every check passed` is fixed here and the rest of the code
    space is left to implementation.

11. **SETTLED** — How many codex32 characters fit a hand-engraved plate. Reasoning in §12.11.

12. **SETTLED** — Should mt1 FILL its chunks rather than balance them. Reasoning in §12.12.

13. **`mt1`'s own encoding, NUMS constant and content id — RULED, ready to
    build.** Operator rulings 2026-08-23.

    R0 round 1 (S-2) read `md-codec` directly: the header *layout* (37 bits),
    chunk ordering, gap detection and missing-chunk checks are payload-agnostic
    and take a transaction cleanly. Three things do not transfer, and all three
    are now decided:

    **(a) Its own NUMS constant — RULED, operator 2026-08-23:**

        domain string    : "shibbolethnumstransaction"
        MT_REGULAR_CONST = 0x1a2fc877f9528d7c1

    the **top 65 bits of `SHA-256("shibbolethnumstransaction")`**, following the
    constellation's rule exactly — `md1` uses `"shibbolethnums"`, `mk1` uses
    `"shibbolethnumskey"`, each appending its distinguishing noun spelled out.
    **Recomputed independently before folding**: SHA-256 is
    `d17e43bfca946be09034ac97e7950cdd50d3b5a3e3cf4bad5cb65516897978f6`, the top
    65 bits are `0x1a2fc877f9528d7c1`, the value occupies exactly 65 bits, and it
    differs from both constants already in use.

    `MD_REGULAR_CONST` is hardcoded into checksum create and verify
    (`crates/md-codec/src/bch.rs`), so every constellation format needs its own.

    > **THIS PARAGRAPH SAID A DISTINCT CONSTANT PREVENTS "an `mt1` chunk
    > verifying as a valid `md1` chunk". IT DOES NOT, AND NOTHING NEEDS TO —
    > R9 B-3.** The HRP is mixed into the polymod on both sides, so
    > `hrp_expand("mt") ≠ hrp_expand("md")` separates the formats **by
    > themselves**, whatever the constants are. Cross-format acceptance is
    > unreachable while the HRPs differ, and cross-format verification is
    > abandoned by operator ruling.
    >
    > **The correction was made in §12.22 and not here, and that is the defect
    > worth recording.** §12.22 is the *historical appendix*; **this** is the
    > normative "RULED, ready to build" section an implementer works from. So
    > the retraction lived in the record of what was decided while the live text
    > still asserted the falsehood — and the finding was marked closed, which
    > made it invisible. **Fixing the wrong location is worse than not fixing
    > it.**
    >
    > **The real reason `mt1` needs its own constant is intra-format.** A wrong
    > constant — copied *or* mistyped — produces chunks that are
    > **self-consistent and unreadable by every other implementation**, and it
    > surfaces at *recovery*, indistinguishable from steel damage: checksum
    > failures on a physically perfect plate, years later, with no second copy
    > of the transaction anywhere. That is a real hazard, and it is not the one
    > this paragraph used to name.

    **(b) Its own HRP — the string is `"mt"`, NOT `"mt1"`.** The `1` in a
    rendered `mt1…` string is bech32's **separator**, not part of the HRP.
    `md-codec` makes this explicit: `const HRP: &str = "md"`
    (`crates/md-codec/src/codex32.rs:15`) while its strings render as `md1…`,
    and the checksum is computed over `hrp_expand("md")`
    (`crates/md-codec/src/chunk.rs:565,615`).

    > **R4 filed this as a MINOR and the R4 fold skipped it; R5 found it makes
    > plates MUTUALLY UNVERIFIABLE.** An implementer reading "its own HRP, `mt1`"
    > would compute `hrp_expand("mt1")`, producing a different polymod residue —
    > so every plate written by one implementation fails the other's checksum,
    > and fails it with a *"damaged beyond correction"* diagnostic that points
    > the recoverer at their steel rather than at their software. **Triage by
    > severity label is what let this through**: the finding was correct and its
    > label was wrong, and I folded by label.

    **(a2) The header's exact layout, because R4 found five things an
    implementer would otherwise guess — and two of the guesses produce plates
    another implementation cannot read.** `mt1`'s **55 bits — exactly 11 bech32
    symbols, and EVERY FIELD a whole number of symbols** — are, in order:

    | field | bits | value |
    | --- | --- | --- |
    | `version` | **5** = 1 symbol | **`0b00001`** — `mt1` wire v1. Not inherited from `md1` |
    | `chunk_set_id` | **20** = 4 symbols | top 20 bits of the extracted txid, display form (c) |
    | `count` | **15** = 3 symbols | **`count − 1`**, matching `md-codec`'s offset convention: a set of 1 stores `0`, a set of 32,768 stores `32767` |
    | `index` | **15** = 3 symbols | **plain, zero-based**, `index < count` |

    **THE HEADER IS SYMBOL-ALIGNED PER FIELD: 55 BITS = 11 SYMBOLS, AND NO
    FIELD STRADDLES A CHARACTER.** Operator ruling 2026-08-23. This supersedes
    the same day's 50-bit ruling, which aligned the *total* while leaving every
    field after `version` spanning a boundary.

    > **`md1` is NOT aligned — 37 bits — and nothing in `md` ever justified
    > that.** Checked rather than assumed: every place the number appears it is
    > stated as arithmetic and never as a decision (`SPEC_v0_30_wire_format.md`
    > *"Total chunk header = 37 bits"*, `crates/md-codec/src/chunk.rs:6`).
    > Its costs, by contrast, **are** documented as things `md` wishes it could
    > redo — `md`'s `design/FOLLOWUPS.md` item 10 wants a v2 layout, and another
    > entry notes the misalignment is absorbed by trailing-zero padding that a
    > length prefix would make unnecessary. **A documented cost, a recorded wish
    > to undo it, and no recorded benefit.**
    >
    > **THE DEAD `chunked` BIT IS DELETED, and it was what broke alignment.** A
    > 1-bit field at offset 5 pushes every later field off a character boundary.
    > It was retained on the argument that dropping it *"keeps the layout
    > identical to the format `mt1` forked from"* — **an argument already void**,
    > since `version` (4 → 5) and `count` (6 → 15) had both diverged. `mt1` is
    > *always* chunked, so the bit encoded nothing; the `version` field alone
    > identifies the format generation. Removing it also removes the temptation
    > the retention argument feared: there is no longer a dead bit for a
    > thoughtful implementer to drop unilaterally.
    >
    > **WHAT PER-FIELD ALIGNMENT BUYS, and the operator's reason is the one that
    > decided it.** The invariant fields — `version + chunk_set_id + count` — are
    > **40 bits = exactly 8 symbols**, and `index` is **15 bits = exactly 3**. So
    > every engraved string reads:
    >
    >     mt1 | <8 symbols identical across the set> | <3 index symbols> | payload…
    >
    > **A hand engraver putting many strings on one plate can therefore stop
    > repeating the header** and keep the payload characters vertically aligned
    > to show a header was dropped — operator ruling 2026-08-23. **The previous
    > 50-bit layout could not support that at all:** its invariant part was 38
    > bits = 7.6 symbols, so the eighth character *mixed* invariant bits with
    > `index` bits and there was nothing clean to elide.
    >
    > It also means **`mt-codec` needs no bit packer**, so the crate inherits
    > none of `md`'s bitstream, padding-tolerance or rollback machinery, and
    > every field is readable off the plate by counting characters.
    >
    > **The cost, measured against real artifacts:** +1 symbol per chunk versus
    > the 50-bit layout — 5 characters on the smallest measured transaction, 14
    > on the 535-byte case, 63 on the 2,498-byte one. Against 1,242 characters
    > for that middle case, 14 is noise.
    >
    > **`count` and `index` are 15 bits, giving 32,768 chunks.** The width is set
    > by alignment rather than by need: standardness caps a transaction at
    > ~100,000 vbytes ≈ 2,500 chunks, so this is ~13× headroom where 12 bits gave
    > 1.6×. The spare range is a **consequence** of the layout, not a target.

    **Bit order and padding.** Fields are written most-significant-bit first in
    the order above. The **55-bit** header is followed immediately by the chunk
    payload with **no padding between them**; padding appears only once, at the
    end of a chunk, to reach the next 5-bit symbol boundary (`mt encode`) or
    byte boundary (`mt qr`).

    > **Since the header is exactly 11 symbols, the payload begins at symbol 11
    > of the data part** — so a reader can locate it by counting characters,
    > with no bit arithmetic. This is the practical dividend of the alignment
    > ruling above, and it is why `mt1` does not need the padding-versus-
    > truncation rollback contract `md1` carries.

    **(c) A content id — the transaction id, and R2 lens 2 found the ruling
    AMBIGUOUS.** A PSBT holds **two** transactions that could be called "the"
    transaction: its `unsigned_tx`, and the one `extract_tx()` produces. **For
    every legacy and `sh(wsh(…))` input their txids DIFFER**, because a legacy
    `scriptSig` is part of the txid preimage while a witness is not. Two
    implementers picking differently would produce plates neither could
    reassemble from the other.

    **Resolved: the id derives from the EXTRACTED transaction's txid** — the
    thing actually engraved, actually broadcast, and actually re-derivable by a
    recoverer who has decoded the plate and holds nothing else. `unsigned_tx` is
    a PSBT-internal artifact a recoverer never sees.

    **The top 20 bits of the txid in its standard display form** — the
    big-endian hex a user reads. Stated to that precision because *"which 20
    bits, from which end"* is exactly where two implementers diverge silently,
    and the internal byte order is the reverse of the displayed one.

    Reassembly re-derives the id from the transaction it decoded and compares.
    `derive_chunk_set_id`
    hashes a *descriptor*, and reassembly re-derives it from the decoded object
    as what the source calls *"the content-id oracle; funds-load-bearing
    invariant."* `mt1`'s analogue is the **txid**: already a canonical hash of
    exactly this content, already present, already what a recoverer would use to
    name the transaction. **Reassembly re-derives it from the decoded
    transaction and compares**, giving `mt1` the same invariant `md1` has.

    **Width stays at 20 bits.** Operator: *"1 in a million is more than unique
    enough. User only needs to distinguish between at most a few dozen engraved
    transactions… 1 in 1000 only saves 2 characters from 1 in 1000000, so 20
    bits is probably not too burdensome."* The arithmetic holds — 20 bits is 4
    codex32 symbols against 10 bits' 2, so narrowing saves 2 characters **per
    chunk** (~24 on a 12-chunk transaction). Worth adding: **the re-derivation in
    (c) is what makes the width non-critical.** A collision cannot yield a wrong
    transaction, because reassembly re-derives the id from what it decoded and a
    mismatch is caught. The 20 bits buy human discrimination and early detection,
    not integrity.

    > **WHERE THIS LANDS — and an earlier statement of mine was wrong.** I said
    > this "lands in `descriptor-mnemonic`". It does not. The constellation's
    > precedent is **forking, not sharing**: `md-codec`'s own BCH decoder says
    > *"Forked from `mk-codec` v0.3.1… The algorithm is constant-agnostic — the
    > caller XORs the polymod residue against the per-HRP target constant"*, and
    > `md-codec` has **no dependency on `mk-codec`**. So `mt1` forks the same
    > machinery into **`mt-codec`, in the new `mnemonic-transaction` repo**, with
    > its own constants. **`descriptor-mnemonic` is untouched.**
    >
    > **There is no future shared crate, and this box said there was — R8 gates
    > I10.** It read: *"a future `mc-codex32` shared crate is planned to retire
    > these forks… so `mt1` should be built to be absorbed by it later."* That
    > plan was **RETIRED on 2026-05-03**, recorded in
    > `mnemonic-key/design/FOLLOWUPS.md` as
    > `mc-codex32-extraction-retired-2026-05-03`, and retired on a technical
    > finding rather than a schedule:
    >
    > > *"md1 and mk1 use HRP-mixed BCH with per-format target residues that are
    > > NOT upstreamable … There is no longer shared code worth extracting —
    > > only a shared **pattern** … md1↔mk1 BCH plumbing stays forked
    > > **indefinitely**."*
    >
    > **The correction changes what an implementer should do.** "Build it to be
    > absorbed later" shapes a crate around a merge that will never come —
    > generic seams, deferred naming, constants held at arm's length. The truth
    > is that HRP-mixing and per-format residues make this code **unshareable in
    > principle**: there is nothing to absorb, now or at v1.0. **`mt-codec` is
    > the third instance of a pattern, not the third tenant of a future crate**,
    > and should be written to be clear on its own terms.

    **What Rust-primary means for this format**, since it binds later rather
    than now: `mt-codec` in Rust is the primary and only implementation today.
    When SH2 learns to read `mt1` — §10.2's static-scan reader and §10.17's
    firmware work — the **Go decoder is written as a PORT**, bound to the Rust
    conformance vectors, and may never lead. If the two ever disagree, Rust is
    right by definition and Go is the bug. That is not theoretical in this
    constellation: Go and Rust once computed **different `WalletPolicyId`s**
    while 887 fork tests passed either way, and only cross-language vectors
    caught it.

    **No longer blocking as a design question** — it is now scoped
    implementation work with every decision made. It still blocks *code* for
    both verbs, since both fragment with this header.

14. **§5's legend budget rests on a doc comment, not on the fork's font
    metrics. DEFERRED** by operator ruling 2026-08-23. `legend.rs` hardcodes
    `CHARS_PER_LINE = 35.0` / `LINES_FULL_PLATE = 20.0` per a doc comment at
    `crates/me-cli/src/lib.rs:46`; the fork's real ladder has six rungs and
    those are the 3.8 mm one. §4's 4.25 mm pitch is `85/20` — full plate height,
    where §4 uses 79 mm elsewhere — and is not a rung of `FontSizes`. Magnitude
    is under a millimetre. **Deferred, not closed:** §4's table must be
    regenerated before implementation anyway, for the three unmodelled inputs
    named there, and this correction rides along with that regeneration.

15. **SETTLED** — §8.4 sets no minimum timelock horizon, and cannot tell a timelock from. Reasoning in §12.15.

16. **SETTLED** — Should mt refuse legacy (non-segwit) inputs at all. Reasoning in §12.16.

17. **MOVED — see `design/SPEC_mt_qr_DEFERRED.md`.** The firmware cannot yet engrave what §4 selects — and will be taught. `mt qr`
   material, deferred with the verb (§0a). The number is kept so citations
   to §10.17 from elsewhere in this document, and from commit messages, keep
   resolving.

18. **SETTLED** — Does §8.2's consensus-engine check survive the scope line. Reasoning in §12.18.

19. **SETTLED** — Does CPFP still require the parent to reach the mempool. Reasoning in §12.19.

20. **Legacy inputs are txid-malleable, and the content id is the txid.** A
    legacy `scriptSig` can be re-encoded by a third party in relay without
    invalidating the signature, changing the txid — what SegWit fixed. The
    engraved bytes still have exactly one txid and a recoverer re-derives it
    deterministically, so §10.13's content id is sound. But **if a malleated
    version confirms first, the confirmed txid will not match the plate's** —
    the plate is not wrong, it is superseded, and the original can no longer
    confirm. Worth a sentence somewhere a recoverer will read.


21. **SETTLED** — Nothing on the plate names the format. Reasoning in §12.21.

22. **SETTLED** — mt1's NUMS domain string is undecided. Reasoning in §12.22.

23. **SETTLED** — Season names are hemisphere-relative on a permanent artifact. Reasoning in §12.23.

## 11. Provenance of the numbers

Everything measured is in `design/measurements/`, with the probe sources and a
reproduce path that is a command rather than a memory. Transaction sizes come
from real transactions — built, signed, finalized, extracted, serialised, never
estimated. QR capacities are gated against the published v40 limits at every
mode and ECC level, a gate that caught three wrong payload constructions before
these numbers were trusted. Plate and module constants are read from the fork
(`backup/backup.go:45,99-102`, `cmd/controller/platform_sh2.go:188`).

The probe crate has been re-run twice, and the counts differ because the crate
grew between them:

- **2026-08-22, before R0** — all **12** binaries then in the crate rebuilt and
  re-run; **9 of 11** results files reproduced **byte-identically**, the two
  exceptions differing only by capture artifacts documented in
  `design/measurements/README.md`.
- **2026-08-23, for the chunk-size correction** — `psbtfinal.rs` had since been
  added, so all **13** binaries were rebuilt and re-run and all **12** results
  files regenerated. This is the current state of every number in this spec.

§3b's chunk counts come from `RESULTS_envelope_2026-08-22.txt` and
`RESULTS_rcw_2026-08-22.txt`, which measure the **raw signed transaction**
against a 40-byte chunk. `mt1`'s ceiling is 32,768 of them (§3).

> **They remain a LOWER BOUND, but not for the reason an earlier draft gave.**
> That draft called them "a floor, for the balancing reason stated in §3b" —
> i.e. because `md`'s chunker balances rather than fills.
>
> **THE REPLACEMENT REASON WAS ITSELF WRONG, AND IT INVERTED THE CITATION IT
> LEANED ON — R6, two lenses independently.** This box went on to say *"§3b's
> correction established that chunk sizing is a flat 40 payload bytes"*. §3b
> establishes the **opposite**: it retracts "a flat 40 bytes per chunk" by name
> as a mis-description of the chunker, and gives `bytes_per_chunk =
> ceil(len / count)`. §11 was citing §3b as the authority for the very sentence
> §3b exists to withdraw.
>
> **It is a Critical rather than a wording slip because the two rules produce
> different chunk boundaries**, and §1.1e's pre-decode length check is mandatory:
> an implementer who followed §11 would read an implementer who followed §3b's
> plates as **damaged steel**, and vice versa. Both would be reporting a hand
> error that never happened, on a set that is byte-perfect.
>
> The counts below remain a lower bound for the reason given next — what is fed
> in — and **not** for any claim about chunk sizing. The normative rule is §3b's,
> stated once under "The chunking rule".
>
> They are a lower bound because of what is fed in. `md-codec` chunks the output
> of `encode_payload`, which is a **framed** payload — canonicalization plus TLV
> sections — not raw bytes. The probe feeds the **raw transaction length**
> straight in, modelling **zero framing overhead** for `mt1`. Whatever header
> `mt1` ends up carrying adds to the payload and can therefore add chunks. That
> is precisely open question §10.13, and it must close before these counts are
> treated as final.

The BCH corrector's existence was read from `crates/md-codec/src/bch_decode.rs` and
`crates/md-codec/src/lib.rs:48` in the `descriptor-mnemonic` repo — a sibling, so
`plan-cite-check.sh` has no root for it and those two were checked by hand.

> The previous draft's §11 claimed *"everything measured is in
> `design/measurements/`"* while the block figures in the old sections 6c and 6d
> — the Merkle-proof and header-cost material — had no results file
> behind them. Those sections are now out of scope (§9), so the claim is true
> again by subtraction rather than by generating the missing evidence.


## 12. Appendix — the settled questions, with the reasoning that settled them

<!-- numbering: preserved -->

> **These are not open questions and stopped being so during the 2026-08-23
> cycle.** They were carved out of §10 so that section holds only what is still
> undecided — it had grown to 701 lines of which 301 were answers, which is the
> opposite of what a reader consults it for.
>
> **This is a MOVE, not a rewrite.** The bodies below are byte-identical to what
> §10 carried; nothing was re-authored in transit, so `git diff` on this commit
> shows relocation and nothing else. That was the deciding argument for an
> appendix over folding each entry back into the section it settled: folding
> would have re-authored ~300 lines, and this cycle has just measured what that
> costs — four of R6's six Criticals were defects in text written the same day.
>
> **The reasoning is the point, not the answers.** The three-position envelope
> history, the NUMS derivation with its independent recomputation, the retracted
> fountain-redundancy claim — these record WHY, which is the most-cited thing in
> this document and the part a future reader cannot reconstruct. Deleting them
> to shorten §10 would have destroyed exactly what was worth keeping.
>
> **Numbers are unchanged.** §12.N carries what §10.N carried, and §10.N now
> points here, so every citation — in this document, in the carved QR file, and
> in commit messages git will never update — still resolves.


4. ~~The legend's FROM and TO fields.~~ **CLOSED**, operator rulings
   2026-08-23: *"we use walletid or seed fp for the from: field and to: field.
   Optional but loudly warn if either not supplied"*, and — for the third-party
   case — *"warn if blank but allow, allow arbitrary text if user passes a
   flag."*

   Both fields are **wallet identities**, not addresses: a wallet id or a seed
   fingerprint. `FROM` is what §6 says a transaction cannot tell you on its own.
   `TO` names the counterparty rather than one of its scripts, which is why it
   replaced the truncated address R0 round 1 filed as a Critical (R-14) — a
   truncated address showed one output of several and could not be checked by
   eye.

   **Three states for `TO`, and paying a third party is the reason for the
   third:**

   | state | behaviour |
   | --- | --- |
   | wallet id or fingerprint | engraved as given |
   | **blank** | **allowed, loudly warned** on `stderr` — a plate with no destination named is legal and worse |
   | **arbitrary text, behind a flag** | engraved as given, e.g. `TO ALICE` |

   **The flag is the point, not a convenience.** A free-text label cannot be
   derived from or checked against the transaction, so requiring an explicit flag
   makes it an **act of assertion by the operator** rather than something that
   quietly appears. It is the same posture as the stub: a human-orientation aid,
   never an authority, and §5 already forbids branching on any of it. If the
   label disagrees with the transaction, the transaction wins.

   **Length: UNBOUNDED for `mt encode`, and the question only arises for the
   deferred verb.** Ruling 2026-08-23, prompted by R10 Important 2, which
   correctly objected that *"what `mt` does with a label too long for the
   field"* was filed as *"not a design question"* when refuse-versus-truncate is
   exactly a design question.

   > **It dissolves rather than needing an answer: `mt encode` HAS NO FIELD.**
   > §0a rules its legend is **`stderr` suggestion text**, and §3b rules the
   > layout is the operator's — so there is no width to exceed. `mt` prints what
   > it was given and the operator decides what fits their steel.
   >
   > **`mt qr` is where a width exists** (§5's 34 characters including the
   > amount, leaving a label roughly 16), and that verb is deferred — so the
   > overflow rule is the QR cycle's to make, alongside the budget that creates
   > the constraint.
   >
   > **Truncation is pre-refused for whenever that cycle runs.** A silently
   > shortened label on permanent steel says something the operator did not
   > write, which is the `PLATE 1 OF 1` failure in a different costume: a plate
   > asserting something false, read by someone who cannot tell.

   **Still to specify (§10.10's CLI work):** the flag's name.
   Refusing with the limit named fits §8's rule that every refusal names its
   number; silent truncation does not.


5. ~~Should `mt` require the node to be out of IBD before trusting
   `gettxout`?~~ **CLOSED — OUT OF SCOPE**, operator ruling 2026-08-23. `mt`
   asks the node it is given and reports what it is told; vouching for the
   node's sync state is not `mt`'s job. §8.5's refusal stands as written, and
   §6a already records that a `null` cannot distinguish "spent" from "this node
   does not know yet".


6. ~~How much fountain redundancy?~~ **CLOSED**, operator ruling 2026-08-23:
   zero. `mt` protects against plate damage (ECC), not plate loss (duplicate
   plates, the operator's choice). See §3.


7. **Back-side engraving — CLOSED for v0.1**, operator ruling 2026-08-23:
   *"yes, but probably better left to user to manage physically."* It would
   recover the 25.5 mm the legend costs and reduce plate counts, but there is no
   back-side path in the fork (`backup/backup.go:247` defines `frontSideSeed`,
   called once at `:134`, with a single `Engraving` per plate), so it is
   firmware work. An operator who wants both sides used can flip the plate and
   run a second job — a physical workflow rather than a `mt` feature. §4's plate
   counts therefore stand as one-sided.


8. ~~How does a recoverer learn the fragment parameters?~~ **ANSWERED, and the
   operator has ruled on what follows.**

   > **Ruling, operator, 2026-08-23: "each piece should say something like
   > n of m."**

   **Machine-readably this holds for both verbs, because §3 made them share one
   header.** `mt1`'s header carries `count` and `index` — n-of-m — plus a 20-bit
   `chunk_set_id` so pieces of different transactions cannot be combined. **It is
   `mt1`'s own 55-bit header, not `md-codec`'s 37-bit one** (§3): the latter's
   6-bit `count` caps a set at 64 chunks, which `mt qr` exceeds. For `mt encode` that header sits inside the
   BCH-protected chunk; for `mt qr` it rides in the bech32-uppercase payload.
   **One
   mechanism, both media.**

   > **This item was answered TWICE, and the first answer is gone.** It
   > originally analysed UR's fountain encoding — `SeqLen`/`MessageLen`/
   > `Checksum` in CBOR, the `ur:psbt/<n>-<m>/` prefix, and three traps in the
   > vendored decoder (the prefix is parsed then discarded; a single-part UR
   > carries no length or checksum at all; `Progress()` is a `x1.75` heuristic
   > that reaches 1.0 while `Result()` is still nil). **All of that is moot: §3
   > dropped UR entirely.** The traps are recorded here only so a future reader
   > who finds UR attractive again knows what the vendored implementation does.

   **The gap the ruling closes, which survives both the envelope change and the
   2026-08-23 removal of plate language:** a count of *carriers* is not a count
   of *parts*. One engraved object may hold several chunks, so a label naming
   the object cannot tell a recoverer which **part** is missing — and a
   recoverer who reads out of sequence, or misses one chunk on a shared
   carrier, cannot tell what is absent.

   **Normative:** every engraved unit carries its own human-readable `n/m`
   beside it, for the chunk it holds. A recoverer must be able to inventory what
   they hold and name what is missing **without decoding anything**. A lone unit
   reads `1/1`, which is the only way it can state that it is whole.

   > **This is now the ONLY completeness label `mt` prints**, since the
   > operator's ruling deleted `PLATE n OF m` and the plate category with it.
   > `n/m` survives precisely because it counts chunks `mt` emitted rather than
   > carriers it cannot see.

   **Unpriced.** These labels consume plate area §4's table does not reserve,
   exactly as the legend did before it was measured — see §10.14, which already
   requires that regeneration. The cost is small per label (3–5 characters) but
   it is per **symbol**, not per plate, and the worst artifact here carries 5.
   **Measure before §4's numbers are treated as final.**


11. ~~How many codex32 characters fit a hand-engraved plate?~~ **CLOSED — OUT
    OF SCOPE**, operator ruling 2026-08-23: *"As many as a user wants. It is not
    our concern."* `mt encode` emits a string; what a user does with steel is
    theirs. See §3b. The **32,768-chunk** ceiling is unaffected — that is a
    property of the codec, not of anyone's plate.


12. ~~Should `mt1` FILL its chunks rather than balance them?~~ **CLOSED — NO.
    Filling would reduce error recoverability, which is the one thing this
    format exists for.** Operator question 2026-08-23: *"does increased packing
    reduce error recoverability?"* Answered from source, and the answer is yes,
    by two independent mechanisms:

    **BCH correction is PER CHUNK, and it is `t = 4`.** `decode_regular_errors`
    returns `None` for any pattern above *"t = 4 errors"*, against a 13-symbol
    checksum (`REGULAR_CHECKSUM_SYMBOLS`) over a codeword of at most 93 symbols
    (`crates/md-codec/src/bch_decode.rs`). Each chunk therefore carries its **own
    independent 4-error budget**.

    1. **Fewer chunks means less total correction.** For a fixed payload,
       filling packs the same bytes into ~12% fewer chunks — and the budget
       scales with chunk *count*. A 535 B transaction **balanced** is 14 chunks
       of 39 B (§3b's rule: `count = ceil(535/40) = 14`, then
       `bytes_per_chunk = ceil(535/14) = 39`) = **56 correctable symbol
       errors**; filled at ~45 B/chunk it is
       12 chunks = **48**. Same data, 8 fewer errors survivable.
    2. **Each chunk is longer under the same `t`.** Filling raises the symbols
       at risk per chunk while the per-chunk budget stays at 4, so the
       probability that any single chunk exceeds its budget rises.

    Both effects push the same way. **Balancing is not a limitation of `md`'s
    chunker — it is error-correction budget bought with plate area**, and for a
    hand-engraved artifact whose entire purpose is surviving a miscut character,
    trading it for ~340 bytes per chunk of capacity is the wrong trade. The
    **1,310,720 B** ceiling stands, and §8.7b refuses past it.


15. ~~§8.4 sets no minimum timelock horizon, and cannot tell a timelock from
    RBF signalling.~~ **CLOSED — OUT OF SCOPE**, operator ruling 2026-08-23:
    *"not our concern. User handles this by their own wallet, or we later create
    our own wallet utilities."* Consistent with §0: `mt` does not build
    transactions, so how long a timelock ought to be is a wallet decision. `mt`
    still verifies that the timelock it was handed is **enforced** (§8.4) — it
    simply does not judge whether the horizon is wise.


16. ~~Should `mt` refuse legacy (non-segwit) inputs at all?~~ **CLOSED — NO**,
    operator ruling 2026-08-23: *"Do not exclude legacy inputs. It is user
    responsibility to know their inputs for such edge cases."* See §8.6. The
    original refusal's premise was false (`non_witness_utxo` binds a legacy
    amount by txid), and `sh(wsh(…))` is no longer unclassified since every
    input type is accepted. The residual risk is handled by §8.2c's `stderr`
    warning — which states the fee arithmetic — and by §8.2d, which binds any
    input carrying `non_witness_utxo` by txid. **Nothing reaches an `mt qr`
    plate**: §5's legend is full (§8.2c). Recorded in §7.


18. ~~Does §8.2's consensus-engine check survive the scope line?~~ **CLOSED —
    NO. Script validity is out of v0.1**, operator ruling 2026-08-23: *"We don't
    care if transaction is valid for initial version. We might never care but we
    might add it someday."* §8.2 is removed, `mt` drops its consensus-engine
    dependency, and §7 carries the accepted hazard: a transaction with a bad
    signature engraves cleanly and fails at broadcast. Reopen if it is ever
    added.


19. ~~Does CPFP still require the parent to reach the mempool?~~ **CLOSED — the
    spec no longer needs the answer.** Operator ruling 2026-08-23: *"We don't
    care about rbf or cpfp… we can't control the future but cpfp is a well known
    standard that will help user in future if they picked a bad fee."*

    `mt` neither implements nor checks either mechanism. §8.2b's low-fee warning
    **names** CPFP and out-of-band miner submission as things a future holder can
    try, and guarantees neither — so the mempool question stops being
    load-bearing. Out-of-band submission is itself the answer to the case that
    prompted this: a fee too low for the parent to reach a mempool at all
    bypasses relay policy by going straight to a miner.


21. ~~Nothing on the plate names the format.~~ **CLOSED**, operator ruling
    2026-08-23: the suggested legend gains a sixth field, **`FORMAT: mt1
    codex32`**, specified in §5 with its reasoning.

    **The question was found by walking Journey B, not by reviewing §5.** A
    recoverer in 2040 holds a string and no indication of which tool reads it;
    `MT1QZRF8X…` in a search engine returns nothing. Every *other* legend field
    is reconstructible by `mt inspect` from the string alone (§1.1) — **which
    program to run is the one thing inspection cannot tell you**, so the field
    that looked like the least important of the six is the only one whose
    absence ends the journey.

    **A repo URL was considered and rejected as the tag.** It fails on
    durability: a domain must outlive the plate, and a lapsed one someone else
    buys points a bearer-instrument holder at a stranger. `codex32` is BIP-93 —
    published and archived independently of this project — so the tag survives
    the project. A URL may ride along as extra suggested text; it may not *be*
    the identifier.

    **The `136 characters` cited in the original entry was already stale**, and
    regenerating the probe to price this field is what exposed it: §5's minimal
    legend measured 141, then 145 after the `BROADCAST` fix, 164 with this
    field, and **152** once `PLATE n OF m` was deleted the same day. The stale figure had survived because nothing re-ran
    `legend.rs`; the number is now emitted by the probe rather than carried in
    prose (§10.14).

    **Residual, inherited by the deferred `mt qr` cycle (§0a):** the sixth field
    takes the legend from 6 lines to **7**, which `legend.rs` shows still fits
    one plate at v13 but forces a second at v18 and above. Free for `mt encode`,
    where the legend is `stderr` text and `mt` owns no layout.


22. ~~`mt1`'s NUMS domain string is undecided.~~ **CLOSED**, operator ruling
    2026-08-23: the domain string is **`"shibbolethnumstransaction"`**, giving
    **`MT_REGULAR_CONST = 0x1a2fc877f9528d7c1`**. Stated with its derivation in
    §10.13(a), and recomputed there before it became normative.

    The *rule* was always derivable — `MD_REGULAR_CONST` is verifiably the top
    65 bits of `SHA-256("shibbolethnums")` — but the **domain string is an
    arbitrary chosen name** no implementer could have inferred. That mattered
    because the fork mechanic makes the worst guess the most tempting: copy a
    sibling codec, change the HRP, and leave the constant.
    **§10.13 now has no undecided input left.**

    > **CORRECTION 2026-08-23 — this entry said the consequence was that "`mt1`
    > chunks verify as `md1` chunks", and that is FALSE.** The HRP is mixed into
    > the checksum on both sides — `bch_create_checksum_regular` computes
    > `polymod_run(hrp_expand(hrp) ‖ data) ^ CONST`, and verification compares
    > `polymod_run(hrp_expand(hrp) ‖ data_with_checksum)` against the constant —
    > so **differing HRPs separate the formats by themselves**, whatever the
    > constant is. Cross-format acceptance is unreachable while the HRPs differ.
    >
    > **The real consequence is intra-format and worse**, because it is silent:
    > a wrong constant — copied *or* mistyped — produces chunks that are
    > **self-consistent and unreadable by every other implementation**. And it
    > surfaces at *recovery*, where it is indistinguishable from steel damage:
    > the recoverer sees checksum failures on a plate that is physically
    > perfect.
    >
    > **Operator ruling: cross-format verification is abandoned** — *"it's
    > unlikely and not worth the effort"* — and the plan's cross-format negative
    > test is deleted with it, since it returned the same result whether or not
    > a constant had been copied and therefore measured nothing. What defends
    > the constant instead is a **spec-authored pinned byte-exact vector**
    > (§10.13 a) that the implementation cannot produce for itself.


23. ~~Season names are hemisphere-relative on a permanent artifact.~~
    **CLOSED**, operator ruling 2026-08-23: seasons are **northern-hemisphere**
    and §8.4 says so. A southern reader misreads the estimate by about six
    months; the harm is bounded because the **mandatory block height beside it
    is unambiguous everywhere**, so a misread costs an orientation rather than a
    recovery. Alternatives considered and not taken: month ranges (`~SEP 2034`)
    or quarters (`~Q4 2034`), both hemisphere-neutral, both less legible to the
    majority of readers.

