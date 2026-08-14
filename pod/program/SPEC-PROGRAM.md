# PoD Escrow — especificación normativa v2

**Subfases 1.3 y 1.4** de [`..._FASE1.md`](../../2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md)
§4 y §5. Tercer documento normativo del proyecto, después de [`../spec/SPEC.md`](../spec/SPEC.md)
(el **pedido**) y [`../prover/SPEC-RUNNER.md`](../prover/SPEC-RUNNER.md) (la **ejecución**).
Este define **el dinero**: quién deposita, dónde queda, por qué caminos sale, y qué pasa
cuando alguien miente.

La v1 cubría 1.3 —escrow sin verificación—. La v2 agrega el camino que hace que el fraude
cueste: `deliver`, `challenge`, `resolve` y `settle`. El changelog está en §9.

Si el programa y este texto difieren, el árbitro es este texto.

---

## 0. Qué tiene que ser cierto

Una sola frase, y todo lo demás sale de ahí:

> En cualquier secuencia de instrucciones, **ningún lamport se crea, se destruye ni queda
> atrapado**. Cada lamport que entra al escrow sale por una ruta declarada, hacia un
> destino que ya estaba grabado en la cuenta antes de que la ruta se abriera.

El énfasis de la subfase no está en la criptografía: está en el tramo aburrido. Los fondos
no se pierden en la parte difícil, se pierden acá.

---

## 1. Alcance — y sobre todo, qué NO está

Esta subfase cierra la máquina de estados con **verificación optimista (Nivel 1)**: se
asume que la entrega es correcta salvo que alguien pague por decir lo contrario.

Lo que **no está**, y no por olvido:

- **ZK.** El modo de prueba `2` está reservado y su interfaz declarada en §8, sin
  implementar. El plan maestro lo condiciona a E0.1, que todavía no corrió.
- **Los eventos `WorkSettled` / `WorkFailed`.** Son la subfase 1.5, y son la salida más
  importante de la Fase 1: se congelan una vez y se reindexa la historia si salen mal.
  Nada acá los adelanta.
- **Slashing por no entregar.** Un worker que acepta y no entrega recupera el bond. El
  castigo existe únicamente para la entrega **falsa**, que es lo que un challenge prueba.
  La diferencia importa: no entregar es incumplir, entregar mal es mentir.

El `spec_hash` y el `output_hash` entran al programa como **32 bytes opacos**. El programa
no los interpreta, no los recalcula y no sabe qué es un recibo: su significado vive en
`../spec/` y `../prover/`. Lo único que la cadena hace con el `output_hash` es **fijarlo
públicamente y ponerle un plazo** — que es exactamente lo que un esquema optimista necesita
de una cadena, y nada más.

## 2. Identidad del programa

```
program_id = D3cmkjUTpEXokdX9Vx5d4ntkU7sYRqsceqU5Yk44FZdD
           = base58( SHA-256( "PoD/Program/1\0" ) )
```

Derivado del registro de dominios de `../spec/SPEC.md` §5.1, con el mismo criterio: el
identificador es función del propósito, no de una clave que alguien tenga guardada. Para el
despliegue real esto se reemplaza por el pubkey del keypair del programa; hasta entonces,
un id reproducible vale más que uno aleatorio.

---

## 3. La cuenta `Order`

### 3.1 Dirección

PDA del programa, con estas semillas en este orden:

```
seeds = [ "order", payer (32 bytes), spec_hash (32 bytes), nonce (u64 little-endian) ]
```

El `nonce` lo elige el pagador. Existe para que un mismo pagador pueda tener **varias
órdenes abiertas del mismo trabajo**: sin él, las semillas colisionarían y el segundo
pedido idéntico sería irrepresentable.

La dirección es función de `(payer, spec_hash, nonce)`, o sea que **cualquiera puede
calcularla sin leer la cadena**. Eso es deliberado: un indexer de Fase 2 tiene que poder
enumerar órdenes sin un registro central.

### 3.2 Layout

304 bytes, tamaño fijo, todos los enteros **little-endian**. Sin borsh y sin serialización
derivada: el layout es normativo y se lee acá, no en un `#[derive]`.

