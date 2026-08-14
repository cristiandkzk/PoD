# ARCHIVO — Proof of Delivery

**Estado: archivado el 2026-08-13, en el checkpoint 1.4, por el criterio de kill de demanda.**

No es un proyecto abandonado a mitad de camino. Es un proyecto que produjo la respuesta que
fue diseñado para producir, y que se detuvo cuando la produjo. Este documento existe para que
esa respuesta se entienda dentro de un año sin releer nada más.

**Si vas a leer dos secciones, leé la §8 y la §9.** La §8 separa lo que quedó *probado* de lo que
quedó *razonado* — el archivo se decidió con una mezcla de las dos, y la diferencia importa más
que cualquier otra cosa escrita acá. La §9 dice dónde esta tecnología es la herramienta correcta
y dónde no, que es lo único que sirve para decidir si vale la pena retomarla.

---

## 1. Qué se estaba probando

La tesis, del [documento de concepto](2026-08-12T04-08-12_evolucion-de-blockchain.md): que
*"trabajo pedido, pagado y entregado"* puede volverse un hecho criptográficamente verificable,
y servir como unidad de liquidación y única fuente de emisión de una cadena.

El [plan maestro](2026-08-12T04-08-12_evolucion-de-blockchain_PLAN.md) fue escrito con una
premisa explícita: **asumir que la idea probablemente fracase, y ordenarla para que el fracaso
aparezca en la semana 3 y no en el mes 18.** Fijó cuatro criterios de kill antes de escribir
una línea de código, mientras todavía eran gratis.

Uno de ellos se disparó.

---

## 2. Qué se construyó, y qué probó cada gate

Cuatro subfases de la Fase 1, todas con su gate cumplido y su evidencia reproducible.

