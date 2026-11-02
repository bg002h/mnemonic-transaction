# mnemonic-transaction

@AGENTS.md

## Known hazards

- **Input sniffing (mt-cli).** Binary PSBT detection runs before any whitespace processing: the bytes `0x09`, `0x0a` and `0x20` are ordinary data inside a binary PSBT, and stripping them corrupts it.
- **Node access is read-only (mt-cli).** The node module only observes the chain (through `bitcoin-cli`); it is not a broadcaster and must not become one.
- **Engrave boundary.** mt1 and `tx:` records reach mnemonic-engrave (`me`) through its systemwide transport boundary. They do not belong in `me`'s public md/mk bundle, which refuses them.
- **Design files are copies.** The spec, vector documents and vector corpus are copied from mnemonic-engrave, which owns the normative spec and the independent vector generator. Change the design there, then re-sync the copies here, and never regenerate reference vectors from mt-codec.
- **BCH port (mt-codec).** The string layer is ported from mk-codec, not md-codec. A defect found in any of the md, mk or mt BCH implementations requires checking the other two.
