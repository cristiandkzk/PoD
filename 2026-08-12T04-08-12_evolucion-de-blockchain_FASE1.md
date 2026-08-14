# 2026-08-12T04-08-12_evolucion-de-blockchain_FASE1

**Fase 1 — PoD v0: escrow verificable, sin moneda propia**
Plan maestro: [`..._PLAN.md`](2026-08-12T04-08-12_evolucion-de-blockchain_PLAN.md) §3 · Concepto: [`..._blockchain.md`](2026-08-12T04-08-12_evolucion-de-blockchain.md)

Este documento reemplaza a §3 del plan maestro como fuente de verdad de la Fase 1. El plan maestro conserva el resumen y el gate de salida; acá está la división en subfases y el protocolo de ejecución.

---

## 0. Protocolo de checkpoint (la regla que ordena todo el documento)

**Se ejecuta una subfase por vez. Al terminarla se para y se pregunta. No se empieza la siguiente sin respuesta explícita.**

Esto no es ceremonia: la Fase 1 es la primera donde se escribe código, o sea la primera que genera costo hundido. Cada subfase agrega una pieza sobre la que las siguientes se apoyan; si una pasa mal, las tres que vienen atrás heredan el error y el rediseño cuesta cinco veces más. El checkpoint es el único momento barato para cambiar de opinión.

**Qué se presenta en cada checkpoint** — en este orden, sin adornos:

1. **Evidencia contra el gate**: comando y salida real. No "el determinismo funciona", sino las 100 corridas y su hash.
2. **Qué se rompió** en el camino, incluido lo que se arregló y no estaba previsto.
3. **Qué cambia hacia adelante**: si lo aprendido invalida o reordena alguna subfase siguiente.
4. **Costo real vs. estimado**, en días.

**La pregunta es siempre la misma, y tiene cuatro respuestas válidas:**

| Respuesta | Cuándo | Qué pasa |
|---|---|---|
| **Continuar** | El gate se cumplió con evidencia | Arranca la subfase siguiente |
| **Repetir** | El gate no se cumplió | Se vuelve sobre la misma subfase; no se avanza |
| **Rediseñar** | El gate se cumplió pero era el gate equivocado | Se corrige este documento antes de seguir |
| **Archivar / bifurcar** | Se disparó un criterio de kill | Se aplica §10 del plan maestro |

**Dos reglas duras, heredadas de §10 del plan maestro:**

- Gate no cumplido → la respuesta por default es *Repetir* o *Rediseñar*. **Nunca "continuar y lo arreglamos en la subfase que viene"**: es la misma trampa que ajustar el parámetro hasta que el KPI dé bien.
- **Nada de trabajo especulativo hacia adelante entre checkpoints.** Si 1.3 termina temprano, no se empieza 1.4 "mientras tanto". El valor del checkpoint es que la información llegue *antes* del gasto, no después.

---

## 1. Mapa de subfases

| Subfase | Qué produce | Gate de salida (verificable, no opinable) | Esfuerzo |
|---|---|---|---|
| **1.1 — Spec** | Formato canónico de `WorkOrder` + `spec_hash` | Dos implementaciones independientes producen el mismo hash sobre los mismos vectores | 3–5 días |
| **1.2 — Runner** | Ejecución determinística + recibo de entrega | Un tercero, en **otra máquina**, reproduce `output_hash` desde el recibo | 4–6 días |
| **1.3 — Escrow** | Contrato: dinero, bond, timeouts. Sin verificación | Todo camino conserva balances; ningún camino deja fondos atrapados | 1–1.5 semanas |
| **1.4 — Optimista** | Ventana de challenge, fraud proof, slashing | Fraude deliberado detectado y castigado en devnet; challenge falso castigado también | 1–1.5 semanas |
| **1.5 — `WorkSettled`** | Disponibilidad de datos resuelta + evento congelado + indexer smoke | `W(t)` calculable y **auditable**, esquema versionado, los cinco desenlaces distinguidos | 1–1.5 semanas |
| **1.6 — E2E** | SDK mínimo + dos procesos sin humano | Devnet sostenido, después mainnet con montos mínimos | 1 semana |
| **D — Demanda** *(paralelo)* | Contrapartes reales | 3 contrapartes que no seas vos, antes de 1.6 | continuo desde día 1 |

