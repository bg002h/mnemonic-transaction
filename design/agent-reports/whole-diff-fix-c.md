# Whole-diff review: mt FIX-C (edeec54 vs 4af58aad), single Sonnet slice

No Critical, no Important. Every FIX-C rule (§2.1-§2.5) is implemented and has a RED row that fails when its hunk is planted back.

## Plant-back (scratch copy `/scratch/cache/bughunt/wd-c/m`, own CARGO_TARGET_DIR; baseline 248 pass / 4 skip)

| Rule | Planted back | Rows that fail |
| --- | --- | --- |
| §2.1 R7 | char-boundary preference in `full` | `07d` |
| §2.1 R7 | `full[..ELIDED_DROP]` slice (panic) | `07`, `07b`, `07c` |
| §2.2 R12 | drop the `positional_autocorrect` arm | `12`, `12b` |
| §2.3 R9 | drop the `\u00XX` arm | `10e` |
| §2.3 R9 | drop the `\n` arm | `10e` |
| §2.4 R9 | `decode` `correction_warnings` | `10`, `10c`, `10d` |
| §2.4 R9 | `inspect` `correction_warnings` | `10`, `10c`, `10d` |
| §2.4 R9 | `inspect` duplicate loop | `10`, `10b` |
| §2.5 R10 | `json_quiet_guard` call in `decode` | `11`, `11b` |
| §2.5 R11 | `verify` guard call | `r11`, `r11_json` |
| §2.5 R11 | `inspect` guard call | `r11`, `r11_json` |

Each fails for the stated reason (the behaviour the row asserts is absent). Not re-planted: the §1 spec paragraph, `refusals.toml`, the EXEMPT entry (text/data, covered by the refusal-coverage script).

## Mutation survivors (`report.rs:443:22` guard to `true`; `:443:33` `<` to `<=`)

Both are the same arm: `c if (c as u32) < 0x20`. Test `bughunt_mt_10e` checks the control-character escapes appear and that the output parses back to the same string. Over-escaping (every char as `\u00XX`, or U+0020 as ` `) still parses back to the same string, so the test cannot see it.

- **Spec mapping: neither survivor maps to a spec rule.** Spec §10.10b says only "Control characters U+0000-U+001F inside any JSON string are escaped (RFC 8259 §7)". Both mutants still escape all of those, and their output stays valid JSON with identical decoded content. No Opus slice is needed.
- They do contradict the FIX §2.3 table row "anything else | itself", so this is a test-strength gap in a FIX table row, not a removed rule. The `true` mutant would also emit `ὠ0` (five hex digits, invalid) for a non-BMP char; this is reachable only under the mutant.

### M1 (Minor): `bughunt_mt_10e` does not pin "anything else escapes to itself"
`crates/mt-cli/src/report.rs` test `bughunt_mt_10e_render_json_escapes_control_characters` (about line 744-772). Fails closed (a mutant, not shipped behaviour); no printed command fails; needs a future edit to the guard plus a consumer reading the raw text.
Fix text: change the warning to `"a\nb\rc\td\u{8}e\u{c}f\u{0}g\u{1}h\u{1f}i\"j\\k l"` (adds a literal space) and add, before the `serde_json::from_str`:
```
        assert!(
            out.contains(r#""a\nb\rc\td\be\ff\u0000g\u0001h\u001fi\"j\\k l""#),
            "non-control characters were escaped:\n{out}"
        );
```
Run as printed with `cargo nextest run --locked -p mt-cli -E 'test(bughunt_mt_10e)'`: real code PASS; guard to `true` FAIL; `<=` FAIL ("non-control characters were escaped").

## Nits
- N1: `design/FIX_bughunt_mt_c.md` §2.5 says the decode guard is "the first statement of its verb"; true. No change.
- N2: the spec §10.10b wording leaves "anything else is literal" to the FIX doc only; optional one-clause addition to the spec. Not needed if M1 is folded.

C / I / M / N: 0 / 0 / 1 / 2
