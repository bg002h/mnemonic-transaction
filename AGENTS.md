# mnemonic-transaction

Paths below are relative to this Git root, including when working inside a crate.

## Start here

Read [design/PROVENANCE.md](design/PROVENANCE.md); this checkout has no local CLAUDE.md.
Read applicable ancestor guidance too. Existing process rules remain applicable;
verify historical architecture/status statements against source.
Read the shared [guide](https://github.com/bg002h/mnemonic-toolkit/blob/a2e633b5384c6c61b3d9394379818b5ecd7d376d/docs/architecture/README.md) and [workflow](https://github.com/bg002h/mnemonic-toolkit/blob/a2e633b5384c6c61b3d9394379818b5ecd7d376d/docs/architecture/WORKFLOW.md).
For sibling checkouts/worktrees, prefer local `../mnemonic-toolkit/docs/architecture/`;
remote fallbacks pin a delivered guide revision. See its workflow for refreshing the pin.
Check Git status, the current branch, Cargo.toml and Cargo.lock before editing.
Preserve unrelated work and use explicit task-owned paths when staging.

## Where behavior lives

- [mt-codec](crates/mt-codec/src/lib.rs) owns the signed-transaction backup format.
- [mt-cli](crates/mt-cli/src/main.rs) owns intake, validation, refusals and reporting.
  Keep binary PSBT sniffing before whitespace processing in
  [input.rs](crates/mt-cli/src/input.rs).
- [node.rs](crates/mt-cli/src/node.rs) supplies read-only observations; it is not a broadcaster.
- mt1 and `tx:` records meet me at its systemwide transport boundary.
  They do not belong in me's public md/mk bundle.

## Contracts and checks

[PROVENANCE.md](design/PROVENANCE.md) identifies mnemonic-engrave as the normative
specification and independent vector-generator owner. Update design there, then
synchronize copies and corpus here; never regenerate reference vectors from mt-codec.
The BCH port derives from mk; a defect requires checking sibling implementations.
Use provenance/refusal coverage checks, the refusal manifest and mutation/journey
tests as directed by [.github/workflows/ci.yml](.github/workflows/ci.yml).
Live-node smoke testing is separate and must match the authorized task/environment.
Verify current branch protections before relying on historical push-policy prose.