| Subfase | Gate | Evidencia |
|---|---|---|
| **1.1 — Spec canónica** | Dos implementaciones independientes producen el mismo `spec_hash` sobre los mismos vectores | Python y Rust, ambas **sin dependencias**. 8 vectores válidos + 35 de rechazo, congelados. `python pod/spec/scripts/gate.py` |
| **1.2 — Runner** | Un tercero, en otra máquina, reproduce el `output_hash` desde el recibo | **Idéntico byte a byte en 3 plataformas, 2 arquitecturas y 3 intérpretes**: Windows/x86-64, Ubuntu WSL2/x86-64 y Android-Termux/**ARM64**. Fidelidad 468/468 contra el simulador original. Ed25519 propio 5/5 contra OpenSSL |
| **1.3 — Escrow** | Todo camino conserva balances; ninguno deja fondos atrapados | Conservación como igualdad exacta en los tres caminos, más las tres prohibiciones del gate |
| **1.4 — Verificación optimista** | Un `output_hash` falso challengeado pierde el bond; un challenge infundado pierde el depósito | **34 tests en proceso + los dos escenarios en cadena**, con el `.so` de 81 KB desplegado sobre un validador local. `pod/program/CADENA.txt` |

**Verificado el día del archivo**, no confiando en corridas viejas: los cuatro gates vuelven a
pasar. `pod/spec` 4/4 · `pod/prover` 3/3 pedidos en 3 plataformas · `pod/program` **34 tests,
exit 0** · los dos escenarios de 1.4 en cadena.

**Lo que no corrió:** la Fase 0 entera (E0.1 overhead de verificación, E0.2 ventana de `k`),
la subfase 1.5 —rediseñada pero no empezada— y la 1.6.

### Datos duros que quedaron medidos

| | valor | nota |
|---|---|---|
| Rent de una orden (304 bytes) | 3 006 720 lamports | **vuelve entero** al cerrar |
| Ciclo normal, 4 transacciones | 20 000 lamports | quemados |
| Ciclo con disputa, 6 transacciones | 30 000 lamports | quemados |
| Despliegue del programa (81 KB) | 0,567421 SOL | una sola vez, recuperable |

El hallazgo económico de 1.3 y 1.4, que sobrevive al archivo: **el costo de transacción no es
la restricción.** Es despreciable frente a cualquier tarea que valga la pena pedir. Lo que pesa
es el **capital inmovilizado** —recompensa + bond + depósito + rent— durante toda la ventana, y
la verificación optimista *alarga* ese plazo por diseño.

---

## 3. El hallazgo que lo archivó

El criterio de kill de la Fase 1 era *"nadie pide trabajo"*, y su lectura honesta ya estaba
escrita en §10 del plan: significaría que **el cuello de botella nunca fue la verificación**.

La clase de trabajo elegida era `backtest.sweep.v1`: barridos de backtests sobre datos de
mercado. El razonamiento y la evidencia externa llegaron al mismo lugar.

**Por razonamiento.** La desconfianza del comprador de una estrategia se descompone en seis
cosas. El acta verificable resuelve las dos primeras —¿los números están inventados?, ¿probaste
mil variantes y me mostrás la mejor?— y **no toca la que domina**: ¿va a funcionar de acá en
adelante? Además, el pre-registro de la rejilla no impide elegir el *período*, que es el fraude
más común, salvo que sea el comprador quien fije el dataset.

**Por evidencia externa.** El MQL5 Market es un mercado real, con dinero real, donde el fraude
con backtests está nombrado y documentado por su propia comunidad: hay EAs codificados para
mostrar ganancias falsas en el backtester, y *"por cada EA con señal en vivo verificada hay
docenas vendiendo capturas de backtests elegidos a dedo"*. La plataforma valida cada producto
automáticamente, pero **"el EA no necesita ser rentable durante la validación"** — valida
ejecutabilidad, no veracidad.

Y ahí está el golpe:

> **El mercado no fracasó en verificar backtests. Concluyó que los backtests no sirven como
> evidencia y se mudó a otra cosa.**

El consejo unánime de esa comunidad no es *"exigí un backtest verificado"*, es *"no confíes en
backtests, pedí verificación de cuenta real"*. Myfxbook se conecta al bróker, contrasta contra
sus datos y publica un track record verificado. El motivo está dicho con todas las letras:
*"encontrar los parámetros correctos para un período delimitado en un instrumento particular no
significa que vayas a ganar en vivo"*.

**Un backtest criptográficamente perfecto sigue sin responder la única pregunta que frena la
compra.** Y donde sí está el gasto —la verificación en vivo— tampoco llegamos: nuestra máquina
verifica *cómputo determinístico sobre datos fijados por hash*, mientras que atestiguar que unas
operaciones ocurrieron de verdad en un bróker de verdad es un problema de otra naturaleza.

**Verificamos cómputo. Ese mercado necesita atestiguar realidad.**

### La corrección, de una segunda ronda de investigación

La frase *"nadie paga por verificación"* quedó **desmentida**, y conviene decirlo con todas las
letras porque cambia el motivo del archivo:

**Sí pagan. Hay empresas viviendo de esto.** En forex, Myfxbook. En cripto —que no tenía
equivalente hasta hace poco— apareció AuditZK, que construye track records verificados de CEX y
on-chain metiendo claves de solo lectura en un enclave de hardware AMD SEV-SNP, con el argumento
explícito de que *"ninguna parte neutral confirma los números de un trader de cripto, así que las
afirmaciones quedan auto-declaradas"*.

O sea que el problema existe, está reconocido, y alguien puso plata en resolverlo.

**Lo que no existe es demanda de verificar *nuestro* objeto.** Miralo en tres mercados:

| Mercado | Qué se desconfía | Con qué se resolvió |
|---|---|---|
| Venta de EAs en forex | ¿Los resultados son reales? | Track record **en vivo** verificado contra el bróker (Myfxbook) |
| Afirmaciones de trading en cripto | ídem | Enclave de hardware leyendo la API del exchange (AuditZK) |
| Venta de bots de Solana | ¿Me va a robar la wallet? | Auditoría de código |

Ninguno de los tres necesita *"¿este cómputo se hizo bien sobre este dataset?"*. Los dos primeros
necesitan **atestiguar realidad** —que esas operaciones ocurrieron de verdad, en un lugar de
verdad— y la tecnología que eligieron para eso es hardware confiable leyendo una API, no
reejecución determinística. El tercero necesita seguridad de software.

**El motivo real del archivo, entonces, no es que falte demanda de verificación. Es que nuestra
primitiva verifica otra cosa.** Es una conclusión más precisa y menos deprimente: el mecanismo no
está de más, está apuntado a un objeto que este mercado no discute.

Y es la confirmación más fuerte del filtro de §4, por la vía del contraejemplo: en trading el
objeto en disputa es la **realidad**, no un cómputo, así que PoD no aplica — y el mercado se fue
a TEE, que es la herramienta correcta para atestiguar realidad.

*Calibración: que AuditZK exista prueba que alguien apostó al mercado; sus afirmaciones sobre el
tamaño del hueco son material de venta y no se verificaron. No se midió su tracción.*

**Fuentes:** [cómo falsear backtests en el MQL5 Market](https://www.earnforex.com/forum/threads/how-to-fake-expert-advisor-back-test-results-in-mql5-market-place.30045/) ·
[evitar estafadores en el MQL5 Market](https://www.forexfactory.com/thread/1160546-tips-to-avoid-scammers-at-the-mql5-market) ·
[estafas con EAs](https://www.mql5.com/en/forum/457568) ·
[reglas del Market](https://www.mql5.com/en/market/rules) ·
[chequeos previos a publicar](https://www.mql5.com/en/articles/2555) ·
[cuentas falsas en Myfxbook](https://www.earnforex.com/forum/threads/ways-to-recognize-fake-and-real-accounts-on-myfxbook.33417/) ·
[verificar señales con Myfxbook](https://www.jptradingcapital.com/blog/en/verified-forex-signals)

---

## 4. El filtro que quedó

Vale más que el resultado, porque se aplica a cualquier otra clase de trabajo en minutos:

> **PoD tiene demanda solo donde el objeto en disputa es un cómputo sobre datos que las partes
> ya aceptan.** No donde se discute si esos datos predicen el futuro, ni donde se discute si
> reflejan la realidad.

El backtest falla en las dos: nadie discute la aritmética, discuten la extrapolación.

Clases que *a priori* pasarían el filtro y nunca se evaluaron: un scoring sobre un dataset
acordado, el cálculo de un índice, la evaluación de una competencia con datos de prueba fijos,
una liquidación contractual sobre datos que ambas partes firmaron. En todas, el dato es
indiscutido y el número es lo que se pelea.

**Advertencia para quien retome esto:** buscar clases hasta encontrar una que sobreviva es la
versión elegante de ajustar el parámetro hasta que el KPI dé bien. El filtro está escrito
*antes* de mirar, y salió del hallazgo, no de las ganas de seguir. Una clase se evalúa contra
él, y se evalúa **sin construir nada**.

---

## 5. Qué sirve igual, independientemente de la tesis

Ninguna de estas piezas depende de que PoD tuviera razón:

- **[`pod/spec/`](pod/spec/README.md)** — Serialización canónica y `spec_hash` con separación de
  dominio, esquema cerrado, sin flotantes, sin nulos, ASCII estricto. **Dos implementaciones sin
  dependencias** (Python y Rust, con SHA-256 escrito a mano) que se arbitran contra vectores
  congelados. La decisión de fondo —*rechazar la ambigüedad en vez de normalizarla*— es
  reutilizable en cualquier formato que tenga que hashearse igual en dos lados.
- **[`pod/prover/`](pod/prover/README.md)** — Runner determinístico con evidencia de
  reproducibilidad **cross-arquitectura**, recibo firmado con Ed25519 puro, y replay. La
  disciplina numérica de [`SPEC-RUNNER.md`](pod/prover/SPEC-RUNNER.md) §1 —solo `+ − × ÷`, cero
  trascendentales— es la razón por la que el hash coincide en un teléfono ARM y en Windows.
- **[`pod/program/`](pod/program/README.md)** — Escrow en Solana nativo con bond, timeouts,
  ventana de impugnación y árbitro, con conservación de balances probada camino por camino y
  cierre sin rent atrapado. Es un escrow programable funcionando, con o sin la tesis encima.
- **El método.** Cuatro checkpoints, cuatro gates verificables, una respuesta *Rediseñar*
  ejercida de verdad, y criterios de kill fijados antes del costo hundido y respetados cuando
  uno se disparó.

---

## 6. Deudas que quedaron escritas y sin resolver

Están documentadas donde corresponde; se listan acá para que no haya que buscarlas:

- **Disponibilidad de datos** — el contrato fija `spec_hash` y `output_hash`, y ninguno
  *contiene* los bytes. Sin ellos, ni el challenger ni el árbitro pueden hacer su trabajo.
  Es la deuda más grande. Ver [`FASE1`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §6.3.

  **Salida candidata, anotada después del archivo:** una red de nodos incentivados a guardar y
  servir los datasets — infraestructura pagada por almacenamiento y ancho de banda, sin
  participar de la validación. Separa bien los roles: el nodo cobra por lo que aporta, y la
  verificación queda en manos de quien tiene interés en el resultado. La categoría existe y
  funciona (Filecoin, Arweave, IPFS con pinning), así que no habría que inventarla.

  Lo difícil, para quien lo intente: **probar que un nodo realmente guarda el dato es un
  problema duro**, no un detalle. Un nodo puede afirmar que lo tiene y buscarlo en otro lado
  cuando se lo piden. No es imposible; es caro, y hay que presupuestarlo como tal.

  On-chain no es una opción y conviene tener el número: los 17 MB del dataset de referencia
  costarían unos **118 SOL de rent**, y de todas formas exceden el límite de 10 MB por cuenta
  de Solana. Es también la razón por la que "que la red entera reejecute y valide" no se puede:
  sin los datos en la cadena, ningún validador puede recomputar nada.
- **Dos roles impagos** — uno con respuesta, el otro no. Ver [`SPEC-PROGRAM`](pod/program/SPEC-PROGRAM.md) §10.

  **El árbitro: resuelto en el papel.** Cobra un honorario que el pagador incluye en el precio
  de la orden, haya disputa o no — arbitraje como seguro, no como premio. No se acuña nada, no
  hay incentivo a inflar volumen (las órdenes las crean pagadores con plata real, y una orden
  falsa le cuesta plata al que la fabrica), y no hay incentivo a fabricar disputas porque cobra
  igual. Todo intento de pagarlo con emisión —pools infinitos, tokens de validación— termina en
  una máquina de imprimir: con un ciclo completo costando 20 000 lamports, fabricar
  transacciones falsas rinde apenas el token acuñado valga más que medio centavo.

  **El challenger: sin resolver, y es el que importa.** Arriesga su depósito para ganar el bond
  solo si acierta; si el fraude es raro su valor esperado es negativo, así que no challengea
  nadie — y si no challengea nadie, el fraude deja de ser raro. Es el **dilema del verificador**,
  un problema conocido que arrastran todos los esquemas optimistas. La única salida específica
  que apareció: **el pagador es el verificador natural**, porque le importa el resultado y ya
  tiene los datos. Lo que deja el nudo a la vista — la ventana existe para cuando el pagador no
  verifica, y quien no puede verificar tampoco puede juzgar al árbitro.
- **La ventana es el mecanismo de prueba, no un costo.** Liquidar por confirmación del pagador
  sería una atestación, no una prueba, y en un esquema con emisión no distinguiría trabajo real
  de dos cuentas coludidas. Por eso `accept_delivery` no existe, a propósito.
- **Identidad y sybil.** Las claves son gratis, así que exigir identidades únicas no frena nada.
  La versión tratable era el grafo de financiamiento de la Fase 2, que no llegó a correr.
- **El entorno del runner no está pineado** — `runner.image_digest` viaja en el pedido y nada
  comprueba que el intérprete que ejecutó sea el declarado.

---

> **Nota de una conversación posterior al archivo.** Se exploraron cinco variantes del
> mecanismo —árbitros cobrando fees, dos pools de tokens con uno infinito, un árbitro único
> global, la red entera como árbitro, y nodos aportando infraestructura—. Todas responden la
> misma pregunta: *quién presta el servicio y cómo se le paga*. Mejoraron el diseño y quedaron
> anotadas arriba. **Ninguna toca la que archivó el proyecto: quién pone plata desde afuera.**
> Queda escrito acá porque es fácil volver a caer: el diseño del lado de la oferta siempre tiene
> una capa más elegante, y ninguna cambia el resultado mientras la columna de la demanda esté
> vacía.
>
> Un hallazgo estructural sí salió de esa ronda, y vale: **casi todas las patologías del sistema
> —wash work, la máquina de imprimir, el árbitro con incentivo perverso, la identidad y el
> sybil— existen únicamente porque hay emisión.** Sin emisión, el diseño cierra solo, y lo que
> queda es lo que §10 del plan maestro ya había anotado como sobreviviente: escrow verificable,
> chico, real y útil.

## 7. Qué haría falta para reabrirlo

Una clase de trabajo que pase el filtro de §4, con una contraparte real dispuesta a pagar por
la verificación de ese cómputo. Ese es el orden: **primero la contraparte, después el código.**

La §9.4 lo afina bastante más que el filtro solo, y conviene usar esa versión: *¿existen dos
partes que hoy no cierran un trato por un trabajo chico, donde el que paga podría verificar el
resultado él mismo?*
Lo construido cubre casi toda la maquinaria y es agnóstico de la clase; lo que faltaba nunca fue
técnico.

Si esa contraparte no aparece, el resultado ya está: **el cuello de botella no era la
verificación**, y eso es exactamente lo que este proyecto se propuso averiguar barato.

---

## 8. Qué quedó validado y qué no

La sección más importante para quien retome esto, incluido el autor. Separa lo que se **probó**
de lo que se **razonó**, porque el archivo se decidió con una mezcla de las dos y conviene saber
cuánto peso aguanta cada cosa.

### 8.1 Validado con evidencia propia y reproducible

Todo esto vuelve a correr hoy y da lo mismo. Son hechos, no interpretaciones.

| Conclusión | Evidencia |
|---|---|
| Un formato canónico con esquema cerrado se puede especificar sin ambigüedad: **dos implementaciones independientes coinciden** | Python y Rust, sin dependencias, sobre 8 vectores válidos y 35 de rechazo congelados |
| **Un cómputo de punto flotante puede ser bit a bit reproducible entre arquitecturas** si se restringe a `+ − × ÷` y se evitan las trascendentales | Mismo `output_hash` en Windows/x86-64, Ubuntu-WSL2/x86-64 y Android-Termux/**ARM64**, con tres intérpretes distintos |
| El port computa lo mismo que el backtester original | 468/468 combinaciones idénticas contra el simulador en Node |
| Ed25519 escrito a mano coincide con OpenSSL | 5/5 casos, clave pública y firma byte a byte |
| Un escrow puede conservar balances en **todos** sus caminos, sin rent atrapado ni fondos huérfanos | 34 tests, conservación escrita como igualdad exacta |
| **Mentir cuesta el bond y challengear en falso cuesta el depósito** — el mecanismo funciona | Los dos escenarios, en proceso y sobre un validador con el `.so` real de 81 KB |
| El costo de transacción **no** es la restricción económica | Ciclo normal 20 000 lamports; el rent (3 006 720) vuelve entero; desplegar 0,567 SOL una vez |
| Los datos no pueden vivir on-chain | 17 MB ≈ **118 SOL** de rent, y exceden el límite de 10 MB por cuenta |

**Límite honesto de lo anterior:** el gate en cadena corrió sobre un **validador local**, no en
devnet ni mainnet. Runtime, VM, presupuesto de cómputo, fees y reloj son reales; la red
compartida con terceros no. El faucet público de devnet no soltó los ~2 SOL del despliegue.

### 8.2 Apoyado en evidencia externa, no propia

Un escalón más abajo: es información real del mundo, pero de segunda mano —foros, artículos,
documentación de terceros— y no algo que hayamos medido.

- Que en un marketplace real de estrategias **el fraude con backtests está documentado y
  nombrado**, y que la plataforma valida ejecutabilidad pero explícitamente no veracidad.
- Que ese mercado **ya se mudó a la verificación de cuentas en vivo** y su consejo unánime es no
  confiar en backtests.
- Que la razón dicha es la extrapolación, no la aritmética.
- Que **sí existe negocio en verificar afirmaciones de trading** —Myfxbook en forex, AuditZK en
  cripto con enclaves de hardware— y que en los tres mercados mirados el objeto verificado es la
  **realidad de las operaciones**, nunca la correctitud de un cómputo.

Es la base del archivo, y es sólida en dirección. No es una medición nuestra.

### 8.3 Razonado, sin validar

Todo lo de abajo puede estar equivocado. Ninguna de estas conclusiones tiene un experimento
detrás; son deducciones, algunas bastante firmes y otras no.

| Conclusión | Qué la validaría |
|---|---|
| **Nadie paga por verificar backtests** — la que archivó el proyecto. Matizada: sí pagan por verificar *trading en vivo*, no por verificar cómputo | **Cero conversaciones con contrapartes reales.** El track D nunca corrió |
| El filtro de §4 (demanda solo donde el dato es indiscutido) | Aplicarlo a una clase concreta con alguien que pague |
| Que casi todas las patologías vienen de la emisión | La Fase 2 nunca corrió: el wash work no se midió |
| Que existe una ventana de `k` que haga viable la emisión | **E0.2 nunca corrió** |
| Que ZK no conviene para tareas chicas | **E0.1 nunca corrió.** Es la única vía a que la red sea el árbitro, y su costo está sin medir |
| El árbitro pago como seguro por el pagador | Diseñado, no implementado, no probado |
| Que el dilema del verificador es fatal acá | Nunca hubo una disputa real: la tasa de fraude es desconocida |
| Que `accept_delivery` sería dañino | Razonamiento sobre un producto que no existe |
| Que el pagador es el verificador natural | Ningún pagador real verificó nada |
| Que una red de nodos resolvería la disponibilidad de datos | La categoría existe; nunca se probó acá |

### 8.4 Lo que ni se intentó

La Fase 0 entera (E0.1 y E0.2), el track D, las subfases 1.5 y 1.6, y el despliegue en una red
pública.

### 8.5 El resumen incómodo

**Se validó mucha ingeniería y casi nada de la tesis.** Todo lo demostrado es que el mecanismo
se puede construir y funciona; todo lo que se afirma sobre si *sirve* está en §8.3.

Eso incluye la conclusión que archivó el proyecto. Es una decisión tomada con evidencia
indirecta y razonamiento, no con tres "no" de gente real — que era exactamente el gate que el
track D pedía y que nunca se corrió. Sigue siendo la decisión correcta con la información
disponible, y sigue siendo barata de revisar: **una sola conversación con alguien que venda
estrategias mueve la conclusión de §8.3 a §8.1 o la tumba.**

---

## 9. Dónde encaja esta tecnología, y dónde no

La conclusión más útil hacia adelante, y la última que salió. No estaba en el plan original.

### 9.1 El intercambio, entero

Nuestra garantía es **económica** —bond, ventana, impugnación— y no técnica. Eso define un
intercambio muy nítido contra la alternativa que el mercado eligió, las pruebas ZK:

| | verificación optimista (lo nuestro) | ZK |
|---|---|---|
| Costo de producir la garantía | ~0: verificar **es** reejecutar | alto: probar cuesta varios órdenes de magnitud más que ejecutar |
| Hardware del verificador | **un teléfono** — corrimos en ARM en 1,5 s | máquinas grandes, a veces GPU |
| Finalidad | hay que esperar la ventana | inmediata |
| ¿Alguien tiene que vigilar? | **sí, y ese es el problema** | no, nadie |

Somos baratos en cómputo y caros en tiempo y en supuestos de confianza. ZK es el espejo exacto.
Ninguno de los dos domina al otro: es un intercambio, y elegir mal de qué lado pararse es
elegir mal el producto.

### 9.2 Los dos pisos, y la banda muerta entre ellos

Cada enfoque tiene un tamaño mínimo de tarea por debajo del cual deja de cerrar, **y son
distintos**:

- **La verificación optimista necesita escala.** El challenger arriesga su depósito para ganar
  el bond solo si acierta; si el fraude es raro, su valor esperado es negativo y nadie vigila.
  Funciona donde hay tanto en juego que vigilar se paga solo — los rollups de Ethereum, con
  miles de millones. Nuestras órdenes de referencia eran de **0,2 SOL**: tres o cuatro órdenes
  de magnitud por debajo de ese umbral.
- **ZK tiene un piso de costo.** Si probar cuesta un dólar y la tarea vale cincuenta centavos,
  no cierra por elegante que sea.

Entre los dos pisos hay una **banda muerta**: tareas demasiado chicas para financiar a un
vigilante y demasiado chicas para pagar una prueba. Ahí no sirve ninguno de los dos enfoques.

### 9.3 La única configuración que funciona en esa banda

> **Cuando el pagador es el verificador.**

El dilema del verificador desaparece, porque quien chequea no necesita incentivo: **iba a mirar
el resultado de todas formas.** Es trabajo que pidió él y que le importa. Su costo marginal de
verificar es cero, y en nuestro caso son 1,5 segundos en cualquier máquina que tenga a mano.

De ahí sale la descripción precisa del nicho donde esta tecnología es la correcta:

- **Dos partes que se conocen** y transaccionan entre sí, no un mercado anónimo.
- **Trabajo chico**, por debajo del piso donde ZK cierra.
- **El pagador quiere el resultado**, así que lo va a mirar igual.
- **La latencia no importa**: no es un pago en tiempo real, tolera una ventana.

Y hay un argumento genuino de que ese hueco está desatendido justamente porque el resto de la
industria se fue a ZK, apuntando a cómputos grandes a nivel protocolo. Nadie está sirviendo
"dos partes, trabajo chico, verificación gratis".

### 9.4 Lo que eso cambia

**El posicionamiento.** No somos una alternativa a los coprocesadores ZK: somos lo que sirve
**abajo del piso donde ellos cierran**. Es defendible y es específico, y es lo contrario de
competir de frente contra Axiom o Brevis — que además ya combinan las dos garantías
(`Brevis coChain` es PoS con staking y slashing produciendo resultados optimistas, impugnables
con pruebas ZK: nuestra arquitectura, con el respaldo que no teníamos).

**La pregunta para reabrir**, que queda mucho más chica que "buscar una clase de trabajo":

> ¿Existen dos partes que hoy **no cierran un trato** por un trabajo chico, donde el que paga
> podría verificar el resultado él mismo?

Eso ya no es "el mercado de backtests". Es cualquier encargo bilateral con resultado computable.

### 9.5 Y la parte incómoda

**E0.1 era exactamente el experimento que medía esto** —cuánto cuesta probar contra cuánto
cuesta la ventana— y fue lo primero que se salteó para ir a construir. Era el primer paso de la
Fase 0 en el plan maestro.

Si hubiera corrido, habría dicho: *"a este tamaño de tarea la economía no alcanza; necesitás ZK,
o tareas mucho más grandes, o que el pagador sea el verificador"*. Eso reorientaba el producto
**antes de la primera línea de código**, y es la lección de proceso más cara de todo el archivo:
saltear la fase de medición para empezar a construir costó cuatro subfases de trabajo bien hecho
sobre un supuesto que nadie había comprobado.