| Offset | Bytes | Campo | Nota |
|---|---|---|---|
| 0 | 1 | `version` | `2`. Otro valor ⇒ la cuenta no es de esta versión y se rechaza |
| 1 | 1 | `state` | `1` CREADA · `2` ACEPTADA · `3` ENTREGADA · `4` DISPUTADA |
| 2 | 1 | `bump` | bump canónico del PDA, grabado en la creación |
| 3 | 1 | `proof_mode` | `1` = optimista. `2` = ZK, reservado y **no implementado** (§8); se rechaza |
| 4 | 4 | `deliver_window_secs` | u32; lo fija el pagador al crear, lo consume `accept_order` |
| 8 | 4 | `challenge_window_secs` | u32; lo consumen `deliver` y `challenge` |
| 12 | 4 | *reservado* | ceros; se rechaza si no lo son |
| 16 | 8 | `nonce` | u64 |
| 24 | 32 | `spec_hash` | opaco |
| 56 | 32 | `payer` | quien depositó la recompensa y el rent |
| 88 | 32 | `worker` | ceros mientras el estado sea CREADA |
| 120 | 32 | `arbiter` | quien resuelve una disputa. Lo elige el **pagador** al crear (§7) |
| 152 | 32 | `challenger` | ceros salvo en DISPUTADA |
| 184 | 32 | `output_hash` | lo que el worker entregó. Ceros antes de ENTREGADA |
| 216 | 32 | `claimed_output_hash` | lo que el challenger afirma que era. Ceros fuera de DISPUTADA |
| 248 | 8 | `reward_lamports` | u64, `> 0` |
| 256 | 8 | `bond_lamports` | u64, `> 0` |
| 264 | 8 | `rent_lamports` | u64; el mínimo exento de rent **en el momento de la creación** |
| 272 | 8 | `challenge_deposit_lamports` | u64, `> 0`. Lo que arriesga quien challengea |
| 280 | 8 | `accept_deadline` | i64, unix seconds |
| 288 | 8 | `deliver_deadline` | i64; `0` mientras el estado sea CREADA |
| 296 | 8 | `challenge_deadline` | i64. **Dos significados** según el estado — ver abajo |

`challenge_deadline` es el único campo con dos lecturas, y conviene que esté dicho y no
deducido: en **ENTREGADA** es el último momento para challengear; en **DISPUTADA** es el
último momento para que el árbitro resuelva. Las dos usan `challenge_window_secs`. Un campo
aparte para el plazo del árbitro sería más explícito y ocho bytes más caro; se prefirió
declarar la ambigüedad a pagarla.

`rent_lamports` se graba en vez de recalcularse al cerrar. El mínimo exento de rent es un
parámetro de la red y **puede cambiar**: si al cerrar se recalculara, la diferencia se
convertiría en un faltante o en un sobrante que nadie reclama. Grabarlo hace que el
reembolso sea exactamente lo que se depositó.

### 3.3 Estados

```
              create_order
                   |
                   v
                CREADA ---- accept_order ----> ACEPTADA ---- deliver ----> ENTREGADA
                   |                              |                            |
   cancel_expired  |              cancel_expired  |               challenge    |    settle
   (venc. aceptar) |              (venc. entrega) |            (con deposito)  |  (venc. ventana)
                   v                              v                            v            v
                cerrada                        cerrada                    DISPUTADA      cerrada
           (todo al pagador)              (bond -> worker,                    |      (reward+bond
                                           resto -> pagador)                  |       -> worker)
                                                                              |
                                              +-------------------------------+
                                              |                               |
                                         resolve(arbitro)              cancel_expired
                                              |                     (el arbitro no vino)
                        +---------------------+---------------+              |
                        |                                     |              v
                   FRAUDE                              INFUNDADO          cerrada
            bond+deposito -> challenger        reward+bond+deposito     (cada uno recupera
            reward+rent   -> pagador                    -> worker        lo suyo, nadie gana)
                                                 rent   -> pagador
```

**No hay estado terminal persistido.** Todo camino que termina cierra la cuenta: datos en
cero y balance en cero, con lo cual el runtime la recolecta. `LIQUIDADA` y `FALLIDA` del
plan maestro no son bytes en una cuenta — son el hecho de que la cuenta se cerró por una
rama o por la otra. Guardarlos sería guardar rent para siempre a cambio de un byte de
historia que 1.5 va a poner en un evento.

Las tres salidas de `cancel_expired` tienen la misma forma: **un vencimiento desarma el
estado en el que se venció**, y nadie gana la discusión que quedó abierta. La rama de
DISPUTADA es la más incómoda de las tres y está ahí a propósito: es lo que pasa cuando el
árbitro —que es un tercero de confianza (§7)— no aparece.

## 4. Instrucciones

Codificación: **byte 0 = discriminante**, el resto son los campos en orden, little-endian.
La longitud es **exacta**: sobrante o faltante ⇒ `E_BAD_INSTRUCTION`. No hay campos
opcionales ni relleno. Es la misma regla que `../spec/SPEC.md` §1 — la ambigüedad se
rechaza, no se normaliza.