Suma: **5.5–7.5 semanas**, arriba de las 4–6 del plan maestro. La diferencia es honesta: el plan maestro estimó sin la subfase 1.1, que es la pieza que §3 marca como "la que más se subestima". El track D corre en paralelo, no suma.

> **Corrección del checkpoint 1.4.** La 1.5 pasó de 3–4 días a 1–1.5 semanas porque absorbió la disponibilidad de datos (§6.3), que 1.4 dejó abierta y que no se puede posponer sin congelar un evento incompleto. No es scope creep: es trabajo que ya existía y no estaba agendado en ninguna parte, que es la forma más cara de tenerlo.
>
> **Dato del costo real, para calibrar lo que queda:** 1.1 a 1.4 salieron muy por debajo de lo estimado —el grueso del tiempo se fue en toolchain, no en diseño ni en código—. Eso *no* es una razón para achicar la estimación de 1.5: las cuatro que salieron rápido eran subfases con una respuesta correcta computable, y 1.5 tiene tres decisiones de arquitectura (§6.3) que no se resuelven compilando.

**Nada de §3 se perdió en la división** — mapeo de control:

| Componente de §3 del plan maestro | Dónde vive ahora |
|---|---|
| `spec_hash` | 1.1 |
| Escrow (*no hay prueba sin pagador*) | 1.3 |
| Bond del worker | 1.3 (depósito) + 1.4 (slashing) |
| Verificación `Optimistic` / `Zk` | 1.4 (optimista implementado, ZK solo interfaz) |
| Evento `WorkSettled` | 1.5 |
| Máquina de estados completa | 1.3 (CREADA/ACEPTADA/CANCELADA) + 1.4 (ENTREGADA/LIQUIDADA/FALLIDA) |
| Gate de salida de fase | 1.6 |

---

## 2. Subfase 1.1 — Spec canónica y `spec_hash`

**Objetivo:** que un pedido tenga una representación en bytes única y reproducible. Sin esto no hay nada que hashear, y sin hash no hay entrega verificable.

**Qué se construye** — `pod/spec/`:

- Esquema mínimo de `WorkOrder`: `schema_version`, `class`, `inputs` (hash del dataset + parámetros), `runner` (identidad exacta del ejecutor: commit + toolchain + imagen), `limits`, `output_shape`, `deadline`, `payment`, `proof_mode`.
- Canonicalización: orden de claves, UTF-8 NFC, sin nulls, **sin floats** (decimales como string — el float es la fuente clásica de hashes que difieren entre plataformas).
- `spec_hash` = SHA-256 sobre los bytes canónicos.
- `pod/spec/testvectors/` — vectores congelados, incluidos los casos hostiles (mismo objeto con claves permutadas, espacios distintos, unicode equivalente).

**Gate de salida:**

1. 100 corridas y 3 permutaciones de orden de claves → hash idéntico.
2. Un cambio semántico mínimo (un parámetro) → hash distinto.
3. **Una segunda implementación, en otro lenguaje, reproduce los vectores bit a bit.** Este es el gate real; los otros dos los pasa cualquier canonicalización rota.
4. Un backtest real de la clase elegida se expresa completo en la spec, sin ningún campo de texto libre.

**Criterio de kill local:** si expresar el pedido real exige un campo libre, el problema no es la spec — es que **la clase de trabajo elegida todavía no es demostrable**. Se corrige la clase, no se agranda el esquema. Un campo libre en la spec es un agujero por donde después entra "prueba correcta ≠ resultado correcto" (riesgo §8).

> **Checkpoint 1.1** → presentar evidencia y preguntar antes de tocar 1.2.

---

## 3. Subfase 1.2 — Runner determinístico y recibo de entrega

**Objetivo:** que cualquiera pueda verificar por re-ejecución (Nivel 0). Es la línea base de la escalera y el **árbitro** de todo lo que viene: sin re-ejecución reproducible, un challenge en 1.4 no se puede resolver.

**Qué se construye** — `pod/prover/`:

- Modo `run`: `spec_hash` → ejecución → `output` + `output_hash`.
- Modo `replay`: `spec_hash` + recibo → verifica que `output_hash` coincide.
- Recibo firmado por el worker: `{spec_hash, output_hash, output, runner_id, tiempo, memoria}`.
- Fijación del entorno: contenedor con digest, semillas fijas, sin reloj, sin red, sin orden de iteración dependiente del hash map.

