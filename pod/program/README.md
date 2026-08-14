# pod-escrow — escrow con verificación optimista en Solana nativo

> **Native Solana program (no Anchor).** Escrow for paid work with an optimistic
> verification layer: a worker posts a bond, delivers a hash, and anyone can challenge it
> within a window. 7 instructions, 304-byte fixed account layout written by hand, 34 tests
> organized by *invariant* rather than by function, and a JS client implemented independently
> from the written spec to cross-validate it. Runs both in-process and as a compiled `.so`
> on a validator. Docs and specs are in Spanish.
>
> `bash scripts/gate.sh` → 34 tests, exit 0 · `bash devnet/localnet.sh` → both dispute
> scenarios verified on chain.

**Estado: proyecto de investigación archivado** (ver [`ARCHIVO.md`](../../ARCHIVO.md)). El
programa funciona y está verificado; lo que se archivó fue la tesis de producto que lo
motivaba, no el código. Se desplegó en un **validador local**, no en devnet ni mainnet — la
diferencia está declarada en [`devnet/README.md`](devnet/README.md) y no se pinta de otra cosa.

---

## Qué hace

Un pagador crea una orden con una recompensa y el hash de un pedido de trabajo. Un worker la
acepta dejando un **bond**. Entrega el hash de su resultado. A partir de ahí se abre una
**ventana de challenge**: si nadie objeta, el worker cobra; si alguien objeta —dejando un
depósito propio— un árbitro declarado de antemano decide quién mentía, y el que mintió paga.

Eso es *verificación optimista*: no se verifica cada entrega, se verifica la que alguien tiene
incentivo económico para disputar.

Las siete instrucciones: `create` · `accept` · `deliver` · `settle` · `challenge` · `resolve` ·
`cancel_expired`.

## Verificarlo

```bash
bash scripts/gate.sh        # 34 tests en proceso, sin validador
bash devnet/build.sh        # compila a SBF (sbpf v3)
bash devnet/localnet.sh     # levanta validador, despliega el .so y corre el gate en cadena
```

**En Windows no compila**, y no es culpa del programa: `solana-program-test` arrastra `openssl`
vendorizado, cuyo build script pide `perl` y `nmake`. Desde Git Bash, con WSL:

```bash
MSYS_NO_PATHCONV=1 wsl -d Ubuntu-24.04 -- bash /mnt/c/.../pod/program/scripts/gate.sh
```

`MSYS_NO_PATHCONV` frena la traducción de rutas de Git Bash, que si no convierte `/mnt/c/...`
en `C:/Program Files/Git/mnt/c/...`.

## Decisiones técnicas que vale la pena mirar

**Solana nativo, sin Anchor.** Con Anchor, buena parte de la seguridad de cuentas vive en
macros de constraints, y los tests terminan probando que Anchor hace lo que promete. Acá cada
chequeo está escrito a mano y **cada uno tiene su test**: dueño de la cuenta, firmante, PDA
derivado, discriminante, longitud exacta del buffer, cuenta del sistema real y no una
suplantada. Dos dependencias de runtime en total: `solana-program` y `solana-system-interface`.

**Los tests se organizan por invariante, no por función.** Hay diez invariantes numerados
(I1–I10) en [`SPEC-PROGRAM.md`](SPEC-PROGRAM.md) §6, y cada archivo de test prueba un grupo:

| archivo | qué prueba |
|---|---|
| `tests/conservacion.rs` | I1–I3: cada camino conserva balances |
| `tests/prohibiciones.rs` | I4–I7: bond, cancelación, rutas de retiro |
| `tests/disputa.rs` | I8–I10: fraude, challenge infundado, árbitro ausente |
| `tests/maquina.rs` | transiciones de estado y límites de entrada |
| `src/state.rs` | el layout de 304 bytes, fijado por offset |

**La conservación se escribe como igualdad exacta.** La fee la paga siempre una cuenta que no
es parte de la orden, así que los balances de pagador, worker y challenger se comparan sin
ruido: la suma antes y después tiene que dar idéntica, no aproximada.

**El cliente no comparte una línea con el programa.** [`devnet/ix.mjs`](devnet/ix.mjs) arma las
siete instrucciones en JavaScript leyendo `SPEC-PROGRAM.md` §4, no importando el código Rust.
Si el documento fuera ambiguo, el cliente y el programa no se entenderían — y eso es
exactamente el punto: valida la spec, no solo la implementación.

**Dos formas de correr el mismo programa.** En proceso (`solana-program-test` con `processor!`)
para iterar en segundos con el modelo de cuentas real: rent, lamports, PDAs, CPI al programa
del sistema, reglas del runtime sobre cambio de dueño. Y en cadena, con el `.so` compilado
sobre un validador: VM real, presupuesto de cómputo real, fees reales, reloj real. Un programa
que solo corre nativo no es un programa.

**Cerrar una cuenta sin dejarla revivir.** No alcanza con vaciar los lamports: hay que poner la
data en cero, hacer `resize(0)` y reasignar el dueño al programa del sistema. `i7_una_orden_
cerrada_no_revive` es el test que lo fija.

## El árbitro: la frontera de confianza, declarada