| Disc. | Instrucción | Bytes | Quién firma |
|---|---|---|---|
| `0` | `create_order` | 109 | el pagador |
| `1` | `accept_order` | 1 | el worker |
| `2` | `cancel_expired` | 1 | **nadie** |
| `3` | `deliver` | 33 | el worker |
| `4` | `challenge` | 33 | el challenger |
| `5` | `resolve` | 2 | el árbitro |
| `6` | `settle` | 1 | **nadie** |

Cualquier otro discriminante ⇒ `E_BAD_INSTRUCTION`. **No hay instrucción número 7.**

Las dos que no llevan firma son las que hacen que el dinero salga solo: `cancel_expired` y
`settle` son cranks permissionless. Si liquidar necesitara la firma del que cobra, cobrar
dependería de estar vivo, y el escrow tendría un modo de falla que no es económico sino de
disponibilidad.

### 4.0 Los sysvars entran como cuentas

Las tres instrucciones reciben el sysvar `Clock` como cuenta, y `create_order` también
recibe `Rent`. No se leen con `Clock::get()`.

La razón es de reproducibilidad de la evidencia, no de estilo. `Clock::get()` funciona
únicamente cuando el programa corre compilado a SBF: fuera de ese target, el syscall no
existe y la llamada devuelve `UnsupportedSysvar`. Los tests de esta subfase corren el
programa **en proceso** (ver el README), que es lo que permite probar el modelo de cuentas
sin un validador local ni platform-tools. Con `Clock::get()`, el gate solo sería
comprobable con la cadena entera levantada.

El costo son una o dos cuentas de solo lectura por instrucción, y a cambio queda explícito en la
lista de cuentas **todo** lo que el programa lee. El programa igual comprueba las
direcciones contra `sysvar::clock::ID` y `sysvar::rent::ID`, así que pasar otra cuenta es
`E_BAD_ACCOUNTS` y no un valor de reloj elegido por quien llama.

### 4.1 `create_order`

```
[0]        u8      disc = 0
[1..9]     u64     nonce
[9..41]    [u8;32] spec_hash
[41..73]   [u8;32] arbiter                    != 0
[73..81]   u64     reward_lamports            > 0
[81..89]   u64     bond_lamports              > 0
[89..97]   u64     challenge_deposit_lamports > 0
[97..101]  u32     accept_window_secs         en [60, 2_592_000]
[101..105] u32     deliver_window_secs        en [60, 2_592_000]
[105..109] u32     challenge_window_secs      en [60, 2_592_000]
```

| # | Cuenta | firma | escribe |
|---|---|---|---|
| 0 | `payer` | sí | sí |
| 1 | `order` (PDA) | no | sí |
| 2 | `system_program` | no | no |
| 3 | `sysvar::rent` | no | no |
| 4 | `sysvar::clock` | no | no |

Efecto: crea el PDA con 152 bytes y lo deja con `rent_exempt_minimum + reward_lamports`.
Estado `CREADA`, `worker` en ceros, `deliver_deadline = 0`,
`accept_deadline = clock.unix_timestamp + accept_window_secs`.

**La creación no usa `system_instruction::create_account`.** Usa la secuencia
`transfer` → `allocate` → `assign`, que es equivalente pero tolera que la cuenta ya tenga
lamports. La diferencia importa: `create_account` falla si el balance no es cero, así que
un tercero puede **bloquear la creación de una orden mandándole un lamport al PDA**, cuya
dirección es pública y calculable de antemano (§3.1). Con `transfer/allocate/assign` la
donación reduce lo que el pagador tiene que poner, y §4.3 se la devuelve.

Las tres ventanas, el depósito de challenge y el árbitro los fija el **pagador**, en la
creación. El worker no negocia nada: acepta los términos o no acepta. Que el árbitro sea
parte de los términos —visible antes de aceptar, igual que la recompensa— es lo que hace
tolerable que sea un tercero de confianza (§7).

### 4.2 `accept_order`

```
[0]  u8  disc = 1
```

| # | Cuenta | firma | escribe |
|---|---|---|---|
| 0 | `worker` | sí | sí |
| 1 | `order` (PDA) | no | sí |
| 2 | `system_program` | no | no |
| 3 | `sysvar::clock` | no | no |

Precondiciones: `state == CREADA` y `clock.unix_timestamp <= accept_deadline`.

Efecto: transfiere **exactamente** `bond_lamports` de `worker` al PDA vía el programa del
sistema; graba `worker`, pasa a `ACEPTADA` y fija
`deliver_deadline = clock.unix_timestamp + deliver_window_secs`.

