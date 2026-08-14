#!/usr/bin/env bash
# El faucet de devnet es caprichoso: limita por IP y por rato. Se pide de a poco y se
# reintenta, en vez de pedir todo junto y que falle entero.
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
cd "$(dirname "$0")"
solana config set --url https://api.devnet.solana.com >/dev/null
for k in payer worker challenger arbiter; do
  p=$(solana-keygen pubkey $k.json)
  for intento in 1 2 3; do
    saldo=$(solana balance "$p" | cut -d' ' -f1)
    if [ "${saldo%%.*}" -ge 2 ] 2>/dev/null; then break; fi
    solana airdrop 2 "$p" >/dev/null 2>&1 || true
    sleep 3
  done
  printf '%-10s %s  %s SOL\n' "$k" "$p" "$(solana balance "$p" | cut -d' ' -f1)"
done