Este sistema **tiene un tercero de confianza**, y conviene decirlo fuerte porque es la decisión
de diseño más importante. Alternativas consideradas: (a) árbitro off-chain, (b) bisección
interactiva on-chain, (c) prueba ZK del paso disputado. Se eligió (a). La declaración completa
está en [`SPEC-PROGRAM.md`](SPEC-PROGRAM.md) §7; lo esencial:

- El árbitro **lo elige el pagador por orden** y queda grabado en la cuenta, visible antes de
  que nadie acepte. Un worker que no confía en ese árbitro no acepta. **La confianza es un
  término del contrato, no una propiedad escondida del sistema.**
- El árbitro decide **qué pasó**, no **a quién le toca**: los tres destinos de `resolve` son
  pubkeys grabados antes de que existiera la disputa. Puede equivocarse; **no puede robar** (I10).
- Si no aparece, la disputa **no se decide, se desarma**: cada uno recupera lo suyo. Un fondo
  congelado para siempre no es una opción, ni siquiera como caso raro.
- El procedimiento del árbitro es re-ejecutar según [`../prover/SPEC-RUNNER.md`](../prover/SPEC-RUNNER.md).
  El determinismo cross-machine verificado ahí es lo que lo convierte en procedimiento y no en
  opinión: cualquiera puede repetirlo y probar que el árbitro mintió.

La interfaz ZK está **declarada y no implementada** (§8): `proof_mode = 2` está reservado y hoy
se rechaza en la puerta.

## Evidencia

`bash scripts/gate.sh` → **34 tests, exit 0**.
`bash devnet/localnet.sh` → **los dos escenarios de disputa, en cadena**, con el `.so` de
81 128 bytes desplegado en un validador local (agave 4.2.0). Salida literal en
[`CADENA.txt`](CADENA.txt).

| Escenario | Cómo se prueba | |
|---|---|---|
| Un worker que entrega un `output_hash` falso pierde el bond | `disputa.rs`, midiendo pagador + worker + challenger antes y después | ok |
| Un worker honesto challengeado en falso cobra igual, y el challenger pierde el depósito | ídem | ok |
| El árbitro ausente no decide ni con el veredicto correcto; `cancel_expired` desarma | | ok |
| Solo el árbitro declarado resuelve, y no puede cambiar un destino | pagador, worker y un desconocido lo intentan | ok |
| Bordes: entrega doble, entrega ajena, hash en ceros, challenge tardío, challenge que afirma lo mismo, challenger sin depósito | | 11/11 |
| Conservación de balances en los tres caminos de escrow | | 4/4 |
| Bond, cancelación, rutas de retiro: discriminantes 7..0xff, longitudes inexactas, PDA ajeno, cuenta cerrada | | 7/7 |
| Máquina de estados y límites de entrada | | 8/8 |
| Layout de la cuenta, fijado por offset | | 3/3 |

## Costo medido por orden

Congelado en [`COSTO.tsv`](COSTO.tsv), reproducible con `cargo test --test costo -- --nocapture`.

| | lamports | SOL | ¿vuelve? |
|---|---|---|---|
| Rent exento de 304 bytes | 3 006 720 | 0,0030067 | **sí**, entero al cerrar |
| Camino normal, 4 tx | 20 000 | 0,00002 | no |
| Camino disputado, 6 tx | 30 000 | 0,00003 | no |
| Despliegue del programa (81 KB) | 567 421 000 | 0,567421 | sí, una sola vez |

**El rent es un depósito, no un costo** — vuelve entero por I2. Lo único que se quema son
20 000 lamports por orden completa, o 30 000 si hay disputa.

El hallazgo, que es lo contrario de lo esperado: **el costo de transacción no fija el tamaño
mínimo de tarea viable.** Es despreciable frente a cualquier tarea que valga la pena pedir. Lo
que sí pesa es el **capital inmovilizado** —recompensa + bond + depósito + rent— durante toda la
ventana. La restricción económica no es abrir la orden: es tenerla abierta. Y la verificación
optimista **alarga** ese plazo, porque le suma la ventana de challenge.

## La deuda abierta

**Disponibilidad de datos.** El programa fija un `spec_hash` y un `output_hash`, y ninguno
contiene los bytes que hacen falta para verificar. Un challenger que no consiga esos bytes no
puede challengear; un árbitro que no los consiga no puede resolver. **La ventana de challenge
presupone que existen; nada en el sistema dice dónde.** Es la deuda más grande del diseño y
está declarada en [`SPEC-PROGRAM.md`](SPEC-PROGRAM.md) §10.

## Mapa de archivos

```
SPEC-PROGRAM.md         normativo: cuentas, layout, 7 instrucciones, invariantes I1-I10, 21 errores
src/state.rs            la cuenta Order: layout fijo de 304 bytes, a mano
src/instruction.rs      codificacion de las 7 instrucciones + derivacion del PDA
src/processor.rs        los 7 handlers
src/error.rs            los 21 codigos de error
tests/                  34 tests, agrupados por invariante
scripts/gate.sh         corre todo, en proceso
devnet/ix.mjs           las 7 instrucciones reimplementadas en JS desde el documento
devnet/gate.mjs         los escenarios de disputa contra una cadena de verdad
devnet/localnet.sh      levanta un validador, despliega el .so y corre gate.mjs
```

`Program Id: PoDJWDugBecU1jjXJtvPTQgUQKVv9rBcNpK1hCfpmS1`
