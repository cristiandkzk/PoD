# Acta de backtest — un resultado que no hace falta que me creas

> Ejecución real del runner de `pod/prover` sobre datos propios de mercado: un barrido de
> **108 configuraciones** sobre dos días reales. No se pide confianza en el número — están
> los hashes, el recibo firmado y el comando para reproducirlo.

| | |
|---|---|
| Clase de trabajo | `backtest.sweep.v1` |
| Configuraciones | 108 |
| Tokens evaluados | 16 521 |
| Registros del dataset | 86 990 |
| Tiempo de cómputo | 1 455 ms |

---

## 1. El problema

Cualquiera puede publicar una captura de un backtest espectacular. Nadie puede distinguirla de
una inventada, de una con datos elegidos a dedo, o de una donde se probaron mil variantes y se
mostró la mejor.

Por eso, en la práctica, un track record declarado no vale nada, y quien evalúa termina
construyendo su propia infraestructura para comprobarlo. Eso cuesta plata, y es la razón por la
que existen las evaluaciones con cuenta fondeada.

> Esto es un intento de resolverlo por el otro lado: que el resultado venga con todo lo
> necesario para que el escéptico lo verifique solo, sin pedirle nada a quien lo publica.

## 2. Lo que se pidió, fijado antes de ejecutar

```
spec_hash   a51f520b203f6add82f125773422c24eaa15c5d147a0a23a770d33756b0b18cc
dataset     d793297fe9349f53e7399229eb8749308a6d9b05a448bdaf015ad2f72ad0c25d
```

El pedido incluye el dataset por hash (86 990 registros, 16 521 tokens, 2 días), las comisiones,
el slippage, el tamaño de posición, el máximo de posiciones simultáneas y **la rejilla completa
de parámetros**. Nada de eso se puede cambiar después sin que cambie el `spec_hash`.

El formato canónico que hace que ese hash sea reproducible está en [`../spec`](../spec), con dos
implementaciones independientes que lo validan.

## 3. Lo que salió

```
output_hash  7c6ff4dc1543992ca6cfa6cf1950d7d43337bce9b21a8f0879a696de37600e76
```

| Puesto | Combo | `tp_mult` | `tp_sell` | `trail_bps` | `time_stop` | Trades | Ganadores | Neto (SOL) |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 81 | 2 | 100 % | 2000 | 30 min | 131 | 52 | **+0,1382** |
| 2 | 84 | 2 | 100 % | 3000 | 30 min | 131 | 52 | **+0,1382** |
| 3 | 87 | 2 | 100 % | 5000 | 30 min | 131 | 52 | **+0,1382** |
| 4 | 63 | 1.5 | 100 % | 2000 | 30 min | 132 | 64 | +0,0860 |
| 5 | 66 | 1.5 | 100 % | 3000 | 30 min | 132 | 64 | +0,0860 |
| 6 | 69 | 1.5 | 100 % | 5000 | 30 min | 132 | 64 | +0,0860 |
| 7 | 9 | 1.5 | 100 % | 2000 | 15 min | 133 | 60 | +0,0433 |
| 8 | 12 | 1.5 | 100 % | 3000 | 15 min | 133 | 60 | +0,0433 |

Los tres primeros puestos **empatan exactamente**, y la tabla muestra por qué: con `tp_sell` al
100 % la posición se cierra entera en el take-profit, así que no queda nada para que el trailing
stop administre y su parámetro no cambia un lamport. El desempate no lo elige quien publica —
está declarado en la especificación y es el número de combinación.

**Un ranking que nunca empata es un ranking al que alguien le puso la mano.**

## 4. El recibo

```
runner_id   3ab7050a0bbca3feeaaad200ee194cee00ebd88fdeffe1c7828f65d559f4ce91
firma       b09e5c15ada09f704cb6cdfd8e5188f4217e82eca33e6941c46a14aacc5d5361
            9105c6e0e869d22577eafeaeb845e0a24616ba5c35badee32acc0a27185e8705
```

Ed25519, implementado desde cero en Python puro (RFC 8032, [`podprover/ed25519.py`](podprover/ed25519.py)),
sin dependencias fuera de la stdlib.

La firma ata las tres cosas: qué se pidió, qué se entregó y quién lo ejecutó. Cambiar un solo
lamport de una métrica invalida el `output_hash`; recalcularlo para que cierre invalida la firma.

## 5. Verificarlo

