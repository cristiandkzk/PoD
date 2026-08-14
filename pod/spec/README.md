# `pod/spec` — formato canónico de `WorkOrder` y `spec_hash`

Subfase **1.1** de [`..._FASE1.md`](../../2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §2.
La fuente de verdad del formato es [`SPEC.md`](SPEC.md); este README solo dice cómo correrlo.

```
SPEC.md                normativo. Las dos implementaciones se escriben contra este texto
python/podspec/        implementacion A  (stdlib: hashlib, re)
rust/                  implementacion B  (cero dependencias: SHA-256 y parser propios)
testvectors/valid/     8 pedidos validos + EXPECTED.tsv        (congelado)
testvectors/reject/    35 rechazos       + EXPECTED_REJECT.tsv (congelado)
scripts/make_vectors.py  regenera los vectores (solo si cambia el formato)
scripts/gate.py          evidencia de los 4 gates de la subfase
```

## Correr

```bash
cd pod/spec/rust && cargo build --release --offline
cd .. && python scripts/gate.py            # sale 0 si los cuatro gates pasan
```

CLI, mismo contrato de salida en los dos lenguajes — 64 hex en stdout, o
`<CODE>\t<path>` en stderr con salida 2:

```bash
cd python && python -m podspec hash  ../testvectors/valid/01_survivor_indent.json
              python -m podspec canon ../testvectors/valid/01_survivor_indent.json
rust/target/release/podspec hash testvectors/valid/01_survivor_indent.json
```

## Qué modela

La clase `backtest.sweep.v1` es el trabajo que hace [`memebot/backtest.mjs`](../../../memebot/backtest.mjs):
un barrido de parámetros de una estrategia de trading sobre un log de ticks congelado, con
validación fuera de muestra, que devuelve las mejores `top_n` combinaciones.

Las tres estrategias (`survivor.v1`, `sniper.v1`, `graduacion.v1`), la política de salida
compartida, el tope de posiciones simultáneas y los costos salen del backtester real, no de
un backtest de manual. Cada parámetro es una **rejilla** (SPEC §3.5); una rejilla de un solo
elemento es un backtest suelto, así que la misma clase cubre las dos granularidades.

El dataset del vector `01` es real: SHA-256 de `data/ticks-2026-06-17.jsonl` +
`data/ticks-2026-07-16.jsonl` concatenados por orden de nombre — 24105 registros,
8407 tokens, 2 días.

## Estado de los gates

| Gate | Criterio (§2 de `..._FASE1.md`) | Evidencia |
|---|---|---|
| 1 | 100 corridas y 3 permutaciones -> hash idéntico | 5 formas de bytes distintos, 500 evaluaciones en Python + 100 en Rust + 10 procesos con `PYTHONHASHSEED` aleatorio -> 1 hash |
| 2 | Cambio semántico mínimo -> hash distinto | `min_buyers` `[25,40,60]`→`[25,40,61]` cambia el hash; 8 vectores dan 4 hashes, sin colisiones |
| 3 | Segunda implementación reproduce los vectores bit a bit | 8/8 hashes, 8/8 bytes canónicos, 35/35 códigos de rechazo (y 35/35 paths, que no era obligatorio) |
| 4 | Un backtest real se expresa completo, sin texto libre | 13 strings de registro cerrado, 17 con patrón fijo, 4 de identidad del runner |

## Qué está congelado

`testvectors/EXPECTED.tsv` y `EXPECTED_REJECT.tsv`. Cambiar un valor de ahí es cambiar el
formato, y eso exige subir `schema_version` **y** el dominio del hash (SPEC §5) — no es una
corrección de test.

El `spec_hash` del pedido del gate 4:

```
61af8eaafc66be7ce57cab3174313747a99f3511a60b82a5bd4eda1e806558f0
```

## Dos limitaciones conocidas

**Independencia del gate 3.** Las dos implementaciones no comparten lenguaje, runtime ni una
sola dependencia — el SHA-256 de Rust está escrito a mano y el de Python viene de `hashlib`.
Pero **las escribió el mismo autor**, a partir del mismo `SPEC.md`. Eso atrapa errores de
transcripción, supuestos de codificación y diferencias de biblioteca; no atrapa un
malentendido compartido del propio documento. La forma de cerrar esa brecha es una tercera
implementación escrita por alguien más, con `SPEC.md` como única entrada.

**El esquema todavía no se ejecutó.** Que un pedido sea expresable no prueba que el runner
pueda cumplirlo. `SPEC.md` §9 lista lo que este formato le exige a la subfase 1.2 —orden de
enumeración del barrido, desempate del ranking, cuantización a lamports— y ninguna de esas
tres está verificada contra código que corra.
