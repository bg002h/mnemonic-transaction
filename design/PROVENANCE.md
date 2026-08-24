# Provenance

Every file here is COPIED from `mnemonic-engrave` and is not edited in this
repo. The design lives there; this repo implements it.

| file | source | source commit |
| --- | --- | --- |
| `design/SPEC_mt_v0_1.md` | `mnemonic-engrave/design/SPEC_mt_v0_1.md` | `ddc4e087248d90423e9c0f0c1e25108277b1e1d8` |
| `design/vectors/mt1_v1_vectors.md` | `mnemonic-engrave/design/vectors/` | `ddc4e087248d90423e9c0f0c1e25108277b1e1d8` |
| `crates/mt-codec/src/test_vectors/mt1_v1.json` | same generator run | `ddc4e087248d90423e9c0f0c1e25108277b1e1d8` |

## The vectors are NOT regenerated here

They are produced by `scripts/gen-mt1-vectors.py` in **`mnemonic-engrave`**,
which re-implements bech32, the 55-bit header and the BCH polymod from BIP-93 and
the spec — **independently of `mt-codec`**.

**Regenerating means re-running that script, never this crate.** A vector
re-derived from the implementation under test cannot falsify it: that is how a
wrong NUMS constant launders itself into looking correct. The generator's own
self-test reproduces 40/40 of `mk-codec`'s committed corpus before it emits
anything.

    mnemonic-engrave $ python3 scripts/gen-mt1-vectors.py --self-test

## The string layer is ported from `mk-codec`

`mt-codec`'s BCH, chunking and header modules follow
`mk-codec/src/string_layer/` — **not** `md-codec`. The constellation forks these
primitives per format rather than sharing a crate: `md1` and `mk1` use HRP-mixed
BCH with per-format target residues that are **not upstreamable**, and the
shared-crate plan was **retired 2026-05-03**
(`mnemonic-key/design/FOLLOWUPS.md`, `mc-codex32-extraction-retired-2026-05-03`).
So this is the third instance of a pattern, not the third tenant of a future
crate.

**Provenance pin:** `mk-codec` 0.5.0. Update this line on every sync, so drift
is auditable rather than folklore.

**A defect found in any of the three BCH implementations triggers checking the
other two**, and the check is recorded even when it finds nothing.
