# 2026-08-12T04-08-12_evolucion-de-blockchain_PLAN

**Plan de implementación — Prueba de Entrega (Proof of Delivery)**
Concepto origen: [`2026-08-12T04-08-12_evolucion-de-blockchain.md`](2026-08-12T04-08-12_evolucion-de-blockchain.md)

---

## 0. Principio rector del plan

Tres reglas que ordenan todo lo que sigue:

1. **No se construye una cadena.** PoD es una capa de liquidación; vive sobre lo que ya existe. Cualquier fase que empiece con "lanzar una L1" está mal planteada.
2. **La moneda es lo último, no lo primero.** El error clásico es diseñar el token en la fase 1. Acá el token solo existe si la Fase 0 demuestra que puede existir.
3. **Escalera de costo también en la verificación.** No arrancar en ZK "por las dudas". La verificación tiene niveles, y hay que medir cuál alcanza:

```
Nivel 0 — Hash de output + re-ejecución determinística   (verificar cuesta lo mismo que hacer)
Nivel 1 — Optimista: ventana de challenge + fraud proof  (camino feliz ≈ gratis)
Nivel 2 — ZK: prueba sucinta, verificación O(1)          (caro de generar, barato de verificar)
```

El concepto **no requiere ZK**. Requiere que la entrega sea verificable. El Nivel 1 puede alcanzar para la mayoría de los casos, y es un orden de magnitud más barato de construir.

---

## 1. Mapa de fases

| Fase | Qué produce | Gate para pasar | Esfuerzo aprox. |
|---|---|---|---|
| **0 — Falsación** | Dos números (KPI 1 y KPI 2) | Ambos KPIs viables | 2–3 semanas |
| **1 — PoD v0** | Escrow verificable en cadena existente, sin moneda | Dos agentes liquidan trabajo real end-to-end | 5–6.5 semanas |
| **2 — Registro** | Ledger de trabajo liquidado `W(t)` + indexer | `W(t)` real > 0 y mayormente no auto-generado | 3–4 semanas |
| **3 — Moneda** | Token con `E(t) = min(curva, k·W(t))` | Red team no logra farmear el subsidio | 8–12 semanas |
| **4 — Agentes** | SDK agente↔agente | Un agente contrata a otro sin humano en el loop | continuo |

Las fases 0→2 tienen valor **aunque el concepto completo falle**: quedan como infraestructura de escrow verificable, que es útil por sí sola.

---

## 2. Fase 0 — Falsación (empieza acá)

Objetivo: matar la idea barato si está muerta. Ninguna línea de protocolo en esta fase.

### E0.1 — KPI 1: overhead de verificación

**Clase de trabajo elegida:** *backtest determinístico de una estrategia sobre un dataset de velas congelado.*

Es el mejor candidato de arranque porque cumple las cuatro condiciones que PoD necesita:
- determinístico y replayable (mismo input → mismo output, siempre);
- lo suficientemente caro como para que alguien quiera comprarlo en vez de correrlo;
- el output es chico (un vector de métricas), la ejecución es grande — la asimetría que hace útil una prueba;
- conecta directo con el proyecto de trading de Solana que ya existe.

**Qué se mide, por cada nivel de la escalera:**

| Nivel | Costo de generar | Costo de verificar | Latencia hasta liquidar | Overhead = verif. / valor tarea |
|---|---|---|---|---|
| 0 — re-ejecución | 0 | = costo de hacer | inmediata | ~1.0 (inservible salvo como base) |
| 1 — optimista | 0 | ~0 en camino feliz | ventana de challenge | a medir |
| 2 — ZK (zkVM) | prover time × $ | verificación on-chain | minutos | a medir |

**Entregable:** una tabla con números reales, no estimaciones. Prover time, tamaño de prueba, costo de verificación on-chain, para tareas de 3 tamaños (1s, 30s, 10min de cómputo).

**Criterio de kill:** si el Nivel 1 y el Nivel 2 dan overhead > 1 en todos los tamaños realistas, la conclusión correcta es **"esperar a que baje el proving"**, no "diseñar la moneda igual". Se archiva el proyecto con la tabla adjunta y una fecha de revisión.

### E0.2 — KPI 2: ventana de `k`

