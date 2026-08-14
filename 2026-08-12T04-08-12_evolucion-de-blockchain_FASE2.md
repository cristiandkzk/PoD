# 2026-08-12T04-08-12_evolucion-de-blockchain_FASE2

**Fase 2 — El registro de trabajo liquidado: `W(t)` y el número de wash work**
Plan maestro: [`..._PLAN.md`](2026-08-12T04-08-12_evolucion-de-blockchain_PLAN.md) §4 · Fase anterior: [`..._FASE1.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) · Concepto: [`..._blockchain.md`](2026-08-12T04-08-12_evolucion-de-blockchain.md)

Este documento reemplaza a §4 del plan maestro como fuente de verdad de la Fase 2. El plan maestro conserva el resumen y el gate de salida.

---

## 0. Qué es realmente esta fase (y por qué el plan maestro la subestima)

§4 la describe como "indexer + análisis + dashboard", 3–4 semanas. Eso describe el *trabajo*, no el *riesgo*. Dos cosas cambian de naturaleza acá:

**1. El indexer deja de ser una herramienta de analítica y pasa a ser un oráculo de emisión.** En la Fase 3, `W(t)` es una entrada de la fórmula de mint. Un `W(t)` que solo existe en la base de datos de quien corre el indexer es exactamente el mismo problema que el árbitro centralizado de la subfase 1.4 — pero peor, porque no resuelve disputas: acuña moneda. Si `W(t)` no es recomputable por un tercero desde la cadena, el techo duro de la Fase 3 es decorativo, que es justo lo que §5 del plan maestro prohíbe.

**2. El gate de esta fase es el único que se evalúa con un instrumento sin calibrar.** Todos los gates anteriores son binarios y auto-evidentes: el hash coincide o no, los balances cierran o no, el fraude se detectó o no. Este no: "qué fracción de `W(t)` es auto-generada" es la salida de un detector, y un detector sin sensibilidad y especificidad medidas no produce evidencia — produce un número. §4 asume que el detector funciona. Esa suposición es la subfase que falta, y es la que explica la diferencia de esfuerzo.

**La asimetría que hay que tener escrita desde el día 1:** el análisis de grafo detecta wash work *torpe*. Dos cuentas fondeadas por separado desde el mismo exchange, con montos y tiempos descorrelacionados, son indistinguibles de dos contrapartes reales. Por lo tanto el número que sale de 2.4 es un **piso** de wash work, nunca un techo. "Pasó el gate" significa *no hay wash detectable arriba del umbral*, no *el wash está abajo del umbral*. El techo lo mide el red team pago de la Fase 3, con un adversario que cobra por romperlo.

---

## 1. Precondiciones de entrada (se verifican antes de abrir la fase, no adentro)

| Precondición | Por qué es dura |
|---|---|
| `WorkSettled` congelado con `payer` y `worker` en claro, `schema_version` desde el primer evento | Decidido en 1.5. Si se anonimizó, el gate de esta fase es **incomputable** y no hay arreglo retroactivo |
| Tráfico en **mainnet con valor real**, aunque sea mínimo | En devnet el dinero es gratis: *todo* el tráfico es wash por construcción. Medir wash en devnet es medir nada |
| **Al menos una contraparte real** del track D, liquidando en mainnet | Sin esto la fracción auto-generada es 100% por construcción y el gate falla por definición — sin haber aprendido nada nuevo |
| Costo por orden (rent + fees) registrado en 1.3, y costo de prueba de E0.1 | Son insumos del segundo número de la fase (§7) |

**Sobre la tercera:** si el checkpoint de demanda de la salida de 1.4 (§8 de `..._FASE1.md`) se resolvió como *"seguir igual, sabiendo que 1.6 es un test técnico"*, entonces esta fase **no se abre todavía**. No es burocracia: correr 4 semanas de análisis anti-sybil sobre tráfico enteramente propio es gastar la fase para confirmar una tautología. La salida correcta en ese estado es quedarse en el track D hasta que exista contraparte, o archivar según §10 del plan maestro.

### Bifurcación por E0.2 — puede recortar la fase a la mitad

Si el KPI 2 de la Fase 0 concluyó que **la ventana de `k` es vacía**, la Fase 3 ya está cancelada. Todo el aparato anti-sybil (2.2, 2.3, 2.4) existe para proteger un subsidio que en esa rama no va a existir. En ese caso:

| Rama | Subfases que corren | Esfuerzo |
|---|---|---|
| Ventana de `k` existe | 2.1 → 2.5 completas | 4–5.5 semanas |
| Ventana de `k` vacía (Fase 3 cancelada) | **Solo 2.1 y 2.5** | 1.5–2 semanas |

En la segunda rama `W(t)` sigue valiendo la pena: es la métrica de negocio del escrow verificable y el insumo del dashboard. Deja de ser un oráculo de emisión y vuelve a ser lo que §4 creía que era.

---

## 2. Protocolo de checkpoint

Rige el mismo de §0 de [`..._FASE1.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md): una subfase por vez, se para y se pregunta, se presenta evidencia contra el gate / qué se rompió / qué cambia hacia adelante / costo real vs. estimado.