El bond no es un campo que el worker declare: es un movimiento de lamports dentro del mismo
handler. **No existe un camino en el que el estado pase a `ACEPTADA` y la transferencia no
ocurra**, porque si la transferencia falla, falla la instrucción, y con ella la transacción
entera. Eso es lo que hace verificable al gate 2 de `..._FASE1.md` §4.

El pagador puede aceptar su propia orden. Es autotrato, y en 1.3 no hay nada que lo haga
rentable —no hay emisión, y bond y recompensa vuelven al mismo bolsillo menos fees— pero
**sí lo habrá en Fase 3**. Queda declarado en §7.

### 4.3 `cancel_expired`

```
[0]  u8  disc = 2
```

| # | Cuenta | firma | escribe | cuándo |
|---|---|---|---|---|
| 0 | `order` (PDA) | no | sí | siempre |
| 1 | `payer` | no | sí | siempre; igual a `order.payer` |
| 2 | `sysvar::clock` | no | no | siempre |
| 3 | `worker` | no | sí | desde `ACEPTADA`; igual a `order.worker` |
| 4 | `challenger` | no | sí | **solo** en `DISPUTADA`; igual a `order.challenger` |

**No lleva firmante.** Es un crank permissionless: cualquiera puede ejecutarla, incluido un
bot que no tiene nada que ver con la orden. Es a propósito — si hiciera falta la firma del
pagador, el reembolso dependería de que el pagador siga vivo y con ganas. Los destinos no
son un parámetro: salen de los bytes ya grabados en la cuenta, así que quien llama no puede
desviar un lamport, y no cobra nada por llamar.

Tres ramas, una por cada estado que puede vencer:

**CREADA**, con `now > accept_deadline` — se pasan **todos** los lamports del PDA al
`payer`. El balance completo, no `reward + rent`: si alguien donó lamports (§4.1), se van
con el resto en vez de quedar huérfanos.

**ACEPTADA**, con `now > deliver_deadline` — `bond_lamports` al `worker`, y **todo el
resto** al `payer`. El bond vuelve entero: no entregar es incumplir, no mentir (§1).

**DISPUTADA**, con `now > challenge_deadline` — el árbitro no resolvió a tiempo.
`bond_lamports` al `worker`, `challenge_deposit_lamports` al `challenger`, el resto al
`payer`. **La disputa no se decide: se desarma.** Nadie gana, nadie pierde, y la orden se
cierra sin veredicto. Es la consecuencia visible de que el árbitro sea un tercero de
confianza, y está acá para que esa consecuencia sea un camino escrito y no un fondo
congelado para siempre.

`ENTREGADA` **no está** entre las ramas: su salida es `settle` (§4.8), no un reembolso. Una
entrega que sobrevive su ventana se liquida; no se cancela.

Antes del vencimiento correspondiente ⇒ `E_NOT_EXPIRED`. Las tres ramas cierran según §4.4.

### 4.4 Cierre

Cerrar significa, en este orden:

1. Repartir los lamports según la rama, dejando el PDA en **0**.
2. Poner los 152 bytes en cero.
3. `realloc(0)` y reasignar la cuenta al programa del sistema.

El paso 3 es el que impide la *revival*: una cuenta con balance 0 la recolecta el runtime al
final de la transacción, pero **dentro de la misma transacción** sigue existiendo y con el
programa como dueño. Reasignarla hace que una segunda instrucción de la misma transacción ya
no la vea como cuenta del programa. Sin ese paso, el chequeo de dueño sería lo único que
separa un cierre de un doble cobro, y no conviene que sea lo único.

---

### 4.5 `deliver`

```
[0]     u8      disc = 3
[1..33] [u8;32] output_hash    != 0
```

| # | Cuenta | firma | escribe |
|---|---|---|---|
| 0 | `worker` | sí | no |
| 1 | `order` (PDA) | no | sí |
| 2 | `sysvar::clock` | no | no |

Precondiciones: `state == ACEPTADA`, el firmante es `order.worker`, y
`now <= deliver_deadline`. Efecto: `state = ENTREGADA`, se graba `output_hash` y
`challenge_deadline = now + challenge_window_secs`.

**No se puede reescribir una entrega.** Un segundo `deliver` da `E_BAD_STATE`: si se pudiera
corregir el hash, un worker esperaría a ver si alguien challengea y recién ahí decidiría qué
entregó, que es exactamente lo contrario de comprometerse.