Simulación pura, sin cadena. Un script.

**Modelo:** dos actores sobre el mismo mecanismo.
- *Honesto:* cobra fee `f` por trabajo real, paga costo de producción `c` y costo de prueba `p`. Gana `f − c − p + k·f`.
- *Auto-pagador (wash work):* se paga a sí mismo `f`, quema el fee, paga `p`, cobra el subsidio. Gana `k·f − f_quemado − p − gas`.

**Pregunta a responder:** ¿existe `k > 0` tal que el auto-pagador tenga retorno esperado **negativo** y el subsidio del honesto siga siendo **significativo** (digamos > 10% de su margen)?

**Entregable:** un heatmap de la región viable de `k` en función de (fee quemado, costo de prueba, gas).

**Criterio de bifurcación** — este es el gate más importante del plan:

- **Ventana de `k` existe** → sigue el plan completo, incluida la Fase 3 (moneda).
- **Ventana vacía o microscópica** → **la regla monetaria no existe.** No se cae el concepto entero: PoD sobrevive como primitivo de liquidación sobre stablecoins. Se ejecutan las Fases 1, 2 y 4, **se cancela la Fase 3**, y el proyecto es "escrow verificable para agentes", que sigue siendo real y útil.

---

## 3. Fase 1 — PoD v0: escrow verificable, sin moneda propia