**Gate de salida:** un tercero, **en otra máquina y otro sistema operativo**, con solo el `spec_hash` y el recibo, obtiene el mismo `output_hash`. El determinismo en una sola máquina no cuenta y es fácil de confundir con el bueno.

**Criterio de kill local:** si el determinismo cross-machine no se consigue, **no se sigue a 1.3**. El contrato entero asume un árbitro reproducible; construir el escrow arriba de un runner no determinístico es construir la parte cara sobre la parte rota. Salidas: pinear el entorno más fuerte, o cambiar de clase de trabajo.

> **Checkpoint 1.2** → presentar evidencia y preguntar antes de tocar 1.3.

---

## 4. Subfase 1.3 — Escrow on-chain, sin verificación

**Objetivo:** que el dinero se mueva bien antes de que exista cualquier prueba. Se construye el tramo aburrido de la máquina de estados, que es donde se pierden los fondos.

**Qué se construye** — `pod/program/` (Rust / Anchor, Solana):

- `create_order` (el pagador deposita — invariante *no hay prueba sin pagador*), `accept_order` (el worker deposita bond), `cancel_expired` (reembolso).
- Estados: `CREADA → ACEPTADA` y `CREADA → CANCELADA`. **`ENTREGADA` todavía no existe.**

**Gate de salida:**

1. Para cada camino, la suma de balances se conserva; ninguna cuenta queda con fondos huérfanos ni rent atrapado.
2. No se puede aceptar sin bond.
3. El pagador no puede cancelar después de `ACEPTADA`.
4. No existe ninguna ruta de retiro del escrow fuera de las declaradas.

**Dato a registrar (no es kill, pero afecta al KPI 1):** costo de rent + fees por orden. Si es comparable al valor de una tarea chica, el tamaño mínimo de tarea económicamente viable sube, y eso mueve la tabla de E0.1.

> **Checkpoint 1.3** → presentar evidencia y preguntar antes de tocar 1.4.

---

## 5. Subfase 1.4 — Verificación optimista (Nivel 1)

**Objetivo:** cerrar la máquina de estados con el camino que hace que el fraude cueste. Es la subfase con la decisión de diseño más comprometida de toda la fase.

**Qué se construye:**

- `deliver(output_hash, receipt)` → `ENTREGADA` + arranca la ventana.
- `challenge(evidence)` con **depósito del challenger** — un challenge gratis es un ataque de denegación gratis.
- `settle()` post-ventana → `LIQUIDADA`. Challenge exitoso → `FALLIDA`: bond al challenger, reembolso al pagador.
- Interfaz `Zk { verifier_key }` **definida y no implementada**, salvo que E0.1 la haya justificado.

**La decisión que hay que tomar acá, en voz alta:** cómo se resuelve un challenge.

| Opción | Costo | Honestidad |
|---|---|---|
| (a) Re-ejecución off-chain por un árbitro designado | Barato | **Centralizado.** Hay que declararlo, no esconderlo |
| (b) Bisección interactiva on-chain | Caro de construir | Trustless de verdad |
| (c) ZK solo del paso disputado | Depende de E0.1 | Trustless, costo medido en Fase 0 |

**Recomendación para v0: (a), con el árbitro declarado en el README y un plan de reemplazo escrito.** v0 no necesita ser trustless para medir demanda, y gastar tres semanas en bisección antes de saber si alguien pide trabajo es exactamente el error que §10 predice. Pero va escrito como **frontera declarada del sistema**, igual que el camino de disputa de la Fase 4 — no como detalle de implementación.

**Gate de salida:** en devnet, (1) un worker que entrega un `output_hash` falso es challengeado dentro de la ventana y pierde el bond; (2) un worker honesto challengeado en falso cobra igual y el challenger pierde su depósito.

**Criterio de kill local:** si la ventana de challenge necesaria (acotada por el tiempo de re-ejecución de la tarea grande) hace la liquidación inaceptablemente lenta para el caso de uso, **el default optimista no sirve para esta clase de trabajo**. Se anota contra el KPI 1 y se decide entre achicar la clase o activar ZK — no se acorta la ventana para que el número quede lindo.