**Esta fase agrega una quinta respuesta válida**, y agregarla no es un tecnicismo:

| Respuesta | Cuándo | Qué pasa |
|---|---|---|
| **Indeterminado** | El gate se corrió pero **no había datos suficientes** para que el resultado signifique algo | Se sigue acumulando tráfico. **No se avanza a la Fase 3** |

Sin esta opción, "el gate pasó" y "no había datos" producen exactamente la misma salida —una fracción de wash baja— y son cosas opuestas. Con N chico, todo detector parece limpio. El N mínimo se fija en 2.4 **antes** de mirar el primer resultado, y se justifica; ver §5.

**Regla dura heredada y afilada:** el umbral de wash tolerable sale de la simulación de E0.2, y se escribe en 2.4 **antes** de correr la medición. Un umbral escrito después de ver el número no es un umbral, es una racionalización. Esta fase llega con toda la Fase 1 como costo hundido — es el momento del plan donde más barato resulta mover el umbral y más caro sale haberlo movido.

---

## 3. Mapa de subfases

| Subfase | Qué produce | Gate de salida (verificable, no opinable) | Esfuerzo |
|---|---|---|---|
| **2.1 — `W(t)` canónico** | Definición congelada + indexer determinístico con backfill y reorgs | Dos implementaciones independientes reproducen `W(t)` para todas las épocas, bit a bit | 4–6 días |
| **2.2 — Grafo de financiamiento** | Grafo `payer`↔`worker` reconstruible, Nivel 0 | El grafo se reconstruye determinísticamente desde la cadena; reglas de clasificación escritas antes de ver resultados | 1 semana |
| **2.3 — Calibración del detector** | Sensibilidad y especificidad contra un set etiquetado | FP y FN medidos sobre etiquetas comprometidas de antemano | 1 semana |
| **2.4 — Medición contra umbral** | Los dos números de la fase | Umbral y N mínimo escritos antes; medición corrida una vez; resultado reportado cualquiera sea | 3–4 días |
| **2.5 — Salud del mecanismo** | Dashboard + series que consume la Fase 3 | Series reproducibles desde el indexer canónico, no desde una base de datos suelta | 3–5 días |

Suma: **4–5.5 semanas**, arriba de las 3–4 del plan maestro. La diferencia es 2.3: §4 no tiene calibración porque asume que el detector funciona.

**Mapeo de control — nada de §4 se perdió:**

| Componente de §4 del plan maestro | Dónde vive ahora |
|---|---|
| Indexer que agrega `WorkSettled` por época | 2.1 |
| Grafo de financiamiento `payer` / `worker` | 2.2 |
| Fracción de `W(t)` de pares del mismo origen | 2.4 (medición) — con 2.3 como requisito para que el número sea evidencia |
| Dashboard `W(t)`, clase, challenges, fraudes | 2.5 |
| Gate: `W(t) > 0` sostenido + fracción bajo umbral | 2.4 (fracción) + 2.5 (sostenido) |

**Qué se congela acá y la Fase 3 hereda sin poder renegociar:**

| Decisión | Subfase | Por qué es irreversible |
|---|---|---|
| Definición exacta de `W(t)` (qué suma y qué no) | 2.1 | Es un término de la fórmula de mint |
| Duración y frontera de la época | 2.1 | Define la cadencia de emisión |
| Reglas de exclusión del grafo (hubs, CEX) | 2.2 | Cambiarlas después mueve el número del gate a gusto |
| Set de etiquetas y su hash | 2.3 | Re-etiquetar después de ver resultados invalida la calibración |

---

## 4. Subfase 2.1 — `W(t)` canónico y reproducible

**Objetivo:** que `W(t)` sea una función determinística del estado de la cadena, no la opinión de una base de datos.

**Qué se construye** — `pod/indexer/` (sube de *smoke* a canónico):

