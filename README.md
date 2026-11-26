# `mt` — engravable backups of signed Bitcoin transactions

`mt` turns an **already-signed** Bitcoin transaction into `mt1` codex32 strings
that a human engraves on steel, and reads them back years later. It builds
nothing, signs nothing, and broadcasts nothing.

> **The engraving is a BEARER instrument.** Anyone holding the plate can
> broadcast the transaction. That single fact shapes most of this tool: what it
> refuses, where it will and will not print things, and why it never takes a
> transaction as a command-line argument.

## The four verbs

```
mt encode  < tx.psbt      # signed transaction  -> mt1 strings on stdout
mt decode  < typed.txt    # mt1 strings         -> broadcastable hex on stdout
mt verify  < typed.txt    # structural check, offline, never asks a node
mt inspect < typed.txt    # what is IN a set, consulting a node if one is there
```

**stdout is the artifact; stderr is everything a human must see.** That is a
hard interface boundary, not a formatting preference — `mt decode` is meant to
be piped, and the moment a warning shares that stream a downstream consumer has
to parse prose out of its own input.
If stdout cannot be written (a full disk), mt prints
`mt: cannot write the output: <error>` and adds 32 to its exit status; a reader
that closes the pipe early only stops the output. mt refuses to write its output
into one of its own input files.

**Exit status** follows the constellation's exit-code table: 0 = done, every
check passed; 64 = bad command line, nothing ran; 101 = internal error.
Otherwise the sum (1 to 63) of: 1 a check said no (`verify --transaction` names
another transaction, a node proves an input spent or disagrees with a value);
2 an input is invalid; 8 refused (bearer material on the command line, a
world-readable stdout, an output that is one of the inputs); 16 a correction was
applied (the output is the same, and a plate is damaged); 32 a file or helper
failed (an input that cannot be read, an output that cannot be written, a
`--bitcoin-cli` you named that is missing, fails or does not answer; no
`bitcoin-cli` at all, none named and none on `PATH`, is the offline path and
stays 0). A script that accepts a corrected plate tests
`[ $(( rc & ~16 )) -eq 0 ]`.

Input comes from a **file or stdin, never an argument**: an argument lands in
shell history and in `ps` for every user on the machine, and this material is
bearer.

`mt` keeps `encode` and `decode`. It has no `repair` verb: `decode`, `verify` and `inspect` correct substituted characters themselves, and the toolkit's `mnemonic repair` does not accept `mt1` yet (planned). `verify` and `inspect` are candidates to move to the toolkit later, only where their code is largely the same as other formats'; a part specific to `mt1`, such as `inspect`'s node query, stays here. Full table: [Which tool owns which verb](https://github.com/bg002h/mnemonic-toolkit/blob/master/docs/manual/src/50-comparing/51-format-decision.md#which-tool-owns-which-verb).

## Try it

```sh
cargo build
PATH=/nonexistent ./target/debug/mt encode --in tx.hex
```

With no `bitcoin-cli` to be found (no `--bitcoin-cli`, none on `PATH`) `mt`
takes the **offline** path, which is the constellation's normal posture, and
exits 0 with a warning. With a real `bitcoin-cli` on `PATH`, `mt` consults it
automatically and asks the operator for nothing — `bitcoin-cli` already holds
the RPC URL, the cookie, the network and the wallet. A `--bitcoin-cli PATH`
you name is a helper you asked for: if it is missing, fails or gives no usable
answer, `mt` still prints its output and adds 32 to the exit status.
`mt` supports Bitcoin Core 0.21 or newer, with -txindex for full node checks.

## Verifying the tool, not just running it

```sh
cargo nextest run --locked          # the suite
./scripts/check-refusal-coverage.sh # every refusal has a test, and vice versa
./scripts/mutate-refusals.sh        # every refusal test goes RED without its check
./scripts/journeys.sh               # three operator journeys, end to end
./scripts/check-provenance.sh       # the copied design files still match source
MT_RT=./rt ./scripts/live-smoke-test.sh   # ...and would a real node accept it?
```

The first four run in CI. The last two **cannot**, and each says so rather than
pretending otherwise: `check-provenance.sh` compares against a second repository
CI does not check out, and `live-smoke-test.sh` needs a funded `bitcoind`.

**`live-smoke-test.sh` is the one that answers the question none of the others
can.** Every gate above tests `mt` against `mt`: the pinned vectors came from an
encoder using the same constants the decoder does, the journeys assert on `mt`'s
own output, and the node stubs answer what they were told to. So it runs the
whole thing end to end against a real node — encode a finalized PSBT, verify the
strings, inspect them, decode them back — and finishes at
`testmempoolaccept`. **Bytes recovered from the engraving are a transaction
Bitcoin Core will accept, or the script exits non-zero.**

`mutate-refusals.sh` is the one worth understanding. A refusal test that passes
against code with its check deleted is testing nothing, so the script neuters
each named check in `crates/mt-cli/tests/refusals.toml`, runs only that
refusal's test, and asserts it goes red. **A green suite is where review starts,
not where it ends:** the mandatory post-implementation review found nine
Criticals in a tree where all of the above already passed.

## Where the design lives

**Not here.** The specification and implementation plan live in
`mnemonic-engrave/design/`, and copies under `design/` are exactly that —
copies, listed with their source commit in `design/PROVENANCE.md` and checked
by `check-provenance.sh`. If a `§`-reference in the code disagrees with the
spec, the spec wins.

Review reports persist verbatim in `design/agent-reports/`. Each was committed
**before** the fold answering it, so `git diff <persist>..<fold>` shows exactly
what changed in response to what — which is the only reason those diffs mean
anything.

## What v0.1 deliberately does not do

- **No `mt qr`**, and nothing QR-shaped. Deferred.
- **No script evaluation.** There is no consensus engine, so `mt` recognises
  signatures **by shape** and says so. It cannot detect a bad signature; that is
  an accepted, recorded hazard.
- **No transaction construction**, and no redundancy coding — the mitigation for
  a lost plate is cutting a second copy, which `mt` supports on the reading side.
- **Releases.** `mt-cli` ships signed binaries per `mt-cli-v*` tag (the
  release workflow signs `SHA256SUMS.portable` with the constellation minisign
  key). **`mt-codec` is not on crates.io**: it has no tag scheme, and
  `mnemonic-engrave`'s `me` takes it by git rev. The **format** is public in
  `SPEC_mt_v0_1.md` and its vectors.

## Licence

MIT OR Unlicense.