> **Checkpoint 1.4** → presentar evidencia y preguntar antes de tocar 1.5.

---

## 6. Subfase 1.5 — `WorkSettled` congelado + indexer smoke

> **Esta sección se rehizo en el checkpoint 1.4, con respuesta *Rediseñar*.** El gate de 1.4
> se cumplió con evidencia en cadena, pero al construir 1.3 y 1.4 aparecieron cuatro hechos
> que la versión anterior de esta sección no podía conocer, y los cuatro cambian qué evento
> hay que congelar. Como este es el artefacto más caro de deshacer de toda la fase, corregir
> el documento cuesta una hora y rehacer el evento cuesta reindexar la historia. Lo que
> cambió está en §6.0; lo que se conserva intacto son las dos decisiones que ya estaban
> congeladas —`payer`/`worker` en claro y `schema_version` desde el primer evento— porque
> nada de 1.3 ni de 1.4 las tocó.

**Objetivo:** producir el insumo de la Fase 2. §3 del plan maestro lo dice sin vueltas: este evento es la salida más importante de toda la fase, y rehacerlo después obliga a reindexar la historia entera.

### 6.0 Los cuatro hechos que obligaron a corregir

**(a) La cuenta de la orden se cierra al liquidar, así que el evento no es una comodidad: es
el único registro que queda.** `pod/program/SPEC-PROGRAM.md` §4.4 cierra el PDA en todos los
caminos terminales —datos en cero, balance en cero— para no dejar rent atrapado. Eso está
bien y no se va a cambiar, pero tiene una consecuencia que la versión anterior de esta
sección no contemplaba: después de la liquidación **no hay nada en cadena** que diga quién
pagó, quién trabajó, cuánto, ni contra qué `spec_hash`. Todo eso vive en el evento o no
vive. Un campo que falte no se puede rellenar después con un `getAccountInfo`.

**(b) `order_id` no es único en el tiempo.** La dirección del PDA es función de
`(payer, spec_hash, nonce)` y el `nonce` lo elige el pagador. Como la cuenta se cierra, el
mismo pagador puede **recrear la misma dirección** con el mismo nonce y generar un segundo
trabajo con el mismo `order_id`. Un indexer que use `order_id` como clave primaria colapsa
los dos en uno y pierde trabajo real de `W(t)`.

**(c) 1.4 produjo cinco desenlaces, no dos.** El evento anterior tenía `WorkSettled` y
`WorkFailed`. Los caminos que existen hoy son: `settle` sin challenge, `resolve(INFUNDADO)`,
`resolve(FRAUDE)`, `cancel_expired` por árbitro ausente, y `cancel_expired` sin aceptar o
sin entregar. **El cuarto no es ni trabajo ni fraude**: es una disputa que nadie decidió.
Meterlo en cualquiera de los dos eventos corrompe `W(t)` o corrompe la tasa de fraude, que
son las dos cosas que la Fase 2 va a medir.

**(d) Los logs de una cadena pública no son un archivo.** Los eventos de Solana son logs de
transacción, y los RPC públicos podan. "Reindexar la historia" solo es posible si alguien la
guardó mientras pasaba. La Fase 2 §2.1 pide un indexer **canónico y reproducible**; eso
exige que 1.5 archive los eventos crudos desde el primer día, no que confíe en poder pedirlos
después.

### 6.1 Qué se construye — `pod/indexer/`

Tres eventos, no dos. Emitidos con `sol_log_data` (estructurado y de longitud declarada), no
con `msg!`, que trunca y no es una interfaz:

- **`WorkSettled`** — hubo trabajo y se pagó. Cuenta para `W(t)`.
- **`WorkFailed`** — hubo entrega y se probó falsa. **No** cuenta para `W(t)`, y es el
  numerador de la tasa de fraude.
- **`WorkVoided`** — la orden se deshizo sin veredicto. No cuenta para nada, y existe para
  que "no pasó nada" no se confunda con "pasó algo malo". Lleva un `reason`:
  `NOT_ACCEPTED`, `NOT_DELIVERED` o `ARBITER_ABSENT`. El tercero es el que importa: es la
  frontera declarada de la opción (a) de §5 volviéndose un número medible.

Campos de `WorkSettled`, con lo que (a) y (b) agregan a la lista original:

