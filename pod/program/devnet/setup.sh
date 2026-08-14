#!/usr/bin/env bash
# Prepara el despliegue en devnet: keypairs, saldo y compilacion a SBF.
set -euo pipefail
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/pod-target}"
cd "$(dirname "$0")"

solana config set --url https://api.devnet.solana.com >/dev/null

if [ ! -f program-keypair.json ]; then
  echo "buscando una direccion de programa que empiece con PoD..."
  solana-keygen grind --starts-with PoD:1 --ignore-case
  mv "$(ls | grep -iE '^pod.*\.json$' | head -1)" program-keypair.json
fi
for k in payer worker challenger arbiter; do
  [ -f "$k.json" ] || solana-keygen new --no-bip39-passphrase -s -o "$k.json" >/dev/null
done

echo "program   $(solana-keygen pubkey program-keypair.json)"
for k in payer worker challenger arbiter; do
  printf '%-10s %s  %s SOL\n' "$k" "$(solana-keygen pubkey $k.json)" \
    "$(solana balance $(solana-keygen pubkey $k.json) | cut -d' ' -f1)"
done
