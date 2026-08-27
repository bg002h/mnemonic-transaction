#!/usr/bin/env bash
# The three journeys, end to end, asserting on WHAT THE OPERATOR SEES.
#
# Not on exit codes. A journey that checks only `$?` passes against a tool that
# succeeds in silence, and every one of these journeys is about a moment where
# the output IS the deliverable: a warning before an irreversible cut, a row
# that must say UNKNOWN rather than vanish, a notice about which plate to re-cut.
#
# Each journey prints the operator's real transcript before asserting, so this
# script is also the readable record of what mt does at those three moments.
#
# WHY THESE THREE. They were named nowhere -- no spec section enumerates them --
# so the implementation plan named them, and this is that list executed:
#
#   A  encode   an operator pastes a finalized PSBT and cuts
#   B  recover  2040: mt1 strings, no legend, no node
#   C  miscut   a string re-cut, the drawer holding both
set -euo pipefail
cd "$(dirname "$0")/.."

# 0600 FOR EVERY FILE THIS SCRIPT CREATES, and it is not housekeeping.
#
# §8.2h refuses a world-readable stdout, and the default umask here is 022 -- so
# `"$MT" encode ... >"$WORK/a.out"` hands mt a 0644 destination and mt correctly
# refuses to write the engraving into it. Under `set -e` that ends the run at
# journey A's first line.
#
# **This gate has never actually run against an unmutated binary.** CI's step
# order is refusal-coverage, refusal-mutation, journeys -- and until 2026-08-27
# `mutate-refusals.sh` restored the SOURCE while leaving `target/debug/mt`
# linked from its last mutation, which is `world_readable_stdout_guard`. The
# journeys therefore ran against a binary with that refusal deleted and passed.
# Fixing the mutation script exposed this; reproduced at the previous commit,
# where journeys fails identically once the binary is rebuilt.
#
# `umask 077` is the right fix rather than `--allow-world-readable`, because it
# is the FIRST remedy mt's own refusal offers -- so the script now does what the
# tool tells an operator to do, instead of overriding the tool. The explicit
# `chmod 600` below is kept: it covers the files python writes, which this umask
# does not reach.
umask 077

MT=target/debug/mt
[ -x "$MT" ] || { echo "FAIL: $MT not built. Run: cargo build"; exit 1; }
VECTORS=crates/mt-codec/src/test_vectors/mt1_v1.json
FIXTURES=crates/mt-cli/tests/fixtures/p5_base.json
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Force OFFLINE. Every journey here is air-gapped by design, and a machine that
# happens to run bitcoind would otherwise take a different path than CI does --
# so the gate would assert one thing locally and another in CI.
OFFLINE=(--bitcoin-cli /nonexistent/bitcoin-cli)

FAILED=0
say()  { printf '\n\033[1m%s\033[0m\n' "$*"; }
step() { printf '  %s\n' "$*"; }
have() {  # have <file> <needle> <why>
  if grep -qF -- "$2" "$1"; then
    printf '    ok   %s\n' "$3"
  else
    printf '    FAIL %s\n         expected to find: %s\n' "$3" "$2"; FAILED=1
  fi
}
lacks() {
  if grep -qF -- "$2" "$1"; then
    printf '    FAIL %s\n         should NOT contain: %s\n' "$3" "$2"; FAILED=1
  else
    printf '    ok   %s\n' "$3"
  fi
}

python3 - "$VECTORS" "$FIXTURES" "$WORK" <<'PY'
import json, sys
vec, fix, work = sys.argv[1], sys.argv[2], sys.argv[3]
v = json.load(open(vec))
f = json.load(open(fix))
even = [x for x in v["vectors"] if x["label"] == "even"][0]
open(f"{work}/tx.psbt", "w").write(f["finalized_psbt_b64"])
open(f"{work}/typed.txt", "w").write("\n".join(even["strings"]))
open(f"{work}/typed-elided.txt", "w").write("\n".join(even["strings_elided"]))

# Journey C's drawer: chunk 3 miscut ONCE and re-cut clean, both kept.
s = even["strings"]
bad = list(s[2]); bad[45] = "q" if bad[45] != "q" else "p"
open(f"{work}/drawer.txt", "w").write("\n".join(s + ["".join(bad)]))

# ...and one plate damaged PAST t = 4, beside its clean re-cut.
scrap = list(s[4])
for i in (20, 30, 40, 50, 60, 70):
    scrap[i] = "q" if scrap[i] != "q" else "p"