```
schema_version   u8    desde el primer evento
order_id         [32]  la direccion del PDA. ATRIBUTO, no clave (b)
payer            [32]  en claro, sin hashear
worker           [32]  en claro, sin hashear
value            u64   = reward_lamports, y NADA mas (§6.2)
class            str   la clase de trabajo, copiada del pedido (a)
proof_mode       u8    1 = optimista; 2 = zk, todavia no existe
spec_hash        [32]  que se pidio
output_hash      [32]  que se entrego
dataset_locator  ???   de donde se bajan los bytes (§6.3)
settled_via      u8    SETTLE | RESOLVE_UNFOUNDED  (c)
was_challenged   bool  redundante con lo anterior, y se pone igual: el dia que exista
                       un tercer camino de liquidacion, este campo va a seguir
                       significando lo mismo y `settled_via` no
settled_at       i64   unix seconds
```

Y el indexer mínimo: cadena → **archivo crudo de eventos** → base de datos → `W(t)` por
época. Las tres flechas, en ese orden, y el archivo crudo antes de la base de datos por (d).

### 6.2 `value` es la recompensa, y solo la recompensa

`value = reward_lamports`. **No** incluye el bond ni el depósito del challenger, y no es un
detalle contable: el bond lo pone el worker y el depósito el challenger, así que contarlos
como valor de trabajo dejaría que un worker infle `W(t)` **poniéndose un bond enorme a sí
mismo**. Es wash work con un disfraz distinto, y el lugar barato de cerrarlo es acá, en la
definición del campo, y no en el detector de la subfase 2.2.

### 6.3 La disponibilidad de datos se decide acá o no se decide nunca

Es la deuda que dejó abierta 1.4 y la razón principal por la que esta sección se rehizo. El
contrato fija un `spec_hash` y un `output_hash`; ninguno de los dos **contiene** los bytes
que hacen falta para verificar —el pedido y los 17 MB del dataset—. Un challenger que no los
consiga no puede challengear; un árbitro que no los consiga no puede resolver; un auditor de
Fase 2 no puede recalcular nada. Hoy la ventana de challenge presupone que esos bytes existen
en algún lado, y nada en el sistema dice dónde.

No es una decisión, son tres, y conviene responderlas en este orden:

1. **Quién publica.** El candidato natural es el **pagador**: es quien quiere el trabajo
   hecho y quien ya está obligado a definirlo. Un worker que publica sus propios datos puede
   publicar unos y ejecutar otros.
2. **Dónde.** Un locator en el pedido, y por lo tanto un cambio en `pod/spec/SPEC.md` §6, con
   el costo que eso tiene: **cambia el `spec_hash` de todo pedido y hay que rehacer los
   vectores congelados de 1.1**. Es la parte cara y hay que decirla antes de empezar, no
   descubrirla a mitad de camino.
3. **Qué pasa si los bytes desaparecen durante la ventana.** Es la pregunta con dientes y la
   que se olvida. Si el dataset se cae, el esquema optimista degrada en silencio a "confiar
   en el worker", que es exactamente lo que 1.4 existe para no hacer. Las salidas posibles
   —extender la ventana, que pierda el pagador, que pierda el worker— son todas caras, y no
   elegir ninguna es elegir la primera por default.

**Esto se resuelve antes de congelar el evento**, porque de (a) se sigue que si el locator no
está en `WorkSettled`, el vínculo entre un trabajo liquidado y sus bytes verificables se
pierde para siempre en cuanto la cuenta se cierra.

### 6.4 Lo que se congela y no se puede posponer

- **`payer` y `worker` van en claro, sin hashear.** El análisis anti-sybil de la Fase 2 es un grafo de financiamiento entre esas direcciones; si se anonimizan ahora, el gate de la Fase 2 se vuelve incomputable.
- **`schema_version` desde el primer evento.** Una migración futura tiene que poder leer la historia vieja sin reindexar.
- **La clave primaria del indexer es la firma de la transacción**, no `order_id` (b).
- **Los tres eventos existen desde el primer día**, aunque `WorkVoided` sea raro. Agregar un evento después obliga a reinterpretar todo lo anterior.
- **`value` excluye bond y depósito** (§6.2).

**Gate de salida:**