```bash
python -m podprover replay \
    --order   orders/order-graduacion.json \
    --dataset ticks-2026-06-10.jsonl ticks-2026-06-15.jsonl \
    --receipt receipt.json

# 7c6ff4dc1543992ca6cfa6cf1950d7d43337bce9b21a8f0879a696de37600e76
# exit 0
```

`replay` sale **0** si reproduce el hash del recibo, **3** si no, **2** si el pedido o el dataset
no validan.

> **Los datasets no están en este repo.** Son los ticks de mercado de `memebot/data`, fuera del
> alcance de este proyecto. El pedido los fija **por hash**, así que cualquiera puede comprobar
> que son los mismos — pero para correr el replay hay que tenerlos. Esa brecha es exactamente la
> deuda de **disponibilidad de datos** declarada en
> [`../program/SPEC-PROGRAM.md`](../program/SPEC-PROGRAM.md) §10: el sistema fija *cuáles* son
> los bytes y no dice *dónde* conseguirlos.

### Determinismo cross-machine

| Plataforma | Arquitectura | Intérprete | `output_hash` |
|---|---|---|---|
| Windows 11 | x86-64 | CPython 3.14.4 | idéntico |
| Ubuntu 24.04 · WSL2 | x86-64 | CPython 3.12.3 | idéntico |
| Android · Termux | ARM64 | CPython 3.14.6 | idéntico |

Byte a byte, en tres máquinas, dos arquitecturas de procesador y tres intérpretes distintos. Se
corrió incluso en un teléfono.

Esa reproducibilidad no es un detalle técnico: es lo único que convierte «confiá en mí» en
«comprobalo», y es lo que permite que la resolución de una disputa sea un **procedimiento
repetible** en vez de la opinión de un árbitro.

## 6. Por qué no se puede elegir el resultado que conviene

Las 108 configuraciones estaban **enumeradas y hasheadas antes de ejecutar**. No se puede probar
mil variantes y mostrar la mejor, porque el espacio de búsqueda es parte del pedido y el pedido
está firmado.

> Es el mismo mecanismo que hace creíbles a los ensayos clínicos. No se confía en el
> laboratorio: se lo obliga a declarar el protocolo antes de empezar.

## 7. Lo que esto **no** prueba

Si esta sección no estuviera, el resto no valdría nada. La garantía es angosta y conviene decir
exactamente dónde termina:

- **No prueba que la estrategia sirva.** Dos días de datos y 131 operaciones no son evidencia de
  nada a futuro.
- **No prueba que los datos sean buenos.** Fija *cuáles* son, por hash. Si el dataset está
  sesgado, el resultado lo hereda — de forma verificable, eso sí.
- **No prueba que no haya sobreajuste** fuera de la rejilla declarada. Impide barrer y elegir a
  escondidas; no impide haber elegido bien la rejilla.
- **No impide elegir el _período_**, que es el fraude más común. El pre-registro de la rejilla no
  cubre eso, salvo que sea el comprador quien fije el dataset.
- **No es una auditoría.** Nadie firmó siendo responsable legal. Lo que hay es un resultado que
  cualquiera puede desmentir y, en el sistema completo, un depósito que se pierde si lo desmienten.

## 8. El hallazgo: por qué el proyecto está archivado

Esta acta se construyó para contestar una pregunta: *¿le destraba algo a alguien, o es una
solución elegante para un problema que nadie tiene?*

**La respuesta fue que no, y es la razón por la que PoD está archivado.**

La desconfianza de quien compra una estrategia se descompone en varias cosas. Esta acta resuelve
las dos primeras — *¿los números están inventados?* y *¿probaste mil variantes y me mostrás la
mejor?* — y **no toca la que domina**: *¿va a funcionar de acá en adelante?* Esa no la contesta
ninguna cantidad de criptografía, porque no es una pregunta sobre el pasado.

La evidencia externa apuntó al mismo lugar. El MQL5 Market es un mercado real, con dinero real,
donde el fraude con backtests está nombrado y documentado por su propia comunidad — y aun así el
mecanismo que la gente usa para creer no es una prueba criptográfica del backtest, sino la señal
en vivo verificada.

El razonamiento completo, con lo que quedó **validado por medición** separado de lo que era
**solo razonamiento**, está en [`../../ARCHIVO.md`](../../ARCHIVO.md).

> Existe demanda por verificar **la realidad**; no por verificar **el cómputo**. Esto último es
> lo que PoD resuelve, y lo resuelve bien.

---

*Los hashes de esta página salen de una ejecución real, no de un ejemplo. El escrow con depósito
y ventana de impugnación existe y corre sobre un validador de Solana local; no está desplegado en
ninguna red pública.*