- **Definición escrita de `W(t)`**, con las exclusiones explícitas y su razón. Las que hay que resolver sí o sí, porque cada una es una puerta de inflado:

  | Pregunta | Default propuesto | Razón |
  |---|---|---|
  | ¿Cuenta una orden `FALLIDA`? | **No** | Decidido en 1.5: `WorkFailed` es un evento separado justamente para esto |
  | ¿Cuenta una orden cancelada por timeout? | **No** | No hubo entrega; no hay nada verificado |
  | ¿El bond del worker suma a `W(t)`? | **No** | Es garantía, no pago. Sumarlo permite inflar `W(t)` subiendo el bond propio |
  | ¿Valor bruto o neto de fees? | **Bruto**, con el fee registrado aparte | El neto se vuelve ambiguo cuando la Fase 3 introduzca un fee quemado |
  | ¿Órdenes de valor mínimo? | Se cuentan en `W(t)`, **pero no en N** | El dust no mueve un total ponderado por valor, pero sí infla el conteo de muestras |

- **Época**: definición fija (por slots, no por reloj de pared), con frontera determinística y regla de asignación de órdenes que caen justo en el borde.
- **Robustez de ingesta**: backfill desde el primer evento, idempotencia (re-procesar un rango no cambia el resultado), manejo de reorgs y espera de finalidad, y lectura de `schema_version` con la historia vieja legible sin reindexar.

**Gate de salida:** un segundo lector independiente —otra implementación, escrita contra la definición y no contra el código del primero— produce el mismo `W(t)` para **todas** las épocas, incluida al menos una época con un reorg y una con una orden en el borde. Es a propósito el mismo gate de 1.1: si el hash canónico mereció dos implementaciones, la entrada de la fórmula de emisión también.

**Criterio de kill local:** ninguno. Si esto no cierra es un bug, no una refutación.

> **Checkpoint 2.1** → evidencia y pregunta antes de tocar 2.2.

---

## 5. Subfase 2.2 — Grafo de financiamiento

**Objetivo:** poder preguntarle a los datos si `payer` y `worker` son la misma persona con dos carteras.

**Qué se construye** — `pod/indexer/graph/`:

- **Nodos**: direcciones. **Aristas**: quién fondeó a quién (creación de cuenta y transferencias de SOL/token hacia una cuenta antes de su primera orden).
- **Regla de origen común**: `payer` y `worker` comparten un ancestro de fondeo dentro de `d` saltos, **excluyendo hubs**.
- **Señales secundarias, todas Nivel 0**: correlación temporal de fondeo, montos espejo, reciprocidad payer↔worker entre las mismas dos direcciones, concentración de un par sobre el total.

**El parámetro que decide la respuesta, y por eso se congela primero: la lista de hubs.** En Solana todo el mundo se fondea desde un exchange. Si los hot wallets de CEX cuentan como ancestro, *todos* los pares comparten origen y el detector marca 100%. Si se excluyen sin más, un washer que rutea por un CEX es invisible y el detector marca 0%. La lista de exclusión —qué es hub, con qué criterio de grado, qué bridges y faucets entran— es literalmente la perilla que fija el resultado del gate de la fase. Va escrita, versionada y **congelada antes de correr el detector sobre datos reales**, con el criterio de grado justificado, no elegida a ojo mirando el output.

**Nivel de la escalera:** todo esto es Nivel 0 —consultas determinísticas sobre un grafo. No hay clasificador ni modelo acá y no debería haberlo: un juicio de "esto parece sospechoso" emitido por un modelo no es auditable, no es reproducible por un tercero y no se puede congelar. Si el grafo determinístico no alcanza, la respuesta es más señales Nivel 0, no subir de nivel.

**Gate de salida:** el grafo se reconstruye determinísticamente desde la cadena (dos corridas, mismo resultado), y el documento de reglas —hubs, `d`, señales, pesos— está escrito y fechado **antes** de que exista cualquier resultado sobre datos reales.

> **Checkpoint 2.2** → evidencia y pregunta antes de tocar 2.3.

---

## 6. Subfase 2.3 — Calibración del detector (la subfase que §4 no tiene)

**Objetivo:** convertir el detector en un instrumento con error conocido. Sin esto, el número de 2.4 no es evidencia.

**La oportunidad que la Fase 1 dejó servida:** §7 de `..._FASE1.md` advierte que el end-to-end de 1.6 se puede pasar con las dos puntas del mismo dueño, y que eso es *wash work por construcción*. Esa advertencia es, acá, un regalo: **ese tráfico es un set de positivos etiquetados, gratis y con verdad de campo perfecta.** Y las contrapartes del track D son los negativos.

