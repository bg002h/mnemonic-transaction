#!/usr/bin/env bash
# THE WHOLE THING, END TO END, AGAINST A REAL NODE — finishing at the only
# question that matters: would Bitcoin Core accept what came off the engraving?
#
# Every other gate in this repo tests mt against mt. The pinned vectors were
# produced by an encoder that uses the same constants the decoder does; the
# journeys assert on mt's own output; the stubs answer what they were told to.
# NONE OF THEM CAN TELL YOU THE BYTES ARE BROADCASTABLE. This can:
#
#     encode a real finalized PSBT  ->  mt1 strings
#     verify them                   ->  the content id re-derives
#     inspect them WITH the node    ->  live chain rows
#     decode them                   ->  raw hex
#     testmempoolaccept             ->  allowed: true
#
# IT CANNOT RUN IN CI, and that is stated rather than hidden: CI has no bitcoind
# and no funded wallet. It is a command to run before trusting a release, on a
# machine that has both.
#
# Requires: a regtest bitcoind with a funded wallet. Point MT_RT at a wrapper
# that adds -regtest/-datadir/-rpcwallet, e.g.
#     #!/bin/sh
#     exec bitcoin-cli -regtest -datadir=/path/to/regtest -rpcwallet=w "$@"
set -euo pipefail
cd "$(dirname "$0")/.."

RT="${MT_RT:-}"
if [ -z "$RT" ] || [ ! -x "$RT" ]; then
  echo "SKIP: set MT_RT to an executable bitcoin-cli wrapper for a funded regtest node."
  exit 0
fi
MT=target/debug/mt
[ -x "$MT" ] || { echo "FAIL: $MT not built. Run: cargo build"; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "== building a finalized PSBT from the node's own UTXOs =="
python3 - "$RT" "$WORK" <<'PY'
import json, subprocess, sys
rtbin, work = sys.argv[1], sys.argv[2]
def rt(*a):
    r = subprocess.run([rtbin, *a], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"FAIL {a}: {r.stderr.strip()[:300]}")
    s = r.stdout.strip()
    try:    return json.loads(s)
    except Exception: return s

u = sorted(rt("listunspent"), key=lambda x: -x["amount"])
if not u:
    sys.exit("FAIL: the wallet has no spendable outputs. Mine some blocks first.")
u = u[0]
dest = rt("getnewaddress", "", "bech32")
ins  = json.dumps([{"txid": u["txid"], "vout": u["vout"]}])
outs = json.dumps([{dest: round(float(u["amount"]) - 0.0005, 8)}])
signed = rt("walletprocesspsbt", rt("utxoupdatepsbt", rt("createpsbt", ins, outs)))
if not signed["complete"]:
    sys.exit("FAIL: the wallet could not finalize the PSBT.")
open(f"{work}/tx.psbt", "w").write(rt("finalizepsbt", signed["psbt"], "false")["psbt"])
print(f"   spending {u['amount']} BTC from {u['txid'][:16]}:{u['vout']}")
PY
chmod 600 "$WORK/tx.psbt"

echo "== 1. encode =="
"$MT" encode --bitcoin-cli "$RT" --in "$WORK/tx.psbt" >"$WORK/plates.txt" 2>"$WORK/enc.err"
chmod 600 "$WORK/plates.txt"
n=$(wc -l <"$WORK/plates.txt")
[ "$n" -gt 0 ] || { echo "FAIL: encode produced no strings"; exit 1; }
grep -q "STATUS    LIVE" "$WORK/enc.err" || { echo "FAIL: the node did not report LIVE"; sed 's/^/   /' "$WORK/enc.err"; exit 1; }
echo "   $n strings, STATUS LIVE"

echo "== 2. verify =="
"$MT" verify --in "$WORK/plates.txt" 2>"$WORK/ver.err" >/dev/null
grep -q "transaction re-derives" "$WORK/ver.err" || { echo "FAIL: no re-derivation"; exit 1; }
sed 's/^/   /' "$WORK/ver.err" | head -1

echo "== 3. inspect, with the node =="
"$MT" inspect --bitcoin-cli "$RT" --in "$WORK/plates.txt" >"$WORK/ins.out" 2>/dev/null
grep -q "STATUS    LIVE" "$WORK/ins.out" || { echo "FAIL: inspect did not see the chain"; exit 1; }
grep -E "^mt1 SET|^TX |^FEE |^STATUS" "$WORK/ins.out" | sed 's/^/   /'

echo "== 4. decode =="
"$MT" decode --bitcoin-cli "$RT" --in "$WORK/plates.txt" >"$WORK/rt.hex" 2>/dev/null
[ -s "$WORK/rt.hex" ] || { echo "FAIL: decode produced nothing"; exit 1; }

echo "== 5. WOULD THE NODE ACCEPT IT? =="
verdict=$("$RT" testmempoolaccept "[\"$(cat "$WORK/rt.hex")\"]")
echo "$verdict" | python3 -c '
import json, sys
d = json.load(sys.stdin)[0]
if not d.get("allowed"):
    print("   REJECTED:", d.get("reject-reason", "?"))
    sys.exit(1)
fee = d.get("fees", {}).get("base", "?")
print("   allowed: true, fee", fee, "BTC")
'

echo
echo "live-smoke-test: the bytes recovered from the engraving are a transaction"
echo "                 this node would broadcast."
