# PoD WorkOrder — especificación normativa v1

**Subfase 1.1** de [`..._FASE1.md`](../../2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §2.

Este documento es **normativo**. Las implementaciones de `python/` y `rust/` se escriben
contra este texto, no una contra la otra. Si difieren, el árbitro es este archivo — y una
divergencia entre las dos es, por definición, una ambigüedad de este documento.

Palabras clave: **DEBE** / **NO DEBE** / **PUEDE**.

> **§6 está modelado sobre el backtester real** de `memebot/backtest.mjs`, no sobre un
> backtest de manual. Ver §10: la primera versión de este esquema asumía velas OHLC y un
> cruce de medias, y no podía expresar el trabajo que se quiere vender.

---

## 0. Qué produce esto

```
bytes de entrada (JSON) ──parse estricto──> valor ──validación──> WorkOrder
                                                                      │
                                                            serialización canónica
                                                                      │
                                                                      ▼
                                              spec_hash = SHA-256(dominio || bytes canónicos)
```

Una entrada que no valida **no tiene `spec_hash`**. No hay "hash de mejor esfuerzo": el
rechazo es parte del contrato, y dos implementaciones DEBEN rechazar las mismas entradas
**con el mismo código de error** (§7).

---

## 1. Principio de diseño: la ambigüedad se rechaza, no se normaliza

Cada vez que dos secuencias de bytes distintas podrían representar el mismo pedido, hay dos
salidas posibles: normalizar (mapear ambas a una forma) o rechazar (aceptar solo una). Este
formato **rechaza** salvo donde la normalización es trivialmente verificable.

La razón es el gate 3 de la subfase: cuanta más transformación haga el canonicalizador, más
superficie hay para que dos implementaciones difieran. Un validador estricto es más fácil de
replicar que un normalizador rico. El costo lo paga quien construye el pedido —que tiene un
formateador a mano— y no el protocolo.

Consecuencia visible: `"5.0"` no es un decimal válido. El válido es `"5"`.

---

## 2. Modelo de valores

Un `value` DEBE ser uno de:

| Tipo | Notas |
|---|---|
| `object` | claves `string` restringidas (§3.1) |
| `array` | ordenado; **DEBE** tener >= 1 elemento |
| `string` | conjunto de caracteres restringido (§3.2) |
| `int` | entero con signo de 64 bits (§3.3) |
| `bool` | `true` / `false` |

**No existen `null` ni `float`.** No son "desaconsejados": son un error de parseo (§7).

- `null` — un campo ausente y un campo nulo son estados distintos que se ven iguales; se
  elimina el segundo. Los campos opcionales se omiten.
- `float` — el motivo clásico: `0.1` no tiene la misma representación en todas las
  plataformas, y el redondeo de impresión difiere entre lenguajes. Los decimales van como
  `string` con la gramática de §3.4, y el dinero va en unidades base enteras (§6.8).

**Profundidad máxima de anidamiento: 8.** El objeto raíz está a profundidad 1.

---

## 3. Léxico

### 3.1 Claves

Una clave DEBE cumplir `^[a-z][a-z0-9_]{0,63}$`.

Solo ASCII en minúscula. Esto hace que el orden por bytes, el orden por code point y el
orden por code unit UTF-16 sean el mismo orden, y que NFC sea la identidad — tres fuentes
de divergencia entre lenguajes eliminadas por construcción y no por cuidado.

Dentro de un mismo objeto, dos claves iguales son un error (`E_DUP_KEY`), aunque el parser
del lenguaje anfitrión las colapse silenciosamente. Es obligación de la implementación
detectarlo **antes** de que el mapa las pierda.

### 3.2 Strings

Un `string` DEBE estar compuesto exclusivamente por caracteres del conjunto:

```
A-Z  a-z  0-9  - _ . : / @ + =
```

Longitud: 1..256 caracteres (todos ASCII, así que 1..256 bytes).

Esto es deliberadamente pobre: **todos los strings de un `WorkOrder` son identificadores**
—hashes, digests, nombres de estrategia, direcciones, nombres de métrica— y ninguno es
prosa. El formato no admite texto libre, lo cual no es una limitación sino el criterio de
kill local de §2 de `..._FASE1.md` expresado como gramática: si un pedido necesita una
frase, la clase de trabajo todavía no es demostrable.

**Sobre UTF-8 y NFC.** El requisito de §2 de `..._FASE1.md` es "UTF-8 NFC". Acá se cumple de
la forma más fuerte disponible: el conjunto permitido es un subconjunto de ASCII, donde NFC
es la identidad y toda entrada no-NFC es rechazada en vez de normalizada. La razón para no
aceptar Unicode general es concreta y no estética: **NFC depende de la versión de Unicode**.
Python 3.14 y una biblioteca de Rust cualquiera pueden estar en versiones distintas del UCD
y normalizar de manera distinta un carácter raro. Si NFC entrara al hash, `spec_hash`
dependería de la versión de Unicode de cada implementación — exactamente la clase de bug
transversal que esta subfase existe para atrapar. Ver §11 antes de ampliar esto.

**Escapes.** El parser DEBE aceptar los escapes de JSON (`\"` `\\` `\/` `\b` `\f` `\n` `\r`
`\t` `\uXXXX`) y decodificarlos. La validación de conjunto de caracteres se aplica **al
texto decodificado**: `"A"` y `"A"` son el mismo string y producen el mismo hash,
mientras que `"ñ"` se rechaza por `E_STRING_CHARSET`. El **serializador canónico nunca
emite un escape** — todo carácter permitido es imprimible y ASCII. Es una propiedad buscada:
dos implementaciones no pueden discrepar sobre reglas de escapado que ninguna ejecuta.

### 3.3 Enteros

Gramática de entrada: `^-?(0|[1-9][0-9]*)$`. Rango: -2^63 .. 2^63-1.

- Sin ceros a la izquierda (`01` → `E_INT_FORMAT`).
- Sin `+` inicial, sin exponente, sin punto decimal.
- `-0` es un error (`E_INT_FORMAT`): dos formas para el mismo valor.
- Cualquier token numérico con `.`, `e` o `E` es `E_FLOAT`, sin importar su valor. `1.0` se
  rechaza; el entero uno es `1`.
- `NaN`, `Infinity`, `-Infinity` son `E_FLOAT`.

Forma canónica de salida: idéntica a la de entrada (la gramática ya es canónica).

### 3.4 Decimales

Un decimal es un **string** que DEBE cumplir:

```
^-?(0|[1-9][0-9]*)(\.[0-9]*[1-9])?$
```

con como máximo 18 dígitos enteros y 18 fraccionarios, y con `-0` prohibido en cualquiera de
sus formas (`E_DECIMAL_FORMAT`).

O sea: sin ceros a la izquierda, **sin ceros a la derecha en la fracción**, sin punto final.

| Entrada | Resultado |
|---|---|
| `"0.5"` | válido |
| `"0.50"` | `E_DECIMAL_FORMAT` — la forma es `"0.5"` |
| `"5"` | válido |
| `"5.0"` | `E_DECIMAL_FORMAT` — la forma es `"5"` |
| `"05"` | `E_DECIMAL_FORMAT` |
| `".5"` | `E_DECIMAL_FORMAT` — la forma es `"0.5"` |
| `-0.5` (sin comillas) | `E_FLOAT` |

Un decimal es un string en la capa léxica: pasa además la validación de §3.2, que su
gramática satisface.

**Comparación de decimales.** Donde el esquema impone un rango u ordenamiento, la
comparación DEBE hacerse convirtiendo el decimal a un entero con escala fija de 10^18 (mover
el punto 18 lugares a la derecha, rellenando con ceros) y comparando enteros. Con el máximo
de §3.4 —18 dígitos enteros y 18 fraccionarios— el resultado entra en 128 bits con signo. Se
especifica el algoritmo y no solo el resultado porque "comparar dos decimales" es justo el
tipo de operación donde dos lenguajes usan bibliotecas distintas y difieren en el borde.

### 3.5 Rejillas de barrido (`grid`)

Varios campos de §6 son **rejillas**: el conjunto de valores que el barrido va a probar para
ese parámetro. Una rejilla es un `array` de >= 1 elemento, **estrictamente ascendente y sin
repetidos** (`E_NOT_SORTED`). Para rejillas de `int` el orden es numérico; para rejillas de
decimales, por el valor escalado de §3.4.

El orden obligatorio no es cosmético: **el orden de enumeración del barrido determina el
orden de las filas del output**, y por lo tanto el `output_hash` que la subfase 1.2 tiene que
reproducir. Sin esta regla, dos pedidos idénticos salvo el orden de una rejilla tendrían
`spec_hash` distinto y `output_hash` distinto, y ninguno de los dos sería un error detectable.

Una rejilla de un solo elemento es un parámetro fijo. Así, **un barrido de una combinación es
un backtest suelto**: la misma clase de trabajo cubre las dos granularidades sin un campo que
las distinga.

---

## 4. Forma canónica

La serialización canónica de un valor ya validado:

1. **`object`** → `{` `}` con los miembros separados por `,`, cada uno `"clave":valor`.
   Las claves se ordenan **ascendente por su secuencia de bytes UTF-8**. Como las claves son
   ASCII en minúscula (§3.1), este orden coincide con el orden por code point.
2. **`array`** → `[` `]` con los elementos separados por `,`, **en el orden dado**. El orden
   de un array es significativo y no se reordena. (Donde el orden no debería importar, el
   esquema exige que ya venga ordenado — ver §3.5 y `output_shape.metrics`.)
3. **`string`** → `"` seguido de los bytes del texto tal cual, seguido de `"`. Nunca hay
   escapes (§3.2).
4. **`int`** → su forma decimal de §3.3.
5. **`bool`** → `true` / `false`.
6. **Sin espacio en blanco** entre tokens. Ni indentación, ni salto de línea final.

La salida es ASCII puro. Los bytes canónicos son esa salida codificada en UTF-8, que para
ASCII es la identidad.

---

## 5. `spec_hash`

```
dominio    = "PoD/WorkOrder/1" || 0x00          (16 bytes)
spec_hash  = SHA-256( dominio || bytes_canónicos )
```

Representación textual: 64 caracteres hexadecimales **en minúscula**, sin prefijo.

**Por qué el prefijo de dominio.** El protocolo hashea más de una cosa. Sin separación de
dominio, un valor de un tipo puede ser reinterpretado como el de otro si sus bytes coinciden.
Agregarlo ahora es gratis; agregarlo después cambia **todos** los hashes emitidos, incluidos
los que ya estén liquidados en cadena. El `1` del dominio se mueve solo si cambia el formato
de forma incompatible, y en ese caso `schema_version` también cambia.

### 5.1 Registro de dominios

Cada dominio es una etiqueta ASCII seguida de un byte `0x00`. Todo lo que este protocolo
hashee DEBE usar uno de estos, y la regla de construcción es siempre la misma:
`SHA-256( dominio || bytes canónicos del valor )`.

| Dominio | Qué hashea | Definido en |
|---|---|---|
| `PoD/WorkOrder/1` | el pedido — `spec_hash` | este documento |
| `PoD/Output/1` | el resultado de la ejecución — `output_hash` | [`../prover/SPEC-RUNNER.md`](../prover/SPEC-RUNNER.md) §3 |
| `PoD/Receipt/1` | el recibo, sin su firma — lo que se firma | [`../prover/SPEC-RUNNER.md`](../prover/SPEC-RUNNER.md) §4 |

El registro vive acá y no en cada documento para que dos etiquetas no colisionen por
descuido: una colisión de dominio es indetectable hasta que alguien la explota.

---

## 6. Esquema del `WorkOrder`

**El esquema es cerrado.** Toda clave no declarada es `E_UNKNOWN_FIELD` y toda clave
requerida ausente es `E_MISSING_FIELD`. No hay campos opcionales en v1: las nueve claves de
raíz son obligatorias. Un esquema cerrado es lo que garantiza que `spec_hash` cubre el
pedido entero y que nadie puede colar un parámetro que el hash no vea.

### 6.0 Raíz

| Clave | Tipo | Regla |
|---|---|---|
| `schema_version` | `int` | DEBE ser `1` |
| `class` | `string` | registro §6.1 |
| `inputs` | `object` | dependiente de `class` (§6.2) |
| `runner` | `object` | §6.6 |
| `limits` | `object` | §6.7 |
| `output_shape` | `object` | §6.5 |
| `deadline` | `object` | §6.9 |
| `payment` | `object` | §6.8 |
| `proof_mode` | `object` | §6.10 |

### 6.1 Registro de clases

| `class` | Estado |
|---|---|
| `backtest.sweep.v1` | única clase de v1 |

Cualquier otro valor: `E_ENUM`. Una clase nueva es un cambio de esquema, no un string nuevo.

**Qué es esta clase.** Un barrido de parámetros de una estrategia de trading sobre un log de
ticks congelado, con validación fuera de muestra, que devuelve las mejores `top_n`
combinaciones. Es lo que hace `memebot/backtest.mjs`. Cumple las cuatro condiciones que PoD
necesita (§2 del plan maestro): es determinística, es cara (miles de combinaciones), el
output es chico frente a la ejecución, y hay alguien que la quiere.

### 6.2 `inputs` para `backtest.sweep.v1`

`inputs` = `{ dataset, split, strategy, exit_policy, portfolio, costs }`, las seis
obligatorias.

### 6.3 `inputs.dataset`

| Clave | Tipo | Regla |
|---|---|---|
| `hash` | `string` | `sha256:` + 64 hex minúscula |
| `format` | `string` | registro: `ticks.jsonl.v1` |
| `records` | `int` | >= 1 — cantidad de líneas |
| `tokens` | `int` | >= 1 — cantidad de mints distintos |
| `days` | `int` | >= 1 — días calendario UTC distintos |
| `first_unix_ms` | `int` | >= 0 |
| `last_unix_ms` | `int` | > `first_unix_ms` |

`ticks.jsonl.v1`: un objeto JSON por línea, con `k` en `c` (creación) / `t` (tick) /
`m` (migración) / `g` (tick post-graduación). Cuando el dataset son varios archivos, el
`hash` es el SHA-256 de la **concatenación de los archivos ordenados por nombre**, sin
separador agregado.

El dataset entra por hash de contenido, no por URL: una URL puede servir bytes distintos
mañana y el pedido dejaría de ser reproducible sin que su `spec_hash` cambie.

`records`, `tokens` y `days` son redundantes con el contenido a propósito: son un compromiso
verificable del pagador sobre lo que hay adentro, y le dan al runner cómo fallar rápido si el
dataset no es el que el pedido cree. `days` además determina si `split` puede aplicarse.

### 6.4 `inputs.split`

Unión etiquetada por `kind`:

| `kind` | Claves adicionales | Qué hace |
|---|---|---|
| `last_day_holdout.v1` | — | Optimiza con todos los días menos el último y reporta la validación con el último. Requiere `dataset.days >= 2` (`E_CONSTRAINT`) |
| `none.v1` | — | Sin validación fuera de muestra |

Que el split esté en el pedido y no en el runner es deliberado: es la diferencia entre
comprar "la mejor config" y comprar "la mejor config que además funcionó en datos que el
optimizador no vio", y son dos productos distintos con dos precios distintos.

### 6.5 `inputs.strategy`

Unión etiquetada por `kind`. **Cada parámetro es una rejilla (§3.5)**, no un escalar.

**`survivor.v1`** — entra cuando un token sobrevive y muestra tracción.

| Param | Tipo | Regla |
|---|---|---|
| `min_age_min` | grid de `int` | 0 .. 1440 |
| `max_age_min` | grid de `int` | 1 .. 1440 |
| `min_buyers` | grid de `int` | 1 .. 100000 |
| `min_mcap` | grid de decimal | > 0 |
| `min_growth` | grid de decimal | > 0 |
| `min_ratio` | grid de decimal | > 0 |

**`sniper.v1`** — entra en la creación, filtrando por lo que puso el creador.

| Param | Tipo | Regla |
|---|---|---|
| `min_dev_lamports` | grid de `int` | >= 0 |
| `max_dev_lamports` | grid de `int` | >= 0 |
| `stall_s` | grid de `int` | 1 .. 86400 |
| `panic_lamports` | grid de `int` | >= 0 |

**`graduacion.v1`** — entra en la migración, opcionalmente esperando un retroceso.

| Param | Tipo | Regla |
|---|---|---|
| `dip_bps` | grid de `int` | 0 .. 10000 — `0` = comprar al migrar |
| `timeout_min` | grid de `int` | 1 .. 1440 |
| `abort_bps` | grid de `int` | 0 .. 10000 |

### 6.6 `inputs.exit_policy`

La política de salida es un bloque propio y no parte de la estrategia, porque en el
backtester real es ortogonal: las tres estrategias comparten el mismo simulador de salidas.

| Clave | Tipo | Regla |
|---|---|---|
| `tp_mult` | grid de decimal | > 1 |
| `tp_sell_bps` | grid de `int` | 1 .. 10000 |
| `trail_bps` | grid de `int` | 1 .. 10000 |
| `trail_arm_bps` | grid de `int` | 0 .. 100000 — `0` = sin armado |
| `trail_sell_bps` | grid de `int` | 1 .. 10000 |
| `hard_stop_bps` | grid de `int` | 1 .. 10000 |
| `time_stop_min` | grid de `int` | 1 .. 1440 |
| `trail_always` | `bool` | no es rejilla: es una variante de política, no un parámetro |

### 6.7 `inputs.portfolio` y `inputs.costs`

**`portfolio`**

| Clave | Tipo | Regla |
|---|---|---|
| `max_open_positions` | `int` | 1 .. 10000 |
| `notional_lamports` | `int` | > 0 — tamaño fijo por posición |

**`costs`**

| Clave | Tipo | Regla |
|---|---|---|
| `slippage_bps_per_side` | `int` | 0 .. 10000 |
| `fee_lamports_per_tx` | `int` | >= 0 — costo fijo por transacción, no proporcional |

El fee es **fijo por transacción** y no en puntos básicos: en Solana el priority fee no
escala con el tamaño de la operación, y modelarlo como proporcional cambia el resultado
justo en las posiciones chicas, que son la mayoría.

### 6.8 `output_shape`

| Clave | Tipo | Regla |
|---|---|---|
| `format` | `string` | registro: `sweep_top.v1` |
| `top_n` | `int` | 1 .. 1000 |
| `metrics` | `array[string]` | >= 1, cada uno del registro de abajo, **estrictamente ascendente** (`E_NOT_SORTED`) |
| `rounding` | `string` | registro: `trunc_to_lamports.v1` |

Registro de métricas — **todas enteras y todas aditivas**:

| Métrica | Qué es |
|---|---|
| `gross_loss_lamports` | suma de los PnL negativos |
| `gross_win_lamports` | suma de los PnL positivos |
| `n_trades` | operaciones después del tope de `max_open_positions` |
| `net_lamports` | PnL neto |
| `unclosed` | posiciones abiertas al cortarse los datos |
| `wins` | operaciones con PnL > 0 |

**No hay `winrate` ni `profit_factor` en el output verificado, y es a propósito.** Las dos
son cocientes: `winrate` divide por `n_trades` y `profit_factor` por la pérdida bruta, que
puede ser cero. El backtester real devuelve `Infinity` en ese caso — un valor que JSON no
representa y que este formato prohíbe (§3.3). Emitir solo cantidades enteras y aditivas saca
la división del camino verificado; quien quiera los cocientes los calcula con estos números
y asume él la convención del caso degenerado. Un `output_hash` no puede depender de cómo
cada lenguaje imprime un infinito.

`rounding` declara cómo se cuantiza el PnL, que internamente es de punto flotante, a
lamports enteros. Sin esta declaración, dos runners honestos difieren en el último dígito y
un challenge de la 1.4 queda sin árbitro.

### 6.9 `runner`, `limits`, `deadline`

**`runner`**

| Clave | Tipo | Regla |
|---|---|---|
| `image_ref` | `string` | referencia del registry, <= 256 |
| `image_digest` | `string` | `sha256:` + 64 hex minúscula |
| `commit` | `string` | 40 hex minúscula |
| `toolchain` | `string` | identificador, ej. `node-22.22.3` |
| `entrypoint` | `array[string]` | >= 1 elemento |

`image_digest` es lo que fija el entorno; `image_ref` solo dice dónde buscarlo. Los dos van
en el hash: si el pedido apunta a otro registry, es otro pedido.

**`limits`**

| Clave | Tipo | Regla |
|---|---|---|
| `wall_time_s` | `int` | 1 .. 86400 |
| `memory_bytes` | `int` | 1 .. 2^40 |
| `cpu_count` | `int` | 1 .. 64 |

**`deadline`**

| Clave | Tipo | Regla |
|---|---|---|
| `accept_by_unix_s` | `int` | > 0 |
| `deliver_within_s` | `int` | 1 .. 2592000 — se cuenta **desde la aceptación**, no desde la creación |

### 6.10 `payment` y `proof_mode`

**`payment`**

| Clave | Tipo | Regla |
|---|---|---|
| `mint` | `string` | base58, 32..44 caracteres |
| `mint_decimals` | `int` | 0 .. 18 |
| `amount_base_units` | `int` | > 0 |
| `bond_base_units` | `int` | >= 0 |

**El dinero va en unidades base enteras**, nunca en decimales. `mint_decimals` está para
poder mostrarlo, no para calcularlo. Así el pago no toca la gramática de §3.4 y no existe la
pregunta de si `"5"` y `"5.00"` son el mismo pago.

`bond_base_units = 0` es representable a propósito: la subfase 1.3 tiene que poder construir
el caso "aceptar sin bond" y verificar que el contrato lo rechaza (gate 2 de 1.3). Que un
pedido sea expresable no significa que sea aceptable en cadena.

**`proof_mode`** — unión etiquetada por `kind`:

| `kind` | Claves adicionales | Estado |
|---|---|---|
| `optimistic` | `challenge_window_s` : `int` 1 .. 2592000 | implementado en 1.4 |
| `zk` | `verifier_key` : `string` `sha256:` + 64 hex | **solo interfaz**, ver §5 de `..._FASE1.md` |

Una clave que no corresponda a la variante es `E_UNKNOWN_FIELD`. El modo se elige por orden,
no globalmente (§3 del plan maestro).

### 6.11 Uniones etiquetadas: el tag se valida primero

`split`, `strategy` y `proof_mode` son uniones etiquetadas por `kind`. En ellas el recorrido
de §7.1 **no** se aplica directamente: primero se valida `kind` (`E_MISSING_FIELD` si falta,
`E_TYPE` si no es string, `E_ENUM` si no está en el registro) y recién con el tag resuelto se
determina el conjunto de claves declaradas y se recorre el resto en orden canónico.

Sin esta regla el formato sería ambiguo: en `proof_mode`, `challenge_window_s` ordena
**antes** que `kind`, así que un recorrido en orden canónico puro tendría que decidir si esa
clave es declarada sin haber leído todavía la variante. Dos implementaciones razonables
contestarían distinto.

**La raíz no es una unión etiquetada**, aunque `class` decida la forma de `inputs`: se
recorre con la regla común de §7.1.2, y funciona porque `class` ordena antes que `inputs`.
Es una coincidencia del alfabeto y conviene saberlo — si una v2 renombra alguno de los dos,
la raíz pasa a necesitar el tratamiento de esta sección.

### 6.12 Restricciones entre campos

Se evalúan al final, en este orden (§7.1.3):

El orden es el del recorrido canónico: cada restricción se evalúa al terminar de validar el
objeto que la contiene, y las de `inputs` recién cuando terminaron todos sus hijos.

| # | Restricción | Dónde |
|---|---|---|
| 1 | `last_unix_ms > first_unix_ms` | `inputs.dataset`, al cerrar ese objeto |
| 2 | `max(min_age_min) < max(max_age_min)` | `inputs.strategy` (`survivor.v1`) |
| 3 | `max(min_dev_lamports) <= max(max_dev_lamports)` | `inputs.strategy` (`sniper.v1`) |
| 4 | `split.kind = last_day_holdout.v1` exige `dataset.days >= 2` | `inputs`, al cerrar |

Las restricciones 2 y 3 se evalúan sobre el **máximo** de la rejilla, no sobre cada
combinación: una rejilla de `min_age_min` que en alguna combinación supere a `max_age_min`
produce simplemente cero operaciones para esa combinación, que es un resultado legítimo. Lo
que se rechaza es el pedido donde *ninguna* combinación puede operar.

---

## 7. Códigos de error

Dos implementaciones DEBEN coincidir en el código, no solo en el hecho de rechazar. Un
rechazo por el motivo equivocado es una divergencia: significa que las dos no comparten el
mismo modelo del formato, y mañana difieren en un caso donde una acepta.

| Código | Cuándo |
|---|---|
| `E_SYNTAX` | JSON mal formado, BOM, basura después del valor raíz, UTF-8 inválido |
| `E_NOT_OBJECT` | la raíz no es un objeto |
| `E_NULL` | apareció `null` |
| `E_FLOAT` | token numérico con `.`/`e`/`E`, o `NaN`/`Infinity` |
| `E_DUP_KEY` | clave repetida en un objeto |
| `E_KEY_CHARSET` | clave fuera de `^[a-z][a-z0-9_]{0,63}$` |
| `E_STRING_CHARSET` | string con caracteres fuera de §3.2, o longitud fuera de 1..256 |
| `E_INT_FORMAT` | ceros a la izquierda, `+`, `-0` |
| `E_INT_RANGE` | fuera de i64, o fuera del rango del campo |
| `E_DECIMAL_FORMAT` | decimal fuera de la gramática de §3.4 |
| `E_STRING_FORMAT` | string bien formado léxicamente pero fuera de la forma que exige el campo (`sha256:…`, hex de 40, base58) |
| `E_DECIMAL_RANGE` | decimal válido pero fuera del rango del campo |
| `E_EMPTY_ARRAY` | array de 0 elementos |
| `E_DEPTH` | anidamiento > 8 |
| `E_TYPE` | tipo distinto al que declara el esquema |
| `E_ENUM` | valor fuera de un registro cerrado |
| `E_UNKNOWN_FIELD` | clave no declarada |
| `E_MISSING_FIELD` | clave requerida ausente |
| `E_NOT_SORTED` | rejilla o array que el esquema exige ascendente y único |
| `E_CONSTRAINT` | restricción entre campos (§6.12) |

Cuando una entrada viola más de una regla, el código reportado es el de **la primera
violación en el orden de evaluación**. Este orden es normativo, porque de lo contrario dos
implementaciones correctas podrían reportar códigos distintos para la misma entrada.

### 7.1 Orden de evaluación

1. **Léxico y estructura**, durante el parseo, en el orden en que aparecen los bytes:
   UTF-8 → sintaxis JSON → `null` / float → formato de entero → conjunto de caracteres de
   clave → clave duplicada → conjunto de caracteres de string → array vacío → profundidad.
   La raíz se verifica objeto (`E_NOT_OBJECT`) apenas termina el parseo.
2. **Esquema**, recorriendo cada objeto **una sola vez** sobre la unión de sus claves
   presentes y sus claves declaradas, en orden canónico ascendente. Para cada clave de esa
   unión, en ese orden:
   - presente y no declarada → `E_UNKNOWN_FIELD`;
   - declarada y ausente → `E_MISSING_FIELD`;
   - en ambas → se valida el valor de inmediato (y se desciende, recursivamente, con esta
     misma regla).

   Consecuencia deliberada: en un objeto con una clave desconocida `a` y una requerida
   ausente `z`, el error es `E_UNKNOWN_FIELD`; con `z` desconocida y `a` ausente, es
   `E_MISSING_FIELD`. Arbitrario pero fijo, que es lo único que importa acá.
   Las uniones etiquetadas son la excepción — ver §6.11.
3. **Restricciones entre campos** (`E_CONSTRAINT`), una vez que todo el objeto validó, en el
   orden en que las lista §6.12.

El `path` que acompaña al código es **diagnóstico, no normativo**: ayuda a ubicar el
problema, pero dos implementaciones pueden diferir en él sin estar en desacuerdo sobre el
formato.

---

## 8. Vectores de prueba

`testvectors/valid/*.json` — cada uno con su `spec_hash` esperado en `testvectors/EXPECTED.tsv`.
`testvectors/reject/*.json` — cada uno con su código esperado en `testvectors/EXPECTED_REJECT.tsv`.

Los `.tsv` están **congelados**: una implementación nueva se valida contra ellos, y un cambio
en un hash esperado es un cambio de formato que exige subir `schema_version` y el dominio.

---

## 9. Lo que este formato le exige al runner (subfase 1.2)

No es parte de la validación, pero se declara acá porque son decisiones del **pedido** y no
del ejecutor, y la 1.2 las hereda:

- **Orden de enumeración del barrido**: producto cartesiano de las rejillas, variando el
  último parámetro más rápido, con los parámetros en el orden canónico de sus claves.
- **Desempate del ranking**: cuando dos combinaciones tienen el mismo `net_lamports`, el
  orden es el de enumeración. Un `sort` inestable acá cambia el `output_hash` sin cambiar
  nada real.
- **Cuantización**: `output_shape.rounding` define cómo se pasa de PnL en punto flotante a
  lamports enteros.
- **Sin transcendentales**: el simulador solo usa `+ - * /` y comparaciones sobre doubles
  IEEE-754, que son bit-exactas entre plataformas. Si alguna vez entra una `Math.*`, el
  determinismo cross-machine del gate de la 1.2 deja de estar garantizado.

---

## 10. Por qué §6 se rehizo

La primera versión de este esquema modelaba un backtest de manual: velas OHLC de un par,
cruce de medias, una configuración por pedido. Pasó los cuatro gates de la subfase 1.1 y era
**inútil**, porque el trabajo que se quiere vender es otro: `memebot/backtest.mjs` corre un
barrido de ~1000 combinaciones sobre un log de eventos tick a tick de miles de tokens, con
tres estrategias propias, una política de salida compartida y un tope de posiciones
simultáneas.

Lo encontró el gate 4 —"un backtest real se expresa completo en la spec"— recién cuando se
lo corrió contra el backtester real y no contra un pedido inventado. Vale registrar el modo
de falla: **un gate 4 evaluado sobre un ejemplo escrito por el mismo que escribió el esquema
siempre pasa.** El pedido de prueba tiene que venir de afuera del esquema.

Ninguna de las secciones §1 a §5 cambió. Que el rediseño haya tocado solo el esquema y no la
canonicalización es la señal de que la separación entre las dos capas era correcta.

---

## 11. Fronteras declaradas de v1

Escritas acá para que no se crucen por accidente:

- **Sin Unicode general.** Ver §3.2. Ampliarlo exige fijar una versión del UCD en el
  documento y verificar que todas las implementaciones la usen; hasta entonces, `spec_hash`
  quedaría atado a la versión de Unicode del intérprete.
- **Sin texto libre**, por el criterio de kill de §2 de `..._FASE1.md`.
- **Sin campos opcionales.** Una v2 con campos opcionales necesita una regla explícita de
  ausencia (¿un opcional ausente y uno con su valor por defecto hashean igual?), y esa regla
  es una fuente de divergencia. Se difiere hasta tener un caso real.
- **Una sola clase de trabajo**, por §9 de `..._FASE1.md`.
- **`spec_hash` no cubre el dataset, solo su hash.** La prueba cubre *ejecución*, no
  *intención* — el riesgo 4 del documento de concepto sigue en pie y este formato no lo
  resuelve, lo acota.
