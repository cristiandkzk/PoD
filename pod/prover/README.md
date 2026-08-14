# `pod/prover` — runner determinístico y recibo de entrega

Subfase **1.2** de [`..._FASE1.md`](../../2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §3.
La normativa está en [`SPEC-RUNNER.md`](SPEC-RUNNER.md); acá va cómo correrlo.

```
SPEC-RUNNER.md          normativo: ejecución, formato del resultado, recibo, replay
podprover/dataset.py    carga determinística del log de ticks
podprover/sim.py        port del simulador de memebot/backtest.mjs
podprover/sweep.py      barrido cartesiano, ranking, documento sweep_top.v1
podprover/ed25519.py    Ed25519 puro (RFC 8032), sin dependencias
podprover/receipt.py    armado, firma y chequeo del recibo
orders/*.json           tres pedidos: graduacion, sniper, survivor
fidelity/harness.mjs    el simulador ORIGINAL en Node, para comparar
scripts/gate.py         evidencia del gate de la subfase
```

Sin dependencias: solo la stdlib de Python, más `podspec` de la subfase 1.1 para la
canonicalización. Node se usa únicamente como oráculo en los chequeos, no en el runner.

## Correr

```bash
cd pod/prover
D=../../../memebot/data
python -m podprover keygen --out runner.key
python -m podprover run    --order orders/order-graduacion.json \
                           --dataset $D/ticks-2026-06-10.jsonl $D/ticks-2026-06-15.jsonl \
                           --key runner.key --out receipt.json
python -m podprover replay --order orders/order-graduacion.json \
                           --dataset $D/ticks-2026-06-10.jsonl $D/ticks-2026-06-15.jsonl \
                           --receipt receipt.json
python scripts/gate.py     # evidencia completa; necesita WSL y Node
```

`run` imprime el `output_hash` y nada más. `replay` sale 0 si reproduce el hash del recibo,
3 si no, 2 si el pedido o el dataset no validan.

## El pedido

Los tres pedidos usan el mismo dataset real: `ticks-2026-06-10.jsonl` +
`ticks-2026-06-15.jsonl` de `memebot/data`, concatenados por orden de nombre —
86990 registros, 16521 tokens, 2 días.

| Pedido | Estrategia | Partición | Combinaciones | Tiempo |
|---|---|---|---|---|
| `order-graduacion.json` | `graduacion.v1` | `none.v1` | 108 | ~2 s |
| `order-sniper.json` | `sniper.v1` | `none.v1` | 72 | ~3 s |
| `order-survivor.json` | `survivor.v1` | `last_day_holdout.v1` | 144 | ~15 s |

## Evidencia del gate

Los hashes esperados están congelados en [`EXPECTED_OUTPUT.tsv`](EXPECTED_OUTPUT.tsv) y las
plataformas que los reprodujeron en [`PLATFORMS.tsv`](PLATFORMS.tsv).

| Qué | Resultado |
|---|---|
| **Gate** — mismo `output_hash` en otra máquina, otro SO y otra arquitectura | 3/3 pedidos en **3 plataformas**: Windows/x86-64/CPython 3.14.4, Ubuntu 24.04 WSL2/x86-64/CPython 3.12.3, y Android-Termux/**aarch64**/CPython 3.14.6 |
| **Replay** desde el recibo, en el otro sistema | sale 0 y reproduce el hash |
| **Manipulación** | 5/5: alterar una métrica, recalcular el `output_hash`, cambiar de pedido, y correr con otro dataset, todos rechazados |
| **Entorno** | 6/6: `PYTHONHASHSEED`, `TZ`, `LC_ALL`, `PYTHONUTF8`, directorio de trabajo y orden de los archivos no mueven el hash |
| **Ed25519** propio vs OpenSSL | 5/5 casos, clave pública y firma byte a byte |
| **Fidelidad** vs el simulador original | **468/468** combinaciones idénticas |

La fidelidad es la mitad que no mide el gate: un `output_hash` reproducible dice que el
resultado es *el mismo*, no que sea *el correcto*. El harness de `fidelity/` corre el
simulador de `backtest.mjs` copiado tal cual, en Node, y compara las métricas de todas las
combinaciones — no solo del top.

## El gate, y lo que sigue abierto

**El gate se cumple.** La corrida en un Motorola Edge 40 Neo bajo Termux cierra el eje que
faltaba: otra máquina, otro sistema operativo y **otra arquitectura de CPU** (ARM64 contra
x86-64). Como Termux trae CPython 3.14.6 y Windows corre 3.14.4, ese cruce aísla el
procesador casi por completo — misma versión mayor de intérprete, mismo código, silicio
distinto. Los tres `output_hash` coinciden byte a byte.

Con eso, la hipótesis de SPEC-RUNNER §1 —que `+ − × ÷` sobre binario64 son bit-exactas entre
plataformas conformes, y que no usar trascendentales alcanza— tiene evidencia empírica en
dos arquitecturas, dos sistemas operativos, tres intérpretes y, vía el harness de fidelidad,
dos runtimes distintos (CPython y V8).

Quedan dos cosas abiertas, y ninguna es de determinismo:

**El entorno no está pineado.** `runner.image_digest` está en el pedido y el recibo lo
arrastra, pero **nada verifica que el intérprete que corrió sea el declarado**. Cerrarlo
exige construir la imagen y que el runner compruebe su propio digest.

**El recibo no basta para verificar.** §3 de `..._FASE1.md` dice "con solo el `spec_hash` y
el recibo". En la práctica hacen falta también los bytes del pedido y los del dataset: el
`spec_hash` los identifica, no los contiene. Quién publica esos bytes y dónde es un problema
de disponibilidad de datos que la subfase 1.4 tiene que resolver antes de que un challenge
sea resoluble por un tercero.