1. `W(t)` calculable desde la cadena, con esquema versionado y congelado por escrito.
2. Los cinco desenlaces de 1.4 producen el evento correcto, cada uno con su test. En
   particular: un `ARBITER_ABSENT` **no** mueve `W(t)` ni la tasa de fraude.
3. El indexer, apagado y vuelto a prender, reconstruye el mismo `W(t)` desde su archivo
   crudo, sin volver a pedirle nada a la cadena (d).
4. Una orden con el mismo `(payer, spec_hash, nonce)` liquidada dos veces aparece como
   **dos** trabajos, no uno (b).

**Criterio de kill local:** si la disponibilidad de datos no tiene una respuesta que un
tercero pueda ejecutar —bajar los bytes y recalcular— entonces el challenge de 1.4 es
decorativo y `W(t)` mide trabajo **declarado**, no trabajo **verificable**. Eso no mata la
Fase 1, pero **sí mata la Fase 3**: emitir contra un `W(t)` que nadie puede auditar es
exactamente el fracaso que §10 del plan maestro quiere descubrir barato. Se anota contra el
criterio de wash work y se decide antes de la Fase 2, no durante.

> **Checkpoint 1.5** → presentar evidencia y preguntar antes de tocar 1.6.

---

## 7. Subfase 1.6 — SDK + end-to-end de dos procesos (gate de salida de la fase)

**Objetivo:** el gate del plan maestro, tal como está escrito: dos procesos independientes, sin humano interviniendo, completan `pedido → ejecución → prueba → verificación → pago liberado`.

**Qué se construye** — `pod/agent-sdk/`: `request_work(spec, budget)` / `fulfill(order)` / `claim(proof)`.

**Gate de salida:**

1. **Devnet sostenido:** N órdenes seguidas sin intervención humana, incluyendo al menos una cancelación por timeout y al menos un challenge resuelto. El camino feliz solo no alcanza — los caminos raros son los que rompen en producción.
2. **Mainnet con montos mínimos**, después de devnet, nunca antes.

**Advertencia sobre este gate:** se puede pasar con las dos puntas corriendo en la misma máquina, del mismo dueño. Eso es un test de integración válido y **wash work por construcción**. Por eso el track D es un prerrequisito y no un adorno: si a esta altura no hay una contraparte real, lo que se demostró es que el software anda, no que el mercado existe.

> **Checkpoint 1.6** → gate de salida de la Fase 1. Se pregunta antes de pasar a la Fase 2.

---

## 8. Track D — Demanda (paralelo, desde el día 1)

No es una subfase: corre en paralelo a todas.

§10 del plan maestro dice que el fracaso más probable de la Fase 1 no es técnico — es que nadie pida trabajo. Ese criterio de kill hoy no está agendado en ninguna parte, y un criterio de kill sin fecha se evalúa siempre al final, cuando ya se gastó todo.

**Qué se mide:** ¿existe alguien que pague por un backtest ejecutado por un tercero, en vez de correrlo él mismo?

> **Este checkpoint está vencido: 1.4 salió.** Se anota acá y no se da por cumplido en silencio. El track D no arrancó, así que hoy la respuesta es la segunda de las dos de abajo por default — y elegirla por default es exactamente lo que §10 del plan maestro no quiere.

**Checkpoint propio, en la salida de 1.4:** si no hay ninguna contraparte que no seas vos, el end-to-end de 1.6 va a ser auto-tráfico por construcción. Ahí la decisión es explícita — y se toma antes de gastar la semana de mainnet, no después:

- **Seguir igual**, sabiendo que 1.6 es un test técnico y no evidencia de demanda; o
- **Parar en 1.5** y volver a la premisa: si el cuello de botella nunca fue la verificación, eso es exactamente lo que §10 quería descubrir barato.

---

## 9. Qué queda explícitamente fuera de la Fase 1

Escrito acá para que no se cuele por el costado a mitad de una subfase:

- Token, emisión, `k`, gobernanza, DAO — Fase 3, y solo si pasa el gate de E0.2.
- ZK implementado — solo la interfaz en 1.4, salvo que E0.1 lo justifique con números.
- Múltiples clases de trabajo — una sola clase, la de E0.1.
- Reputación, árbitro descentralizado, camino de disputa para trabajo no demostrable — Fase 4.
- Cualquier optimización de gas que no haya sido disparada por una medición real de 1.3.