**Qué se construye:**

- **Set etiquetado**, en dos clases: *positivos* — todas las direcciones propias, tráfico e2e de 1.6, cualquier par donde ambas puntas son tuyas; *negativos* — pares con contraparte real verificada del track D.
- **Compromiso previo del set**: el set se congela y se publica su hash **antes** de correr el detector. Si las etiquetas se pueden retocar después de ver los resultados, la calibración mide la habilidad para retocar etiquetas.
- **Positivos adversariales, baratos:** un puñado de pares wash hechos a propósito con evasión creciente — mismo fondeo directo, un salto intermedio, fondeo separado desde un CEX, fondeo separado + tiempos y montos descorrelacionados. Cuestan poco y son lo único que muestra **dónde se cae el detector**.
- **Métricas**: sensibilidad (qué fracción de wash conocido detecta) y especificidad (qué fracción de pares reales marca mal), reportadas por nivel de evasión.

**Gate de salida:** sensibilidad y especificidad medidas y publicadas, con la curva de degradación por nivel de evasión. **No hay un umbral que aprobar acá** — el gate es que los números existan y estén escritos. Un detector con 60% de sensibilidad es utilizable si se sabe que tiene 60%; lo inutilizable es no saberlo.

**Qué hacer si la especificidad es mala** (marca reales como wash): eso sí bloquea. Un detector que confunde clientes reales con wash va a subestimar la demanda genuina justo en la fase que decide si hay demanda genuina. Se corrige antes de 2.4.

**Salida obligatoria de esta subfase, aunque incomode:** el nivel de evasión a partir del cual la sensibilidad cae a ~0. Ese es el enunciado honesto del alcance del gate de la fase, y es lo que vuelve a §0: el número de 2.4 es un piso.

> **Checkpoint 2.3** → evidencia y pregunta antes de tocar 2.4.

---

## 7. Subfase 2.4 — Umbral, N mínimo y la medición

**Objetivo:** producir los dos números de la fase, una sola vez, con los criterios escritos de antemano.

### Se escribe primero (sin haber corrido nada)

**El umbral de wash tolerable** no se inventa acá: se **deriva** del heatmap de E0.2. La regla de derivación —qué punto de la región viable se toma y con qué margen— se escribe explícita, de modo que el umbral sea una consecuencia de la simulación y no una preferencia. Si E0.2 no permite derivarlo sin ambigüedad, eso es un hallazgo de E0.2 y se corrige ahí, no acá.

**El N mínimo**, con su justificación. El punto es que la incertidumbre a N chico se traga la decisión: con 30 pares independientes y una fracción observada de 40%, el intervalo de confianza del 95% es de aproximadamente ±18 puntos — abarca a la vez "tolerable" y "fatal". Con 100 baja a ~±10 puntos, y con 300 a ~±6. Y hay dos correcciones que empeoran el cuadro, las dos hacia el mismo lado:

- **N no es el número de órdenes: es el número de componentes de financiamiento independientes.** Cien órdenes entre las mismas dos direcciones son una sola muestra para un análisis de sybil.
- **`W(t)` está ponderado por valor**, así que el N efectivo es todavía menor cuando unas pocas órdenes grandes dominan el total. Se reporta también la fracción no ponderada, para ver cuánto depende el resultado de una sola orden grande.

Si al momento de medir `N < N_mínimo`, la respuesta es **Indeterminado** (§2), no *aprobado*.

### Se corre después

- Fracción de `W(t)` proveniente de pares con origen común, con su intervalo de confianza, corregida por la sensibilidad medida en 2.3.
- **Segundo número — el costo real de un ciclo de wash**, en dólares de mainnet: gas + rent (medidos en 1.3) + costo de prueba (E0.1) + capital inmovilizado en el bond durante la ventana de challenge. Vale tanto como el primero: la ventana de `k` de E0.2 se calculó con costos **supuestos**, y este número los reemplaza por medidos. Sin él, la Fase 3 elige `k` desde una simulación cuyos insumos nadie verificó.

**Nota que hay que dejar escrita para la Fase 3:** en la Fase 1 el escrow no cobra fee. El modelo de E0.2 asume que el auto-pagador quema un fee, y hoy ese fee **no existe en el contrato**. O sea que el costo medido de un ciclo de wash es solamente gas + rent + prueba + capital. Si la seguridad de `k` depende de un fee quemado, ese fee es una pieza faltante del contrato y hay que agendarla en la Fase 3, no darla por hecha.

