# PoD — Proof of Delivery

> **Verifiable work as a settlement unit.** A written spec, two independent implementations
> that validate it by disagreeing when it's ambiguous, a deterministic runner with a signed
> receipt, and a native Solana escrow with optimistic verification. Zero runtime dependencies
> in the spec and prover layers; no Anchor in the program. Specs and docs are in Spanish.

**Estado: archivado.** El código funciona y está verificado — lo que se archivó fue la tesis de
producto, no la ingeniería. El porqué, con la evidencia y las conclusiones que quedaron
validadas y las que no, está en [`ARCHIVO.md`](ARCHIVO.md).

---

## La idea

Si el trabajo puede **entregarse de forma verificable**, puede liquidarse como una transacción.
El sistema define un pedido de trabajo con un hash canónico, un runner que lo ejecuta de forma
determinística y firma un recibo, y un escrow on-chain que paga contra ese recibo — con una
ventana durante la cual cualquiera puede objetar la entrega poniendo plata propia detrás de la
objeción.

## Los tres componentes

### [`pod/spec`](pod/spec) — formato canónico y `spec_hash`

El formato de `WorkOrder` está definido en un documento normativo, y se implementa **dos veces
de forma independiente**: una en Python (solo `hashlib` y `re`) y otra en Rust **con cero
dependencias — SHA-256 y parser propios**. Las dos se escriben contra el texto, no una contra
la otra.

Esa duplicación no es redundancia: es el mecanismo que valida la spec. Si el documento fuera
ambiguo, las dos implementaciones producirían hashes distintos sobre los mismos vectores.

**Evidencia:** 8/8 hashes idénticos, 8/8 bytes canónicos, 35/35 códigos de rechazo, sobre
vectores congelados.

### [`pod/prover`](pod/prover) — runner determinístico y recibo firmado

Ejecuta el pedido y firma el resultado. **Ed25519 implementado desde cero en Python puro**
(RFC 8032), sin dependencias fuera de la stdlib. Incluye el simulador original en Node como
harness de fidelidad, para comprobar que el port no cambió el comportamiento.

**Evidencia:** los 3 pedidos reproducen bit a bit en 3 plataformas distintas. Ese determinismo
cross-machine es lo que convierte la resolución de una disputa en un **procedimiento
repetible** y no en la opinión de un árbitro.

### [`pod/program`](pod/program) — escrow con verificación optimista en Solana

**Es la pieza principal de este repo.** Programa nativo, sin Anchor: 7 instrucciones, cuenta de
304 bytes con layout fijo escrito a mano, 34 tests agrupados **por invariante** (I1–I10), y un
cliente JS implementado independientemente desde el documento para cruzar la spec. Corre en
proceso y como `.so` compilado sobre un validador.

**Evidencia:** 34 tests exit 0, más los dos escenarios de disputa verificados en cadena.
Detalle completo en [`pod/program/README.md`](pod/program/README.md).

## Verificarlo

```bash
python pod/spec/scripts/gate.py        # formato canonico: las dos implementaciones
python pod/prover/scripts/gate.py      # determinismo y recibo
bash   pod/program/scripts/gate.sh     # 34 tests del programa (Linux o WSL)
bash   pod/program/devnet/localnet.sh  # despliega el .so y corre el gate en cadena
```

Las keypairs de `pod/program/devnet/` **no están versionadas** a propósito: `setup.sh` y
`localnet.sh` las regeneran si faltan. Una consecuencia: un clon nuevo grindea su propia
dirección vanity, así que el program id no va a coincidir con el de la documentación.

## Qué muestra este repo

Es un proyecto archivado, así que el valor está en el método más que en el producto:

- **Especificar antes de implementar**, y después validar la spec implementándola dos veces
  sin que una mire a la otra.
- **Tests organizados por invariante**, no por función. Diez propiedades que el sistema debe
  cumplir, y cada archivo de test prueba un grupo.
- **Declarar las fronteras de confianza en voz alta.** El sistema tiene un árbitro, o sea un
  tercero de confianza. Está escrito qué puede y qué no puede hacer uno malicioso, y por qué
  no puede robar aunque mienta — en vez de dejarlo implícito.
- **Medir y frenar.** El proyecto se archivó con la medición hecha y la conclusión escrita:
  existe demanda por verificar *la realidad*, no por verificar *cómputo*, que es lo que este
  sistema resuelve. Está en [`ARCHIVO.md`](ARCHIVO.md), incluida la lista de lo que quedó
  validado y lo que era solo razonamiento.

## Mapa

```
ARCHIVO.md          por que se archivo, con la evidencia y las conclusiones separadas
                    entre validadas y razonadas
CONTEXT_INDEX.md    indice de navegacion del repo
pod/spec/           subfase 1.1 — formato canonico, dos implementaciones
pod/prover/         subfase 1.2 — runner determinista y recibo firmado
pod/program/        subfases 1.3 y 1.4 — escrow y verificacion optimista  <-- la pieza principal
*_PLAN.md           el plan por fases y los criterios de corte
*_FASE1.md          la normativa de las subfases 1.1 a 1.6
```