open(f"{work}/drawer-scrap.txt", "w").write("\n".join(s + ["".join(scrap)]))

# A single plate the operator mis-typed, with no second copy.
mistyped = list(s[0])
for i in (40, 55):
    mistyped[i] = "q" if mistyped[i] != "q" else "p"
open(f"{work}/mistyped.txt", "w").write("\n".join(["".join(mistyped)] + s[1:]))
PY
chmod 600 "$WORK"/*.txt "$WORK"/tx.psbt

# ── A — encode ───────────────────────────────────────────────────────────────
say "JOURNEY A — the operator pastes a finalized PSBT and cuts"
step "\$ mt encode < tx.psbt"
"$MT" encode "${OFFLINE[@]}" --in "$WORK/tx.psbt" >"$WORK/a.out" 2>"$WORK/a.err"
sed 's/^/    | /' "$WORK/a.err"
step "stdout (the artifact):"
sed 's/^/    | /' "$WORK/a.out"

step "assertions:"
# THE THREE MANDATORY BLOCKS. Each answers a question the operator cannot ask
# again once the plate is cut.
have "$WORK/a.err" "anyone holding this engraving can broadcast" "bearer warning present"
have "$WORK/a.err" "corrects up to 4 wrong CHARACTERS per string"  "correction-coverage block present"
have "$WORK/a.err" "verify the ENGRAVING"                          "verify-the-steel block present"
# §1.1's report rows.
for row in "TX " "OUT " "FEE " "LOCKTIME " "INPUTS " "STATUS " "CUT " "PREFIX "; do
  have "$WORK/a.err" "$row" "report row ${row% } present"
done
# THE ARTIFACT EXISTS AT ALL. Asserted FIRST and separately, because every
# check below it is a statement about the LINES of a file and all of them are
# vacuously true of a file with no lines. An earlier version of this journey had
# only those checks: mutating `mt encode` to write zero strings to stdout left
# the whole gate green, which is the precise failure the script's own header
# warns about -- a tool that succeeds in silence.
lines=$(wc -l < "$WORK/a.out")
if [ "$lines" -gt 0 ]; then
  echo "    ok   stdout carries $lines strings (the artifact is not empty)"
else
  echo "    FAIL stdout is EMPTY — mt reported success and engraved nothing"; FAILED=1
fi
# ...and it is the RIGHT number of them: the report's own CUT row says how many
# strings it cut, so the two halves of mt's output must agree.
cut_n=$(sed -n 's/^CUT  *\([0-9]*\) strings.*/\1/p' "$WORK/a.err")
if [ -n "$cut_n" ] && [ "$lines" -eq "$cut_n" ]; then
  echo "    ok   stdout line count matches the report's CUT row ($cut_n)"
else
  echo "    FAIL CUT row says '${cut_n:-?}' strings, stdout has $lines"; FAILED=1
fi
# stdout is the artifact and nothing else.
lacks "$WORK/a.out" "WARNING" "stdout carries no prose"
lacks "$WORK/a.out" "FEE"     "stdout carries no report"
bad=$(grep -cv '^mt1[0-9a-z]*$' "$WORK/a.out" || true)
if [ "$lines" -gt 0 ] && [ "$bad" -eq 0 ]; then
  echo "    ok   every stdout line is a lowercase, ungrouped mt1 string"
else
  echo "    FAIL $bad of $lines stdout lines are not lowercase ungrouped mt1 strings"; FAILED=1
fi