**Gate de salida:** los dos números publicados con su método, y la comparación contra el umbral escrito de antemano. El gate se cumple si la fracción está bajo el umbral **y** `N ≥ N_mínimo` **y** `W(t) > 0`.

> **Checkpoint 2.4** → es el checkpoint pesado de la fase: acá se decide si la Fase 3 existe. Evidencia y pregunta.

---

## 8. Subfase 2.5 — Salud del mecanismo

**Objetivo:** las series que la Fase 3 necesita para calibrar y para detectar que algo se está rompiendo, no un dashboard bonito.

**Qué se construye:**

- `W(t)` por época, y **`W(t)` sostenido**: cuántas épocas consecutivas con `W(t) > 0`, con dos condiciones anti-trivialidad —ningún `payer` solo aporta más de una fracción declarada del total, y hay al menos `M` componentes de financiamiento distintos. Un único cliente grande durante cuatro semanas satisface "sostenido" sin significar nada.
- Distribución por clase de trabajo (hoy una sola, por diseño de la Fase 1 — la serie existe para cuando haya más).
- Tasa de challenges, tasa de challenges exitosos, tasa de challenges falsos castigados. Si es 0% durante toda la fase, no es una buena noticia: significa que **nadie está verificando**, y la seguridad optimista de 1.4 es teórica.
- Latencia de liquidación real vs. ventana de challenge — dato contra el criterio de kill local de 1.4.
- Costo por orden observado, contra lo registrado en 1.3.

**Gate de salida:** todas las series se regeneran desde el indexer canónico de 2.1 y son reproducibles por un tercero. Ninguna métrica que se muestre puede depender de una consulta manual a una base de datos que solo vos corrés — misma razón que en 2.1: en la Fase 3 estas series justifican decisiones sobre dinero.

> **Checkpoint 2.5** → gate de salida de la Fase 2. Se pregunta antes de pasar a la Fase 3.

---

## 9. Qué significa fallar acá (refinamiento de §10 del plan maestro)

§10 dice que si la mayoría de `W(t)` es auto-generado *"se cancela el proyecto entero"* y que no sobrevive nada. Es demasiado grueso, y conviene afilarlo antes de estar parado en el resultado:

| Qué muere | Qué sobrevive |
|---|---|
| **La Fase 3, sin negociación.** No hay `k` que arregle un `W(t)` mayormente fabricado: el subsidio se lo lleva el fabricante | **El escrow verificable de la Fase 1 y el SDK de la Fase 4** |

La distinción es a quién daña el wash work. **Sin subsidio, alguien que se paga a sí mismo por un backtest solo se daña a sí mismo**: quema gas y no le saca nada a nadie. Con subsidio, ese mismo comportamiento es una transferencia desde todos los tenedores hacia el que fabrica trabajo. El wash work es letal *porque hay emisión*, no por sí mismo.

Esto **no ablanda el criterio de kill** — lo vuelve más duro donde importa: la Fase 3 se cancela con el umbral incumplido, sin la salida de "ajustamos `k` un poco más abajo". Lo que hace es reconocer que el resultado es el mismo al que se llega si E0.2 hubiera fallado: PoD sin moneda, escrow verificable sobre stablecoins. Esa rama ya estaba prevista en §2 del plan maestro y no hay razón para tirarla acá.

**El fracaso más caro de esta fase no es que el número dé mal — es que dé bien sin significar nada:** N chico, cero challenges, un solo cliente. Contra eso están el N mínimo de 2.4 y las condiciones anti-trivialidad de 2.5, y por eso *Indeterminado* es una respuesta válida del checkpoint.

---

## 10. Qué queda explícitamente fuera de la Fase 2

- **Token, `k`, emisión, red team pago** — Fase 3. Acá se produce el insumo, no se usa.
- **Reputación o scoring de direcciones.** Detectar origen común no es puntuar participantes. El concepto ya deslindó que PoD **no es reputación** (§6 del documento de concepto); un score de direcciones es reputación con otro nombre.
- **Bloquear o filtrar pares sospechosos.** Esta fase **mide**, no interviene. Excluir tráfico del registro es una decisión de protocolo y necesita el diseño de la Fase 3.
- **Cualquier clasificador basado en modelo.** Ver 2.2: el detector es Nivel 0 o no es auditable.
- **Múltiples clases de trabajo** — sigue habiendo una sola hasta la Fase 4.
- **Privacidad de `payer` / `worker`.** Congelado en claro en 1.5 y no se toca acá; si alguna vez se quiere anonimizar, es un cambio de esquema con migración, posterior a la Fase 3.
