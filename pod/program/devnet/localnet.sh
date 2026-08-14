#!/usr/bin/env bash
# Levanta un validador local, despliega el .so y corre el gate 1.4 en cadena.
#
# Por que local y no devnet: el faucet publico de devnet limita por IP y no suelta los ~2 SOL
# que cuesta un despliegue. Un validador local corre el MISMO runtime, la misma VM de SBF, el
# mismo presupuesto de computo, las mismas fees y un reloj real; lo unico que no da es una red
# compartida con terceros. La diferencia esta declarada en README.md.
set -euo pipefail
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.local/bin:$PATH"
cd "$(dirname "$0")"

LEDGER="${LEDGER:-$HOME/pod-ledger}"
RPC=http://127.0.0.1:8899

limpiar() { [ -n "${VPID:-}" ] && kill "$VPID" 2>/dev/null || true; }
trap limpiar EXIT

echo "levantando el validador..."
rm -rf "$LEDGER"
solana-test-validator --ledger "$LEDGER" --reset --quiet &
VPID=$!
for _ in $(seq 1 60); do
  solana cluster-version --url $RPC >/dev/null 2>&1 && break
  sleep 1
done
echo "validador arriba: agave $(solana cluster-version --url $RPC)"

# El despliegue lo paga una cuenta propia. Si lo pagara el pagador de las ordenes, el gate
# estaria midiendo su saldo mezclado con el costo del deploy.
[ -f deployer.json ] || solana-keygen new --no-bip39-passphrase -s -o deployer.json >/dev/null
for k in payer worker challenger arbiter deployer; do
  solana airdrop 10 "$(solana-keygen pubkey $k.json)" --url $RPC >/dev/null
done

echo "desplegando pod_escrow.so ($(stat -c%s deploy/pod_escrow.so) bytes)..."
solana program deploy deploy/pod_escrow.so \
  --program-id program-keypair.json --keypair deployer.json \
  --url $RPC --commitment confirmed
echo "costo del despliegue: $(echo "10 - $(solana balance $(solana-keygen pubkey deployer.json) --url $RPC | cut -d' ' -f1)" | awk '{print $1-$3}') SOL"

echo
node gate.mjs $RPC