Un `output_hash` en ceros se rechaza con `E_ZERO_HASH`. No es una restricción criptográfica
—ningún SHA-256 va a dar cero— sino de legibilidad: los ceros son lo que ve un lector en un
campo sin inicializar, y aceptarlos haría indistinguible "no entregó" de "entregó ceros".

**El recibo no entra a la cadena.** `..._FASE1.md` §5 dice `deliver(output_hash, receipt)`;
acá viaja solo el hash. El recibo de [`../prover/SPEC-RUNNER.md`](../prover/SPEC-RUNNER.md)
§4 contiene el `sweep_top.v1` entero —kilobytes— y su firma Ed25519 la haría un `runner_id`
que la cadena no conoce. Lo que la cadena necesita de la entrega es un **compromiso público
con plazo**, y la firma de la transacción del worker es un compromiso más fuerte que el del
recibo: es la misma clave que puso el bond. El recibo sigue siendo el artefacto que un
challenger usa para verificar off-chain; simplemente no hace falta subirlo.

### 4.6 `challenge`

```
[0]     u8      disc = 4
[1..33] [u8;32] claimed_output_hash    != 0, != order.output_hash
```

| # | Cuenta | firma | escribe |
|---|---|---|---|
| 0 | `challenger` | sí | sí |
| 1 | `order` (PDA) | no | sí |
| 2 | `system_program` | no | no |
| 3 | `sysvar::clock` | no | no |

Precondiciones: `state == ENTREGADA` y `now <= challenge_deadline`. Efecto: transfiere
`challenge_deposit_lamports` del challenger al PDA, graba `challenger` y
`claimed_output_hash`, pasa a `DISPUTADA`, y **reinicia** `challenge_deadline` a
`now + challenge_window_secs` — que a partir de acá es el plazo del árbitro (§3.2).

**El challenger tiene que comprometerse a una respuesta.** No alcanza con decir "está mal":
hay que decir cuál era el hash correcto. Eso cambia la naturaleza del challenge — de una
objeción a una afirmación falsable— y le da al árbitro un procedimiento en vez de un juicio:
recalcula, y compara contra dos hashes que ya están escritos. Un challenge que afirma el
mismo hash entregado se rechaza con `E_SAME_HASH`: no hay nada que arbitrar.

Como en `accept_order`, el depósito **es** la transferencia, no un campo declarado: no
existe un camino en el que el estado pase a `DISPUTADA` sin que el depósito haya entrado.
Un challenge gratis sería un ataque de denegación gratis.

Un solo challenger por orden. El segundo recibe `E_BAD_STATE`.

### 4.7 `resolve`

```
[0]  u8  disc = 5
[1]  u8  verdict   1 = FRAUDE, 2 = INFUNDADO
```

| # | Cuenta | firma | escribe |
|---|---|---|---|
| 0 | `arbiter` | sí | no |
| 1 | `order` (PDA) | no | sí |
| 2 | `payer` | no | sí |
| 3 | `worker` | no | sí |
| 4 | `challenger` | no | sí |
| 5 | `sysvar::clock` | no | no |

Precondiciones: `state == DISPUTADA`, el firmante es `order.arbiter`, las tres cuentas de
destino son las grabadas, y `now <= challenge_deadline`.

| Veredicto | Reparto |
|---|---|
| `FRAUDE` (1) | bond + depósito → **challenger** · recompensa + rent → **pagador** |
| `INFUNDADO` (2) | recompensa + bond + depósito → **worker** · rent → **pagador** |

Cualquier otro valor ⇒ `E_BAD_VERDICT`.

Las cuatro cuentas van **siempre**, gane quien gane: quien liquida no elige a quién paga.
El árbitro decide *qué* pasó, no *a quién* le toca — los destinos ya estaban escritos antes
de que hubiera disputa. Es la diferencia entre un árbitro que puede equivocarse y un árbitro
que puede robar.

El depósito del challenger, cuando el challenge es infundado, va al **worker**: es quien
sufrió la demora. Que no vuelva al pagador es deliberado — el pagador no arriesgó nada en
la disputa.

Pasado su plazo el árbitro ya no decide, ni siquiera con el veredicto correcto (§4.3,
rama DISPUTADA). Un árbitro que llega tarde no puede llegar igual: si pudiera, el plazo no
sería un plazo.

### 4.8 `settle`

```
[0]  u8  disc = 6
```

| # | Cuenta | firma | escribe |
|---|---|---|---|
| 0 | `order` (PDA) | no | sí |
| 1 | `payer` | no | sí |
| 2 | `worker` | no | sí |
| 3 | `sysvar::clock` | no | no |

