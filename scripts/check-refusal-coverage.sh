#!/usr/bin/env bash
# The exhaustiveness gate: refusals.toml and the tests are a BIJECTION.
#
# A refusal cannot be added and silently go untested, and a refusal test cannot
# exist without an entry saying which rule it proves.
#
# THE INPUT IS A COMMITTED LIST, NOT A PARSER OVER §8, and that is three
# findings with one cause:
#
#   - §8's numbering contains NON-refusals. §8.2 (script validity, removed) and
#     §8.8 are numbered items that are not v0.1 refusals; §8.7 and §8.7c are
#     pointers to deferred `mt qr` material. A script counting `^\d+[a-z]?\. `
#     in §8 would demand tests for four things that cannot fire.
#   - A REAL REFUSAL LIVES OUTSIDE §8's NUMBERING. §6a's value-mismatch refusal
#     is normative and is not a numbered §8 item, so a §8-numbering script is
#     STRUCTURALLY UNABLE TO SEE IT -- the one class this gate exists to prevent.
#   - The spec is in a DIFFERENT REPOSITORY, so a parser had no file to read.
#
# WHAT THIS DOES NOT COVER. It cannot tell whether the LIST is complete against
# the spec -- only a human comparing it to §8 can, and the list was seeded from
# the implementation plan for exactly that reason. It checks the coupling, not
# the ruling.
set -euo pipefail
cd "$(dirname "$0")/.."

TOML=crates/mt-cli/tests/refusals.toml

python3 - "$TOML" <<'PY'
import re, sys, os

toml_path = sys.argv[1]
text = open(toml_path).read()
entries = []
for b in text.split("[[refusal]]")[1:]:
    f = dict(re.findall(r'^\s*(spec|test|check)\s*=\s*"([^"]+)"', b, re.M))
    missing = {"spec", "test", "check"} - set(f)
    if missing:
        sys.exit(f"FAIL: an entry is missing {sorted(missing)}:\n{b.strip()[:200]}")
    entries.append(f)

if not entries:
    sys.exit("FAIL: refusals.toml lists no refusals")

# EVERY suite, not one file. Scanning only refusals.rs meant a refusal test
# placed one file away was invisible -- and one already was:
# encode.rs::refuses_a_hex_encoded_psbt_by_name, a genuine §8.2e refusal that
# went the whole of P5 with no entry and no mutation control. An independent
# review proved the gap by adding an undeclared test to encode.rs and watching
# this gate report no problem.
import glob
declared = {}          # test name -> file it lives in
for path in sorted(glob.glob("crates/*/tests/*.rs")):
    body = open(path).read()
    for m in re.finditer(r"^fn ([a-z0-9_]+)\(\)(?:.|\n)*?(?=\n#\[test\]|\n/// |\Z)", body, re.M):
        declared[m.group(1)] = (path, m.group(0))

# Exempt by NAME, and only with a stated reason that a reader can check. An
# exemption list is safer than a cleverer heuristic: it is visible in the diff.
EXEMPT = {
    "ordinary_amounts_parse":
        "the CONTROL for §8.2c's amount parser -- it asserts a legitimate amount "
        "is NOT refused, so it names a §-refusal in order to check its absence",
    "refuses_beyond_the_budget":
        "mt-codec BCH t=4 correction limit, not a §8 refusal -- it matches the "
        "naming convention by coincidence and asserts on a codec error",
}

problems = []

# 1. Every listed test EXISTS.
for e in entries:
    if e["test"] not in declared:
        problems.append(f"{e['spec']}: no test named `{e['test']}` in any crates/*/tests/*.rs")

