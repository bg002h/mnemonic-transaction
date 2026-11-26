# Review r2 (fold review): bg002h/mnemonic-transaction PR #32, fold commit 140fbfb

Reviewer: second, independent fold review (Opus 5.5), 2026-11-25. Branch `claude/project-thread-utm1aa`, head `140fbfb`, clean tree. Nothing tracked was edited, committed, pushed or posted. Probes ran against `target/debug/mt` built at head, using scratch files in `/tmp/claude-0/-home-user/efb9fedd-ab0a-5019-871b-00d31aeaaeb6/scratchpad/r2probe/`.

**Verdict: 0 Critical, 0 Important, 1 Minor, 2 Nit.** The fold is faithful. r1's grading stands: no r1 Minor rises to Important or Critical. The new Minor (R2-M1) is pre-existing and outside the PR diff. It should be logged, and it does not block.

## Q1. Does the fold log every r1 Minor and Nit accurately?

Yes. `design/FOLLOWUPS.md` (lines 323-333) gains 11 entries, one per finding: M1-M4 as `grade=M` and N1-N7 as `grade=N`. No grade is inflated or deflated. Every entry uses the tracker's existing one-line form: `- [slug] status=open grade= kind= owner=none filed= src=...#anchor :: text`. Every `kind` value (secret, ux, docs, code, api, test) is in use elsewhere in the file except `code` and `api`, and the format leaves `kind` free. M1 is correctly tagged `kind=secret`. The `src` anchors (#m1 through #n7) match r1's headings.

I checked the text of each entry against r1:
- **M1**: accurate. It covers the six forms, the exit-0 outcome, "nothing past the codex32 header is printed", and both fix directions, including the rename.
- **M2**: accurate.
- **M3**: mostly accurate (see R2-N1).
- **M4**: accurate.
- **N1-N7**: accurate summaries.

r1 recommended M1, N4 and N7 before merge. The fold logs them instead of fixing them. That matches Brian's rule (Minor and Nit are logged, not fixed), so it is not a defect.

## Q2. Did the fold change anything else?

No. `git diff a611567 140fbfb --stat` shows only `design/FOLLOWUPS.md` (+11, append-only, no existing line touched) and the new `design/agent-reports/bughunt-2026-11-25-mnemonic-transaction-review-r1.md` (+107). The saved report is byte-identical to the scratchpad r1 (`diff` is clean). No code, test, spec or CI file changed. No CI script parses FOLLOWUPS lines (`ci/code-docs-check*` only checks crate README/include naming).

## Q3. Is any r1 Minor actually Important or Critical? (focus: M1)

**No. M1 stays Minor.** At head, none of the ms1 forms that slip past §8.9 causes secret material to be printed, engraved or stored.

Probes, all with the payload of the test vector `ms12wxvvw3vc7g9pm7f6h20pjm9pjm9pjm9pjm9pjm9pjm9p6hmun7`. "Leak" means any 6-character window of the codex32 body past offset 9, found in stdout plus stderr after lowercasing and stripping `-` and spaces.
- **Reading verbs** (`verify`, `decode`, `inspect`, text and `--json`):
  - Forms tested: space after the HRP, quoted, JSON, `seed:` label, hyphen-grouped, BOM, NBSP, one body typo, and an `mt1`-prefixed ms1.
  - Each form was placed both after and before a valid 6-string set: 96 runs, 0 leaks.
  - Forms after the set exit 0 (as r1 found).
  - Forms before the set: space, hyphen and typo are refused by §8.9 (the 64-byte head check). Quoted, JSON, label, BOM, NBSP and mt1-prefixed also exit 0. r1 only tested "after", so this widens M1's scope slightly but not its grade.
  - The only trace of the stray token is the UNREADABLE STRING notice (`main.rs:1579-1582`). It prints one offending character and a data-part offset (for example `character '"' at data-part offset 8`), never the string.
  - `decode`'s stdout is only the broadcastable hex of the mt1 set's transaction.
  - `inspect`'s plate legend is built from the decoded transaction, not from stray tokens.
  - (`verify --json` exits 1 under §10.10 in every case, including the bare set. That is the ruled refusal, not a symptom.)
- **Engraving:** `encode` can be fed each form alone, or a valid tx hex followed by the form: 16 runs, every one exit 1, so no strings are produced and nothing reaches steel. Leaks: 0. The only echo is §8.2e step 4, which prints the first 8 bytes of the input as hex (`input.rs`, `take(8)`). For an ms1 starting at input offset k ≥ 0, the payload starts at k + 9 ≥ 9, so the 8-byte window cannot contain a payload character. At most it shows the HRP, threshold and id characters, as r1 said.
- **Stored:** no reading verb writes a file. `decode` writes the transaction hex to stdout only. mt never copies its input anywhere.

So M1 is a refusal-policy gap: mt accepts a file that also holds a seed share. It is not a disclosure. Minor is right for wallet software. M2 (the plate does not say what the lock counts from) and M3 (legend width) are presentation and spec-budget issues with no funds or secret effect. M4 is pre-existing copy drift. None of them rises in grade.

## Findings

### R2-M1 (Minor, pre-existing, outside the PR diff): `verify --transaction <file>` bypasses §8.9
`crates/mt-cli/src/main.rs:1798-1818`. The `--transaction` file is read with `std::fs::read` and goes straight to `input::sniff`, with no `secret_guard`. The comment at `main.rs:1295-1298` says §8.9 "binds the READING verbs too", but that holds only for `--in`/stdin.

Probe: `mt verify --in set.txt --transaction <ms1 file>` refuses under §8.2e and prints `This input begins 6d 73 31 32 77 78 76 76` (`ms12wxvv`). That is the HRP, the threshold and 4 id characters, with no payload, and by the 8-byte bound above it can never include payload. So nothing is disclosed, but the secret is named under the wrong rule.

Fix direction: call `validate::secret_guard(&supplied, "verify")` before the sniff. Log it with M1 (`mt-t2-ms1-scan-forms`) or as a sibling entry.

### R2-N1 (Nit): M3's FOLLOWUPS fix text drifts from r1
`design/FOLLOWUPS.md:325` (`mt-t1-legend-width`) says "re-measure the plate field or shorten the spelling". r1 said "re-measure, or change the stated budget to '35 per nLockTime line; relative lines extra'". "Shorten the spelling" pulls against M2 (`mt-t1-legend-counts-from`), whose fix makes the line longer. The entry should name the budget-restatement option.

### R2-N2 (Nit): the new entries' `src=` paths drop the `design/` prefix
`design/FOLLOWUPS.md:323-333` cite `src=agent-reports/bughunt-2026-11-25-...`. The repo-local entries already in the file use `src=design/agent-reports/...` (for example lines 303-305). The `coordinator/` and `C:` prefixes denote other trees. As written, the path does not resolve from the repo root.

## Tests
I did not re-run the suite. The fold changes only Markdown, and r1 ran `cargo test --locked -j 2 -p mt-cli` at `a611567`; the only failures were the environmental `history_purge` ones. `cargo build --locked -p mt-cli` at head was already up to date.

## Summary for the controller
- Fold: faithful, complete, grades unchanged, no stray changes.
- Grading: confirmed. M1 has no print, engrave or store path for secret material.
- Log only (none blocks): R2-M1 (`verify --transaction` has no §8.9 guard; pre-existing), R2-N1, R2-N2.