Precondiciones: `state == ENTREGADA` y `now > challenge_deadline`. Efecto: recompensa +
bond al `worker`, el resto —rent y cualquier donación— al `payer`, y la cuenta se cierra.

Sin firmante, como `cancel_expired`. Es el camino que se recorre casi siempre: **el caso
normal de un esquema optimista es que nadie challengee**, y ese caso no le cuesta a nadie
más que una transacción de una firma.

---

## 5. Invariantes — lo que los tests tienen que probar

Numeradas porque los tests las citan por nombre.

| # | Invariante |
|---|---|
| **I1** | **Conservación.** En toda transacción exitosa, la suma de lamports de las cuentas involucradas antes = después + fee. Ningún camino crea ni destruye lamports |
| **I2** | **Sin rent atrapado.** Todo camino que termina deja el PDA en 0 lamports; el rent vuelve íntegro a quien lo puso, el pagador |
| **I3** | **Sin fondos huérfanos.** El cierre drena el **balance**, no las cantidades grabadas. Lamports donados por terceros vuelven al pagador |
| **I4** | **No hay aceptación sin bond.** Si el worker no tiene `bond_lamports`, la transacción falla entera y el estado sigue `CREADA` |
| **I5** | **El pagador no cancela después de `ACEPTADA`.** Entre `accept_order` y `deliver_deadline` no existe ninguna ruta que devuelva fondos al pagador |
| **I6** | **Destinos fijos.** `cancel_expired` es permissionless, pero paga a los pubkeys grabados. Quien la invoca no cobra |
| **I7** | **Rutas declaradas.** Las únicas instrucciones que mueven lamports de un PDA `Order` son las siete de §4. No hay autoridad administrativa, ni cuenta de fees, ni ruta de upgrade de datos |
| **I8** | **Mentir cuesta el bond.** Un worker que entrega un `output_hash` falso y es challengeado dentro de la ventana pierde `bond_lamports`, que van al challenger |
| **I9** | **Challengear en falso cuesta el depósito.** Un worker honesto challengeado en falso cobra la recompensa completa, y el challenger pierde `challenge_deposit_lamports` |
| **I10** | **El árbitro decide qué pasó, no a quién le toca.** Los tres destinos de `resolve` son los pubkeys grabados antes de que hubiera disputa; el árbitro no puede redirigir un lamport, solo elegir entre dos repartos ya escritos |

---

## 6. Errores

Todos se devuelven como `ProgramError::Custom(n)`.

| n | Código | Cuándo |
|---|---|---|
| 1 | `E_BAD_INSTRUCTION` | discriminante desconocido, o longitud que no es la exacta |
| 2 | `E_BAD_ACCOUNTS` | cantidad de cuentas equivocada, falta un permiso de escritura, o el programa del sistema / un sysvar no es el que dice ser |
| 3 | `E_BAD_PDA` | `order` no es el PDA de `(payer, spec_hash, nonce)`, o su dueño no es el programa |
| 4 | `E_ALREADY_EXISTS` | `create_order` sobre una cuenta que ya tiene datos |
| 5 | `E_BAD_STATE` | la instrucción no aplica al estado actual |
| 6 | `E_NOT_EXPIRED` | todavía no se pasó el vencimiento de la rama |
| 7 | `E_EXPIRED` | `accept_order` después de `accept_deadline` |
| 8 | `E_NOT_SIGNER` | falta la firma requerida |
| 9 | `E_WRONG_PAYER` | la cuenta pasada no coincide con `order.payer` |
| 10 | `E_WRONG_WORKER` | la cuenta pasada no coincide con `order.worker` |
| 11 | `E_ZERO_REWARD` | `reward_lamports == 0` |
| 12 | `E_ZERO_BOND` | `bond_lamports == 0` |
| 13 | `E_BAD_WINDOW` | ventana fuera de `[60, 2_592_000]` |
| 14 | `E_BAD_ACCOUNT_DATA` | `version`, `state`, el largo o los bytes reservados no son los esperados |
| 15 | `E_OVERFLOW` | una suma de lamports o de tiempo se sale de rango |
| 16 | `E_WRONG_ARBITER` | quien firma `resolve` no es `order.arbiter`, o se creó con árbitro en ceros |
| 17 | `E_BAD_VERDICT` | el veredicto no es `1` ni `2` |
| 18 | `E_ZERO_DEPOSIT` | `challenge_deposit_lamports == 0` |
| 19 | `E_WRONG_CHALLENGER` | la cuenta pasada no coincide con `order.challenger` |
| 20 | `E_SAME_HASH` | el challenge afirma el mismo hash que se entregó |
| 21 | `E_ZERO_HASH` | `output_hash` o `claimed_output_hash` en ceros |

