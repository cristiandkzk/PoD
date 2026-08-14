#!/usr/bin/env bash
# Compila el programa a SBF, que es la unica forma en que puede correr en una cadena de
# verdad. Los tests de `scripts/gate.sh` lo corren nativo, en proceso; esto comprueba que
# ademas exista como artefacto desplegable.
set -euo pipefail
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."
# --arch v3, no el v0 por default de cargo-build-sbf: agave ya activo
# `disable_sbpf_v0_execution`, asi que un .so v0 se despliega y no se puede ejecutar.
cargo build-sbf --arch "${ARCH:-v3}" --sbf-out-dir devnet/deploy
ls -la devnet/deploy/*.so
echo
echo "id declarado en el programa: $(sed -n 's/.*declare_id!("\([^"]*\)").*//p' src/lib.rs)"
echo "id del keypair de despliegue: $(solana-keygen pubkey devnet/program-keypair.json)"
