#!/usr/bin/env bash
# Evidencia de los gates de las subfases 1.3 y 1.4 — ..._FASE1.md §4 y §5.
#
# Corre en Linux. En Windows el arbol de solana-program-test no compila: arrastra
# openssl vendorizado, que pide perl y nmake. Desde Windows:
#
#     wsl -d Ubuntu-24.04 -- bash /mnt/c/.../pod/program/scripts/gate.sh
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/pod-target}"
# El runtime de Solana escribe un DEBUG por instruccion; tapa la evidencia.
export RUST_LOG="${RUST_LOG:-error}"
cd "$(dirname "$0")/.."
echo "=========================================================================="
echo "GATES 1.3 y 1.4 — escrow, conservacion de balances y verificacion optimista"
echo "=========================================================================="
echo "  rustc  $(rustc --version)"
echo "  target $CARGO_TARGET_DIR"
echo
# Un binario por vez. Corriendo todo junto, las lineas que cargo escribe por stderr se
# intercalan con la salida del harness y el conteo queda ilegible.
total=0
for t in "" conservacion prohibiciones disputa maquina costo; do
  if [ -z "$t" ]; then nombre="unitarios (src/)"; args="--lib"; else nombre="tests/$t.rs"; args="--test $t"; fi
  n=$(cargo test $args 2>/dev/null | grep -oE 'ok\. [0-9]+ passed' | grep -oE '[0-9]+')
  printf '  %-24s %s tests
' "$nombre" "${n:-0}"
  total=$((total + ${n:-0}))
done
echo "  ------------------------------------"
printf '  %-24s %s tests
' "TOTAL" "$total"

echo
echo "El dato a registrar — costo por orden:"
cargo test --test costo -- --nocapture --quiet 2>/dev/null | sed -n '/COSTO POR ORDEN/,/tabla de E0.1/p'