---

## 7. La decisión: cómo se resuelve un challenge

`..._FASE1.md` §5 pide tomar esta decisión **en voz alta**, con tres opciones sobre la mesa:
(a) re-ejecución off-chain por un árbitro designado, (b) bisección interactiva on-chain,
(c) ZK solo del paso disputado.

**Se eligió (a). El sistema tiene un tercero de confianza, y esto es la declaración.**

### 7.1 Qué significa exactamente

Cuando hay disputa, un árbitro re-ejecuta el pedido off-chain siguiendo
[`../prover/SPEC-RUNNER.md`](../prover/SPEC-RUNNER.md), obtiene un `output_hash`, y firma
`resolve` con `FRAUDE` si coincide con `claimed_output_hash`, o con `INFUNDADO` si coincide
con `output_hash`. El determinismo cross-machine del gate 1.2 es lo que hace que ese
procedimiento sea un procedimiento y no una opinión: **cualquiera puede repetirlo y llegar
al mismo resultado**, y por eso un árbitro que miente es un árbitro al que se le puede
probar la mentira, aunque el contrato no pueda hacerlo por sí solo.

El árbitro **no es global**: lo elige el pagador por orden y queda grabado en la cuenta
(§3.2), visible antes de que nadie acepte. Un worker que no confía en ese árbitro no acepta
la orden. Es la mitigación más honesta disponible sin construir (b) o (c): la confianza es
un término del contrato, no una propiedad oculta del sistema.

### 7.2 Qué puede hacer y qué no

| Puede | No puede |
|---|---|
| Decidir mal a propósito y hacer perder el bond a un worker honesto | Redirigir un lamport: los destinos son los pubkeys grabados (I10) |
| Coludirse con el pagador contra el worker | Actuar sin que haya habido un challenge con depósito |
| No aparecer, y congelar la orden hasta que venza | Decidir después de su plazo (§4.7) |
| | Tocar una orden que no lo nombró |

El daño máximo de un árbitro malicioso está acotado a `bond + depósito` **de las órdenes que
lo nombraron**, y es visible: quien quiera puede re-ejecutar y publicar la discrepancia.

### 7.3 Plan de reemplazo

Está escrito acá porque una frontera declarada sin plan de salida es una excusa:

1. **E0.1 primero.** El plan maestro condiciona ZK a medir el overhead de verificación. Si
   E0.1 da que verificar el paso disputado cuesta menos que la ventana, (c) reemplaza a (a)
   para esa clase de trabajo. La interfaz ya está declarada en §8.
2. **La forma del reemplazo no toca la máquina de estados.** Lo único que cambia es *quién o
   qué* produce el veredicto: `resolve` pasa de "el árbitro firma" a "el verificador acepta
   una prueba". Los estados, los plazos y los repartos quedan iguales. Eso es deliberado —
   la parte cara de esta subfase se diseñó para sobrevivir al reemplazo.
3. **Mientras tanto, medir.** Cada disputa resuelta es un dato sobre la tasa real de fraude.
   Si esa tasa es cero durante toda la v0, (b) y (c) son gasto especulativo; si no lo es, el
   número justifica el costo.

---

## 8. La interfaz ZK — declarada, no implementada

`..._FASE1.md` §5 pide la interfaz `Zk { verifier_key }` definida y sin implementar. Lo
está, y el punto de extensión es real: `proof_mode` es un byte de la cuenta (§3.2) con el
valor `2` reservado. Hoy `unpack` **rechaza** cualquier orden con `proof_mode != 1`, así que
no hay forma de crear una orden ZK por accidente.

Lo que haría falta para implementarlo, para que quede claro que no es una casilla vacía:

- `verifier_key` (32 bytes) en la cuenta, elegido por el pagador junto con el árbitro, y un
  `create_order` que acepte `proof_mode = 2`.
- Una instrucción `prove(proof)` que reemplace a `challenge` + `resolve`: si la prueba
  verifica contra `verifier_key` y contradice el `output_hash` entregado, el reparto es el
  de `FRAUDE`, sin árbitro y sin plazo de arbitraje.
- El circuito, que es el trabajo de verdad, y que solo tiene sentido construir para una
  clase de trabajo concreta. Para `backtest.sweep.v1` significaría probar en ZK un barrido
  de ~500 backtests sobre 17 MB de ticks — el orden de magnitud que E0.1 tiene que medir
  antes de que nadie lo intente.

Con `proof_mode = 2` el campo `arbiter` quedaría sin uso, y ese es el punto: la máquina de
estados no cambia, cambia quién ocupa el lugar del que decide.

