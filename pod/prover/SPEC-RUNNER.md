# PoD Runner — especificación normativa v1

**Subfase 1.2** de [`..._FASE1.md`](../../2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §3.
Complementa a [`../spec/SPEC.md`](../spec/SPEC.md), que define el **pedido**; este documento
define la **ejecución**, el **resultado** y el **recibo**.

Este documento es normativo. Si el runner y este texto difieren, el árbitro es este texto —
porque es lo que un challenge de la subfase 1.4 va a tener que resolver.

---

## 0. Qué tiene que ser cierto

Un tercero que tenga (a) los bytes del pedido, (b) los bytes del dataset y (c) el recibo,
DEBE poder recalcular el mismo `output_hash` en otra máquina y otro sistema operativo.

Eso obliga a que **todo** lo que entra al resultado sea función de esos bytes. En concreto,
el runner NO DEBE leer el reloj, la red, variables de entorno, el orden del sistema de
archivos, ni ninguna fuente de aleatoriedad, en ninguna parte del cálculo. El reloj se lee
una sola vez, para el campo de tiempo del recibo, que está **fuera** de `output_hash` (§4).

---

## 1. Modelo numérico

Toda la aritmética de simulación es en **binario64 IEEE-754**, usando exclusivamente
`+ - * /` y comparaciones. Esas cuatro operaciones están definidas al bit por el estándar y
dan el mismo resultado en cualquier plataforma que lo cumpla. **Ninguna función
trascendental** (`exp`, `log`, `pow`, `sqrt`, trigonométricas): no están garantizadas al bit
entre bibliotecas matemáticas, y una sola de ellas rompe el gate de esta subfase.

Conversión de los campos del pedido (SPEC §6) al modelo numérico:

| Campo del pedido | Conversión | Nota |
|---|---|---|
| decimal (`tp_mult`, `min_mcap`, `min_growth`, `min_ratio`) | `float(texto)` | conversión decimal→binario64 correctamente redondeada |
| puntos básicos (`*_bps`) | `bps / 100` → **porcentaje** | se trabaja en porcentaje, no en fracción, para que la aritmética coincida con la del backtester de referencia |
| lamports (`notional_lamports`, `fee_lamports_per_tx`, `panic_lamports`, `*_dev_lamports`) | `lamports / 1000000000` → SOL | |

La conversión de bps a porcentaje es exacta para todo múltiplo de 100, que es el caso de
todas las rejillas de referencia. Para los demás valores es el redondeo correcto de la
división, que también es determinístico.

---

## 2. Ejecución

### 2.1 Carga del dataset

1. Los archivos se ordenan **por nombre** y se concatenan sin separador. El SHA-256 de esa
   concatenación DEBE coincidir con `inputs.dataset.hash`; si no coincide, el runner aborta.
   No hay ejecución sobre un dataset que el pedido no haya fijado.
2. Cada línea no vacía es un registro JSON con `k` en `c` / `t` / `m` / `g`. Una línea que
   no parsea se **descarta**, igual que hace el backtester de referencia.
3. Los registros se agrupan por `m` (mint). Para cada token: `create` = último registro `c`
   visto, `ticks` = todos los `t`, `grad` = todos los `g`, `migrate_at` = el `t` del último
   registro `m`.
4. `ticks` y `grad` se ordenan por `t` con **orden estable**; el desempate es el orden de
   aparición en el dataset, que es fijo porque los bytes lo son.
5. El día de un token es la fecha UTC de `t0`, donde `t0` es el `t` de `create` si existe,
   si no el `t` del primer tick, si no `migrate_at`. Se computa con división entera de
   milisegundos por 1000 — sin punto flotante.
6. **Los tokens se recorren en orden ascendente de mint** (bytes del string base58). Este
   orden reemplaza al orden de inserción del backtester de referencia y es lo que hace que
   el resultado no dependa de cómo quedó escrito el archivo.

### 2.2 Partición

- `split.kind = none.v1`: se optimiza y se reporta sobre todos los tokens.
- `split.kind = last_day_holdout.v1`: sea `D` el mayor día presente. **Train** = tokens con
  día distinto de `D`; **test** = tokens con día `D`. El barrido corre sobre train; la
  validación de §3 corre sobre test.

### 2.3 Reglas de entrada

**`survivor.v1`** — recorre los ticks acumulando `buys`, `sells`, el conjunto de wallets
compradoras, y el primer y último market cap no nulo. Para cada tick con edad ≥
`min_age_min` (y abortando si la edad supera `max_age_min`), entra si
`len(buyers) >= min_buyers`, `last_mcap >= min_mcap`, `last_mcap/first_mcap >= min_growth` y
`buys/sells >= min_ratio` (con `buys` como razón cuando `sells = 0`). Requiere ≥ 3 ticks.

**`sniper.v1`** — entra en el `create` si `min_dev_lamports <= create.sol <= max_dev_lamports`.
Sale además por dos causas propias: **token muerto** (pasaron más de `stall_s` sin registros)
y **pánico** (una venta del creador, o una venta de al menos `panic_lamports`). Requiere
`create` y ≥ 2 ticks.

**`graduacion.v1`** — sobre la serie `grad`. Con `dip_bps = 0` entra en el primer registro
post-migración; con `dip_bps > 0` espera un retroceso de `dip_bps` desde el máximo, aborta si
el retroceso llega a `abort_bps` (eso es un dump, no un retroceso) y se rinde pasados
`timeout_min`. Requiere `migrate_at` y ≥ 2 registros `grad`.

### 2.4 Simulación de salida

Compartida por las tres estrategias. Precio efectivo de entrada
`entry * (1 + slippage)`; cada venta liquida a `p * (1 - slippage)`; cada compra y cada venta
descuentan `fee`. Reglas evaluadas **en este orden** por cada tick: stop duro → stop por
tiempo → take profit → trailing. El trailing se activa si ya hubo take profit, o si
`trail_always`, o si el pico superó `trail_arm_bps`. Si el pedido lo permite, el primer
disparo de trailing previo al take profit puede ser parcial, y en ese caso el pico se
rearma en el precio actual. Si la serie se agota con la posición abierta, se cierra al
último precio conocido.

### 2.5 Tope de posiciones simultáneas

Las operaciones se ordenan por `(entry_t, mint)` y se recorren manteniendo las salidas
abiertas; una operación se descarta si al momento de su entrada ya hay
`max_open_positions` abiertas. El desempate por mint es necesario: sin él, dos entradas en
el mismo milisegundo se resolverían por el orden de iteración, y eso no es reproducible.

### 2.6 Cuantización

El PnL de cada operación se calcula en SOL (binario64) y se convierte a lamports enteros
**truncando hacia cero**, operación por operación. Los agregados se suman ya en enteros.

Es una decisión con consecuencia: cuantizar por operación y sumar no da lo mismo que sumar y
cuantizar al final. Se elige por operación porque hace que `net_lamports` sea exactamente
`gross_win_lamports + gross_loss_lamports`, un invariante que un verificador puede chequear
sin repetir la simulación.

Por la misma razón, `wins` cuenta las operaciones cuyo valor **ya cuantizado** es > 0. Una
operación con ganancia positiva pero menor a un lamport cuenta como no ganadora: es lo único
consistente con los números que se emiten.

---

## 3. Barrido y resultado

### 3.1 Orden de enumeración

Los **parámetros barridos** son todos los campos de rejilla bajo `inputs.exit_policy` y
`inputs.strategy.params`. Se ordenan por su **ruta canónica completa** (`inputs.exit_policy.…`
ordena antes que `inputs.strategy.params.…`, y dentro de cada uno por nombre de clave).

La enumeración es el producto cartesiano recorrido como un cuentakilómetros: **el último
parámetro de esa lista es el que varía más rápido**. `combo_index` es la posición en esa
enumeración, empezando en 0.

> El backtester de referencia (`memebot/backtest.mjs`) **no** hace producto cartesiano: hace
> una búsqueda golosa en dos etapas (barre entradas con salidas fijas, después barre salidas
> con la mejor entrada). Este runner implementa el producto cartesiano, que es lo que declara
> SPEC §3.5. Son dos productos distintos: el cartesiano es exhaustivo y su resultado no
> depende de cómo se desempató la primera etapa. Ofrecer la golosa exige agregar una variante
> de búsqueda al pedido, y eso es un cambio de esquema, no del runner.

### 3.2 Ranking

Las combinaciones se ordenan por `net_lamports` **descendente**, y los empates por
`combo_index` **ascendente**. Se emiten las primeras `output_shape.top_n` (o todas, si hay
menos). El desempate explícito no es un detalle: sin él, dos configuraciones con el mismo
neto cambian de lugar según el algoritmo de ordenamiento y el `output_hash` deja de ser
reproducible.

### 3.3 Formato `sweep_top.v1`

```
{
  "schema_version": 1,
  "format":         "sweep_top.v1",
  "spec_hash":      "<64 hex>",
  "combos":         <int>,          // tamaño del producto cartesiano
  "evaluated":      <int>,          // tokens considerados en el barrido
  "rows": [
    { "rank": <int>,                 // 1..top_n
      "combo_index": <int>,
      "params": { "exit_policy": {…}, "strategy": {…} },
      "metrics": { … } },           // exactamente las claves de output_shape.metrics
    …
  ],
  "validation": { "kind": "none.v1" }
             |  { "kind": "last_day_holdout.v1", "evaluated": <int>, "metrics": {…} }
}
```

Todo el documento cumple SPEC §2–§4: enteros, strings del conjunto restringido, sin floats y
sin nulls. `params` repite los valores del pedido tal como estaban ahí (los decimales siguen
siendo strings), para que la fila se lea sin tener que reconstruir el `combo_index`.

La validación reporta las métricas de la **combinación de rank 1** evaluada sobre el
conjunto de test. No se re-optimiza sobre test: ese es justamente el punto.

```
output_hash = SHA-256( "PoD/Output/1" || 0x00 || bytes canónicos del documento )
```

---

## 4. Recibo

```
{
  "schema_version":   1,
  "spec_hash":        "<64 hex>",
  "output_hash":      "<64 hex>",
  "output":           { … },        // el documento completo de §3.3
  "runner_id":        "<64 hex>",   // clave pública Ed25519
  "started_unix_ms":  <int>,
  "wall_ms":          <int>,
  "peak_bytes":       <int>,
  "signature":        "<128 hex>"
}
```

La firma es Ed25519 (RFC 8032) sobre
`"PoD/Receipt/1" || 0x00 || bytes canónicos del recibo **sin** la clave `signature``.

`started_unix_ms`, `wall_ms` y `peak_bytes` **no son reproducibles y no tienen por qué
serlo**: quedan fuera de `output_hash` y adentro de la firma. Sirven para que el worker no
pueda desdecirse de lo que dijo que tardó, no para verificar el resultado. Un verificador que
los compare entre dos corridas está mirando el campo equivocado.

`peak_bytes` es el pico de asignación del intérprete, no el RSS del proceso: el RSS no es
portable entre sistemas operativos y el recibo tiene que poder emitirse igual en los dos.

---

## 5. Replay

Con el pedido, el dataset y el recibo:

1. Verificar que el SHA-256 del dataset coincide con `inputs.dataset.hash`.
2. Recalcular `spec_hash` del pedido y compararlo con el del recibo.
3. Verificar la firma del recibo contra `runner_id`.
4. Verificar que el `output` que trae el recibo hashea a su `output_hash` — atrapa un recibo
   armado a mano donde el hash y el contenido no se corresponden.
5. **Re-ejecutar** y comparar el `output_hash` obtenido con el del recibo.

Los pasos 1 a 4 son baratos y detectan recibos mal formados. El paso 5 es la verificación de
Nivel 0 propiamente dicha, y cuesta lo mismo que hacer el trabajo — por eso el default del
protocolo es optimista y esto solo corre cuando hay un challenge.

---

## 6. Fronteras declaradas

- **El determinismo se apoya en IEEE-754, no está demostrado formalmente.** Se apoya en que
  `+ - * /` son bit-exactas y en que no hay trascendentales. La evidencia empírica cubre dos
  arquitecturas (x86-64 y ARM64), dos sistemas operativos, tres intérpretes y dos runtimes
  —ver `PLATFORMS.tsv`— pero sigue siendo evidencia, no una prueba. Si alguna vez entra una
  función trascendental, esa evidencia deja de valer y hay que rehacerla.
- **Sin contenedor pineado todavía.** §3 de `..._FASE1.md` pide fijar el entorno con un
  digest de imagen. El pedido ya tiene el campo (`runner.image_digest`) y el runner lo
  registra, pero **no lo verifica contra el entorno en el que corre**: hoy nada impide
  ejecutar con un intérprete distinto al declarado. Cerrarlo exige construir la imagen.
- **La búsqueda es exhaustiva y no golosa** — ver §3.1.
- **Un `output_hash` reproducible no dice que el resultado sea el correcto**, solo que es el
  mismo. Es el riesgo 4 del documento de concepto y este documento no lo toca.
