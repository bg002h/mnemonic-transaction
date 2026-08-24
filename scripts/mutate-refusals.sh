#!/usr/bin/env bash
# Prove every refusal test can FAIL.
#
# For each entry in crates/mt-cli/tests/refusals.toml this neuters the named
# `check` -- inserting `return Ok(());` at the top of its body -- runs ONLY that
# refusal's test, and asserts it goes RED. Then it restores the source.
#
# WHY THIS EXISTS. "Each refusal must be shown to fail when the refusal is
# removed" is a thing a person does once and nobody re-runs. A refusal test that
# passes against code with the check deleted is testing nothing, and this
# constellation has paid for that lesson twice.
#
# IT FAILS LOUDLY IF A MUTATION DOES NOT APPLY. A sed that matches nothing
# leaves the code intact, the test passes, and the run reports success -- a
# VACUOUS control, which has already happened twice in this cycle alone. Each
# mutation asserts the file changed before the test runs, and asserts the test
# was GREEN before it was mutated, because "red after" proves nothing about a
# test that was red already.
#
# WHAT IT DOES NOT COVER, stated because a gate that hides its blind spot is
# worse than no gate:
#   - It proves the test NOTICES the check. It does not prove the check is
#     CORRECT, that the message is right, or that the refusal fires on every
#     input it should. Those belong to a reviewer's execution pass.
#   - A `check` shared by two entries is neutered twice, once per entry. Both
#     runs are still valid controls; neither is redundant, since they run
#     different tests.
# ─────────────────────────────────────────────────────────────────────────────
# THIS SCRIPT OWNS THE WORKING TREE WHILE IT RUNS.
#
# Between every entry it does `rm -rf <crate>/src` and restores from a copy, so
# for the whole run the tree is not yours. DO NOT EDIT, FORMAT, COMMIT OR STASH
# anything in this repository until it exits.
#
# What happens if you do (observed 2026-08-24): a restore lands on top of your
# edit and reverts it, silently and partially -- other changes in the same file
# survive, so the tree still compiles and nothing announces the loss. Worse, the
# run reports a FAILURE that is not real, because the tree it tested was half
# your edit and half its own restore, and "the test is RED before any mutation"
# looks identical whether the cause is a broken test or a corrupted checkout.
#
# Safest is to run it against a COPY of the repo, where it cannot reach your
# working files at all.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

cd "$(dirname "$0")/.."
TOML=crates/mt-cli/tests/refusals.toml
CRATE=crates/mt-cli
[ -f "$TOML" ] || { echo "FAIL: $TOML not found"; exit 1; }

# RESTORE FROM A BYTE COPY, not from git. `git checkout -- src` would discard
# uncommitted work, which makes the gate unrunnable at exactly the moment it is
# most useful -- mid-fold, before the commit. A copy also means the script does
# not care whether this is a git repository at all.
BACKUP="$(mktemp -d)"
cp -a "$CRATE/src" "$BACKUP/src"
# TOUCH AFTER RESTORING. `cp -a` preserves mtimes, so the restored sources look
# OLDER than the artifacts built from the mutated ones -- cargo decides nothing
# changed, skips the rebuild, and the NEXT entry runs against the PREVIOUS
# entry's mutated binary. That is worse than a missing control: it reports a
# test as red for a mutation that is no longer in the file. Observed here, on
# the first run of this script: three entries reported "RED before any
# mutation" while passing individually.
restore() {
  rm -rf "$CRATE/src"
  cp -a "$BACKUP/src" "$CRATE/src"
  find "$CRATE/src" -name '*.rs' -exec touch {} +
}
cleanup() { restore; rm -rf "$BACKUP"; }
trap cleanup EXIT

mapfile -t ENTRIES < <(
  python3 - "$TOML" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
blocks = text.split("[[refusal]]")[1:]
for b in blocks:
    f = dict(re.findall(r'^\s*(spec|test|check)\s*=\s*"([^"]+)"', b, re.M))
    print(f"{f['spec']}\t{f['test']}\t{f['check']}")
PY
)

echo "mutate-refusals: ${#ENTRIES[@]} refusals to check"
echo

FAILED=0
for e in "${ENTRIES[@]}"; do
  IFS=$'\t' read -r SPEC TEST CHECK <<<"$e"
  FILE="$CRATE/${CHECK%%::*}"
  FN="${CHECK##*::}"
  printf '%-8s %-56s ' "$SPEC" "$FN"

  if [ ! -f "$FILE" ]; then
    echo "FAIL — no such file: $FILE"; FAILED=1; continue
  fi

  # 1. The test must be GREEN before it is mutated.
  if ! cargo nextest run --locked -E "test(=$TEST)" >/dev/null 2>&1; then
    echo "FAIL — the test is RED before any mutation"; FAILED=1; continue
  fi

  # 2. Neuter the named check, and ASSERT THE FILE CHANGED.
  if ! python3 - "$FILE" "$FN" <<'PY'
import sys
path, fn = sys.argv[1], sys.argv[2]
lines = open(path).read().split("\n")
start = None
for i, l in enumerate(lines):
    s = l.strip()
    if s.startswith(f"fn {fn}(") or s.startswith(f"pub fn {fn}("):
        start = i
        break
if start is None:
    sys.exit(f"no `fn {fn}(` in {path}")
# THE NEUTRAL RETURN, chosen by the signature. A guard is neutered by making it
# NOT FIRE, and what that looks like depends on how it reports: a refusing guard
# returns `Ok(Default::default())`, an advisory one returns `None`. One mutation
# CONCEPT, two spellings -- so a genuine refusal is never kept out of the ledger
# because of the shape of its return type.
sig = "\n".join(lines[start:start + 12])
neutral = "None" if "-> Option<" in sig else "Ok(Default::default())"
# The body opens at the first following line ending in `{`.
for j in range(start, min(start + 25, len(lines))):
    if lines[j].rstrip().endswith("{"):
        indent = " " * (len(lines[j]) - len(lines[j].lstrip()) + 4)
        lines.insert(j + 1, f"{indent}return {neutral}; // MUTATED by mutate-refusals.sh")
        open(path, "w").write("\n".join(lines))
        sys.exit(0)
sys.exit(f"could not find the body of `{fn}` in {path}")
PY
  then
    echo "FAIL — mutation did not apply"; FAILED=1; restore; continue
  fi
  if cmp -s "$FILE" "$BACKUP/${FILE#"$CRATE/"}"; then
    echo "FAIL — mutation reported success but the file is unchanged"; FAILED=1; restore; continue
  fi

  # 3. The test must now be RED.
  if cargo nextest run --locked -E "test(=$TEST)" >/dev/null 2>&1; then
    echo "FAIL — VACUOUS: the test still passes with the check removed"
    FAILED=1
  else
    echo "ok — red without the check"
  fi
  restore
done

echo
if [ "$FAILED" -ne 0 ]; then
  echo "mutate-refusals: FAILED"
  exit 1
fi
echo "mutate-refusals: all ${#ENTRIES[@]} refusal tests go red when their check is removed"