---

## 9. Changelog

**v2 — subfase 1.4.** La cuenta pasa de 152 a 304 bytes y `version` de `1` a `2`. La v1 no
llegó a desplegarse en ninguna parte, así que no hay migración que escribir; el byte sube
igual porque un lector que confunda los dos layouts lee basura, y eso tiene que fallar en la
puerta y no más adentro.

Agrega: estados `ENTREGADA` y `DISPUTADA`; campos `arbiter`, `challenger`, `output_hash`,
`claimed_output_hash`, `challenge_deposit_lamports`, `challenge_window_secs`,
`challenge_deadline`, `proof_mode`; instrucciones `deliver`, `challenge`, `resolve` y
`settle`; una tercera rama de `cancel_expired` para el árbitro ausente; los códigos de error
16 a 21; y las invariantes I8, I9 e I10.

No cambia: la canonicalización del PDA (§3.1), el modelo de cierre (§4.4), ni ninguno de los
repartos de 1.3.

---

## 10. Lo demás que se decide no decidir

**Slashing por no entregar.** Un worker que acepta y no entrega recupera el bond entero al
vencer `deliver_deadline`. El bond se pierde por **mentir**, no por incumplir. Eso deja
abierto un griefing barato —aceptar órdenes para congelar los fondos del pagador hasta el
vencimiento, sin más costo que las fees— y la mitigación natural (castigar la no entrega)
tiene un costo propio: un worker honesto con un problema de infraestructura pagaría lo mismo
que un saboteador. Sin datos de la tasa real de abandono, elegir un castigo es elegir un
número; queda para cuando 1.5 los produzca.

**Autotrato.** Nada impide que `payer == worker`, ni que el pagador se nombre árbitro de su
propia orden. En 1.4 lo primero no rinde y lo segundo lo ve el worker antes de aceptar. En
Fase 3, con emisión, el autotrato es la primera forma de wash work: es entrada de la subfase
2.2, el grafo de financiamiento, y no una regla que se pueda escribir en el contrato sin un
detector calibrado detrás.

**El pagador se compromete hasta `accept_deadline`.** No hay cancelación anticipada, ni
siquiera si nadie aceptó todavía. Es lo conservador: una cancelación temprana es una carrera
contra `accept_order`, y perder esa carrera de un lado o del otro es exactamente la clase de
bug que estas subfases existen para no tener.

**El pagador no puede liquidar por confirmación.** No existe un `accept_delivery` que el
pagador firme para pagar en el acto, y la ausencia es deliberada. Sería la instrucción más
pedida del contrato: elimina la ventana y con ella el capital inmovilizado, que es el único
costo que la medición de §COSTO señala como relevante. Se decidió no agregarla porque
**convertiría la ventana en un trámite opcional, y la ventana es lo que produce la prueba**.

Una liquidación por confirmación del pagador es una **atestación**: dos partes de acuerdo.
Una liquidación por ventana vencida es otra cosa: nadie objetó habiendo podido. La primera
no distingue trabajo real de dos cuentas coludidas, y en Fase 3 —donde el registro liquidado
sostiene emisión— esa diferencia es todo. Un escrow que liquida por confirmación ya existe
y se llama Upwork; lo que PoD agrega es exactamente lo que `accept_delivery` sacaría.

Si el track D mata la Fase 3 y lo que sobrevive es el escrow verificable sobre stablecoins,
esta instrucción pasa a ser correcta y hay que agregarla. Queda escrita acá para que esa sea
una decisión con su motivo y no un olvido.

**El incentivo a challengear no existe.** Un challenger que acierta recupera su depósito y
cobra el bond del mentiroso; si el fraude es raro, el valor esperado de challengear es
negativo —hay que ejecutar, bajar los datos y arriesgar el depósito para casi nunca cobrar—.
Y si nadie challengea, el fraude deja de ser raro. **La probabilidad de que alguien chequee
es el parámetro central del sistema y hoy no está ni medido ni incentivado.** No es un
problema de esta subfase, pero tampoco tiene subfase asignada; ver `..._FASE1.md` §5.

**Disponibilidad de datos.** El programa fija un `spec_hash` y un `output_hash`, y ninguno
de los dos contiene los bytes que hacen falta para verificar: el pedido y los 17 MB del
dataset. Un challenger que no consiga esos bytes no puede challengear, y un árbitro que no
los consiga no puede resolver. **La ventana de challenge de este documento presupone que
existen; el documento no dice dónde.** Es la deuda más grande que 1.4 deja abierta y está
anotada acá para que no se pierda entre las líneas del README.