> **Dividida en subfases:** la fuente de verdad para ejecutar esta fase es [`..._FASE1.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) — subfases 1.1–1.6 más un track de demanda en paralelo, con **parada obligatoria y pregunta entre cada subfase**. Lo que sigue es el resumen.

Pago en stablecoin existente. Sin token. Sin gobernanza. Sin DAO.

### Máquina de estados del `WorkOrder`

```
CREADA ──(worker acepta + deposita bond)──> ACEPTADA
   │                                            │
   │(deadline sin aceptar)                      │(entrega + prueba)
   ↓                                            ↓
CANCELADA                                   ENTREGADA
   (reembolso)                                  │
                                    ┌───────────┴───────────┐
                          (verificación OK)          (challenge exitoso)
                                    ↓                       ↓
                               LIQUIDADA                 FALLIDA
                          (pago liberado +          (bond al challenger,
                           evento WorkSettled)       reembolso al pagador)
```

### Componentes

- **`spec_hash`** — hash del pedido canonicalizado. Sin especificación determinística no hay entrega verificable; esta es la pieza que más se subestima. Definir un formato mínimo de work order (JSON canónico + hash) **antes** que el contrato.
- **Escrow** — el pagador deposita antes de que se ejecute nada. Invariante: *no hay prueba sin pagador*.
- **Bond del worker** — hace que el fraude cueste. En modo optimista es lo único que sostiene la seguridad.
- **Verificación** — dos modos desde el día 1: `Optimistic { challenge_window }` y `Zk { verifier_key }`. El modo se elige por orden, no globalmente.
- **Evento `WorkSettled { payer, worker, value, class, proof_mode }`** — la salida más importante de toda la fase. Es el insumo de `W(t)` en la Fase 2. Diseñarlo bien ahora evita rehacer todo después.

### Layout propuesto

```
pod/
├── spec/            formato canónico de work order + hashing
├── program/         contrato on-chain (escrow, bond, verificación, settle)
├── prover/          runner determinístico + generación de prueba (niveles 0/1/2)
├── indexer/         consumo de eventos WorkSettled → base de datos
├── agent-sdk/       request_work() / fulfill() / claim()
└── sim/             simulaciones de la Fase 0 (KPI 2, red team)
```

**Gate de salida:** dos procesos independientes, sin humano interviniendo, completan `pedido → ejecución → prueba → verificación → pago liberado` en devnet y después en mainnet con montos mínimos. Los seis gates intermedios que llevan hasta acá están en [`..._FASE1.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §1.

---

## 4. Fase 2 — El registro de trabajo liquidado

> **Dividida en subfases:** la fuente de verdad para ejecutar esta fase es [`..._FASE2.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE2.md) — subfases 2.1–2.5, con **parada obligatoria y pregunta entre cada una**, más precondiciones de entrada que se verifican antes de abrirla. Lo que sigue es el resumen.

Sin esto, la Fase 3 es imposible: no hay `W(t)` que medir.

- Indexer que agrega `WorkSettled` por época → `W(t)` = valor total de trabajo verificado y liquidado.
- **Análisis anti-sybil sobre datos reales:** grafo de financiamiento entre `payer` y `worker`. ¿Qué fracción de `W(t)` viene de pares que se fondean desde el mismo origen? Ese porcentaje es la estimación empírica de wash work, y es el número que valida o destruye el supuesto de la Fase 0.
- Dashboard con `W(t)`, distribución por clase de trabajo, tasa de challenges, tasa de fraudes detectados.

**Gate de salida:** `W(t) > 0` de forma sostenida y con fracción auto-generada por debajo del umbral que la simulación de E0.2 marcó como tolerable.

---

## 5. Fase 3 — La moneda (solo si el gate de E0.2 pasó)

```
E(t) = min( curva_temporal(t), k · W(t) )
```

- **Techo duro** fijo en génesis, no gobernable, sin función de mint arbitrario. Si existe una llave que puede emitir fuera de la fórmula, el diseño entero es decorativo.
- **Curva temporal** — freno de seguridad: ninguna explosión de `W(t)` puede acelerar la emisión más allá del calendario.
- **`k · W(t)`** — piso de realidad: sin trabajo liquidado no hay emisión, aunque pase el tiempo.
- **Quema** por consumo de recursos de la red (`creación → utilización → quema`).

**Secuencia obligatoria antes de mainnet:**
1. Testnet con valor simbólico, economía corriendo 60+ días.
2. **Red team pago:** contratar a alguien con el mandato explícito de farmear el subsidio. Si nadie lo intenta en serio antes del lanzamiento, lo van a intentar después, gratis, y con dinero real de por medio.
3. `k` arranca en el extremo conservador de la ventana y solo puede moverse dentro de ella por una regla predefinida — nunca por votación discrecional.

---

## 6. Fase 4 — Agentes

- SDK donde un agente **contrata a otro**: `request_work(spec, budget)` / `fulfill(order)` / `claim(proof)`.
- Primer par real, aprovechando el proyecto existente: un agente que **compra backtests y señales** a otro en vez de correr todo internamente. Es el caso más chico donde la economía de agentes es literal y no una metáfora.
- Camino de disputa para trabajo **no demostrable** (escrow + árbitro + reputación). Es peor que una prueba, y hay que admitirlo en el diseño: es la frontera declarada del sistema, no un agujero.

---

## 7. Decisiones técnicas

| Decisión | Elección | Razón |
|---|---|---|
| Cadena | Solana | Fees compatibles con micropagos entre agentes; ya es el terreno del proyecto existente. |
| Pago | Stablecoin existente | Fase 1 no debe depender de un token propio. |
| Verificación default | **Optimista (Nivel 1)** | Camino feliz casi gratis. ZK solo donde la latencia de la ventana de challenge sea inaceptable. |
| zkVM (si Nivel 2) | zkVM de propósito general con wrapper Groth16 | Permite verificación on-chain barata vía los syscalls de curva disponibles. **El costo real se mide en E0.1 — no asumirlo.** |
| Lenguaje del contrato | Rust / Anchor | Estándar del ecosistema. |

---

## 8. Riesgos y dónde se mitigan

| Riesgo (del documento de concepto) | Fase que lo ataca | Mitigación concreta |
|---|---|---|
| No todo trabajo es demostrable | 1 y 4 | Empezar por una sola clase demostrable; declarar la frontera y dar camino de disputa fuera de ella. |
| Probar cuesta más que hacer | 0 (E0.1) | Es el KPI 1. Escalera de verificación: usar el nivel más barato que alcance. |
| Colusión sobre el subsidio | 0 (E0.2), 2, 3 | Ventana de `k`, análisis de grafo sobre datos reales, red team pago. |
| Prueba correcta ≠ resultado correcto | 1 | `spec_hash` canónico. La prueba cubre *ejecución*, no *intención*; queda explícito en el diseño. |

---

## 9. Qué hacer esta semana

Tres cosas concretas, ninguna requiere decidir nada del protocolo:

1. **Congelar un dataset de velas** y escribir un runner de backtest 100% determinístico (mismo input → mismo hash de output, verificado en 100 corridas). Sin determinismo no hay nada que probar — literalmente.
2. **Medir el Nivel 0**: cuánto cuesta ejecutar y cuánto cuesta re-ejecutar, en los 3 tamaños de tarea. Es la línea base contra la que se comparan los Niveles 1 y 2.
3. **Escribir `sim/k_window.py`**: el modelo honesto-vs-auto-pagador. Es un script chico y responde el gate más caro del plan.

Si al final de la semana el runner no es determinístico, ese es el problema real y todo lo demás espera.

---

## 10. Cómo se ve el fracaso (criterios de kill explícitos)

**Este plan asume que el concepto completo — con moneda incluida — probablemente fracase.** No está escrito para que salga bien; está escrito para que, si sale mal, se sepa en la semana 3 y no en el mes 18. Lo que sigue son los umbrales, fijados ahora, mientras la idea todavía no tiene costo hundido.

| Gate | Criterio de kill | Lectura honesta | Qué sobrevive |
|---|---|---|---|
| **KPI 1** — overhead de verificación | > 1 en todos los niveles y tamaños | Probablemente **pasa** en Nivel 1 (optimista) y **falla** en ZK para tareas chicas. No es el asesino — es la razón de que el default sea optimista. | Archivar con fecha de revisión, esperando que baje el proving. |
| **KPI 2** — ventana de `k` | Ventana vacía o microscópica | Genuinamente incierto. Es el que decide si hay moneda o no hay moneda. | Se cancela la Fase 3. **Sobrevive PoD sin moneda**: escrow verificable sobre stablecoins — chico, real, útil. |
| **Fase 1** — demanda | Nadie pide trabajo | ⬛ **DISPARADO — 2026-08-13.** Era el fracaso más probable y lo fue. Para `backtest.sweep.v1`, el mercado ya concluyó que los backtests no sirven como evidencia y se mudó a la verificación de cuentas en vivo. Un backtest criptográficamente perfecto no responde la pregunta que frena la compra. | **Cobrado.** La medición está en [`ARCHIVO.md`](ARCHIVO.md) §3, con fuentes. El proyecto se archivó en el checkpoint 1.4 |
| **Fase 2** — wash work | La mayoría de `W(t)` es auto-generado | El único que se lleva todo puesto: el mecanismo no distingue trabajo real de trabajo fabricado. | **La Fase 3 se cancela, sin negociar `k`.** Sobrevive PoD sin moneda, igual que si fallara KPI 2 — el wash work es letal *porque hay emisión*. Refinado en [`..._FASE2.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE2.md) §9. |

> **Cierre de este cuadro, 2026-08-13.** De los cuatro criterios, el de demanda es el único que
> llegó a evaluarse, y se disparó. Los otros tres nunca corrieron: E0.1 y E0.2 quedaron
> pendientes y la Fase 2 no se abrió. La decisión que sigue a un kill —archivar o bifurcar, no
> ajustar el diseño hasta que el número dé— se tomó tal como está escrita abajo: se archivó con
> la medición hecha, sin salir a buscar una variante que salvara la tesis. Ver [`ARCHIVO.md`](ARCHIVO.md).

**Por qué los umbrales van escritos antes de empezar.** Ahora son gratis. En el mes 8, con código escrito y algo de identidad puesta en el proyecto, los mismos números se vuelven negociables — siempre aparece el argumento de que el resultado malo se arregla con un parámetro más. Un umbral fijado antes del costo hundido es la única versión del umbral que después se respeta. Si un KPI da mal, la respuesta es archivar o bifurcar, **no ajustar el diseño hasta que el número dé**.

**El límite estructural que ningún gate resuelve.** Aunque los cuatro pasen, PoD cubre solo el **trabajo demostrable**. El resultado sería una economía sobre un subconjunto del trabajo económico, no sobre todo. Si ese subconjunto resulta chico, la idea es correcta y aun así no llega a ser lo que la sesión imaginaba. Eso no es un riesgo a mitigar: es el techo del concepto, y conviene tenerlo a la vista desde el día 1.
