# LIVE-NODE SMOKE TEST, 2026-08-24 — one Critical, found in ten minutes

**Not a review finding.** Found by the controller running the built binary
against the real regtest `bitcoind`, after P6 was committed and CI was green.
Recorded here because the fix must land as a visible RESPONSE, and the four
post-implementation reviewers were reading the repo when it was found.

## C1 — `mt encode` tells the operator a CONFIRMED transaction "can never be broadcast"

**Reproduced end to end, not reasoned about.** The P5 fixture transaction was
broadcast to the regtest chain and mined into block 103. Re-running `encode`
against the same PSBT, with the same node:

    mt encode: REFUSED — §8.5, input 0 (d13d7cbb…:1) is not in the UTXO set

      gettxout returned null for this outpoint AND its parent transaction
      is confirmed, so the output was spent or never existed. This
      transaction can never be broadcast — engraving it would produce a
      plate that looks like a backup and is not one.

      Build a new transaction from outputs that are still unspent.

Every sentence after the first is **false**. The output was spent *by this very
transaction*; the transaction **was** broadcast and is in a block; and there is
nothing to rebuild.

**Why it is Critical rather than a wording defect.** The remedy line tells an
operator whose payment SUCCEEDED to *"build a new transaction"*. An operator who
follows it pays twice. That is the money.

**`inspect` gets it right, on the same node, for the same transaction:**

    STATUS    SPENT — ALREADY CONFIRMED (this transaction is in a block)

So the correct answer is already implemented — in `report.rs`, whose
`Report::build` asks *"did THIS transaction confirm?"* **first**, with a comment
saying exactly why:

> ASKED FIRST: … otherwise the success case reports as the theft case: every
> input of a confirmed transaction is spent (by itself) and every parent is
> confirmed, which is exactly the DEAD condition.

`main.rs::encode`'s node loop never asks it. **Same defect, second site** — and
the spec (§6a's five states) had already written the fix down. This is the
single-owner rule violated one level up from where it was being enforced: not
two implementations of a ROW, but two implementations of a QUESTION.

**Why no test caught it.** §8.5's test uses a stub that answers `gettxout` null
and the parent confirmed. That is a faithful model of the theft case and an
equally faithful model of the success case — **the stub cannot tell them
apart**, because the distinguishing question is about a THIRD txid the stub was
never asked about. Offline tests could not catch it either: they have no node.
The plan's own P4 note predicted this exact shape — *"a fixture that has drifted
from the real RPC is exactly what a fixture-only gate cannot see, and the point
of running it once is to find that"* — and the once-only run happened at P4,
before §8.5 existed.

## The fix, non-authoritative

Ask the AlreadyConfirmed question **before** the per-input loop in
`encode`, exactly as `Report::build` does, and skip §8.5 when it is true. §8.5's
refusal is about an input taken by SOMEBODY ELSE; a transaction that spent its
own inputs is not that case. Then let the report say `ALREADY CONFIRMED` and
let the run proceed — engraving a confirmed transaction is pointless but
harmless, and `mt` should say which of the two it is rather than refuse with a
false reason.

The regression test cannot use the existing stub shape. It needs a stub that
answers `getrawtransaction` differently for the TRANSACTION'S OWN txid than for
its parents — which is the whole content of the defect.

## Consequence for the fixtures

The regtest chain's UTXO set has moved: the fixture's inputs are now spent, so
the "inputs unspent / STATUS LIVE" observation above is not reproducible on this
chain. Regenerating `p5_base.json` needs a fresh regtest node, which
`scripts/gen-refusal-fixtures.sh` should stand up rather than assume. No
committed test depends on the chain — every one is offline or stubbed.

## I1 — reaching a node makes the report STRICTLY WORSE

Same PSBT, same binary, two runs. It spends an output whose parent is sitting
unconfirmed in the mempool — an ordinary chained/CPFP spend.

**Offline:**

    FEE       0.00100000 BTC
    INPUTS    1 input(s)
                9fafebde4ed989f7…   2.00000000 BTC   TXID-BOUND
    STATUS    UNKNOWN — no node reachable

**With the node reachable:**

    FEE       UNKNOWN   (needs input values, which the transaction
              does not carry)
    INPUTS    1 input(s)
                9fafebde4ed989f7…   UNKNOWN
    STATUS    PENDING — a parent has not confirmed. This may still become live

`STATUS` improved, and **everything else got worse**. The operator who does the
more careful thing is shown less. The parenthetical is false as well — the
transaction *does* carry that value, in the PSBT's `non_witness_utxo`, and
§8.2d hashed it and matched the txid one step earlier.

**Cause.** `Report::build` branches on `node`, and only the `None` arm consults
the `claimed` values `encode` resolved. With a node present, an input that
`gettxout` reports as `Null` goes straight to `Provenance::Unknown` — discarding
a record that is *verified*, in favour of a lookup that merely failed to find it.
`include_mempool` is `false` by ruling, so `Null` is the EXPECTED answer here,
not evidence of anything.

**It contradicts §1.1's own row table**, which makes `FEE` present *"when a node
is reachable **OR** the input was a PSBT carrying values"* — an OR that the code
implements as an either/or.

**Fix, non-authoritative.** Make provenance a fallback CHAIN rather than a
branch: chain value → txid-bound record → PSBT-claimed record → operator
assertion → `UNKNOWN`, taking the strongest available at each input
independently. The `Null` arm should fall through to `claimed` instead of
terminating in `Unknown`.

**Why no test caught this one either.** Every report test is offline, where the
`None` arm is the only one that runs; the node-fixture tests all answer
`Unspent`, where the chain value legitimately wins. The gap is the third
combination — **node reachable AND the outpoint not in the UTXO set** — which no
fixture produced and a real mempool produces constantly.

## What the live node was worth

Two defects in about ten minutes, neither reachable from any offline or stubbed
test, and both in code that passed 117 tests, three gates and CI. The pattern in
both: **a stub answers the question it was asked, and the defect is in a
question nobody thought to ask it.**

## The three cases, all run against the real node — and only ONE is wrong

`gettxout` answers `null` in all three. They are not the same situation, and the
question that separates them is one `getrawtransaction` on the transaction's
**own** txid.

| # | situation | gettxout(input) | parent confirmed? | THIS tx confirmed? | mt says | correct? |
| --- | --- | --- | --- | --- | --- | --- |
| B | this transaction already confirmed | null | yes | **yes** | REFUSED §8.5, *"can never be broadcast"* | **NO — every clause false** |
| C | parent still in the mempool | null | no | no | STATUS PENDING, no refusal | yes |
| D | somebody else took the input | null | yes | no | REFUSED §8.5 | **yes — every clause true** |

D's message, verbatim, is the same text that is false in B:

    gettxout returned null for this outpoint AND its parent transaction is
    confirmed, so the output was spent or never existed. This transaction can
    never be broadcast … Build a new transaction from outputs that are still
    unspent.

In D that is exactly right and exactly the advice to give. **The words are not
the defect — the missing question is.** So the fix must not soften D's wording;
it must ask the question that tells B from D, which is the same question
`Report::build` already asks first.

C was worth running for its own sake: it is R6 adversarial C-3's case
(*"a mempool-only parent is a WARNING, not a refusal"*), and it behaves
correctly against a real mempool, not just against a stub that was told to say
so.

**The regression test needs a stub that answers per-txid**, because all three
cases share `gettxout → null` and differ only in what `getrawtransaction` says
about two different txids. The current stub answers `getrawtransaction`
identically for every argument, which is precisely why it models B and D
identically and could never have caught this.
