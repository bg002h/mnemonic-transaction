#!/usr/bin/env bash
# Do the COPIED design files still match their source?
#
# `design/PROVENANCE.md` says every file under `design/` is copied from
# `mnemonic-engrave` and never edited here. That is a promise, and a promise
# about two repositories is exactly the kind that goes quietly false: the spec
# gets folded over there, nobody re-copies, and this repo's tests start citing
# section numbers that have moved.
#
# THIS CANNOT RUN IN CI, and saying so is the point -- a gate that hides its own
# blind spot is worse than no gate. CI checks out ONE repository; the source
# lives in another that is not a submodule and not published. So this is a local
# command, to run before trusting a §-reference or re-copying anything.
set -euo pipefail
cd "$(dirname "$0")/.."

SRC="${MT_SPEC_SOURCE:-../mnemonic-engrave}"
if [ ! -d "$SRC/design" ]; then
  echo "SKIP: source repo not found at $SRC"
  echo "      (set MT_SPEC_SOURCE to point at mnemonic-engrave)"
  exit 0
fi

# Every file PROVENANCE.md claims is a copy, read from the table rather than
# listed here -- so adding a row to that table extends this check for free, and
# forgetting to add one is the only way to escape it.
# READ BOTH COLUMNS. The first version matched `design/...` only, silently
# skipping `crates/mt-codec/src/test_vectors/mt1_v1.json` -- the pinned vector
# corpus, which is the ONE copied file a drift would actually break, since every
# codec test asserts against it. Widening it then found the second problem: the
# source column is not always a path. One row reads "same generator run", so a
# path-shaped comparison fails on it, and a check that reports FAIL on a
# correctly-copied file gets switched off.
#
# So each row is resolved explicitly, and an unrecognised source is an ERROR
# rather than a skip -- a silent skip is how the first version lost a file.
python3 - "$SRC" <<'PYEOF'
import re, sys, hashlib, os
src = sys.argv[1]
rows = re.findall(r'^\| `([^`]+)` \| (.+?) \| `([0-9a-f]{40})` \|$',
                  open('design/PROVENANCE.md').read(), re.M)
if not rows:
    sys.exit("FAIL: PROVENANCE.md lists no copied files -- has its table changed shape?")

# Sources that are not a plain path, resolved by name. Stated here so the
# mapping is visible in the diff rather than inferred at runtime.
NAMED = {
    "same generator run": "design/vectors/mt1_v1_vectors.json",
}

failed = False
for here, source, _pin in rows:
    source = source.strip().strip('`')
    if source.startswith("mnemonic-engrave/"):
        there = source[len("mnemonic-engrave/"):]
        # A row may name a DIRECTORY; the file keeps its own basename.
        if there.endswith("/"):
            there += os.path.basename(here)
    elif source in NAMED:
        there = NAMED[source]
    else:
        print(f"{here:<44} FAIL — unrecognised source {source!r}")
        failed = True
        continue

    if not os.path.isfile(here):
        print(f"{here:<44} FAIL — missing here"); failed = True; continue
    p = os.path.join(src, there)
    if not os.path.isfile(p):
        print(f"{here:<44} FAIL — source {there} missing"); failed = True; continue
    a = hashlib.sha256(open(here, 'rb').read()).hexdigest()
    b = hashlib.sha256(open(p, 'rb').read()).hexdigest()
    print(f"{here:<44} {'identical' if a == b else 'DRIFTED from ' + there}")
    failed = failed or a != b

sys.exit(1 if failed else 0)
PYEOF
FAILED=$?

PIN=$(grep -oE '[0-9a-f]{40}' design/PROVENANCE.md | head -1 || true)
if [ -n "$PIN" ] && git -C "$SRC" cat-file -e "$PIN^{commit}" 2>/dev/null; then
  MOVED=$(git -C "$SRC" log --oneline "$PIN..HEAD" -- design/SPEC_mt_v0_1.md | wc -l)
  if [ "$MOVED" -gt 0 ]; then
    echo
    echo "NOTE: the spec has $MOVED commit(s) in the source since the pinned"
    echo "      commit $PIN. The copies above still match, so nothing is"
    echo "      broken — but the pin in PROVENANCE.md is stale."
  fi
fi

echo
[ "$FAILED" -eq 0 ] || { echo "check-provenance: FAILED"; exit 1; }
echo "check-provenance: every copied file matches its source"