# ── B — recover ──────────────────────────────────────────────────────────────
say "JOURNEY B — 2040: mt1 strings, no legend, no node"
for form in typed typed-elided; do
  case "$form" in
    typed)        step "\$ mt inspect < typed-from-steel.txt      (full strings)" ;;
    typed-elided) step "\$ mt inspect < typed-from-steel.txt      (--elide-prefix form)" ;;
  esac
  "$MT" inspect "${OFFLINE[@]}" --in "$WORK/$form.txt" >"$WORK/b.out" 2>"$WORK/b.err"
  sed 's/^/    | /' "$WORK/b.out"
  sed 's/^/    | /' "$WORK/b.err"

  step "assertions ($form):"
  # A ROW IS NEVER OMITTED FOR BEING UNANSWERABLE. Omission and ignorance look
  # identical on a terminal, and the reader cannot tell a row that was skipped
  # from one that never existed.
  for row in "TX " "OUT " "FEE " "LOCKTIME " "INPUTS " "STATUS " "mt1 SET"; do
    have "$WORK/b.out" "$row" "row ${row% } present rather than omitted"
  done
  have "$WORK/b.out" "FEE       UNKNOWN"    "the fee says UNKNOWN, not nothing"
  have "$WORK/b.out" "current height unknown (no node)" "the height uses §8.4's offline spelling"
  have "$WORK/b.out" "STATUS    UNKNOWN"   "liveness says UNKNOWN"
  # READ vs VERIFIED, and BOTH ways out.
  have "$WORK/b.err" "what the transaction SAYS" "the read-vs-verified split is visible"
  have "$WORK/b.err" "None of it is confirmed"   "...and states what was NOT verified"
  have "$WORK/b.err" "mt inspect again with a bitcoind" "resolution names a node"
  have "$WORK/b.err" "block explorer"                   "resolution names an explorer"
  # A recoverer's decision is broadcast-or-not; "before cutting" names a
  # decision made years ago and the engraving already exists.
  lacks "$WORK/b.err" "before cutting" "not the encode-time wording"
done

# The two forms must produce the SAME artifact -- that is what makes eliding
# safe to do on steel.
"$MT" decode --in "$WORK/typed.txt" 2>/dev/null >"$WORK/b-full.hex"
"$MT" decode --in "$WORK/typed-elided.txt" 2>/dev/null >"$WORK/b-elided.hex"
if [ -s "$WORK/b-full.hex" ] && cmp -s "$WORK/b-full.hex" "$WORK/b-elided.hex"; then
  echo "    ok   the elided form decodes to the same transaction, with no flag"
else
  # `-s` FIRST: two empty files compare equal, so cmp alone would call a decode
  # that produced nothing "the same transaction".
  echo "    FAIL elided and full forms decode differently, or to nothing"; FAILED=1
fi

# ── C — miscut ───────────────────────────────────────────────────────────────
say "JOURNEY C — a string re-cut, the drawer holding both"
step "\$ mt verify < drawer.txt                   (chunk 3 present twice)"
"$MT" verify --in "$WORK/drawer.txt" >"$WORK/c.out" 2>"$WORK/c.err"
sed 's/^/    | /' "$WORK/c.err"

step "assertions:"
have "$WORK/c.err" "DUPLICATE RESOLVED. chunk 3" "the duplicate is announced, by chunk"
have "$WORK/c.err" "KEPT       the copy needing 0 of 4"      "says which copy it kept"
have "$WORK/c.err" "DISCARDED  the copy needing 1 of 4"      "says which it DISCARDED, and why"
have "$WORK/c.err" "it is the one to re-cut"                 "names the plate to act on"

step "\$ mt verify < mistyped.txt                 (one plate, two wrong characters)"
"$MT" verify --in "$WORK/mistyped.txt" >/dev/null 2>"$WORK/c2.err"
sed 's/^/    | /' "$WORK/c2.err"
step "assertions:"
have "$WORK/c2.err" "CORRECTION APPLIED"  "the margin report fires"
have "$WORK/c2.err" "2 of 4 symbols"      "...and quantifies the budget spent"
have "$WORK/c2.err" "pos "                "...localises each correction"
have "$WORK/c2.err" "read "               "...and gives the BEFORE-value"
have "$WORK/c2.err" "corrected to "       "...beside the after-value"

step "\$ mt verify < drawer-scrap.txt             (one plate damaged past t = 4)"
"$MT" verify --in "$WORK/drawer-scrap.txt" >/dev/null 2>"$WORK/c3.err"
sed 's/^/    | /' "$WORK/c3.err"
step "assertions:"
have "$WORK/c3.err" "mt verify: OK"        "a scrap plate does not kill a recoverable set"
have "$WORK/c3.err" "UNREADABLE STRING"    "the unreadable plate is named"
have "$WORK/c3.err" "cannot tell you which chunk that string was" "...and mt states the limit of what it knows"
lacks "$WORK/c3.err" "that plate is scrap" "mt does not assert a plate is scrap it never identified"

echo
if [ "$FAILED" -ne 0 ]; then
  echo "journeys: FAILED"
  exit 1
fi
echo "journeys: A, B (both forms) and C all pass on what the operator SEES"