# 2. Every listed check RESOLVES to a real function in a real file.
for e in entries:
    rel, _, fn = e["check"].partition("::")
    path = os.path.join("crates/mt-cli", rel)
    if not os.path.isfile(path):
        problems.append(f"{e['spec']}: check names {path}, which does not exist")
        continue
    body = open(path).read()
    if not re.search(rf"^\s*(pub )?fn {re.escape(fn)}\(", body, re.M):
        problems.append(f"{e['spec']}: {path} has no `fn {fn}(`")
        continue
    # The mutation shape mutate-refusals.sh relies on.
    sig = re.search(rf"^\s*(pub )?fn {re.escape(fn)}\((?:.|\n)*?\)\s*->\s*([^{{]+)\{{", body, re.M)
    # The shapes mutate-refusals.sh can neuter: a REFUSING guard
    # (`Result<T: Default, Refusal>`) and an ADVISORY one (`Option<Refusal>`).
    # One mutation concept -- make the check not fire -- with the spelling chosen
    # by the signature, so a genuine refusal is never kept OUT of this ledger
    # because of the shape of its return type.
    ret = sig.group(2).strip() if sig else "?"
    ok_shape = ("Refusal>" in ret) and ("Result<" in ret or ret.startswith("Option<"))
    if not ok_shape:
        problems.append(
            f"{e['spec']}: `{fn}` returns `{ret}`, which mutate-refusals.sh has no "
            "neutral return for -- it handles `Result<T: Default, Refusal>` "
            "(`Ok(Default::default())`) and `Option<Refusal>` (`None`)"
        )

# 3. THE SEEDED SET. These twelve are the implementation plan's own list, put
#    here rather than derived from the implementer's reading of §8 -- an
#    exhaustiveness gate whose input is the implementer's reading checks only
#    that they were self-consistent. §8.2b rules FOUR checks under one number
#    (inputs>=outputs, absurd fee, no duplicate outpoints, vin non-empty), so a
#    spec reference may carry several entries; the TEST is the unique key.
REQUIRED = [
    "§8.1", "§8.2b", "§8.2d", "§8.2e", "§8.2f", "§8.3",
    "§8.5", "§8.6a", "§8.6b", "§8.7b", "§8.9",
    "§6a",   # normative, and OUTSIDE §8's numbering -- see the header.
    # FOUND BY WIDENING THIS GATE TO EVERY SUITE. The seeded list came from the
    # plan's §8 table, so it could only ever contain §8 refusals plus the one
    # §6a entry someone remembered. These two are refusals mt actually emits,
    # each with a test that existed and an entry that did not:
    "§3b",    # all-elided input: no prefix to restore from, and mt will not guess
    "§8.2c",  # input values: required when a PSBT lacks them, and PER INPUT
    "§1.1",   # the reassembled transaction must re-derive the content id
    "§1.1e",  # a MISSING or EXTRA character, named as such rather than as a lost plate
]
cited = {e["spec"] for e in entries}
for r in REQUIRED:
    if r not in cited:
        problems.append(f"the seeded list rules {r} and no entry claims it")
for c in sorted(cited - set(REQUIRED)):
    problems.append(
        f"{c} is cited but is not in the seeded set -- either the plan's list "
        "changed and this gate was not updated, or the reference is a typo"
    )

seen = {}
for e in entries:
    seen.setdefault(e["test"], []).append(e["spec"])
for k, v in seen.items():
    if len(v) > 1:
        problems.append(f"duplicate test {k!r}, claimed by {v}")

# 4. THE OTHER DIRECTION, which is the half that catches a new refusal: a test
#    that LOOKS like a refusal test but is in no entry. Named by convention --
#    `refuses_*` -- so adding one without an entry is caught.
listed = {e["test"] for e in entries}
for t, (path, body) in sorted(declared.items()):
    if t in listed or t in EXEMPT:
        continue
    # TWO signals, so the check cannot be dodged by renaming: the convention,
    # and the thing a §8 refusal test actually does.
    why = []
    if t.startswith("refuses_"):
        why.append("named refuses_*")
    if "REFUSED — §" in body:
        why.append("asserts on a §-refusal message")
    if "assert_refused(" in body:
        why.append("calls assert_refused")
    if why:
        problems.append(
            f"`{t}` ({path}) {' and '.join(why)}, but has no entry in "
            "refusals.toml -- add one, or add it to EXEMPT with a reason"
        )

if problems:
    print("check-refusal-coverage: FAILED")
    for p in problems:
        print(f"  - {p}")
    sys.exit(1)

print(f"check-refusal-coverage: {len(entries)} refusal tests over "
      f"{len(cited)} ruled refusals, each with a test that exists and a check "
      "that resolves")
for e in entries:
    print(f"  {e['spec']:<7} {e['test']}")
PY
